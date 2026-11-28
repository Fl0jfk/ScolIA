//! Évaluations, lot V3f (socle 21, complément de V3) : barème détaillé par exercice (EVA-262), réussite par exercice
//! (EVA-263), détail montré à l'élève (EVA-264), conversion des notes à un changement de barème (CE-39) ou à
//! l'annulation d'un exercice (CE-83), duplication de plusieurs évaluations (EVA-125, M-667). Compile pour `wasm32`.
//!
//! Principes :
//! * **la note est la somme des points** : le moteur de calcul ne change pas, il lit la note ; le barème de l'évaluation
//!   est la somme des maximums des exercices non annulés ;
//! * **un détail n'est vrai que pour la version de la note qu'il compose** : si la note est changée à la main dans la
//!   grille, le détail est « périmé » et n'est plus montré à personne d'autre que le professeur ;
//! * **une conversion n'écrit rien sans aperçu** : l'aperçu rend une empreinte, la confirmation l'exige, et toute
//!   écriture intervenue entre les deux rend l'empreinte fausse (rien n'est écrit) ;
//! * **la réussite par exercice ne classe personne** : moyenne et répartition en quatre tranches, sans nom ni rang.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::evaluations_v3::CelluleRetourDto;

/// Nombre maximal d'exercices ou de critères d'un barème détaillé.
pub const MAX_EXERCICES: usize = 40;
/// Nombre maximal d'élèves dans un envoi de détails.
pub const MAX_ELEVES_DETAILS: usize = 300;
/// Nombre maximal d'évaluations dupliquées en une opération.
pub const MAX_DUPLICATIONS: usize = 60;

/// Un exercice (ou critère) du barème détaillé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExerciceDto {
    pub id: Uuid,
    pub ordre: i16,
    pub libelle: String,
    /// Maximum en millièmes de point.
    pub maximum_m: i64,
    pub annule: bool,
}

/// Réussite d'un exercice sur les détails à jour (EVA-263) : jamais de nom, jamais de rang.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReussiteExerciceDto {
    pub exercice_id: Uuid,
    /// Élèves dont le détail à jour porte des points sur cet exercice.
    pub effectif: i32,
    /// Moyenne des points, en millièmes (arrondie au millième inférieur).
    pub moyenne_m: Option<i64>,
    /// Répartition en quatre tranches du maximum : moins du quart, du quart à la moitié, de la moitié aux trois quarts,
    /// trois quarts et plus.
    pub repartition: [i32; 4],
}

/// Le barème détaillé d'une évaluation, tel que le professeur le voit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaremeDetailleDto {
    pub evaluation_id: Uuid,
    /// Version de l'évaluation (jeton de compare-et-écrit de toutes les écritures de ce lot).
    pub version: i32,
    pub bareme_m: i64,
    /// Somme des maximums des exercices non annulés (égale au barème quand le barème détaillé est cohérent).
    pub somme_maximums_m: i64,
    pub detail_visible_eleve: bool,
    pub publiee: bool,
    pub exercices: Vec<ExerciceDto>,
    pub reussite: Vec<ReussiteExerciceDto>,
    pub nb_details_a_jour: i32,
    pub nb_details_perimes: i32,
    /// Les exercices se redéfinissent tant qu'aucun point n'est saisi.
    pub redefinissable: bool,
}

/// Un exercice à définir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExerciceSaisiDto {
    pub libelle: String,
    pub maximum_m: i64,
}

/// Définir (ou redéfinir, tant qu'aucun point n'est saisi) le barème détaillé (EVA-262). La somme des maximums doit
/// égaler le barème de l'évaluation ; une liste vide retire le barème détaillé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinirBaremeBody {
    pub version_attendue: i32,
    pub exercices: Vec<ExerciceSaisiDto>,
}

/// Montrer ou cacher le détail par exercice à l'élève et à sa famille (EVA-264).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibiliteDetailBody {
    pub version_attendue: i32,
    pub visible: bool,
}

/// Points d'un exercice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointsExerciceDto {
    pub exercice_id: Uuid,
    pub points_m: i64,
}

/// Le détail d'un élève : un point par exercice non annulé ; la note écrite est leur somme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailEleveSaisiDto {
    pub eleve_id: Uuid,
    pub points: Vec<PointsExerciceDto>,
    /// Version de la note lue par l'écran (`None` : la case doit être vide).
    pub version_attendue: Option<i32>,
    /// Correction après l'arrêt de la période de l'élève : motif obligatoire.
    pub motif: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaisirDetailsBody {
    pub eleves: Vec<DetailEleveSaisiDto>,
}

/// Retour d'un envoi de détails : un retour par élève, au format de la grille (`ecrit`, `inchange`, `conflit`, `refuse`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaisirDetailsRetourDto {
    pub eleves: Vec<CelluleRetourDto>,
}

/// Une ligne de la grille de détail du professeur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailEleveDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    /// Statut de la note (`note`, `absent`…), `None` si la case est vide.
    pub statut: Option<String>,
    pub note_m: Option<i64>,
    pub version: Option<i32>,
    /// Points par exercice, dans l'ordre de `exercices` (`None` : pas de point saisi).
    pub points_m: Vec<Option<i64>>,
    /// Le détail compose la note actuelle (même version) ; faux si la note a été changée à la main.
    pub a_jour: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailsEvaluationDto {
    pub evaluation_id: Uuid,
    pub exercices: Vec<ExerciceDto>,
    pub eleves: Vec<DetailEleveDto>,
}

/// Demande de conversion : un nouveau barème (CE-39) **ou** l'annulation d'un exercice (CE-83), jamais les deux.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversionBody {
    pub version_attendue: i32,
    pub nouveau_bareme_m: Option<i64>,
    pub annuler_exercice_id: Option<Uuid>,
}

/// Une note avant et après la conversion proposée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LigneConversionDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub version: i32,
    pub avant_m: i64,
    pub apres_m: i64,
    /// `detail` (somme des exercices restants) ou `proportion` (règle de trois arrondie au pas de saisie).
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApercuConversionDto {
    pub evaluation_id: Uuid,
    pub bareme_avant_m: i64,
    pub bareme_apres_m: i64,
    pub lignes: Vec<LigneConversionDto>,
    /// Élèves notés qu'on ne peut pas convertir (élève protégé non visible, période arrêtée sans droit de correction).
    pub nb_non_convertibles: i32,
    pub publiee: bool,
    pub empreinte: String,
}

/// Confirmer une conversion : la même demande que l'aperçu, son empreinte et un motif (l'évaluation porte des notes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliquerConversionBody {
    pub version_attendue: i32,
    pub nouveau_bareme_m: Option<i64>,
    pub annuler_exercice_id: Option<Uuid>,
    pub empreinte: String,
    pub motif: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversionAppliqueeDto {
    pub evaluation_id: Uuid,
    pub version: i32,
    pub bareme_m: i64,
    pub nb_notes_converties: i32,
}

/// Une évaluation à dupliquer et la date de sa copie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DupliquerItemDto {
    pub evaluation_id: Uuid,
    pub date_eval: NaiveDate,
}

/// Dupliquer plusieurs évaluations (de l'an passé ou d'un autre de ses cours) dans un de ses cours, sans résultats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DupliquerBody {
    pub cours_cible_id: Uuid,
    pub evaluations: Vec<DupliquerItemDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationDupliqueeDto {
    pub source_id: Uuid,
    pub evaluation_id: Uuid,
    pub date_eval: NaiveDate,
    pub nb_exercices: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicationDto {
    pub creees: Vec<EvaluationDupliqueeDto>,
}

/// Un exercice tel que l'élève ou sa famille le voit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExerciceFamilleDto {
    pub libelle: String,
    pub maximum_m: i64,
    pub annule: bool,
    pub points_m: Option<i64>,
}

/// Le détail par exercice d'une note publiée, quand le professeur a choisi de le montrer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailFamilleDto {
    pub evaluation_id: Uuid,
    pub titre: String,
    pub bareme_m: i64,
    pub note_m: i64,
    pub exercices: Vec<ExerciceFamilleDto>,
}

/// Arrondit `x` (millièmes) au multiple de `pas_m` le plus proche, demi vers le haut. `pas_m <= 0` : inchangé.
pub fn arrondir_au_pas(x: i64, pas_m: i64) -> i64 {
    if pas_m <= 1 {
        return x;
    }
    let q = x.div_euclid(pas_m);
    let r = x.rem_euclid(pas_m);
    if r * 2 >= pas_m {
        (q + 1) * pas_m
    } else {
        q * pas_m
    }
}

/// Règle de trois exacte en entiers : `valeur × nouveau / ancien`, arrondie au pas (demi vers le haut) et bornée au
/// nouveau barème. `None` si l'ancien barème est nul ou si le calcul déborde.
pub fn convertir_proportion(
    valeur_m: i64,
    ancien_m: i64,
    nouveau_m: i64,
    pas_m: i64,
) -> Option<i64> {
    if ancien_m <= 0 || nouveau_m <= 0 || valeur_m < 0 {
        return None;
    }
    let num = i128::from(valeur_m).checked_mul(i128::from(nouveau_m))?;
    let den = i128::from(ancien_m);
    // Arrondi au millième le plus proche (demi vers le haut), puis au pas.
    let m = (num * 2 + den).div_euclid(den * 2);
    let m = i64::try_from(m).ok()?;
    Some(arrondir_au_pas(m, pas_m).min(nouveau_m.max(0)))
}

/// Tranche (0 à 3) des points d'un exercice : moins du quart, moins de la moitié, moins des trois quarts, au-delà.
pub fn tranche(points_m: i64, maximum_m: i64) -> usize {
    if maximum_m <= 0 {
        return 0;
    }
    let p = i128::from(points_m) * 4;
    let m = i128::from(maximum_m);
    if p < m {
        0
    } else if p < 2 * m {
        1
    } else if p < 3 * m {
        2
    } else {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrondi_au_pas_demi_haut() {
        assert_eq!(arrondir_au_pas(12_250, 500), 12_500);
        assert_eq!(arrondir_au_pas(12_249, 500), 12_000);
        assert_eq!(arrondir_au_pas(12_345, 1), 12_345);
        assert_eq!(arrondir_au_pas(12_345, 0), 12_345);
    }

    #[test]
    fn proportion_sur_dix_vers_vingt_et_inverse() {
        // 7/10 → 14/20 ; 13/20 → 6,5/10 ; 13,5/20 au demi-point → 6,75 → 7 sur 10.
        assert_eq!(
            convertir_proportion(7_000, 10_000, 20_000, 500),
            Some(14_000)
        );
        assert_eq!(
            convertir_proportion(13_000, 20_000, 10_000, 500),
            Some(6_500)
        );
        assert_eq!(
            convertir_proportion(13_500, 20_000, 10_000, 500),
            Some(7_000)
        );
        assert_eq!(convertir_proportion(13_500, 20_000, 10_000, 1), Some(6_750));
        // Barème réduit de 20 à 17 (exercice de 3 points annulé) : 20/20 → 17/17, jamais au-delà.
        assert_eq!(
            convertir_proportion(20_000, 20_000, 17_000, 250),
            Some(17_000)
        );
        assert_eq!(convertir_proportion(1, 0, 17_000, 250), None);
    }

    #[test]
    fn tranches_de_reussite() {
        assert_eq!(tranche(0, 4_000), 0);
        assert_eq!(tranche(999, 4_000), 0);
        assert_eq!(tranche(1_000, 4_000), 1);
        assert_eq!(tranche(2_000, 4_000), 2);
        assert_eq!(tranche(3_000, 4_000), 3);
        assert_eq!(tranche(4_000, 4_000), 3);
    }
}
