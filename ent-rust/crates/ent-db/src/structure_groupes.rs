//! Socle 5, lot S6 (1/2) : groupes (`05_STRUCTURE.md` §9.6, M8, STR-50 à 55, R-S15) et port de lecture de leur
//! composition. La composition (affectations, changement de classe, règles, dédoublement, correction) est dans
//! `structure_composition.rs`.
//!
//! Un **groupe** est un ensemble d'élèves à une date, de **nature** générique (`enseignement`, `dispositif`,
//! `activite`, `liste`). Il a 0..n **classes d'origine** (`ent_groupe_classe`), porte au plus **une offre
//! d'enseignements** (`ent_offre_groupe`, lot S5) et une capacité (avertissement, ou blocage pour une capacité de
//! sécurité, R-S15). Sa **composition** est l'ensemble des affectations datées (`ent_affectation`) : le groupe reste
//! le même objet, sa composition change. Un groupe de nature `liste` ne donne aucun droit (`ouvre_droits` faux) et ne
//! peut pas être le public d'un cours.
//!
//! * droits : lecture `referentiel.lire` (la liste ne nomme aucun élève) ; écriture `structure.gerer`
//!   (`referentiel.gerer` la couvre) ; création d'une activité aussi par `groupes.creer_activite` (le créateur devient
//!   responsable) ; l'établissement vient toujours de la session ;
//! * chaque écriture est journalisée (avant / après) ; les événements `referentiel.*` ne portent que des
//!   identifiants, des codes et des dates, jamais un nom d'élève ni un libellé libre ;
//! * jour = **jour de l'établissement** (`ent_aujourdhui_etab`), jamais le jour UTC.
//!
//! **Contrat de lecture pour les autres socles** : le module [`groupes`] (fonctions qui prennent la connexion ou la
//! transaction de l'appelant, sans acteur : l'appelant a déjà autorisé).
//! * socle 10 (emploi du temps), socle 12 (appel), socle 21 (notes) : [`groupes::membres`],
//!   [`groupes::groupes_de_eleve`], [`groupes::lire`] ; le public d'un cours reste [`crate::structure_cours::cours::public`] ;
//! * socle 6 (équipes, services) : [`groupes::groupes_de_classe`], [`groupes::classes_du_groupe`],
//!   [`groupes::vue_scolarite`] ;
//! * socle 2 (sortie d'un élève) : [`groupes::fermer_affectations`], appelé dans la transaction de la sortie.

use chrono::{Duration, NaiveDate};
use ent_auth::{Action, Portee};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::structure_etab::structure::aujourdhui;
use crate::structure_etab::{journaliser, net, publier, requis};
use crate::tenant_tx::TenantTx;

pub const NATURES: [&str; 4] = ["enseignement", "dispositif", "activite", "liste"];
pub const MODES_DEMI_GROUPES: [&str; 4] = ["alphabetique", "alterne", "equilibre", "manuel"];

pub(crate) async fn tx(e: &PgEnt, a: &Acteur<'_>) -> Result<TenantTx<'static>, EntDbError> {
    TenantTx::begin(e.pool(), a.etablissement_id)
        .await
        .map_err(db)
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct GroupeVue {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub code: String,
    pub libelle: String,
    /// `enseignement`, `dispositif`, `activite`, `liste`.
    pub nature: String,
    /// Reprise : nature déduite (liste « à revoir »).
    pub nature_a_verifier: bool,
    pub offre_id: Option<Uuid>,
    pub capacite: Option<i32>,
    pub capacite_bloquante: bool,
    /// Faux pour une `liste` : aucun droit n'en découle.
    pub ouvre_droits: bool,
    pub ouvert_le: NaiveDate,
    /// Dernier jour du groupe (inclus) ; vide tant qu'il est ouvert.
    pub ferme_le: Option<NaiveDate>,
    pub classes: Vec<Uuid>,
    /// Effectif à la date demandée (élèves scolarisés ce jour-là).
    pub effectif: i64,
}

const SQL_GROUPE: &str = "SELECT g.id, g.annee_id, g.code, g.libelle, g.nature, g.nature_a_verifier,
        (SELECT og.offre_id FROM ent_offre_groupe og
          WHERE og.groupe_id = g.id AND og.etablissement_id = g.etablissement_id) AS offre_id,
        g.capacite, g.capacite_bloquante, g.ouvre_droits, g.ouvert_le, g.ferme_le,
        COALESCE((SELECT array_agg(gc.classe_id ORDER BY gc.classe_id) FROM ent_groupe_classe gc
                   WHERE gc.groupe_id = g.id AND gc.etablissement_id = g.etablissement_id), ARRAY[]::uuid[]) AS classes,
        (SELECT count(*) FROM ent_groupe_membres_jour(g.etablissement_id, $2, g.id)) AS effectif
   FROM ent_groupe g
  WHERE g.etablissement_id = $1";

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct MembreVue {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub depuis: NaiveDate,
    pub jusqu_a: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupeDetail {
    #[serde(flatten)]
    pub groupe: GroupeVue,
    pub membres: Vec<MembreVue>,
    /// Cours ouverts dont ce groupe est le public.
    pub cours_ouverts: Vec<Uuid>,
}

pub(crate) fn json_groupe(g: &GroupeVue) -> Value {
    json!({
        "code": g.code, "nature": g.nature, "offre_id": g.offre_id, "capacite": g.capacite,
        "capacite_bloquante": g.capacite_bloquante, "ouvre_droits": g.ouvre_droits,
        "ouvert_le": g.ouvert_le, "ferme_le": g.ferme_le, "classes": g.classes,
    })
}

// ───────────────────────── Contrat de lecture (socles 2, 6, 10, 12, 21) ─────────────────────────

/// Fonctions sans acteur, qui prennent la connexion ou la transaction de l'appelant (`&mut TenantTx`).
/// L'appelant a déjà autorisé ; l'établissement est toujours un paramètre (jamais déduit d'un identifiant).
pub mod groupes {
    use super::*;

    /// Un groupe de l'établissement, avec son effectif à `jour` (`None` : inconnu ou d'un autre établissement).
    pub async fn lire(
        conn: &mut PgConnection,
        etab: Uuid,
        jour: NaiveDate,
        groupe_id: Uuid,
    ) -> Result<Option<GroupeVue>, EntDbError> {
        sqlx::query_as(&format!("{SQL_GROUPE} AND g.id = $3"))
            .bind(etab)
            .bind(jour)
            .bind(groupe_id)
            .fetch_optional(conn)
            .await
            .map_err(db)
    }

    /// Élèves membres d'un groupe à `jour` (scolarisés ce jour-là). Seule implémentation : la fonction SQL
    /// `ent_groupe_membres_jour`.
    pub async fn membres(
        conn: &mut PgConnection,
        etab: Uuid,
        jour: NaiveDate,
        groupe_id: Uuid,
    ) -> Result<Vec<Uuid>, EntDbError> {
        sqlx::query_scalar("SELECT eleve_id FROM ent_groupe_membres_jour($1, $2, $3) ORDER BY 1")
            .bind(etab)
            .bind(jour)
            .bind(groupe_id)
            .fetch_all(conn)
            .await
            .map_err(db)
    }

    /// Groupes d'un élève à `jour`.
    pub async fn groupes_de_eleve(
        conn: &mut PgConnection,
        etab: Uuid,
        jour: NaiveDate,
        eleve_id: Uuid,
    ) -> Result<Vec<Uuid>, EntDbError> {
        sqlx::query_scalar("SELECT groupe_id FROM ent_groupes_eleve_jour($1, $2, $3) ORDER BY 1")
            .bind(etab)
            .bind(jour)
            .bind(eleve_id)
            .fetch_all(conn)
            .await
            .map_err(db)
    }

    /// Groupes ouverts à `jour` dont `classe_id` est une classe d'origine (groupes inter-classes compris).
    pub async fn groupes_de_classe(
        conn: &mut PgConnection,
        etab: Uuid,
        jour: NaiveDate,
        classe_id: Uuid,
    ) -> Result<Vec<Uuid>, EntDbError> {
        sqlx::query_scalar(
            "SELECT g.id FROM ent_groupe g
               JOIN ent_groupe_classe gc ON gc.groupe_id = g.id AND gc.etablissement_id = g.etablissement_id
              WHERE g.etablissement_id = $1 AND gc.classe_id = $3
                AND g.ouvert_le <= $2 AND (g.ferme_le IS NULL OR g.ferme_le >= $2)
              ORDER BY g.code, g.id",
        )
        .bind(etab)
        .bind(jour)
        .bind(classe_id)
        .fetch_all(conn)
        .await
        .map_err(db)
    }

    /// Classes d'origine d'un groupe (vide : groupe transverse).
    pub async fn classes_du_groupe(
        conn: &mut PgConnection,
        etab: Uuid,
        groupe_id: Uuid,
    ) -> Result<Vec<Uuid>, EntDbError> {
        sqlx::query_scalar(
            "SELECT classe_id FROM ent_groupe_classe WHERE etablissement_id = $1 AND groupe_id = $2 ORDER BY 1",
        )
        .bind(etab)
        .bind(groupe_id)
        .fetch_all(conn)
        .await
        .map_err(db)
    }

    /// STR-69 : scolarité structurelle d'un élève à une date (classe, groupes, cours, enseignements suivis).
    pub async fn vue_scolarite(
        conn: &mut PgConnection,
        etab: Uuid,
        jour: NaiveDate,
        eleve_id: Uuid,
    ) -> Result<VueScolarite, EntDbError> {
        let classe: Option<Uuid> = sqlx::query_scalar(
            "SELECT a.classe_id FROM ent_affectation a
              WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND a.classe_id IS NOT NULL
                AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)
              ORDER BY a.date_debut DESC LIMIT 1",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(jour)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let groupes = groupes_de_eleve(&mut *conn, etab, jour, eleve_id).await?;
        let cours: Vec<Uuid> = sqlx::query_scalar(
            "SELECT cours_id FROM ent_public_cours_jour($1, $2, NULL, NULL, $3) ORDER BY 1",
        )
        .bind(etab)
        .bind(jour)
        .bind(eleve_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let enseignements_suivis: Vec<Uuid> = sqlx::query_scalar(
            "SELECT e.offre_id FROM ent_eleve_enseignement e
              WHERE e.etablissement_id = $1 AND e.eleve_id = $2 AND ent_enseignement_suivi_le(e, $3)
              ORDER BY 1",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(jour)
        .fetch_all(conn)
        .await
        .map_err(db)?;
        Ok(VueScolarite {
            classe,
            groupes,
            cours,
            enseignements_suivis,
        })
    }

    /// STR-67 : à la sortie d'un élève (premier jour d'absence = `a_partir_du`), toutes ses affectations
    /// s'arrêtent la veille ; celles qui n'avaient pas commencé sont annulées (archivées avec leur motif). Appelé
    /// par le socle 2 dans la transaction de la sortie. Publie un fait `referentiel.composition_modifiee` par groupe
    /// quitté (identifiants seulement).
    pub async fn fermer_affectations(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve_id: Uuid,
        a_partir_du: NaiveDate,
        par: Option<Uuid>,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let veille = a_partir_du - Duration::days(1);
        annuler_a_partir_de(conn, etab, eleve_id, a_partir_du, "sortie", par).await?;
        let groupes: Vec<Uuid> = sqlx::query_scalar::<_, Option<Uuid>>(
            "UPDATE ent_affectation SET date_fin = $3
              WHERE etablissement_id = $1 AND eleve_id = $2 AND (date_fin IS NULL OR date_fin > $3)
              RETURNING groupe_id",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(veille)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?
        .into_iter()
        .flatten()
        .collect();
        for g in &groupes {
            sys_composition(conn, etab, *g, &[], &[eleve_id], a_partir_du, "sortie", par).await?;
        }
        Ok(groupes)
    }

    /// Annule (archive avec motif) les affectations de l'élève qui commencent à partir de `jour` : sans objet pour
    /// l'élève (il n'était pas encore dans ces classes ou groupes). Renvoie les groupes concernés.
    pub(crate) async fn annuler_a_partir_de(
        conn: &mut PgConnection,
        etab: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
        motif: &str,
        par: Option<Uuid>,
    ) -> Result<Vec<Uuid>, EntDbError> {
        poser_motif_annulation(conn, motif, par).await?;
        let ids: Vec<Option<Uuid>> = sqlx::query_scalar(
            "DELETE FROM ent_affectation
              WHERE etablissement_id = $1 AND eleve_id = $2 AND date_debut >= $3
              RETURNING groupe_id",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(jour)
        .fetch_all(conn)
        .await
        .map_err(db)?;
        Ok(ids.into_iter().flatten().collect())
    }
}

/// Verrou de ligne sur le groupe pour la durée de la transaction : deux compositions, fermetures ou dédoublements
/// simultanés d'un même groupe s'exécutent l'un après l'autre (la capacité bloquante reste exacte).
pub(crate) async fn verrouiller_groupe(
    conn: &mut PgConnection,
    etab: Uuid,
    groupe_id: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query("SELECT id FROM ent_groupe WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
        .bind(etab)
        .bind(groupe_id)
        .fetch_optional(conn)
        .await
        .map_err(db)?;
    Ok(())
}

/// Posé dans la transaction : le déclencheur d'archive recopie le motif et l'auteur d'une annulation.
pub(crate) async fn poser_motif_annulation(
    conn: &mut PgConnection,
    motif: &str,
    par: Option<Uuid>,
) -> Result<(), EntDbError> {
    sqlx::query("SELECT set_config('ent.annulation_motif', $1, true), set_config('ent.annulation_par', $2, true)")
        .bind(motif)
        .bind(par.map(|p| p.to_string()).unwrap_or_default())
        .execute(conn)
        .await
        .map_err(db)?;
    Ok(())
}

/// Fait système (sans acteur humain) : composition modifiée, publiée dans la transaction de l'appelant.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn sys_composition(
    conn: &mut PgConnection,
    etab: Uuid,
    groupe_id: Uuid,
    entrants: &[Uuid],
    sortants: &[Uuid],
    date: NaiveDate,
    motif: &str,
    par: Option<Uuid>,
) -> Result<(), EntDbError> {
    use crate::outbox::Publier;
    use ent_events::{AggregateKind, MetierEventDraft};
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: "referentiel.composition_modifiee".to_string(),
        aggregate: AggregateKind::Referentiel,
        aggregate_id: groupe_id,
        eleve_id: None,
        payload: json!({
            "groupe_id": groupe_id, "entrants": entrants, "sortants": sortants,
            "date": date, "motif": motif,
        }),
        actor_user_id: par.map(|p| p.to_string()),
    })
    .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct VueScolarite {
    pub classe: Option<Uuid>,
    pub groupes: Vec<Uuid>,
    pub cours: Vec<Uuid>,
    /// Offres suivies (enseignements suivis, lot S5).
    pub enseignements_suivis: Vec<Uuid>,
}

// ───────────────────────── Droits ─────────────────────────

impl PgEnt {
    /// Exige `action` à la portée de l'établissement (les raccourcis `accordee_par` comptent : `referentiel.gerer`
    /// couvre `structure.gerer` et `affectations.gerer`).
    pub(crate) async fn exiger_structure(
        &self,
        a: &Acteur<'_>,
        action: Action,
    ) -> Result<(), EntDbError> {
        if self.a_droit_etablissement(a, action).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    pub(crate) async fn a_droit_etablissement(
        &self,
        a: &Acteur<'_>,
        action: Action,
    ) -> Result<bool, EntDbError> {
        let droits = self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?;
        Ok(droits.iter().any(|d| {
            d.portee == Portee::Etablissement && crate::droits_ctx::accorde(d.action, action)
        }))
    }

    /// Qui peut composer ce groupe ? Les gestionnaires (`affectations.gerer`) pour tout groupe ; le détenteur de
    /// `groupes.composer` pour les groupes d'ACTIVITÉ ou de LISTE dont il est responsable à la date (portée « mes
    /// groupes »), ou désignés, ou de l'établissement. Renvoie `true` si l'acteur est un gestionnaire (pas de
    /// restriction de date ni de nature).
    pub(crate) async fn exiger_composition(
        &self,
        a: &Acteur<'_>,
        conn: &mut PgConnection,
        groupe: &GroupeVue,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let droits = self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?;
        if droits.iter().any(|d| {
            d.portee == Portee::Etablissement
                && crate::droits_ctx::accorde(d.action, Action::AffectationsGerer)
        }) {
            return Ok(true);
        }
        // Un groupe qui ouvre des droits sur ses membres (portée « mes groupes ») ne se compose que par la gestion :
        // sinon un responsable s'ouvrirait le dossier de n'importe quel élève.
        if !matches!(groupe.nature.as_str(), "activite" | "liste") || groupe.ouvre_droits {
            return Err(EntDbError::Forbidden);
        }
        let mut ok = false;
        for d in droits
            .iter()
            .filter(|d| crate::droits_ctx::accorde(d.action, Action::GroupesComposer))
        {
            ok |= match d.portee {
                Portee::Etablissement => true,
                Portee::Designes => d.cibles.contains(&groupe.id),
                Portee::MesGroupes => {
                    sqlx::query_scalar::<_, bool>(
                        "SELECT EXISTS (SELECT 1 FROM ent_groupe_responsable r
                                         WHERE r.etablissement_id = $1 AND r.groupe_id = $2 AND r.user_id = $3
                                           AND r.du <= $4 AND (r.au IS NULL OR r.au >= $4))",
                    )
                    .bind(a.etablissement_id)
                    .bind(groupe.id)
                    .bind(a.user_id)
                    .bind(jour)
                    .fetch_one(&mut *conn)
                    .await
                    .map_err(db)?
                }
                _ => false,
            };
        }
        if ok {
            Ok(false)
        } else {
            Err(EntDbError::Forbidden)
        }
    }
}

// ───────────────────────── Saisies ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct GroupeSaisie {
    pub code: String,
    pub libelle: Option<String>,
    pub nature: String,
    /// Classes d'origine (0..n) ; toutes de la même année.
    #[serde(default)]
    pub classes: Vec<Uuid>,
    /// Offre d'enseignements portée (organisée par groupe, même année).
    pub offre_id: Option<Uuid>,
    pub capacite: Option<i32>,
    #[serde(default)]
    pub capacite_bloquante: bool,
    /// Sans classe : l'année demandée, à défaut l'année qui contient aujourd'hui.
    pub annee_id: Option<Uuid>,
    /// Défaut : début de l'année.
    pub ouvert_le: Option<NaiveDate>,
    /// Défaut : vrai, sauf pour une `liste` (toujours faux).
    pub ouvre_droits: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GroupeModif {
    pub libelle: Option<String>,
    pub capacite: Option<i32>,
    pub effacer_capacite: Option<bool>,
    pub capacite_bloquante: Option<bool>,
    pub ouvre_droits: Option<bool>,
    /// Nouvelle nature (refusée vers `liste` si un cours ouvert ou un responsable en dépend).
    pub nature: Option<String>,
    /// Remplace l'ensemble des classes d'origine.
    pub classes: Option<Vec<Uuid>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerationSaisie {
    pub offre_id: Uuid,
    /// Classes concernées (au moins une).
    pub classes: Vec<Uuid>,
    /// `par_classe` : un groupe par classe ; `regroupe` : un seul groupe pour toutes les classes.
    pub mode: String,
    pub capacite: Option<i32>,
    #[serde(default)]
    pub capacite_bloquante: bool,
    /// Faux (défaut) : aperçu seulement.
    #[serde(default)]
    pub ecrire: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupePropose {
    pub code: String,
    pub libelle: String,
    pub classes: Vec<Uuid>,
    /// Un groupe de la même offre sur le même ensemble de classes existe déjà (rien n'est créé).
    pub existant: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerationResultat {
    pub applique: bool,
    pub groupes: Vec<GroupePropose>,
    pub crees: Vec<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DemiGroupesSaisie {
    pub classe_id: Uuid,
    /// `alphabetique`, `alterne`, `equilibre` (filles / garçons), `manuel`.
    pub mode: String,
    pub code_a: Option<String>,
    pub code_b: Option<String>,
    /// Pour `manuel` : les élèves du demi-groupe B (les autres vont dans A).
    #[serde(default)]
    pub eleves_b: Vec<Uuid>,
    /// Défaut : aujourd'hui (au plus tôt début de l'année).
    pub date_effet: Option<NaiveDate>,
    pub capacite_bloquante: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DemiGroupesResultat {
    pub groupe_a: Uuid,
    pub groupe_b: Uuid,
    pub eleves_a: Vec<Uuid>,
    pub eleves_b: Vec<Uuid>,
    /// Élèves de la classe à la date : chacun dans exactement un des deux (contrôle).
    pub couvre_la_classe: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupeFerme {
    pub groupe: GroupeVue,
    pub membres_clos: i64,
    pub affectations_futures_annulees: i64,
    pub responsables_clos: i64,
    pub cours_fermes: Vec<Uuid>,
    /// Créneaux d'emploi du temps qui visent encore les cours fermés (à reprendre au socle 10).
    pub creneaux_touches: i64,
}

fn nature_valide(n: &str) -> Result<(), EntDbError> {
    if NATURES.contains(&n) {
        Ok(())
    } else {
        Err(invalide())
    }
}

fn capacite_valide(c: Option<i32>) -> Result<Option<i32>, EntDbError> {
    match c {
        None => Ok(None),
        Some(v) if (1..=5000).contains(&v) => Ok(Some(v)),
        Some(_) => Err(invalide()),
    }
}

/// Année et existence des classes (toutes de l'établissement et d'une même année).
async fn annee_des_classes(
    conn: &mut PgConnection,
    etab: Uuid,
    classes: &[Uuid],
) -> Result<Option<Uuid>, EntDbError> {
    if classes.is_empty() {
        return Ok(None);
    }
    let lignes: Vec<(Uuid, Uuid, bool)> = sqlx::query_as(
        "SELECT id, annee_id, archivee_le IS NOT NULL FROM ent_classe
          WHERE etablissement_id = $1 AND id = ANY($2)",
    )
    .bind(etab)
    .bind(classes)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    if lignes.len() != classes.len() {
        return Err(introuvable());
    }
    let an = lignes[0].1;
    if lignes.iter().any(|l| l.1 != an || l.2) {
        return Err(invalide());
    }
    Ok(Some(an))
}

fn dedoublonner(v: &[Uuid]) -> Vec<Uuid> {
    let mut vus = std::collections::HashSet::new();
    v.iter().copied().filter(|x| vus.insert(*x)).collect()
}

impl PgEnt {
    /// Groupes de l'établissement à une date (défaut : aujourd'hui). Sans `annee_id`, l'année qui contient le jour.
    pub async fn structure_groupes_liste(
        &self,
        a: &Acteur<'_>,
        annee_id: Option<Uuid>,
        classe_id: Option<Uuid>,
        nature: Option<&str>,
        avec_fermes: bool,
        jour: Option<NaiveDate>,
    ) -> Result<Vec<GroupeVue>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        if let Some(n) = nature {
            nature_valide(n)?;
        }
        let mut t = tx(self, a).await?;
        let jour = match jour {
            Some(j) => j,
            None => aujourdhui(&mut t, a.etablissement_id).await?,
        };
        let annee: Option<Uuid> = match annee_id {
            Some(y) => Some(y),
            None => sqlx::query_scalar(
                "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1
                    AND date_debut <= $2 AND date_fin >= $2 ORDER BY date_debut DESC LIMIT 1",
            )
            .bind(a.etablissement_id)
            .bind(jour)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?,
        };
        let Some(annee) = annee else {
            t.commit().await.map_err(db)?;
            return Ok(Vec::new());
        };
        let rows = sqlx::query_as::<_, GroupeVue>(&format!(
            "{SQL_GROUPE} AND g.annee_id = $3
                AND ($4::uuid IS NULL OR EXISTS (SELECT 1 FROM ent_groupe_classe c
                         WHERE c.groupe_id = g.id AND c.etablissement_id = g.etablissement_id AND c.classe_id = $4))
                AND ($5::text IS NULL OR g.nature = $5)
                AND ($6 OR g.ferme_le IS NULL OR g.ferme_le >= $2)
              ORDER BY g.code, g.id"
        ))
        .bind(a.etablissement_id)
        .bind(jour)
        .bind(annee)
        .bind(classe_id)
        .bind(nature)
        .bind(avec_fermes)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(rows)
    }

    /// Un groupe avec sa composition à `jour` (identité des élèves : réservé à qui compose le groupe ou le gère).
    pub async fn structure_groupe_detail(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        jour: Option<NaiveDate>,
    ) -> Result<GroupeDetail, EntDbError> {
        let mut t = tx(self, a).await?;
        let aujourd = aujourdhui(&mut t, a.etablissement_id).await?;
        let jour = jour.unwrap_or(aujourd);
        let g = groupes::lire(&mut t, a.etablissement_id, jour, id)
            .await?
            .ok_or_else(introuvable)?;
        let gestion = self
            .a_droit_etablissement(a, Action::StructureGerer)
            .await?
            || self
                .a_droit_etablissement(a, Action::AffectationsGerer)
                .await?;
        if !gestion {
            self.exiger_composition(a, &mut t, &g, aujourd).await?;
        }
        let membres = sqlx::query_as::<_, MembreVue>(
            "SELECT e.id AS eleve_id, e.nom, e.prenom, a.date_debut AS depuis, a.date_fin AS jusqu_a
               FROM ent_groupe_membres_jour($1, $2, $3) m
               JOIN ent_affectation a ON a.etablissement_id = $1 AND a.groupe_id = m.groupe_id AND a.eleve_id = m.eleve_id
                AND a.date_debut <= $2 AND (a.date_fin IS NULL OR a.date_fin >= $2)
               JOIN ent_eleve e ON e.id = m.eleve_id AND e.etablissement_id = $1
              ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(a.etablissement_id)
        .bind(jour)
        .bind(id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let cours_ouverts: Vec<Uuid> = sqlx::query_scalar(
            "SELECT c.id FROM ent_cours c WHERE c.etablissement_id = $1 AND c.groupe_id = $2
                AND c.ouvert_le <= $3 AND (c.ferme_le IS NULL OR c.ferme_le >= $3) ORDER BY c.id",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(jour)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(GroupeDetail {
            groupe: g,
            membres,
            cours_ouverts,
        })
    }

    /// STR-50 : crée un groupe (nature, classes d'origine, offre, capacité).
    pub async fn structure_groupe_creer(
        &self,
        a: &Acteur<'_>,
        s: GroupeSaisie,
    ) -> Result<GroupeVue, EntDbError> {
        self.exiger_structure(a, Action::StructureGerer).await?;
        let mut t = tx(self, a).await?;
        let g = self.creer_groupe_sur(&mut t, a, &s, "groupe_cree").await?;
        t.commit().await.map_err(db)?;
        Ok(g.0)
    }

    /// Cœur partagé de STR-50, 51, 52, 53 : valide, insère le groupe, ses classes d'origine et son offre, journalise
    /// et publie dans la transaction. Renvoie le groupe et l'événement à diffuser après le commit.
    pub(crate) async fn creer_groupe_sur(
        &self,
        t: &mut TenantTx<'static>,
        a: &Acteur<'_>,
        s: &GroupeSaisie,
        evenement: &str,
    ) -> Result<(GroupeVue, ent_events::MetierEvent), EntDbError> {
        let etab = a.etablissement_id;
        nature_valide(&s.nature)?;
        let code = requis(&s.code, 40)?;
        let libelle = net(s.libelle.as_deref(), 160)?.unwrap_or_else(|| code.clone());
        let capacite = capacite_valide(s.capacite)?;
        let classes = dedoublonner(&s.classes);
        let ouvre_droits = match (s.nature.as_str(), s.ouvre_droits) {
            ("liste", Some(true)) => return Err(invalide()),
            ("liste", _) => false,
            (_, v) => v.unwrap_or(true),
        };
        let annee_classes = annee_des_classes(t, etab, &classes).await?;
        let annee = match (annee_classes, s.annee_id) {
            (Some(c), Some(y)) if c != y => return Err(invalide()),
            (Some(c), _) => c,
            (None, Some(y)) => y,
            (None, None) => {
                let jour = aujourdhui(t, etab).await?;
                sqlx::query_scalar(
                    "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1
                        AND date_debut <= $2 AND date_fin >= $2 ORDER BY date_debut DESC LIMIT 1",
                )
                .bind(etab)
                .bind(jour)
                .fetch_optional(&mut **t)
                .await
                .map_err(db)?
                .ok_or_else(introuvable)?
            }
        };
        let (debut, fin, close): (NaiveDate, NaiveDate, bool) = sqlx::query_as(
            "SELECT date_debut, date_fin, cloturee_le IS NOT NULL FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(annee)
        .fetch_optional(&mut **t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if close {
            return Err(invalide());
        }
        let ouvert_le = s.ouvert_le.unwrap_or(debut);
        if ouvert_le < debut || ouvert_le > fin {
            return Err(invalide());
        }
        if let Some(offre) = s.offre_id {
            self.verifier_offre_de_groupe(t, etab, annee, offre).await?;
        }
        let id = Uuid::new_v4();
        let r = sqlx::query(
            "INSERT INTO ent_groupe (id, etablissement_id, annee_id, code, libelle, nature, capacite,
                                     capacite_bloquante, ouvre_droits, ouvert_le)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(etab)
        .bind(annee)
        .bind(&code)
        .bind(&libelle)
        .bind(&s.nature)
        .bind(capacite)
        .bind(s.capacite_bloquante)
        .bind(ouvre_droits)
        .bind(ouvert_le)
        .execute(&mut **t)
        .await;
        if let Err(e) = r {
            return Err(db(e)); // 23505 : le code existe déjà dans l'année → conflit
        }
        for c in &classes {
            sqlx::query("INSERT INTO ent_groupe_classe (etablissement_id, groupe_id, classe_id) VALUES ($1,$2,$3)")
                .bind(etab)
                .bind(id)
                .bind(c)
                .execute(&mut **t)
                .await
                .map_err(db)?;
        }
        if let Some(offre) = s.offre_id {
            sqlx::query(
                "INSERT INTO ent_offre_groupe (id, etablissement_id, offre_id, groupe_id, created_by)
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(offre)
            .bind(id)
            .bind(a.user_id)
            .execute(&mut **t)
            .await
            .map_err(db)?;
        }
        let jour = aujourdhui(t, etab).await?;
        let g = groupes::lire(t, etab, jour, id)
            .await?
            .ok_or_else(introuvable)?;
        journaliser(
            t,
            a,
            "groupe",
            &id.to_string(),
            "cree",
            None,
            Some(json_groupe(&g)),
        )
        .await?;
        let ev = publier(
            t,
            a,
            &format!("referentiel.{evenement}"),
            id,
            json!({ "annee_id": annee, "nature": g.nature }),
        )
        .await?;
        Ok((g, ev))
    }

    /// L'offre existe, est de l'année, organisée par groupe, ouverte.
    async fn verifier_offre_de_groupe(
        &self,
        t: &mut TenantTx<'static>,
        etab: Uuid,
        annee: Uuid,
        offre: Uuid,
    ) -> Result<(), EntDbError> {
        let o: Option<(Uuid, String, bool)> = sqlx::query_as(
            "SELECT annee_id, public_type, ferme_le IS NOT NULL FROM ent_offre_enseignement
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(offre)
        .fetch_optional(&mut **t)
        .await
        .map_err(db)?;
        let (an, public, fermee) = o.ok_or_else(introuvable)?;
        if an != annee || public != "groupe" || fermee {
            return Err(invalide());
        }
        Ok(())
    }

    /// STR-51 : génère les groupes d'un enseignement (« un groupe par classe », « un groupe pour 4A + 4B »).
    /// Aperçu par défaut ; **idempotent** : un groupe de la même offre sur le même ensemble de classes n'est jamais
    /// recréé.
    pub async fn structure_groupes_generer(
        &self,
        a: &Acteur<'_>,
        s: GenerationSaisie,
    ) -> Result<GenerationResultat, EntDbError> {
        self.exiger_structure(a, Action::StructureGerer).await?;
        let etab = a.etablissement_id;
        let classes = dedoublonner(&s.classes);
        if classes.is_empty() || !matches!(s.mode.as_str(), "par_classe" | "regroupe") {
            return Err(invalide());
        }
        let capacite = capacite_valide(s.capacite)?;
        let mut t = tx(self, a).await?;
        let annee = annee_des_classes(&mut t, etab, &classes)
            .await?
            .ok_or_else(invalide)?;
        self.verifier_offre_de_groupe(&mut t, etab, annee, s.offre_id)
            .await?;
        let (offre_code, offre_libelle): (String, String) = sqlx::query_as(
            "SELECT code, libelle FROM ent_offre_enseignement WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(s.offre_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let codes: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, code FROM ent_classe WHERE etablissement_id = $1 AND id = ANY($2) ORDER BY code, id",
        )
        .bind(etab)
        .bind(&classes)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let ensembles: Vec<Vec<(Uuid, String)>> = if s.mode == "par_classe" {
            codes.iter().map(|c| vec![c.clone()]).collect()
        } else {
            vec![codes.clone()]
        };
        let mut propose = Vec::new();
        for e in &ensembles {
            let ids: Vec<Uuid> = e.iter().map(|c| c.0).collect();
            let code = format!(
                "{}-{}",
                e.iter().map(|c| c.1.as_str()).collect::<Vec<_>>().join("+"),
                offre_code
            );
            // Même offre ET même ensemble de classes d'origine = déjà généré.
            let existant: Option<Uuid> = sqlx::query_scalar(
                "SELECT g.id FROM ent_groupe g
                   JOIN ent_offre_groupe og ON og.groupe_id = g.id AND og.etablissement_id = g.etablissement_id
                  WHERE g.etablissement_id = $1 AND g.annee_id = $2 AND og.offre_id = $3 AND g.ferme_le IS NULL
                    AND (SELECT array_agg(gc.classe_id ORDER BY gc.classe_id) FROM ent_groupe_classe gc
                          WHERE gc.groupe_id = g.id AND gc.etablissement_id = g.etablissement_id)
                        = (SELECT array_agg(x ORDER BY x) FROM unnest($4::uuid[]) x)
                  ORDER BY g.created_at LIMIT 1",
            )
            .bind(etab)
            .bind(annee)
            .bind(s.offre_id)
            .bind(&ids)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?;
            propose.push(GroupePropose {
                libelle: format!(
                    "{offre_libelle} — {}",
                    e.iter()
                        .map(|c| c.1.as_str())
                        .collect::<Vec<_>>()
                        .join(" + ")
                ),
                code,
                classes: ids,
                existant,
            });
        }
        let mut crees = Vec::new();
        let mut events = Vec::new();
        if s.ecrire {
            for p in propose.iter_mut().filter(|p| p.existant.is_none()) {
                let saisie = GroupeSaisie {
                    code: p.code.clone(),
                    libelle: Some(p.libelle.chars().take(160).collect()),
                    nature: "enseignement".into(),
                    classes: p.classes.clone(),
                    offre_id: Some(s.offre_id),
                    capacite,
                    capacite_bloquante: s.capacite_bloquante,
                    annee_id: Some(annee),
                    ouvert_le: None,
                    ouvre_droits: None,
                };
                let (g, _) = self
                    .creer_groupe_sur(&mut t, a, &saisie, "groupe_cree")
                    .await?;
                p.existant = Some(g.id);
                crees.push(g.id);
            }
            if !crees.is_empty() {
                events.push(
                    publier(
                        &mut t,
                        a,
                        "referentiel.groupes_generes",
                        s.offre_id,
                        json!({ "annee_id": annee, "offre_id": s.offre_id, "groupes": crees }),
                    )
                    .await?,
                );
            }
        }
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&events);
        let _ = offre_libelle;
        Ok(GenerationResultat {
            applique: s.ecrire,
            groupes: propose,
            crees,
        })
    }

    /// STR-53 : crée un atelier, un club, un groupe d'activité. Un gestionnaire (`structure.gerer`) peut en créer
    /// pour d'autres ; le détenteur de `groupes.creer_activite` en crée un dont il devient **responsable**.
    pub async fn structure_activite_creer(
        &self,
        a: &Acteur<'_>,
        mut s: GroupeSaisie,
    ) -> Result<GroupeVue, EntDbError> {
        let gestion = self
            .a_droit_etablissement(a, Action::StructureGerer)
            .await?;
        if !gestion
            && !self
                .a_droit_etablissement(a, Action::GroupesCreerActivite)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        s.nature = "activite".into();
        s.offre_id = None;
        if !gestion {
            // Un créateur d'activité qui n'est pas gestionnaire ne se donne jamais de droit sur des élèves par ce biais.
            s.ouvre_droits = Some(false);
        }
        let mut t = tx(self, a).await?;
        let (g, ev) = self.creer_groupe_sur(&mut t, a, &s, "groupe_cree").await?;
        if !gestion {
            let jour = aujourdhui(&mut t, a.etablissement_id).await?;
            sqlx::query(
                "INSERT INTO ent_groupe_responsable (id, groupe_id, etablissement_id, user_id, nature, du)
                 VALUES ($1,$2,$3,$4,'responsable',$5)",
            )
            .bind(Uuid::new_v4())
            .bind(g.id)
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .bind(jour)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            let ev2 = publier(
                &mut t,
                a,
                "referentiel.responsable_groupe_change",
                g.id,
                json!({ "groupe_id": g.id, "nature": "responsable" }),
            )
            .await?;
            t.commit().await.map_err(db)?;
            self.diffuser_apres_commit(&[ev, ev2]);
        } else {
            t.commit().await.map_err(db)?;
            self.diffuser_apres_commit(&[ev]);
        }
        Ok(g)
    }

    /// STR-55 : modifie un groupe (libellé, capacité, nature, droits, classes d'origine). Le code, l'année et
    /// l'offre ne changent pas.
    pub async fn structure_groupe_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        m: GroupeModif,
    ) -> Result<GroupeVue, EntDbError> {
        self.exiger_structure(a, Action::StructureGerer).await?;
        let etab = a.etablissement_id;
        let libelle = net(m.libelle.as_deref(), 160)?;
        if m.libelle.is_some() && libelle.is_none() {
            return Err(invalide());
        }
        if let Some(n) = &m.nature {
            nature_valide(n)?;
        }
        let capacite = capacite_valide(m.capacite)?;
        let mut t = tx(self, a).await?;
        verrouiller_groupe(&mut t, etab, id).await?;
        let jour = aujourdhui(&mut t, etab).await?;
        let avant = groupes::lire(&mut t, etab, jour, id)
            .await?
            .ok_or_else(introuvable)?;
        if avant.ferme_le.is_some_and(|f| f < jour) {
            return Err(invalide()); // un groupe fermé ne se modifie plus
        }
        let nature = m.nature.clone().unwrap_or_else(|| avant.nature.clone());
        if nature == "liste" && avant.nature != "liste" {
            let dependances: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_cours c WHERE c.etablissement_id = $1 AND c.groupe_id = $2 AND c.ferme_le IS NULL)
                     OR EXISTS (SELECT 1 FROM ent_groupe_responsable r WHERE r.etablissement_id = $1 AND r.groupe_id = $2
                                 AND (r.au IS NULL OR r.au >= $3))
                     OR EXISTS (SELECT 1 FROM ent_service_enseignement s WHERE s.etablissement_id = $1 AND s.groupe_id = $2)",
            )
            .bind(etab)
            .bind(id)
            .bind(jour)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
            if dependances {
                return Err(conflit());
            }
        }
        let ouvre_droits = if nature == "liste" {
            if m.ouvre_droits == Some(true) {
                return Err(invalide());
            }
            false
        } else {
            m.ouvre_droits.unwrap_or(if avant.nature == "liste" {
                true
            } else {
                avant.ouvre_droits
            })
        };
        let cap_finale = if m.effacer_capacite == Some(true) {
            None
        } else {
            capacite.or(avant.capacite)
        };
        let r = sqlx::query(
            "UPDATE ent_groupe SET libelle = COALESCE($3, libelle), nature = $4, nature_a_verifier = false,
                    capacite = $5, capacite_bloquante = COALESCE($6, capacite_bloquante), ouvre_droits = $7
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&libelle)
        .bind(&nature)
        .bind(cap_finale)
        .bind(m.capacite_bloquante)
        .bind(ouvre_droits)
        .execute(&mut *t)
        .await;
        r.map_err(db)?;
        if let Some(nouvelles) = &m.classes {
            let nouvelles = dedoublonner(nouvelles);
            let annee = annee_des_classes(&mut t, etab, &nouvelles).await?;
            if annee.is_some_and(|y| y != avant.annee_id) {
                return Err(invalide());
            }
            sqlx::query(
                "DELETE FROM ent_groupe_classe WHERE etablissement_id = $1 AND groupe_id = $2 AND classe_id <> ALL($3)",
            )
            .bind(etab)
            .bind(id)
            .bind(&nouvelles)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            for c in &nouvelles {
                sqlx::query(
                    "INSERT INTO ent_groupe_classe (etablissement_id, groupe_id, classe_id) VALUES ($1,$2,$3)
                     ON CONFLICT DO NOTHING",
                )
                .bind(etab)
                .bind(id)
                .bind(c)
                .execute(&mut *t)
                .await
                .map_err(db)?;
            }
        }
        let apres = groupes::lire(&mut t, etab, jour, id)
            .await?
            .ok_or_else(introuvable)?;
        journaliser(
            &mut t,
            a,
            "groupe",
            &id.to_string(),
            "modifie",
            Some(json_groupe(&avant)),
            Some(json_groupe(&apres)),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.groupe_modifie",
            id,
            json!({ "annee_id": apres.annee_id, "nature": apres.nature }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(apres)
    }

    /// STR-55 : ferme un groupe. `a_la_date` est son **dernier jour** (inclus ; défaut : aujourd'hui). Les
    /// affectations ouvertes sont closes à cette date, celles qui commenceraient après sont annulées (archivées),
    /// les responsabilités datées sont closes, et les cours dont le groupe est le public sont fermés à la même date.
    pub async fn structure_groupe_fermer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        a_la_date: Option<NaiveDate>,
        apercu: bool,
    ) -> Result<GroupeFerme, EntDbError> {
        self.exiger_structure(a, Action::StructureGerer).await?;
        let etab = a.etablissement_id;
        let mut t = tx(self, a).await?;
        verrouiller_groupe(&mut t, etab, id).await?;
        let aujourd = aujourdhui(&mut t, etab).await?;
        let date = a_la_date.unwrap_or(aujourd);
        let g = groupes::lire(&mut t, etab, date, id)
            .await?
            .ok_or_else(introuvable)?;
        if g.ferme_le.is_some() {
            return Err(conflit());
        }
        let (debut, fin): (NaiveDate, NaiveDate) = sqlx::query_as(
            "SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(g.annee_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if date < g.ouvert_le.max(debut) || date > fin {
            return Err(invalide());
        }
        let membres: Vec<Uuid> = groupes::membres(&mut t, etab, date, id).await?;
        let a_clore: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_affectation WHERE etablissement_id = $1 AND groupe_id = $2
                AND date_debut <= $3 AND (date_fin IS NULL OR date_fin > $3)",
        )
        .bind(etab)
        .bind(id)
        .bind(date)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let futures: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_affectation WHERE etablissement_id = $1 AND groupe_id = $2 AND date_debut > $3",
        )
        .bind(etab)
        .bind(id)
        .bind(date)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let cours: Vec<Uuid> = sqlx::query_scalar(
            "SELECT c.id FROM ent_cours c WHERE c.etablissement_id = $1 AND c.groupe_id = $2
                AND c.ferme_le IS NULL AND c.ouvert_le <= $3 ORDER BY c.id",
        )
        .bind(etab)
        .bind(id)
        .bind(date)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let creneaux: i64 = if cours.is_empty() {
            0
        } else {
            sqlx::query_scalar(
                "SELECT count(*) FROM ent_edt_creneau WHERE etablissement_id = $1 AND cours_id = ANY($2)",
            )
            .bind(etab)
            .bind(&cours)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?
        };
        let resp: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_groupe_responsable WHERE etablissement_id = $1 AND groupe_id = $2
                AND (au IS NULL OR au > $3)",
        )
        .bind(etab)
        .bind(id)
        .bind(date)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let mut resultat = GroupeFerme {
            groupe: g.clone(),
            membres_clos: a_clore,
            affectations_futures_annulees: futures,
            responsables_clos: resp,
            cours_fermes: cours.clone(),
            creneaux_touches: creneaux,
        };
        if apercu {
            t.rollback().await.map_err(db)?;
            return Ok(resultat);
        }
        poser_motif_annulation(&mut t, "groupe_ferme", Some(a.user_id)).await?;
        sqlx::query("DELETE FROM ent_affectation WHERE etablissement_id = $1 AND groupe_id = $2 AND date_debut > $3")
            .bind(etab)
            .bind(id)
            .bind(date)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        sqlx::query("UPDATE ent_groupe SET ferme_le = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .bind(date)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        sqlx::query(
            "UPDATE ent_affectation SET date_fin = $3
              WHERE etablissement_id = $1 AND groupe_id = $2 AND date_debut <= $3 AND (date_fin IS NULL OR date_fin > $3)",
        )
        .bind(etab)
        .bind(id)
        .bind(date)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_groupe_responsable SET au = $3, termine_par_user_id = $4
              WHERE etablissement_id = $1 AND groupe_id = $2 AND (au IS NULL OR au > $3)",
        )
        .bind(etab)
        .bind(id)
        .bind(date)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        for c in &cours {
            sqlx::query(
                "UPDATE ent_cours SET ferme_le = $3 WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(c)
            .bind(date)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        let apres = groupes::lire(&mut t, etab, date, id)
            .await?
            .ok_or_else(introuvable)?;
        journaliser(
            &mut t,
            a,
            "groupe",
            &id.to_string(),
            "ferme",
            Some(json_groupe(&g)),
            Some(json_groupe(&apres)),
        )
        .await?;
        let mut events = vec![
            publier(
                &mut t,
                a,
                "referentiel.groupe_ferme",
                id,
                json!({ "annee_id": g.annee_id, "ferme_le": date, "cours_fermes": cours }),
            )
            .await?,
        ];
        if !membres.is_empty() {
            events.push(
                publier(
                    &mut t,
                    a,
                    "referentiel.composition_modifiee",
                    id,
                    json!({ "groupe_id": id, "entrants": [], "sortants": membres, "date": date, "motif": "groupe_ferme" }),
                )
                .await?,
            );
        }
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&events);
        resultat.groupe = apres;
        Ok(resultat)
    }
}
