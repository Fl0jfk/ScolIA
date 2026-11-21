//! Personnels (socle 6, lot P4) : missions, périmètre des droits, explication et simulation
//! (`docs/socles/06_PERSONNELS.md` §8.6, §9, §10.5, PRS-60 à 65, 67, 101, 102).
//! Routes `ent-api/src/personnels_missions.rs` (préfixe `/personnels/missions`). Aucune logique métier.
//!
//! Une **mission** donne à un agent une responsabilité sur un **périmètre** (une classe pour le professeur
//! principal, des niveaux pour un CPE référent, des élèves pour un AESH…). Quand son type porte un
//! modèle de rôle, la mission crée une attribution de ce rôle dont les cibles sont le périmètre : la
//! portée `mon_perimetre` les lit, et la fin de la mission les termine.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Un élément du périmètre d'une mission. `nature` : `etablissement`, `site`, `niveau`, `classe`,
/// `groupe`, `eleves`, `agents`, `discipline`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PerimetreMissionDto {
    pub nature: String,
    #[serde(default)]
    pub cible_id: Option<Uuid>,
    #[serde(default)]
    pub discipline_code: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TypeMissionDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub description: Option<String>,
    pub perimetres_admis: Vec<String>,
    pub cardinalite_max: Option<i16>,
    pub annuelle: bool,
    /// Le type applique un rôle au périmètre pendant la mission.
    pub porte_un_role: bool,
    pub visible_familles: bool,
    pub visible_eleves: bool,
    pub regle_eligibilite: String,
    pub effet_regle: String,
    pub proprietaire_socle: Option<i16>,
    /// Fourni par ScolIA (non modifiable) ou propre à l'établissement.
    pub fourni: bool,
    pub actif: bool,
}

/// `POST /personnels/missions/types` (PRS-64).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CreerTypeMissionBody {
    pub code: String,
    pub libelle: String,
    #[serde(default)]
    pub description: Option<String>,
    pub perimetres_admis: Vec<String>,
    #[serde(default)]
    pub cardinalite_max: Option<i16>,
    #[serde(default)]
    pub annuelle: bool,
    /// Rôle de l'établissement appliqué au périmètre (anti-escalade : l'acteur doit couvrir ses droits).
    #[serde(default)]
    pub modele_role_id: Option<Uuid>,
    #[serde(default)]
    pub visible_familles: bool,
    #[serde(default)]
    pub visible_eleves: bool,
    /// `enseigne_dans_perimetre`, `fonction`, `aucune` (défaut).
    #[serde(default)]
    pub regle_eligibilite: Option<String>,
    /// `avertir` (défaut) ou `bloquer`.
    #[serde(default)]
    pub effet_regle: Option<String>,
    #[serde(default)]
    pub fonctions_eligibles: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionDto {
    pub id: Uuid,
    pub type_id: Uuid,
    pub type_code: String,
    pub type_libelle: String,
    pub agent_id: Uuid,
    pub agent: String,
    pub annee_id: Option<Uuid>,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub origine: String,
    pub perimetres: Vec<PerimetreMissionDto>,
    pub annulee: bool,
    /// Active à la date de la requête.
    pub active: bool,
    /// Un rôle est appliqué au périmètre (le compte de l'agent existe et le type porte un modèle).
    pub droits_appliques: bool,
    /// Éligibilité non satisfaite sous l'effet `avertir` (codes, jamais de nom) : l'écran les affiche.
    pub avertissements: Vec<String>,
}

/// `POST /personnels/missions` (PRS-60 professeur principal, PRS-62 référent, coordinateur, tuteur).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NommerMissionBody {
    #[serde(default)]
    pub type_id: Option<Uuid>,
    /// Alternative à `type_id` : le code du type (`professeur_principal`…).
    #[serde(default)]
    pub type_code: Option<String>,
    pub agent_id: Uuid,
    pub perimetres: Vec<PerimetreMissionDto>,
    /// Défaut : le jour de l'établissement.
    #[serde(default)]
    pub du: Option<NaiveDate>,
    /// Défaut : fin de l'année scolaire pour une mission annuelle, sinon sans fin.
    #[serde(default)]
    pub au: Option<NaiveDate>,
    #[serde(default)]
    pub annee_id: Option<Uuid>,
}

/// Une ligne de `POST /personnels/missions/professeurs-principaux` (PRS-61).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PpClasseBody {
    pub classe_id: Uuid,
    pub agent_id: Uuid,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NommerPpLotBody {
    pub nominations: Vec<PpClasseBody>,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    #[serde(default)]
    pub annee_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LotMissionsDto {
    pub missions: Vec<MissionDto>,
}

/// `GET /personnels/missions/professeurs-principaux` (PRS-61) : classes × candidats.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidatPpDto {
    pub agent_id: Uuid,
    pub agent: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClassePpDto {
    pub classe_id: Uuid,
    pub classe: String,
    /// Professeurs principaux en cours à la date.
    pub professeurs_principaux: Vec<CandidatPpDto>,
    /// Agents qui ont une intervention sur un cours de la classe, en tête de liste.
    pub candidats: Vec<CandidatPpDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableauPpDto {
    pub annee_id: Option<Uuid>,
    pub classes: Vec<ClassePpDto>,
    /// Classes sans professeur principal à la date.
    pub classes_sans_pp: Vec<Uuid>,
}

/// `POST /personnels/missions/{id}/terminer` (PRS-63).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminerMissionBody {
    pub au: NaiveDate,
}

/// `POST /personnels/missions/{id}/transferer` (PRS-63) : fin de l'ancienne mission la veille, nouvelle
/// mission le jour du transfert, même type, même périmètre.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransfererMissionBody {
    pub agent_id: Uuid,
    pub le: NaiveDate,
}

/// `POST /personnels/missions/reconduire` (PRS-65) : une PROPOSITION (`appliquer` faux, défaut) ou son
/// application. Les missions annuelles de l'année source qui ne visent ni classe ni groupe sont
/// reconduites telles quelles ; celles qui visent une classe ou un groupe demandent une cible de l'année
/// suivante (`cibles`) : sans elle, elles restent proposées et ne sont pas appliquées.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReconduireMissionsBody {
    pub annee_source_id: Uuid,
    pub annee_cible_id: Uuid,
    #[serde(default)]
    pub appliquer: bool,
    #[serde(default)]
    pub cibles: Vec<ReconductionCibleBody>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReconductionCibleBody {
    pub mission_id: Uuid,
    /// Nouvelle cible (classe ou groupe de l'année suivante) en remplacement de l'ancienne.
    pub cible_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropositionReconductionDto {
    pub mission_id: Uuid,
    pub type_code: String,
    pub agent_id: Uuid,
    pub agent: String,
    /// `reconductible` (périmètre inchangé), `cible_a_choisir` (classe ou groupe : choisir la suivante),
    /// `agent_sans_engagement` (aucun engagement actif au début de l'année suivante).
    pub etat: String,
    pub appliquee: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReconductionDto {
    pub propositions: Vec<PropositionReconductionDto>,
}

/// `POST /personnels/missions/simuler` (PRS-102) : « Mme X verra N élèves de plus, jusqu'au… ».
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SimulerMissionBody {
    #[serde(default)]
    pub type_id: Option<Uuid>,
    #[serde(default)]
    pub type_code: Option<String>,
    pub agent_id: Uuid,
    pub perimetres: Vec<PerimetreMissionDto>,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    #[serde(default)]
    pub annee_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimulationDto {
    /// Élèves que l'agent verrait EN PLUS de ce qu'il voit déjà (par son enseignement).
    pub eleves_supplementaires: i64,
    /// Élèves du périmètre de la mission, au total.
    pub eleves_du_perimetre: i64,
    /// Codes des actions que le rôle du type accorderait sur ce périmètre.
    pub actions: Vec<String>,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    /// Le type ne porte aucun rôle : la mission n'ouvre aucun droit.
    pub sans_effet_sur_les_droits: bool,
    /// L'agent n'a pas de compte : aucun droit tant qu'il n'en a pas.
    pub agent_sans_compte: bool,
    pub avertissements: Vec<String>,
}

/// Une raison pour laquelle un agent voit un élève (PRS-101).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RaisonAccesDto {
    /// `role_etablissement`, `intervention`, `mission`, `groupe_responsable`, `role_designe`.
    pub origine: String,
    pub action: Option<String>,
    pub service_id: Option<Uuid>,
    pub intervention_nature: Option<String>,
    pub mission_id: Option<Uuid>,
    pub mission_type_code: Option<String>,
    pub groupe_id: Option<Uuid>,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
    /// Lecture seule : l'intervention ne saisit pas à cette date.
    pub lecture_seule: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplicationAccesDto {
    pub agent_id: Uuid,
    pub eleve_id: Uuid,
    pub date: NaiveDate,
    /// Au moins une raison : l'agent voit l'élève.
    pub voit: bool,
    pub raisons: Vec<RaisonAccesDto>,
}

/// Filtres de `GET /personnels/missions`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FiltreMissions {
    #[serde(default)]
    pub type_code: Option<String>,
    #[serde(default)]
    pub agent_id: Option<Uuid>,
    /// Missions actives à cette date (défaut : aujourd'hui).
    #[serde(default)]
    pub le: Option<NaiveDate>,
    /// Inclut les missions terminées et annulées.
    #[serde(default)]
    pub avec_historique: bool,
}
