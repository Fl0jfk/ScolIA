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
