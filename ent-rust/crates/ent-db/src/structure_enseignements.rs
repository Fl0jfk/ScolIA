//! Socle 5, lot S5 (2/2) : enseignements SUIVIS par l'élève, distincts de ses groupes
//! (`05_STRUCTURE.md` §7.2 n° 1, Q-S2, STR-43, STR-62, STR-68, R-S16).
//!
//! * L'élève *suit* un enseignement (son choix) ; il est *membre* d'un groupe (l'organisation
//!   concrète). Le lien est un contrôle ([`PgEnt::structure_enseignements_incoherences`]), jamais
//!   une écriture silencieuse.
//! * Cycle de vie : `choisi → accepte → quitte`, ou `choisi → refuse | retire`. Un enseignement
//!   quitté garde sa nature (R-S16) et ses dates ; rien ne se supprime ; chaque transition laisse
//!   une ligne d'historique (déclencheur, en ajout seul).
//! * Droits : écriture `referentiel.gerer` (direction, administratif) ; lecture des enseignements
//!   d'UN élève : gestionnaire, ou personnel dont le périmètre D-022 contient cet élève
//!   (`autoriser_eleve`), jamais les familles ici. Un professeur ne voit pas le motif libre.
//! * Les règles de choix sont celles du moteur unique `structure_offre::offre::verifier_choix`.

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::{Action, Role};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::structure_etab::{journaliser, net, publier, structure};
use crate::structure_offre::offre::{verifier_choix, ContexteChoix, Demande, Violation};
use crate::structure_offre::{offre_lue, OffreDetail};
use crate::tenant_tx::TenantTx;

/// Natures de fin R-S16 qu'un acteur peut poser à la main (`fermeture_offre` est réservée à la
/// fermeture de l'offre, `depart` à la sortie de l'élève par le socle 2).
const FINS_MANUELLES: [&str; 4] = [
    "changement_en_cours_annee",
    "abandon_fin_premiere",
    "depart",
    "correction",
];
const AVIS: [&str; 3] = ["favorable", "defavorable", "reserve"];

async fn tx(e: &PgEnt, a: &Acteur<'_>) -> Result<TenantTx<'static>, EntDbError> {
    TenantTx::begin(e.pool(), a.etablissement_id)
        .await
        .map_err(db)
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct EnseignementVue {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub offre_id: Uuid,
    pub offre_code: String,
    pub offre_libelle: String,
    pub matiere_id: Option<Uuid>,
    pub modalite: String,
    pub etat: String,
    pub choisi_le: NaiveDate,
    pub debut_le: Option<NaiveDate>,
    pub quitte_le: Option<NaiveDate>,
    pub fin_nature: Option<String>,
    /// Texte libre : réservé aux gestionnaires (masqué pour les autres lecteurs).
    pub motif: Option<String>,
    pub avis: Option<String>,
    pub source: String,
}

const SQL_VUE: &str = "SELECT e.id, e.annee_id, e.offre_id, o.code AS offre_code, o.libelle AS offre_libelle,
        o.matiere_id, o.modalite, e.etat, e.choisi_le, e.debut_le, e.quitte_le, e.fin_nature, e.motif, e.avis,
        e.source
   FROM ent_eleve_enseignement e
   JOIN ent_offre_enseignement o ON o.id = e.offre_id AND o.etablissement_id = e.etablissement_id";

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct HistoLigne {
    pub de_etat: Option<String>,
    pub vers_etat: String,
    pub effet_le: Option<NaiveDate>,
    pub fin_nature: Option<String>,
    pub motif: Option<String>,
    pub le: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ajout {
    pub offre_id: Uuid,
    /// Dépasser une capacité atteinte (tracé au journal).
    #[serde(default)]
    pub forcer_capacite: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Sortie {
    pub offre_id: Uuid,
    pub fin_nature: String,
    pub motif: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Modification {
    #[serde(default)]
    pub ajouter: Vec<Ajout>,
    #[serde(default)]
    pub quitter: Vec<Sortie>,
    /// Date d'effet (par défaut aujourd'hui, dans le fuseau de l'établissement).
    pub effet: Option<NaiveDate>,
    /// Exiger aussi les minimums des règles (choix achevé) ; sinon ce ne sont que des avertissements.
    #[serde(default)]
    pub exiger_complet: bool,
    /// Ne rien écrire : renvoyer ce qu'il se passerait.
    #[serde(default)]
    pub apercu: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultatModification {
    pub ecrit: bool,
    pub apercu: bool,
    pub violations: Vec<Violation>,
    pub enseignements: Vec<EnseignementVue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApercuFermeture {
    pub offre_id: Uuid,
    pub ferme: bool,
    pub nb_acceptes: i64,
    pub nb_en_attente: i64,
    /// Élèves touchés (pour que les familles reçoivent une proposition de remplacement) : réservé
    /// au gestionnaire qui ferme ; l'événement émis, lui, n'en porte aucun.
    pub eleve_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DecisionSaisie {
    /// `accepter` ou `refuser`.
    pub decision: String,
    /// Avis rendu (sur avis, test ou dossier) ; obligatoire pour accepter une offre non libre.
    pub avis: Option<String>,
    pub effet: Option<NaiveDate>,
    pub motif: Option<String>,
    #[serde(default)]
    pub forcer_capacite: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Incoherence {
    pub cause: String,
    pub eleve_id: Option<Uuid>,
    pub offre_id: Uuid,
    pub enseignement_id: Option<Uuid>,
    pub nb_groupes: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComparaisonReprise {
    pub heritees: i64,
    pub reprises: i64,
    pub a_traiter: i64,
}

/// Niveau de l'élève à une date (classe active de l'année) : sert aux cibles et aux règles par niveau.
async fn contexte_eleve(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    annee: Uuid,
    jour: NaiveDate,
) -> Result<ContexteChoix, EntDbError> {
    let niveau: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT c.niveau_id FROM ent_affectation a
           JOIN ent_classe c ON c.id = a.classe_id AND c.etablissement_id = a.etablissement_id
          WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND a.annee_id = $3 AND a.classe_id IS NOT NULL
            AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $4)
          ORDER BY a.date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .bind(eleve)
    .bind(annee)
    .bind(jour)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(ContexteChoix {
        niveau_id: niveau.flatten(),
        mef_id: None,
    })
}

async fn vues(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    annee: Option<Uuid>,
    actifs_seulement: bool,
) -> Result<Vec<EnseignementVue>, EntDbError> {
    sqlx::query_as::<_, EnseignementVue>(&format!(
        "{SQL_VUE} WHERE e.etablissement_id = $1 AND e.eleve_id = $2 AND ($3::uuid IS NULL OR e.annee_id = $3)
            AND (NOT $4 OR e.etat IN ('choisi', 'accepte'))
          ORDER BY e.annee_id, o.ordre, o.code, e.choisi_le, e.id"
    ))
    .bind(etab)
    .bind(eleve)
    .bind(annee)
    .bind(actifs_seulement)
    .fetch_all(conn)
    .await
    .map_err(db)
}

async fn eleve_existe(conn: &mut PgConnection, etab: Uuid, eleve: Uuid) -> Result<(), EntDbError> {
    let ok: Option<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT fusionne_dans FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    match ok {
        None => Err(introuvable()),
        Some((Some(_),)) => Err(conflit()),
        Some(_) => Ok(()),
    }
}

impl PgEnt {
    /// Lecture des enseignements d'un élève : gestionnaire, ou personnel dont le périmètre D-022
    /// contient l'élève. Un élève d'un autre établissement ou hors périmètre répond « introuvable »
    /// aux non-gestionnaires (pas d'oracle d'existence).
    pub async fn structure_enseignements_eleve(
        &self,
        a: &Acteur<'_>,
        eleve_id: Uuid,
        annee_id: Option<Uuid>,
        actifs_seulement: bool,
    ) -> Result<Vec<EnseignementVue>, EntDbError> {
        let gest = self.acces_eleve(a, eleve_id).await?;
        let mut t = tx(self, a).await?;
        eleve_existe_lecture(&mut t, a.etablissement_id, eleve_id).await?;
        let mut v = vues(
            &mut t,
            a.etablissement_id,
            eleve_id,
            annee_id,
            actifs_seulement,
        )
        .await?;
        t.commit().await.map_err(db)?;
        if !gest {
            for e in &mut v {
                e.motif = None;
            }
        }
        Ok(v)
    }

    pub async fn structure_enseignement_historique(
        &self,
        a: &Acteur<'_>,
        enseignement_id: Uuid,
    ) -> Result<Vec<HistoLigne>, EntDbError> {
        let mut t = tx(self, a).await?;
        let eleve: Option<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_eleve_enseignement WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(enseignement_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let (eleve,) = eleve.ok_or_else(introuvable)?;
        let gest = match self.acces_eleve(a, eleve).await {
            Ok(g) => g,
            Err(EntDbError::Forbidden) => return Err(introuvable()),
            Err(e) => return Err(e),
        };
        let mut h = sqlx::query_as::<_, HistoLigne>(
            "SELECT de_etat, vers_etat, effet_le, fin_nature, motif, le FROM ent_eleve_enseignement_histo
              WHERE etablissement_id = $1 AND enseignement_id = $2 ORDER BY le, id",
        )
        .bind(a.etablissement_id)
        .bind(enseignement_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        if !gest {
            for l in &mut h {
                l.motif = None;
            }
        }
        Ok(h)
    }

    /// `Ok(true)` = gestionnaire ; `Ok(false)` = lecteur du périmètre ; sinon `Forbidden`.
    async fn acces_eleve(&self, a: &Acteur<'_>, eleve_id: Uuid) -> Result<bool, EntDbError> {
        if !Role::parse(a.role).is_some_and(|r| r.is_staff()) {
            return Err(EntDbError::Forbidden);
        }
        if self.ref_peut(a, Action::ReferentielGerer).await? {
            return Ok(true);
        }
        if self
            .autoriser_eleve(
                a.etablissement_id,
                a.user_id,
                a.role,
                Action::SuiviEleve,
                eleve_id,
            )
            .await?
        {
            return Ok(false);
        }
        // Hors périmètre : indistinguable d'un élève inexistant.
        Err(EntDbError::Forbidden)
    }

    /// STR-62 : ajoute et/ou quitte des enseignements suivis d'un élève, en une transaction.
    /// Les règles de choix s'appliquent à l'ENSEMBLE final : un refus bloquant (`ecrit = false`)
    /// n'écrit rien. Les minimums des règles ne bloquent qu'avec `exiger_complet`.
    pub async fn structure_enseignements_modifier(
        &self,
        a: &Acteur<'_>,
        eleve_id: Uuid,
        annee_id: Uuid,
        m: Modification,
    ) -> Result<ResultatModification, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if m.ajouter.is_empty() && m.quitter.is_empty() && !m.exiger_complet {
            return Err(invalide());
        }
        if m.ajouter.len() + m.quitter.len() > 40 {
            return Err(invalide());
        }
        for s in &m.quitter {
            if !FINS_MANUELLES.contains(&s.fin_nature.as_str()) {
                return Err(invalide());
            }
        }
        let mut t = tx(self, a).await?;
        eleve_existe(&mut t, a.etablissement_id, eleve_id).await?;
        let annee: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
            "SELECT cloturee_le FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        match annee {
            None => return Err(introuvable()),
            Some((Some(_),)) => return Err(conflit()),
            Some(_) => {}
        }
        // Les offres qui reçoivent un nouvel élève sont verrouillées dans un ordre fixe : le calcul
        // de capacité qui suit ne peut pas être dépassé par une écriture concurrente.
        let mut a_verrouiller: Vec<Uuid> = m.ajouter.iter().map(|x| x.offre_id).collect();
        a_verrouiller.sort();
        a_verrouiller.dedup();
        if a_verrouiller.len() != m.ajouter.len() {
            return Err(invalide());
        }
        for o in &a_verrouiller {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM ent_offre_enseignement
                  WHERE etablissement_id = $1 AND annee_id = $2 AND id = $3 FOR UPDATE",
            )
            .bind(a.etablissement_id)
            .bind(annee_id)
            .bind(o)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        }
        let jour = match m.effet {
            Some(j) => j,
            None => structure::aujourdhui(&mut t, a.etablissement_id).await?,
        };
        // Enseignements actuellement suivis ou en attente (verrou de ligne).
        let actuels: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT id, offre_id, etat FROM ent_eleve_enseignement
              WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_id = $3 AND etat IN ('choisi', 'accepte')
              FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(eleve_id)
        .bind(annee_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        for ajout in &m.ajouter {
            if actuels.iter().any(|x| x.1 == ajout.offre_id) {
                return Err(conflit());
            }
        }
        for s in &m.quitter {
            if !actuels.iter().any(|x| x.1 == s.offre_id)
                || m.ajouter.iter().any(|x| x.offre_id == s.offre_id)
            {
                return Err(introuvable());
            }
        }
        let mut ensemble: Vec<Uuid> = actuels
            .iter()
            .map(|x| x.1)
            .filter(|o| !m.quitter.iter().any(|s| s.offre_id == *o))
            .collect();
        let nouveaux: Vec<Uuid> = m.ajouter.iter().map(|x| x.offre_id).collect();
        ensemble.extend(nouveaux.iter().copied());
        let ctx = contexte_eleve(&mut t, a.etablissement_id, eleve_id, annee_id, jour).await?;
        let violations = verifier_choix(
            &mut t,
            a.etablissement_id,
            &Demande {
                annee_id,
                contexte: ctx,
                eleve_id: Some(eleve_id),
                ensemble: &ensemble,
                nouveaux: &nouveaux,
                jour,
                complet: m.exiger_complet,
                forcer_capacite: m.ajouter.iter().any(|x| x.forcer_capacite),
            },
        )
        .await?;
        let bloque = violations.iter().any(|v| v.bloquant);
        if m.apercu || bloque {
            let v = vues(&mut t, a.etablissement_id, eleve_id, Some(annee_id), true).await?;
            t.commit().await.map_err(db)?;
            return Ok(ResultatModification {
                ecrit: false,
                apercu: m.apercu,
                violations,
                enseignements: v,
            });
        }

        let avant: Vec<Value> = actuels
            .iter()
            .map(|x| json!({ "offre_id": x.1, "etat": x.2 }))
            .collect();
        let mut premier: Option<Uuid> = None;
        for s in &m.quitter {
            let (id, etat) = actuels
                .iter()
                .find(|x| x.1 == s.offre_id)
                .map(|x| (x.0, x.2.clone()))
                .ok_or_else(introuvable)?;
            premier.get_or_insert(id);
            let motif = net(Some(&s.motif), 300)?.ok_or_else(invalide)?;
            if etat == "choisi" {
                sqlx::query(
                    "UPDATE ent_eleve_enseignement SET etat = 'retire', motif = $3, modifie_par = $4
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(a.etablissement_id)
                .bind(id)
                .bind(&motif)
                .bind(a.user_id)
                .execute(&mut *t)
                .await
                .map_err(db)?;
            } else {
                // Jamais avant le début : un enseignement accepté aujourd'hui peut être quitté aujourd'hui.
                sqlx::query(
                    "UPDATE ent_eleve_enseignement
                        SET etat = 'quitte', quitte_le = greatest($3, debut_le), fin_nature = $4, motif = $5,
                            modifie_par = $6
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(a.etablissement_id)
                .bind(id)
                .bind(jour)
                .bind(&s.fin_nature)
                .bind(&motif)
                .bind(a.user_id)
                .execute(&mut *t)
                .await
                .map_err(db)?;
            }
        }
        for ajout in &m.ajouter {
            let adm: String = sqlx::query_scalar(
                "SELECT admission FROM ent_offre_enseignement WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(a.etablissement_id)
            .bind(ajout.offre_id)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
            let id = Uuid::new_v4();
            premier.get_or_insert(id);
            // Offre libre : acceptée tout de suite ; sinon « choisi », en attente d'avis et de décision.
            let (etat, debut) = if adm == "libre" {
                ("accepte", Some(jour))
            } else {
                ("choisi", None)
            };
            sqlx::query(
                "INSERT INTO ent_eleve_enseignement (id, etablissement_id, annee_id, eleve_id, offre_id, etat,
                        choisi_le, debut_le, source, choisi_par, decide_par, modifie_par)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'saisie',$9,$10,$9)",
            )
            .bind(id)
            .bind(a.etablissement_id)
            .bind(annee_id)
            .bind(eleve_id)
            .bind(ajout.offre_id)
            .bind(etat)
            .bind(jour)
            .bind(debut)
            .bind(a.user_id)
            .bind((etat == "accepte").then_some(a.user_id))
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        let apres_vues = vues(&mut t, a.etablissement_id, eleve_id, Some(annee_id), true).await?;
        journaliser(
            &mut t,
            a,
            "enseignements_suivis",
            &eleve_id.to_string(),
            "modifies",
            Some(json!(avant)),
            Some(json!({
                "ajouts": m.ajouter.iter().map(|x| json!({ "offre_id": x.offre_id,
                    "capacite_forcee": x.forcer_capacite && violations.iter().any(|v|
                        v.code == "capacite_atteinte" && v.offre_id == Some(x.offre_id)) })).collect::<Vec<_>>(),
                "sorties": m.quitter.iter().map(|s| json!({ "offre_id": s.offre_id, "fin_nature": s.fin_nature }))
                    .collect::<Vec<_>>(),
                "effet": jour,
            })),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.enseignements_suivis_modifies",
            premier.unwrap_or_else(Uuid::new_v4),
            json!({ "annee_id": annee_id, "ajouts": m.ajouter.len(), "sorties": m.quitter.len() }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ResultatModification {
            ecrit: true,
            apercu: false,
            violations,
            enseignements: apres_vues,
        })
    }

    /// Décide d'un choix en attente (admission sur avis, test ou dossier). Accepter une offre non
    /// libre exige un avis favorable ; la capacité est vérifiée au moment de l'acceptation.
    pub async fn structure_enseignement_decider(
        &self,
        a: &Acteur<'_>,
        enseignement_id: Uuid,
        d: DecisionSaisie,
    ) -> Result<EnseignementVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if d.decision != "accepter" && d.decision != "refuser" {
            return Err(invalide());
        }
        if d.avis.as_deref().is_some_and(|v| !AVIS.contains(&v)) {
            return Err(invalide());
        }
        let motif = net(d.motif.as_deref(), 300)?;
        let mut t = tx(self, a).await?;
        type Ligne = (Uuid, Uuid, Uuid, String, Option<String>);
        let l: Option<Ligne> = sqlx::query_as(
            "SELECT eleve_id, offre_id, annee_id, etat, avis FROM ent_eleve_enseignement
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(enseignement_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let (eleve, offre, annee, etat, avis_actuel) = l.ok_or_else(introuvable)?;
        if etat != "choisi" {
            return Err(conflit());
        }
        sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM ent_offre_enseignement WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(offre)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let avis = d.avis.clone().or(avis_actuel);
        if d.decision == "accepter" {
            let jour = match d.effet {
                Some(j) => j,
                None => structure::aujourdhui(&mut t, a.etablissement_id).await?,
            };
            // Capacité à l'acceptation (l'élève lui-même ne compte pas).
            let cap: Option<i32> = sqlx::query_scalar(
                "SELECT capacite FROM ent_offre_enseignement WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(a.etablissement_id)
            .bind(offre)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
            if let Some(cap) = cap {
                let pris: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM ent_eleve_enseignement
                      WHERE etablissement_id = $1 AND offre_id = $2 AND etat = 'accepte' AND quitte_le IS NULL",
                )
                .bind(a.etablissement_id)
                .bind(offre)
                .fetch_one(&mut *t)
                .await
                .map_err(db)?;
                if pris >= i64::from(cap) && !d.forcer_capacite {
                    return Err(conflit());
                }
            }
            sqlx::query(
                "UPDATE ent_eleve_enseignement
                    SET etat = 'accepte', debut_le = $3, avis = $4, avis_le = CASE WHEN $4::text IS NULL THEN NULL ELSE now() END,
                        avis_par = CASE WHEN $4::text IS NULL THEN NULL ELSE $5 END,
                        decide_par = $5, modifie_par = $5, motif = $6
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(a.etablissement_id)
            .bind(enseignement_id)
            .bind(jour)
            .bind(&avis)
            .bind(a.user_id)
            .bind(&motif)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        } else {
            sqlx::query(
                "UPDATE ent_eleve_enseignement
                    SET etat = 'refuse', avis = $3, avis_le = CASE WHEN $3::text IS NULL THEN NULL ELSE now() END,
                        avis_par = CASE WHEN $3::text IS NULL THEN NULL ELSE $4 END,
                        decide_par = $4, modifie_par = $4, motif = $5
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(a.etablissement_id)
            .bind(enseignement_id)
            .bind(&avis)
            .bind(a.user_id)
            .bind(&motif)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        journaliser(
            &mut t,
            a,
            "enseignement_suivi",
            &enseignement_id.to_string(),
            if d.decision == "accepter" {
                "accepte"
            } else {
                "refuse"
            },
            Some(json!({ "etat": "choisi" })),
            Some(json!({ "offre_id": offre, "annee_id": annee, "avis": avis })),
        )
        .await?;
        let vue = vues(&mut t, a.etablissement_id, eleve, Some(annee), false)
            .await?
            .into_iter()
            .find(|v| v.id == enseignement_id)
            .ok_or_else(introuvable)?;
        t.commit().await.map_err(db)?;
        Ok(vue)
    }

    /// STR-43 : ferme un enseignement proposé. Sans `confirmer`, renvoie l'aperçu (qui est touché)
    /// sans rien écrire ; avec, ferme l'offre, quitte les enseignements acceptés (nature
    /// `fermeture_offre`) et retire les choix en attente. Direction et administrateur seulement.
    pub async fn structure_offre_fermer(
        &self,
        a: &Acteur<'_>,
        offre_id: Uuid,
        motif: &str,
        effet: Option<NaiveDate>,
        confirmer: bool,
    ) -> Result<ApercuFermeture, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if !matches!(Role::parse(a.role), Some(Role::Direction | Role::Admin)) {
            return Err(EntDbError::Forbidden);
        }
        let motif = net(Some(motif), 300)?.ok_or_else(invalide)?;
        let mut t = tx(self, a).await?;
        sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM ent_offre_enseignement WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(offre_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let avant: OffreDetail = offre_lue(&mut t, a.etablissement_id, offre_id).await?;
        if avant.ferme_le.is_some() {
            return Err(conflit());
        }
        let touches: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT id, eleve_id, etat FROM ent_eleve_enseignement
              WHERE etablissement_id = $1 AND offre_id = $2 AND etat IN ('choisi', 'accepte')
                AND (etat = 'choisi' OR quitte_le IS NULL) ORDER BY id FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(offre_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let apercu = ApercuFermeture {
            offre_id,
            ferme: confirmer,
            nb_acceptes: touches.iter().filter(|x| x.2 == "accepte").count() as i64,
            nb_en_attente: touches.iter().filter(|x| x.2 == "choisi").count() as i64,
            eleve_ids: touches.iter().map(|x| x.1).collect(),
        };
        if !confirmer {
            t.commit().await.map_err(db)?;
            return Ok(apercu);
        }
        let jour = match effet {
            Some(j) => j,
            None => structure::aujourdhui(&mut t, a.etablissement_id).await?,
        };
        let motif_ligne = format!("Offre fermée : {motif}");
        let motif_ligne = motif_ligne.chars().take(300).collect::<String>();
        sqlx::query(
            "UPDATE ent_eleve_enseignement
                SET etat = 'quitte', quitte_le = greatest($3, debut_le), fin_nature = 'fermeture_offre',
                    motif = $4, modifie_par = $5
              WHERE etablissement_id = $1 AND offre_id = $2 AND etat = 'accepte' AND quitte_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(offre_id)
        .bind(jour)
        .bind(&motif_ligne)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_eleve_enseignement SET etat = 'retire', motif = $3, modifie_par = $4
              WHERE etablissement_id = $1 AND offre_id = $2 AND etat = 'choisi'",
        )
        .bind(a.etablissement_id)
        .bind(offre_id)
        .bind(&motif_ligne)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_offre_enseignement SET ferme_le = now(), ferme_motif = $3, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(offre_id)
        .bind(&motif)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(
            &mut t,
            a,
            "offre",
            &offre_id.to_string(),
            "fermee",
            Some(json!({ "code": avant.code, "ferme": false })),
            Some(
                json!({ "ferme": true, "motif": motif, "acceptes": apercu.nb_acceptes,
                         "en_attente": apercu.nb_en_attente }),
            ),
        )
        .await?;
        // L'événement ne porte aucune identité : les familles touchées se relisent par l'offre.
        let ev = publier(
            &mut t,
            a,
            "referentiel.offre_fermee",
            offre_id,
            json!({ "code": avant.code, "acceptes": apercu.nb_acceptes, "en_attente": apercu.nb_en_attente }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(apercu)
    }

    /// STR-68 : file de travail « choix sans groupe ». Cause `sans_groupe` (suit l'enseignement
    /// sans être dans un de ses groupes), `plusieurs_groupes`, `groupe_sans_choix` (membre d'un
    /// groupe de l'offre sans la suivre), `offre_sans_groupe` (offre par groupe sans groupe associé).
    /// Réservé aux gestionnaires : la liste nomme des élèves de tout l'établissement.
    pub async fn structure_enseignements_incoherences(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        jour: Option<NaiveDate>,
    ) -> Result<Vec<Incoherence>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let j = match jour {
            Some(j) => j,
            None => structure::aujourdhui(&mut t, a.etablissement_id).await?,
        };
        let r = sqlx::query_as::<_, Incoherence>(
            "SELECT cause, eleve_id, offre_id, enseignement_id, nb_groupes
               FROM ent_enseignement_incoherences($1, $2, $3) ORDER BY cause, offre_id, eleve_id",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .bind(j)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Élèves dont le choix est achevé mais ne respecte pas les règles de l'année (minimums non
    /// atteints, maximums dépassés après un changement de règle…). Gestionnaires seulement.
    pub async fn structure_enseignements_controle_regles(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<Violation>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        eleve_existe_lecture(&mut t, a.etablissement_id, eleve_id).await?;
        let jour = structure::aujourdhui(&mut t, a.etablissement_id).await?;
        let ensemble: Vec<Uuid> = sqlx::query_scalar(
            "SELECT offre_id FROM ent_eleve_enseignement
              WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_id = $3 AND etat IN ('choisi', 'accepte')",
        )
        .bind(a.etablissement_id)
        .bind(eleve_id)
        .bind(annee_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let ctx = contexte_eleve(&mut t, a.etablissement_id, eleve_id, annee_id, jour).await?;
        let v = verifier_choix(
            &mut t,
            a.etablissement_id,
            &Demande {
                annee_id,
                contexte: ctx,
                eleve_id: Some(eleve_id),
                ensemble: &ensemble,
                nouveaux: &[],
                jour,
                complet: true,
                forcer_capacite: true,
            },
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Comparaison avant / après de la reprise des options historiques : tout ce qui existait est
    /// repris ou rangé « à traiter » (critère « aucune option perdue »).
    pub async fn structure_enseignements_comparaison_reprise(
        &self,
        a: &Acteur<'_>,
    ) -> Result<ComparaisonReprise, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let (heritees, reprises, a_traiter): (i64, i64, i64) = sqlx::query_as(
            "SELECT heritees, reprises, a_traiter FROM ent_eleve_enseignement_comparaison($1)",
        )
        .bind(a.etablissement_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(ComparaisonReprise {
            heritees,
            reprises,
            a_traiter,
        })
    }
}

/// Lecture : un élève inexistant ou d'un autre établissement est « introuvable ».
async fn eleve_existe_lecture(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
) -> Result<(), EntDbError> {
    let ok: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(eleve)
            .fetch_optional(conn)
            .await
            .map_err(db)?;
    ok.map(|_| ()).ok_or_else(introuvable)
}
