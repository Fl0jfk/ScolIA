//! Confidentialité renforcée, photos et coordonnées propres de l'élève (socle 2, lot E7) :
//! ELV-15, ELV-16, ELV-17, ELV-42 et journal de consultation. Routes `ent-api` sous
//! `/eleves/{id}/confidentialite`, `/eleves/{id}/photos`, `/eleves/{id}/coordonnees`,
//! `/eleves/{id}/consultations`… Aucune logique métier.
//!
//! * Le **motif** d'une mesure n'est jamais renvoyé qu'à G6, descellé, dans `MesureConfidentialiteDto`.
//! * Aucune valeur d'identité n'est portée par une charge utile autre que la coordonnée que
//!   l'acteur a le droit de lire.
//! * La clé d'objet d'une photo n'est jamais exposée : la photo se sert par `GET /eleves/{id}/photo`
//!   ou par son lien court.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn vrai() -> bool {
    true
}

// ─────────────────────────── Confidentialité renforcée (ELV-15) ───────────────────────────

/// `POST` / `PUT /eleves/{id}/confidentialite` : poser ou remplacer la mesure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoserConfidentialiteBody {
    /// Motif en clair ; scellé côté serveur, jamais relu hors G6.
    pub motif: String,
    /// Mesures demandées en plus des effets minimaux : `hors_trombinoscope_exporte` (**effet en E11** :
    /// enregistrée seulement, aucun export n'existe encore ; le lecteur sera écrit avec l'export),
    /// `adresse_protegee`, `alerte_consultation` (`masquer_photo` et `hors_annuaire` sont toujours posées).
    #[serde(default)]
    pub mesures: Vec<String>,
    /// Début (défaut : aujourd'hui, jour de l'établissement).
    #[serde(default)]
    pub du: Option<NaiveDate>,
    /// Fin prévue (facultative).
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

/// `POST /eleves/{id}/confidentialite/lever`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeverConfidentialiteBody {
    /// `fin_de_mesure`, `decision_levee`, `erreur_de_saisie`, `changement_de_mesure`, `autre`.
    pub motif_code: String,
}

/// Une mesure. `motif` : G6 seulement (`GET /eleves/{id}/confidentialite`), jamais dans une liste.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MesureConfidentialiteDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub mesures: Vec<String>,
    pub du: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub au: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decide_par: Option<Uuid>,
    pub decide_le: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leve_par: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leve_le: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leve_motif_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif: Option<String>,
}

/// `POST …/lever` : nombre de mesures levées.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeveeResultatDto {
    pub levees: u64,
}

/// `GET /eleves/{id}/consultations` : une ligne du journal de consultation (ajout seul).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConsultationDto {
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eleve_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acteur: Option<Uuid>,
    /// `fiche_protegee`, `etat_civil`, `identifiants`, `coordonnees`, `sensible`, `pastilles_sensibles`…
    pub action: String,
    pub libelle: String,
    pub quand: DateTime<Utc>,
}

// ─────────────────────────── Photos (ELV-16, ELV-42) ───────────────────────────

/// `POST /eleves/{id}/photos` : déposer une photo (même forme que la route héritée `…/photo`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeposerPhotoBody {
    pub mime_type: String,
    pub contenu_base64: String,
    #[serde(default)]
    pub prise_le: Option<NaiveDate>,
    /// Personnel seulement : valider du même geste (défaut : oui). Ignoré pour la famille et l'élève.
    #[serde(default = "vrai")]
    pub valider_aussitot: bool,
    /// Personnel seulement : photo d'un photographe scolaire plutôt que de l'établissement.
    #[serde(default)]
    pub photographe: bool,
}

/// Motif codé d'un rejet ou d'un retrait de photo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotifBody {
    pub motif_code: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhotoDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    /// `etablissement`, `photographe`, `famille`, `eleve`, `import` (déduite de l'acteur).
    pub source: String,
    /// `proposee`, `validee`, `retiree`, `rejetee`.
    pub statut: String,
    pub mime: String,
    pub taille_octets: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prise_le: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub propose_par: Option<Uuid>,
    pub propose_le: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decide_par: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decide_le: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif_code: Option<String>,
}

// ─────────────────────────── Coordonnées propres (ELV-17) ───────────────────────────

/// `POST /eleves/{id}/coordonnees` : ajouter (ou, pour la famille, proposer) une coordonnée.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AjouterCoordonneeBody {
    /// `email` ou `mobile`.
    #[serde(rename = "type")]
    pub type_: String,
    pub valeur: String,
    /// `notifications`, `compte` (e-mail seulement), `urgence`.
    #[serde(default)]
    pub usages: Vec<String>,
}

/// `POST /eleves/coordonnees/{id}/decision` : le secrétariat accepte ou refuse une proposition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionCoordonneeBody {
    pub accepter: bool,
    /// Obligatoire pour un refus.
    #[serde(default)]
    pub motif_code: Option<String>,
}

/// `POST /eleves/coordonnees/{id}/usages`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UsagesBody {
    pub usages: Vec<String>,
}

/// `POST /eleves/coordonnees/{id}/verifier`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CodeBody {
    pub code: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoordonneeEleveDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    #[serde(rename = "type")]
    pub type_: String,
    pub valeur: String,
    pub usages: Vec<String>,
    /// `etablissement`, `eleve`, `famille`, `import`.
    pub source: String,
    /// `proposee`, `active`, `refusee`, `retiree`.
    pub statut: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verifiee_le: Option<DateTime<Utc>>,
    pub du: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub au: Option<NaiveDate>,
    pub cree_le: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decide_le: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif_code: Option<String>,
}

/// `POST /eleves/coordonnees/{id}/verification` : ce que l'écran peut dire (jamais le code).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VerificationEnvoyeeDto {
    pub coordonnee_id: Uuid,
    /// `email` ou `sms`.
    pub canal: String,
    pub expire_le: DateTime<Utc>,
}

/// `GET /eleves/photos/manquantes` : élèves sans photo validée ni refus ouvert.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ElevesDto {
    pub eleves: Vec<Uuid>,
}
