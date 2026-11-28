//! Évaluations, lot V8 « fin de période » (socle 21, §9 et §17.8 à §17.9 de `docs/socles/21_EVALUATIONS.md`). Compile
//! pour `wasm32` (aucune dépendance serveur).
//!
//! Principes :
//! * **l'arrêt des notes publie tout** (R-V9) : relevé de la famille et bulletin comptent les mêmes évaluations ; un
//!   aperçu dit avant le geste ce qui sera publié et ce qui reste vide ;
//! * **l'instantané d'arrêt est versionné et porte une empreinte** (R-V11) : une correction, un rattrapage ou une
//!   réouverture en produisent une nouvelle version, jamais une réécriture ; l'écart entre deux versions se lit ;
//! * **après l'arrêt, le professeur demande, la direction décide** (§9.3) en voyant l'effet sur les moyennes ; une
//!   décision du conseil ne change jamais seule (CE-58) ;
//! * **une réouverture est motivée, bornée dans le temps et journalisée** (EVA-185) ;
//! * **les rappels sont des faits** : nombres de ce qui manque, jamais un nom d'élève.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Longueur minimale du motif d'une réouverture (comme une réouverture de période close, 463).
pub const MOTIF_REOUVERTURE_MIN: usize = 10;
/// Durée maximale d'une réouverture, en jours (D-069 : un mois scolaire).
pub const REOUVERTURE_MAX_JOURS: i64 = 30;
/// Classes au plus par geste d'arrêt, de clôture ou de réouverture (coût borné).
pub const MAX_CLASSES_PAR_GESTE: usize = 60;
/// Seuils (millièmes sur 20) dont le franchissement par une correction est signalé à la direction (CE-58).
pub const SEUILS_SIGNALES_M: [i64; 5] = [8_000, 10_000, 12_000, 14_000, 16_000];
/// Jours avant la date d'arrêt où partent les rappels automatiques (§9.1).
pub const RAPPEL_J7: i64 = 7;
pub const RAPPEL_J2: i64 = 2;

/// Une évaluation que l'arrêt publiera (ou qui a des cases vides).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationArretDto {
    pub evaluation_id: Uuid,
    pub cours_id: Uuid,
    pub titre: String,
    pub date_eval: NaiveDate,
    /// Non encore publiée : l'arrêt la publiera.
    pub a_publier: bool,
    /// Retirée pour vérification par son professeur : l'arrêt la publiera quand même (R-V9).
    pub retiree: bool,
    /// Marquée « en cours de correction » : l'arrêt la publiera quand même (R-V9).
    pub en_cours_de_correction: bool,
    /// Élèves de la classe concernés par l'évaluation et sans résultat.
    pub cases_vides: i64,
    pub concernes: i64,
}

/// Ce que l'arrêt d'une classe ferait, avant le geste (EVA-180 : « aperçu : publications automatiques, cases vides »).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApercuArretDto {
    pub periode_id: Uuid,
    pub classe_id: Uuid,
    pub etat: String,
    /// Date d'arrêt effective si la classe est arrêtée, sinon celle de la période.
    pub date_arret: Option<NaiveDate>,
    pub eleves: i64,
    pub evaluations: Vec<EvaluationArretDto>,
    pub a_publier: i64,
    pub cases_vides: i64,
    /// Phrases d'avertissement (évaluations retirées ou en cours de correction qui seront publiées…).
    pub avertissements: Vec<String>,
}

/// Arrêter les notes d'une ou plusieurs classes pour une période (EVA-180).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArreterBody {
    pub classes: Vec<Uuid>,
}

/// Fixer une date d'arrêt propre à des classes encore ouvertes (EVA-182, CE-41) ; l'arrêt automatique la suit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateArretBody {
    pub classes: Vec<Uuid>,
    pub date_arret: NaiveDate,
}

/// Clore les appréciations d'une ou plusieurs classes (EVA-184).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassesBody {
    pub classes: Vec<Uuid>,
}

/// Issue d'un geste sur une classe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GesteClasseDto {
    pub classe_id: Uuid,
    pub etat: String,
    /// Évaluations publiées par l'arrêt.
    pub publiees: i64,
    /// Version de l'instantané après le geste (`None` : aucun instantané pris).
    pub instantane_version: Option<i32>,
    pub empreinte: Option<String>,
}

/// Rouvrir une période pour des classes (EVA-185 ★) : motif, durée, classes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouvrirBody {
    pub classes: Vec<Uuid>,
    pub motif: String,
    /// Dernier jour (inclus) de la réouverture ; au lendemain, la classe est de nouveau arrêtée.
    pub jusqu_au: NaiveDate,
    /// `ouverte` (saisie des notes) ou `notes_arretees` (appréciations seulement) ; défaut : `ouverte`.
    #[serde(default)]
    pub vers: Option<String>,
}

/// Une réouverture du journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReouvertureDto {
    pub id: Uuid,
    pub lot_id: Uuid,
    pub classe_id: Uuid,
    pub etat_avant: String,
    pub etat_apres: String,
    pub motif: String,
    pub jusqu_au: NaiveDate,
    pub le: DateTime<Utc>,
    pub refermee_le: Option<DateTime<Utc>>,
    pub refermee_origine: Option<String>,
}

/// Une version de l'instantané d'arrêt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInstantaneDto {
    pub version: i32,
    pub empreinte: String,
    pub motif: Option<String>,
    pub pris_le: DateTime<Utc>,
    /// `None` : pris par le passage automatique.
    pub pris_par: Option<Uuid>,
    pub lignes: i64,
    pub appreciations: i64,
}

/// Les versions de l'instantané d'une classe, et si la dernière est encore à jour (EVA-186).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstantanesDto {
    pub periode_id: Uuid,
    pub classe_id: Uuid,
    pub etat: String,
    pub versions: Vec<VersionInstantaneDto>,
    /// Empreinte que prendrait un instantané maintenant.
    pub empreinte_courante: String,
    /// La dernière version a la même empreinte que l'état courant (`false` : une note, une appréciation a changé depuis).
    pub a_jour: bool,
}

/// Une moyenne d'un élève dans un instantané.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneInstantaneDto {
    pub eleve_id: Uuid,
    /// `moyenne_matiere` ou `moyenne_generale`.
    pub nature: String,
    pub matiere_id: Option<Uuid>,
    /// Valeur affichée en millièmes (`None` : aucune moyenne).
    pub valeur_m: Option<i64>,
    pub coef_m: Option<i64>,
}

/// Un écart entre deux versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcartInstantaneDto {
    pub eleve_id: Uuid,
    pub nature: String,
    pub matiere_id: Option<Uuid>,
    pub avant_m: Option<i64>,
    pub apres_m: Option<i64>,
}

/// Le contenu d'une version, et ses écarts avec une autre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstantaneDto {
    pub version: VersionInstantaneDto,
    pub lignes: Vec<LigneInstantaneDto>,
    /// Version comparée (`comparer_a`) et ses écarts, ligne à ligne.
    pub comparee_a: Option<i32>,
    pub ecarts: Vec<EcartInstantaneDto>,
    /// Appréciations dont la version a changé (ou apparues, ou disparues) entre les deux versions.
    pub appreciations_changees: i64,
}

/// Reprendre l'instantané (nouvelle version, motivée).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReprendreInstantaneBody {
    pub motif: String,
}

/// Issue d'une reprise : nouvelle version, ou version inchangée (même empreinte).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepriseInstantaneDto {
    pub version: i32,
    pub empreinte: String,
    pub nouvelle: bool,
}

/// Autoriser un rattrapage après l'arrêt (EVA-183, R-V10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoriserRattrapageBody {
    pub evaluation_id: Uuid,
    pub eleve_id: Uuid,
    pub jusqu_au: NaiveDate,
    pub motif: String,
}

/// Demander une correction après l'arrêt (EVA-189, professeur du cours).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemanderCorrectionBody {
    pub evaluation_id: Uuid,
    pub eleve_id: Uuid,
    /// Version de la case lue par le professeur (la demande vise cette version).
    pub version_vue: i32,
    /// Statut proposé (`note`, `absent`, `a_rattraper`, `dispense`, `non_rendu`, `non_note`).
    pub statut: String,
    /// Note proposée en millièmes (statut `note` seulement).
    #[serde(default)]
    pub valeur_m: Option<i64>,
    pub motif: String,
}

/// Effet d'une correction sur les moyennes de l'élève (EVA-190 : « effet sur moyennes affiché »).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffetCorrectionDto {
    pub eleve_id: Uuid,
    pub periode_id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub matiere_avant_m: Option<i64>,
    pub matiere_apres_m: Option<i64>,
    pub generale_avant_m: Option<i64>,
    pub generale_apres_m: Option<i64>,
    /// Seuils franchis par la moyenne générale (millièmes, voir [`SEUILS_SIGNALES_M`]).
    pub seuils_franchis_m: Vec<i64>,
    /// État de la période de la classe de l'élève au moment du calcul.
    pub etat_periode: String,
    /// Phrases à lire avant de décider (« le conseil a déjà eu lieu : ses décisions ne changent pas seules »…).
    pub messages: Vec<String>,
}

/// Une demande de correction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemandeCorrectionDto {
    pub id: Uuid,
    pub evaluation_id: Uuid,
    pub eleve_id: Uuid,
    pub resultat_id: Uuid,
    pub demandeur: Uuid,
    pub de_moi: bool,
    pub motif: String,
    pub version_vue: Option<i32>,
    pub statut_propose: Option<String>,
    pub valeur_proposee_m: Option<i64>,
    /// `en_attente`, `acceptee`, `refusee`, `retiree`.
    pub etat: String,
    pub decision_motif: Option<String>,
    pub decide_le: Option<DateTime<Utc>>,
    pub cree_le: DateTime<Utc>,
    pub matiere_avant_m: Option<i64>,
    pub matiere_apres_m: Option<i64>,
    pub generale_avant_m: Option<i64>,
    pub generale_apres_m: Option<i64>,
}

/// Décider d'une demande (EVA-190 ★).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeciderCorrectionBody {
    pub accepter: bool,
    /// Obligatoire pour un refus ; repris dans le journal pour une acceptation.
    #[serde(default)]
    pub motif: Option<String>,
}

/// Corriger directement (EVA-191 ★, direction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrigerBody {
    pub evaluation_id: Uuid,
    pub eleve_id: Uuid,
    /// Version de la case lue (`None` : la case est vide).
    #[serde(default)]
    pub version_attendue: Option<i32>,
    pub statut: String,
    #[serde(default)]
    pub valeur_m: Option<i64>,
    pub motif: String,
}

/// Issue d'une correction (directe ou acceptée).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrectionFaiteDto {
    pub resultat_id: Uuid,
    pub version: i32,
    pub effet: EffetCorrectionDto,
    /// Version de l'instantané après la correction (`None` : la classe n'a pas d'instantané).
    pub instantane_version: Option<i32>,
}

/// Une ligne du journal des corrections (EVA-193).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneJournalCorrectionDto {
    pub le: DateTime<Utc>,
    pub evaluation_id: Uuid,
    pub titre: String,
    pub eleve_id: Uuid,
    pub version: i32,
    pub statut_avant: Option<String>,
    pub valeur_avant_m: Option<i64>,
    pub statut_apres: String,
    pub valeur_apres_m: Option<i64>,
    pub motif: Option<String>,
    pub auteur: Option<Uuid>,
}

/// Un cours dans l'état de saisie d'une classe (EVA-178).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoursSaisieDto {
    pub cours_id: Uuid,
    pub libelle: String,
    pub professeurs: Vec<Uuid>,
    pub evaluations: i64,
    pub non_publiees: i64,
    pub cases_vides: i64,
    /// Élèves de la classe sans appréciation de cours validée (à partir de l'arrêt des notes).
    pub appreciations_manquantes: i64,
}

/// Ce qui manque à un professeur pour une classe, et le message de relance prérempli (EVA-179).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManquesProfesseurDto {
    pub prof_user_id: Uuid,
    pub non_publiees: i64,
    pub cases_vides: i64,
    pub cours_sans_evaluation: i64,
    pub appreciations_manquantes: i64,
    pub message: String,
    /// Une relance manuelle est déjà partie aujourd'hui.
    pub relance_aujourdhui: bool,
}

impl ManquesProfesseurDto {
    pub fn a_des_manques(&self) -> bool {
        self.non_publiees
            + self.cases_vides
            + self.cours_sans_evaluation
            + self.appreciations_manquantes
            > 0
    }
}

/// État de saisie d'une classe pour une période (EVA-178).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EtatSaisieDto {
    pub periode_id: Uuid,
    pub classe_id: Uuid,
    pub etat: String,
    pub date_arret: Option<NaiveDate>,
    /// Jours (de l'établissement) jusqu'à la date d'arrêt ; négatif une fois passée.
    pub jours_avant_arret: Option<i64>,
    pub date_conseil: Option<NaiveDate>,
    pub eleves: i64,
    pub cours: Vec<CoursSaisieDto>,
    pub cours_sans_evaluation: i64,
    pub professeurs: Vec<ManquesProfesseurDto>,
}

/// Relancer des professeurs (EVA-179) : aperçu dans l'état de saisie, puis ce geste confirmé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelancerBody {
    /// Professeurs à relancer ; vide : tous ceux à qui il manque quelque chose.
    #[serde(default)]
    pub professeurs: Vec<Uuid>,
    /// Message de la direction ; défaut : le message prérempli.
    #[serde(default)]
    pub message: Option<String>,
}

/// Une relance du journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelanceDto {
    pub id: Uuid,
    pub prof_user_id: Uuid,
    /// `manuelle`, `j7`, `j2`.
    pub nature: String,
    pub jour: NaiveDate,
    pub echeance: Option<NaiveDate>,
    pub non_publiees: i64,
    pub cases_vides: i64,
    pub cours_sans_evaluation: i64,
    pub message: Option<String>,
    pub par: Option<Uuid>,
    pub le: DateTime<Utc>,
}

/// Un enseignement obligatoire sans moyenne annuelle (R-03, CE-63).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManqueAnnuelDto {
    pub eleve_id: Uuid,
    pub matiere_id: Uuid,
    pub matiere: String,
}

/// Contrôle de fin d'année de 3ᵉ (EVA-188).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Controle3eDto {
    pub periode_id: Uuid,
    pub classe_id: Uuid,
    /// La classe est de niveau 3ᵉ et la période est la dernière de sa périodisation.
    pub applicable: bool,
    pub manques: Vec<ManqueAnnuelDto>,
    /// La clôture de la période est-elle permise ?
    pub cloture_permise: bool,
}

/// Texte d'un rappel ou d'une relance (faits seulement, aucun nom d'élève).
pub fn message_relance(
    non_publiees: i64,
    cases_vides: i64,
    sans_evaluation: i64,
    appreciations: i64,
    date_arret: Option<NaiveDate>,
) -> String {
    let mut faits = Vec::new();
    if non_publiees > 0 {
        faits.push(format!(
            "{non_publiees} évaluation{} non publiée{}",
            pluriel(non_publiees),
            pluriel(non_publiees)
        ));
    }
    if cases_vides > 0 {
        faits.push(format!(
            "{cases_vides} résultat{} non saisi{}",
            pluriel(cases_vides),
            pluriel(cases_vides)
        ));
    }
    if sans_evaluation > 0 {
        faits.push(format!("{sans_evaluation} cours sans évaluation"));
    }
    if appreciations > 0 {
        faits.push(format!(
            "{appreciations} appréciation{} à rédiger",
            pluriel(appreciations)
        ));
    }
    let echeance = match date_arret {
        Some(d) => format!(" avant l'arrêt des notes du {}", d.format("%d/%m/%Y")),
        None => String::new(),
    };
    if faits.is_empty() {
        format!("Rien ne manque pour cette classe{echeance}.")
    } else {
        format!(
            "Pour cette classe, il reste{echeance} : {}.",
            faits.join(", ")
        )
    }
}

fn pluriel(n: i64) -> &'static str {
    if n > 1 {
        "s"
    } else {
        ""
    }
}

/// Seuils franchis entre deux moyennes (millièmes), dans l'ordre croissant.
pub fn seuils_franchis(avant: Option<i64>, apres: Option<i64>) -> Vec<i64> {
    match (avant, apres) {
        (Some(a), Some(b)) => SEUILS_SIGNALES_M
            .iter()
            .copied()
            .filter(|s| (a < *s) != (b < *s))
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seuils_franchis_dans_les_deux_sens() {
        assert_eq!(seuils_franchis(Some(9_960), Some(10_040)), vec![10_000]);
        assert_eq!(
            seuils_franchis(Some(12_500), Some(7_900)),
            vec![8_000, 10_000, 12_000]
        );
        assert!(seuils_franchis(Some(10_000), Some(10_500)).is_empty());
        assert!(seuils_franchis(None, Some(10_500)).is_empty());
    }

    #[test]
    fn message_de_relance_en_mots() {
        let d = NaiveDate::from_ymd_opt(2031, 12, 5).unwrap();
        assert_eq!(
            message_relance(2, 1, 0, 0, Some(d)),
            "Pour cette classe, il reste avant l'arrêt des notes du 05/12/2031 : 2 évaluations non publiées, 1 résultat non saisi."
        );
        assert_eq!(
            message_relance(0, 0, 1, 3, None),
            "Pour cette classe, il reste : 1 cours sans évaluation, 3 appréciations à rédiger."
        );
        assert_eq!(
            message_relance(0, 0, 0, 0, None),
            "Rien ne manque pour cette classe."
        );
    }
}
