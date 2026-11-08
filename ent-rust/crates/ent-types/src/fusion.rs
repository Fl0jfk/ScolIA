//! Doublons et fusion de fiches d'élèves (socle 2, lot E6) : ELV-22, ELV-23, ELV-24, ELV-33.
//! Routes `ent-api` sous `/eleves/doublons…` et `/eleves/fusions…`. Aucune logique métier.
//!
//! Aucune charge utile ne porte une valeur d'identité autre que celle que l'élève montre déjà à
//! l'acteur (désignation, classe, et, selon ses droits, état civil et INE) : les champs que
//! l'acteur ne peut pas voir sont absents (`null`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Un élève dans une comparaison, minimisé selon les droits de l'acteur.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EleveComparaisonDto {
    pub id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub statut: String,
    /// Absents sans le droit correspondant (état civil G2, identifiants G3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_naissance: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ine: Option<String>,
}

/// `GET /eleves/doublons` : une paire de doublons probables.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DoublonDto {
    pub id: Uuid,
    /// 0 à 100 (60 au moins pour figurer dans la liste).
    pub score: u8,
    /// Codes : `ine_identique`, `identite_exacte`, `nom_prenom_proches`, `meme_naissance`…
    pub raisons: Vec<String>,
    pub detecte_le: DateTime<Utc>,
    pub a: EleveComparaisonDto,
    pub b: EleveComparaisonDto,
}

/// `POST /eleves/doublons/detecter` : bilan d'un passage de détection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BilanDetectionDto {
    pub paires: usize,
    pub nouvelles: usize,
    pub devenues_obsoletes: usize,
}

/// `POST /eleves/doublons/non-doublon` (ELV-33) : « ce sont bien deux élèves ».
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NonDoublonBody {
    pub eleve_a: Uuid,
    pub eleve_b: Uuid,
    /// `jumeaux`, `homonymes`, `fratrie` ou `autre`.
    pub motif_code: String,
}

/// Ligne de l'aperçu : ce qui passe, reste ou est en conflit pour un domaine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LigneApercuDto {
    pub domaine: String,
    /// Absent pour les domaines protégés (santé, suivi) : seulement un total.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    pub a_deplacer: usize,
    pub en_conflit: usize,
    pub restent: usize,
}

/// Une ligne de l'absorbé qui ne peut pas passer à l'élève conservé (clé, jamais le contenu).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConflitFusionDto {
    pub table: String,
    pub cle: Value,
    pub domaine: String,
}

/// `POST /eleves/fusions/apercu` : ce que la fusion ferait. Rien n'est écrit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApercuFusionDto {
    pub absorbe: EleveComparaisonDto,
    pub conserve: EleveComparaisonDto,
    pub lignes: Vec<LigneApercuDto>,
    pub conflits: Vec<ConflitFusionDto>,
    /// Champs d'identité qui diffèrent (`nom`, `prenom`, `date_naissance`…).
    pub differences: Vec<String>,
    /// `eleve_protege`, `declaree_distincte`, `ine_differents`, `deux_comptes`,
    /// `absorbe_encore_scolarise`.
    pub avertissements: Vec<String>,
    pub possible: bool,
}

/// `POST /eleves/fusions/apercu` et `POST /eleves/fusions` : les deux fiches.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApercuFusionBody {
    pub absorbe_id: Uuid,
    pub conserve_id: Uuid,
}

/// `POST /eleves/fusions` (ELV-23).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusionnerBody {
    pub absorbe_id: Uuid,
    pub conserve_id: Uuid,
    /// `doublon_saisie`, `doublon_import`, `erreur_creation` ou `autre` (motif libre obligatoire).
    pub motif_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif: Option<String>,
    /// Chaque conflit de l'aperçu, regardé et accepté : la ligne du conservé l'emporte.
    #[serde(default)]
    pub conflits_acquittes: Vec<ConflitFusionDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusionDto {
    pub id: Uuid,
    pub absorbe_id: Uuid,
    pub conserve_id: Uuid,
    pub motif_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif: Option<String>,
    pub conflits: Vec<ConflitFusionDto>,
    pub lignes: Vec<LigneApercuDto>,
    pub fusionne_le: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annule_le: Option<DateTime<Utc>>,
    /// Dernier instant où l'annulation est possible (30 jours).
    pub annulable_jusqu_au: DateTime<Utc>,
}

/// Réponse de `POST /eleves/fusions` : la fusion faite, ou les conflits à acquitter (rien n'est écrit).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultatFusionDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fusion: Option<FusionDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflits_a_resoudre: Vec<ConflitFusionDto>,
}

/// Une ligne déplacée qui a changé depuis la fusion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LigneModifieeDto {
    pub domaine: String,
    pub table: String,
    pub cle: Value,
    /// `modifiee` ou `absente`.
    pub raison: String,
}

/// Réponse de `POST /eleves/fusions/{id}/annuler` : la fusion annulée, ou la liste des lignes
/// modifiées depuis (rien n'est restauré).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultatDefusionDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fusion: Option<FusionDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lignes_modifiees: Vec<LigneModifieeDto>,
}
