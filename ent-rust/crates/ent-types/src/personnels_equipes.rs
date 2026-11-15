//! Personnels (socle 6, lot P5) : responsables de groupes datés, équipes calculées et équipes constituées
//! (`docs/socles/06_PERSONNELS.md` §8.7, §8.8, §10.5, §10.6, PRS-66, PRS-70 à 75).
//! Routes `ent-api/src/personnels_equipes.rs`. Aucune logique métier.
//!
//! **Une équipe ne donne aucun droit** (Q-P9) : les DTO ne portent aucun droit, et aucun lecteur de droits
//! ne lit les tables d'équipes.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn nature_responsable() -> String {
    "responsable".to_string()
}

// ── PRS-66 : responsables de groupes et d'ateliers ─────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResponsableGroupeDto {
    pub id: Uuid,
    pub groupe_id: Uuid,
    pub agent_id: Uuid,
    pub agent_nom: Option<String>,
    /// `responsable`, `animateur`, `co_animateur`.
    pub nature: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    /// Active au jour de l'établissement (calculé à la lecture).
    pub active: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AjouterResponsableBody {
    pub agent_id: Uuid,
    #[serde(default = "nature_responsable")]
    pub nature: String,
    /// Début (défaut : aujourd'hui) ; peut être futur, jamais antérieur à hier.
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct TerminerResponsableBody {
    /// Dernier jour de la responsabilité (défaut : aujourd'hui).
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

// ── PRS-70 : équipes calculées ─────────────────────────────────────────────────────────────────

/// Pourquoi un agent figure dans une équipe calculée. `code` : `titulaire`, `co_enseignant`, `suppleant`,
/// `mission`, `responsable_groupe`, `discipline_poste`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RaisonEquipeDto {
    pub code: String,
    pub libelle: Option<String>,
    /// Service, mission ou responsabilité à l'origine.
    pub ref_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MembreCalculeDto {
    pub agent_id: Uuid,
    pub nom: Option<String>,
    pub fonction_code: Option<String>,
    pub raisons: Vec<RaisonEquipeDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EquipeCalculeeDto {
    /// `classe`, `groupe`, `niveau`, `discipline`, `eleve`.
    pub nature: String,
    pub cible_id: Option<Uuid>,
    pub discipline_code: Option<String>,
    pub jour: NaiveDate,
    pub membres: Vec<MembreCalculeDto>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct FiltreEquipeCalculee {
    pub nature: String,
    #[serde(default)]
    pub cible_id: Option<Uuid>,
    #[serde(default)]
    pub discipline_code: Option<String>,
    /// Date de calcul (défaut : aujourd'hui, au jour de l'établissement).
    #[serde(default)]
    pub date: Option<NaiveDate>,
}

// ── PRS-71 à 75 : équipes constituées ──────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TypeEquipeDto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    /// Le type exige un élève pour objet (équipe éducative).
    pub objet_eleve: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MembreEquipeDto {
    pub id: Uuid,
    pub agent_id: Option<Uuid>,
    pub personne_id: Option<Uuid>,
    pub nom: Option<String>,
    /// `coordinateur`, `membre`, `invite` (famille ou personne extérieure).
    pub role_dans_equipe: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub actif: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EquipeDto {
    pub id: Uuid,
    pub type_code: String,
    pub type_libelle: String,
    pub libelle: String,
    pub annee_id: Option<Uuid>,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub objet_eleve_id: Option<Uuid>,
    pub liste_diffusion: bool,
    pub visible_annuaire: bool,
    pub confidentielle: bool,
    /// `a_venir`, `active`, `terminee`, `close`.
    pub etat: String,
    /// Membres : toujours présents pour qui peut lire l'équipe (une équipe confidentielle n'est lisible
    /// que de ses membres et de la direction ; les autres ne la voient pas du tout).
    pub membres: Vec<MembreEquipeDto>,
    /// Aucun droit n'est ouvert par une équipe (Q-P9) : constant, exposé pour les clients.
    pub ouvre_des_droits: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct NouveauMembreBody {
    #[serde(default)]
    pub agent_id: Option<Uuid>,
    /// Famille ou personne invitée (équipe éducative) : rôle `invite` d'office.
    #[serde(default)]
    pub personne_id: Option<Uuid>,
    /// `coordinateur` ou `membre` (défaut) pour un agent.
    #[serde(default)]
    pub role_dans_equipe: Option<String>,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreerEquipeBody {
    pub type_code: String,
    pub libelle: String,
    #[serde(default)]
    pub annee_id: Option<Uuid>,
    #[serde(default)]
    pub du: Option<NaiveDate>,
    #[serde(default)]
    pub au: Option<NaiveDate>,
    /// Équipe éducative ou de suivi autour d'un élève (PRS-73) : confidentielle d'office.
    #[serde(default)]
    pub objet_eleve_id: Option<Uuid>,
    #[serde(default)]
    pub liste_diffusion: bool,
    #[serde(default)]
    pub visible_annuaire: bool,
    #[serde(default)]
    pub confidentielle: bool,
    #[serde(default)]
    pub membres: Vec<NouveauMembreBody>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct TerminerMembreBody {
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct ClorePeriodeBody {
    /// Dernier jour de l'équipe (défaut : aujourd'hui).
    #[serde(default)]
    pub au: Option<NaiveDate>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct FiltreEquipes {
    /// Équipes terminées ou closes comprises.
    #[serde(default)]
    pub avec_closes: bool,
}

/// Port `personnels::equipe_destinataires` (socle 16, PRS-74) : membres actifs à la date, SANS les invités
/// (familles) sauf demande. La liste est calculée au moment de l'envoi, jamais copiée.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DestinatairesEquipeDto {
    pub equipe_id: Uuid,
    pub jour: NaiveDate,
    pub agents: Vec<Uuid>,
    /// Comptes des agents (un agent sans compte n'a pas de destinataire).
    pub comptes: Vec<Uuid>,
    pub personnes_invitees: Vec<Uuid>,
}
