//! Personnels (socle 6, lot P6) : délégations par objet, intérim de direction, pouvoirs de signature
//! (`docs/socles/06_PERSONNELS.md` §8.9, §10.7, PRS-80 à 84). Routes `ent-api/src/personnels_delegations.rs`.
//! Aucune logique métier.
//!
//! La délégation de DROITS (`/delegations`) existe depuis S2-5 (ADM-04) ; P6 l'enrichit (objet, agents, motif codé)
//! et ajoute les délégations de MISSION, de SIGNATURE et l'INTÉRIM de direction. Le port `peut_signer` du socle 7
//! (documents) renvoie un `PouvoirSignatureDto`, FORMULE comprise (« Pour le chef d'établissement et par délégation »).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Formule imprimée sous la signature d'un délégataire (donnée renvoyée par `peut_signer`).
pub const FORMULE_PAR_DELEGATION: &str = "Pour le chef d'établissement et par délégation";

// ── Pouvoirs de signature (PRS-82) ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HabilitationDto {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub agent_nom: Option<String>,
    /// Natures de documents ; `*` = toutes (intérim de direction).
    pub documents: Vec<String>,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    /// `habilitation`, `delegation`, `interim`.
    pub origine: String,
    pub origine_ref: Option<Uuid>,
    pub formule: Option<String>,
    pub revoquee: bool,
    /// Valide au jour de l'établissement.
    pub active: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HabiliterBody {
    pub agent_id: Uuid,
    pub documents: Vec<String>,
    /// Début (défaut : aujourd'hui) ; jamais dans le passé.
    #[serde(default)]
    pub du: Option<NaiveDate>,
    /// Fin facultative pour une habilitation permanente ; obligatoire pour une délégation.
    #[serde(default)]
    pub au: Option<NaiveDate>,
    /// Formule sous la signature (défaut : signature en son nom propre).
    #[serde(default)]
    pub formule: Option<String>,
}

/// Réponse du port `peut_signer(agent, nature, date)` (socle 7).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PouvoirSignatureDto {
    pub autorise: bool,
    /// `fonction` (chef d'établissement), `habilitation`, `delegation`, `interim`. `None` si refusé.
    pub origine: Option<String>,
    pub habilitation_id: Option<Uuid>,
    /// Formule à imprimer sous la signature ; `None` : signature en son nom propre.
    pub formule: Option<String>,
    /// Dernier jour du pouvoir (`None` : permanent ou refusé).
    pub jusqu_au: Option<NaiveDate>,
}

// ── Délégations par objet (PRS-80, 81, 83) ─────────────────────────────────────────────────────

#[derive(Clone, Debug, Deserialize)]
pub struct DeleguerMissionBody {
    /// Mission détenue par le délégant (le délégant est son titulaire).
    pub mission_id: Uuid,
    pub delegataire_agent_id: Uuid,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    /// Fin obligatoire, au plus la fin de la mission détenue.
    pub au: NaiveDate,
    /// `absence`, `surcharge`, `organisation`, `autre` (défaut).
    #[serde(default)]
    pub motif_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeleguerSignatureBody {
    pub delegataire_agent_id: Uuid,
    pub documents: Vec<String>,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    pub au: NaiveDate,
    #[serde(default)]
    pub motif_code: Option<String>,
    #[serde(default)]
    pub formule: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OrganiserInterimBody {
    pub delegataire_agent_id: Uuid,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    pub au: NaiveDate,
    #[serde(default)]
    pub motif_code: Option<String>,
    #[serde(default)]
    pub formule: Option<String>,
}

/// Une délégation créée ou lue par objet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DelegationObjetDto {
    pub id: Uuid,
    /// `mission`, `signature`, `interim_direction` (`role` : voir `/delegations`).
    pub objet: String,
    pub delegant_agent_id: Option<Uuid>,
    pub delegataire_agent_id: Option<Uuid>,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub motif_code: String,
    /// `a_venir`, `active`, `expiree`, `revoquee`.
    pub etat: String,
    /// Mission copiée (objet `mission`) ou habilitation produite (signature, intérim).
    pub mission_derivee_id: Option<Uuid>,
    pub habilitation_id: Option<Uuid>,
}
