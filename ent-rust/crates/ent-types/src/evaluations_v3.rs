//! Évaluations, lot V3 « grille et saisie » (socle 21) : types JSON des routes `/eva/*` et analyse d'une
//! cellule saisie au clavier.
//!
//! **Aucun calcul de moyenne ici** : toutes les valeurs d'affichage (moyennes, minimum, maximum, médiane)
//! sont fournies par le moteur unique du serveur (R-V3), en millièmes de point. Le seul calcul partagé est
//! l'**analyse du texte d'une cellule** (`analyser_cellule`) : le serveur l'applique pour décider, le
//! navigateur la même pour montrer tout de suite ce qui sera compris (pas de divergence possible).

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ───────────────────────────── Saisie rapide ─────────────────────────────

/// Statuts qu'une saisie peut poser (§8.2) : les autres se calculent ou demandent un motif.
pub const STATUTS_SAISISSABLES: [&str; 5] =
    ["absent", "a_rattraper", "dispense", "non_rendu", "non_note"];

/// Codes rapides au clavier, avec leur statut et leur libellé : affichés dans l'aide de la grille.
pub const CODES_RAPIDES: [(&str, &str, &str); 5] = [
    ("abs", "absent", "Absent"),
    ("ar", "a_rattraper", "À rattraper"),
    ("disp", "dispense", "Dispensé"),
    ("nr", "non_rendu", "Non rendu"),
    ("nn", "non_note", "Non noté"),
];

/// Libellé français d'un statut de résultat.
pub fn libelle_statut(statut: &str) -> &'static str {
    match statut {
        "note" => "Note",
        "absent" => "Absent",
        "a_rattraper" => "À rattraper",
        "dispense" => "Dispensé",
        "non_rendu" => "Non rendu",
        "non_note" => "Non noté",
        "annule" => "Annulé",
        _ => "Statut inconnu",
    }
}

/// Code court d'un statut (celui qu'on tape), pour l'affichage compact d'une cellule.
pub fn code_statut(statut: &str) -> &'static str {
    match statut {
        "absent" => "abs",
        "a_rattraper" => "ar",
        "dispense" => "disp",
        "non_rendu" => "nr",
        "non_note" => "nn",
        "annule" => "ann",
        _ => "",
    }
}

/// Ce que dit une case : contraintes de l'évaluation et de l'établissement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContrainteCellule {
    /// Barème en millièmes.
    pub bareme_m: i64,
    pub hors_bareme_autorise: bool,
    /// Pas de saisie en millièmes (10 : 0,01 point) ; 0 : aucun contrôle.
    pub pas_m: i64,
}

/// Résultat de l'analyse d'un texte de cellule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Analyse {
    /// Case vide : rien à écrire (un résultat ne se supprime jamais, R-V6).
    Vide,
    /// Un statut sans valeur.
    Statut(&'static str),
    /// Une note en millièmes.
    Note(i64),
    /// Texte refusé, avec la raison (affichable telle quelle).
    Erreur(String),
}

fn sans_accents_minuscules(t: &str) -> String {
    t.trim()
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            _ => c,
        })
        .collect()
}

/// Statut désigné par un code rapide ou un mot entier (« abs », « absent », « non rendu »…).
pub fn statut_du_code(texte: &str) -> Option<&'static str> {
    let t = sans_accents_minuscules(texte);
    let t = t.trim_end_matches('.');
    match t {
        "abs" | "absent" | "absente" => Some("absent"),
        "ar" | "a rattraper" | "arattraper" | "rattrapage" => Some("a_rattraper"),
        "disp" | "dispense" | "dispensee" => Some("dispense"),
        "nr" | "non rendu" | "nonrendu" | "n.r" => Some("non_rendu"),
        "nn" | "non note" | "nonnote" | "n.n" => Some("non_note"),
        _ => None,
    }
}

/// Nombre décimal en millièmes (« 12 », « 12,5 », « 12.25 »), au plus trois décimales.
pub fn milliemes_depuis_texte(t: &str) -> Option<i64> {
    let t = t.trim().replace(',', ".");
    if t.is_empty() || t.starts_with('+') {
        return None;
    }
    let (entier, frac) = match t.split_once('.') {
        Some((e, f)) => (e, f),
        None => (t.as_str(), ""),
    };
    if entier.is_empty() && frac.is_empty() {
        return None;
    }
    if !entier.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if entier.len() > 6 || frac.len() > 3 {
        return None;
    }
    let e: i64 = if entier.is_empty() {
        0
    } else {
        entier.parse().ok()?
    };
    let mut f = frac.to_string();
    while f.len() < 3 {
        f.push('0');
    }
    Some(e * 1000 + f.parse::<i64>().ok()?)
}

/// Formate des millièmes à la française (« 12,5 », « 8 », « 13,125 »).
pub fn formater_milliemes(m: i64) -> String {
    let neg = m < 0;
    let a = m.abs();
    let (e, f) = (a / 1000, a % 1000);
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    s.push_str(&e.to_string());
    if f != 0 {
        let mut d = format!("{f:03}");
        while d.ends_with('0') {
            d.pop();
        }
        s.push(',');
        s.push_str(&d);
    }
    s
}

/// Analyse le texte d'une cellule : vide, code rapide, note. **Le serveur reste juge** (il rejoue cette même
/// fonction puis contrôle le droit, la période et la version).
pub fn analyser_cellule(texte: &str, c: &ContrainteCellule) -> Analyse {
    let t = texte.trim();
    if t.is_empty() {
        return Analyse::Vide;
    }
    if let Some(s) = statut_du_code(t) {
        return Analyse::Statut(s);
    }
    if t.starts_with('-') {
        return Analyse::Erreur("une note ne peut pas être négative".to_string());
    }
    let Some(m) = milliemes_depuis_texte(t) else {
        return Analyse::Erreur(format!(
            "« {} » n'est ni une note ni un code (abs, ar, disp, nr, nn)",
            t.chars().take(20).collect::<String>()
        ));
    };
    if m > c.bareme_m && !c.hors_bareme_autorise {
        return Analyse::Erreur(format!(
            "{} dépasse le barème ({}) : vérifiez le barème de l'évaluation",
            formater_milliemes(m),
            formater_milliemes(c.bareme_m)
        ));
    }
    if c.pas_m > 1 && m % c.pas_m != 0 {
        return Analyse::Erreur(format!(
            "la note doit être un multiple de {}",
            formater_milliemes(c.pas_m)
        ));
    }
    Analyse::Note(m)
}

// ───────────────────────────── Enseignements et périodes ─────────────────────────────

/// Un enseignement (cours) auquel l'utilisateur a accès : y compris un cours **sans matière du
/// référentiel** (libellé libre), qui bloquait l'ancienne grille.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnseignementV3Dto {
    pub cours_id: Uuid,
    pub libelle: String,
    pub cible_code: String,
    pub matiere_id: Option<Uuid>,
    pub sans_matiere: bool,
    /// `saisie` (peut saisir et créer) ou `lecture` (grille seulement).
    pub droit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodeV3Dto {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
}

/// Une sous-matière de la matière d'un cours (réglage d'une évaluation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SousMatiereV3Dto {
    pub id: Uuid,
    pub libelle: String,
}

// ───────────────────────────── Grille ─────────────────────────────

/// Statistiques d'une série, en millièmes, fournies par le moteur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsV3Dto {
    pub effectif: usize,
    pub moyenne_m: i64,
    pub min_m: i64,
    pub max_m: i64,
    pub mediane_m: i64,
}

/// Une colonne : une évaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationColonneDto {
    pub id: Uuid,
    pub titre: String,
    pub date_eval: NaiveDate,
    pub bareme_m: i64,
    pub coef_m: i64,
    /// `normal`, `si_ameliore`, `ne_compte_pas`, `bonus`.
    pub comptage: String,
    pub hors_bareme_autorise: bool,
    /// `brouillon`, `ouverte`, `annulee`.
    pub etat: String,
    pub motif_annulation: Option<String>,
    pub version: i32,
    /// `explicite`, `programmee`, `a_l_arret`.
    pub publication: String,
    pub publication_prevue_le: Option<DateTime<Utc>>,
    pub publiee_le: Option<DateTime<Utc>>,
    pub retiree_le: Option<DateTime<Utc>>,
    pub en_cours_de_correction: bool,
    pub sous_matiere_id: Option<Uuid>,
    /// L'évaluation ne vise qu'une partie du public (demi-groupe, volontaires).
    pub restreinte: bool,
    /// L'évaluation est commune à plusieurs cours (examen blanc).
    pub commune: bool,
    pub annoncee: bool,
    /// Elle porte déjà des résultats : changer barème, coefficient ou date exige un motif.
    pub a_des_resultats: bool,
    pub peut_modifier: bool,
    /// Le retrait est encore possible par l'auteur (24 h après la publication).
    pub retrait_possible: bool,
    pub stats: Option<StatsV3Dto>,
}

/// Une case : le résultat d'un élève à une évaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CelluleDto {
    /// L'élève est dans le public du cours à la date de l'évaluation (sinon « non concerné », jamais saisi).
    pub concerne: bool,
    pub statut: Option<String>,
    pub valeur_m: Option<i64>,
    pub version: Option<i32>,
    pub resultat_id: Option<Uuid>,
    /// La saisie est ouverte pour cet élève (période de sa classe ouverte ou rattrapage autorisé).
    pub saisie_ouverte: bool,
    pub rattrapage_autorise: bool,
    /// La période de l'élève n'est pas celle affichée : la case ne compte pas dans la moyenne affichée.
    pub hors_periode: bool,
    /// Le résultat a été corrigé au moins une fois (historique à voir).
    pub corrige: bool,
    pub commentaire: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EleveLigneDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    /// Arrivé ou parti en cours de période : phrase d'information (jamais de motif).
    pub mouvement: Option<String>,
    /// Même ordre que `GrilleV3Dto::evaluations`.
    pub cellules: Vec<CelluleDto>,
    /// Moyenne du cours, calculée par le moteur (millièmes), `None` sans note comptée.
    pub moyenne_m: Option<i64>,
    pub nb_comptees: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrilleV3Dto {
    pub enseignement: EnseignementV3Dto,
    pub periode: PeriodeV3Dto,
    /// Jour de l'établissement au moment de la lecture.
    pub jour: NaiveDate,
    pub peut_saisir: bool,
    pub peut_corriger: bool,
    /// Échelle de la moyenne (20000 : sur 20).
    pub echelle_m: i64,
    /// Pas de saisie de l'établissement (10 : 0,01 point).
    pub pas_m: i64,
    pub delai_publication_max_jours: Option<i16>,
    pub evaluations: Vec<EvaluationColonneDto>,
    pub eleves: Vec<EleveLigneDto>,
    pub moyenne_classe: Option<StatsV3Dto>,
    pub avertissements: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplicationDto {
    pub moyenne_m: Option<i64>,
    pub echelle_m: i64,
    pub lignes: Vec<String>,
    pub avertissements: Vec<String>,
}

// ───────────────────────────── Saisie ─────────────────────────────

/// Une cellule à écrire : le **texte tapé** (le serveur l'analyse) et la version que l'écran avait lue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CelluleSaisieDto {
    pub evaluation_id: Uuid,
    pub eleve_id: Uuid,
    pub texte: String,
    /// `None` : l'écran croyait la case vide (création) ; `Some(v)` : version lue. Une version différente en
    /// base est un **conflit**, jamais un écrasement silencieux.
    #[serde(default)]
    pub version_attendue: Option<i32>,
    /// Motif écrit d'une correction après l'arrêt de la période de l'élève.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaisieLotBody {
    pub cours_id: Uuid,
    pub cellules: Vec<CelluleSaisieDto>,
    /// `saisie` (défaut), `import` : conservé dans l'historique.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origine: Option<String>,
}

/// Ce que contient la case au moment du conflit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConflitDto {
    pub statut: String,
    pub valeur_m: Option<i64>,
    pub version: i32,
    pub par_nom: Option<String>,
    pub le: Option<DateTime<Utc>>,
}

/// `ecrit`, `inchange`, `vide` (rien à écrire), `conflit`, `refuse`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CelluleRetourDto {
    pub evaluation_id: Uuid,
    pub eleve_id: Uuid,
    pub issue: String,
    pub version: Option<i32>,
    pub statut: Option<String>,
    pub valeur_m: Option<i64>,
    pub message: Option<String>,
    pub conflit: Option<ConflitDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaisieLotRetourDto {
    pub cellules: Vec<CelluleRetourDto>,
}

// ───────────────────────────── Évaluations ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreerEvaluationV3Body {
    pub cours_id: Uuid,
    pub titre: String,
    pub date_eval: NaiveDate,
    pub bareme_m: i64,
    pub coef_m: i64,
    #[serde(default)]
    pub comptage: Option<String>,
    #[serde(default)]
    pub hors_bareme_autorise: bool,
    #[serde(default)]
    pub sous_matiere_id: Option<Uuid>,
    /// Élèves visés si l'évaluation ne concerne qu'une partie du public (demi-groupe, volontaires).
    #[serde(default)]
    pub restreinte_a: Vec<Uuid>,
    /// `explicite` (défaut), `programmee`, `a_l_arret`.
    #[serde(default)]
    pub publication: Option<String>,
    #[serde(default)]
    pub publication_prevue_le: Option<DateTime<Utc>>,
    #[serde(default)]
    pub annoncee: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModifierEvaluationV3Body {
    /// Version lue par l'écran (conflit si elle a changé).
    pub version_attendue: i32,
    #[serde(default)]
    pub titre: Option<String>,
    #[serde(default)]
    pub date_eval: Option<NaiveDate>,
    #[serde(default)]
    pub bareme_m: Option<i64>,
    #[serde(default)]
    pub coef_m: Option<i64>,
    #[serde(default)]
    pub comptage: Option<String>,
    #[serde(default)]
    pub hors_bareme_autorise: Option<bool>,
    #[serde(default)]
    pub annoncee: Option<bool>,
    /// Motif écrit : exigé quand l'évaluation porte déjà des résultats ou que la période est arrêtée.
    #[serde(default)]
    pub motif: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnnulerEvaluationV3Body {
    pub version_attendue: i32,
    pub motif: String,
}

/// `maintenant`, `programmer`, `a_l_arret`, `retirer`, `annuler_programmation`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublierV3Body {
    pub action: String,
    #[serde(default)]
    pub prevue_le: Option<DateTime<Utc>>,
    /// Heure **de l'établissement** (sans fuseau), convertie par le serveur : ce que saisit l'écran.
    #[serde(default)]
    pub prevue_locale: Option<NaiveDateTime>,
    #[serde(default)]
    pub motif: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnCoursCorrectionBody {
    pub en_cours: bool,
}

// ───────────────────────────── Historique ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LigneHistoriqueV3Dto {
    pub version: i32,
    pub statut_avant: Option<String>,
    pub valeur_avant_m: Option<i64>,
    pub statut_apres: String,
    pub valeur_apres_m: Option<i64>,
    pub correction: bool,
    pub motif: Option<String>,
    pub source: String,
    pub auteur_nom: Option<String>,
    pub le: DateTime<Utc>,
}

// ───────────────────────────── Collage et import ─────────────────────────────

/// Une ligne à rapprocher d'un élève : par identifiant (collage dans la grille), par nom (fichier), sinon par rang.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportLigneBody {
    #[serde(default)]
    pub eleve_id: Option<Uuid>,
    #[serde(default)]
    pub nom: Option<String>,
    /// Rang (à partir de 1) dans l'ordre de la grille, quand ni identifiant ni nom ne sont donnés.
    #[serde(default)]
    pub rang: Option<usize>,
    /// Un texte par évaluation cible, dans l'ordre de `cibles`.
    pub cellules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportBody {
    pub cours_id: Uuid,
    /// `collage` ou `fichier`.
    pub mode: String,
    pub cibles: Vec<Uuid>,
    pub lignes: Vec<ImportLigneBody>,
    /// Empreinte de l'aperçu : exigée à la confirmation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empreinte: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportCelluleApercuDto {
    pub evaluation_id: Uuid,
    pub texte: String,
    /// `nouveau`, `modifie`, `inchange`, `vide`, `erreur`, `verrouille`.
    pub etat: String,
    pub statut: Option<String>,
    pub valeur_m: Option<i64>,
    pub avant_statut: Option<String>,
    pub avant_valeur_m: Option<i64>,
    pub erreur: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportEleveRapprocheDto {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportLigneApercuDto {
    /// Numéro de la ligne dans l'envoi (à partir de 1).
    pub numero: usize,
    pub nom_saisi: Option<String>,
    pub eleve: Option<ImportEleveRapprocheDto>,
    /// `ok`, `non_rapproche`, `ambigu`, `hors_public`.
    pub rapprochement: String,
    pub cellules: Vec<ImportCelluleApercuDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ImportTotauxDto {
    pub lignes: usize,
    pub rapprochees: usize,
    pub non_rapprochees: usize,
    pub nouvelles: usize,
    pub modifiees: usize,
    pub inchangees: usize,
    pub erreurs: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportApercuDto {
    pub empreinte: String,
    pub lignes: Vec<ImportLigneApercuDto>,
    pub totaux: ImportTotauxDto,
    /// Élèves de la grille absents du fichier (information, jamais une erreur).
    pub eleves_absents_du_fichier: usize,
}

// ───────────────────────────── Tests ─────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const C: ContrainteCellule = ContrainteCellule {
        bareme_m: 20_000,
        hors_bareme_autorise: false,
        pas_m: 10,
    };

    #[test]
    fn notes_et_virgules() {
        assert_eq!(analyser_cellule("12", &C), Analyse::Note(12_000));
        assert_eq!(analyser_cellule(" 12,5 ", &C), Analyse::Note(12_500));
        assert_eq!(analyser_cellule("12.25", &C), Analyse::Note(12_250));
        assert_eq!(analyser_cellule(",5", &C), Analyse::Note(500));
        assert_eq!(analyser_cellule("20", &C), Analyse::Note(20_000));
    }

    #[test]
    fn codes_rapides_et_mots() {
        assert_eq!(analyser_cellule("abs", &C), Analyse::Statut("absent"));
        assert_eq!(analyser_cellule("ABS", &C), Analyse::Statut("absent"));
        assert_eq!(
            analyser_cellule("Dispensé", &C),
            Analyse::Statut("dispense")
        );
        assert_eq!(
            analyser_cellule("non rendu", &C),
            Analyse::Statut("non_rendu")
        );
        assert_eq!(analyser_cellule("nn", &C), Analyse::Statut("non_note"));
        assert_eq!(analyser_cellule("ar", &C), Analyse::Statut("a_rattraper"));
        assert_eq!(
            analyser_cellule("à rattraper", &C),
            Analyse::Statut("a_rattraper")
        );
    }

    #[test]
    fn vide_ne_supprime_rien() {
        assert_eq!(analyser_cellule("   ", &C), Analyse::Vide);
    }

    #[test]
    fn refus_explicites() {
        assert!(matches!(analyser_cellule("21", &C), Analyse::Erreur(_)));
        assert!(matches!(analyser_cellule("-1", &C), Analyse::Erreur(_)));
        assert!(matches!(analyser_cellule("douze", &C), Analyse::Erreur(_)));
        assert!(matches!(analyser_cellule("12,345", &C), Analyse::Erreur(_)));
        assert!(matches!(analyser_cellule("1,2,3", &C), Analyse::Erreur(_)));
        assert!(matches!(analyser_cellule("1e3", &C), Analyse::Erreur(_)));
        assert!(matches!(analyser_cellule("+5", &C), Analyse::Erreur(_)));
    }

    #[test]
    fn hors_bareme_autorise_et_pas() {
        let c = ContrainteCellule {
            hors_bareme_autorise: true,
            ..C
        };
        assert_eq!(analyser_cellule("22", &c), Analyse::Note(22_000));
        let demi = ContrainteCellule { pas_m: 500, ..C };
        assert_eq!(analyser_cellule("12,5", &demi), Analyse::Note(12_500));
        assert!(matches!(
            analyser_cellule("12,25", &demi),
            Analyse::Erreur(_)
        ));
    }

    #[test]
    fn formatage() {
        assert_eq!(formater_milliemes(12_500), "12,5");
        assert_eq!(formater_milliemes(8_000), "8");
        assert_eq!(formater_milliemes(13_125), "13,125");
        assert_eq!(formater_milliemes(0), "0");
    }

    #[test]
    fn aller_retour_serie() {
        let c = CelluleSaisieDto {
            evaluation_id: Uuid::nil(),
            eleve_id: Uuid::nil(),
            texte: "12,5".into(),
            version_attendue: None,
            motif: None,
        };
        let j = serde_json::to_string(&c).unwrap();
        assert!(!j.contains("motif"));
        assert_eq!(serde_json::from_str::<CelluleSaisieDto>(&j).unwrap(), c);
    }
}
