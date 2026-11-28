//! Évaluations, lot V10 « pilotage et statistiques » (socle 21, §17.12 et §23 de `docs/socles/21_EVALUATIONS.md` ;
//! EVA-153, EVA-156, EVA-212 à EVA-218, EVA-279 ; règle R-V44). Compile pour `wasm32` (aucune dépendance serveur).
//!
//! Principes :
//! * **des statistiques, jamais un classement** (R-V44) : les lignes sont rangées par libellé, jamais par valeur ; aucune
//!   ligne ne porte de rang ;
//! * **la direction voit tout, le professeur ses groupes** : dans une évaluation commune, un professeur concerné voit le
//!   détail de ses groupes et la seule moyenne **anonyme** des autres, sous le seuil d'effectif ;
//! * **chaque consultation de statistiques est journalisée** (R-V44) ;
//! * **la charge et les retards de publication sont des faits sans note** : dates, nombres, titres.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Évaluations au plus dans une évaluation commune (un examen blanc de niveau : quelques dizaines de cours).
pub const MAX_EVALUATIONS_PAR_COMMUNE: usize = 60;
/// Durée maximale d'une fenêtre de charge, en jours (une période et demie).
pub const CHARGE_MAX_JOURS: i64 = 120;
/// Délai par défaut au-delà duquel une évaluation non publiée est signalée, en jours (D-069 : trois semaines de
/// cours moins les fins de semaine), quand l'établissement n'a pas fixé de délai maximal de publication (EVA-106).
pub const NON_PUBLIEE_JOURS_DEFAUT: i64 = 15;
/// Bornes du délai demandé pour les évaluations non publiées.
pub const NON_PUBLIEE_JOURS_MIN: i64 = 1;
pub const NON_PUBLIEE_JOURS_MAX: i64 = 120;
/// Journal des consultations : fenêtre maximale d'une lecture, en jours.
pub const JOURNAL_MAX_JOURS: i64 = 366;

/// Statistiques d'une série (millièmes de l'échelle de l'établissement : 20 000 = 20/20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsDto {
    pub effectif: i64,
    pub moyenne_m: i64,
    pub mediane_m: i64,
    pub min_m: i64,
    pub max_m: i64,
}

// ───────────────────────── Évaluations communes (EVA-156, EVA-214, EVA-216, CE-46) ─────────────────────────

/// Créer une évaluation commune (examen blanc, évaluation commune de niveau) et y rattacher des évaluations déjà
/// créées par les professeurs, une par cours.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreerCommuneBody {
    pub libelle: String,
    pub date_eval: NaiveDate,
    /// Barème commun, en millièmes de point (20 000 : sur 20).
    pub bareme_m: i64,
    /// Examen blanc : ouvre la comparaison avec le contrôle continu (EVA-216).
    #[serde(default)]
    pub examen_blanc: bool,
    #[serde(default)]
    pub evaluations: Vec<Uuid>,
}

/// Rattacher ou détacher des évaluations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationsBody {
    pub evaluations: Vec<Uuid>,
}

/// Une évaluation rattachée à une évaluation commune.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommuneEvaluationDto {
    pub evaluation_id: Uuid,
    pub cours_id: Uuid,
    pub cours_libelle: String,
    pub titre: String,
    pub publiee: bool,
    /// Le cours est l'un de ceux de l'acteur (professeur concerné).
    pub mon_cours: bool,
}

/// Une évaluation commune.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommuneDto {
    pub id: Uuid,
    pub libelle: String,
    pub date_eval: NaiveDate,
    pub bareme_m: i64,
    pub examen_blanc: bool,
    pub cree_le: DateTime<Utc>,
    pub evaluations: Vec<CommuneEvaluationDto>,
}

/// Les statistiques d'un groupe (un cours) dans une évaluation commune.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupeCommuneDto {
    pub evaluation_id: Uuid,
    pub cours_id: Uuid,
    pub cours_libelle: String,
    /// `None` : aucun élève noté.
    pub stats: Option<StatsDto>,
    /// Résultats saisis qui ne comptent pas (absent, dispensé, non noté…).
    pub non_comptes: i64,
}

/// Ce qu'un professeur voit des groupes qui ne sont pas les siens : une moyenne anonyme, sous le seuil d'effectif.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutresGroupesDto {
    pub groupes: i64,
    pub effectif: i64,
    /// `None` sous le seuil d'effectif (un groupe trop petit se reconnaîtrait).
    pub moyenne_m: Option<i64>,
}

/// Comparaison des résultats d'une évaluation commune entre groupes (EVA-156, EVA-214, R-V44).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparaisonCommuneDto {
    pub commune_id: Uuid,
    pub libelle: String,
    pub date_eval: NaiveDate,
    pub bareme_m: i64,
    pub examen_blanc: bool,
    /// `direction` (tous les groupes) ou `professeur` (ses groupes et la moyenne anonyme des autres).
    pub vue: String,
    /// Toute l'évaluation commune (réunion des séries). `None` pour un professeur : il n'a que ses groupes et les autres.
    pub ensemble: Option<StatsDto>,
    /// Rangés par libellé de cours, jamais par valeur.
    pub groupes: Vec<GroupeCommuneDto>,
    /// Vue professeur seulement.
    pub autres_groupes: Option<AutresGroupesDto>,
    pub avertissements: Vec<String>,
}

/// Écart entre l'examen blanc et le contrôle continu d'un groupe (EVA-216).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcartGroupeDto {
    pub evaluation_id: Uuid,
    pub cours_id: Uuid,
    pub cours_libelle: String,
    /// Élèves notés à l'examen ET qui ont une moyenne de contrôle continu dans le cours.
    pub effectif: i64,
    pub moyenne_examen_m: Option<i64>,
    pub moyenne_controle_continu_m: Option<i64>,
    /// Examen − contrôle continu, millièmes (négatif : l'examen est plus bas).
    pub ecart_m: Option<i64>,
}

/// Écarts entre contrôle continu et examen blanc, groupe par groupe (EVA-216).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcartsExamenDto {
    pub commune_id: Uuid,
    pub libelle: String,
    pub date_eval: NaiveDate,
    pub groupes: Vec<EcartGroupeDto>,
    pub ensemble: EcartGroupeDto,
    pub avertissements: Vec<String>,
}

// ───────────────────────── Statistiques de période (EVA-213, EVA-215, EVA-279) ─────────────────────────

/// Axe des statistiques de période.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxeStatistiques {
    Matiere,
    Classe,
    Niveau,
    Professeur,
    /// Imposé au professeur : ses propres cours.
    Cours,
}

impl AxeStatistiques {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Matiere => "matiere",
            Self::Classe => "classe",
            Self::Niveau => "niveau",
            Self::Professeur => "professeur",
            Self::Cours => "cours",
        }
    }
}

/// Une ligne de statistiques : la série est celle des **moyennes de cours des élèves** (une valeur par élève et par
/// cours) qui relèvent de la clé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneStatsDto {
    /// Matière, classe, niveau, professeur (identifiant du compte) ou cours ; `None` : « sans matière », « sans niveau ».
    pub cle: Option<Uuid>,
    pub libelle: String,
    pub cours: i64,
    pub evaluations: i64,
    pub stats: Option<StatsDto>,
}

/// Statistiques d'une période selon un axe (EVA-213).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatistiquesPeriodeDto {
    pub periode_id: Uuid,
    pub axe: AxeStatistiques,
    /// Rangées par libellé, jamais par valeur (R-V44).
    pub lignes: Vec<LigneStatsDto>,
    pub avertissements: Vec<String>,
}

/// Répartition par nature d'évaluation (EVA-215) : aide au projet d'évaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneNatureDto {
    pub nature_id: Option<Uuid>,
    /// « Sans nature » quand l'évaluation n'en porte pas.
    pub libelle: String,
    pub evaluations: i64,
    pub notes: i64,
    /// Somme des coefficients des évaluations de cette nature, millièmes.
    pub coefficients_m: i64,
    /// Part de ces coefficients dans le total de la période, en millièmes (1 000 = tout).
    pub part_coefficients_m: i64,
    /// Série des moyennes **par évaluation** (une valeur par évaluation notée).
    pub moyennes_evaluations: Option<StatsDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepartitionNaturesDto {
    pub periode_id: Uuid,
    pub lignes: Vec<LigneNatureDto>,
    pub avertissements: Vec<String>,
}

/// Une classe d'un niveau, pour une matière (EVA-279).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClasseHarmonisationDto {
    pub classe_id: Uuid,
    pub classe_libelle: String,
    pub stats: Option<StatsDto>,
    /// Moyenne de la classe − moyenne du niveau, millièmes.
    pub ecart_m: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatiereHarmonisationDto {
    pub matiere_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub niveau: Option<StatsDto>,
    /// Rangées par libellé de classe. Aucun nom de professeur (R-V44).
    pub classes: Vec<ClasseHarmonisationDto>,
}

/// Harmonisation entre les classes d'un niveau (EVA-279).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarmonisationDto {
    pub periode_id: Uuid,
    pub niveau_id: Uuid,
    pub niveau_libelle: String,
    pub matieres: Vec<MatiereHarmonisationDto>,
    pub avertissements: Vec<String>,
}

// ───────────────────────── Charge et retards (EVA-153, EVA-217, EVA-218) ─────────────────────────

/// Une évaluation du calendrier d'une classe : sans note ni statistique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationChargeDto {
    pub evaluation_id: Uuid,
    pub date_eval: NaiveDate,
    pub cours_id: Uuid,
    pub cours_libelle: String,
    pub titre: String,
    pub annoncee: bool,
    pub commune: bool,
}

/// Une semaine (du lundi au dimanche).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemaineChargeDto {
    pub lundi: NaiveDate,
    pub evaluations: i64,
    /// Jours où la classe a deux évaluations ou plus.
    pub jours_charges: Vec<NaiveDate>,
}

/// Charge d'évaluations d'une classe, semaine par semaine (EVA-153, EVA-217).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeClasseDto {
    pub classe_id: Uuid,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub semaines: Vec<SemaineChargeDto>,
    pub evaluations: Vec<EvaluationChargeDto>,
}

/// Une évaluation non publiée depuis longtemps (EVA-218).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonPublieeDto {
    pub evaluation_id: Uuid,
    pub date_eval: NaiveDate,
    pub jours: i64,
    pub cours_id: Uuid,
    pub cours_libelle: String,
    pub titre: String,
    pub resultats_saisis: i64,
    pub en_cours_de_correction: bool,
    pub publication_programmee: bool,
    /// Comptes des professeurs du cours (à relancer) ; le nom est résolu par l'écran.
    pub professeurs: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonPublieesDto {
    pub jour: NaiveDate,
    pub delai_jours: i64,
    /// Rangées de la plus ancienne à la plus récente.
    pub evaluations: Vec<NonPublieeDto>,
}

// ───────────────────────── Journal des consultations (R-V44) ─────────────────────────

/// Une consultation de statistiques.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsultationDto {
    pub id: Uuid,
    pub acteur_id: Uuid,
    pub vue: String,
    pub periode_id: Option<Uuid>,
    pub commune_id: Option<Uuid>,
    pub niveau_id: Option<Uuid>,
    pub axe: Option<String>,
    pub le: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axe_en_snake_case() {
        assert_eq!(serde_json::to_string(&AxeStatistiques::Professeur).unwrap(), "\"professeur\"");
        let a: AxeStatistiques = serde_json::from_str("\"matiere\"").unwrap();
        assert_eq!(a, AxeStatistiques::Matiere);
        for a in [
            AxeStatistiques::Matiere,
            AxeStatistiques::Classe,
            AxeStatistiques::Niveau,
            AxeStatistiques::Professeur,
            AxeStatistiques::Cours,
        ] {
            assert_eq!(serde_json::to_string(&a).unwrap(), format!("\"{}\"", a.as_str()));
        }
    }

    #[test]
    fn creer_commune_sans_evaluations_par_defaut() {
        let b: CreerCommuneBody =
            serde_json::from_str(r#"{"libelle":"Bac blanc","date_eval":"2032-02-10","bareme_m":20000}"#).unwrap();
        assert!(b.evaluations.is_empty());
        assert!(!b.examen_blanc);
    }
}
