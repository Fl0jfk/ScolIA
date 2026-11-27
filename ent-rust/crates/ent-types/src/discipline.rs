//! Discipline et assiduité (socle 2, lot S2-4) : punitions (≠ sanctions), procédure
//! disciplinaire, signalements d'assiduité, décompte en demi-journées. JSON des structures
//! d'`ent-db` (champs inconnus ignorés). Aucune logique métier.

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type de punition ou de sanction (`/vs/punition-types`, `/vs/sanction-types`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeVsDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub actif: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PunitionDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub type_id: Uuid,
    pub jour: NaiveDate,
    pub motif: String,
    #[serde(default)]
    pub echeance: Option<NaiveDate>,
    #[serde(default)]
    pub duree_minutes: Option<i32>,
    #[serde(default)]
    pub incident_id: Option<Uuid>,
    /// `donnee` | `effectuee` | `annulee`.
    pub statut: String,
}

/// Corps de `POST /eleves/{id}/punitions`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DonnerPunitionBody {
    pub type_id: Uuid,
    pub jour: NaiveDate,
    pub motif: String,
    #[serde(default)]
    pub echeance: Option<NaiveDate>,
    #[serde(default)]
    pub duree_minutes: Option<i32>,
}

/// Sanction (`GET /eleves/{id}/sanctions`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanctionDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub type_id: Uuid,
    #[serde(default)]
    pub type_label: Option<String>,
    #[serde(default)]
    pub motif_label: Option<String>,
    pub jour: NaiveDate,
    #[serde(default)]
    pub duree_minutes: Option<i32>,
    #[serde(default)]
    pub note: Option<String>,
    pub statut: String,
}

/// Procédure disciplinaire (chef d'établissement ou conseil de discipline).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub instance: String,
    pub statut: String,
    pub faits: String,
    pub engagee_le: NaiveDate,
    #[serde(default)]
    pub observations_recueillies_le: Option<NaiveDate>,
    #[serde(default)]
    pub convocation_envoyee_le: Option<NaiveDate>,
    #[serde(default)]
    pub conseil_le: Option<NaiveDate>,
    pub delai_convocation_jours: i32,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub decision_le: Option<NaiveDate>,
}

/// Corps de `POST /vs/procedures`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngagerProcedureBody {
    pub eleve_id: Uuid,
    pub instance: String,
    pub faits: String,
}

/// Signalement d'assiduité (`propose` → `valide` → `emis`, ou `refuse`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignalementDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub mois: NaiveDate,
    pub demi_journees: i32,
    pub seuil: i32,
    pub statut: String,
    #[serde(default)]
    pub dossier: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecompteEleveDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub demi_journees_comptees: i64,
    pub demi_journees_non_justifiees: i64,
    pub seuil_atteint: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecompteMoisDto {
    pub mois: NaiveDate,
    pub seuil_signalement: i32,
    pub eleves: Vec<DecompteEleveDto>,
}

/// Règle d'assiduité de l'établissement (hypothèses de travail paramétrables).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegleAssiduiteDto {
    pub heure_coupure: NaiveTime,
    pub seuil_minutes: i32,
    pub seuil_signalement: i32,
    pub delai_verification_minutes: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffacementRapportDto {
    pub examinees: usize,
    #[serde(default)]
    pub effacees: Vec<Uuid>,
}

/// Demi-journée d'absence d'un élève, dérivée par le serveur des absences à la séance
/// (`GET /eleves/{id}/demi-journees`). Le client n'en recalcule jamais les règles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemiJourneeDto {
    pub jour: NaiveDate,
    /// `matin` | `apres_midi`.
    pub periode: String,
    pub minutes: i64,
    pub minutes_non_justifiees: i64,
    /// La demi-journée compte (seuil de minutes atteint).
    pub comptee: bool,
    pub non_justifiee: bool,
}
