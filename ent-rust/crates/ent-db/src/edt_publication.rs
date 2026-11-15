#![allow(clippy::too_many_arguments, clippy::type_complexity)]
//! Socle 10 (emploi du temps), lot T5 — **publication des versions** (`docs/socles/10_EMPLOI_DU_TEMPS.md` §14.1,
//! EDT-150 à EDT-156, R-E7).
//!
//! | Action | Fonction | Droit |
//! |---|---|---|
//! | 150 contrôler (sans écriture) | [`PgEnt::edt_publication_controler`] | `edt.publication_lire` |
//! | aperçu (séances créées, retirées, conservées…) | [`PgEnt::edt_publication_apercu`] | `edt.publication_lire` |
//! | 151–152 publier (aux personnels, puis familles ; provisoire) | [`PgEnt::edt_publier`] | `edt.publier` (+ `edt.forcer_conflit` pour un effet le jour même) |
//! | 153 avancer ou retarder la diffusion | [`PgEnt::edt_diffusion_regler`], [`PgEnt::edt_diffuser`] | `edt.publier` |
//! | 154 historique et différences | [`PgEnt::edt_publications_historique`], [`PgEnt::edt_versions_difference`] | `edt.publication_lire` |
//! | 155 revenir à la version précédente | [`PgEnt::edt_retour_arriere`] | `edt.revenir_version` |
//! | 156 version individuelle d'un élève | [`PgEnt::edt_publier_individuelle`] | `edt.publier` |
//!
//! **Deux temps.** Annoncer (aux personnels) fige la version (placements non modifiables), pose la date de prise d'effet et
//! la date de diffusion ; le réel (les séances) ne change pas encore. Diffuser est UNE transaction : version précédente
//! tronquée à la veille, nouvelle version `publiee`, séances matérialisées (T5a), événement. La publication immédiate fait
//! les deux. Un lecteur voit donc l'ancienne version ou la nouvelle, jamais un mélange (R-E7) : la bascule est atomique et
//! sérialisée avec la génération par le même verrou.
//!
//! **Le lendemain, pas avant (CL-60).** Une publication faite pendant la journée ne prend effet qu'au lendemain (jour de
//! l'établissement, fuseau de Paris par défaut) ; la direction peut forcer un effet le jour même, avec un motif journalisé :
//! les séances déjà commencées ne bougent pas.
//!
//! **Le journal du réel est conservé.** Les séances gardent leur identifiant quand le placement ne change pas (T5a) : les
//! modifications ponctuelles (T3), l'appel et le cahier suivent. L'aperçu dit combien de modifications vivantes sont conservées et
//! combien sont à revoir (séance retirée ou modifiée).
//!
//! **Aperçu = la vraie bascule, annulée.** L'aperçu exécute la bascule dans la transaction puis revient au point de sauvegarde :
//! les chiffres sont exacts, aucune écriture ne reste.
//!
//! Limites assumées (voir le rapport) : les jours fériés ne sont pas retirés du délai en jours ouvrés ; l'alignement
//! (barrettes), les trous, la pause méridienne, la capacité des salles et la quotité des temps partiels ne sont pas encore
//! contrôlés ; la diffusion échue est déclenchée par `edt_diffuser_echues` (aucun ordonnanceur ne l'appelle encore, sauf le
//! recalcul de la fenêtre).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc, Weekday};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::edt_versions::{
    conflits_du_plan, fenetre_en_grille, Fenetre, NatureConflitPlan, PlacementPourConflits,
};
use crate::ref_commun::{conflit, db, introuvable, invalide, publier_agg, Acteur};
use crate::referentiel_structure::CreneauGrille;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

pub const EVT_VERSION_PUBLIEE: &str = "edt.version_publiee";
pub const EVT_VERSION_DIFFUSEE: &str = "edt.version_diffusee";
pub const EVT_VERSION_INDIVIDUELLE: &str = "edt.version_individuelle";

// ───────────────────────────── Types ─────────────────────────────

/// Un contrôle qui a trouvé quelque chose : jamais un nom de personne, seulement des identifiants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Controle {
    pub code: String,
    pub nombre: i64,
    pub cours: Vec<Uuid>,
    pub placements: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportControles {
    pub version_id: Uuid,
    pub a_partir_du: NaiveDate,
    /// Empêchent la publication.
    pub bloquants: Vec<Controle>,
    /// Signalés, n'empêchent pas.
    pub avertissements: Vec<Controle>,
    /// Empreinte (SHA-256) de ce qui sera publié : même plan, même date, même empreinte (idempotence).
    pub empreinte: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApercuPublication {
    pub version_id: Uuid,
    pub effet_demande: Option<NaiveDate>,
    /// Date réelle de prise d'effet (au plus tôt demain, sauf forçage).
    pub effet_reel: NaiveDate,
    /// La date demandée a été repoussée au lendemain.
    pub decalee: bool,
    pub fin: NaiveDate,
    pub seances_creees: i64,
    pub seances_retirees: i64,
    pub seances_conservees: i64,
    pub seances_modifiees: i64,
    pub cours_touches: i64,
    pub classes_touchees: i64,
    pub intervenants_touches: i64,
    /// Modifications ponctuelles (T3) vivantes sur des séances qui restent identiques.
    pub modifications_conservees: i64,
    /// Modifications ponctuelles vivantes sur des séances retirées ou modifiées : à revoir.
    pub modifications_a_revoir: i64,
    /// Messages qui partiront : un par personne (intervenants à la publication, élèves et familles à la diffusion).
    pub notifications_personnels: i64,
    pub notifications_familles: i64,
    pub deux_temps: bool,
    pub diffusion_le: Option<DateTime<Utc>>,
    pub controles: Option<RapportControles>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CmdPublier {
    pub version_id: Uuid,
    /// Premier jour d'effet ; absent : demain. Repoussé à demain si c'est aujourd'hui ou avant (sauf forçage).
    #[serde(default)]
    pub a_partir_du: Option<NaiveDate>,
    /// Dernier jour d'une version **temporaire** (obligatoire pour elle).
    #[serde(default)]
    pub au: Option<NaiveDate>,
    /// Publication en deux temps ; absent : le réglage `edt.publication_deux_temps` (oui par défaut).
    #[serde(default)]
    pub deux_temps: Option<bool>,
    /// Instant de diffusion aux familles ; absent : dans le délai du réglage (2 jours ouvrés par défaut).
    #[serde(default)]
    pub diffusion_le: Option<DateTime<Utc>>,
    /// EDT provisoire de rentrée (CL-22) : la mention est portée par la version et par les messages.
    #[serde(default)]
    pub provisoire: bool,
    /// Effet le jour même : motif obligatoire, droit `edt.forcer_conflit` (direction).
    #[serde(default)]
    pub forcer_effet_immediat: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultatPublication {
    pub version_id: Uuid,
    pub applique: bool,
    pub deja_fait: bool,
    pub bloquants: Vec<Controle>,
    pub avertissements: Vec<Controle>,
    pub effet: Option<NaiveDate>,
    pub decalee: bool,
    /// La version est publiée (réel basculé) ; sinon seulement annoncée aux personnels.
    pub diffusee: bool,
    pub diffusion_le: Option<DateTime<Utc>>,
    pub apercu: Option<ApercuPublication>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaitPublication {
    pub id: Uuid,
    pub fait: String,
    pub date_effet: Option<NaiveDate>,
    pub diffusion_le: Option<DateTime<Utc>>,
    pub nb_creees: i32,
    pub nb_retirees: i32,
    pub nb_conservees: i32,
    pub nb_modifiees: i32,
    pub nb_modifications_conservees: i32,
    pub nb_avertissements: i32,
    pub forcage_motif: Option<String>,
    pub decalee: bool,
    pub par: Uuid,
    pub le: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionHistorique {
    pub id: Uuid,
    pub libelle: String,
    pub nature: String,
    pub etat: String,
    pub valable_du: Option<NaiveDate>,
    pub valable_au: Option<NaiveDate>,
    pub provisoire: bool,
    pub annoncee_le: Option<DateTime<Utc>>,
    pub diffusion_familles_le: Option<DateTime<Utc>>,
    pub publiee_le: Option<DateTime<Utc>>,
    pub remplace_version_id: Option<Uuid>,
    pub copie_de_version_id: Option<Uuid>,
    pub faits: Vec<FaitPublication>,
}

/// Position lisible d'un placement (heures de la grille, lieux).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PositionLisible {
    pub jour_semaine: i16,
    pub debut: NaiveTime,
    pub fin: NaiveTime,
    pub semaine_type: String,
    pub lieux: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifferenceCours {
    pub cours_id: Uuid,
    pub libelle: String,
    pub classe: Option<String>,
    pub intervenants: Vec<Uuid>,
    pub avant: Vec<PositionLisible>,
    pub apres: Vec<PositionLisible>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifferencesVersions {
    pub de: Uuid,
    pub vers: Uuid,
    /// Cours qui n'avaient aucun placement et en ont.
    pub ajoutes: Vec<DifferenceCours>,
    /// Cours qui n'ont plus de placement.
    pub retires: Vec<DifferenceCours>,
    /// Cours dont les placements changent (jour, plage, semaine, lieu).
    pub modifies: Vec<DifferenceCours>,
    pub inchanges: i64,
    /// Nombre de cours touchés par intervenant (sans nom : l'identifiant du compte).
    pub par_intervenant: Vec<(Uuid, i64)>,
    /// Nombre de cours touchés par classe (code de la classe).
    pub par_classe: Vec<(String, i64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdIndividuelle {
    pub eleve_id: Uuid,
    #[serde(default)]
    pub a_partir_du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    /// Motif du socle 5 (`upe2a`, `parcours_amenage`, `ulis`…).
    pub motif_code: String,
    #[serde(default)]
    pub ajouter: Vec<Uuid>,
    #[serde(default)]
    pub retirer: Vec<Uuid>,
    #[serde(default)]
    pub cle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultatIndividuelle {
    pub id: Option<Uuid>,
    pub applique: bool,
    pub deja_fait: bool,
    pub conflits: Vec<Controle>,
    pub effet: Option<NaiveDate>,
}

// ───────────────────────────── Logique pure ─────────────────────────────

/// Ajoute `n` jours ouvrés (hors week-end) à une date. Les jours fériés ne sont pas retirés (limite assumée).
pub fn ajouter_jours_ouvres(d: NaiveDate, n: u32) -> NaiveDate {
    let mut jour = d;
    let mut reste = n;
    while reste > 0 {
        jour += Duration::days(1);
        if !matches!(jour.weekday(), Weekday::Sat | Weekday::Sun) {
            reste -= 1;
        }
    }
    jour
}

/// Date d'effet réelle : au plus tôt demain, sauf forçage de la direction (aujourd'hui). Retourne `(effet, décalée)`.
pub fn date_d_effet(
    demandee: Option<NaiveDate>,
    aujourdhui: NaiveDate,
    forcer: bool,
) -> (NaiveDate, bool) {
    let demain = aujourdhui + Duration::days(1);
    match demandee {
        None => (demain, false),
        Some(d) if d > aujourdhui => (d, false),
        Some(_) if forcer => (aujourdhui, false),
        Some(_) => (demain, true),
    }
}

/// Ligne de séance telle que la comparaison avant/après la voit.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct LigneSeance {
    pub id: Uuid,
    pub cours_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
    pub prof_user_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub salle: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffSeances {
    pub creees: Vec<Uuid>,
    pub retirees: Vec<Uuid>,
    pub conservees: Vec<Uuid>,
    pub modifiees: Vec<Uuid>,
}

/// Compare les séances générées avant et après une bascule : créées, retirées, conservées (identiques), modifiées
/// (même séance, autre heure, salle ou intervenant). Fonction pure, testée sans base.
pub fn comparer_seances(avant: &[LigneSeance], apres: &[LigneSeance]) -> DiffSeances {
    let a: HashMap<Uuid, &LigneSeance> = avant.iter().map(|s| (s.id, s)).collect();
    let b: HashMap<Uuid, &LigneSeance> = apres.iter().map(|s| (s.id, s)).collect();
    let mut d = DiffSeances::default();
    for s in apres {
        match a.get(&s.id) {
            None => d.creees.push(s.id),
            Some(x) if **x == *s => d.conservees.push(s.id),
            Some(_) => d.modifiees.push(s.id),
        }
    }
    for s in avant {
        if !b.contains_key(&s.id) {
            d.retirees.push(s.id);
        }
    }
    d
}

/// Compare deux plans (cours → positions) : cours ajoutés, retirés, modifiés, et nombre de cours inchangés.
pub fn comparer_plans(
    de: &BTreeMap<Uuid, Vec<PositionLisible>>,
    vers: &BTreeMap<Uuid, Vec<PositionLisible>>,
) -> (Vec<Uuid>, Vec<Uuid>, Vec<Uuid>, i64) {
    let (mut ajoutes, mut retires, mut modifies, mut inchanges) = (vec![], vec![], vec![], 0);
    for (cours, apres) in vers {
        match de.get(cours) {
            None => ajoutes.push(*cours),
            Some(avant) => {
                let (mut x, mut y) = (avant.clone(), apres.clone());
                x.sort();
                y.sort();
                if x == y {
                    inchanges += 1;
                } else {
                    modifies.push(*cours);
                }
            }
        }
    }
    for cours in de.keys() {
        if !vers.contains_key(cours) {
            retires.push(*cours);
        }
    }
    (ajoutes, retires, modifies, inchanges)
}

// ───────────────────────────── Lectures internes ─────────────────────────────

#[derive(Debug, Clone, FromRow)]
struct VersionLigne {
    id: Uuid,
    annee_id: Uuid,
    nature: String,
    base_version_id: Option<Uuid>,
    etat: String,
    valable_du: Option<NaiveDate>,
    valable_au: Option<NaiveDate>,
    annoncee_le: Option<DateTime<Utc>>,
    diffusion_familles_le: Option<DateTime<Utc>>,
    provisoire: bool,
    remplace_version_id: Option<Uuid>,
    remplace_valable_au_avant: Option<NaiveDate>,
    remplace_etat_avant: Option<String>,
}

const SELECT_VERSION_LIGNE: &str =
    "SELECT id, etablissement_id, annee_id, nature, base_version_id, etat, valable_du, valable_au,
        annoncee_le, diffusion_familles_le, provisoire, remplace_version_id,
        remplace_valable_au_avant, remplace_etat_avant
   FROM ent_edt_version";

impl VersionLigne {
    fn annoncee(&self) -> bool {
        self.etat == "brouillon" && self.annoncee_le.is_some()
    }
}

#[derive(Debug, Clone, Copy)]
struct Annee {
    debut: NaiveDate,
    fin: NaiveDate,
}

async fn charger_version(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<VersionLigne, EntDbError> {
    sqlx::query_as(&format!(
        "{SELECT_VERSION_LIGNE} WHERE etablissement_id = $1 AND id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

async fn charger_annee(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    annee_id: Uuid,
) -> Result<Annee, EntDbError> {
    let r: Option<(NaiveDate, NaiveDate, String)> = sqlx::query_as(
        "SELECT date_debut, date_fin, statut FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(annee_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    let (debut, fin, statut) = r.ok_or_else(introuvable)?;
    if statut == "cloturee" {
        return Err(conflit());
    }
    Ok(Annee { debut, fin })
}

async fn aujourdhui_tx(tx: &mut TenantTx<'_>, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ent_aujourdhui_etab($1)")
        .bind(etab)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)
}

#[derive(Debug, FromRow)]
struct PlacementLigne {
    id: Uuid,
    cours_id: Uuid,
    jour_semaine: i16,
    creneau_debut_code: Option<String>,
    duree_creneaux: Option<i16>,
    heure_debut_libre: Option<NaiveTime>,
    heure_fin_libre: Option<NaiveTime>,
    semaine_type: String,
    lieux: Vec<Uuid>,
}

async fn placements_de(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    versions: &[Uuid],
) -> Result<Vec<PlacementLigne>, EntDbError> {
    sqlx::query_as(
        "SELECT p.id, p.cours_id, p.jour_semaine, p.creneau_debut_code, p.duree_creneaux, p.heure_debut_libre,
                p.heure_fin_libre, p.semaine_type,
                coalesce((SELECT array_agg(pl.lieu_id ORDER BY pl.principal DESC, pl.lieu_id)
                            FROM ent_edt_placement_lieu pl
                           WHERE pl.placement_id = p.id AND pl.etablissement_id = p.etablissement_id), '{}') AS lieux
           FROM ent_edt_placement p
          WHERE p.etablissement_id = $1 AND p.version_id = ANY($2)
          ORDER BY p.cours_id, p.jour_semaine, p.id",
    )
    .bind(etab)
    .bind(versions)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)
}

async fn grille_de(tx: &mut TenantTx<'_>, etab: Uuid) -> Result<Vec<CreneauGrille>, EntDbError> {
    sqlx::query_as(
        "SELECT id, site_id, code, libelle, demi_journee, ordre, heure_debut, heure_fin
           FROM ent_grille_horaire WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)
}

async fn sites_des_cours(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    annee_id: Uuid,
) -> Result<HashMap<Uuid, Option<Uuid>>, EntDbError> {
    let v: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT c.id, cl.site_id FROM ent_cours c
           LEFT JOIN ent_classe cl ON cl.id = COALESCE(c.classe_id, (SELECT g.classe_id FROM ent_groupe g
                                                                      WHERE g.id = c.groupe_id AND g.etablissement_id = c.etablissement_id))
                                  AND cl.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.annee_id = $2",
    )
    .bind(etab)
    .bind(annee_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    Ok(v.into_iter().collect())
}

fn fenetre_du_placement(
    p: &PlacementLigne,
    grille: &[CreneauGrille],
    site: Option<Uuid>,
) -> Option<Fenetre> {
    match (&p.creneau_debut_code, p.duree_creneaux) {
        (Some(code), Some(d)) => fenetre_en_grille(grille, site, code, d)
            .or_else(|| fenetre_en_grille(grille, None, code, d)),
        _ => Some(Fenetre {
            debut: p.heure_debut_libre?,
            fin: p.heure_fin_libre?,
        }),
    }
}

/// Cours de la version temporaire (cours visés, cours des agents visés, cours des placements de base d'un lieu visé).
async fn cours_perimetre(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    v: &VersionLigne,
    placements: &[PlacementLigne],
) -> Result<HashSet<Uuid>, EntDbError> {
    let mut cours: HashSet<Uuid> = placements.iter().map(|p| p.cours_id).collect();
    let direct: Vec<Uuid> = sqlx::query_scalar(
        "SELECT cours_id FROM ent_edt_version_perimetre WHERE etablissement_id = $1 AND version_id = $2 AND cours_id IS NOT NULL
         UNION
         SELECT s.cours_id FROM ent_edt_version_perimetre vp
           JOIN ent_service_intervenant i ON i.agent_id = vp.agent_id AND i.etablissement_id = vp.etablissement_id
                AND i.annule_le IS NULL
           JOIN ent_service_enseignement s ON s.id = i.service_id AND s.etablissement_id = i.etablissement_id
          WHERE vp.etablissement_id = $1 AND vp.version_id = $2 AND vp.agent_id IS NOT NULL AND s.cours_id IS NOT NULL
         UNION
         SELECT p.cours_id FROM ent_edt_version_perimetre vp
           JOIN ent_edt_placement_lieu pl ON pl.lieu_id = vp.lieu_id AND pl.etablissement_id = vp.etablissement_id
           JOIN ent_edt_placement p ON p.id = pl.placement_id AND p.etablissement_id = pl.etablissement_id
          WHERE vp.etablissement_id = $1 AND vp.version_id = $2 AND vp.lieu_id IS NOT NULL AND p.version_id = $3",
    )
    .bind(etab)
    .bind(v.id)
    .bind(v.base_version_id.unwrap_or_else(Uuid::nil))
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    cours.extend(direct);
    Ok(cours)
}

fn controle(code: &str, cours: Vec<Uuid>, placements: Vec<Uuid>) -> Controle {
    let mut cours = cours;
    cours.sort();
    cours.dedup();
    let nombre = if placements.is_empty() {
        cours.len()
    } else {
        placements.len()
    } as i64;
    Controle {
        code: code.to_string(),
        nombre: nombre.max(1),
        cours,
        placements,
    }
}

/// Contrôles du §14.1 sur une version, à une date d'effet, sans rien écrire.
async fn controler_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    v: &VersionLigne,
    annee: Annee,
    effet: NaiveDate,
    au: Option<NaiveDate>,
) -> Result<RapportControles, EntDbError> {
    let mut bloquants: Vec<Controle> = Vec::new();
    let mut avertissements: Vec<Controle> = Vec::new();
    let placements = placements_de(tx, etab, &[v.id]).await?;
    if placements.is_empty() {
        bloquants.push(Controle {
            code: "version_vide".into(),
            nombre: 1,
            cours: vec![],
            placements: vec![],
        });
    }
    // Dates dans l'année.
    let fin_version = au.or(v.valable_au).unwrap_or(annee.fin);
    if effet < annee.debut || effet > annee.fin || fin_version > annee.fin || fin_version < effet {
        bloquants.push(Controle {
            code: "date_hors_annee".into(),
            nombre: 1,
            cours: vec![],
            placements: vec![],
        });
    }
    if v.nature == "temporaire" {
        if au.or(v.valable_au).is_none() {
            bloquants.push(Controle {
                code: "temporaire_sans_plage".into(),
                nombre: 1,
                cours: vec![],
                placements: vec![],
            });
        }
        let base_ok: bool = match v.base_version_id {
            Some(b) => sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_edt_version WHERE etablissement_id = $1 AND id = $2
                                  AND nature = 'base' AND etat = 'publiee')",
            )
            .bind(etab)
            .bind(b)
            .fetch_one(&mut **tx)
            .await
            .map_err(db)?,
            None => false,
        };
        if !base_ok {
            bloquants.push(Controle {
                code: "temporaire_sans_base_publiee".into(),
                nombre: 1,
                cours: vec![],
                placements: vec![],
            });
        }
    } else {
        // Une publication prend le relais de la version en vigueur à cette date ; elle ne peut pas en précéder une autre.
        let apres: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_edt_version
              WHERE etablissement_id = $1 AND annee_id = $2 AND nature = 'base' AND etat = 'publiee'
                AND valable_du > $3 AND id <> $4",
        )
        .bind(etab)
        .bind(v.annee_id)
        .bind(effet)
        .bind(v.id)
        .fetch_all(&mut **tx)
        .await
        .map_err(db)?;
        if !apres.is_empty() {
            bloquants.push(Controle {
                code: "precede_une_version_publiee".into(),
                nombre: apres.len() as i64,
                cours: vec![],
                placements: vec![],
            });
        }
        // Un trou entre la version en vigueur et la nouvelle : averti.
        let fin_precedente: Option<NaiveDate> = sqlx::query_scalar(
            "SELECT max(coalesce(valable_au, $3)) FROM ent_edt_version
              WHERE etablissement_id = $1 AND annee_id = $2 AND nature = 'base' AND etat = 'publiee'
                AND valable_du < $4 AND id <> $5",
        )
        .bind(etab)
        .bind(v.annee_id)
        .bind(annee.fin)
        .bind(effet)
        .bind(v.id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        if fin_precedente.is_some_and(|f| f < effet - Duration::days(1)) {
            avertissements.push(Controle {
                code: "trou_entre_versions".into(),
                nombre: 1,
                cours: vec![],
                placements: vec![],
            });
        }
    }

    // Conflits durs sur l'ensemble des placements qui seront en vigueur (cours, intervenant, lieu).
    let grille = grille_de(tx, etab).await?;
    let sites = sites_des_cours(tx, etab, v.annee_id).await?;
    let interv: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT s.cours_id, s.prof_user_id FROM ent_service_enseignement s
          WHERE s.etablissement_id = $1 AND s.cours_id IS NOT NULL AND s.annee_id = $2
            AND s.etat = 'ouvert' AND s.prof_user_id IS NOT NULL",
    )
    .bind(etab)
    .bind(v.annee_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let mut agents_du_cours: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (c, ag) in interv {
        agents_du_cours.entry(c).or_default().push(ag);
    }
    let mut ensemble: Vec<PlacementLigne> = Vec::new();
    let mut autres: Vec<PlacementLigne> = Vec::new();
    if v.nature == "temporaire" {
        if let Some(base) = v.base_version_id {
            let perimetre = cours_perimetre(tx, etab, v, &placements).await?;
            autres = placements_de(tx, etab, &[base]).await?;
            autres.retain(|p| !perimetre.contains(&p.cours_id));
        }
    }
    ensemble.extend(autres);
    let nb_autres = ensemble.len();
    let versions_ensemble: Vec<&PlacementLigne> =
        ensemble.iter().chain(placements.iter()).collect();
    let mut hors_grille: Vec<Uuid> = Vec::new();
    let pour: Vec<PlacementPourConflits> = versions_ensemble
        .iter()
        .filter_map(|p| {
            let site = sites.get(&p.cours_id).copied().flatten();
            let Some(fenetre) = fenetre_du_placement(p, &grille, site) else {
                hors_grille.push(p.id);
                return None;
            };
            Some(PlacementPourConflits {
                id: p.id,
                cours_id: p.cours_id,
                jour_semaine: p.jour_semaine,
                semaine_type: p.semaine_type.clone(),
                fenetre,
                lieux: p.lieux.clone(),
                intervenants: agents_du_cours
                    .get(&p.cours_id)
                    .cloned()
                    .unwrap_or_default(),
            })
        })
        .collect();
    let cours_de: HashMap<Uuid, Uuid> = versions_ensemble
        .iter()
        .map(|p| (p.id, p.cours_id))
        .collect();
    let conflits = conflits_du_plan(&pour);
    for (nature, code) in [
        (NatureConflitPlan::Cours, "conflit_cours"),
        (NatureConflitPlan::Intervenant, "conflit_intervenant"),
        (NatureConflitPlan::Lieu, "conflit_lieu"),
    ] {
        let ceux: Vec<_> = conflits.iter().filter(|c| c.nature == nature).collect();
        if !ceux.is_empty() {
            let mut pl: Vec<Uuid> = ceux
                .iter()
                .flat_map(|c| [c.placement_a, c.placement_b])
                .collect();
            pl.sort();
            pl.dedup();
            let cours = pl.iter().filter_map(|p| cours_de.get(p).copied()).collect();
            let mut c = controle(code, cours, pl);
            c.nombre = ceux.len() as i64;
            bloquants.push(c);
        }
    }
    let _ = nb_autres;
    // Un placement dont le créneau de grille n'existe plus : averti (il n'a pas de séance).
    if !hors_grille.is_empty() {
        let cours = hors_grille
            .iter()
            .filter_map(|p| cours_de.get(p).copied())
            .collect();
        avertissements.push(controle(
            "placement_sans_creneau_de_grille",
            cours,
            hors_grille,
        ));
    }
    // Cours sans classe (cours de groupe sans classe d'origine) : ses séances ne peuvent pas exister (pont T5a).
    let sans_classe: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT p.id, p.cours_id FROM ent_edt_placement p
           JOIN ent_cours c ON c.id = p.cours_id AND c.etablissement_id = p.etablissement_id
          WHERE p.etablissement_id = $1 AND p.version_id = $2
            AND NOT EXISTS (SELECT 1 FROM ent_classe cl WHERE cl.etablissement_id = c.etablissement_id AND cl.id = COALESCE(
                    c.classe_id,
                    (SELECT g.classe_id FROM ent_groupe g WHERE g.id = c.groupe_id AND g.etablissement_id = c.etablissement_id),
                    (SELECT gc.classe_id FROM ent_groupe_classe gc
                      WHERE gc.groupe_id = c.groupe_id AND gc.etablissement_id = c.etablissement_id
                      ORDER BY gc.classe_id LIMIT 1)))",
    )
    .bind(etab)
    .bind(v.id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    if !sans_classe.is_empty() {
        bloquants.push(controle(
            "cours_sans_classe",
            sans_classe.iter().map(|x| x.1).collect(),
            sans_classe.iter().map(|x| x.0).collect(),
        ));
    }

    // Avertissements : cours vacants (aucun intervenant), heures placées ≠ volume du cours.
    let cours_de_la_version: BTreeSet<Uuid> = placements.iter().map(|p| p.cours_id).collect();
    let vacants: Vec<Uuid> = cours_de_la_version
        .iter()
        .filter(|c| !agents_du_cours.contains_key(c))
        .copied()
        .collect();
    if !vacants.is_empty() {
        avertissements.push(controle("cours_vacant", vacants, vec![]));
    }
    let besoins: Vec<(Uuid, f64)> = sqlx::query_as(
        "SELECT cours_id, heures_hebdo::float8 FROM ent_edt_besoin WHERE etablissement_id = $1 AND version_id = $2",
    )
    .bind(etab)
    .bind(v.id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let mut minutes: HashMap<Uuid, f64> = HashMap::new();
    for p in &placements {
        let site = sites.get(&p.cours_id).copied().flatten();
        if let Some(f) = fenetre_du_placement(p, &grille, site) {
            let m = (f.fin - f.debut).num_minutes() as f64;
            let poids = if p.semaine_type == "toutes" { 1.0 } else { 0.5 };
            *minutes.entry(p.cours_id).or_default() += m * poids;
        }
    }
    let ecarts: Vec<Uuid> = besoins
        .iter()
        .filter(|(c, h)| (minutes.get(c).copied().unwrap_or(0.0) / 60.0 - h).abs() > 0.26)
        .map(|(c, _)| *c)
        .collect();
    if !ecarts.is_empty() {
        avertissements.push(controle("heures_differentes_du_volume", ecarts, vec![]));
    }

    let empreinte: String = sqlx::query_scalar(
        "SELECT encode(sha256(convert_to($3 || '|' || coalesce(string_agg(x, ';' ORDER BY x), ''), 'UTF8')), 'hex')
           FROM (SELECT concat_ws(',', p.cours_id, p.jour_semaine, coalesce(p.creneau_debut_code, ''),
                                  coalesce(p.duree_creneaux::text, ''), coalesce(p.heure_debut_libre::text, ''),
                                  coalesce(p.heure_fin_libre::text, ''), p.semaine_type,
                                  coalesce((SELECT string_agg(l.lieu_id::text, ' ' ORDER BY l.lieu_id)
                                              FROM ent_edt_placement_lieu l
                                             WHERE l.placement_id = p.id AND l.etablissement_id = p.etablissement_id), '')) AS x
                   FROM ent_edt_placement p WHERE p.etablissement_id = $1 AND p.version_id = $2) t",
    )
    .bind(etab)
    .bind(v.id)
    .bind(format!("{}:{}:{}:{}", v.id, effet, fin_version, v.nature))
    .fetch_one(&mut **tx)
    .await
    .map_err(db)?;
    Ok(RapportControles {
        version_id: v.id,
        a_partir_du: effet,
        bloquants,
        avertissements,
        empreinte,
    })
}

// ───────────────────────────── La bascule (cœur commun) ─────────────────────────────

struct Bascule {
    effet: NaiveDate,
    fin: NaiveDate,
    diff: DiffSeances,
    evenement: MetierEvent,
    modifications_conservees: i64,
    modifications_a_revoir: i64,
    cours_touches: i64,
    classes_touchees: i64,
    intervenants_touches: i64,
}

async fn lignes_generees(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    annee_id: Uuid,
    du: NaiveDate,
    au: NaiveDate,
) -> Result<Vec<LigneSeance>, EntDbError> {
    sqlx::query_as(
        "SELECT id, cours_id, classe_id, prof_user_id, jour, heure_debut, heure_fin, salle
           FROM ent_edt_seance
          WHERE etablissement_id = $1 AND annee_id = $2 AND jour BETWEEN $3 AND $4
            AND source = 'genere' AND statut <> 'retiree'",
    )
    .bind(etab)
    .bind(annee_id)
    .bind(du)
    .bind(au)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)
}

impl PgEnt {
    /// Bascule le réel sur `v` à partir de `effet` : version en vigueur tronquée à la veille (ou remplacée si elle
    /// commence le même jour), `v` publiée, séances matérialisées. **Dans la transaction de l'appelant.**
    async fn basculer_tx(
        &self,
        tx: &mut TenantTx<'_>,
        a: &Acteur<'_>,
        v: &VersionLigne,
        annee: Annee,
        effet: NaiveDate,
        fin_version: Option<NaiveDate>,
        fige_avant: Option<DateTime<Utc>>,
    ) -> Result<Bascule, EntDbError> {
        let e = a.etablissement_id;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 3))")
            .bind(format!("edt-publication-{e}"))
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        // Les créneaux historiques sont repris dans la version « reprise » AVANT de la tronquer (rejouable).
        let _ = Self::reprendre_creneaux_tx(&mut **tx, a, v.annee_id).await?;
        let fin = fin_version.unwrap_or(annee.fin).min(annee.fin);
        let avant = lignes_generees(tx, e, v.annee_id, effet, fin).await?;

        let (mut remplace, mut valable_au_avant, mut etat_avant): (
            Option<Uuid>,
            Option<NaiveDate>,
            Option<String>,
        ) = (None, None, None);
        if v.nature == "base" {
            let en_vigueur: Option<(Uuid, NaiveDate, Option<NaiveDate>, String)> = sqlx::query_as(
                "SELECT id, valable_du, valable_au, etat FROM ent_edt_version
                  WHERE etablissement_id = $1 AND annee_id = $2 AND nature = 'base' AND etat = 'publiee'
                    AND id <> $3 AND valable_du <= $4 AND coalesce(valable_au, $5) >= $4
                  FOR UPDATE",
            )
            .bind(e)
            .bind(v.annee_id)
            .bind(v.id)
            .bind(effet)
            .bind(annee.fin)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db)?;
            if let Some((prev, prev_du, prev_au, prev_etat)) = en_vigueur {
                remplace = Some(prev);
                valable_au_avant = prev_au;
                etat_avant = Some(prev_etat);
                if prev_du < effet {
                    sqlx::query(
                        "UPDATE ent_edt_version SET valable_au = $3, modifie_le = now()
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(e)
                    .bind(prev)
                    .bind(effet - Duration::days(1))
                    .execute(&mut **tx)
                    .await
                    .map_err(db)?;
                } else {
                    // Elle commence le même jour : entièrement remplacée (ses dates sont gardées pour le retour arrière).
                    sqlx::query(
                        "UPDATE ent_edt_version SET etat = 'remplacee', modifie_le = now()
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(e)
                    .bind(prev)
                    .execute(&mut **tx)
                    .await
                    .map_err(db)?;
                }
            }
        }
        sqlx::query(
            "UPDATE ent_edt_version
                SET etat = 'publiee', valable_du = $3, valable_au = $4, publiee_le = now(),
                    remplace_version_id = $5, remplace_valable_au_avant = $6, remplace_etat_avant = $7,
                    modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(v.id)
        .bind(effet)
        .bind(if v.nature == "temporaire" { fin_version } else { Some(annee.fin) })
        .bind(remplace)
        .bind(valable_au_avant)
        .bind(etat_avant)
        .execute(&mut **tx)
        .await
        .map_err(db)?;

        let (_rapport, evenement) = self
            .generer_seances_dans(tx, a, v.annee_id, Some(effet), Some(fin), fige_avant)
            .await?;
        let apres = lignes_generees(tx, e, v.annee_id, effet, fin).await?;
        let diff = comparer_seances(&avant, &apres);

        // Journal du réel : conservé (séances identiques) ou à revoir (séances retirées ou modifiées).
        let conservees: Vec<Uuid> = diff.conservees.clone();
        let a_revoir: Vec<Uuid> = diff
            .retirees
            .iter()
            .chain(diff.modifiees.iter())
            .copied()
            .collect();
        let modifications_conservees: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_edt_modification
              WHERE etablissement_id = $1 AND seance_id = ANY($2) AND levee_le IS NULL",
        )
        .bind(e)
        .bind(&conservees)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        let modifications_a_revoir: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_edt_modification
              WHERE etablissement_id = $1 AND seance_id = ANY($2) AND levee_le IS NULL",
        )
        .bind(e)
        .bind(&a_revoir)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        let touchees: HashMap<Uuid, &LigneSeance> = avant
            .iter()
            .chain(apres.iter())
            .filter(|s| diff.creees.contains(&s.id) || a_revoir.contains(&s.id))
            .map(|s| (s.id, s))
            .collect();
        let cours_touches = touchees
            .values()
            .filter_map(|s| s.cours_id)
            .collect::<HashSet<_>>()
            .len() as i64;
        let classes_touchees = touchees
            .values()
            .filter_map(|s| s.classe_id)
            .collect::<HashSet<_>>()
            .len() as i64;
        let intervenants_touches = touchees
            .values()
            .map(|s| s.prof_user_id)
            .collect::<HashSet<_>>()
            .len() as i64;
        Ok(Bascule {
            effet,
            fin,
            diff,
            evenement,
            modifications_conservees,
            modifications_a_revoir,
            cours_touches,
            classes_touchees,
            intervenants_touches,
        })
    }

    /// Chiffres de l'aperçu : la bascule exécutée puis annulée (point de sauvegarde). Aucune écriture ne reste.
    async fn apercu_dans(
        &self,
        tx: &mut TenantTx<'_>,
        a: &Acteur<'_>,
        v: &VersionLigne,
        annee: Annee,
        effet: NaiveDate,
        fin_version: Option<NaiveDate>,
        fige_avant: Option<DateTime<Utc>>,
    ) -> Result<ApercuPublication, EntDbError> {
        sqlx::query("SAVEPOINT edt_apercu")
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        let r = self
            .basculer_tx(tx, a, v, annee, effet, fin_version, fige_avant)
            .await;
        let resultat = match r {
            Ok(b) => {
                // Familles touchées : élèves du public à la date d'effet des cours touchés.
                let cours: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT DISTINCT cours_id FROM ent_edt_seance
                      WHERE etablissement_id = $1 AND id = ANY($2) AND cours_id IS NOT NULL",
                )
                .bind(a.etablissement_id)
                .bind(
                    b.diff
                        .creees
                        .iter()
                        .chain(b.diff.modifiees.iter())
                        .copied()
                        .collect::<Vec<_>>(),
                )
                .fetch_all(&mut **tx)
                .await
                .map_err(db)?;
                let eleves: i64 = sqlx::query_scalar(
                    "SELECT count(DISTINCT eleve_id) FROM ent_public_cours_jour($1, $2) WHERE cours_id = ANY($3)",
                )
                .bind(a.etablissement_id)
                .bind(b.effet)
                .bind(&cours)
                .fetch_one(&mut **tx)
                .await
                .map_err(db)?;
                Ok(ApercuPublication {
                    version_id: v.id,
                    effet_demande: None,
                    effet_reel: b.effet,
                    decalee: false,
                    fin: b.fin,
                    seances_creees: b.diff.creees.len() as i64,
                    seances_retirees: b.diff.retirees.len() as i64,
                    seances_conservees: b.diff.conservees.len() as i64,
                    seances_modifiees: b.diff.modifiees.len() as i64,
                    cours_touches: b.cours_touches,
                    classes_touchees: b.classes_touchees,
                    intervenants_touches: b.intervenants_touches,
                    modifications_conservees: b.modifications_conservees,
                    modifications_a_revoir: b.modifications_a_revoir,
                    notifications_personnels: b.intervenants_touches,
                    notifications_familles: eleves,
                    ..Default::default()
                })
            }
            Err(e) => Err(e),
        };
        sqlx::query("ROLLBACK TO SAVEPOINT edt_apercu")
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        sqlx::query("RELEASE SAVEPOINT edt_apercu")
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        resultat
    }

    async fn parametre_bool(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        cle: &str,
        defaut: bool,
    ) -> Result<bool, EntDbError> {
        let v: Option<Option<bool>> = sqlx::query_scalar(
            "SELECT valeur_booleen FROM ent_parametre WHERE etablissement_id = $1 AND cle = $2",
        )
        .bind(etab)
        .bind(cle)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        Ok(v.flatten().unwrap_or(defaut))
    }

    async fn parametre_entier(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        cle: &str,
        defaut: i32,
    ) -> Result<i32, EntDbError> {
        let v: Option<Option<i32>> = sqlx::query_scalar(
            "SELECT valeur_entier FROM ent_parametre WHERE etablissement_id = $1 AND cle = $2",
        )
        .bind(etab)
        .bind(cle)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        Ok(v.flatten().unwrap_or(defaut))
    }

    // ───────────────────────────── EDT-150 : contrôler ─────────────────────────────

    /// Contrôle une version avant publication (sans aucune écriture) : bloquants et avertissements.
    pub async fn edt_publication_controler(
        &self,
        a: &Acteur<'_>,
        version_id: Uuid,
        a_partir_du: Option<NaiveDate>,
        au: Option<NaiveDate>,
    ) -> Result<RapportControles, EntDbError> {
        self.exiger_edt(a, Action::EdtPublicationLire).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let v = charger_version(&mut tx, e, version_id, false).await?;
        let annee = charger_annee(&mut tx, e, v.annee_id).await?;
        let aujourdhui = aujourdhui_tx(&mut tx, e).await?;
        let (effet, _) = date_d_effet(a_partir_du.or(v.valable_du), aujourdhui, false);
        controler_tx(&mut tx, e, &v, annee, effet, au).await
    }

    /// Aperçu de ce que la publication changerait : séances créées, retirées, conservées, modifiées ; publics et
    /// intervenants touchés ; modifications ponctuelles conservées ou à revoir ; messages qui partiront. Sans écriture.
    pub async fn edt_publication_apercu(
        &self,
        a: &Acteur<'_>,
        version_id: Uuid,
        a_partir_du: Option<NaiveDate>,
        au: Option<NaiveDate>,
    ) -> Result<ApercuPublication, EntDbError> {
        self.exiger_edt(a, Action::EdtPublicationLire).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let v = charger_version(&mut tx, e, version_id, false).await?;
        if v.etat != "brouillon" {
            return Err(conflit()); // déjà publiée : rien à prévoir
        }
        let annee = charger_annee(&mut tx, e, v.annee_id).await?;
        let aujourdhui = aujourdhui_tx(&mut tx, e).await?;
        let (effet, decalee) = date_d_effet(a_partir_du.or(v.valable_du), aujourdhui, false);
        let controles = controler_tx(&mut tx, e, &v, annee, effet, au).await?;
        let fin_version = au.or(v.valable_au);
        let (deux_temps, diffusion) = self
            .plan_diffusion(&mut tx, e, aujourdhui, effet, None, None)
            .await?;
        let mut apercu = if controles.bloquants.is_empty() {
            self.apercu_dans(&mut tx, a, &v, annee, effet, fin_version, None)
                .await?
        } else {
            ApercuPublication {
                version_id,
                effet_reel: effet,
                ..Default::default()
            }
        };
        apercu.effet_demande = a_partir_du;
        apercu.effet_reel = effet;
        apercu.decalee = decalee;
        apercu.deux_temps = deux_temps;
        apercu.diffusion_le = diffusion;
        apercu.version_id = version_id;
        apercu.controles = Some(controles);
        Ok(apercu)
    }

    /// Deux temps ou immédiat, et l'instant de diffusion : le réglage de l'établissement, sauf demande contraire. Si la
    /// diffusion tombait le jour de l'effet ou après, les familles seraient prévenues trop tard : diffusion immédiate.
    async fn plan_diffusion(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        aujourdhui: NaiveDate,
        effet: NaiveDate,
        deux_temps: Option<bool>,
        diffusion_le: Option<DateTime<Utc>>,
    ) -> Result<(bool, Option<DateTime<Utc>>), EntDbError> {
        let deux = match deux_temps {
            Some(x) => x,
            None => {
                self.parametre_bool(tx, etab, "edt.publication_deux_temps", true)
                    .await?
            }
        };
        if !deux {
            return Ok((false, None));
        }
        let instant: DateTime<Utc> = match diffusion_le {
            Some(i) => i,
            None => {
                let delai = self
                    .parametre_entier(tx, etab, "edt.publication_delai_jours_ouvres", 2)
                    .await?;
                let jour = ajouter_jours_ouvres(aujourdhui, u32::try_from(delai).unwrap_or(2));
                sqlx::query_scalar(
                    "SELECT (($2::date + time '06:00') AT TIME ZONE coalesce((SELECT fuseau FROM ent_etablissement_parametre
                                                                                 WHERE etablissement_id = $1), 'Europe/Paris'))",
                )
                .bind(etab)
                .bind(jour)
                .fetch_one(&mut **tx)
                .await
                .map_err(db)?
            }
        };
        let (jour_diffusion, maintenant): (NaiveDate, DateTime<Utc>) = sqlx::query_as(
            "SELECT ($2::timestamptz AT TIME ZONE coalesce((SELECT fuseau FROM ent_etablissement_parametre
                                                              WHERE etablissement_id = $1), 'Europe/Paris'))::date, now()",
        )
        .bind(etab)
        .bind(instant)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        if instant <= maintenant || jour_diffusion >= effet {
            return Ok((false, None)); // pas le temps de laisser un délai aux personnels : tout de suite
        }
        Ok((true, Some(instant)))
    }

    // ───────────────────────────── EDT-151/152 : publier ─────────────────────────────

    /// Publie une version : l'annonce aux personnels (version figée, effet et diffusion posés) puis, si la publication est
    /// immédiate, la bascule du réel. Idempotent (même version, même plan, même date).
    pub async fn edt_publier(
        &self,
        a: &Acteur<'_>,
        cmd: &CmdPublier,
    ) -> Result<ResultatPublication, EntDbError> {
        self.exiger_edt(a, Action::EdtPublier).await?;
        let forcage = match &cmd.forcer_effet_immediat {
            Some(m) => {
                let m = m.trim();
                if m.is_empty() || m.chars().count() > 300 {
                    return Err(invalide());
                }
                self.exiger_edt(a, Action::EdtForcerConflit).await?;
                Some(m.to_string())
            }
            None => None,
        };
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 3))")
            .bind(format!("edt-publication-{e}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let v = charger_version(&mut tx, e, cmd.version_id, true).await?;
        let annee = charger_annee(&mut tx, e, v.annee_id).await?;
        let aujourdhui = aujourdhui_tx(&mut tx, e).await?;
        let (effet, decalee) = date_d_effet(cmd.a_partir_du, aujourdhui, forcage.is_some());
        let fin_version = if v.nature == "temporaire" {
            cmd.au.or(v.valable_au)
        } else {
            None
        };

        // Déjà publiée ou annoncée avec ce plan et cette date : rien à refaire.
        if matches!(v.etat.as_str(), "publiee" | "remplacee") || v.annoncee() {
            let meme_date = cmd.a_partir_du.is_none() || v.valable_du == Some(effet);
            if meme_date && v.etat != "remplacee" {
                return Ok(ResultatPublication {
                    version_id: v.id,
                    applique: false,
                    deja_fait: true,
                    bloquants: vec![],
                    avertissements: vec![],
                    effet: v.valable_du,
                    decalee: false,
                    diffusee: v.etat == "publiee",
                    diffusion_le: v.diffusion_familles_le,
                    apercu: None,
                });
            }
            return Err(conflit()); // autre date : retirer l'annonce (ou revenir en arrière) d'abord
        }
        if v.etat != "brouillon" {
            return Err(conflit());
        }
        let controles = controler_tx(&mut tx, e, &v, annee, effet, fin_version).await?;
        if !controles.bloquants.is_empty() {
            return Ok(ResultatPublication {
                version_id: v.id,
                applique: false,
                deja_fait: false,
                bloquants: controles.bloquants,
                avertissements: controles.avertissements,
                effet: Some(effet),
                decalee,
                diffusee: false,
                diffusion_le: None,
                apercu: None,
            });
        }
        let (deux_temps, diffusion) = self
            .plan_diffusion(
                &mut tx,
                e,
                aujourdhui,
                effet,
                cmd.deux_temps,
                cmd.diffusion_le,
            )
            .await?;
        // Chiffres du journal : la bascule simulée (annulée).
        let fige = forcage.as_ref().map(|_| Utc::now());
        let apercu = self
            .apercu_dans(&mut tx, a, &v, annee, effet, fin_version, fige)
            .await?;

        // Temps 1 : annoncer aux personnels (figer la version).
        sqlx::query(
            "UPDATE ent_edt_version
                SET annoncee_le = now(), annoncee_par = $3, valable_du = $4, valable_au = $5,
                    diffusion_familles_le = $6, provisoire = $7, empreinte_controles = $8, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'brouillon' AND annoncee_le IS NULL",
        )
        .bind(e)
        .bind(v.id)
        .bind(a.user_id)
        .bind(effet)
        .bind(fin_version)
        .bind(diffusion)
        .bind(cmd.provisoire)
        .bind(&controles.empreinte)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            e,
            v.annee_id,
            v.id,
            "annoncee",
            Some(effet),
            diffusion,
            &apercu,
            controles.avertissements.len() as i32,
            forcage.as_deref(),
            decalee,
            a.user_id,
        )
        .await?;
        let mut evenements = vec![
            publier_agg(
                &mut tx,
                a,
                AggregateKind::Edt,
                EVT_VERSION_PUBLIEE,
                v.id,
                json!({ "version_id": v.id, "a_partir_du": effet, "diffusion_le": diffusion,
                        "provisoire": cmd.provisoire, "deux_temps": deux_temps }),
            )
            .await?,
        ];
        let mut diffusee = false;
        if !deux_temps {
            let ev = self
                .diffuser_dans(
                    &mut tx,
                    a,
                    v.id,
                    forcage.as_deref(),
                    decalee,
                    controles.avertissements.len() as i32,
                )
                .await?;
            evenements.extend(ev);
            diffusee = true;
        }
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evenements);
        Ok(ResultatPublication {
            version_id: v.id,
            applique: true,
            deja_fait: false,
            bloquants: vec![],
            avertissements: controles.avertissements,
            effet: Some(effet),
            decalee,
            diffusee,
            diffusion_le: diffusion,
            apercu: Some(ApercuPublication {
                effet_demande: cmd.a_partir_du,
                decalee,
                deux_temps,
                diffusion_le: diffusion,
                controles: None,
                ..apercu
            }),
        })
    }

    /// Temps 2 : la bascule du réel d'une version annoncée, dans la transaction de l'appelant. Si le jour d'effet est
    /// déjà passé (diffusion tardive), il est repoussé au lendemain.
    async fn diffuser_dans(
        &self,
        tx: &mut TenantTx<'_>,
        a: &Acteur<'_>,
        version_id: Uuid,
        forcage: Option<&str>,
        decalee: bool,
        nb_avertissements: i32,
    ) -> Result<Vec<MetierEvent>, EntDbError> {
        let e = a.etablissement_id;
        let v = charger_version(tx, e, version_id, true).await?;
        if !v.annoncee() {
            return Err(conflit());
        }
        let annee = charger_annee(tx, e, v.annee_id).await?;
        let aujourdhui = aujourdhui_tx(tx, e).await?;
        let prevu = v.valable_du.ok_or_else(invalide)?;
        let (effet, decale_ici) = if prevu <= aujourdhui && forcage.is_none() {
            (aujourdhui + Duration::days(1), true)
        } else {
            (prevu, false)
        };
        let fige = forcage.map(|_| Utc::now());
        let b = self
            .basculer_tx(tx, a, &v, annee, effet, v.valable_au, fige)
            .await?;
        let apercu = ApercuPublication {
            seances_creees: b.diff.creees.len() as i64,
            seances_retirees: b.diff.retirees.len() as i64,
            seances_conservees: b.diff.conservees.len() as i64,
            seances_modifiees: b.diff.modifiees.len() as i64,
            modifications_conservees: b.modifications_conservees,
            ..Default::default()
        };
        sqlx::query(
            "UPDATE ent_edt_version SET diffusion_familles_le = coalesce(diffusion_familles_le, now()) WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(v.id)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        journaliser(
            tx,
            e,
            v.annee_id,
            v.id,
            "diffusee",
            Some(b.effet),
            None,
            &apercu,
            nb_avertissements,
            forcage,
            decalee || decale_ici,
            a.user_id,
        )
        .await?;
        let ev = publier_agg(
            &mut **tx,
            a,
            AggregateKind::Edt,
            EVT_VERSION_DIFFUSEE,
            v.id,
            json!({ "version_id": v.id, "a_partir_du": b.effet, "au": b.fin, "provisoire": v.provisoire,
                    "nb_creees": apercu.seances_creees, "nb_retirees": apercu.seances_retirees,
                    "nb_modifiees": apercu.seances_modifiees }),
        )
        .await?;
        Ok(vec![b.evenement, ev])
    }

    // ───────────────────────────── EDT-153 : la diffusion ─────────────────────────────

    /// Diffuse tout de suite une version annoncée (avancer la diffusion).
    pub async fn edt_diffuser(
        &self,
        a: &Acteur<'_>,
        version_id: Uuid,
    ) -> Result<ResultatPublication, EntDbError> {
        self.exiger_edt(a, Action::EdtPublier).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let v = charger_version(&mut tx, e, version_id, true).await?;
        if v.etat == "publiee" {
            return Ok(ResultatPublication {
                version_id,
                applique: false,
                deja_fait: true,
                bloquants: vec![],
                avertissements: vec![],
                effet: v.valable_du,
                decalee: false,
                diffusee: true,
                diffusion_le: v.diffusion_familles_le,
                apercu: None,
            });
        }
        let ev = self
            .diffuser_dans(&mut tx, a, version_id, None, false, 0)
            .await?;
        let v2 = charger_version(&mut tx, e, version_id, false).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&ev);
        Ok(ResultatPublication {
            version_id,
            applique: true,
            deja_fait: false,
            bloquants: vec![],
            avertissements: vec![],
            effet: v2.valable_du,
            decalee: v2.valable_du != v.valable_du,
            diffusee: true,
            diffusion_le: v2.diffusion_familles_le,
            apercu: None,
        })
    }

    /// Avance ou retarde la diffusion d'une version annoncée. Une date passée diffuse tout de suite ; au plus tard la
    /// veille de l'effet (sinon les familles seraient prévenues le jour même).
    pub async fn edt_diffusion_regler(
        &self,
        a: &Acteur<'_>,
        version_id: Uuid,
        nouvelle: DateTime<Utc>,
    ) -> Result<ResultatPublication, EntDbError> {
        self.exiger_edt(a, Action::EdtPublier).await?;
        if nouvelle <= Utc::now() {
            return self.edt_diffuser(a, version_id).await;
        }
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let v = charger_version(&mut tx, e, version_id, true).await?;
        if !v.annoncee() {
            return Err(conflit());
        }
        let effet = v.valable_du.ok_or_else(invalide)?;
        let jour: NaiveDate = sqlx::query_scalar(
            "SELECT ($2::timestamptz AT TIME ZONE coalesce((SELECT fuseau FROM ent_etablissement_parametre
                                                              WHERE etablissement_id = $1), 'Europe/Paris'))::date",
        )
        .bind(e)
        .bind(nouvelle)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if jour >= effet {
            return Err(invalide());
        }
        sqlx::query(
            "UPDATE ent_edt_version SET diffusion_familles_le = $3, modifie_le = now() WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(version_id)
        .bind(nouvelle)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            e,
            v.annee_id,
            version_id,
            "diffusion_reglee",
            Some(effet),
            Some(nouvelle),
            &ApercuPublication::default(),
            0,
            None,
            false,
            a.user_id,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ResultatPublication {
            version_id,
            applique: true,
            deja_fait: false,
            bloquants: vec![],
            avertissements: vec![],
            effet: Some(effet),
            decalee: false,
            diffusee: false,
            diffusion_le: Some(nouvelle),
            apercu: None,
        })
    }

    /// Diffuse les versions annoncées dont l'heure de diffusion est venue. À appeler par la tâche de fond (le recalcul de
    /// la fenêtre l'appelle). Chaque version dans sa transaction : une qui échoue n'empêche pas les autres.
    pub async fn edt_diffuser_echues(&self, a: &Acteur<'_>) -> Result<Vec<Uuid>, EntDbError> {
        self.exiger_edt(a, Action::EdtPublier).await?;
        let e = a.etablissement_id;
        let echues: Vec<Uuid> = {
            let mut tx = TenantTx::begin(self.pool(), e).await?;
            sqlx::query_scalar(
                "SELECT id FROM ent_edt_version
                  WHERE etablissement_id = $1 AND etat = 'brouillon' AND annoncee_le IS NOT NULL
                    AND diffusion_familles_le IS NOT NULL AND diffusion_familles_le <= now()
                  ORDER BY valable_du, id",
            )
            .bind(e)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
        };
        let mut faites = Vec::new();
        for id in echues {
            if self.edt_diffuser(a, id).await.is_ok() {
                faites.push(id);
            }
        }
        Ok(faites)
    }

    /// Retire l'annonce d'une version **non encore diffusée** : elle redevient un brouillon modifiable.
    pub async fn edt_annonce_retirer(
        &self,
        a: &Acteur<'_>,
        version_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_edt(a, Action::EdtRevenirVersion).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let v = charger_version(&mut tx, e, version_id, true).await?;
        if !v.annoncee() {
            return Err(conflit());
        }
        sqlx::query(
            "UPDATE ent_edt_version
                SET annoncee_le = NULL, annoncee_par = NULL, valable_du = NULL, valable_au = NULL,
                    diffusion_familles_le = NULL, empreinte_controles = NULL, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(version_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            e,
            v.annee_id,
            version_id,
            "annonce_retiree",
            v.valable_du,
            None,
            &ApercuPublication::default(),
            0,
            None,
            false,
            a.user_id,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────────── EDT-154 : historique et différences ─────────────────────────────

    /// Historique des versions de l'année, avec les faits de publication (la plus récente d'abord).
    pub async fn edt_publications_historique(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
    ) -> Result<Vec<VersionHistorique>, EntDbError> {
        self.exiger_edt(a, Action::EdtPublicationLire).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        #[derive(FromRow)]
        struct V {
            id: Uuid,
            libelle: String,
            nature: String,
            etat: String,
            valable_du: Option<NaiveDate>,
            valable_au: Option<NaiveDate>,
            provisoire: bool,
            annoncee_le: Option<DateTime<Utc>>,
            diffusion_familles_le: Option<DateTime<Utc>>,
            publiee_le: Option<DateTime<Utc>>,
            remplace_version_id: Option<Uuid>,
            copie_de_version_id: Option<Uuid>,
        }
        let versions: Vec<V> = sqlx::query_as(
            "SELECT id, libelle, nature, etat, valable_du, valable_au, provisoire, annoncee_le, diffusion_familles_le,
                    publiee_le, remplace_version_id, copie_de_version_id
               FROM ent_edt_version
              WHERE etablissement_id = $1 AND annee_id = $2 AND etat <> 'abandonnee'
              ORDER BY cree_le DESC, id",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let faits: Vec<(Uuid, Uuid, String, Option<NaiveDate>, Option<DateTime<Utc>>, i32, i32, i32, i32, i32, i32, Option<String>, bool, Uuid, DateTime<Utc>)> =
            sqlx::query_as(
                "SELECT id, version_id, fait, date_effet, diffusion_le, nb_creees, nb_retirees, nb_conservees, nb_modifiees,
                        nb_modifications_conservees, nb_avertissements, forcage_motif, decalee, par, le
                   FROM ent_edt_version_journal WHERE etablissement_id = $1 AND annee_id = $2 ORDER BY le, id",
            )
            .bind(e)
            .bind(annee_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut par_version: HashMap<Uuid, Vec<FaitPublication>> = HashMap::new();
        for f in faits {
            par_version.entry(f.1).or_default().push(FaitPublication {
                id: f.0,
                fait: f.2,
                date_effet: f.3,
                diffusion_le: f.4,
                nb_creees: f.5,
                nb_retirees: f.6,
                nb_conservees: f.7,
                nb_modifiees: f.8,
                nb_modifications_conservees: f.9,
                nb_avertissements: f.10,
                forcage_motif: f.11,
                decalee: f.12,
                par: f.13,
                le: f.14,
            });
        }
        Ok(versions
            .into_iter()
            .map(|v| VersionHistorique {
                faits: par_version.remove(&v.id).unwrap_or_default(),
                id: v.id,
                libelle: v.libelle,
                nature: v.nature,
                etat: v.etat,
                valable_du: v.valable_du,
                valable_au: v.valable_au,
                provisoire: v.provisoire,
                annoncee_le: v.annoncee_le,
                diffusion_familles_le: v.diffusion_familles_le,
                publiee_le: v.publiee_le,
                remplace_version_id: v.remplace_version_id,
                copie_de_version_id: v.copie_de_version_id,
            })
            .collect())
    }

    /// Différences entre deux versions de la même année : cours ajoutés, retirés, modifiés (positions avant et après),
    /// par intervenant et par classe. Lecture seule ; jamais un nom de personne.
    pub async fn edt_versions_difference(
        &self,
        a: &Acteur<'_>,
        de: Uuid,
        vers: Uuid,
    ) -> Result<DifferencesVersions, EntDbError> {
        self.exiger_edt(a, Action::EdtPublicationLire).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let v_de = charger_version(&mut tx, e, de, false).await?;
        let v_vers = charger_version(&mut tx, e, vers, false).await?;
        if v_de.annee_id != v_vers.annee_id {
            return Err(invalide());
        }
        let grille = grille_de(&mut tx, e).await?;
        let sites = sites_des_cours(&mut tx, e, v_de.annee_id).await?;
        let mut plans: Vec<BTreeMap<Uuid, Vec<PositionLisible>>> = Vec::new();
        for v in [de, vers] {
            let mut plan: BTreeMap<Uuid, Vec<PositionLisible>> = BTreeMap::new();
            for p in placements_de(&mut tx, e, &[v]).await? {
                let site = sites.get(&p.cours_id).copied().flatten();
                let Some(f) = fenetre_du_placement(&p, &grille, site) else {
                    continue;
                };
                plan.entry(p.cours_id).or_default().push(PositionLisible {
                    jour_semaine: p.jour_semaine,
                    debut: f.debut,
                    fin: f.fin,
                    semaine_type: p.semaine_type.clone(),
                    lieux: p.lieux.clone(),
                });
            }
            plans.push(plan);
        }
        let (ajoutes, retires, modifies, inchanges) = comparer_plans(&plans[0], &plans[1]);
        let touches: Vec<Uuid> = ajoutes
            .iter()
            .chain(&retires)
            .chain(&modifies)
            .copied()
            .collect();
        let infos: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
            "SELECT c.id, c.libelle,
                    (SELECT cl.code FROM ent_classe cl WHERE cl.etablissement_id = c.etablissement_id AND cl.id = COALESCE(
                        c.classe_id, (SELECT g.classe_id FROM ent_groupe g WHERE g.id = c.groupe_id AND g.etablissement_id = c.etablissement_id)))
               FROM ent_cours c WHERE c.etablissement_id = $1 AND c.id = ANY($2)",
        )
        .bind(e)
        .bind(&touches)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let info: HashMap<Uuid, (String, Option<String>)> =
            infos.into_iter().map(|(c, l, k)| (c, (l, k))).collect();
        let interv: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT s.cours_id, s.prof_user_id FROM ent_service_enseignement s
              WHERE s.etablissement_id = $1 AND s.cours_id = ANY($2) AND s.etat = 'ouvert' AND s.prof_user_id IS NOT NULL",
        )
        .bind(e)
        .bind(&touches)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut interv_de: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for (c, u) in interv {
            interv_de.entry(c).or_default().push(u);
        }
        let ligne = |c: Uuid| DifferenceCours {
            cours_id: c,
            libelle: info.get(&c).map(|x| x.0.clone()).unwrap_or_default(),
            classe: info.get(&c).and_then(|x| x.1.clone()),
            intervenants: interv_de.get(&c).cloned().unwrap_or_default(),
            avant: plans[0].get(&c).cloned().unwrap_or_default(),
            apres: plans[1].get(&c).cloned().unwrap_or_default(),
        };
        let mut par_interv: BTreeMap<Uuid, i64> = BTreeMap::new();
        let mut par_classe: BTreeMap<String, i64> = BTreeMap::new();
        for c in &touches {
            for u in interv_de.get(c).into_iter().flatten() {
                *par_interv.entry(*u).or_default() += 1;
            }
            if let Some(k) = info.get(c).and_then(|x| x.1.clone()) {
                *par_classe.entry(k).or_default() += 1;
            }
        }
        Ok(DifferencesVersions {
            de,
            vers,
            ajoutes: ajoutes.into_iter().map(ligne).collect(),
            retires: retires.into_iter().map(ligne).collect(),
            modifies: modifies.into_iter().map(ligne).collect(),
            inchanges,
            par_intervenant: par_interv.into_iter().collect(),
            par_classe: par_classe.into_iter().collect(),
        })
    }

    // ───────────────────────────── EDT-155 : revenir en arrière ─────────────────────────────

    /// Revient à la version précédente. Version annoncée non diffusée : l'annonce est retirée. Version publiée dont l'effet
    /// n'a pas commencé : la version remplacée est rétablie, la nouvelle est archivée. Version en vigueur : une **copie**
    /// de la version précédente est publiée à partir de `a_partir_du` (au plus tôt demain) : le passé ne change pas.
    pub async fn edt_retour_arriere(
        &self,
        a: &Acteur<'_>,
        version_id: Uuid,
        a_partir_du: Option<NaiveDate>,
    ) -> Result<ResultatPublication, EntDbError> {
        self.exiger_edt(a, Action::EdtRevenirVersion).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 3))")
            .bind(format!("edt-publication-{e}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let v = charger_version(&mut tx, e, version_id, true).await?;
        if v.annoncee() {
            drop(tx);
            self.edt_annonce_retirer(a, version_id).await?;
            return Ok(ResultatPublication {
                version_id,
                applique: true,
                deja_fait: false,
                bloquants: vec![],
                avertissements: vec![],
                effet: None,
                decalee: false,
                diffusee: false,
                diffusion_le: None,
                apercu: None,
            });
        }
        if v.etat != "publiee" || v.nature != "base" {
            return Err(conflit());
        }
        let precedente = v.remplace_version_id.ok_or_else(conflit)?;
        let annee = charger_annee(&mut tx, e, v.annee_id).await?;
        let aujourdhui = aujourdhui_tx(&mut tx, e).await?;
        let effet_v = v.valable_du.ok_or_else(invalide)?;
        if effet_v > aujourdhui {
            // L'effet n'a pas commencé : rétablir la version remplacée, archiver celle-ci.
            let prev = charger_version(&mut tx, e, precedente, true).await?;
            sqlx::query(
                "UPDATE ent_edt_version SET etat = 'archivee', retiree_le = now(), retiree_par = $3, modifie_le = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(e)
            .bind(version_id)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "UPDATE ent_edt_version SET etat = $3, valable_au = $4, modifie_le = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(e)
            .bind(precedente)
            .bind(v.remplace_etat_avant.as_deref().unwrap_or("publiee"))
            .bind(v.remplace_valable_au_avant.or(Some(annee.fin)))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            let (rapport, ev) = self
                .generer_seances_dans(&mut tx, a, v.annee_id, Some(effet_v), Some(annee.fin), None)
                .await?;
            let apercu = ApercuPublication {
                seances_creees: rapport.creees,
                seances_retirees: rapport.supprimees + rapport.retirees,
                ..Default::default()
            };
            journaliser(
                &mut tx,
                e,
                v.annee_id,
                version_id,
                "retour_arriere",
                Some(effet_v),
                None,
                &apercu,
                0,
                None,
                false,
                a.user_id,
            )
            .await?;
            tx.commit().await.map_err(db)?;
            self.diffuser_apres_commit(&[ev]);
            let _ = prev;
            return Ok(ResultatPublication {
                version_id: precedente,
                applique: true,
                deja_fait: false,
                bloquants: vec![],
                avertissements: vec![],
                effet: Some(effet_v),
                decalee: false,
                diffusee: true,
                diffusion_le: None,
                apercu: Some(apercu),
            });
        }
        // En vigueur : copie de la version précédente, publiée à une date future (immédiatement).
        let (effet, decalee) = date_d_effet(a_partir_du, aujourdhui, false);
        let copie = copier_version(&mut tx, a, precedente, version_id).await?;
        let vc = charger_version(&mut tx, e, copie, true).await?;
        let controles = controler_tx(&mut tx, e, &vc, annee, effet, None).await?;
        if !controles.bloquants.is_empty() {
            drop(tx);
            return Ok(ResultatPublication {
                version_id: copie,
                applique: false,
                deja_fait: false,
                bloquants: controles.bloquants,
                avertissements: controles.avertissements,
                effet: Some(effet),
                decalee,
                diffusee: false,
                diffusion_le: None,
                apercu: None,
            });
        }
        sqlx::query(
            "UPDATE ent_edt_version SET annoncee_le = now(), annoncee_par = $3, valable_du = $4, empreinte_controles = $5,
                    modifie_le = now() WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(copie)
        .bind(a.user_id)
        .bind(effet)
        .bind(&controles.empreinte)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let mut evenements = vec![
            publier_agg(
                &mut tx,
                a,
                AggregateKind::Edt,
                EVT_VERSION_PUBLIEE,
                copie,
                json!({ "version_id": copie, "a_partir_du": effet, "diffusion_le": null,
                        "provisoire": false, "deux_temps": false, "retour_arriere": true }),
            )
            .await?,
        ];
        evenements.extend(
            self.diffuser_dans(
                &mut tx,
                a,
                copie,
                None,
                decalee,
                controles.avertissements.len() as i32,
            )
            .await?,
        );
        journaliser(
            &mut tx,
            e,
            v.annee_id,
            copie,
            "copie_retour",
            Some(effet),
            None,
            &ApercuPublication::default(),
            0,
            None,
            decalee,
            a.user_id,
        )
        .await?;
        sqlx::query(
            "UPDATE ent_edt_version SET retiree_le = now(), retiree_par = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(version_id)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evenements);
        Ok(ResultatPublication {
            version_id: copie,
            applique: true,
            deja_fait: false,
            bloquants: vec![],
            avertissements: controles.avertissements,
            effet: Some(effet),
            decalee,
            diffusee: true,
            diffusion_le: None,
            apercu: None,
        })
    }

    // ───────────────────────────── EDT-156 : version individuelle ─────────────────────────────

    /// Publie une version **individuelle** (élève UPE2A, aménagement) sans republier l'établissement : des inclusions et des
    /// exclusions datées de cours (socle 5). Refus si l'ajout crée un conflit d'horaire dans l'emploi du temps de l'élève.
    pub async fn edt_publier_individuelle(
        &self,
        a: &Acteur<'_>,
        cmd: &CmdIndividuelle,
    ) -> Result<ResultatIndividuelle, EntDbError> {
        self.exiger_edt(a, Action::EdtPublier).await?;
        const MOTIFS: [&str; 10] = [
            "ulis",
            "upe2a",
            "decloisonnement",
            "avance",
            "parcours_amenage",
            "scolarite_partagee",
            "echange",
            "accueil_temporaire",
            "dispense_cours",
            "autre",
        ];
        if !MOTIFS.contains(&cmd.motif_code.as_str())
            || (cmd.ajouter.is_empty() && cmd.retirer.is_empty())
            || cmd.ajouter.iter().any(|c| cmd.retirer.contains(c))
            || cmd
                .cle
                .as_ref()
                .is_some_and(|c| c.is_empty() || c.len() > 80)
        {
            return Err(invalide());
        }
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        if let Some(cle) = &cmd.cle {
            let deja: Option<(Uuid, NaiveDate)> = sqlx::query_as(
                "SELECT id, a_partir_du FROM ent_edt_individuelle WHERE etablissement_id = $1 AND cle = $2",
            )
            .bind(e)
            .bind(cle)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some((id, du)) = deja {
                return Ok(ResultatIndividuelle {
                    id: Some(id),
                    applique: false,
                    deja_fait: true,
                    conflits: vec![],
                    effet: Some(du),
                });
            }
        }
        let aujourdhui = aujourdhui_tx(&mut tx, e).await?;
        let (effet, _) = date_d_effet(cmd.a_partir_du, aujourdhui, false);
        // L'élève et les cours sont de l'établissement ; les cours sont ceux de l'année en cours à cette date.
        let eleve_ok: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(e)
        .bind(cmd.eleve_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let tous: Vec<Uuid> = cmd
            .ajouter
            .iter()
            .chain(cmd.retirer.iter())
            .copied()
            .collect();
        let cours: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT c.id, c.annee_id FROM ent_cours c
               JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.id = ANY($2) AND y.date_debut <= $3 AND y.date_fin >= $3",
        )
        .bind(e)
        .bind(&tous)
        .bind(effet)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if !eleve_ok || cours.len() != tous.iter().collect::<HashSet<_>>().len() {
            return Err(introuvable());
        }
        // Conflit d'horaire : un cours ajouté ne doit pas recouvrir un cours que l'élève suit (et ne quitte pas).
        let annee_id = cours[0].1;
        let grille = grille_de(&mut tx, e).await?;
        let sites = sites_des_cours(&mut tx, e, annee_id).await?;
        let suivis: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT cours_id FROM ent_public_cours_jour($1, $2, NULL, NULL, $3)",
        )
        .bind(e)
        .bind(effet)
        .bind(cmd.eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let restent: Vec<Uuid> = suivis
            .into_iter()
            .filter(|c| !cmd.retirer.contains(c))
            .collect();
        let versions: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_edt_version
              WHERE etablissement_id = $1 AND annee_id = $2 AND etat = 'publiee' AND nature = 'base'
                AND valable_du <= $3 AND coalesce(valable_au, $3) >= $3",
        )
        .bind(e)
        .bind(annee_id)
        .bind(effet)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let placements = placements_de(&mut tx, e, &versions).await?;
        let fen = |p: &PlacementLigne| {
            fenetre_du_placement(p, &grille, sites.get(&p.cours_id).copied().flatten())
        };
        let mut conflits_p: Vec<Uuid> = Vec::new();
        let mut conflits_c: Vec<Uuid> = Vec::new();
        for ajout in placements
            .iter()
            .filter(|p| cmd.ajouter.contains(&p.cours_id))
        {
            let Some(fa) = fen(ajout) else { continue };
            for autre in placements.iter().filter(|p| restent.contains(&p.cours_id)) {
                let Some(fb) = fen(autre) else { continue };
                if ajout.jour_semaine == autre.jour_semaine
                    && crate::edt_versions::semaines_se_recoupent(
                        &ajout.semaine_type,
                        &autre.semaine_type,
                    )
                    && fa.chevauche(&fb)
                {
                    conflits_p.push(ajout.id);
                    conflits_c.push(ajout.cours_id);
                }
            }
        }
        if !conflits_p.is_empty() {
            conflits_p.sort();
            conflits_p.dedup();
            return Ok(ResultatIndividuelle {
                id: None,
                applique: false,
                deja_fait: false,
                conflits: vec![controle("conflit_eleve", conflits_c, conflits_p)],
                effet: Some(effet),
            });
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_edt_individuelle (id, etablissement_id, eleve_id, motif_code, a_partir_du, au, cle, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(e)
        .bind(cmd.eleve_id)
        .bind(&cmd.motif_code)
        .bind(effet)
        .bind(cmd.au)
        .bind(&cmd.cle)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (liste, nature) in [(&cmd.ajouter, "inclusion"), (&cmd.retirer, "exclusion")] {
            for cours_id in liste {
                let membre = Uuid::new_v4();
                // Le déclencheur du socle 5 refuse un chevauchement de la même nature, une période hors de l'année.
                sqlx::query(
                    "INSERT INTO ent_cours_membre (id, etablissement_id, cours_id, eleve_id, nature, motif_code, du, au, cree_par)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
                )
                .bind(membre)
                .bind(e)
                .bind(cours_id)
                .bind(cmd.eleve_id)
                .bind(nature)
                .bind(&cmd.motif_code)
                .bind(effet)
                .bind(cmd.au)
                .bind(a.user_id)
                .execute(&mut *tx)
                .await
                .map_err(|x| if matches!(&x, sqlx::Error::Database(d) if d.code().as_deref() == Some("23514") || d.code().as_deref() == Some("23P01")) { conflit() } else { db(x) })?;
                sqlx::query(
                    "INSERT INTO ent_edt_individuelle_ligne (individuelle_id, etablissement_id, cours_id, nature, membre_id)
                     VALUES ($1,$2,$3,$4,$5)",
                )
                .bind(id)
                .bind(e)
                .bind(cours_id)
                .bind(nature)
                .bind(membre)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            EVT_VERSION_INDIVIDUELLE,
            id,
            // Ni motif ni identité dans la charge utile (données sensibles) : le consommateur relit tout.
            json!({ "individuelle_id": id, "a_partir_du": effet }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ResultatIndividuelle {
            id: Some(id),
            applique: true,
            deja_fait: false,
            conflits: vec![],
            effet: Some(effet),
        })
    }
}

/// Écrit un fait de publication dans le journal (ajout seul).
async fn journaliser(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    annee_id: Uuid,
    version_id: Uuid,
    fait: &str,
    date_effet: Option<NaiveDate>,
    diffusion_le: Option<DateTime<Utc>>,
    c: &ApercuPublication,
    nb_avertissements: i32,
    forcage: Option<&str>,
    decalee: bool,
    par: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_edt_version_journal
           (id, etablissement_id, annee_id, version_id, fait, date_effet, diffusion_le, nb_creees, nb_retirees,
            nb_conservees, nb_modifiees, nb_modifications_conservees, nb_avertissements, forcage_motif, decalee, par, le)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16, clock_timestamp())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(annee_id)
    .bind(version_id)
    .bind(fait)
    .bind(date_effet)
    .bind(diffusion_le)
    .bind(i32::try_from(c.seances_creees).unwrap_or(i32::MAX))
    .bind(i32::try_from(c.seances_retirees).unwrap_or(i32::MAX))
    .bind(i32::try_from(c.seances_conservees).unwrap_or(i32::MAX))
    .bind(i32::try_from(c.seances_modifiees).unwrap_or(i32::MAX))
    .bind(i32::try_from(c.modifications_conservees).unwrap_or(i32::MAX))
    .bind(nb_avertissements)
    .bind(forcage)
    .bind(decalee)
    .bind(par)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Copie une version (besoins, pièces, placements et leurs lieux) en brouillon. Les placements gardent leur créneau
/// (identité stable des séances).
async fn copier_version(
    tx: &mut TenantTx<'_>,
    a: &Acteur<'_>,
    source: Uuid,
    remplacee: Uuid,
) -> Result<Uuid, EntDbError> {
    let e = a.etablissement_id;
    let nouvelle = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_version (id, etablissement_id, annee_id, libelle, nature, etat, origine, cree_par, copie_de_version_id)
         SELECT $3, etablissement_id, annee_id, left('Retour à : ' || libelle, 160), 'base', 'brouillon', 'saisie', $4, id
           FROM ent_edt_version WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(e)
    .bind(source)
    .bind(nouvelle)
    .bind(a.user_id)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    let _ = remplacee;
    let besoins: Vec<(Uuid, Uuid, Uuid, f64, String)> = sqlx::query_as(
        "SELECT id, annee_id, cours_id, heures_hebdo::float8, source FROM ent_edt_besoin
          WHERE etablissement_id = $1 AND version_id = $2",
    )
    .bind(e)
    .bind(source)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let mut besoin_neuf: HashMap<Uuid, Uuid> = HashMap::new();
    for (id, annee, cours, h, src) in besoins {
        let n = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_edt_besoin (id, etablissement_id, version_id, annee_id, cours_id, heures_hebdo, source)
             VALUES ($1,$2,$3,$4,$5,$6::float8::numeric,$7)",
        )
        .bind(n)
        .bind(e)
        .bind(nouvelle)
        .bind(annee)
        .bind(cours)
        .bind(h)
        .bind(src)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        besoin_neuf.insert(id, n);
    }
    #[derive(FromRow)]
    struct Piece {
        id: Uuid,
        besoin_id: Uuid,
        duree_creneaux: i16,
        semaine_type: String,
        consecutif: bool,
        type_lieu: Option<String>,
        lieu_impose_id: Option<Uuid>,
        ordre: i16,
    }
    let pieces: Vec<Piece> = sqlx::query_as(
        "SELECT p.id, p.besoin_id, p.duree_creneaux, p.semaine_type, p.consecutif, p.type_lieu, p.lieu_impose_id, p.ordre
           FROM ent_edt_piece p JOIN ent_edt_besoin b ON b.id = p.besoin_id AND b.etablissement_id = p.etablissement_id
          WHERE p.etablissement_id = $1 AND b.version_id = $2",
    )
    .bind(e)
    .bind(source)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    let mut piece_neuve: HashMap<Uuid, Uuid> = HashMap::new();
    for p in pieces {
        let Some(b) = besoin_neuf.get(&p.besoin_id) else {
            continue;
        };
        let n = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_edt_piece (id, etablissement_id, besoin_id, duree_creneaux, semaine_type, consecutif, type_lieu,
                                         lieu_impose_id, ordre)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(n)
        .bind(e)
        .bind(b)
        .bind(p.duree_creneaux)
        .bind(&p.semaine_type)
        .bind(p.consecutif)
        .bind(&p.type_lieu)
        .bind(p.lieu_impose_id)
        .bind(p.ordre)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        piece_neuve.insert(p.id, n);
    }
    let placements: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, piece_id FROM ent_edt_placement WHERE etablissement_id = $1 AND version_id = $2",
    )
    .bind(e)
    .bind(source)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    for (pid, piece) in placements {
        let Some(np) = piece_neuve.get(&piece) else {
            continue;
        };
        let n = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_edt_placement
               (id, etablissement_id, version_id, piece_id, cours_id, jour_semaine, creneau_debut_code, duree_creneaux,
                heure_debut_libre, heure_fin_libre, semaine_type, verrouille, origine, creneau_id, cree_par)
             SELECT $3, etablissement_id, $4, $5, cours_id, jour_semaine, creneau_debut_code, duree_creneaux,
                    heure_debut_libre, heure_fin_libre, semaine_type, verrouille, 'main', creneau_id, $6
               FROM ent_edt_placement WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(pid)
        .bind(n)
        .bind(nouvelle)
        .bind(np)
        .bind(a.user_id)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_edt_placement_lieu (placement_id, etablissement_id, lieu_id, principal)
             SELECT $3, etablissement_id, lieu_id, principal FROM ent_edt_placement_lieu
              WHERE etablissement_id = $1 AND placement_id = $2",
        )
        .bind(e)
        .bind(pid)
        .bind(n)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    }
    Ok(nouvelle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, j).unwrap()
    }

    fn h(a: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(a, m, 0).unwrap()
    }

    #[test]
    fn jours_ouvres_sautent_le_week_end() {
        // Vendredi 13 novembre 2026 + 2 jours ouvrés = mardi 17.
        assert_eq!(ajouter_jours_ouvres(d(11, 13), 2), d(11, 17));
        assert_eq!(ajouter_jours_ouvres(d(11, 16), 2), d(11, 18));
        assert_eq!(ajouter_jours_ouvres(d(11, 16), 0), d(11, 16));
    }

    #[test]
    fn la_prise_d_effet_est_au_plus_tot_demain_sauf_forcage() {
        let auj = d(11, 16); // lundi
        assert_eq!(date_d_effet(None, auj, false), (d(11, 17), false));
        // Demandée aujourd'hui ou dans le passé : repoussée à demain.
        assert_eq!(date_d_effet(Some(d(11, 16)), auj, false), (d(11, 17), true));
        assert_eq!(date_d_effet(Some(d(11, 2)), auj, false), (d(11, 17), true));
        // Forcée : aujourd'hui, jamais le passé.
        assert_eq!(date_d_effet(Some(d(11, 16)), auj, true), (d(11, 16), false));
        assert_eq!(date_d_effet(Some(d(11, 2)), auj, true), (d(11, 16), false));
        // Dans le futur : telle quelle.
        assert_eq!(date_d_effet(Some(d(12, 1)), auj, false), (d(12, 1), false));
    }

    #[test]
    fn la_date_d_effet_suit_le_jour_de_paris_pas_le_jour_utc() {
        use chrono::TimeZone;
        // 23 h 30 UTC le 15 novembre = 0 h 30 le 16 à Paris : « aujourd'hui » est le 16, l'effet au plus tôt le 17.
        let t = Utc.with_ymd_and_hms(2026, 11, 15, 23, 30, 0).unwrap();
        let paris = domain_eleve::horloge::date_paris(t);
        assert_eq!(paris, d(11, 16));
        assert_eq!(
            date_d_effet(Some(d(11, 16)), paris, false),
            (d(11, 17), true)
        );
        // En heure d'été (UTC+2) : 22 h 30 UTC le 30 mai = 0 h 30 le 31.
        let t = Utc.with_ymd_and_hms(2027, 5, 30, 22, 30, 0).unwrap();
        assert_eq!(
            domain_eleve::horloge::date_paris(t),
            NaiveDate::from_ymd_opt(2027, 5, 31).unwrap()
        );
    }

    fn ligne(id: Uuid, jour: NaiveDate, debut: NaiveTime, salle: Option<&str>) -> LigneSeance {
        LigneSeance {
            id,
            cours_id: None,
            classe_id: None,
            prof_user_id: Uuid::nil(),
            jour,
            heure_debut: debut,
            heure_fin: debut + Duration::hours(1),
            salle: salle.map(str::to_string),
        }
    }

    #[test]
    fn la_comparaison_des_seances_distingue_creees_retirees_conservees_et_modifiees() {
        let (a, b, c, e) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let avant = [
            ligne(a, d(11, 20), h(8, 0), Some("B12")),
            ligne(b, d(11, 21), h(9, 0), Some("B12")),
            ligne(c, d(11, 22), h(10, 0), None),
        ];
        let apres = [
            ligne(a, d(11, 20), h(8, 0), Some("B12")),
            ligne(b, d(11, 21), h(9, 0), Some("C3")),
            ligne(e, d(11, 23), h(8, 0), None),
        ];
        let diff = comparer_seances(&avant, &apres);
        assert_eq!(diff.conservees, vec![a]);
        assert_eq!(diff.modifiees, vec![b]);
        assert_eq!(diff.creees, vec![e]);
        assert_eq!(diff.retirees, vec![c]);
    }

    fn pos(jour: i16, debut: u32) -> PositionLisible {
        PositionLisible {
            jour_semaine: jour,
            debut: h(debut, 0),
            fin: h(debut + 1, 0),
            semaine_type: "toutes".into(),
            lieux: vec![],
        }
    }

    #[test]
    fn la_comparaison_des_plans_classe_les_cours() {
        let (stable, deplace, nouveau, supprime) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let de: BTreeMap<_, _> = [
            (stable, vec![pos(1, 8), pos(3, 9)]),
            (deplace, vec![pos(2, 8)]),
            (supprime, vec![pos(4, 8)]),
        ]
        .into_iter()
        .collect();
        // L'ordre des positions ne compte pas.
        let vers: BTreeMap<_, _> = [
            (stable, vec![pos(3, 9), pos(1, 8)]),
            (deplace, vec![pos(2, 9)]),
            (nouveau, vec![pos(5, 8)]),
        ]
        .into_iter()
        .collect();
        let (ajoutes, retires, modifies, inchanges) = comparer_plans(&de, &vers);
        assert_eq!(
            (ajoutes, retires, modifies, inchanges),
            (vec![nouveau], vec![supprime], vec![deplace], 1)
        );
    }
}
