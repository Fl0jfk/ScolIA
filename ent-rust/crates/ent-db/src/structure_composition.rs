//! Socle 5, lot S6 (2/2) : composition des groupes et affectations des élèves
//! (`05_STRUCTURE.md` §4.5, §9.6, STR-52, 60 à 68, 103, 104, R-S5, R-S6, R-S15).
//!
//! * **Tout est daté, rien n'est effacé** : une entrée est une affectation qui commence à la date d'effet, une
//!   sortie close l'affectation la veille ; une affectation erronée est **annulée** (archivée avec son motif dans
//!   `ent_affectation_supprimee`, jamais perdue) ; la bonne est posée à la date d'origine (STR-104).
//! * **Une proposition ne s'applique jamais seule** (STR-64) : le serveur calcule, une personne valide, le serveur
//!   recalcule et n'applique que l'intersection.
//! * **Les commandes ★** (changement de classe, lot, dédoublement, correction) sont des **aperçus par défaut** : elles
//!   disent ce qui sera touché (cours quittés et rejoints, professeurs concernés) puis n'écrivent qu'avec
//!   `apercu: false`.
//! * **Droits** : `affectations.gerer` (`referentiel.gerer` la couvre) pour tout ; `groupes.composer` pour composer SON
//!   groupe d'activité ou de liste (jamais rétroactif) ; les autres natures ne se composent que par les gestionnaires.
//! * **Jour** : celui de l'établissement. Aucun jour UTC.

use chrono::{Duration, NaiveDate};
use ent_auth::Action;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::structure_etab::structure::aujourdhui;
use crate::structure_etab::{journaliser, net, publier};
use crate::structure_groupes::{
    groupes, poser_motif_annulation, tx, GroupeSaisie, GroupeVue, MODES_DEMI_GROUPES,
};
use crate::tenant_tx::TenantTx;

fn vrai() -> bool {
    true
}

fn dedoublonner(v: &[Uuid]) -> Vec<Uuid> {
    let mut vus = std::collections::HashSet::new();
    v.iter().copied().filter(|x| vus.insert(*x)).collect()
}

fn motif_code(m: Option<&str>) -> Result<Option<String>, EntDbError> {
    match net(m, 40)? {
        None => Ok(None),
        Some(v)
            if v.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') =>
        {
            Ok(Some(v))
        }
        Some(_) => Err(invalide()),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Refus {
    pub eleve_id: Uuid,
    /// `inconnu`, `non_scolarise`, `non_propose`, `dans_une_autre_classe`, `non_membre`.
    pub cause: String,
}

// ───────────────────────── Composition d'un groupe (STR-63, 65, 66) ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct CompositionSaisie {
    #[serde(default)]
    pub entrants: Vec<Uuid>,
    #[serde(default)]
    pub sortants: Vec<Uuid>,
    /// Défaut : aujourd'hui (jour de l'établissement). Une date passée est refusée au responsable d'atelier.
    pub date_effet: Option<NaiveDate>,
    /// Code court (minuscules, chiffres, `_`) : `rentree`, `recomposition_periode`, `arrivee`…
    pub motif_code: Option<String>,
    /// Vrai : calcule et ne touche à rien.
    #[serde(default)]
    pub apercu: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompositionResultat {
    pub applique: bool,
    pub groupe_id: Uuid,
    pub date_effet: NaiveDate,
    pub entrants: Vec<Uuid>,
    pub sortants: Vec<Uuid>,
    /// Entrants déjà membres à cette date : ignorés, jamais écrasés.
    pub deja_membres: Vec<Uuid>,
    pub refuses: Vec<Refus>,
    /// Entrants qui ne sont dans aucune classe d'origine du groupe (avertissement, pas un blocage).
    pub hors_classes_origine: Vec<Uuid>,
    pub effectif_apres: i64,
    pub capacite: Option<i32>,
    pub capacite_depassee: bool,
    /// `retroactif`, `capacite_depassee`, `hors_classes_origine`.
    pub avertissements: Vec<String>,
}

#[derive(FromRow)]
struct EleveEtat {
    id: Uuid,
    date_entree: Option<NaiveDate>,
    date_sortie: Option<NaiveDate>,
    fusionne: bool,
}

impl EleveEtat {
    fn scolarise_le(&self, d: NaiveDate) -> bool {
        !self.fusionne
            && self.date_entree.is_none_or(|e| e <= d)
            && self.date_sortie.is_none_or(|s| s > d)
    }
}

async fn etats_eleves(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    ids: &[Uuid],
) -> Result<Vec<EleveEtat>, EntDbError> {
    sqlx::query_as(
        "SELECT id, date_entree, date_sortie, fusionne_dans IS NOT NULL AS fusionne
           FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)",
    )
    .bind(etab)
    .bind(ids)
    .fetch_all(conn)
    .await
    .map_err(db)
}

impl PgEnt {
    /// STR-63 (entrées), STR-66 (sorties), STR-65 (lot) : modifie la composition d'un groupe à une date.
    /// Les doublons de la liste sont dédoublonnés avant calcul ; un élève déjà membre est ignoré et signalé, jamais
    /// écrasé ; la capacité est un avertissement, ou un refus si elle est bloquante (R-S15).
    pub async fn structure_composition_modifier(
        &self,
        a: &Acteur<'_>,
        groupe_id: Uuid,
        s: CompositionSaisie,
    ) -> Result<CompositionResultat, EntDbError> {
        let mut t = tx(self, a).await?;
        crate::structure_groupes::verrouiller_groupe(&mut t, a.etablissement_id, groupe_id).await?;
        let aujourd = aujourdhui(&mut t, a.etablissement_id).await?;
        let g = groupes::lire(&mut t, a.etablissement_id, aujourd, groupe_id)
            .await?
            .ok_or_else(introuvable)?;
        let gestion = self.exiger_composition(a, &mut t, &g, aujourd).await?;
        let res = self
            .composer_sur(&mut t, a, &g, gestion, &s, "composition", aujourd)
            .await?;
        if s.apercu {
            t.rollback().await.map_err(db)?;
        } else {
            t.commit().await.map_err(db)?;
            self.diffuser_apres_commit(&res.1);
        }
        Ok(res.0)
    }

    /// Cœur partagé : valide, calcule et (sauf aperçu) écrit la composition. Renvoie le résultat et les événements.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn composer_sur(
        &self,
        t: &mut TenantTx<'static>,
        a: &Acteur<'_>,
        g: &GroupeVue,
        gestion: bool,
        s: &CompositionSaisie,
        source: &str,
        aujourd: NaiveDate,
    ) -> Result<(CompositionResultat, Vec<ent_events::MetierEvent>), EntDbError> {
        let etab = a.etablissement_id;
        crate::structure_groupes::verrouiller_groupe(t, etab, g.id).await?;
        let date = s.date_effet.unwrap_or(aujourd);
        if !gestion && date < aujourd {
            return Err(invalide()); // un responsable d'atelier ne compose pas dans le passé
        }
        let motif = motif_code(s.motif_code.as_deref())?;
        let (debut, fin): (NaiveDate, NaiveDate) = sqlx::query_as(
            "SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(g.annee_id)
        .fetch_one(&mut **t)
        .await
        .map_err(db)?;
        if date < g.ouvert_le.max(debut) || date > fin || g.ferme_le.is_some_and(|f| date > f) {
            return Err(invalide());
        }
        let cloturee: bool = sqlx::query_scalar(
            "SELECT cloturee_le IS NOT NULL FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(g.annee_id)
        .fetch_one(&mut **t)
        .await
        .map_err(db)?;
        if cloturee {
            return Err(invalide()); // année clôturée : en lecture seule
        }
        let entrants = dedoublonner(&s.entrants);
        let sortants = dedoublonner(&s.sortants);
        if entrants.iter().any(|e| sortants.contains(e)) {
            return Err(invalide());
        }
        let mut refuses = Vec::new();
        // Entrants : connus, scolarisés à la date, pas déjà membres (affectation qui couvre la date ou la suit).
        let etats = etats_eleves(t, etab, &entrants).await?;
        let mut valides = Vec::new();
        let mut deja = Vec::new();
        for e in &entrants {
            match etats.iter().find(|x| x.id == *e) {
                None => refuses.push(Refus {
                    eleve_id: *e,
                    cause: "inconnu".into(),
                }),
                Some(x) if !x.scolarise_le(date) => refuses.push(Refus {
                    eleve_id: *e,
                    cause: "non_scolarise".into(),
                }),
                Some(_) => {
                    let membre: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2
                                         AND groupe_id = $3 AND (date_fin IS NULL OR date_fin >= $4))",
                    )
                    .bind(etab)
                    .bind(e)
                    .bind(g.id)
                    .bind(date)
                    .fetch_one(&mut **t)
                    .await
                    .map_err(db)?;
                    if membre {
                        deja.push(*e);
                    } else {
                        valides.push(*e);
                    }
                }
            }
        }
        // Sortants : membres à la date.
        let mut a_sortir: Vec<(Uuid, Uuid, NaiveDate)> = Vec::new(); // (affectation, élève, début)
        for e in &sortants {
            let r: Option<(Uuid, NaiveDate)> = sqlx::query_as(
                "SELECT id, date_debut FROM ent_affectation
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND groupe_id = $3
                    AND date_debut <= $4 AND (date_fin IS NULL OR date_fin >= $4)",
            )
            .bind(etab)
            .bind(e)
            .bind(g.id)
            .bind(date)
            .fetch_optional(&mut **t)
            .await
            .map_err(db)?;
            match r {
                Some((id, d)) => a_sortir.push((id, *e, d)),
                None => refuses.push(Refus {
                    eleve_id: *e,
                    cause: "non_membre".into(),
                }),
            }
        }
        let effectif_avant = groupes::membres(t, etab, date, g.id).await?.len() as i64;
        let effectif_apres = effectif_avant + valides.len() as i64 - a_sortir.len() as i64;
        let depassee = g.capacite.is_some_and(|c| effectif_apres > c as i64);
        if depassee && g.capacite_bloquante && !valides.is_empty() {
            return Err(conflit()); // capacité de sécurité (R-S15) : refus, rien n'est écrit
        }
        let mut hors = Vec::new();
        if !g.classes.is_empty() {
            for e in &valides {
                let dans: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2
                                     AND classe_id = ANY($3) AND date_debut <= $4 AND (date_fin IS NULL OR date_fin >= $4))",
                )
                .bind(etab)
                .bind(e)
                .bind(&g.classes)
                .bind(date)
                .fetch_one(&mut **t)
                .await
                .map_err(db)?;
                if !dans {
                    hors.push(*e);
                }
            }
        }
        let mut avertissements = Vec::new();
        if date < aujourd {
            avertissements.push("retroactif".to_string());
        }
        if depassee {
            avertissements.push("capacite_depassee".into());
        }
        if !hors.is_empty() {
            avertissements.push("hors_classes_origine".into());
        }
        let mut events = Vec::new();
        if !s.apercu && (!valides.is_empty() || !a_sortir.is_empty()) {
            let veille = date - Duration::days(1);
            poser_motif_annulation(t, motif.as_deref().unwrap_or("retrait"), Some(a.user_id))
                .await?;
            for (id, _, d_debut) in &a_sortir {
                if *d_debut >= date {
                    sqlx::query(
                        "DELETE FROM ent_affectation WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(id)
                    .execute(&mut **t)
                    .await
                    .map_err(db)?;
                } else {
                    sqlx::query("UPDATE ent_affectation SET date_fin = $3 WHERE etablissement_id = $1 AND id = $2")
                        .bind(etab)
                        .bind(id)
                        .bind(veille)
                        .execute(&mut **t)
                        .await
                        .map_err(db)?;
                }
            }
            for e in &valides {
                sqlx::query(
                    "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut,
                                                   source, motif_code, decide_par)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(g.annee_id)
                .bind(e)
                .bind(g.id)
                .bind(date)
                .bind("saisie")
                .bind(motif.clone().unwrap_or_else(|| source.to_string()))
                .bind(a.user_id)
                .execute(&mut **t)
                .await
                .map_err(db)?;
            }
            let sorties: Vec<Uuid> = a_sortir.iter().map(|x| x.1).collect();
            journaliser(
                t,
                a,
                "composition",
                &g.id.to_string(),
                "modifiee",
                Some(json!({ "effectif": effectif_avant })),
                Some(json!({ "effectif": effectif_apres, "entrants": valides, "sortants": sorties, "date": date })),
            )
            .await?;
            events.push(
                publier(
                    t,
                    a,
                    "referentiel.composition_modifiee",
                    g.id,
                    json!({
                        "groupe_id": g.id, "entrants": valides, "sortants": sorties, "date": date,
                        "motif": motif, "retroactif": date < aujourd,
                    }),
                )
                .await?,
            );
        }
        let sorties: Vec<Uuid> = a_sortir.iter().map(|x| x.1).collect();
        let res = CompositionResultat {
            applique: !s.apercu,
            groupe_id: g.id,
            date_effet: date,
            entrants: valides,
            sortants: sorties,
            deja_membres: deja,
            refuses,
            hors_classes_origine: hors,
            effectif_apres,
            capacite: g.capacite,
            capacite_depassee: depassee,
            avertissements,
        };
        Ok((res, events))
    }
}

// ───────────────────────── Règle de composition (STR-64) ─────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct PropositionsGroupe {
    pub groupe_id: Uuid,
    pub jour: NaiveDate,
    /// Faux : le groupe n'a ni offre ni classe d'origine, la règle ne peut rien proposer.
    pub regle_complete: bool,
    /// Élèves des classes d'origine qui suivent l'enseignement du groupe sans être dans un de ses groupes.
    pub entrants: Vec<Uuid>,
    /// Membres qui ne suivent plus l'enseignement du groupe (abandon, retrait).
    pub sortants: Vec<Uuid>,
    /// Membres qui ne sont dans aucune classe d'origine (avertissement).
    pub hors_classes_origine: Vec<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationRegle {
    #[serde(default)]
    pub entrants: Vec<Uuid>,
    #[serde(default)]
    pub sortants: Vec<Uuid>,
    pub date_effet: Option<NaiveDate>,
}

/// Calcul pur de la règle (une seule implémentation) : classes d'origine × enseignement suivi.
async fn proposer(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    g: &GroupeVue,
    jour: NaiveDate,
) -> Result<PropositionsGroupe, EntDbError> {
    let complete = g.offre_id.is_some() && !g.classes.is_empty();
    let (mut entrants, mut sortants) = (Vec::new(), Vec::new());
    if let (Some(offre), true) = (g.offre_id, complete) {
        entrants = sqlx::query_scalar(
            "SELECT DISTINCT ac.eleve_id
               FROM ent_affectation ac
               JOIN ent_eleve e ON e.id = ac.eleve_id AND e.etablissement_id = ac.etablissement_id
               JOIN ent_eleve_enseignement ee ON ee.etablissement_id = ac.etablissement_id AND ee.eleve_id = ac.eleve_id
                AND ee.offre_id = $3 AND ent_enseignement_suivi_le(ee, $4)
              WHERE ac.etablissement_id = $1 AND ac.classe_id = ANY($2)
                AND ac.date_debut <= $4 AND (ac.date_fin IS NULL OR ac.date_fin >= $4)
                AND (e.date_sortie IS NULL OR e.date_sortie > $4) AND e.fusionne_dans IS NULL
                AND NOT EXISTS (
                      SELECT 1 FROM ent_affectation m
                        JOIN ent_offre_groupe og ON og.groupe_id = m.groupe_id AND og.etablissement_id = m.etablissement_id
                       WHERE m.etablissement_id = $1 AND m.eleve_id = ac.eleve_id AND og.offre_id = $3
                         AND (m.date_fin IS NULL OR m.date_fin >= $4))
              ORDER BY 1",
        )
        .bind(etab)
        .bind(&g.classes)
        .bind(offre)
        .bind(jour)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        sortants = sqlx::query_scalar(
            "SELECT m.eleve_id FROM ent_groupe_membres_jour($1, $3, $2) m
              WHERE NOT EXISTS (SELECT 1 FROM ent_eleve_enseignement ee
                                 WHERE ee.etablissement_id = $1 AND ee.eleve_id = m.eleve_id AND ee.offre_id = $4
                                   AND ent_enseignement_suivi_le(ee, $3))
              ORDER BY 1",
        )
        .bind(etab)
        .bind(g.id)
        .bind(jour)
        .bind(offre)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
    }
    let hors: Vec<Uuid> = if g.classes.is_empty() {
        Vec::new()
    } else {
        sqlx::query_scalar(
            "SELECT m.eleve_id FROM ent_groupe_membres_jour($1, $3, $2) m
              WHERE NOT EXISTS (SELECT 1 FROM ent_affectation ac
                                 WHERE ac.etablissement_id = $1 AND ac.eleve_id = m.eleve_id AND ac.classe_id = ANY($4)
                                   AND ac.date_debut <= $3 AND (ac.date_fin IS NULL OR ac.date_fin >= $3))
              ORDER BY 1",
        )
        .bind(etab)
        .bind(g.id)
        .bind(jour)
        .bind(&g.classes)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?
    };
    Ok(PropositionsGroupe {
        groupe_id: g.id,
        jour,
        regle_complete: complete,
        entrants,
        sortants,
        hors_classes_origine: hors,
    })
}

impl PgEnt {
    /// STR-64 : propositions de la règle de composition (lecture seule, rien n'est écrit).
    pub async fn structure_groupe_propositions(
        &self,
        a: &Acteur<'_>,
        groupe_id: Uuid,
        jour: Option<NaiveDate>,
    ) -> Result<PropositionsGroupe, EntDbError> {
        self.exiger_structure(a, Action::AffectationsGerer).await?;
        let mut t = tx(self, a).await?;
        let aujourd = aujourdhui(&mut t, a.etablissement_id).await?;
        let jour = jour.unwrap_or(aujourd);
        let g = groupes::lire(&mut t, a.etablissement_id, jour, groupe_id)
            .await?
            .ok_or_else(introuvable)?;
        let p = proposer(&mut t, a.etablissement_id, &g, jour).await?;
        t.commit().await.map_err(db)?;
        Ok(p)
    }

    /// STR-64 : applique, parmi les propositions, celles que la personne a validées. Le serveur **recalcule** à la
    /// date d'effet et n'applique que l'intersection ; tout autre identifiant est refusé (`non_propose`).
    pub async fn structure_groupe_regle_appliquer(
        &self,
        a: &Acteur<'_>,
        groupe_id: Uuid,
        s: ApplicationRegle,
    ) -> Result<CompositionResultat, EntDbError> {
        self.exiger_structure(a, Action::AffectationsGerer).await?;
        let mut t = tx(self, a).await?;
        let aujourd = aujourdhui(&mut t, a.etablissement_id).await?;
        let date = s.date_effet.unwrap_or(aujourd);
        let g = groupes::lire(&mut t, a.etablissement_id, date, groupe_id)
            .await?
            .ok_or_else(introuvable)?;
        let p = proposer(&mut t, a.etablissement_id, &g, date).await?;
        let entrants: Vec<Uuid> = dedoublonner(&s.entrants)
            .into_iter()
            .filter(|e| p.entrants.contains(e))
            .collect();
        let sortants: Vec<Uuid> = dedoublonner(&s.sortants)
            .into_iter()
            .filter(|e| p.sortants.contains(e))
            .collect();
        let non_proposes: Vec<Refus> = dedoublonner(&s.entrants)
            .into_iter()
            .filter(|e| !entrants.contains(e))
            .chain(
                dedoublonner(&s.sortants)
                    .into_iter()
                    .filter(|e| !sortants.contains(e)),
            )
            .map(|e| Refus {
                eleve_id: e,
                cause: "non_propose".into(),
            })
            .collect();
        let saisie = CompositionSaisie {
            entrants,
            sortants,
            date_effet: Some(date),
            motif_code: Some("regle".into()),
            apercu: false,
        };
        let (mut res, events) = self
            .composer_sur(&mut t, a, &g, true, &saisie, "regle", aujourd)
            .await?;
        res.refuses.extend(non_proposes);
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&events);
        Ok(res)
    }
}

// ───────────────────────── Changement de classe (STR-60, 61) ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ChangementClasseSaisie {
    pub eleve_id: Uuid,
    pub classe_id: Uuid,
    /// Défaut : aujourd'hui.
    pub date_effet: Option<NaiveDate>,
    /// Aperçu par défaut : `apercu: false` pour écrire.
    #[serde(default = "vrai")]
    pub apercu: bool,
    /// Groupes à rejoindre, choisis parmi `groupes_proposes` et `groupes_a_choisir` de l'aperçu.
    #[serde(default)]
    pub entrer_dans: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChoixGroupe {
    /// `None` : demi-groupes de la nouvelle classe (sans enseignement porté).
    pub offre_id: Option<Uuid>,
    pub groupes: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApercuChangementClasse {
    pub applique: bool,
    pub eleve_id: Uuid,
    pub date_effet: NaiveDate,
    pub classe_avant: Option<Uuid>,
    pub classe_apres: Uuid,
    pub groupes_quittes: Vec<Uuid>,
    pub groupes_conserves: Vec<Uuid>,
    /// Un seul groupe de la nouvelle classe porte l'enseignement suivi : proposé.
    pub groupes_proposes: Vec<Uuid>,
    /// Plusieurs groupes possibles (ou demi-groupes) : la personne choisit.
    pub groupes_a_choisir: Vec<ChoixGroupe>,
    pub groupes_rejoints: Vec<Uuid>,
    pub cours_quittes: Vec<Uuid>,
    pub cours_rejoints: Vec<Uuid>,
    /// Comptes des professeurs (services) des cours quittés et rejoints : à prévenir (socle 8).
    pub professeurs_concernes: Vec<Uuid>,
    /// `retroactif`.
    pub avertissements: Vec<String>,
}

impl PgEnt {
    /// STR-60 / STR-61 ★ : affecte un élève à une classe ou le change de classe, en une commande : affectation de
    /// classe (et période de scolarité, ELV-09) à la date, **groupes recalculés** (il quitte les groupes liés à son
    /// ancienne classe, garde ceux qui incluent aussi la nouvelle, se voit proposer ceux de la nouvelle classe selon
    /// ses enseignements suivis), aperçu des conséquences, événements pour les consommateurs. Les notes ne bougent pas :
    /// elles appartiennent à l'élève et au cours d'origine.
    pub async fn structure_eleve_classe_changer(
        &self,
        a: &Acteur<'_>,
        s: ChangementClasseSaisie,
    ) -> Result<ApercuChangementClasse, EntDbError> {
        self.exiger_structure(a, Action::AffectationsGerer).await?;
        let etab = a.etablissement_id;
        let mut t = self.ouvrir_tx_eleve(etab, s.eleve_id).await?;
        let aujourd = aujourdhui(&mut t, etab).await?;
        let date = s.date_effet.unwrap_or(aujourd);
        let classe: (Uuid, NaiveDate, NaiveDate, bool) = sqlx::query_as(
            "SELECT c.annee_id, y.date_debut, y.date_fin, c.archivee_le IS NOT NULL
               FROM ent_classe c JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.id = $2",
        )
        .bind(etab)
        .bind(s.classe_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let (annee, debut, fin, archivee) = classe;
        if archivee || date < debut || date > fin {
            return Err(invalide());
        }
        let etat = etats_eleves(&mut t, etab, &[s.eleve_id]).await?;
        if !etat.first().is_some_and(|e| e.scolarise_le(date)) {
            return Err(conflit()); // l'élève n'est pas scolarisé à cette date
        }
        let avant: Option<(Uuid, NaiveDate)> = sqlx::query_as(
            "SELECT classe_id, date_debut FROM ent_affectation
              WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_id = $3 AND classe_id IS NOT NULL
                AND date_debut <= $4 AND (date_fin IS NULL OR date_fin >= $4)
              ORDER BY date_debut DESC LIMIT 1",
        )
        .bind(etab)
        .bind(s.eleve_id)
        .bind(annee)
        .bind(date)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        if avant.is_some_and(|(c, _)| c == s.classe_id) {
            return Err(conflit()); // déjà dans cette classe à cette date
        }
        let classe_avant = avant.map(|x| x.0);
        // Groupes de l'élève à la date : liés à l'ancienne classe et pas à la nouvelle → quittés.
        let groupes_eleve = groupes::groupes_de_eleve(&mut t, etab, date, s.eleve_id).await?;
        let (mut quittes, mut conserves) = (Vec::new(), Vec::new());
        for g in &groupes_eleve {
            let origine = groupes::classes_du_groupe(&mut t, etab, *g).await?;
            let lie_avant = classe_avant.is_some_and(|c| origine.contains(&c));
            if lie_avant && !origine.contains(&s.classe_id) {
                quittes.push(*g);
            } else {
                conserves.push(*g);
            }
        }
        // Propositions : groupes ouverts de la nouvelle classe selon les enseignements suivis.
        let candidats: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT g.id, og.offre_id
               FROM ent_groupe g
               JOIN ent_groupe_classe gc ON gc.groupe_id = g.id AND gc.etablissement_id = g.etablissement_id
               LEFT JOIN ent_offre_groupe og ON og.groupe_id = g.id AND og.etablissement_id = g.etablissement_id
              WHERE g.etablissement_id = $1 AND gc.classe_id = $2 AND g.nature = 'enseignement'
                AND g.ouvert_le <= $3 AND (g.ferme_le IS NULL OR g.ferme_le >= $3)
              ORDER BY g.code, g.id",
        )
        .bind(etab)
        .bind(s.classe_id)
        .bind(date)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let suivies: Vec<Uuid> = sqlx::query_scalar(
            "SELECT e.offre_id FROM ent_eleve_enseignement e
              WHERE e.etablissement_id = $1 AND e.eleve_id = $2 AND ent_enseignement_suivi_le(e, $3)",
        )
        .bind(etab)
        .bind(s.eleve_id)
        .bind(date)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut groupes_proposes = Vec::new();
        let mut a_choisir: Vec<ChoixGroupe> = Vec::new();
        let mut offres_vues: Vec<Uuid> = Vec::new();
        for (g, offre) in &candidats {
            if conserves.contains(g) {
                continue;
            }
            match offre {
                Some(o) if suivies.contains(o) => {
                    if offres_vues.contains(o) {
                        continue;
                    }
                    offres_vues.push(*o);
                    // L'offre est-elle déjà couverte par un groupe conservé ?
                    let couverte: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM ent_offre_groupe WHERE etablissement_id = $1 AND offre_id = $2
                                         AND groupe_id = ANY($3))",
                    )
                    .bind(etab)
                    .bind(o)
                    .bind(&conserves)
                    .fetch_one(&mut *t)
                    .await
                    .map_err(db)?;
                    if couverte {
                        continue;
                    }
                    let meme_offre: Vec<Uuid> = candidats
                        .iter()
                        .filter(|c| c.1 == Some(*o) && !conserves.contains(&c.0))
                        .map(|c| c.0)
                        .collect();
                    if meme_offre.len() == 1 {
                        groupes_proposes.push(meme_offre[0]);
                    } else {
                        a_choisir.push(ChoixGroupe {
                            offre_id: Some(*o),
                            groupes: meme_offre,
                        });
                    }
                }
                Some(_) => {}
                None => {
                    // Demi-groupes (sans enseignement porté) : jamais proposés d'office.
                    match a_choisir.iter_mut().find(|c| c.offre_id.is_none()) {
                        Some(c) => c.groupes.push(*g),
                        None => a_choisir.push(ChoixGroupe {
                            offre_id: None,
                            groupes: vec![*g],
                        }),
                    }
                }
            }
        }
        // Les groupes choisis doivent venir de l'aperçu (proposés ou à choisir) : rien d'autre.
        let permis: Vec<Uuid> = groupes_proposes
            .iter()
            .copied()
            .chain(a_choisir.iter().flat_map(|c| c.groupes.iter().copied()))
            .collect();
        let entrer = dedoublonner(&s.entrer_dans);
        if entrer.iter().any(|g| !permis.contains(g)) {
            return Err(invalide());
        }
        // Au plus un groupe par enseignement (ou un seul demi-groupe) : jamais les deux demi-groupes, ni deux groupes
        // d'une même offre.
        for c in &a_choisir {
            if c.groupes.iter().filter(|g| entrer.contains(g)).count() > 1 {
                return Err(invalide());
            }
        }
        let rejoints_hyp: Vec<Uuid> = if s.apercu {
            groupes_proposes.clone()
        } else {
            entrer.clone()
        };
        // Cours quittés et rejoints, professeurs concernés.
        let cours_quittes: Vec<Uuid> = sqlx::query_scalar(
            "SELECT c.id FROM ent_cours c
              WHERE c.etablissement_id = $1 AND c.ouvert_le <= $2 AND (c.ferme_le IS NULL OR c.ferme_le >= $2)
                AND (c.classe_id = $3 OR c.groupe_id = ANY($4)) ORDER BY c.id",
        )
        .bind(etab)
        .bind(date)
        .bind(classe_avant)
        .bind(&quittes)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let cours_rejoints: Vec<Uuid> = sqlx::query_scalar(
            "SELECT c.id FROM ent_cours c
              WHERE c.etablissement_id = $1 AND c.ouvert_le <= $2 AND (c.ferme_le IS NULL OR c.ferme_le >= $2)
                AND (c.classe_id = $3 OR c.groupe_id = ANY($4)) ORDER BY c.id",
        )
        .bind(etab)
        .bind(date)
        .bind(s.classe_id)
        .bind(&rejoints_hyp)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let tous_cours: Vec<Uuid> = cours_quittes
            .iter()
            .chain(cours_rejoints.iter())
            .copied()
            .collect();
        let profs: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT prof_user_id FROM ent_service_enseignement
              WHERE etablissement_id = $1 AND cours_id = ANY($2) AND prof_user_id IS NOT NULL ORDER BY 1",
        )
        .bind(etab)
        .bind(&tous_cours)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut res = ApercuChangementClasse {
            applique: false,
            eleve_id: s.eleve_id,
            date_effet: date,
            classe_avant,
            classe_apres: s.classe_id,
            groupes_quittes: quittes.clone(),
            groupes_conserves: conserves,
            groupes_proposes,
            groupes_a_choisir: a_choisir,
            groupes_rejoints: rejoints_hyp,
            cours_quittes,
            cours_rejoints,
            professeurs_concernes: profs,
            avertissements: if date < aujourd {
                vec!["retroactif".into()]
            } else {
                Vec::new()
            },
        };
        if s.apercu {
            t.rollback().await.map_err(db)?;
            return Ok(res);
        }
        // Écriture : le cœur d'ELV-09 (période de scolarité + affectation de classe), puis les groupes.
        self.changer_classe_sur(
            &mut t,
            crate::scolarite::ChangementClasse {
                etablissement_id: etab,
                eleve_id: s.eleve_id,
                classe_id: s.classe_id,
                date_effet: date,
                acteur: a.user_id,
            },
        )
        .await?;
        let mut events = Vec::new();
        let veille = date - Duration::days(1);
        poser_motif_annulation(&mut t, "changement_classe", Some(a.user_id)).await?;
        for g in &quittes {
            // Affectation de groupe qui couvre la date : close la veille (ou annulée si elle commence ce jour-là).
            sqlx::query(
                "DELETE FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2 AND groupe_id = $3
                    AND date_debut >= $4",
            )
            .bind(etab)
            .bind(s.eleve_id)
            .bind(g)
            .bind(date)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            sqlx::query(
                "UPDATE ent_affectation SET date_fin = $4
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND groupe_id = $3
                    AND date_debut <= $4 AND (date_fin IS NULL OR date_fin > $4)",
            )
            .bind(etab)
            .bind(s.eleve_id)
            .bind(g)
            .bind(veille)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            events.push(
                publier(
                    &mut t,
                    a,
                    "referentiel.composition_modifiee",
                    *g,
                    json!({ "groupe_id": g, "entrants": [], "sortants": [s.eleve_id], "date": date,
                            "motif": "changement_classe" }),
                )
                .await?,
            );
        }
        let mut entrer_tries = entrer.clone();
        entrer_tries.sort();
        for g in &entrer_tries {
            crate::structure_groupes::verrouiller_groupe(&mut t, etab, *g).await?;
            let (cap, bloquante): (Option<i32>, bool) = sqlx::query_as(
                "SELECT capacite, capacite_bloquante FROM ent_groupe WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(g)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
            if let (Some(c), true) = (cap, bloquante) {
                let n = groupes::membres(&mut t, etab, date, *g).await?.len() as i64;
                if n >= c as i64 {
                    return Err(conflit()); // capacité de sécurité atteinte (R-S15)
                }
            }
        }
        for g in &entrer {
            let annee_g: Option<(Uuid, NaiveDate)> = sqlx::query_as(
                "SELECT annee_id, ouvert_le FROM ent_groupe WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(g)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?;
            let (an_g, ouvert) = annee_g.ok_or_else(introuvable)?;
            let debut_g = date.max(ouvert);
            sqlx::query(
                "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut,
                                               source, motif_code, decide_par)
                 VALUES ($1,$2,$3,$4,$5,$6,'saisie','changement_classe',$7)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(an_g)
            .bind(s.eleve_id)
            .bind(g)
            .bind(debut_g)
            .bind(a.user_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            events.push(
                publier(
                    &mut t,
                    a,
                    "referentiel.composition_modifiee",
                    *g,
                    json!({ "groupe_id": g, "entrants": [s.eleve_id], "sortants": [], "date": date,
                            "motif": "changement_classe" }),
                )
                .await?,
            );
        }
        journaliser(
            &mut t,
            a,
            "affectation",
            &s.eleve_id.to_string(),
            "classe_changee",
            Some(json!({ "classe_id": classe_avant })),
            Some(json!({ "classe_id": s.classe_id, "date": date, "groupes_quittes": quittes, "groupes_rejoints": entrer })),
        )
        .await?;
        events.push(
            publier(
                &mut t,
                a,
                "referentiel.affectation_changee",
                s.eleve_id,
                json!({ "eleve_id": s.eleve_id, "classe_avant": classe_avant, "classe_apres": s.classe_id, "date": date }),
            )
            .await?,
        );
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&events);
        res.applique = true;
        res.groupes_rejoints = entrer;
        Ok(res)
    }
}

// ───────────────────────── Affectation en lot (STR-65) ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct LotSaisie {
    pub classe_id: Uuid,
    pub eleves: Vec<Uuid>,
    pub date_effet: Option<NaiveDate>,
    #[serde(default = "vrai")]
    pub apercu: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LotResultat {
    pub applique: bool,
    pub classe_id: Uuid,
    pub date_effet: NaiveDate,
    /// Élèves affectés (ou qui le seraient).
    pub affectes: Vec<Uuid>,
    pub deja_dans_la_classe: Vec<Uuid>,
    /// `inconnu`, `non_scolarise`, `dans_une_autre_classe` : jamais écrasés (STR-61 pour les déplacer).
    pub refuses: Vec<Refus>,
    /// Doublons retirés de la liste avant calcul.
    pub doublons_retires: usize,
}

impl PgEnt {
    /// STR-65 ★ : affecte une liste d'élèves à une classe (rentrée, import). Aperçu exact (les doublons du lot sont
    /// retirés avant calcul) ; un élève déjà dans une autre classe n'est **jamais déplacé** par un lot.
    pub async fn structure_affectations_lot(
        &self,
        a: &Acteur<'_>,
        s: LotSaisie,
    ) -> Result<LotResultat, EntDbError> {
        self.exiger_structure(a, Action::AffectationsGerer).await?;
        let etab = a.etablissement_id;
        if s.eleves.is_empty() || s.eleves.len() > 2000 {
            return Err(invalide());
        }
        let eleves = dedoublonner(&s.eleves);
        let doublons = s.eleves.len() - eleves.len();
        let mut t = tx(self, a).await?;
        let aujourd = aujourdhui(&mut t, etab).await?;
        let date = s.date_effet.unwrap_or(aujourd);
        let (annee, debut, fin, archivee): (Uuid, NaiveDate, NaiveDate, bool) = sqlx::query_as(
            "SELECT c.annee_id, y.date_debut, y.date_fin, c.archivee_le IS NOT NULL
               FROM ent_classe c JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.id = $2",
        )
        .bind(etab)
        .bind(s.classe_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if archivee || date < debut || date > fin {
            return Err(invalide());
        }
        // Verrou des élèves du lot, dans un ordre stable (pas d'interblocage entre deux lots).
        let mut ordonnes = eleves.clone();
        ordonnes.sort();
        sqlx::query("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2) ORDER BY id FOR UPDATE")
            .bind(etab)
            .bind(&ordonnes)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
        let etats = etats_eleves(&mut t, etab, &eleves).await?;
        let (mut affectes, mut deja, mut refuses) = (Vec::new(), Vec::new(), Vec::new());
        for e in &eleves {
            match etats.iter().find(|x| x.id == *e) {
                None => refuses.push(Refus {
                    eleve_id: *e,
                    cause: "inconnu".into(),
                }),
                Some(x) if !x.scolarise_le(date) => refuses.push(Refus {
                    eleve_id: *e,
                    cause: "non_scolarise".into(),
                }),
                Some(_) => {
                    let actuelle: Option<Uuid> = sqlx::query_scalar(
                        "SELECT classe_id FROM ent_affectation
                          WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_id = $3 AND classe_id IS NOT NULL
                            AND (date_fin IS NULL OR date_fin >= $4)
                          ORDER BY date_debut DESC LIMIT 1",
                    )
                    .bind(etab)
                    .bind(e)
                    .bind(annee)
                    .bind(date)
                    .fetch_optional(&mut *t)
                    .await
                    .map_err(db)?;
                    match actuelle {
                        Some(c) if c == s.classe_id => deja.push(*e),
                        Some(_) => refuses.push(Refus {
                            eleve_id: *e,
                            cause: "dans_une_autre_classe".into(),
                        }),
                        None => affectes.push(*e),
                    }
                }
            }
        }
        if s.apercu {
            t.rollback().await.map_err(db)?;
        } else {
            for e in &affectes {
                self.changer_classe_sur(
                    &mut t,
                    crate::scolarite::ChangementClasse {
                        etablissement_id: etab,
                        eleve_id: *e,
                        classe_id: s.classe_id,
                        date_effet: date,
                        acteur: a.user_id,
                    },
                )
                .await?;
            }
            let mut events = Vec::new();
            if !affectes.is_empty() {
                journaliser(
                    &mut t,
                    a,
                    "affectation",
                    &s.classe_id.to_string(),
                    "lot",
                    None,
                    Some(json!({ "nombre": affectes.len(), "date": date })),
                )
                .await?;
                events.push(
                    publier(
                        &mut t,
                        a,
                        "referentiel.affectations_lot",
                        s.classe_id,
                        json!({ "annee_id": annee, "classe_id": s.classe_id, "nombre": affectes.len(), "date": date }),
                    )
                    .await?,
                );
            }
            t.commit().await.map_err(db)?;
            self.diffuser_apres_commit(&events);
        }
        Ok(LotResultat {
            applique: !s.apercu,
            classe_id: s.classe_id,
            date_effet: date,
            affectes,
            deja_dans_la_classe: deja,
            refuses,
            doublons_retires: doublons,
        })
    }
}

// ───────────────────────── Demi-groupes (STR-52) ─────────────────────────

impl PgEnt {
    /// STR-52 : crée les deux demi-groupes d'une classe (alphabétique, alterné, équilibré filles / garçons, manuel).
    /// Contrôle : chaque élève de la classe est dans exactement un des deux.
    pub async fn structure_demi_groupes_creer(
        &self,
        a: &Acteur<'_>,
        s: DemiGroupesSaisie,
    ) -> Result<DemiGroupesResultat, EntDbError> {
        self.exiger_structure(a, Action::StructureGerer).await?;
        self.exiger_structure(a, Action::AffectationsGerer).await?;
        let etab = a.etablissement_id;
        if !MODES_DEMI_GROUPES.contains(&s.mode.as_str()) {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let aujourd = aujourdhui(&mut t, etab).await?;
        let date = s.date_effet.unwrap_or(aujourd);
        let (code, annee, debut, fin): (String, Uuid, NaiveDate, NaiveDate) = sqlx::query_as(
            "SELECT c.code, c.annee_id, y.date_debut, y.date_fin
               FROM ent_classe c JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.id = $2 AND c.archivee_le IS NULL",
        )
        .bind(etab)
        .bind(s.classe_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let date = date.clamp(debut, fin);
        #[derive(FromRow)]
        struct E {
            id: Uuid,
            sexe: Option<String>,
        }
        let mut membres: Vec<E> = sqlx::query_as(
            "SELECT e.id, e.sexe FROM ent_affectation ac
               JOIN ent_eleve e ON e.id = ac.eleve_id AND e.etablissement_id = ac.etablissement_id
              WHERE ac.etablissement_id = $1 AND ac.classe_id = $2
                AND ac.date_debut <= $3 AND (ac.date_fin IS NULL OR ac.date_fin >= $3)
                AND (e.date_sortie IS NULL OR e.date_sortie > $3) AND e.fusionne_dans IS NULL
              ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(s.classe_id)
        .bind(date)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        if membres.len() < 2 {
            return Err(invalide());
        }
        let (mut a_ids, mut b_ids): (Vec<Uuid>, Vec<Uuid>) = (Vec::new(), Vec::new());
        match s.mode.as_str() {
            "alphabetique" => {
                let moitie = membres.len().div_ceil(2);
                for (i, m) in membres.iter().enumerate() {
                    if i < moitie {
                        a_ids.push(m.id)
                    } else {
                        b_ids.push(m.id)
                    }
                }
            }
            "alterne" => {
                for (i, m) in membres.iter().enumerate() {
                    if i % 2 == 0 {
                        a_ids.push(m.id)
                    } else {
                        b_ids.push(m.id)
                    }
                }
            }
            "equilibre" => {
                // Chaque sexe est réparti à parts égales ; le demi-groupe le moins fourni reçoit le premier.
                membres.sort_by(|x, y| x.sexe.cmp(&y.sexe));
                let mut debut_bucket = 0;
                while debut_bucket < membres.len() {
                    let sexe = membres[debut_bucket].sexe.clone();
                    let fin_bucket = membres[debut_bucket..]
                        .iter()
                        .position(|m| m.sexe != sexe)
                        .map_or(membres.len(), |p| debut_bucket + p);
                    for m in &membres[debut_bucket..fin_bucket] {
                        if a_ids.len() <= b_ids.len() {
                            a_ids.push(m.id)
                        } else {
                            b_ids.push(m.id)
                        }
                    }
                    debut_bucket = fin_bucket;
                }
            }
            _ => {
                let eleves_b = dedoublonner(&s.eleves_b);
                if eleves_b.iter().any(|b| !membres.iter().any(|m| m.id == *b)) {
                    return Err(invalide());
                }
                for m in &membres {
                    if eleves_b.contains(&m.id) {
                        b_ids.push(m.id)
                    } else {
                        a_ids.push(m.id)
                    }
                }
                if a_ids.is_empty() || b_ids.is_empty() {
                    return Err(invalide());
                }
            }
        }
        let mk = |suffixe: &str, code_saisi: &Option<String>| GroupeSaisie {
            code: code_saisi
                .clone()
                .unwrap_or_else(|| format!("{code}-{suffixe}")),
            libelle: None,
            nature: "enseignement".into(),
            classes: vec![s.classe_id],
            offre_id: None,
            capacite: None,
            capacite_bloquante: s.capacite_bloquante.unwrap_or(false),
            annee_id: Some(annee),
            ouvert_le: None,
            ouvre_droits: None,
        };
        let (ga, ev_a) = self
            .creer_groupe_sur(&mut t, a, &mk("A", &s.code_a), "groupe_cree")
            .await?;
        let (gb, ev_b) = self
            .creer_groupe_sur(&mut t, a, &mk("B", &s.code_b), "groupe_cree")
            .await?;
        let mut events = vec![ev_a, ev_b];
        for (g, ids) in [(&ga, &a_ids), (&gb, &b_ids)] {
            for e in ids {
                sqlx::query(
                    "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut,
                                                   source, motif_code, decide_par)
                     VALUES ($1,$2,$3,$4,$5,$6,'saisie','demi_groupes',$7)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(annee)
                .bind(e)
                .bind(g.id)
                .bind(date)
                .bind(a.user_id)
                .execute(&mut *t)
                .await
                .map_err(db)?;
            }
            events.push(
                publier(
                    &mut t,
                    a,
                    "referentiel.composition_modifiee",
                    g.id,
                    json!({ "groupe_id": g.id, "entrants": ids, "sortants": [], "date": date, "motif": "demi_groupes" }),
                )
                .await?,
            );
        }
        // Contrôle : les deux demi-groupes couvrent la classe (chacun dans exactement un).
        let couvre = a_ids.len() + b_ids.len() == membres.len()
            && a_ids.iter().all(|x| !b_ids.contains(x))
            && membres
                .iter()
                .all(|m| a_ids.contains(&m.id) || b_ids.contains(&m.id));
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&events);
        Ok(DemiGroupesResultat {
            groupe_a: ga.id,
            groupe_b: gb.id,
            eleves_a: a_ids,
            eleves_b: b_ids,
            couvre_la_classe: couvre,
        })
    }
}

pub use crate::structure_groupes::{DemiGroupesResultat, DemiGroupesSaisie};

// ───────────────────────── Dédoublement (STR-103) ─────────────────────────

/// Cours ouvert d'un groupe à dupliquer : (id, libellé, matière, offre, heures hebdomadaires, semaines).
type CoursADupliquer = (
    Uuid,
    String,
    Option<Uuid>,
    Option<Uuid>,
    Option<f64>,
    Option<String>,
);

#[derive(Debug, Clone, Deserialize)]
pub struct DedoublementSaisie {
    pub code_b: Option<String>,
    /// `alphabetique`, `alterne`, `manuel`.
    pub repartition: String,
    #[serde(default)]
    pub eleves_b: Vec<Uuid>,
    pub date_effet: Option<NaiveDate>,
    #[serde(default = "vrai")]
    pub apercu: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DedoublementResultat {
    pub applique: bool,
    pub groupe_source: Uuid,
    pub groupe_b: Option<Uuid>,
    pub date_effet: NaiveDate,
    pub reste_dans_a: Vec<Uuid>,
    pub passent_dans_b: Vec<Uuid>,
    /// Cours de A dupliqués pour B (un poste vacant par cours : service à pourvoir, socle 6).
    pub cours_dupliques: Vec<Uuid>,
    pub postes_a_pourvoir: Vec<Uuid>,
}

impl PgEnt {
    /// STR-103 ★ : dédouble un groupe (ex. 36 élèves d'espagnol) en deux. Le groupe B reprend la nature, les classes
    /// d'origine, l'enseignement et la capacité de A ; la moitié des élèves y passe à la date ; les cours ouverts de A
    /// sont dupliqués pour B (postes vacants : services à pourvoir, signalés au socle 6).
    pub async fn structure_groupe_dedoubler(
        &self,
        a: &Acteur<'_>,
        groupe_id: Uuid,
        s: DedoublementSaisie,
    ) -> Result<DedoublementResultat, EntDbError> {
        self.exiger_structure(a, Action::StructureGerer).await?;
        self.exiger_structure(a, Action::AffectationsGerer).await?;
        let etab = a.etablissement_id;
        if !matches!(
            s.repartition.as_str(),
            "alphabetique" | "alterne" | "manuel"
        ) {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let aujourd = aujourdhui(&mut t, etab).await?;
        let date = s.date_effet.unwrap_or(aujourd);
        crate::structure_groupes::verrouiller_groupe(&mut t, etab, groupe_id).await?;
        let g = groupes::lire(&mut t, etab, date, groupe_id)
            .await?
            .ok_or_else(introuvable)?;
        if g.ferme_le.is_some() || g.nature != "enseignement" {
            return Err(invalide());
        }
        let membres: Vec<Uuid> = sqlx::query_scalar(
            "SELECT m.eleve_id FROM ent_groupe_membres_jour($1, $2, $3) m
               JOIN ent_eleve e ON e.id = m.eleve_id AND e.etablissement_id = $1
              ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(etab)
        .bind(date)
        .bind(groupe_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        if membres.len() < 2 {
            return Err(invalide());
        }
        let b_ids: Vec<Uuid> = match s.repartition.as_str() {
            "alphabetique" => membres
                .iter()
                .skip(membres.len().div_ceil(2))
                .copied()
                .collect(),
            "alterne" => membres
                .iter()
                .enumerate()
                .filter(|(i, _)| i % 2 == 1)
                .map(|(_, m)| *m)
                .collect(),
            _ => {
                let b = dedoublonner(&s.eleves_b);
                if b.is_empty()
                    || b.len() >= membres.len()
                    || b.iter().any(|x| !membres.contains(x))
                {
                    return Err(invalide());
                }
                b
            }
        };
        let a_ids: Vec<Uuid> = membres
            .iter()
            .filter(|m| !b_ids.contains(m))
            .copied()
            .collect();
        let cours_a: Vec<CoursADupliquer> = sqlx::query_as(
            "SELECT c.id, c.libelle, c.matiere_id, c.offre_id, c.heures_hebdo::float8, c.semaines FROM ent_cours c
              WHERE c.etablissement_id = $1 AND c.groupe_id = $2 AND c.ouvert_le <= $3
                AND (c.ferme_le IS NULL OR c.ferme_le >= $3) ORDER BY c.id",
        )
        .bind(etab)
        .bind(groupe_id)
        .bind(date)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let mut res = DedoublementResultat {
            applique: false,
            groupe_source: groupe_id,
            groupe_b: None,
            date_effet: date,
            reste_dans_a: a_ids.clone(),
            passent_dans_b: b_ids.clone(),
            cours_dupliques: cours_a.iter().map(|c| c.0).collect(),
            postes_a_pourvoir: Vec::new(),
        };
        if s.apercu {
            t.rollback().await.map_err(db)?;
            return Ok(res);
        }
        let saisie = GroupeSaisie {
            code: s.code_b.clone().unwrap_or_else(|| format!("{}-B", g.code)),
            libelle: None,
            nature: g.nature.clone(),
            classes: g.classes.clone(),
            offre_id: g.offre_id,
            capacite: g.capacite,
            capacite_bloquante: g.capacite_bloquante,
            annee_id: Some(g.annee_id),
            ouvert_le: Some(date.max(g.ouvert_le)),
            ouvre_droits: Some(g.ouvre_droits),
        };
        let (gb, ev_b) = self
            .creer_groupe_sur(&mut t, a, &saisie, "groupe_cree")
            .await?;
        let mut events = vec![ev_b];
        poser_motif_annulation(&mut t, "dedoublement", Some(a.user_id)).await?;
        let veille = date - Duration::days(1);
        for e in &b_ids {
            sqlx::query("DELETE FROM ent_affectation WHERE etablissement_id = $1 AND eleve_id = $2 AND groupe_id = $3 AND date_debut >= $4")
                .bind(etab)
                .bind(e)
                .bind(groupe_id)
                .bind(date)
                .execute(&mut *t)
                .await
                .map_err(db)?;
            sqlx::query(
                "UPDATE ent_affectation SET date_fin = $4
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND groupe_id = $3
                    AND date_debut <= $4 AND (date_fin IS NULL OR date_fin > $4)",
            )
            .bind(etab)
            .bind(e)
            .bind(groupe_id)
            .bind(veille)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            sqlx::query(
                "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut,
                                               source, motif_code, decide_par)
                 VALUES ($1,$2,$3,$4,$5,$6,'saisie','dedoublement',$7)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(g.annee_id)
            .bind(e)
            .bind(gb.id)
            .bind(date.max(g.ouvert_le))
            .bind(a.user_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        let mut postes = Vec::new();
        for (_, libelle, matiere, offre, heures, semaines) in &cours_a {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_cours (id, etablissement_id, annee_id, libelle, matiere_id, offre_id, groupe_id,
                                        heures_hebdo, semaines, ouvert_le, source, cree_par)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8::float8::numeric,$9,$10,'generation',$11)",
            )
            .bind(id)
            .bind(etab)
            .bind(g.annee_id)
            .bind(libelle)
            .bind(matiere)
            .bind(offre)
            .bind(gb.id)
            .bind(heures)
            .bind(semaines)
            .bind(date.max(g.ouvert_le))
            .bind(a.user_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
            postes.push(id);
        }
        journaliser(
            &mut t,
            a,
            "groupe",
            &groupe_id.to_string(),
            "dedouble",
            None,
            Some(json!({ "groupe_b": gb.id, "date": date, "passent_dans_b": b_ids.len(), "cours": postes })),
        )
        .await?;
        events.push(
            publier(
                &mut t,
                a,
                "referentiel.groupe_scinde",
                groupe_id,
                json!({ "groupe_source": groupe_id, "groupes_cibles": [gb.id], "date": date, "cours_crees": postes }),
            )
            .await?,
        );
        events.push(
            publier(
                &mut t,
                a,
                "referentiel.composition_modifiee",
                gb.id,
                json!({ "groupe_id": gb.id, "entrants": b_ids, "sortants": [], "date": date, "motif": "dedoublement" }),
            )
            .await?,
        );
        events.push(
            publier(
                &mut t,
                a,
                "referentiel.composition_modifiee",
                groupe_id,
                json!({ "groupe_id": groupe_id, "entrants": [], "sortants": b_ids, "date": date, "motif": "dedoublement" }),
            )
            .await?,
        );
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&events);
        res.applique = true;
        res.groupe_b = Some(gb.id);
        res.postes_a_pourvoir = postes;
        Ok(res)
    }
}

// ───────────────────────── Correction rétroactive (STR-104, R-S6) ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct CorrectionSaisie {
    pub eleve_id: Uuid,
    /// Groupe où l'élève a été placé à tort.
    pub groupe_errone: Uuid,
    /// Le bon groupe, posé à la date d'origine ; vide = l'élève n'aurait pas dû être dans un groupe.
    pub groupe_juste: Option<Uuid>,
    pub motif_code: Option<String>,
    #[serde(default = "vrai")]
    pub apercu: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CorrectionResultat {
    pub applique: bool,
    pub eleve_id: Uuid,
    pub groupe_errone: Uuid,
    pub groupe_juste: Option<Uuid>,
    /// Période concernée (de l'affectation erronée) : les socles 12 (appel) et 21 (notes) la relisent.
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
}

impl PgEnt {
    /// STR-104 ★ : **corriger n'est pas changer**. L'affectation erronée est annulée (archivée avec son motif), la
    /// bonne est posée à la même période ; aucune annonce de « changement » (jamais le socle 8) ; un fait
    /// `referentiel.affectation_corrigee` signale la période aux socles 12 et 21, qui listent les appels et les notes à
    /// revoir : rien n'est déplacé seul.
    pub async fn structure_affectation_corriger(
        &self,
        a: &Acteur<'_>,
        s: CorrectionSaisie,
    ) -> Result<CorrectionResultat, EntDbError> {
        self.exiger_structure(a, Action::AffectationsGerer).await?;
        let etab = a.etablissement_id;
        let motif = motif_code(s.motif_code.as_deref())?.unwrap_or_else(|| "correction".into());
        if s.groupe_juste == Some(s.groupe_errone) {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let aujourd = aujourdhui(&mut t, etab).await?;
        let erronee: (Uuid, Uuid, NaiveDate, Option<NaiveDate>) = sqlx::query_as(
            "SELECT id, annee_id, date_debut, date_fin FROM ent_affectation
              WHERE etablissement_id = $1 AND eleve_id = $2 AND groupe_id = $3
              ORDER BY date_debut DESC LIMIT 1",
        )
        .bind(etab)
        .bind(s.eleve_id)
        .bind(s.groupe_errone)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let (id, annee, du, au) = erronee;
        if let Some(j) = s.groupe_juste {
            let gj = groupes::lire(&mut t, etab, aujourd, j)
                .await?
                .ok_or_else(introuvable)?;
            if gj.annee_id != annee
                || gj.ouvert_le > du
                || gj
                    .ferme_le
                    .is_some_and(|f| au.is_none_or(|x| x > f) || f < du)
            {
                return Err(invalide());
            }
        }
        let res = CorrectionResultat {
            applique: !s.apercu,
            eleve_id: s.eleve_id,
            groupe_errone: s.groupe_errone,
            groupe_juste: s.groupe_juste,
            du,
            au,
        };
        if s.apercu {
            t.rollback().await.map_err(db)?;
            return Ok(res);
        }
        poser_motif_annulation(&mut t, &motif, Some(a.user_id)).await?;
        sqlx::query("DELETE FROM ent_affectation WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        if let Some(j) = s.groupe_juste {
            sqlx::query(
                "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut, date_fin,
                                               source, motif_code, decide_par)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,'saisie',$8,$9)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(annee)
            .bind(s.eleve_id)
            .bind(j)
            .bind(du)
            .bind(au)
            .bind(&motif)
            .bind(a.user_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        journaliser(
            &mut t,
            a,
            "affectation",
            &s.eleve_id.to_string(),
            "corrigee",
            Some(json!({ "groupe_id": s.groupe_errone, "du": du, "au": au })),
            Some(json!({ "groupe_id": s.groupe_juste, "du": du, "au": au, "motif": motif })),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.affectation_corrigee",
            s.eleve_id,
            json!({ "eleve_id": s.eleve_id, "cible_erronee": s.groupe_errone, "cible_juste": s.groupe_juste,
                    "du": du, "au": au }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(res)
    }
}

// ───────────────────────── Élèves à affecter (STR-68) ─────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct IncoherenceChoix {
    pub cause: String,
    pub eleve_id: Option<Uuid>,
    pub offre_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct AAffecter {
    pub jour: NaiveDate,
    pub annee_id: Option<Uuid>,
    /// Élèves scolarisés ce jour-là, dans aucune classe.
    pub sans_classe: Vec<Uuid>,
    /// Choix d'enseignement sans groupe, groupe sans choix, plusieurs groupes (contrôle du lot S5).
    pub choix_incoherents: Vec<IncoherenceChoix>,
    /// Membres d'un groupe qui ne sont dans aucune de ses classes d'origine (avertissement).
    pub groupes_hors_classe: Vec<(Uuid, Uuid)>,
    /// Groupes ouverts de nature `liste` à revoir (reprise).
    pub groupes_a_revoir: Vec<Uuid>,
}

impl PgEnt {
    /// STR-68 : la file de travail de la structure (identifiants seulement ; les noms viennent de l'annuaire).
    pub async fn structure_a_affecter(
        &self,
        a: &Acteur<'_>,
        annee_id: Option<Uuid>,
        jour: Option<NaiveDate>,
    ) -> Result<AAffecter, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let etab = a.etablissement_id;
        let mut t = tx(self, a).await?;
        let jour = match jour {
            Some(j) => j,
            None => aujourdhui(&mut t, etab).await?,
        };
        let annee: Option<Uuid> = match annee_id {
            Some(y) => Some(y),
            None => sqlx::query_scalar(
                "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1
                    AND date_debut <= $2 AND date_fin >= $2 ORDER BY date_debut DESC LIMIT 1",
            )
            .bind(etab)
            .bind(jour)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?,
        };
        let Some(annee) = annee else {
            t.commit().await.map_err(db)?;
            return Ok(AAffecter {
                jour,
                annee_id: None,
                sans_classe: vec![],
                choix_incoherents: vec![],
                groupes_hors_classe: vec![],
                groupes_a_revoir: vec![],
            });
        };
        let sans_classe: Vec<Uuid> = sqlx::query_scalar(
            "SELECT e.id FROM ent_eleve e
              WHERE e.etablissement_id = $1 AND e.fusionne_dans IS NULL
                AND (e.date_entree IS NULL OR e.date_entree <= $2) AND (e.date_sortie IS NULL OR e.date_sortie > $2)
                AND e.status = 'actif'
                AND NOT EXISTS (SELECT 1 FROM ent_affectation a
                                 WHERE a.etablissement_id = e.etablissement_id AND a.eleve_id = e.id
                                   AND a.annee_id = $3 AND a.classe_id IS NOT NULL
                                   AND a.date_debut <= $2 AND (a.date_fin IS NULL OR a.date_fin >= $2))
              ORDER BY e.id",
        )
        .bind(etab)
        .bind(jour)
        .bind(annee)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let incoh: Vec<(String, Option<Uuid>, Uuid)> =
            sqlx::query_as("SELECT cause, eleve_id, offre_id FROM ent_enseignement_incoherences($1, $2, $3) ORDER BY 1, 2, 3")
                .bind(etab)
                .bind(annee)
                .bind(jour)
                .fetch_all(&mut *t)
                .await
                .map_err(db)?;
        let hors: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT m.groupe_id, m.eleve_id
               FROM ent_groupe g
               JOIN ent_groupe_membres_jour($1, $3, NULL) m ON m.groupe_id = g.id
              WHERE g.etablissement_id = $1 AND g.annee_id = $2 AND g.nature <> 'liste'
                AND EXISTS (SELECT 1 FROM ent_groupe_classe gc WHERE gc.groupe_id = g.id AND gc.etablissement_id = g.etablissement_id)
                AND NOT EXISTS (SELECT 1 FROM ent_groupe_classe gc
                                  JOIN ent_affectation ac ON ac.etablissement_id = gc.etablissement_id AND ac.classe_id = gc.classe_id
                                 WHERE gc.groupe_id = g.id AND gc.etablissement_id = g.etablissement_id
                                   AND ac.eleve_id = m.eleve_id AND ac.date_debut <= $3
                                   AND (ac.date_fin IS NULL OR ac.date_fin >= $3))
              ORDER BY 1, 2",
        )
        .bind(etab)
        .bind(annee)
        .bind(jour)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        let a_revoir: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_groupe WHERE etablissement_id = $1 AND annee_id = $2 AND nature_a_verifier
                AND (ferme_le IS NULL OR ferme_le >= $3) ORDER BY id",
        )
        .bind(etab)
        .bind(annee)
        .bind(jour)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(AAffecter {
            jour,
            annee_id: Some(annee),
            sans_classe,
            choix_incoherents: incoh
                .into_iter()
                .map(|(cause, eleve_id, offre_id)| IncoherenceChoix {
                    cause,
                    eleve_id,
                    offre_id,
                })
                .collect(),
            groupes_hors_classe: hors,
            groupes_a_revoir: a_revoir,
        })
    }
}
