//! Cahier de textes, lot C9 : **progressions et reconduction** (`20_CAHIER_DE_TEXTES.md` §16).
//!
//! Contrat JSON des routes `/cahier/c9/*` d'`ent-api`. Une progression est le plan **privé** d'un professeur : étapes (titre,
//! notions, durée en séances). Elle ne contient aucune donnée d'élève ; son partage (avec un cours ou une discipline) n'ouvre
//! jamais le cahier d'un autre cours.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtapeBody {
    /// Absent pour une étape nouvelle ; présent pour conserver une étape déjà planifiée.
    pub id: Option<Uuid>,
    pub titre: String,
    #[serde(default)]
    pub notions: String,
    /// Durée en séances (1 à 30).
    pub duree_seances: i16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressionBody {
    pub titre: String,
    pub matiere_id: Option<Uuid>,
    pub niveau_id: Option<Uuid>,
    pub annee_id: Option<Uuid>,
    pub etapes: Vec<EtapeBody>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtapeDto {
    pub id: Uuid,
    pub ordre: i16,
    pub titre: String,
    pub notions: String,
    pub duree_seances: i16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartageDto {
    pub id: Uuid,
    pub cours_id: Option<Uuid>,
    pub cours_libelle: Option<String>,
    pub matiere_id: Option<Uuid>,
    pub matiere_libelle: Option<String>,
    pub depuis: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressionResumeDto {
    pub id: Uuid,
    pub titre: String,
    pub auteur: String,
    pub mienne: bool,
    pub nb_etapes: i32,
    pub nb_seances: i32,
    pub matiere_libelle: Option<String>,
    pub niveau_libelle: Option<String>,
    pub annee_id: Option<Uuid>,
    pub copie_de_id: Option<Uuid>,
    pub modifie_le: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressionsDto {
    pub mes: Vec<ProgressionResumeDto>,
    /// Progressions de collègues partagées avec un de mes cours ou avec ma discipline.
    pub partagees: Vec<ProgressionResumeDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressionDto {
    pub resume: ProgressionResumeDto,
    pub matiere_id: Option<Uuid>,
    pub niveau_id: Option<Uuid>,
    pub etapes: Vec<EtapeDto>,
    /// Partages en vigueur : lus de l'auteur seulement.
    pub partages: Vec<PartageDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartageBody {
    pub cours_id: Option<Uuid>,
    pub matiere_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanifierBody {
    pub cours_id: Uuid,
    /// Premier jour planifiable (aujourd'hui par défaut ; jamais avant aujourd'hui).
    pub a_partir_du: Option<NaiveDate>,
    /// Dernier jour planifiable (« répartir sur les séances jusqu'au… »).
    pub jusqu_au: Option<NaiveDate>,
    /// Aperçu sans écriture.
    #[serde(default)]
    pub apercu: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectationDto {
    pub seance_id: Uuid,
    pub jour: NaiveDate,
    pub etape_id: Uuid,
    pub etape_titre: String,
    /// Rang de la séance dans l'étape (1 pour la première).
    pub rang: i16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanDto {
    pub cours_id: Uuid,
    pub progression_id: Uuid,
    pub apercu: bool,
    pub affectations: Vec<AffectationDto>,
    /// Entrées créées en brouillon (0 pour un aperçu).
    pub creees: i64,
    /// Étapes déjà planifiées dans ce cours, laissées telles quelles.
    pub deja_planifiees: i64,
    /// Étapes (ou fins d'étape) sans séance libre dans la fenêtre.
    pub sans_seance: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconduireBody {
    pub annee_id: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopierCoursBody {
    pub source_cours_id: Uuid,
    pub cible_cours_id: Uuid,
    /// Premier jour de séance de destination (aujourd'hui par défaut).
    pub a_partir_du: Option<NaiveDate>,
    #[serde(default)]
    pub apercu: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopieEntreeDto {
    pub entree_source_id: Uuid,
    pub titre: Option<String>,
    pub seance_cible_id: Uuid,
    pub jour_cible: NaiveDate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopieCoursDto {
    pub apercu: bool,
    pub copiees: Vec<CopieEntreeDto>,
    /// Entrées de l'ancien cours sans séance libre de destination.
    pub sans_seance: i64,
    /// Entrées qui ne sont pas de l'appelant (jamais copiées : R4).
    pub ignorees_autrui: i64,
}
