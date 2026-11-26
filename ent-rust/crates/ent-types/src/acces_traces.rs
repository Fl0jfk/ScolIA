//! Socle 27, lot G4-1 : **traces d'accès**. Contrat JSON des routes `/acces/*` d'`ent-api` (`27_RGPD_ARCHIVES.md` §11).
//!
//! La page « Consultations du dossier » ne porte **jamais** le nom de l'agent (§11.3, S-549) : seulement sa fonction et
//! son service, figés au moment de la consultation (R-RG31).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Un motif proposé pour une consultation hors périmètre (liste S-542).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotifConsultationDto {
    pub code: String,
    pub libelle: String,
}

/// Réponse « motif requis » (HTTP 409) à l'ouverture d'une fiche hors du périmètre de l'agent (RGP-191).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotifRequisDto {
    /// Toujours `motif_requis`.
    pub code: String,
    pub motifs: Vec<MotifConsultationDto>,
}

/// Une consultation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneConsultationDto {
    pub le: DateTime<Utc>,
    pub fonction: String,
    pub service: String,
    pub nature: String,
    pub section: String,
    pub motif: Option<String>,
    /// « Support ScolIA, intervention autorisée le … » pour le support de l'éditeur.
    pub mention: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JourConsultationsDto {
    pub jour: NaiveDate,
    pub lignes: Vec<LigneConsultationDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DossierConsulteDto {
    /// `eleve` ou `agent`.
    pub sujet_type: String,
    pub sujet_id: Uuid,
    /// Prénom de l'élève ; vide pour le dossier de l'agent.
    pub libelle: String,
    pub total: i64,
    pub jours: Vec<JourConsultationsDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsultationsDossierDto {
    pub depuis: NaiveDate,
    pub dossiers: Vec<DossierConsulteDto>,
}

// ─────────────── Lot G4-2 : contrôles internes, alertes, réglages, traces d'un sujet ───────────────

/// Un réglage d'accès avec sa valeur par défaut et ses bornes (S-541, S-543, S-554).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneReglageDto {
    pub code: String,
    pub libelle: String,
    pub valeur: i32,
    pub defaut: i32,
    pub min: i32,
    pub max: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangementReglageDto {
    pub reglage: String,
    pub ancienne: Option<String>,
    pub nouvelle: String,
    pub le: DateTime<Utc>,
}

/// Réglages d'accès et historique des modifications de tous les réglages RGPD.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglagesAccesDto {
    pub lignes: Vec<LigneReglageDto>,
    pub historique: Vec<ChangementReglageDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglageAccesBody {
    pub reglage: String,
    pub valeur: i32,
}

/// Demande d'ouverture d'un contrôle : sert au défi puis à l'ouverture (mêmes valeurs, liées par le défi).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControleDemandeBody {
    pub agent_compte_id: Uuid,
    /// `signalement`, `alerte`, `plainte` ou `verification_periodique`.
    pub motif_code: String,
    pub motif_texte: String,
    pub jours: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControleOuvrirBody {
    #[serde(flatten)]
    pub demande: ControleDemandeBody,
    pub defi_id: Uuid,
    pub credential: serde_json::Value,
}

/// Défi de passkey récente lié au contrôle demandé.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControleDefiDto {
    pub defi_id: Uuid,
    pub options: serde_json::Value,
    pub intention: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControleDto {
    pub id: Uuid,
    pub agent_compte_id: Uuid,
    pub agent_nom: String,
    pub motif_code: String,
    pub motif_texte: String,
    pub ouvert_le: DateTime<Utc>,
    pub echeance_le: DateTime<Utc>,
    pub clos_le: Option<DateTime<Utc>>,
    pub issue: Option<String>,
    pub ouvert: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControleCloreBody {
    /// `rien`, `violation` ou `transmis`.
    pub issue: String,
}

/// Une consultation vue par la direction ou le DPO : fonction et service, jamais le nom (sauf arbitrage motivé).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceLigneDto {
    pub id: Uuid,
    pub le: DateTime<Utc>,
    pub fonction: String,
    pub service: String,
    pub nature: String,
    pub section: String,
    pub hors_perimetre: bool,
    pub perimetre_temporaire: Option<String>,
    pub motif: Option<String>,
    /// Élève consulté (traces d'un agent sous contrôle) ; vide sinon.
    pub sujet: String,
    pub differee: bool,
    pub nom_communique: bool,
    pub agent_nom: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TracesSujetDto {
    pub sujet_type: String,
    pub sujet_id: Uuid,
    pub libelle: String,
    pub lignes: Vec<TraceLigneDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TracesControleDto {
    pub controle: ControleDto,
    pub lignes: Vec<TraceLigneDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlerteAccesDto {
    pub id: Uuid,
    pub regle: String,
    pub libelle: String,
    pub agent_compte_id: Uuid,
    pub agent_nom: String,
    pub fenetre_debut: DateTime<Utc>,
    pub fenetre_fin: DateTime<Utc>,
    pub nombre: i32,
    pub cree_le: DateTime<Utc>,
    /// `a_traiter`, `rien`, `violation` ou `controle`.
    pub suite: String,
    pub suite_le: Option<DateTime<Utc>>,
    pub controle_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlerteSuiteBody {
    pub suite: String,
}

/// Report de l'affichage d'une consultation liée à un signalement (R-RG28) : motivé, borné à 90 jours.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifferBody {
    pub trace_id: Uuid,
    pub motif: String,
    pub jours: i64,
}

/// Communication du nom d'un agent pour une demande (CL27-16) : arbitrage motivé de la direction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NomAgentBody {
    pub trace_id: Uuid,
    pub motif: String,
}
