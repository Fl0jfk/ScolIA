//! Cahier de textes, lot C4 : la **charge de travail** (`20_CAHIER_DE_TEXTES.md` §11).
//!
//! Contrat JSON des routes `/cahier/c4/*` d'`ent-api`. La charge se calcule **par élève réel** (jamais par classe : un
//! groupe de spécialité mélange plusieurs classes) et ne sort que des **nombres et des natures** : jamais le nom d'un
//! élève, jamais le titre ni le contenu d'un travail d'un collègue, jamais une matière hors de la classe du cours
//! (§11.1). Les repères sont ceux de l'établissement, à trois positions ; aucun plafond national n'existe (§5) :
//! l'alerte se lit, on peut passer outre, la dérogation est comptée (§11.2), rien n'est jamais bloqué.
//! Jours de l'établissement partout.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const POSITION_SOUPLE: &str = "souple";
pub const POSITION_NORMAL: &str = "normal";
pub const POSITION_STRICT: &str = "strict";

pub const REPERE_MINUTES: &str = "minutes";
pub const REPERE_EVALUATIONS: &str = "evaluations";
pub const REPERE_SEMAINE: &str = "travaux";

/// Repères d'un groupe de niveaux pour une position (tableau du §11.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReperePositionDto {
    /// `6e-5e`, `4e-3e` ou `lycee`.
    pub groupe: String,
    pub libelle: String,
    /// Évaluations annoncées le même jour pour un élève : au-delà, l'alerte.
    pub evaluations_jour: i32,
    /// Durée cumulée pour un soir, en minutes.
    pub minutes_soir: i32,
    /// Évaluations annoncées sur la semaine.
    pub evaluations_semaine: i32,
}

/// Repères en vigueur : la position choisie par l'établissement et ses valeurs pour chaque groupe de niveaux.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReperesDto {
    pub position: String,
    pub groupes: Vec<ReperePositionDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionBody {
    pub position: String,
}

/// Travail que le professeur s'apprête à donner (bandeau « au moment de donner »).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApercuBody {
    pub cours_ids: Vec<Uuid>,
    /// Jour de l'établissement de l'échéance ; absent quand `prochaine_seance` est vrai.
    #[serde(default)]
    pub echeance_jour: Option<NaiveDate>,
    /// Échéance à la prochaine séance du (premier) cours, comme à la saisie du travail : le serveur la résout.
    #[serde(default)]
    pub prochaine_seance: bool,
    /// Code de la nature (`exercices`, `revision`, …) ; une nature d'évaluation compte comme évaluation annoncée.
    #[serde(default)]
    pub nature: Option<String>,
    #[serde(default)]
    pub duree_estimee_min: Option<i16>,
}

/// « n élèves ont la valeur v ».
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistributionDto {
    pub valeur: i32,
    pub nb_eleves: i32,
}

/// Autre échéance des élèves du cours ce jour-là : une nature et un nombre. La **matière** n'est donnée que pour les
/// échéances de la classe du cours (l'équipe de la classe lit déjà son cahier) ; jamais pour un groupe transversal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcheanceAutreDto {
    pub jour: NaiveDate,
    /// `evaluation` ou `travail`.
    pub genre: String,
    pub nature: String,
    pub matiere: Option<String>,
    pub nb_eleves: i32,
}

/// Un repère dépassé par le travail à donner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlerteChargeDto {
    /// `minutes`, `evaluations` ou `travaux` (évaluations de la semaine).
    pub repere: String,
    pub seuil: i32,
    /// Plus haute valeur atteinte parmi les élèves concernés.
    pub valeur_max: i32,
    pub nb_eleves: i32,
}

/// Date alternative calculée (échéance plus légère pour ces élèves).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateProposeeDto {
    pub jour: NaiveDate,
    pub nb_eleves_au_dessus: i32,
    pub evaluations_max: i32,
    pub minutes_soir_mediane: i32,
}

/// Charge des élèves du ou des cours au moment de fixer l'échéance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApercuChargeDto {
    pub echeance_jour: NaiveDate,
    /// Soir où le travail se fait : le dernier jour ouvré avant l'échéance.
    pub soir_jour: NaiveDate,
    pub nb_eleves: i32,
    pub position: String,
    /// Évaluations déjà annoncées l'échéance, par élève (sans le travail à donner).
    pub evaluations_ce_jour: Vec<DistributionDto>,
    /// Durée cumulée du soir, avec le travail à donner : médiane et maximum, en minutes.
    pub minutes_soir_mediane: i32,
    pub minutes_soir_max: i32,
    pub autres_echeances: Vec<EcheanceAutreDto>,
    pub alertes: Vec<AlerteChargeDto>,
    pub alerte: bool,
    pub dates_proposees: Vec<DateProposeeDto>,
}

/// Charge d'un élève un jour donné.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeJourEleveDto {
    pub jour: NaiveDate,
    pub nb_evaluations: i32,
    pub nb_travaux: i32,
    /// Durée estimée du soir **précédant** ce jour (le travail se fait la veille), travaux longs répartis (R-C32).
    pub minutes_soir_precedent: i32,
    /// Repères dépassés ce jour-là (`minutes`, `evaluations`).
    pub au_dessus: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeSemaineEleveDto {
    pub lundi: NaiveDate,
    pub nb_evaluations: i32,
    pub au_dessus: bool,
}

/// Charge d'un élève sur une période, pour lui-même ou ses responsables habilités.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeEleveDto {
    pub eleve_id: Uuid,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub position: String,
    pub repere: ReperePositionDto,
    pub jours: Vec<ChargeJourEleveDto>,
    pub semaines: Vec<ChargeSemaineEleveDto>,
}

/// Évaluation ou travail long d'un jour dans le calendrier de classe (matières : l'équipe lit déjà le cahier de la classe).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneClasseDto {
    pub genre: String,
    pub matiere: String,
    pub nature: String,
    pub titre: String,
    pub duree_estimee_min: Option<i16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JourClasseDto {
    pub jour: NaiveDate,
    pub lignes: Vec<LigneClasseDto>,
    /// Élèves de la classe au-dessus d'un repère ce jour-là (charge réelle, par élève, y compris leurs groupes
    /// transversaux : agrégée, sans nom).
    pub nb_eleves_au_dessus: i32,
    pub evaluations_max_par_eleve: i32,
    pub minutes_soir_precedent_mediane: i32,
}

/// Calendrier d'une classe (PP, direction, équipe) : par jour, avec matières ; pas de blocage, une discussion d'équipe.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendrierClasseDto {
    pub classe_id: Uuid,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub position: String,
    pub nb_eleves: i32,
    pub jours: Vec<JourClasseDto>,
}

/// Alerte passée outre par le professeur, relue à son propre compte.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonAlerteIgnoreeDto {
    pub travail_id: Uuid,
    pub titre: String,
    pub repere: String,
    pub valeur: i32,
    pub seuil: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertesParClasseDto {
    pub classe: String,
    pub nb: i32,
}

/// Alertes ignorées sur une période, **comptées** : agrégées par classe pour la direction et le PP, jamais nominatives ;
/// un professeur ne retrouve que les siennes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertesIgnoreesDto {
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub total: i32,
    pub par_classe: Vec<AlertesParClasseDto>,
    pub mes_alertes: Vec<MonAlerteIgnoreeDto>,
}
