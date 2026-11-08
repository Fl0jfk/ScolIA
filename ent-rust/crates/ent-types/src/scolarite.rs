//! Scolarité de l'élève (socle 2, lot E4) : scolarisations, inscriptions annuelles,
//! changements datés, capacité juridique, mobilité, modalités. Routes `ent-api` sous
//! `/eleves/{id}/…`. Aucune logique métier. Les codes (`motif`, `nature`, `etat`…) sont des
//! chaînes validées côté serveur.
//!
//! Convention de date : `date_sortie` est le PREMIER jour où l'élève n'est plus dans
//! l'établissement.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtablissementBody {
    /// UAI (lettres et chiffres, 20 au plus) ; facultatif.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uai: Option<String>,
    /// Libellé libre de secours (instruction en famille, étranger…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub libelle: Option<String>,
    /// `public`, `prive_sous_contrat`, `prive_hors_contrat`, `instruction_famille`, `etranger`, `inconnu`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
}

/// ELV-08 — `POST /eleves/{id}/scolarisations`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OuvrirScolarisationBody {
    pub date_entree: NaiveDate,
    /// `premiere_inscription`, `changement_etablissement`, `passage_interne`, `reprise`.
    pub motif: String,
    #[serde(default)]
    pub origine: EtablissementBody,
}

/// ELV-12 — `POST /eleves/{id}/scolarisations/reintegrer` (motif « retour »).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReintegrerBody {
    pub date_entree: NaiveDate,
    #[serde(default)]
    pub origine: EtablissementBody,
}

/// ELV-11 — `POST /eleves/{id}/scolarisations/cloturer`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloturerScolarisationBody {
    pub date_sortie: NaiveDate,
    /// `fin_de_cycle`, `changement_etablissement`, `demenagement`, `exclusion_definitive`,
    /// `abandon`, `fin_obligation`, `instruction_famille`, `etranger`, `autre`.
    pub motif: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif_libre: Option<String>,
    #[serde(default)]
    pub destination: EtablissementBody,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exeat_document_id: Option<Uuid>,
}

/// ELV-34 — `POST /eleves/{id}/deces`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecesBody {
    pub date: NaiveDate,
}

/// ELV-09 — `PUT /eleves/{id}/inscriptions/{annee_id}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InscriptionAnnuelleBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub niveau_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mef_id: Option<Uuid>,
    /// `scolaire` (défaut), `apprenti`, `autre`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statut_apprenant: Option<String>,
    /// `prevue`, `confirmee`, `en_cours`, `terminee`, `annulee`.
    pub etat: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redoublement: Option<bool>,
}

/// ELV-09 — `POST /eleves/{id}/scolarite/classe`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangerClasseBody {
    pub classe_id: Uuid,
    pub date_effet: NaiveDate,
}

/// ELV-09 — `POST /eleves/{id}/scolarite/options` (options = groupes du référentiel).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangerOptionsBody {
    pub annee_id: Uuid,
    #[serde(default)]
    pub ajouter: Vec<Uuid>,
    #[serde(default)]
    pub retirer: Vec<Uuid>,
    pub date_effet: NaiveDate,
}

/// ELV-10 — `POST /eleves/{id}/scolarite/regime` et `/scolarite/site`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangerRegimeSiteBody {
    pub valeur: String,
    pub date_effet: NaiveDate,
    /// Régime seulement, hors dates autorisées : `demenagement`, `raison_medicale`,
    /// `situation_familiale`, `decision_etablissement`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derogation: Option<String>,
}

/// ELV-13 — `POST /eleves/{id}/capacites`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarerCapaciteBody {
    /// `emancipation`, `tutelle`, `curatelle`, `sauvegarde_justice`, `habilitation_familiale`.
    pub nature: String,
    pub du: NaiveDate,
    /// Pièce obligatoire (jugement, acte) : document administratif ou familial de l'élève.
    pub piece_document_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeverCapaciteBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub au: Option<NaiveDate>,
}

/// ELV-35 — `POST /eleves/{id}/mobilites`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OuvrirMobiliteBody {
    /// `etranger`, `soins`, `cned`, `autre`.
    pub type_: String,
    pub du: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retour_prevu: Option<NaiveDate>,
}

/// Fin d'une mobilité ou d'une modalité.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinPeriodeBody {
    pub au: NaiveDate,
}

/// ELV-36 — `POST /eleves/{id}/modalites`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnregistrerModaliteBody {
    /// `temps_partiel`, `double_inscription_cned`, `scolarisation_partagee`, `upe2a`, `ulis`, `autre`.
    pub code: String,
    pub du: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub au: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub piece_document_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScolarisationDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub date_entree: NaiveDate,
    pub motif_entree: String,
    pub origine: EtablissementBody,
    pub date_sortie: Option<NaiveDate>,
    pub motif_sortie: Option<String>,
    pub motif_sortie_libre: Option<String>,
    pub destination: EtablissementBody,
    pub exeat_document_id: Option<Uuid>,
    pub source: String,
    pub cree_le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InscriptionDto {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub scolarisation_id: Uuid,
    pub niveau_id: Option<Uuid>,
    pub mef_id: Option<Uuid>,
    pub statut_apprenant: String,
    pub etat: String,
    pub redoublement: Option<bool>,
    pub decision_passage: Option<String>,
    pub source: String,
    /// Libellé de l'année, fourni aux familles (le référentiel des années ne leur est pas ouvert).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annee_libelle: Option<String>,
}

/// `GET /eleves/{id}/scolarite` : statut DÉRIVÉ (jamais saisi) et historique.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScolariteDto {
    /// `a_venir`, `scolarise`, `sorti`, `archive`.
    pub statut: String,
    pub scolarisations: Vec<ScolarisationDto>,
    pub inscriptions: Vec<InscriptionDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatScolarisationDto {
    pub scolarisation: ScolarisationDto,
    pub statut: String,
    pub statut_change: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapaciteDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nature: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub piece_document_id: Uuid,
    pub motif: Option<String>,
    pub saisi_le: DateTime<Utc>,
    pub levee_le: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobiliteDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub type_: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub retour_prevu: Option<NaiveDate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModaliteDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub code: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
}
