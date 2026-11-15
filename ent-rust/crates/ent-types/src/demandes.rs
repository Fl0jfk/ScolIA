//! Socle N1, lot N1-4 : types d'**échange** de l'API des demandes (`/demandes…`).
//!
//! Aucune adresse, aucun montant : les HSE se comptent en **quarts d'heure** (D-061) ; le demandeur ne reçoit jamais
//! la file, le traitant ni une note interne (R-DM18). Les identifiants et les versions viennent du serveur.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Une démarche proposée par le guichet (« Écrire à l'établissement »).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatureGuichet {
    pub code: String,
    pub libelle: String,
    pub mots_cles: Vec<String>,
    pub eleve_requis: bool,
    /// « le secrétariat des élèves » : ce que le demandeur lit sur son accusé.
    pub role_traitant: Option<String>,
    pub message_accueil: Option<String>,
    pub delai_traitement_jours: u16,
    /// Démarche à formulaire dédié (HSE, copies) : le guichet y renvoie au lieu du formulaire générique.
    pub formulaire: Option<String>,
}

/// Une démarche qui se fait chez une autre brique (lien profond interne, jamais une demande).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaccourciGuichet {
    pub code: String,
    pub libelle: String,
    pub lien: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Guichet {
    pub natures: Vec<NatureGuichet>,
    pub raccourcis: Vec<RaccourciGuichet>,
    /// « Je veux signaler une situation grave » : jamais une demande, une orientation.
    pub signalement_grave: String,
    pub numeros_urgence: Vec<String>,
}

/// Valeur typée d'un champ (même forme JSON que le noyau).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "valeur", rename_all = "snake_case")]
pub enum ValeurDepot {
    Texte(String),
    Entier(i64),
    Decimal(String),
    Date(NaiveDate),
    Heure(NaiveTime),
    Bool(bool),
    Code(String),
    Ref(Uuid),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChampDepot {
    pub champ: String,
    #[serde(default)]
    pub rang: u16,
    pub valeur: ValeurDepot,
}

/// Corps d'un dépôt : **aucun champ d'identité** (le demandeur est la personne de la session).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepotDemandeDto {
    pub nature_code: String,
    pub objet: String,
    #[serde(default)]
    pub texte: Option<String>,
    #[serde(default)]
    pub valeurs: Vec<ChampDepot>,
    #[serde(default)]
    pub eleve_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtatDemandeDto {
    pub id: Uuid,
    pub numero: String,
    pub statut: String,
    pub version: i32,
}

/// Ce que voit le **demandeur** (projection séparée, jamais la file ni le traitant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaDemande {
    pub id: Uuid,
    pub numero: String,
    pub nature_code: String,
    pub statut: String,
    pub issue: Option<String>,
    pub objet: Option<String>,
    pub depose_le: Option<DateTime<Utc>>,
    pub clos_le: Option<DateTime<Utc>>,
    pub version: i32,
}

/// Ce que voit le **personnel** dans « Ma file » : l'exploitation, jamais la coordonnée du demandeur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemandeFile {
    pub id: Uuid,
    pub numero: String,
    pub nature_code: String,
    pub statut: String,
    pub confidentialite: String,
    pub urgence: String,
    pub objet: Option<String>,
    pub depose_le: Option<DateTime<Utc>>,
    /// `true` : prise en charge (par quelqu'un).
    pub prise: bool,
    pub version: i32,
}

/// Geste simple sur une demande (la version évite d'écraser un geste concurrent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GesteDto {
    pub version: i32,
}

// ───────────────────────── HSE ─────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceDto {
    pub jour: NaiveDate,
    #[serde(default)]
    pub debut: Option<NaiveTime>,
    pub duree_quarts: i32,
    pub fait: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepotHseDto {
    pub motif: String,
    pub objet: String,
    #[serde(default)]
    pub precisions: Option<String>,
    pub volume_quarts: i32,
    pub periode_du: NaiveDate,
    pub periode_au: NaiveDate,
    #[serde(default)]
    pub lieu: Option<String>,
    #[serde(default)]
    pub seances: Vec<SeanceDto>,
    /// Déposer pour le compte d'un collègue (accueil, secrétariat, comptabilité, direction) : il confirmera.
    #[serde(default)]
    pub pour_le_compte_de: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneHseDto {
    pub demande_id: Uuid,
    pub numero: String,
    pub statut: String,
    pub motif: String,
    pub volume_demande_quarts: i32,
    pub volume_accorde_quarts: Option<i32>,
    pub fait_quarts: i64,
    pub transmis_quarts: i64,
    pub decision: Option<String>,
    pub periode_au: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionHseDto {
    /// `accordee`, `accordee_partiellement` ou `refusee`.
    pub decision: String,
    #[serde(default)]
    pub volume_quarts: Option<i32>,
    #[serde(default)]
    pub motif: Option<String>,
    #[serde(default)]
    pub confirmer_depassement: bool,
    #[serde(default)]
    pub motif_depassement: Option<String>,
    pub version: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatDecisionHseDto {
    pub decision: String,
    pub volume_accorde_quarts: i32,
    pub seances_transmises: u32,
    pub depassement_enveloppe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnveloppeHseDto {
    pub saisie: bool,
    pub total_quarts: i64,
    pub engage_quarts: i64,
    pub fait_quarts: i64,
    pub restant_quarts: i64,
}

// ───────────────────────── Reprographie ─────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneCopiesDto {
    pub demande_id: Uuid,
    pub numero: String,
    pub site_id: Uuid,
    pub etat: String,
    pub format: String,
    pub couleur: bool,
    pub recto_verso: bool,
    pub agrafage: bool,
    pub copies_total: i32,
    pub exemplaires_total: i32,
    pub pour_le: NaiveDate,
    pub avertissement_delai: bool,
    pub demandeur_nom: Option<String>,
    pub retrait_lieu: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCopiesDto {
    pub accepter: bool,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreteDto {
    #[serde(default)]
    pub lieu_retrait: Option<Uuid>,
}

/// Erreur d'API des demandes : une phrase fixe, sans identifiant ni donnée (NJ-93).
pub const MESSAGE_INTROUVABLE: &str = "Demande introuvable.";
