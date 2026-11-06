//! Cahier de textes, travail à faire / à rendre, copies, agenda et charge (lot S2-6, D7).
//!
//! Contrat JSON des routes `/cahier/*` d'`ent-api`. Aucune clé de stockage n'apparaît
//! ici : les fichiers sont ouverts par un lien signé court émis après contrôle des droits.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Pièce jointe (métadonnées seules). `genre` : `ressource`, `seance`, `copie`, `annotation`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceCahierDto {
    pub id: Uuid,
    pub genre: String,
    pub nom: String,
    pub mime_type: String,
    pub taille_octets: i64,
    pub created_at: DateTime<Utc>,
}

/// Contenu d'une séance datée (CDT-01).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceContenuDto {
    pub id: Uuid,
    pub seance_id: Uuid,
    pub auteur_user_id: Uuid,
    pub contenu: String,
    pub ressources: String,
    pub version: i32,
    pub pieces: Vec<PieceCahierDto>,
    pub updated_at: DateTime<Utc>,
}

/// Séance datée telle que présentée dans le cahier du professeur.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceCahierDto {
    pub seance_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: String,
    pub heure_fin: String,
    pub classe_id: Option<Uuid>,
    pub classe: String,
    pub matiere_libelle: String,
    pub statut: String,
    pub contenu: Option<SeanceContenuDto>,
    pub nb_travaux: i64,
}

/// Travail à faire, à rendre en ligne, ou contrôle annoncé (CDT-02, CDT-05).
/// `type_travail` : `a_faire`, `a_rendre`, `controle`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravailDto {
    pub id: Uuid,
    pub seance_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    /// Libellé lisible de la cible (code de la classe ou du groupe).
    pub cible: String,
    pub matiere_libelle: String,
    pub auteur_user_id: Uuid,
    pub type_travail: String,
    pub titre: String,
    pub consigne: String,
    pub donne_le: NaiveDate,
    pub pour_le: NaiveDate,
    pub duree_estimee_min: Option<i32>,
    pub rendu_en_ligne: bool,
    pub date_limite: Option<DateTime<Utc>>,
    pub acceptation_retard: bool,
    pub formats_acceptes: Vec<String>,
    pub version: i32,
    pub pieces: Vec<PieceCahierDto>,
}

/// Remise d'un élève et retour du professeur (CDT-06, CDT-07).
/// `statut` : `rendu`, `corrige`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemiseDto {
    pub id: Uuid,
    pub travail_id: Uuid,
    pub eleve_id: Uuid,
    pub soumise_le: DateTime<Utc>,
    pub derniere_le: DateTime<Utc>,
    pub en_retard: bool,
    pub commentaire_eleve: String,
    pub statut: String,
    pub retour_commentaire: Option<String>,
    pub retour_le: Option<DateTime<Utc>>,
    pub note_id: Option<Uuid>,
    pub pieces: Vec<PieceCahierDto>,
}

/// Élément de l'agenda « à faire » (CDT-08).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgendaItemDto {
    pub travail: TravailDto,
    pub fait: bool,
    pub vu: bool,
    pub remise: Option<RemiseDto>,
}

/// Une ligne du suivi d'un travail à rendre : un élève de la cible et sa remise éventuelle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneSuiviDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub remise: Option<RemiseDto>,
}

/// Suivi d'un travail à rendre : remises reçues, élèves en attente.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiviTravailDto {
    pub travail: TravailDto,
    pub lignes: Vec<LigneSuiviDto>,
    pub nb_rendus: i64,
    pub nb_en_retard: i64,
    pub nb_attendus: i64,
}

/// Une charge de travail d'un jour pour une classe ou un groupe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeJourDto {
    pub jour: NaiveDate,
    pub minutes: i32,
    pub nb_travaux: i32,
    pub nb_controles: i32,
    /// Alerte de surcharge (minutes au-dessus du seuil, ou trop de contrôles le même jour).
    pub surcharge: bool,
    pub travaux: Vec<ChargeTravailDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeTravailDto {
    pub travail_id: Uuid,
    pub matiere_libelle: String,
    pub titre: String,
    pub type_travail: String,
    pub duree_estimee_min: Option<i32>,
}

/// Charge de travail d'une cible sur une période (équipe pédagogique).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeDto {
    pub seuil_minutes: i32,
    pub seuil_controles: i32,
    pub jours: Vec<ChargeJourDto>,
}

/// Visa du cahier (CDT-10).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisaDto {
    pub id: Uuid,
    pub classe_id: Uuid,
    pub prof_user_id: Option<Uuid>,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub commentaire: String,
    pub vise_par_user_id: Uuid,
    pub vise_le: DateTime<Utc>,
}

// ── Corps de requête ─────────────────────────────────────────────────────────────────────

/// Fichier déposé en base64 (même convention que les pièces de messagerie).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FichierDepotBody {
    pub nom: String,
    pub mime_type: String,
    pub contenu_base64: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContenuSeanceBody {
    pub contenu: String,
    #[serde(default)]
    pub ressources: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreerTravailBody {
    /// Séance d'origine (la cible en est déduite si `classe_id`/`groupe_id` sont absents).
    #[serde(default)]
    pub seance_id: Option<Uuid>,
    #[serde(default)]
    pub classe_id: Option<Uuid>,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    #[serde(default)]
    pub matiere_libelle: Option<String>,
    /// `a_faire` (défaut), `a_rendre`, `controle`.
    #[serde(default)]
    pub type_travail: Option<String>,
    pub titre: String,
    pub consigne: String,
    pub pour_le: NaiveDate,
    #[serde(default)]
    pub duree_estimee_min: Option<i32>,
    /// Pour un travail à rendre : date limite (par défaut, fin de la journée `pour_le`).
    #[serde(default)]
    pub date_limite: Option<DateTime<Utc>>,
    #[serde(default)]
    pub acceptation_retard: Option<bool>,
    /// Extensions acceptées (`pdf`, `docx`…) ; vide = tous les formats autorisés.
    #[serde(default)]
    pub formats_acceptes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ModifierTravailBody {
    #[serde(default)]
    pub titre: Option<String>,
    #[serde(default)]
    pub consigne: Option<String>,
    #[serde(default)]
    pub pour_le: Option<NaiveDate>,
    #[serde(default)]
    pub duree_estimee_min: Option<i32>,
    #[serde(default)]
    pub date_limite: Option<DateTime<Utc>>,
    #[serde(default)]
    pub acceptation_retard: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemiseBody {
    #[serde(default)]
    pub commentaire: String,
    #[serde(default)]
    pub fichier: Option<FichierDepotBody>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetourBody {
    pub commentaire: String,
    #[serde(default)]
    pub note_id: Option<Uuid>,
    #[serde(default)]
    pub fichier: Option<FichierDepotBody>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaitBody {
    pub fait: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisaBody {
    pub classe_id: Uuid,
    #[serde(default)]
    pub prof_user_id: Option<Uuid>,
    pub du: NaiveDate,
    pub au: NaiveDate,
    #[serde(default)]
    pub commentaire: String,
}

/// Lien court vers un fichier (réponse de `…/lien`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LienCahierDto {
    pub url: String,
    pub expires_at: i64,
}

/// Réponse à la création d'un travail : le travail et la charge du jour d'échéance
/// (alerte de surcharge pour l'équipe, calculée avec les seuils par défaut).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravailCreeDto {
    pub travail: TravailDto,
    pub charge: ChargeJourDto,
}
