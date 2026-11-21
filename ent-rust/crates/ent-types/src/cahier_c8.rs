//! Cahier de textes, lot C8 : **tenue, visa, inspection, exports figés, clôture et archive** (`20_CAHIER_DE_TEXTES.md` §14).
//!
//! Contrat JSON des routes `/cahier/c8/*` d'`ent-api`. Aucune donnée d'élève n'y figure : ni nom, ni remise, ni retour, ni
//! variante (R-C18, §19.4). Les noms de professeurs sont des noms d'affichage. Jours et heures de l'établissement.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const VISA_COURS: &str = "cours";
pub const VISA_CLASSE: &str = "classe";
pub const VISA_AGENT: &str = "agent";

pub const EXPORT_LITIGE: &str = "litige";
pub const EXPORT_INSPECTION: &str = "inspection";
pub const EXPORT_AUTRE: &str = "autre";

pub const ACCES_ACTIF: &str = "actif";
pub const ACCES_A_VENIR: &str = "a_venir";
pub const ACCES_EXPIRE: &str = "expire";
pub const ACCES_REVOQUE: &str = "revoque";

/// Durée par défaut d'un accès d'inspection (jours) et plafond, prolongations comprises.
pub const ACCES_DUREE_DEFAUT_JOURS: i32 = 30;
pub const ACCES_DUREE_MAX_JOURS: i32 = 90;
/// Un cours est « à viser » quand son dernier visa a plus de ce nombre de semaines (ou n'existe pas).
pub const SEUIL_SEMAINES_VISA: i32 = 6;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoursRefDto {
    pub cours_id: Uuid,
    pub libelle: String,
    /// Code de la classe ou du groupe.
    pub public: String,
}

// ───────────────────────────── tableau de tenue ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenueLigneDto {
    pub cours_id: Uuid,
    pub libelle: String,
    pub public: String,
    pub intervenants: Vec<String>,
    /// Séances passées (non annulées) de la période.
    pub seances_passees: i64,
    /// Séances passées portant une entrée publiée.
    pub seances_renseignees: i64,
    /// Taux de tenue en pourcentage ; absent s'il n'y a eu aucune séance.
    pub taux_pct: Option<i32>,
    pub travaux_donnes: i64,
    pub derniere_entree: Option<NaiveDate>,
    pub dernier_visa_le: Option<DateTime<Utc>>,
    /// Éléments modifiés depuis le dernier visa du cours.
    pub modifies_apres_visa: i64,
    /// Aucun visa, ou visa plus ancien que le seuil.
    pub a_viser: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenueDto {
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub seuil_semaines: i32,
    pub lignes: Vec<TenueLigneDto>,
}

// ───────────────────────────── visa ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisaCahierBody {
    /// `cours`, `classe` ou `agent`.
    pub cible: String,
    pub cours_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub du: NaiveDate,
    pub au: NaiveDate,
    /// Observation adressée aux intervenants du cours ou au professeur (cible `cours` ou `agent` seulement).
    pub observation: Option<String>,
    /// Confirmer le visa d'un cahier vide (aucune entrée ni travail publié sur la période).
    #[serde(default)]
    pub confirmer_vide: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReponseDto {
    pub id: Uuid,
    pub texte: String,
    pub par: String,
    pub le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationDto {
    pub id: Uuid,
    pub texte: String,
    pub par: String,
    pub le: DateTime<Utc>,
    pub reponses: Vec<ReponseDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisaCahierDto {
    pub id: Uuid,
    pub cible: String,
    pub libelle_cible: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub vise_le: DateTime<Utc>,
    pub vise_par: String,
    /// Éléments figés par le visa (entrées et travaux publiés de la période).
    pub nb_elements: i32,
    /// Visa d'un cahier vide (aucun élément) : constaté et confirmé par la direction.
    pub vide: bool,
    pub modifies_apres_visa: i64,
    pub observations: Vec<ObservationDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModificationDto {
    pub version: i32,
    pub etat: String,
    pub par: String,
    pub le: DateTime<Utc>,
    pub motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisaElementDto {
    /// `entree` ou `travail`.
    pub objet: String,
    pub objet_id: Uuid,
    pub cours_id: Uuid,
    pub cours_libelle: String,
    pub jour: Option<NaiveDate>,
    pub titre: Option<String>,
    pub version_visee: i32,
    pub version_actuelle: i32,
    pub etat_actuel: String,
    pub modifie_apres_visa: bool,
    /// Texte tel que le viseur l'a vu et texte actuel : seulement quand l'élément a été modifié depuis.
    pub contenu_vise: Option<String>,
    pub contenu_actuel: Option<String>,
    pub modifications: Vec<ModificationDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisaDetailDto {
    pub visa: VisaCahierDto,
    pub elements: Vec<VisaElementDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationBody {
    pub texte: String,
}

// ───────────────────────────── chronologie du cours (écran 3) ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChronoItemDto {
    /// `entree` ou `travail`.
    pub objet: String,
    pub objet_id: Uuid,
    pub jour: NaiveDate,
    pub titre: Option<String>,
    pub contenu: String,
    pub etat: String,
    pub version: i32,
    pub auteur: String,
    pub modifie_le: DateTime<Utc>,
    /// Modifié depuis le dernier visa qui couvrait cet élément.
    pub modifie_apres_visa: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChronologieDto {
    pub cours: CoursRefDto,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub items: Vec<ChronoItemDto>,
    /// Visas du cours : date, viseur, période ; les observations ne sont servies qu'à ceux qui y ont droit.
    pub visas: Vec<VisaCahierDto>,
}

// ───────────────────────────── accès d'inspection ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccesInspectionBody {
    /// Compte de l'inspecteur (compte invité de l'établissement).
    pub beneficiaire_user_id: Uuid,
    /// Cours ouverts ; ou, à défaut, tous les cours du professeur `agent_id` sur la période.
    #[serde(default)]
    pub cours_ids: Vec<Uuid>,
    pub agent_id: Option<Uuid>,
    pub du_cahier: NaiveDate,
    pub au_cahier: NaiveDate,
    /// Durée de l'accès en jours (30 par défaut, 90 au plus).
    pub duree_jours: Option<i32>,
    /// `rdv_carriere`, `inspection` ou `autre`.
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProlongerBody {
    pub jours: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccesInspectionDto {
    pub id: Uuid,
    pub beneficiaire: String,
    pub motif: String,
    pub agent_cible: Option<String>,
    pub cours: Vec<CoursRefDto>,
    pub du_cahier: NaiveDate,
    pub au_cahier: NaiveDate,
    pub valable_du: DateTime<Utc>,
    pub valable_au: DateTime<Utc>,
    /// `actif`, `a_venir`, `expire` ou `revoque`.
    pub etat: String,
    pub accorde_par: String,
    pub revoque_le: Option<DateTime<Utc>>,
    /// Lectures déjà faites (comptage ; le détail est au journal).
    pub nb_consultations: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntreeInspectionDto {
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub titre: Option<String>,
    pub contenu: String,
    pub notions: Vec<String>,
    pub version: i32,
    pub auteur: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TravailInspectionDto {
    pub donne_le: NaiveDate,
    pub echeance_jour: NaiveDate,
    pub titre: String,
    pub consigne: String,
    pub nature: String,
    pub duree_estimee_min: Option<i16>,
    pub version: i32,
    pub auteur: String,
}

/// Ce que lit l'inspecteur : entrées et consignes publiées, jamais de remise, de retour, de variante ni de nom d'élève.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CahierInspectionDto {
    pub acces_id: Uuid,
    pub cours: CoursRefDto,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub valable_au: DateTime<Utc>,
    pub entrees: Vec<EntreeInspectionDto>,
    pub travaux: Vec<TravailInspectionDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsultationDto {
    pub le: DateTime<Utc>,
    pub qui: String,
    pub cours: Option<String>,
}

// ───────────────────────────── exports figés ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportBody {
    /// `litige`, `inspection` ou `autre`.
    pub nature: String,
    pub cours_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub motif: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportDto {
    pub id: Uuid,
    pub nature: String,
    pub titre: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub nb_entrees: i32,
    pub nb_travaux: i32,
    /// SHA-256 du contenu figé, recalculée et contrôlée par la base.
    pub empreinte: String,
    pub produit_le: DateTime<Utc>,
    pub produit_par: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportContenuDto {
    pub export: ExportDto,
    pub contenu: String,
}

// ───────────────────────────── clôture et archive ─────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControleClotureDto {
    pub annee_id: Uuid,
    pub annee_libelle: String,
    /// Cahier déjà figé (clôture du cahier, ou année close au socle 5).
    pub deja_fige: bool,
    pub cours_total: i64,
    /// Cours ayant du contenu publié sans visa couvrant la fin de l'année.
    pub cours_non_vises: Vec<CoursRefDto>,
    pub entrees_brouillon: i64,
    pub travaux_brouillon: i64,
    /// Constats « non rendu » en cours (nombre seulement) : l'élève garde ses dépôts après la clôture.
    pub non_rendus: i64,
    pub archives_existantes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClotureBody {
    pub annee_id: Uuid,
    /// Clôturer malgré les points du contrôle (cours non visés, brouillons, non-rendus).
    #[serde(default)]
    pub confirmer: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClotureDto {
    pub annee_id: Uuid,
    pub archives_produites: i64,
    pub fige: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReouvrirBody {
    pub annee_id: Uuid,
    /// Motif écrit (10 caractères au moins) : la réouverture est une procédure tracée de la direction.
    pub motif: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveDto {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub cours: CoursRefDto,
    pub produite_le: DateTime<Utc>,
    pub empreinte: String,
    pub version: i32,
    pub echeance_archive: NaiveDate,
    /// `a_decider`, `versee` ou `detruite`.
    pub decision: String,
    pub decide_le: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveDecisionBody {
    /// `versee` ou `detruite`.
    pub decision: String,
}
