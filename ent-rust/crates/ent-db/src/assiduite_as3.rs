//! Socle 12, lot AS3 : épisodes d'absence et qualification (`docs/socles/12_ASSIDUITE.md` §8.3, §8.4,
//! §10, §17.3, §22.4, R-AS3, R-AS5, R-AS8, R-AS19, R-AS21, R-AS22, R-AS35).
//!
//! Ce que le lot ajoute :
//! - **épisodes** (`ent_asd_absence`) : à la validation d'un appel, chaque ligne `absent` est rattachée à
//!   l'épisode ouvert du même élève si **aucune séance où il était présent ne les sépare** (contiguïté,
//!   §8.3), sinon un nouvel épisode est ouvert ; une séance n'appartient qu'à un épisode ; les
//!   déclarations qui couvrent la séance sont rapprochées (épisode « prévenu ») ;
//! - **qualification** à quatre issues (`reputee_legitime` réservée aux cinq natures de L131-8,
//!   `excuse_valable`, `non_valable`, `sans_motif`), **requalification** avec motif obligatoire,
//!   **qualification en masse** (aperçu puis confirmation), **proposition** de l'AED, note interne et
//!   libellé famille à liste fermée ;
//! - **justificatifs** : pièce facultative dans `ent-object` (clé générée par le serveur, type contrôlé
//!   par signature binaire, aucun nom de fichier conservé), texte seul, « vu non conservé », classement
//!   C3 des pièces médicales, lecture journalisée, lien pré-signé de 5 minutes au plus ;
//! - **demandes d'autorisation d'absence** (AS-125) décidées par la direction ;
//! - **projection** vers l'ancien circuit (`ent_absence.statut`, lu par la famille et le décompte
//!   jusqu'au lot AS4) : une qualification réécrit l'ancien statut, et une décision de l'ancien
//!   circuit s'applique aux épisodes (déclencheur SQL de la migration 2244).
//!
//! Minimisation : le professeur ne voit que l'état de régularisation (« à régulariser » /
//! « régularisée »), jamais un motif, une nature, une pièce ni une note ; l'AED voit la **nature** d'un
//! motif mais pas le texte libre (R-AS19) ; la famille voit un libellé à liste fermée, jamais la note
//! interne (R-AS22). Les événements ne portent que des codes et des identifiants (R-AS25).

use crate::appel_repo::Appel;
use crate::cahier::FichierDepot;
use crate::outbox::{Causalite, Publier};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use crate::vie_scolaire::{db, introuvable};
use chrono::{Datelike, NaiveDate, NaiveTime};
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEvent, MetierEventDraft};
use ent_object::presign::{Disposition, LinkSpec, PresignError};
use ent_object::upload::UsageFichier;
use ent_object::ObjectStore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

/// Types d'événements du lot (codes et identifiants seulement, R-AS25).
pub mod types_as3 {
    pub const ABSENCE_QUALIFIEE: &str = "assiduite.absence_qualifiee";
    pub const ABSENCE_REQUALIFIEE: &str = "assiduite.absence_requalifiee";
    pub const QUALIFICATION_PROPOSEE: &str = "assiduite.qualification_proposee";
    pub const JUSTIFICATIF_DEPOSE: &str = "assiduite.justificatif_depose";
    pub const AUTORISATION_DEMANDEE: &str = "assiduite.autorisation_demandee";
    pub const AUTORISATION_DECIDEE: &str = "assiduite.autorisation_decidee";
}

/// Issues d'une qualification.
pub const QUALIFICATIONS: [&str; 4] = ["reputee_legitime", "excuse_valable", "non_valable", "sans_motif"];
/// Libellés famille (liste fermée, D-023) : jamais de texte libre, jamais la note interne.
pub const LIBELLES_FAMILLE: [&str; 6] = [
    "absence_justifiee",
    "absence_excusee",
    "motif_non_retenu",
    "motif_non_retenu_delai",
    "motif_non_retenu_piece",
    "motif_non_communique",
];
/// Motifs d'une requalification (obligatoires, codés).
pub const MOTIFS_REQUALIFICATION: [&str; 5] =
    ["erreur_de_saisie", "nouvelle_piece", "decision_direction", "changement_de_nature", "autre"];
/// Décisions de refus d'une demande d'autorisation.
const REFUS_DEMANDE: [&str; 4] =
    ["refusee_motif_non_recevable", "refusee_absences_repetees", "refusee_delai", "refusee_autre"];
/// Plafonds.
pub const MAX_JUSTIFICATIFS_EPISODE: i64 = 10;
pub const MAX_MASSE: usize = 200;
pub const TTL_MAX_LIEN_JUSTIFICATIF_SECS: u64 = 300;
const MAX_NOTE: usize = 500;

/// Texte affiché à la famille pour un code de libellé (liste fermée).
pub fn libelle_famille_texte(code: &str) -> &'static str {
    match code {
        "absence_justifiee" => "Absence justifiée",
        "absence_excusee" => "Absence excusée",
        "motif_non_retenu" => "Motif non retenu",
        "motif_non_retenu_delai" => "Motif non retenu : justificatif reçu hors délai",
        "motif_non_retenu_piece" => "Motif non retenu : pièce justificative manquante",
        "motif_non_communique" => "Aucun motif communiqué",
        _ => "Absence à régulariser",
    }
}

/// Libellé famille par défaut d'une qualification.
fn libelle_defaut(qualification: &str) -> &'static str {
    match qualification {
        "reputee_legitime" => "absence_justifiee",
        "excuse_valable" => "absence_excusee",
        "non_valable" => "motif_non_retenu",
        _ => "motif_non_communique",
    }
}

/// Un libellé famille est cohérent avec la qualification (pas de « justifiée » pour un motif non valable).
fn libelle_coherent(qualification: &str, libelle: &str) -> bool {
    match qualification {
        "reputee_legitime" | "excuse_valable" => matches!(libelle, "absence_justifiee" | "absence_excusee"),
        "non_valable" => libelle.starts_with("motif_non_retenu"),
        "sans_motif" => libelle == "motif_non_communique",
        _ => false,
    }
}

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

fn conflit(m: &'static str) -> EntDbError {
    EntDbError::ConflitDossier(m)
}

fn court(texte: Option<String>, max: usize) -> Result<Option<String>, EntDbError> {
    match texte.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) {
        Some(t) if t.chars().count() > max => Err(invalide("texte trop long")),
        autre => Ok(autre),
    }
}

// ── DTO ────────────────────────────────────────────────────────────────────────────────────

/// Séance d'un épisode (horaires figés au constat).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
pub struct SeanceEpisode {
    pub id: Uuid,
    pub jour: NaiveDate,
    pub kind: String,
    pub periode: Option<String>,
    pub heure_debut: Option<NaiveTime>,
    pub heure_fin: Option<NaiveTime>,
    pub prevenue: bool,
    /// `true` si le constat a été défait (correction, séance annulée) : la ligne reste, elle ne compte plus.
    pub retiree: bool,
}

/// Épisode vu par la vie scolaire (`absences.lire_motif`). La note interne et les textes libres ne sont
/// remplis que pour qui peut qualifier (R-AS19) ; jamais pour le professeur ni la famille.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpisodeAsd {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eleve_label: Option<String>,
    pub jour_debut: NaiveDate,
    pub jour_fin: NaiveDate,
    pub etat: String,
    pub qualification: String,
    pub nature_code: Option<String>,
    pub motif_id: Option<Uuid>,
    pub prevenue: bool,
    pub source: String,
    pub libelle_famille_code: Option<String>,
    pub version: i32,
    pub seances: Vec<SeanceEpisode>,
    pub justificatifs: i64,
    pub propositions_en_attente: i64,
    /// Textes libres déclarés (famille, accueil) : seulement pour qui peut qualifier.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub textes: Vec<String>,
    /// Dernière note interne : seulement pour qui peut qualifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_interne: Option<String>,
}

/// Une ligne de l'historique d'un épisode.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
pub struct EtatEpisode {
    pub etat: String,
    pub qualification: String,
    pub origine: String,
    pub auteur_id: Option<Uuid>,
    pub motif_code: Option<String>,
    pub cree_le: chrono::DateTime<chrono::Utc>,
}

/// Détail d'un épisode avec son historique (réservé à la vie scolaire).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailEpisode {
    pub episode: EpisodeAsd,
    pub historique: Vec<EtatEpisode>,
    pub propositions: Vec<PropositionAsd>,
}

/// Vue du professeur (R-AS21) : seulement l'état de régularisation, **aucun motif ni nature ni pièce**.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpisodeVueProfesseur {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub jour_debut: NaiveDate,
    pub jour_fin: NaiveDate,
    /// `a_regulariser` ou `regularisee`.
    pub regularisation: String,
}

/// Vue de la famille ou de l'élève majeur : libellé à liste fermée, jamais la note interne (R-AS22).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpisodeVueFamille {
    pub id: Uuid,
    pub jour_debut: NaiveDate,
    pub jour_fin: NaiveDate,
    pub prevenue: bool,
    /// `true` tant que l'épisode attend une décision : la famille peut déposer un justificatif.
    pub a_justifier: bool,
    pub libelle: String,
    pub libelle_code: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FiltreEpisodes {
    pub eleve_id: Option<Uuid>,
    pub depuis: Option<NaiveDate>,
    pub jusqu_au: Option<NaiveDate>,
    /// `a_qualifier` (défaut : ouverts et à qualifier), `qualifie`, `retire` ou `tous`.
    pub etat: Option<String>,
    pub nature_code: Option<String>,
    pub avec_justificatif: Option<bool>,
    pub source: Option<String>,
    pub limite: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QualifierEpisode {
    pub qualification: String,
    /// Version lue : un épisode modifié entre-temps refuse la qualification (conflit).
    pub version: i32,
    pub nature_code: Option<String>,
    pub motif_id: Option<Uuid>,
    pub libelle_famille_code: Option<String>,
    pub note_interne: Option<String>,
    /// Proposition de l'AED acceptée par cette qualification.
    pub proposition_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequalifierEpisode {
    pub qualification: String,
    pub version: i32,
    /// Motif codé, obligatoire (`MOTIFS_REQUALIFICATION`).
    pub motif_code: String,
    pub nature_code: Option<String>,
    pub libelle_famille_code: Option<String>,
    pub note_interne: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SelectionMasse {
    pub absence_ids: Vec<Uuid>,
    pub qualification: String,
    pub libelle_famille_code: Option<String>,
    pub note_interne: Option<String>,
    /// Empreinte rendue par l'aperçu ; absente pour demander l'aperçu.
    pub empreinte: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApercuMasse {
    pub retenus: Vec<Uuid>,
    /// `(épisode, raison codée)` : déjà qualifié, retiré, nature non légitime, introuvable…
    pub ecartes: Vec<(Uuid, String)>,
    pub empreinte: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BilanMasse {
    pub qualifies: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
pub struct PropositionAsd {
    pub id: Uuid,
    pub absence_id: Uuid,
    pub qualification: String,
    pub note: Option<String>,
    pub proposee_par: Uuid,
    pub etat: String,
    pub cree_le: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProposerQualification {
    pub qualification: String,
    pub note: Option<String>,
}

/// Justificatif tel que rendu : jamais le nom d'origine ni la clé d'objet.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
pub struct JustificatifAsd {
    pub id: Uuid,
    pub absence_id: Uuid,
    pub presente: String,
    pub medical: bool,
    pub mime: Option<String>,
    pub taille_octets: Option<i64>,
    pub depose_qualite: String,
    pub conserve_jusqu_au: Option<NaiveDate>,
    pub cree_le: chrono::DateTime<chrono::Utc>,
    pub detruit_le: Option<chrono::DateTime<chrono::Utc>>,
    /// Texte du justificatif « texte seul » : seulement pour qui peut le lire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texte: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DepotJustificatif {
    pub texte: Option<String>,
    /// Pièce médicale ? Défaut : médicale si la nature du motif est médicale.
    pub medical: Option<bool>,
    /// « Certificat vu, non conservé » : aucune pièce stockée (minimisation, AS-128).
    #[serde(default)]
    pub vu_non_conserve: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LienJustificatif {
    pub url: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DemandeAutorisation {
    pub eleve_id: Uuid,
    pub jour_debut: NaiveDate,
    pub jour_fin: NaiveDate,
    pub heure_debut: Option<NaiveTime>,
    pub heure_fin: Option<NaiveTime>,
    pub motif_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
pub struct DemandeAsd {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub type_demande: String,
    pub jour_debut: NaiveDate,
    pub jour_fin: NaiveDate,
    pub heure_debut: Option<NaiveTime>,
    pub heure_fin: Option<NaiveTime>,
    pub motif_id: Option<Uuid>,
    pub etat: String,
    pub decision_code: Option<String>,
    pub cree_le: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DecisionDemande {
    /// `accordee`, `refusee` ou `transmise_dasen`.
    pub decision: String,
    /// Obligatoire pour un refus (`refusee_*`).
    pub decision_code: Option<String>,
}

#[derive(sqlx::FromRow)]
struct EpisodeLigne {
    id: Uuid,
    eleve_id: Uuid,
    jour_debut: NaiveDate,
    jour_fin: NaiveDate,
    etat: String,
    qualification: String,
    nature_code: Option<String>,
    motif_id: Option<Uuid>,
    prevenue: bool,
    source: String,
    libelle_famille_code: Option<String>,
    absence_legacy_id: Option<Uuid>,
    version: i32,
}

const EPISODE_COLONNES: &str = "id, eleve_id, jour_debut, jour_fin, etat, qualification, nature_code, motif_id, \
     prevenue, source, libelle_famille_code, absence_legacy_id, version";

async fn jour_etab(c: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    let (j,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())")
        .bind(etab)
        .fetch_one(c)
        .await
        .map_err(db)?;
    Ok(j)
}

#[allow(clippy::too_many_arguments)]
async fn historiser(
    c: &mut PgConnection,
    etab: Uuid,
    absence_id: Uuid,
    etat: &str,
    qualification: &str,
    origine: &str,
    auteur: Option<Uuid>,
    motif_code: Option<&str>,
    note: Option<&str>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_asd_absence_etat
            (id, etablissement_id, absence_id, etat, qualification, origine, auteur_id, motif_code, note_interne)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(absence_id)
    .bind(etat)
    .bind(qualification)
    .bind(origine)
    .bind(auteur)
    .bind(motif_code)
    .bind(note)
    .execute(c)
    .await
    .map_err(db)?;
    Ok(())
}

async fn publier_asd(
    c: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    aggregate_id: Uuid,
    eleve_id: Uuid,
    payload: serde_json::Value,
    acteur: Option<Uuid>,
    publies: &mut Vec<MetierEvent>,
) -> Result<(), EntDbError> {
    publies.push(
        c.publier_avec(
            MetierEventDraft {
                etablissement_id: etab,
                type_name: type_name.to_string(),
                aggregate: AggregateKind::Absence,
                aggregate_id,
                eleve_id: Some(eleve_id),
                payload,
                actor_user_id: acteur.map(|u| u.to_string()),
            },
            Causalite::default(),
        )
        .await?,
    );
    Ok(())
}

/// Nature médicale d'un motif (classement C3 par défaut d'une pièce).
fn nature_medicale(code: Option<&str>) -> bool {
    matches!(code, Some(c) if c.starts_with("maladie_") || c == "rendez_vous_medical")
}

fn ajouter_un_an(d: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(d.year() + 1, d.month(), d.day())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(d.year() + 1, d.month(), 28).expect("28 existe"))
}

// ── Points d'attache de l'appel (même transaction que la validation ou la correction) ───────────

impl PgEnt {
    /// Rattache la séance d'un élève **absent** à un épisode (§8.3) : épisode ouvert contigu de la même
    /// absence du jour, sinon nouvel épisode ; rapproche les déclarations qui couvrent la séance ;
    /// une autorisation accordée qualifie d'office l'épisode (« excusé »). Idempotent.
    pub(crate) async fn episode_rattacher_seance_tx(
        &self,
        tx: &mut PgConnection,
        appel: &Appel,
        eleve_id: Uuid,
        absence_legacy_id: Uuid,
        actor_user_id: &Option<String>,
        publies: &mut Vec<MetierEvent>,
    ) -> Result<(), EntDbError> {
        let etab = appel.etablissement_id;
        let acteur = actor_user_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
        // La séance telle que l'ancien circuit vient de l'enregistrer (horaires figés).
        let seance: Option<(Uuid, Option<Uuid>, String, NaiveTime, NaiveTime, bool, String)> = sqlx::query_as(
            "SELECT id, seance_ref, seance_source, heure_debut, heure_fin, prevenue, source
               FROM ent_absence_seance
              WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3 AND kind = 'seance'
                AND heure_debut IS NOT NULL
              ORDER BY created_at DESC, id LIMIT 1",
        )
        .bind(etab)
        .bind(appel.id)
        .bind(eleve_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((seance_id, seance_ref, seance_source, debut, fin, prevenue_ancien, source)) = seance else {
            // Appel historique sans horaire : l'ancien circuit ne la compte pas à la séance non plus.
            return Ok(());
        };
        let (deja,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_asd_absence_seance WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(seance_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja {
            return Ok(());
        }
        let jour = appel.jour;

        // 1. Épisode contigu : même absence du jour, ouvert ou à qualifier, sans présence entre les deux.
        let candidats: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_asd_absence
              WHERE etablissement_id = $1 AND eleve_id = $2 AND absence_legacy_id = $3
                AND etat IN ('ouvert', 'a_qualifier')
              ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(absence_legacy_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut episode: Option<Uuid> = None;
        for (cand,) in candidats {
            let bornes: (Option<NaiveTime>, Option<NaiveTime>) = sqlx::query_as(
                "SELECT min(heure_debut), max(heure_fin) FROM ent_asd_absence_seance
                  WHERE etablissement_id = $1 AND absence_id = $2 AND jour = $3 AND kind = 'seance'
                    AND retiree_le IS NULL",
            )
            .bind(etab)
            .bind(cand)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let (Some(b_debut), Some(b_fin)) = bornes else { continue };
            // Intervalle libre entre l'épisode et la nouvelle séance (vide si elles se touchent ou se recouvrent).
            let trou = if debut >= b_fin {
                Some((b_fin, debut))
            } else if fin <= b_debut {
                Some((fin, b_debut))
            } else {
                None
            };
            let separe = match trou {
                Some((t_debut, t_fin)) if t_fin > t_debut => {
                    presence_dans_intervalle(tx, etab, eleve_id, jour, appel.id, t_debut, t_fin).await?
                }
                _ => false,
            };
            if !separe {
                episode = Some(cand);
                break;
            }
        }

        // 2. Déclarations et autorisations qui couvrent la séance.
        let couvrantes: Vec<(Uuid, String, Option<Uuid>, Option<String>, Option<Uuid>)> = sqlx::query_as(
            "SELECT s.id, s.nature, s.motif_id, m.nature_code, s.cree_par
               FROM ent_asd_situation s
               LEFT JOIN ent_asd_motif m ON m.etablissement_id = s.etablissement_id AND m.id = s.motif_id
              WHERE s.etablissement_id = $1 AND s.eleve_id = $2 AND s.nature IN ('declaration', 'autorisation')
                AND s.etat = 'active' AND s.debut_jour <= $3 AND s.fin_jour >= $3
                AND (CASE WHEN s.debut_jour < $3 THEN TIME '00:00' ELSE COALESCE(s.debut_heure, TIME '00:00') END) < $5
                AND COALESCE(CASE WHEN s.fin_jour > $3 THEN NULL ELSE s.fin_heure END, TIME '23:59:59') > $4
              ORDER BY s.cree_le, s.id",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(jour)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let prevenue = prevenue_ancien || !couvrantes.is_empty();
        let (motif_id, nature_code): (Option<Uuid>, Option<String>) = couvrantes
            .iter()
            .find(|c| c.2.is_some())
            .map(|c| (c.2, c.3.clone()))
            .unwrap_or((None, None));
        let autorisation = couvrantes.iter().find(|c| c.1 == "autorisation");

        // 3. Création de l'épisode si aucun n'est contigu.
        let episode_id = match episode {
            Some(e) => e,
            None => {
                let id = Uuid::new_v4();
                let src = match source.as_str() {
                    "appel" => "appel",
                    "accueil" => "accueil",
                    "famille" => "famille",
                    "vie_scolaire" => "vie_scolaire",
                    _ => "appel",
                };
                sqlx::query(
                    "INSERT INTO ent_asd_absence
                        (id, etablissement_id, eleve_id, jour_debut, jour_fin, etat, qualification, nature_code,
                         motif_id, prevenue, source, absence_legacy_id)
                     VALUES ($1,$2,$3,$4,$4,
                             CASE WHEN $4 < ent_aujourdhui_etab($2, now()) THEN 'a_qualifier' ELSE 'ouvert' END,
                             'en_attente',$5,$6,$7,$8,$9)",
                )
                .bind(id)
                .bind(etab)
                .bind(eleve_id)
                .bind(jour)
                .bind(&nature_code)
                .bind(motif_id)
                .bind(prevenue)
                .bind(src)
                .bind(absence_legacy_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                let (etat,): (String,) =
                    sqlx::query_as("SELECT etat FROM ent_asd_absence WHERE etablissement_id = $1 AND id = $2")
                        .bind(etab)
                        .bind(id)
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(db)?;
                historiser(tx, etab, id, &etat, "en_attente", "appel", acteur, None, None).await?;
                id
            }
        };

        // 4. La séance, avec la règle en vigueur au constat (R-AS15).
        sqlx::query(
            "INSERT INTO ent_asd_absence_seance
                (id, etablissement_id, absence_id, eleve_id, jour, kind, seance_ref, seance_source,
                 heure_debut, heure_fin, appel_id, regle_valide_du, prevenue, source)
             VALUES ($1,$2,$3,$4,$5,'seance',$6,$7,$8,$9,$10,
                     (SELECT r.valide_du FROM ent_asd_regle r
                       WHERE r.etablissement_id = $2 AND r.valide_du <= $5 ORDER BY r.valide_du DESC LIMIT 1),
                     $11,'appel')
             ON CONFLICT DO NOTHING",
        )
        .bind(seance_id)
        .bind(etab)
        .bind(episode_id)
        .bind(eleve_id)
        .bind(jour)
        .bind(seance_ref)
        .bind(&seance_source)
        .bind(debut)
        .bind(fin)
        .bind(appel.id)
        .bind(prevenue)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (sit, ..) in &couvrantes {
            sqlx::query(
                "INSERT INTO ent_asd_absence_declaration (absence_id, situation_id, etablissement_id)
                 VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
            )
            .bind(episode_id)
            .bind(sit)
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        if prevenue {
            sqlx::query(
                "UPDATE ent_asd_absence
                    SET prevenue = true,
                        nature_code = COALESCE(nature_code, $3),
                        motif_id = COALESCE(motif_id, $4),
                        modifie_le = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(episode_id)
            .bind(&nature_code)
            .bind(motif_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }

        // 5. Autorisation accordée : l'absence est autorisée, donc excusée d'office (R131-5).
        if let Some((_, _, _, _, decideur)) = autorisation {
            let ep = Self::episode_ligne(tx, etab, episode_id, false).await?;
            if matches!(ep.etat.as_str(), "ouvert" | "a_qualifier") {
                self.appliquer_qualification_tx(
                    tx,
                    etab,
                    &ep,
                    "excuse_valable",
                    None,
                    None,
                    Some("absence_excusee"),
                    None,
                    Some("autorisation_accordee"),
                    "qualification",
                    decideur.or(acteur),
                    publies,
                )
                .await?;
            }
        }
        Ok(())
    }

    /// Défait les séances d'un élève pour un appel (correction en « présent », séance annulée) : la
    /// ligne reste, marquée retirée avec sa cause ; un épisode sans séance restante est **retiré**, jamais
    /// supprimé (R-AS8, R-AS12).
    pub(crate) async fn episode_retirer_appel_tx(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        appel_id: Uuid,
        eleve_id: Uuid,
        cause: &'static str,
        acteur: Option<Uuid>,
    ) -> Result<(), EntDbError> {
        let touches: Vec<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_asd_absence_seance
                SET retiree_le = now(), retiree_cause = $4, retiree_par = $5
              WHERE etablissement_id = $1 AND appel_id = $2 AND eleve_id = $3 AND kind = 'seance'
                AND retiree_le IS NULL
              RETURNING absence_id",
        )
        .bind(etab)
        .bind(appel_id)
        .bind(eleve_id)
        .bind(cause)
        .bind(acteur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut ids: Vec<Uuid> = touches.into_iter().map(|(a,)| a).collect();
        ids.sort();
        ids.dedup();
        for id in ids {
            let (reste,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_asd_absence_seance
                                 WHERE etablissement_id = $1 AND absence_id = $2 AND retiree_le IS NULL)",
            )
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if reste {
                continue;
            }
            let r: Option<(String,)> = sqlx::query_as(
                "UPDATE ent_asd_absence SET etat = 'retire', version = version + 1, modifie_le = now()
                  WHERE etablissement_id = $1 AND id = $2 AND etat <> 'retire' RETURNING qualification",
            )
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some((qualification,)) = r {
                let origine = if cause == "neutralisation" { "neutralisation" } else { "correction" };
                historiser(tx, etab, id, "retire", &qualification, origine, acteur, Some(cause), None).await?;
            }
        }
        Ok(())
    }

    /// Réécrit l'ancien statut (`ent_absence.statut`, encore lu par la famille et le décompte) à partir
    /// des épisodes de la même absence ancienne : en attente tant qu'un épisode attend ; sinon
    /// « justifiée » si tous les épisodes qualifiés sont valables, « refusée » sinon. Le déclencheur de
    /// projection inverse est neutralisé pour cette écriture.
    async fn projeter_vers_ancien_tx(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        legacy_id: Uuid,
        acteur: Option<Uuid>,
        publies: &mut Vec<MetierEvent>,
    ) -> Result<(), EntDbError> {
        let (en_attente, valides, qualifies): (i64, i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE etat IN ('ouvert', 'a_qualifier')),
                    count(*) FILTER (WHERE etat = 'qualifie' AND qualification IN ('reputee_legitime', 'excuse_valable')),
                    count(*) FILTER (WHERE etat = 'qualifie')
               FROM ent_asd_absence WHERE etablissement_id = $1 AND absence_legacy_id = $2",
        )
        .bind(etab)
        .bind(legacy_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if en_attente > 0 || qualifies == 0 {
            return Ok(());
        }
        let statut = if valides == qualifies { "justifiee" } else { "refusee" };
        sqlx::query("SELECT set_config('ent.asd_projection', 'episode', true)")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let r: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_absence
                SET statut = $3, decision_par_user_id = $4, decision_at = now(),
                    justifie_at = CASE WHEN $3 = 'justifiee' THEN COALESCE(justifie_at, now()) ELSE justifie_at END
              WHERE etablissement_id = $1 AND id = $2 AND statut <> $3 AND statut <> 'annulee_present'
              RETURNING eleve_id",
        )
        .bind(etab)
        .bind(legacy_id)
        .bind(statut)
        .bind(acteur)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query("SELECT set_config('ent.asd_projection', '', true)")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if let Some((eleve_id,)) = r {
            // L'ancien événement de décision reste émis : la famille est informée comme avant (sans la note).
            let type_name = if statut == "justifiee" {
                ent_events::types::ELEVE_ABSENCE_JUSTIFIED
            } else {
                ent_events::types::ELEVE_ABSENCE_REFUSED
            };
            publier_asd(tx, etab, type_name, legacy_id, eleve_id, json!({ "statut": statut }), acteur, publies).await?;
        }
        Ok(())
    }

    async fn episode_ligne(
        c: &mut PgConnection,
        etab: Uuid,
        id: Uuid,
        verrou: bool,
    ) -> Result<EpisodeLigne, EntDbError> {
        sqlx::query_as::<_, EpisodeLigne>(&format!(
            "SELECT {EPISODE_COLONNES} FROM ent_asd_absence WHERE etablissement_id = $1 AND id = $2 {}",
            if verrou { "FOR UPDATE" } else { "" }
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(c)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    /// Pose une qualification sur un épisode (qualification ou requalification), l'historise, publie
    /// l'événement et réécrit l'ancien statut. La version lue doit être la version courante.
    #[allow(clippy::too_many_arguments)]
    async fn appliquer_qualification_tx(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        ep: &EpisodeLigne,
        qualification: &str,
        nature_code: Option<&str>,
        motif_id: Option<Uuid>,
        libelle: Option<&str>,
        note: Option<&str>,
        motif_code: Option<&str>,
        origine: &str,
        acteur: Option<Uuid>,
        publies: &mut Vec<MetierEvent>,
    ) -> Result<(), EntDbError> {
        let nature_effective = nature_code.map(str::to_string).or_else(|| ep.nature_code.clone());
        if qualification == "reputee_legitime" {
            let legitime: Option<(bool,)> =
                sqlx::query_as("SELECT legitime_l131_8 FROM ref_asd_nature_motif WHERE code = $1")
                    .bind(nature_effective.as_deref().unwrap_or(""))
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db)?;
            if !matches!(legitime, Some((true,))) {
                return Err(invalide(
                    "seules les cinq natures de L131-8 peuvent être réputées légitimes : choisir une autre qualification ou une nature légale",
                ));
            }
        }
        let libelle = libelle.unwrap_or_else(|| libelle_defaut(qualification));
        let r = sqlx::query(
            "UPDATE ent_asd_absence
                SET etat = 'qualifie', qualification = $3, nature_code = COALESCE($4, nature_code),
                    motif_id = COALESCE($5, motif_id), libelle_famille_code = $6,
                    qualifie_par = $7, qualifie_le = now(), version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND version = $8 AND etat <> 'retire'",
        )
        .bind(etab)
        .bind(ep.id)
        .bind(qualification)
        .bind(nature_code)
        .bind(motif_id)
        .bind(libelle)
        .bind(acteur)
        .bind(ep.version)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(conflit("l'épisode a été modifié entre-temps : relire avant de qualifier"));
        }
        historiser(tx, etab, ep.id, "qualifie", qualification, origine, acteur, motif_code, note).await?;
        let type_name = if origine == "requalification" { types_as3::ABSENCE_REQUALIFIEE } else { types_as3::ABSENCE_QUALIFIEE };
        let sens_change = origine == "requalification"
            && (matches!(ep.qualification.as_str(), "non_valable" | "sans_motif")
                != matches!(qualification, "non_valable" | "sans_motif"));
        publier_asd(
            tx,
            etab,
            type_name,
            ep.id,
            ep.eleve_id,
            json!({
                "qualification": qualification,
                "nature_code": nature_effective,
                "libelle_famille_code": libelle,
                "sens_change": sens_change,
            }),
            acteur,
            publies,
        )
        .await?;
        if let Some(legacy) = ep.absence_legacy_id {
            self.projeter_vers_ancien_tx(tx, etab, legacy, acteur, publies).await?;
        }
        Ok(())
    }
}

/// Un adulte a-t-il constaté l'élève (présent, retard, localisé, dispensé) pendant l'intervalle libre ?
/// Cet intervalle sépare deux séances absentes : s'il en contient une, elles ne sont pas contiguës.
async fn presence_dans_intervalle(
    tx: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    sauf_appel: Uuid,
    debut: NaiveTime,
    fin: NaiveTime,
) -> Result<bool, EntDbError> {
    let (b,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM ent_appel_ligne l
              JOIN ent_appel a ON a.etablissement_id = l.etablissement_id AND a.id = l.appel_id
              LEFT JOIN ent_edt_creneau c ON c.etablissement_id = a.etablissement_id AND c.id = a.creneau_id
             WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND a.jour = $3 AND a.id <> $4
               AND a.etat = 'valide'
               AND l.etat IN ('present', 'retard', 'localise', 'present_dispense')
               AND COALESCE(a.heure_debut, c.heure_debut) < $6
               AND COALESCE(a.heure_fin, c.heure_fin) > $5)",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(jour)
    .bind(sauf_appel)
    .bind(debut)
    .bind(fin)
    .fetch_one(tx)
    .await
    .map_err(db)?;
    Ok(b)
}

// ── Lecture : vie scolaire, professeur, famille ─────────────────────────────────────────────────

fn valider_qualification(q: &str) -> Result<(), EntDbError> {
    if QUALIFICATIONS.contains(&q) {
        Ok(())
    } else {
        Err(invalide("qualification inconnue"))
    }
}

fn valider_libelle(qualification: &str, l: &Option<String>) -> Result<(), EntDbError> {
    match l {
        None => Ok(()),
        Some(l) if LIBELLES_FAMILLE.contains(&l.as_str()) && libelle_coherent(qualification, l) => Ok(()),
        Some(_) => Err(invalide("libellé famille hors liste ou incohérent avec la qualification")),
    }
}

impl PgEnt {
    /// Le motif et la nature d'un épisode sont lus de la vie scolaire (`absences.lire_motif`) ; qui n'a
    /// pas ce droit reçoit « introuvable » (même réponse que pour un épisode inexistant, R-AS35).
    async fn exiger_lire_motif(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        if self.a_action_etablissement(etab, acteur, role, Action::AbsencesLireMotif).await? {
            Ok(())
        } else {
            Err(introuvable())
        }
    }

    async fn peut_qualifier(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<bool, EntDbError> {
        self.a_action_etablissement(etab, acteur, role, Action::AbsencesQualifier).await
    }

    async fn seances_des_episodes(
        c: &mut PgConnection,
        etab: Uuid,
        ids: &[Uuid],
    ) -> Result<Vec<(Uuid, SeanceEpisode)>, EntDbError> {
        #[derive(sqlx::FromRow)]
        struct L {
            absence_id: Uuid,
            id: Uuid,
            jour: NaiveDate,
            kind: String,
            periode: Option<String>,
            heure_debut: Option<NaiveTime>,
            heure_fin: Option<NaiveTime>,
            prevenue: bool,
            retiree: bool,
        }
        let rows: Vec<L> = sqlx::query_as(
            "SELECT absence_id, id, jour, kind, periode, heure_debut, heure_fin, prevenue,
                    (retiree_le IS NOT NULL) AS retiree
               FROM ent_asd_absence_seance
              WHERE etablissement_id = $1 AND absence_id = ANY($2)
              ORDER BY jour, heure_debut NULLS FIRST, periode NULLS LAST, id",
        )
        .bind(etab)
        .bind(ids)
        .fetch_all(c)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|l| {
                (
                    l.absence_id,
                    SeanceEpisode {
                        id: l.id,
                        jour: l.jour,
                        kind: l.kind,
                        periode: l.periode,
                        heure_debut: l.heure_debut,
                        heure_fin: l.heure_fin,
                        prevenue: l.prevenue,
                        retiree: l.retiree,
                    },
                )
            })
            .collect())
    }

    async fn composer_episodes(
        &self,
        c: &mut PgConnection,
        etab: Uuid,
        lignes: Vec<EpisodeLigne>,
        avec_textes: bool,
    ) -> Result<Vec<EpisodeAsd>, EntDbError> {
        let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
        let seances = Self::seances_des_episodes(c, etab, &ids).await?;
        let justifs: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT absence_id, count(*) FROM ent_asd_justificatif
              WHERE etablissement_id = $1 AND absence_id = ANY($2) AND presente <> 'retire' GROUP BY absence_id",
        )
        .bind(etab)
        .bind(&ids)
        .fetch_all(&mut *c)
        .await
        .map_err(db)?;
        let props: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT absence_id, count(*) FROM ent_asd_proposition
              WHERE etablissement_id = $1 AND absence_id = ANY($2) AND etat = 'en_attente' GROUP BY absence_id",
        )
        .bind(etab)
        .bind(&ids)
        .fetch_all(&mut *c)
        .await
        .map_err(db)?;
        let mut textes: Vec<(Uuid, String)> = Vec::new();
        let mut notes: Vec<(Uuid, String)> = Vec::new();
        if avec_textes {
            textes = sqlx::query_as(
                "SELECT d.absence_id, t.texte
                   FROM ent_asd_absence_declaration d
                   JOIN ent_asd_situation_texte t ON t.etablissement_id = d.etablissement_id AND t.situation_id = d.situation_id
                  WHERE d.etablissement_id = $1 AND d.absence_id = ANY($2) ORDER BY t.cree_le",
            )
            .bind(etab)
            .bind(&ids)
            .fetch_all(&mut *c)
            .await
            .map_err(db)?;
            // Justification libre de l'ancien circuit (motif_justif) : même confidentialité que le texte.
            let anciens: Vec<(Uuid, String)> = sqlx::query_as(
                "SELECT e.id, a.motif_justif
                   FROM ent_asd_absence e JOIN ent_absence a ON a.etablissement_id = e.etablissement_id AND a.id = e.absence_legacy_id
                  WHERE e.etablissement_id = $1 AND e.id = ANY($2) AND a.motif_justif IS NOT NULL AND btrim(a.motif_justif) <> ''",
            )
            .bind(etab)
            .bind(&ids)
            .fetch_all(&mut *c)
            .await
            .map_err(db)?;
            textes.extend(anciens);
            notes = sqlx::query_as(
                "SELECT DISTINCT ON (absence_id) absence_id, note_interne FROM ent_asd_absence_etat
                  WHERE etablissement_id = $1 AND absence_id = ANY($2) AND note_interne IS NOT NULL
                  ORDER BY absence_id, cree_le DESC",
            )
            .bind(etab)
            .bind(&ids)
            .fetch_all(&mut *c)
            .await
            .map_err(db)?;
        }
        Ok(lignes
            .into_iter()
            .map(|l| EpisodeAsd {
                seances: seances.iter().filter(|(a, _)| *a == l.id).map(|(_, s)| s.clone()).collect(),
                justificatifs: justifs.iter().find(|(a, _)| *a == l.id).map(|(_, n)| *n).unwrap_or(0),
                propositions_en_attente: props.iter().find(|(a, _)| *a == l.id).map(|(_, n)| *n).unwrap_or(0),
                textes: textes.iter().filter(|(a, _)| *a == l.id).map(|(_, t)| t.clone()).collect(),
                note_interne: notes.iter().find(|(a, _)| *a == l.id).map(|(_, n)| n.clone()),
                id: l.id,
                eleve_id: l.eleve_id,
                eleve_label: None,
                jour_debut: l.jour_debut,
                jour_fin: l.jour_fin,
                etat: l.etat,
                qualification: l.qualification,
                nature_code: l.nature_code,
                motif_id: l.motif_id,
                prevenue: l.prevenue,
                source: l.source,
                libelle_famille_code: l.libelle_famille_code,
                version: l.version,
            })
            .collect())
    }

    /// Passe à « à qualifier » les épisodes des jours passés (le jour métier de l'établissement).
    async fn consolider_episodes_tx(&self, tx: &mut PgConnection, etab: Uuid) -> Result<(), EntDbError> {
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_asd_absence SET etat = 'a_qualifier', version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND etat = 'ouvert' AND jour_fin < ent_aujourdhui_etab($1, now())
              RETURNING id",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (id,) in ids {
            historiser(tx, etab, id, "a_qualifier", "en_attente", "consolidation", None, None, None).await?;
        }
        Ok(())
    }

    /// File des épisodes (AS-120) : `absences.lire_motif` ; le texte libre et la note interne ne sont
    /// remplis que pour qui peut qualifier (l'AED voit la nature, pas le texte, R-AS19).
    pub async fn episodes_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        filtre: FiltreEpisodes,
    ) -> Result<Vec<EpisodeAsd>, EntDbError> {
        self.exiger_lire_motif(etab, acteur, role).await?;
        let avec_textes = self.peut_qualifier(etab, acteur, role).await?;
        let etat = filtre.etat.as_deref().unwrap_or("a_qualifier");
        if !matches!(etat, "a_qualifier" | "qualifie" | "retire" | "tous") {
            return Err(invalide("état de filtre inconnu"));
        }
        let limite = filtre.limite.unwrap_or(100).clamp(1, 500);
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        self.consolider_episodes_tx(&mut tx, etab).await?;
        let lignes: Vec<EpisodeLigne> = sqlx::query_as(&format!(
            "SELECT {EPISODE_COLONNES} FROM ent_asd_absence e
              WHERE etablissement_id = $1
                AND ($2::uuid IS NULL OR eleve_id = $2)
                AND ($3::date IS NULL OR jour_fin >= $3)
                AND ($4::date IS NULL OR jour_debut <= $4)
                AND (CASE $5::text WHEN 'a_qualifier' THEN etat IN ('ouvert', 'a_qualifier')
                             WHEN 'tous' THEN true ELSE etat = $5::text END)
                AND ($6::text IS NULL OR nature_code = $6)
                AND ($7::text IS NULL OR source = $7)
                AND ($8::bool IS NULL OR $8 = EXISTS (
                      SELECT 1 FROM ent_asd_justificatif j
                       WHERE j.etablissement_id = e.etablissement_id AND j.absence_id = e.id AND j.presente <> 'retire'))
              ORDER BY jour_debut, cree_le, id LIMIT $9"
        ))
        .bind(etab)
        .bind(filtre.eleve_id)
        .bind(filtre.depuis)
        .bind(filtre.jusqu_au)
        .bind(etat)
        .bind(filtre.nature_code)
        .bind(filtre.source)
        .bind(filtre.avec_justificatif)
        .bind(limite)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let out = self.composer_episodes(&mut tx, etab, lignes, avec_textes).await?;
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Détail d'un épisode, historique et propositions comprises (`absences.lire_motif`).
    pub async fn episode_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<DetailEpisode, EntDbError> {
        self.exiger_lire_motif(etab, acteur, role).await?;
        let avec_textes = self.peut_qualifier(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ligne = Self::episode_ligne(&mut tx, etab, absence_id, false).await?;
        let mut v = self.composer_episodes(&mut tx, etab, vec![ligne], avec_textes).await?;
        let historique: Vec<EtatEpisode> = sqlx::query_as(
            "SELECT etat, qualification, origine, auteur_id, motif_code, cree_le FROM ent_asd_absence_etat
              WHERE etablissement_id = $1 AND absence_id = $2 ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let propositions: Vec<PropositionAsd> = sqlx::query_as(
            "SELECT id, absence_id, qualification, note, proposee_par, etat, cree_le FROM ent_asd_proposition
              WHERE etablissement_id = $1 AND absence_id = $2 ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(DetailEpisode { episode: v.remove(0), historique, propositions })
    }

    /// Vue du professeur (R-AS21) : les épisodes qui touchent **ses** séances, pour les élèves qu'il
    /// enseigne, avec le seul état de régularisation. Aucun motif, nature, pièce ni note.
    pub async fn episodes_de_mes_seances_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        depuis: NaiveDate,
    ) -> Result<Vec<EpisodeVueProfesseur>, EntDbError> {
        #[derive(sqlx::FromRow)]
        struct L {
            id: Uuid,
            eleve_id: Uuid,
            jour_debut: NaiveDate,
            jour_fin: NaiveDate,
            etat: String,
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let rows: Vec<L> = sqlx::query_as(
            "SELECT DISTINCT e.id, e.eleve_id, e.jour_debut, e.jour_fin, e.etat
               FROM ent_asd_absence e
               JOIN ent_asd_absence_seance s ON s.etablissement_id = e.etablissement_id AND s.absence_id = e.id
               JOIN ent_appel a ON a.etablissement_id = s.etablissement_id AND a.id = s.appel_id
              WHERE e.etablissement_id = $1 AND e.etat <> 'retire' AND s.retiree_le IS NULL
                AND e.jour_fin >= $3 AND (a.auteur_user_id = $2 OR a.delegue_a = $2)
              ORDER BY e.jour_debut, e.id",
        )
        .bind(etab)
        .bind(acteur)
        .bind(depuis)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut out = Vec::new();
        for r in rows {
            // Chaque élève reste soumis à la portée de lecture du professeur (D-022).
            if self.autoriser_eleve(etab, acteur, role, Action::AbsencesLire, r.eleve_id).await? {
                out.push(EpisodeVueProfesseur {
                    id: r.id,
                    eleve_id: r.eleve_id,
                    jour_debut: r.jour_debut,
                    jour_fin: r.jour_fin,
                    regularisation: if r.etat == "qualifie" { "regularisee" } else { "a_regulariser" }.into(),
                });
            }
        }
        Ok(out)
    }

    /// Qui, parmi la famille ou l'élève, peut agir sur cet élève : `famille` (responsable avec
    /// `peut_justifier_absence`), `eleve_majeur`, ou personne.
    async fn qualite_famille(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
        pour_lire: bool,
    ) -> Result<Option<&'static str>, EntDbError> {
        match Role::parse(role) {
            Some(Role::Parent) => {
                let Some(lien) = self.lien_parent_eleve(etab, acteur, eleve_id).await? else { return Ok(None) };
                let ok = if pour_lire { lien.peut_voir_vs || lien.peut_justifier_absence } else { lien.peut_justifier_absence };
                Ok(ok.then_some("famille"))
            }
            Some(Role::Eleve) => {
                if self.eleve_id_for_compte(etab, acteur).await? != Some(eleve_id) {
                    return Ok(None);
                }
                if pour_lire {
                    return Ok(Some("eleve_lecteur"));
                }
                let jour = self.aujourdhui_etablissement(etab).await?;
                Ok(self.eleve_est_majeur(etab, eleve_id, jour).await?.then_some("eleve_majeur"))
            }
            _ => Ok(None),
        }
    }

    /// Vue de la famille ou de l'élève (R-AS22) : libellé à liste fermée, jamais la note interne ni le
    /// texte d'un autre responsable.
    pub async fn episodes_famille_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<EpisodeVueFamille>, EntDbError> {
        if self.qualite_famille(etab, acteur, role, eleve_id, true).await?.is_none() {
            return Err(introuvable());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let rows: Vec<(Uuid, NaiveDate, NaiveDate, bool, String, Option<String>)> = sqlx::query_as(
            "SELECT id, jour_debut, jour_fin, prevenue, etat, libelle_famille_code FROM ent_asd_absence
              WHERE etablissement_id = $1 AND eleve_id = $2 AND etat <> 'retire'
              ORDER BY jour_debut DESC, id LIMIT 200",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, jour_debut, jour_fin, prevenue, etat, code)| EpisodeVueFamille {
                id,
                jour_debut,
                jour_fin,
                prevenue,
                a_justifier: matches!(etat.as_str(), "ouvert" | "a_qualifier"),
                libelle: libelle_famille_texte(code.as_deref().unwrap_or("")).to_string(),
                libelle_code: code,
            })
            .collect())
    }

    // ── Qualification ───────────────────────────────────────────────────────────────────────

    /// Qualifie un épisode en attente (AS-121) : `absences.qualifier`. `reputee_legitime` exige l'une des
    /// cinq natures de L131-8. Un épisode déjà qualifié se requalifie (motif obligatoire).
    pub async fn qualifier_episode_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        absence_id: Uuid,
        q: QualifierEpisode,
    ) -> Result<EpisodeAsd, EntDbError> {
        self.exiger_lire_motif(etab, acteur, role).await?;
        if !self.peut_qualifier(etab, acteur, role).await? {
            return Err(EntDbError::Forbidden);
        }
        valider_qualification(&q.qualification)?;
        valider_libelle(&q.qualification, &q.libelle_famille_code)?;
        let note = court(q.note_interne, MAX_NOTE)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ep = Self::episode_ligne(&mut tx, etab, absence_id, true).await?;
        if ep.version != q.version {
            return Err(conflit("l'épisode a été modifié entre-temps : relire avant de qualifier"));
        }
        if !matches!(ep.etat.as_str(), "ouvert" | "a_qualifier") {
            return Err(conflit("épisode déjà qualifié ou retiré : utiliser la requalification"));
        }
        self.controler_nature_motif(&mut tx, etab, q.nature_code.as_deref(), q.motif_id).await?;
        let mut publies = Vec::new();
        self.appliquer_qualification_tx(
            &mut tx,
            etab,
            &ep,
            &q.qualification,
            q.nature_code.as_deref(),
            q.motif_id,
            q.libelle_famille_code.as_deref(),
            note.as_deref(),
            None,
            "qualification",
            Some(acteur),
            &mut publies,
        )
        .await?;
        if let Some(p) = q.proposition_id {
            sqlx::query(
                "UPDATE ent_asd_proposition SET etat = CASE WHEN id = $3 THEN 'acceptee' ELSE 'ecartee' END,
                        decidee_par = $4, decidee_le = now()
                  WHERE etablissement_id = $1 AND absence_id = $2 AND etat = 'en_attente'",
            )
            .bind(etab)
            .bind(absence_id)
            .bind(p)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let ligne = Self::episode_ligne(&mut tx, etab, absence_id, false).await?;
        let mut v = self.composer_episodes(&mut tx, etab, vec![ligne], true).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&publies);
        Ok(v.remove(0))
    }

    async fn controler_nature_motif(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        nature: Option<&str>,
        motif: Option<Uuid>,
    ) -> Result<(), EntDbError> {
        if let Some(n) = nature {
            let (ok,): (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM ref_asd_nature_motif WHERE code = $1)")
                .bind(n)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            if !ok {
                return Err(invalide("nature de motif inconnue"));
            }
        }
        if let Some(m) = motif {
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_asd_motif WHERE etablissement_id = $1 AND id = $2 AND actif)",
            )
            .bind(etab)
            .bind(m)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(invalide("motif inconnu"));
            }
        }
        Ok(())
    }

    /// Requalifie un épisode déjà qualifié (AS-123) : motif codé obligatoire, historisé ; l'événement
    /// signale si le sens change pour la famille (« non valable » ↔ « valable »).
    pub async fn requalifier_episode_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        absence_id: Uuid,
        q: RequalifierEpisode,
    ) -> Result<EpisodeAsd, EntDbError> {
        self.exiger_lire_motif(etab, acteur, role).await?;
        if !self.peut_qualifier(etab, acteur, role).await? {
            return Err(EntDbError::Forbidden);
        }
        valider_qualification(&q.qualification)?;
        valider_libelle(&q.qualification, &q.libelle_famille_code)?;
        if !MOTIFS_REQUALIFICATION.contains(&q.motif_code.as_str()) {
            return Err(invalide("motif de requalification obligatoire et codé"));
        }
        let note = court(q.note_interne, MAX_NOTE)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ep = Self::episode_ligne(&mut tx, etab, absence_id, true).await?;
        if ep.etat != "qualifie" {
            return Err(conflit("seul un épisode qualifié se requalifie"));
        }
        if ep.version != q.version {
            return Err(conflit("l'épisode a été modifié entre-temps : relire avant de requalifier"));
        }
        if ep.qualification == q.qualification {
            return Err(conflit("la qualification demandée est déjà celle de l'épisode"));
        }
        self.controler_nature_motif(&mut tx, etab, q.nature_code.as_deref(), None).await?;
        let mut publies = Vec::new();
        self.appliquer_qualification_tx(
            &mut tx,
            etab,
            &ep,
            &q.qualification,
            q.nature_code.as_deref(),
            None,
            q.libelle_famille_code.as_deref(),
            note.as_deref(),
            Some(&q.motif_code),
            "requalification",
            Some(acteur),
            &mut publies,
        )
        .await?;
        let ligne = Self::episode_ligne(&mut tx, etab, absence_id, false).await?;
        let mut v = self.composer_episodes(&mut tx, etab, vec![ligne], true).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&publies);
        Ok(v.remove(0))
    }

    /// Aperçu d'une qualification en masse (AS-122, AS-213) : épisodes retenus, écartés avec la raison, et
    /// une **empreinte** que la confirmation doit rendre à l'identique (aucune surprise entre l'aperçu et
    /// l'application). N'écrit rien.
    pub async fn apercu_masse_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        s: &SelectionMasse,
    ) -> Result<ApercuMasse, EntDbError> {
        self.exiger_lire_motif(etab, acteur, role).await?;
        if !self.peut_qualifier(etab, acteur, role).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = Self::apercu_masse_tx(&mut tx, etab, s).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    async fn apercu_masse_tx(tx: &mut PgConnection, etab: Uuid, s: &SelectionMasse) -> Result<ApercuMasse, EntDbError> {
        valider_qualification(&s.qualification)?;
        valider_libelle(&s.qualification, &s.libelle_famille_code)?;
        if s.absence_ids.is_empty() || s.absence_ids.len() > MAX_MASSE {
            return Err(invalide("sélection vide ou trop large (200 épisodes au plus)"));
        }
        let mut ids = s.absence_ids.clone();
        ids.sort();
        ids.dedup();
        let lignes: Vec<EpisodeLigne> = sqlx::query_as(&format!(
            "SELECT {EPISODE_COLONNES} FROM ent_asd_absence WHERE etablissement_id = $1 AND id = ANY($2) ORDER BY id"
        ))
        .bind(etab)
        .bind(&ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let legitimes: Vec<(String,)> = sqlx::query_as("SELECT code FROM ref_asd_nature_motif WHERE legitime_l131_8")
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let mut retenus = Vec::new();
        let mut ecartes = Vec::new();
        let mut empreinte = format!("{}|", s.qualification);
        for id in &ids {
            match lignes.iter().find(|l| l.id == *id) {
                None => ecartes.push((*id, "introuvable".to_string())),
                Some(l) if !matches!(l.etat.as_str(), "ouvert" | "a_qualifier") => {
                    ecartes.push((*id, "deja_qualifie_ou_retire".to_string()))
                }
                Some(l)
                    if s.qualification == "reputee_legitime"
                        && !l.nature_code.as_deref().map(|n| legitimes.iter().any(|(c,)| c == n)).unwrap_or(false) =>
                {
                    ecartes.push((*id, "nature_non_legitime".to_string()))
                }
                Some(l) => {
                    empreinte.push_str(&format!("{}:{};", l.id, l.version));
                    retenus.push(l.id);
                }
            }
        }
        Ok(ApercuMasse { retenus, ecartes, empreinte: ent_object::sha256_hex(empreinte.as_bytes()) })
    }

    /// Qualifie en masse après confirmation : l'empreinte de l'aperçu doit correspondre à l'état actuel,
    /// sinon conflit. Un événement par épisode (AS-122). Atomique : tout est qualifié ou rien.
    pub async fn qualifier_masse_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        s: SelectionMasse,
    ) -> Result<BilanMasse, EntDbError> {
        self.exiger_lire_motif(etab, acteur, role).await?;
        if !self.peut_qualifier(etab, acteur, role).await? {
            return Err(EntDbError::Forbidden);
        }
        let Some(attendue) = s.empreinte.clone() else {
            return Err(invalide("l'aperçu doit être demandé et confirmé (empreinte manquante)"));
        };
        let note = court(s.note_interne.clone(), MAX_NOTE)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Verrouille les épisodes pour que l'état comparé soit celui qui sera écrit.
        let mut ids = s.absence_ids.clone();
        ids.sort();
        ids.dedup();
        sqlx::query("SELECT id FROM ent_asd_absence WHERE etablissement_id = $1 AND id = ANY($2) ORDER BY id FOR UPDATE")
            .bind(etab)
            .bind(&ids)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        let apercu = Self::apercu_masse_tx(&mut tx, etab, &s).await?;
        if apercu.empreinte != attendue {
            return Err(conflit("l'aperçu n'est plus à jour : relancer l'aperçu avant de confirmer"));
        }
        let mut publies = Vec::new();
        let mut qualifies = Vec::new();
        for id in &apercu.retenus {
            let ep = Self::episode_ligne(&mut tx, etab, *id, false).await?;
            self.appliquer_qualification_tx(
                &mut tx,
                etab,
                &ep,
                &s.qualification,
                None,
                None,
                s.libelle_famille_code.as_deref(),
                note.as_deref(),
                None,
                "masse",
                Some(acteur),
                &mut publies,
            )
            .await?;
            qualifies.push(*id);
        }
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&publies);
        Ok(BilanMasse { qualifies })
    }

    /// Propose une qualification (AS-124) : `absences.proposer_qualification` (l'AED) ; la vie scolaire
    /// décide. La proposition ne change ni l'état ni l'ancien statut.
    pub async fn proposer_qualification_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        absence_id: Uuid,
        p: ProposerQualification,
    ) -> Result<PropositionAsd, EntDbError> {
        self.exiger_lire_motif(etab, acteur, role).await?;
        if !self.a_action_etablissement(etab, acteur, role, Action::AbsencesProposerQualification).await? {
            return Err(EntDbError::Forbidden);
        }
        valider_qualification(&p.qualification)?;
        let note = court(p.note, MAX_NOTE)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ep = Self::episode_ligne(&mut tx, etab, absence_id, true).await?;
        if !matches!(ep.etat.as_str(), "ouvert" | "a_qualifier") {
            return Err(conflit("épisode déjà qualifié ou retiré"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_asd_proposition (id, etablissement_id, absence_id, qualification, note, proposee_par)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id)
        .bind(etab)
        .bind(absence_id)
        .bind(&p.qualification)
        .bind(&note)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let mut publies = Vec::new();
        publier_asd(
            &mut tx,
            etab,
            types_as3::QUALIFICATION_PROPOSEE,
            absence_id,
            ep.eleve_id,
            json!({ "qualification": p.qualification, "proposition_id": id }),
            Some(acteur),
            &mut publies,
        )
        .await?;
        let r: PropositionAsd = sqlx::query_as(
            "SELECT id, absence_id, qualification, note, proposee_par, etat, cree_le FROM ent_asd_proposition
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&publies);
        Ok(r)
    }

    // ── Justificatifs ───────────────────────────────────────────────────────────────────────

    /// Résout qui agit pour un épisode : la famille (responsable habilité, élève majeur) ou le personnel
    /// qui peut déclarer pour cet élève. Tout autre cas : « introuvable » (R-AS35).
    async fn acteur_justificatif(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<(EpisodeLigne, &'static str), EntDbError> {
        let ep = {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            Self::episode_ligne(&mut tx, etab, absence_id, false).await?
        };
        if let Some(q) = self.qualite_famille(etab, acteur, role, ep.eleve_id, false).await? {
            return Ok((ep, q));
        }
        if self.autoriser_eleve(etab, acteur, role, Action::AbsencesDeclarer, ep.eleve_id).await? {
            let vs = self.a_action_etablissement(etab, acteur, role, Action::AbsencesLireMotif).await?;
            return Ok((ep, if vs { "vie_scolaire" } else { "accueil" }));
        }
        Err(introuvable())
    }

    /// Dépose un justificatif (AS-126, AS-128) : pièce (type contrôlé par signature binaire, clé
    /// générée par le serveur, aucun nom conservé), texte seul, ou « vu, non conservé ».
    pub async fn deposer_justificatif_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        absence_id: Uuid,
        depot: DepotJustificatif,
        fichier: Option<FichierDepot>,
    ) -> Result<JustificatifAsd, EntDbError> {
        let (ep, qualite) = self.acteur_justificatif(etab, acteur, role, absence_id).await?;
        let texte = court(depot.texte, 1000)?;
        if depot.vu_non_conserve && !matches!(qualite, "vie_scolaire" | "accueil") {
            return Err(EntDbError::Forbidden);
        }
        if fichier.is_none() && texte.is_none() && !depot.vu_non_conserve {
            return Err(invalide("un justificatif porte une pièce, un texte ou la mention « vu, non conservé »"));
        }
        if fichier.is_some() && depot.vu_non_conserve {
            return Err(invalide("« vu, non conservé » exclut le dépôt d'une pièce"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ep = Self::episode_ligne(&mut tx, etab, ep.id, true).await?;
        if ep.etat == "retire" {
            return Err(conflit("épisode retiré : aucun justificatif ne peut être déposé"));
        }
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_asd_justificatif WHERE etablissement_id = $1 AND absence_id = $2",
        )
        .bind(etab)
        .bind(ep.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if n >= MAX_JUSTIFICATIFS_EPISODE {
            return Err(conflit("nombre maximal de justificatifs atteint pour cet épisode"));
        }
        let medical = depot.medical.unwrap_or_else(|| nature_medicale(ep.nature_code.as_deref()));
        let id = Uuid::new_v4();
        let jour = jour_etab(&mut tx, etab).await?;
        let conserve = Some(ajouter_un_an(jour));
        let qualite_depot = qualite;
        let (presente, cle, sha, taille, mime) = match &fichier {
            Some(f) => {
                // Le type vient de la signature binaire et de la politique, jamais du client (NJ-38) ; la clé
                // d'objet est produite ici, jamais fournie par le client (NJ-29, NJ-39).
                let valide = self
                    .politique_fichiers()
                    .valider(UsageFichier::Document, &f.nom, Some(&f.mime_type), &f.contenu)
                    .map_err(EntDbError::Fichier)?;
                let cle = ObjectStore::key_for(etab, "assiduite", id, valide.extension);
                (
                    "depose",
                    Some(cle),
                    Some(ent_object::sha256_hex(&f.contenu)),
                    Some(f.contenu.len() as i64),
                    Some(valide.mime.to_string()),
                )
            }
            None if depot.vu_non_conserve => ("vu_non_conserve", None, None, None, None),
            None => ("texte", None, None, None, None),
        };
        sqlx::query(
            "INSERT INTO ent_asd_justificatif
                (id, etablissement_id, absence_id, presente, cle_objet, sha256, taille_octets, mime, texte, medical,
                 depose_par, depose_qualite, conserve_jusqu_au)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(id)
        .bind(etab)
        .bind(ep.id)
        .bind(presente)
        .bind(&cle)
        .bind(&sha)
        .bind(taille)
        .bind(&mime)
        .bind(&texte)
        .bind(medical)
        .bind(acteur)
        .bind(qualite_depot)
        .bind(if cle.is_some() { conserve } else { None })
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Ligne d'abord, objet ensuite, dans la même transaction : un échec du stockage annule la ligne.
        if let (Some(f), Some(cle)) = (&fichier, &cle) {
            let info = self.objet_put(cle, &f.contenu).await?;
            crate::objets::enregistrer_objet(&mut *tx, etab, &info, mime.as_deref()).await?;
        }
        historiser(&mut tx, etab, ep.id, &ep.etat, &ep.qualification, "justificatif", Some(acteur), Some(presente), None)
            .await?;
        let mut publies = Vec::new();
        publier_asd(
            &mut tx,
            etab,
            types_as3::JUSTIFICATIF_DEPOSE,
            ep.id,
            ep.eleve_id,
            json!({ "justificatif_id": id, "presente": presente, "medical": medical }),
            Some(acteur),
            &mut publies,
        )
        .await?;
        let r = Self::justificatif_ligne(&mut tx, etab, id, true).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&publies);
        Ok(r)
    }

    async fn justificatif_ligne(
        c: &mut PgConnection,
        etab: Uuid,
        id: Uuid,
        avec_texte: bool,
    ) -> Result<JustificatifAsd, EntDbError> {
        sqlx::query_as::<_, JustificatifAsd>(&format!(
            "SELECT id, absence_id, presente, medical, mime, taille_octets, depose_qualite, conserve_jusqu_au, cree_le,
                    detruit_le, {} AS texte
               FROM ent_asd_justificatif WHERE etablissement_id = $1 AND id = $2",
            if avec_texte { "texte" } else { "NULL::text" }
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(c)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    /// Justificatifs d'un épisode. Le **personnel** voit ceux qu'il peut lire : `absences.justificatif_lire`
    /// (non médicaux) ou `absences.justificatif_medical_lire` (tous) ; les pièces médicales sont
    /// **omises**, pas refusées, pour qui n'a pas le droit. La **famille** voit les dépôts de son enfant
    /// sans contenu. Le professeur et tout autre acteur reçoivent « introuvable ».
    pub async fn justificatifs_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<Vec<JustificatifAsd>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ep = Self::episode_ligne(&mut tx, etab, absence_id, false).await?;
        let lire = self.a_action_etablissement(etab, acteur, role, Action::AbsencesJustificatifLire).await?;
        let medical = self.a_action_etablissement(etab, acteur, role, Action::AbsencesJustificatifMedicalLire).await?;
        let famille = self.qualite_famille(etab, acteur, role, ep.eleve_id, true).await?.is_some();
        if !(lire || medical || famille) {
            return Err(introuvable());
        }
        let rows: Vec<(Uuid, bool)> = sqlx::query_as(
            "SELECT id, medical FROM ent_asd_justificatif
              WHERE etablissement_id = $1 AND absence_id = $2 AND ($3 OR NOT medical OR depose_par = $4)
              ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(absence_id)
        .bind(medical || famille)
        .bind(acteur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (id, est_medical) in rows {
            // Le texte n'est montré qu'au personnel habilité : jamais à la famille (elle l'a écrit), et pour
            // un justificatif médical seulement à qui lit les pièces médicales.
            let texte_visible = (lire || medical) && (!est_medical || medical);
            out.push(Self::justificatif_ligne(&mut tx, etab, id, texte_visible).await?);
        }
        Ok(out)
    }

    /// Lien pré-signé de 5 minutes au plus vers la pièce d'un justificatif. La lecture est **journalisée
    /// avant** l'émission du lien (E7). Une pièce médicale exige `absences.justificatif_medical_lire`.
    pub async fn lien_justificatif_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        absence_id: Uuid,
        justificatif_id: Uuid,
    ) -> Result<LienJustificatif, EntDbError> {
        if !self.presigner().is_enabled() || self.presigner().ttl_secs() > TTL_MAX_LIEN_JUSTIFICATIF_SECS {
            return Err(EntDbError::LiensIndisponibles);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (cle, mime, medical, presente, detruit): (Option<String>, Option<String>, bool, String, Option<chrono::DateTime<chrono::Utc>>) =
            sqlx::query_as(
                "SELECT cle_objet, mime, medical, presente, detruit_le FROM ent_asd_justificatif
                  WHERE etablissement_id = $1 AND absence_id = $2 AND id = $3",
            )
            .bind(etab)
            .bind(absence_id)
            .bind(justificatif_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        let besoin = if medical { Action::AbsencesJustificatifMedicalLire } else { Action::AbsencesJustificatifLire };
        if !self.a_action_etablissement(etab, acteur, role, besoin).await? {
            // Réponse identique pour « interdit » et « inexistant » (R-AS35).
            return Err(introuvable());
        }
        let (Some(cle), Some(mime)) = (cle, mime) else { return Err(EntDbError::FichierAbsent) };
        if presente != "depose" || detruit.is_some() {
            return Err(EntDbError::FichierAbsent);
        }
        sqlx::query(
            "INSERT INTO ent_asd_justificatif_lecture (id, etablissement_id, justificatif_id, lecteur_user_id, medical)
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(justificatif_id)
        .bind(acteur)
        .bind(medical)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ext = cle.rsplit_once('.').map(|(_, e)| e).unwrap_or("bin").to_string();
        let link = self
            .presigner()
            .issue(
                &LinkSpec {
                    key: cle.clone(),
                    user_id: acteur,
                    etablissement_id: etab,
                    disposition: Disposition::Attachment,
                    // Nom générique : le nom d'origine n'est jamais conservé (minimisation).
                    filename: format!("justificatif.{ext}"),
                    mime,
                },
                ent_object::presign::now_unix(),
            )
            .map_err(|e| match e {
                PresignError::Disabled => EntDbError::LiensIndisponibles,
                _ => EntDbError::FichierAbsent,
            })?;
        if !self.objet_existe(&cle).await? {
            return Err(EntDbError::FichierAbsent);
        }
        tx.commit().await.map_err(db)?;
        Ok(LienJustificatif { url: link.url, expires_at: link.expires_at })
    }

    /// Retire un justificatif déposé par erreur : son dépositaire, ou qui peut qualifier. La ligne reste
    /// (état `retire`) ; la pièce suit la conservation.
    pub async fn retirer_justificatif_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        absence_id: Uuid,
        justificatif_id: Uuid,
    ) -> Result<JustificatifAsd, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (depose_par,): (Uuid,) = sqlx::query_as(
            "SELECT depose_par FROM ent_asd_justificatif WHERE etablissement_id = $1 AND absence_id = $2 AND id = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(absence_id)
        .bind(justificatif_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if depose_par != acteur && !self.peut_qualifier(etab, acteur, role).await? {
            return Err(introuvable());
        }
        sqlx::query(
            "UPDATE ent_asd_justificatif SET presente = 'retire', retire_par = $3, retire_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND presente <> 'retire'",
        )
        .bind(etab)
        .bind(justificatif_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let r = Self::justificatif_ligne(&mut tx, etab, justificatif_id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Qui a lu quelles pièces d'un épisode (E7) : direction (`absences.justificatif_medical_lire`).
    pub async fn lectures_justificatifs_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        absence_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, bool, chrono::DateTime<chrono::Utc>)>, EntDbError> {
        if !self.a_action_etablissement(etab, acteur, role, Action::AbsencesJustificatifMedicalLire).await? {
            return Err(introuvable());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let rows = sqlx::query_as(
            "SELECT l.justificatif_id, l.lecteur_user_id, l.medical, l.cree_le
               FROM ent_asd_justificatif_lecture l
               JOIN ent_asd_justificatif j ON j.etablissement_id = l.etablissement_id AND j.id = l.justificatif_id
              WHERE l.etablissement_id = $1 AND j.absence_id = $2 ORDER BY l.cree_le, l.id",
        )
        .bind(etab)
        .bind(absence_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows)
    }

    // ── Demandes d'autorisation d'absence (AS-125) ──────────────────────────────────────────

    /// Demande d'autorisation d'absence (fête religieuse, convocation, compétition…) : le responsable
    /// habilité ou l'élève majeur. Dates futures et bornées (120 jours).
    pub async fn demander_autorisation_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        d: DemandeAutorisation,
    ) -> Result<DemandeAsd, EntDbError> {
        if self.qualite_famille(etab, acteur, role, d.eleve_id, false).await?.is_none() {
            return Err(introuvable());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let jour = jour_etab(&mut tx, etab).await?;
        if d.jour_debut < jour || d.jour_fin < d.jour_debut || (d.jour_fin - d.jour_debut).num_days() > 120 {
            return Err(invalide("période invalide : à venir, 120 jours au plus"));
        }
        if (d.jour_debut - jour).num_days() > 366 {
            return Err(invalide("période trop lointaine"));
        }
        if d.jour_fin == d.jour_debut {
            if let (Some(h1), Some(h2)) = (d.heure_debut, d.heure_fin) {
                if h2 <= h1 {
                    return Err(invalide("heures incohérentes"));
                }
            }
        }
        // Motif proposé à la famille : actif, visible, jamais le motif technique.
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_asd_motif WHERE etablissement_id = $1 AND id = $2
                             AND actif AND visible_famille AND code NOT IN ('appel', 'reprise'))",
        )
        .bind(etab)
        .bind(d.motif_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !ok {
            return Err(invalide("motif inconnu ou non proposé à la famille"));
        }
        let (doublon,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_asd_demande WHERE etablissement_id = $1 AND eleve_id = $2
                             AND type = 'autorisation' AND etat = 'en_attente'
                             AND jour_debut <= $4 AND jour_fin >= $3)",
        )
        .bind(etab)
        .bind(d.eleve_id)
        .bind(d.jour_debut)
        .bind(d.jour_fin)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if doublon {
            return Err(conflit("une demande est déjà en attente sur cette période"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_asd_demande
                (id, etablissement_id, eleve_id, type, jour_debut, jour_fin, heure_debut, heure_fin, motif_id, demandeur_user_id)
             VALUES ($1,$2,$3,'autorisation',$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(etab)
        .bind(d.eleve_id)
        .bind(d.jour_debut)
        .bind(d.jour_fin)
        .bind(d.heure_debut)
        .bind(d.heure_fin)
        .bind(d.motif_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let mut publies = Vec::new();
        publier_asd(
            &mut tx,
            etab,
            types_as3::AUTORISATION_DEMANDEE,
            id,
            d.eleve_id,
            json!({ "demande_id": id }),
            Some(acteur),
            &mut publies,
        )
        .await?;
        let r = Self::demande_ligne(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&publies);
        Ok(r)
    }

    async fn demande_ligne(c: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<DemandeAsd, EntDbError> {
        sqlx::query_as::<_, DemandeAsd>(
            "SELECT id, eleve_id, type, jour_debut, jour_fin, heure_debut, heure_fin, motif_id, etat, decision_code, cree_le
               FROM ent_asd_demande WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(c)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    /// Demandes : le personnel qui décide (ou qualifie) voit la file de l'établissement ; la famille voit
    /// celles de son enfant (`eleve_id` obligatoire) ; tout autre acteur reçoit « introuvable ».
    pub async fn demandes_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Option<Uuid>,
    ) -> Result<Vec<DemandeAsd>, EntDbError> {
        let staff = self.a_action_etablissement(etab, acteur, role, Action::AbsencesAutorisationDecider).await?
            || self.peut_qualifier(etab, acteur, role).await?;
        if !staff {
            let Some(e) = eleve_id else { return Err(introuvable()) };
            if self.qualite_famille(etab, acteur, role, e, true).await?.is_none() {
                return Err(introuvable());
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let rows = sqlx::query_as::<_, DemandeAsd>(
            "SELECT id, eleve_id, type, jour_debut, jour_fin, heure_debut, heure_fin, motif_id, etat, decision_code, cree_le
               FROM ent_asd_demande
              WHERE etablissement_id = $1 AND ($2::uuid IS NULL OR eleve_id = $2)
              ORDER BY (etat = 'en_attente') DESC, cree_le DESC, id LIMIT 200",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows)
    }

    /// Décide d'une demande (`absences.autorisation_decider`) : **accordée** (pose une situation
    /// « autorisation » qui pré-remplit l'appel et excuse d'office l'absence), **refusée** (code
    /// obligatoire) ou **transmise au DASEN** en cas de doute (R131-5). Une demande transmise peut être
    /// décidée ensuite.
    pub async fn decider_demande_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande_id: Uuid,
        decision: DecisionDemande,
    ) -> Result<DemandeAsd, EntDbError> {
        if !self.a_action_etablissement(etab, acteur, role, Action::AbsencesAutorisationDecider).await? {
            return Err(introuvable());
        }
        let (etat, code) = match decision.decision.as_str() {
            "accordee" => ("accordee", "accordee".to_string()),
            "transmise_dasen" => ("transmise_dasen", "transmise_dasen".to_string()),
            "refusee" => {
                let c = decision.decision_code.clone().unwrap_or_default();
                if !REFUS_DEMANDE.contains(&c.as_str()) {
                    return Err(invalide("un refus porte un code de décision (refusee_*)"));
                }
                ("refusee", c)
            }
            _ => return Err(invalide("décision inconnue")),
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let d: DemandeAsd = sqlx::query_as(
            "SELECT id, eleve_id, type, jour_debut, jour_fin, heure_debut, heure_fin, motif_id, etat, decision_code, cree_le
               FROM ent_asd_demande WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(demande_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if !matches!(d.etat.as_str(), "en_attente" | "transmise_dasen") || d.type_demande != "autorisation" {
            return Err(conflit("demande déjà décidée ou retirée"));
        }
        if etat == "transmise_dasen" && d.etat == "transmise_dasen" {
            return Err(conflit("demande déjà transmise"));
        }
        let mut situation: Option<Uuid> = None;
        if etat == "accordee" {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_asd_situation
                    (id, etablissement_id, eleve_id, nature, debut_jour, debut_heure, fin_jour, fin_heure, source,
                     origine_ref_type, origine_ref_id, motif_id, declarant_verifie, canal, cree_par)
                 VALUES ($1,$2,$3,'autorisation',$4,$5,$6,$7,'vie_scolaire','ent_asd_demande',$8,$9,true,'web',$10)",
            )
            .bind(id)
            .bind(etab)
            .bind(d.eleve_id)
            .bind(d.jour_debut)
            .bind(d.heure_debut)
            .bind(d.jour_fin)
            .bind(d.heure_fin)
            .bind(d.id)
            .bind(d.motif_id)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "INSERT INTO ent_asd_situation_version (id, etablissement_id, situation_id, etat_avant, etat_apres, auteur_user_id)
                 VALUES ($1,$2,$3,NULL,'active',$4)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            situation = Some(id);
        }
        sqlx::query(
            "UPDATE ent_asd_demande SET etat = $3, decision_code = $4, decide_par = $5, decide_le = now(),
                    situation_id = COALESCE($6, situation_id), modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande_id)
        .bind(etat)
        .bind(&code)
        .bind(acteur)
        .bind(situation)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let mut publies = Vec::new();
        publier_asd(
            &mut tx,
            etab,
            types_as3::AUTORISATION_DECIDEE,
            demande_id,
            d.eleve_id,
            json!({ "demande_id": demande_id, "decision": etat, "decision_code": code }),
            Some(acteur),
            &mut publies,
        )
        .await?;
        let r = Self::demande_ligne(&mut tx, etab, demande_id).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&publies);
        if situation.is_some() {
            let today = self.aujourdhui_etablissement(etab).await?;
            if r.jour_debut <= today && today <= r.jour_fin {
                self.rafraichir_appels_ouverts_eleve(etab, r.eleve_id, today).await?;
            }
        }
        Ok(r)
    }

    /// Retire une demande en attente : son demandeur seulement.
    pub async fn retirer_demande_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        demande_id: Uuid,
    ) -> Result<DemandeAsd, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "UPDATE ent_asd_demande SET etat = 'retiree', modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND demandeur_user_id = $3 AND etat = 'en_attente'",
        )
        .bind(etab)
        .bind(demande_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        let d = Self::demande_ligne(&mut tx, etab, demande_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libelles_famille_a_liste_fermee_et_coherents() {
        for c in LIBELLES_FAMILLE {
            assert!(!libelle_famille_texte(c).is_empty());
        }
        assert!(libelle_coherent("excuse_valable", "absence_justifiee"));
        assert!(!libelle_coherent("non_valable", "absence_justifiee"), "pas de « justifiée » pour un motif refusé");
        assert!(libelle_coherent("non_valable", "motif_non_retenu_delai"));
        assert!(!libelle_coherent("sans_motif", "motif_non_retenu"));
        assert_eq!(libelle_defaut("reputee_legitime"), "absence_justifiee");
    }

    #[test]
    fn natures_medicales_et_conservation_d_un_an() {
        assert!(nature_medicale(Some("maladie_enfant")));
        assert!(nature_medicale(Some("rendez_vous_medical")));
        assert!(!nature_medicale(Some("transport")));
        assert!(!nature_medicale(None));
        let d = NaiveDate::from_ymd_opt(2028, 2, 29).unwrap();
        assert_eq!(ajouter_un_an(d), NaiveDate::from_ymd_opt(2029, 2, 28).unwrap());
    }

    #[test]
    fn textes_trop_longs_refuses() {
        assert!(court(Some("a".repeat(501)), MAX_NOTE).is_err());
        assert_eq!(court(Some("  ".into()), MAX_NOTE).unwrap(), None);
    }
}
