//! Évaluations, lot V11 « alertes précoces », partie **déterministe** (socle 21, §13.3, §13.8, R-V20, R-V40, S-118).
//! Compile pour `wasm32` (aucune dépendance serveur).
//!
//! Principes :
//! * une alerte est une **règle écrite par des personnes**, jamais un modèle : chaque alerte dit **pourquoi** (les faits),
//!   jamais un « score de risque » ni une probabilité ;
//! * **aucune conséquence automatique** (R-V20, RGPD art. 22) : la suite est décidée et tracée par une personne ;
//! * **jamais visible** de la famille ni de l'élève ;
//! * réglage à **trois positions** (S-118), sans curseur.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Position du réglage des alertes (S-118), identique à la contrainte de `eva_reglage.seuils_alertes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PositionAlertes {
    /// Moins d'alertes : seuils plus larges.
    Souple,
    /// Les seuils de la conception (§13.3).
    #[default]
    Normal,
    /// Plus d'alertes : seuils plus serrés.
    Strict,
}

impl PositionAlertes {
    pub const TOUTES: [PositionAlertes; 3] = [Self::Souple, Self::Normal, Self::Strict];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Souple => "souple",
            Self::Normal => "normal",
            Self::Strict => "strict",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|p| p.as_str() == s.trim())
    }

    /// Seuils de la position, pour une échelle donnée (millièmes : `20000` pour une échelle sur 20). Les seuils de note
    /// sont proportionnels à l'échelle (8 sur 20 = 4 sur 10).
    pub fn seuils(self, echelle_m: i64) -> SeuilsAlertes {
        // Écart de baisse en vingtièmes d'échelle, nombre de matières sous le seuil, nombre d'absences et non-rendus.
        let (baisse_vingtiemes_x10, matieres, absences) = match self {
            Self::Souple => (30, 4, 4),
            Self::Normal => (20, 3, 3),
            Self::Strict => (15, 2, 2),
        };
        SeuilsAlertes {
            position: self,
            baisse_generale_m: echelle_m * baisse_vingtiemes_x10 / 200,
            note_basse_m: echelle_m * 8 / 20,
            matieres_sous_seuil: matieres,
            absences_evaluations: absences,
        }
    }
}

/// Seuils effectifs d'une position, en millièmes de point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeuilsAlertes {
    pub position: PositionAlertes,
    /// Baisse de la moyenne générale d'une période à la précédente (2 points sur 20 en position normale).
    pub baisse_generale_m: i64,
    /// Une moyenne de matière (ou de cours) **strictement** sous ce seuil est « basse » (8 sur 20).
    pub note_basse_m: i64,
    /// Nombre de matières basses qui déclenche l'alerte (3 en position normale).
    pub matieres_sous_seuil: usize,
    /// Nombre d'absences et de non-rendus aux évaluations de la période qui déclenche l'alerte (3, S-114).
    pub absences_evaluations: usize,
}

/// Réglage des alertes d'une année.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglageAlertesDto {
    pub annee_id: Uuid,
    pub seuils: SeuilsAlertes,
    /// Les seuils des trois positions, pour que la direction choisisse en connaissance de cause.
    pub positions: Vec<SeuilsAlertes>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinirReglageAlertesBody {
    pub annee_id: Uuid,
    pub position: PositionAlertes,
}

/// Règle qui a déclenché une alerte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RegleAlerte {
    /// Baisse de la moyenne générale depuis la période précédente.
    BaisseGenerale,
    /// Plusieurs matières sous le seuil.
    MatieresSousSeuil,
    /// Absences et non-rendus répétés aux évaluations.
    AbsencesEvaluations,
    /// Vue du professeur : moyenne du cours sous le seuil.
    CoursSousSeuil,
}

impl RegleAlerte {
    pub fn libelle(self) -> &'static str {
        match self {
            Self::BaisseGenerale => "Baisse de la moyenne générale",
            Self::MatieresSousSeuil => "Plusieurs matières sous le seuil",
            Self::AbsencesEvaluations => "Absences ou travaux non rendus aux évaluations",
            Self::CoursSousSeuil => "Moyenne du cours sous le seuil",
        }
    }
}

/// Une alerte : la règle et les faits qui l'expliquent. Jamais de score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlerteDto {
    pub regle: RegleAlerte,
    pub libelle: String,
    /// Phrases factuelles (« Moyenne générale : 13,40 puis 10,90, soit 2,50 points de moins »).
    pub faits: Vec<String>,
}

/// Moyenne d'une matière dans une vue d'alerte (millièmes d'affichage).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatiereAlerteDto {
    pub matiere_id: Option<Uuid>,
    pub libelle: String,
    pub moyenne_m: Option<i64>,
}

/// Dernière suite humaine posée pour l'élève et la période.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiteDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub periode_id: Uuid,
    pub nature: NatureSuite,
    pub commentaire: Option<String>,
    pub auteur_id: Uuid,
    pub le: DateTime<Utc>,
}

/// Un élève en alerte dans une vue de classe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveAlerteDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub moyenne_generale_m: Option<i64>,
    pub moyenne_generale_precedente_m: Option<i64>,
    pub alertes: Vec<AlerteDto>,
    pub derniere_suite: Option<SuiteDto>,
}

/// Alertes d'une classe pour une période (PP, CPE référent, direction). Les élèves sont rangés par nom : jamais de
/// classement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertesClasseDto {
    pub classe_id: Uuid,
    pub classe_libelle: String,
    pub periode_id: Uuid,
    pub periode_precedente_id: Option<Uuid>,
    pub seuils: SeuilsAlertes,
    /// Élèves de la classe pendant la période (ceux que l'acteur peut voir).
    pub effectif: usize,
    pub eleves: Vec<EleveAlerteDto>,
    /// Ce que le calcul n'a pas pu établir (moyenne indisponible, période précédente absente…).
    pub avertissements: Vec<String>,
    /// Rappel affiché avec la liste.
    pub mention: String,
}

/// Un élève en alerte dans la vue d'un cours (professeur : ses cours seulement).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveCoursAlerteDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub moyenne_cours_m: Option<i64>,
    pub alertes: Vec<AlerteDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertesCoursDto {
    pub cours_id: Uuid,
    pub cours_libelle: String,
    pub periode_id: Uuid,
    pub seuils: SeuilsAlertes,
    /// Élèves notés ou attendus dans le cours sur la période.
    pub effectif: usize,
    /// Nombre d'élèves dont la moyenne du cours est sous le seuil (« 4 élèves sous 8 dans ce groupe »).
    pub sous_seuil: usize,
    pub eleves: Vec<EleveCoursAlerteDto>,
    pub mention: String,
}

/// Évolution d'une matière entre la période précédente et la période.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionMatiereDto {
    pub matiere_id: Option<Uuid>,
    pub libelle: String,
    pub avant_m: Option<i64>,
    pub apres_m: Option<i64>,
    /// `apres - avant` quand les deux existent.
    pub ecart_m: Option<i64>,
}

/// Appréciation validée de la période précédente (générale ou de vie scolaire), rappelée au conseil.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppreciationRappelDto {
    pub type_appreciation: String,
    pub texte: String,
}

/// « Ce qui a changé depuis le dernier conseil » (EVA-276), déterministe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangementsEleveDto {
    pub eleve_id: Uuid,
    pub periode_id: Uuid,
    pub periode_precedente_id: Option<Uuid>,
    pub generale: EvolutionMatiereDto,
    pub matieres: Vec<EvolutionMatiereDto>,
    /// Alertes de la période qui n'existaient pas à la période précédente.
    pub alertes_nouvelles: Vec<AlerteDto>,
    /// Règles qui se déclenchaient à la période précédente et ne se déclenchent plus.
    pub alertes_levees: Vec<RegleAlerte>,
    pub alertes_maintenues: Vec<RegleAlerte>,
    pub appreciations_precedentes: Vec<AppreciationRappelDto>,
    pub suites: Vec<SuiteDto>,
    pub avertissements: Vec<String>,
}

/// Suite humaine donnée à une situation (EVA-154, EVA-277). Liste fermée, identique à la contrainte de la base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatureSuite {
    Vue,
    EntretienPrevu,
    EntretienFait,
    FamilleInformee,
    CelluleDeVeille,
    SansSuite,
}

impl NatureSuite {
    pub const TOUTES: [NatureSuite; 6] = [
        Self::Vue,
        Self::EntretienPrevu,
        Self::EntretienFait,
        Self::FamilleInformee,
        Self::CelluleDeVeille,
        Self::SansSuite,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vue => "vue",
            Self::EntretienPrevu => "entretien_prevu",
            Self::EntretienFait => "entretien_fait",
            Self::FamilleInformee => "famille_informee",
            Self::CelluleDeVeille => "cellule_de_veille",
            Self::SansSuite => "sans_suite",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|n| n.as_str() == s.trim())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoserSuiteBody {
    pub periode_id: Uuid,
    pub nature: NatureSuite,
    /// 1 à 500 caractères ; facultatif. Aucune donnée de santé, aucun motif d'absence.
    #[serde(default)]
    pub commentaire: Option<String>,
}

/// Une exclusion écrite de l'IA (R-V40), avec sa raison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExclusionIaDto {
    pub code: String,
    pub ce_que_l_ia_ne_fait_pas: String,
    pub raison: String,
}

/// Une consultation d'une vue d'alertes (journal, direction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsultationAlerteDto {
    pub id: Uuid,
    pub acteur_id: Uuid,
    pub vue: String,
    pub periode_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
    pub cours_id: Option<Uuid>,
    pub eleve_id: Option<Uuid>,
    pub le: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seuils_normaux_sur_vingt() {
        let s = PositionAlertes::Normal.seuils(20_000);
        assert_eq!(s.baisse_generale_m, 2_000);
        assert_eq!(s.note_basse_m, 8_000);
        assert_eq!(s.matieres_sous_seuil, 3);
        assert_eq!(s.absences_evaluations, 3);
    }

    #[test]
    fn seuils_proportionnels_a_l_echelle_et_ordonnes() {
        let s = PositionAlertes::Souple.seuils(10_000);
        assert_eq!(s.baisse_generale_m, 1_500);
        assert_eq!(s.note_basse_m, 4_000);
        let (so, no, st) = (
            PositionAlertes::Souple.seuils(20_000),
            PositionAlertes::Normal.seuils(20_000),
            PositionAlertes::Strict.seuils(20_000),
        );
        assert!(
            so.baisse_generale_m > no.baisse_generale_m
                && no.baisse_generale_m > st.baisse_generale_m
        );
        assert!(
            so.matieres_sous_seuil > no.matieres_sous_seuil
                && no.matieres_sous_seuil > st.matieres_sous_seuil
        );
        assert!(so.absences_evaluations > no.absences_evaluations);
    }

    #[test]
    fn listes_fermees_identiques_a_la_base() {
        for p in PositionAlertes::TOUTES {
            assert_eq!(PositionAlertes::parse(p.as_str()), Some(p));
        }
        for n in NatureSuite::TOUTES {
            assert_eq!(NatureSuite::parse(n.as_str()), Some(n));
            assert_eq!(
                serde_json::to_value(n).unwrap(),
                serde_json::Value::String(n.as_str().into())
            );
        }
        assert_eq!(PositionAlertes::parse("curseur"), None);
    }
}
