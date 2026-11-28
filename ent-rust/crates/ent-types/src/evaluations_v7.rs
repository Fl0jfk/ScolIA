//! Évaluations, lot V7 « famille et élève » (socle 21, `21_EVALUATIONS.md` §10, §17.6, §17.15 ; EVA-159 à EVA-168,
//! EVA-269 à EVA-272). Compile pour `wasm32` (aucune dépendance serveur).
//!
//! Principes :
//! * **rien n'est visible sans publication** (R-V7) : relevé, explication, statistiques et demandes de révision ne
//!   portent que sur des évaluations publiées ; une évaluation retirée après publication est dite « retirée pour
//!   vérification », jamais effacée sans un mot (§10.1) ;
//! * **la moyenne est provisoire** jusqu'à l'arrêt des notes de la classe de l'élève, et le relevé le dit avec la date ;
//! * **statistiques** : jamais sous le seuil d'effectif de l'établissement (5 par défaut, plancher 3, R-V12), jamais si
//!   l'établissement les masque (S-113), médiane seulement si elle est choisie ;
//! * **demande de révision** (EVA-166, CE-59, R-24) : un message tracé au professeur, pas un recours ; rien ne change
//!   sans le geste du professeur ou de la direction ; un parent ne voit jamais les demandes de l'autre (R-V46) ;
//! * **simulation** (EVA-270, S-130) : l'élève seulement, calcul par le moteur, rien n'est enregistré.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Longueur d'une demande de révision (comme la contrainte de 466).
pub const REVISION_MESSAGE_MIN: usize = 3;
pub const REVISION_MESSAGE_MAX: usize = 1000;
/// Longueur d'une réponse.
pub const REVISION_REPONSE_MAX: usize = 1000;
/// Demandes ouvertes au plus par demandeur, tous élèves confondus (pas de rafale).
pub const MAX_REVISIONS_OUVERTES: i64 = 20;
/// Horizon des évaluations annoncées (jours).
pub const HORIZON_A_VENIR_JOURS: i64 = 60;
/// Mention d'une évaluation retirée après publication (§10.1).
pub const MENTION_RETIREE: &str = "résultats retirés pour vérification";
/// Mention de la simulation (S-130).
pub const MENTION_SIMULATION: &str =
    "Simulation : rien n'est enregistré, ni vu par personne. La moyenne réelle dépend des notes qui seront publiées.";

/// Statistiques d'une évaluation montrées à la famille et à l'élève (millièmes, échelle de l'établissement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsFamilleDto {
    pub effectif: usize,
    pub moyenne_m: i64,
    pub min_m: i64,
    pub max_m: i64,
    /// Présente seulement si l'établissement montre la médiane aux familles.
    pub mediane_m: Option<i64>,
}

/// Un résultat publié dans le relevé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultatReleveDto {
    pub evaluation_id: Uuid,
    pub titre: String,
    pub date_eval: NaiveDate,
    pub statut: String,
    /// Libellé court (« Abs », « Disp », « NR »…) ; vide pour une note.
    pub abreviation: String,
    pub valeur_m: Option<i64>,
    pub bareme_m: i64,
    pub coef_m: i64,
    /// Commentaire du professeur destiné à l'élève, publié avec la note.
    pub commentaire: Option<String>,
    /// Statistiques de l'évaluation, sous les conditions de R-V12 et S-113.
    pub stats: Option<StatsFamilleDto>,
    /// Une demande de révision est encore possible (période de l'élève avant le conseil).
    pub revision_possible: bool,
}

/// Une matière du relevé : sa moyenne provisoire, son explication ligne à ligne, ses résultats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatiereReleveDto {
    pub matiere_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub moyenne_m: Option<i64>,
    pub coef_m: i64,
    pub hors_moyenne_generale: bool,
    /// Une phrase par évaluation, puis la conclusion (moteur du socle 21).
    pub explication: Vec<String>,
    pub resultats: Vec<ResultatReleveDto>,
}

/// Une évaluation retirée après publication : la famille voit le mot, jamais la note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationRetireeDto {
    pub evaluation_id: Uuid,
    pub titre: String,
    pub date_eval: NaiveDate,
    pub matiere_libelle: String,
    pub mention: String,
}

/// Dates prévues de la période (EVA-269).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendrierPeriodeDto {
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    /// Date d'arrêt des notes de la classe de l'élève (effective si la classe est arrêtée).
    pub date_arret: Option<NaiveDate>,
    /// Date prévue du conseil de classe (référentiel des périodes).
    pub date_conseil: Option<NaiveDate>,
}

/// Le relevé d'une période, tel que le voient la famille (lien « voir les notes ») et l'élève.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleveV7Dto {
    pub eleve_id: Uuid,
    pub periode_id: Uuid,
    pub periode_libelle: String,
    pub echelle_m: i64,
    /// État de la période de la classe de l'élève (`ouverte`, `notes_arretees`…), s'il en a une.
    pub etat_periode: Option<String>,
    /// Vrai tant que les notes de la classe de l'élève ne sont pas arrêtées.
    pub provisoire: bool,
    /// Phrase à afficher quand la moyenne est provisoire.
    pub mention_provisoire: Option<String>,
    pub calendrier: CalendrierPeriodeDto,
    pub matieres: Vec<MatiereReleveDto>,
    pub moyenne_generale_m: Option<i64>,
    pub explication_generale: Vec<String>,
    pub retirees: Vec<EvaluationRetireeDto>,
}

/// Une évaluation annoncée, pas encore publiée (EVA-162) : jamais de note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationAnnonceeDto {
    pub evaluation_id: Uuid,
    pub titre: String,
    pub date_eval: NaiveDate,
    pub matiere_libelle: String,
    pub bareme_m: Option<i64>,
    pub coef_m: i64,
}

/// Un rattrapage ou une évaluation de remplacement à passer (EVA-162), sans motif.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RattrapageFamilleDto {
    pub rattrapage_id: Uuid,
    pub evaluation_titre: String,
    pub matiere_libelle: String,
    /// `rattrapage` ou `remplacement`.
    pub nature: String,
    pub prevu_le: NaiveDate,
    pub heure: Option<String>,
    pub lieu: Option<String>,
    pub modalite: String,
    pub echeance: NaiveDate,
}

/// Une dispense en vigueur (EVA-163) : nature et matière, jamais le motif ni la pièce.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispenseFamilleDto {
    /// Libellé de la matière, ou « tous les enseignements ».
    pub matiere_libelle: String,
    pub nature: String,
    pub libelle: String,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
}

/// Ce qui vient : évaluations annoncées, rattrapages convoqués, dispenses en vigueur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AVenirDto {
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub evaluations: Vec<EvaluationAnnonceeDto>,
    pub rattrapages: Vec<RattrapageFamilleDto>,
    pub dispenses: Vec<DispenseFamilleDto>,
}

/// Coefficient d'une matière pour la classe de l'élève (EVA-164).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoefficientFamilleDto {
    pub matiere_id: Uuid,
    pub matiere_libelle: String,
    /// `None` : aucun coefficient trouvé (le calcul applique 1 et le signale à la direction).
    pub coef_m: Option<i64>,
    pub hors_moyenne_generale: bool,
}

/// Le projet d'évaluation : comment les moyennes se calculent, et les coefficients (EVA-164, R-15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjetEvaluationDto {
    pub eleve_id: Uuid,
    pub periode_id: Uuid,
    pub echelle_m: i64,
    /// Les règles de l'établissement, en phrases.
    pub regles: Vec<String>,
    pub coefficients: Vec<CoefficientFamilleDto>,
}

/// Simuler une note (élève seulement).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationBody {
    pub periode_id: Uuid,
    pub matiere_id: Uuid,
    /// Note imaginée, en millièmes de point.
    pub valeur_m: i64,
    /// Barème en millièmes (défaut : sur 20).
    #[serde(default)]
    pub bareme_m: Option<i64>,
    /// Coefficient en millièmes (défaut : 1).
    #[serde(default)]
    pub coef_m: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationDto {
    pub matiere_id: Uuid,
    pub matiere_avant_m: Option<i64>,
    pub matiere_apres_m: Option<i64>,
    pub generale_avant_m: Option<i64>,
    pub generale_apres_m: Option<i64>,
    pub mention: String,
}

/// Demander la révision d'une note publiée (élève, ou parent au lien « voir les notes »).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemanderRevisionBody {
    pub eleve_id: Uuid,
    pub evaluation_id: Uuid,
    pub message: String,
}

/// Répondre à une demande (professeur du cours, ou direction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepondreRevisionBody {
    pub reponse: String,
}

/// Une demande de révision vue par son auteur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub evaluation_id: Uuid,
    pub evaluation_titre: String,
    pub matiere_libelle: String,
    pub message: String,
    /// `ouverte`, `repondue`, `retiree`.
    pub etat: String,
    pub cree_le: DateTime<Utc>,
    pub reponse: Option<String>,
    pub repondue_le: Option<DateTime<Utc>>,
}

/// Une demande de révision vue du personnel (professeur du cours, direction) : le rôle du demandeur, pas son identité.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionPersonnelDto {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub evaluation_id: Uuid,
    pub evaluation_titre: String,
    pub cours_id: Uuid,
    /// `parent` ou `eleve` (`None` pour une demande antérieure à V7).
    pub demandeur_role: Option<String>,
    pub message: String,
    pub etat: String,
    pub cree_le: DateTime<Utc>,
    pub reponse: Option<String>,
    pub repondue_le: Option<DateTime<Utc>>,
}

/// Une entrée du glossaire (EVA-272).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntreeGlossaireDto {
    pub categorie: String,
    pub terme: String,
    pub abreviation: String,
    pub definition: String,
}

/// Glossaire des statuts, des mentions et des statistiques, en mots simples (EVA-272). Statique.
pub fn glossaire() -> Vec<EntreeGlossaireDto> {
    let e = |categorie: &str, terme: &str, abreviation: &str, definition: &str| EntreeGlossaireDto {
        categorie: categorie.to_string(),
        terme: terme.to_string(),
        abreviation: abreviation.to_string(),
        definition: definition.to_string(),
    };
    vec![
        e("statut", "Note", "", "La copie est corrigée et notée ; la note compte dans la moyenne selon son coefficient."),
        e("statut", "Absent", "Abs", "L'élève était absent. Une absence ne vaut jamais zéro : elle ne compte pas dans la moyenne."),
        e("statut", "À rattraper", "Abs", "L'évaluation sera rattrapée ; elle ne compte pas tant qu'elle n'est pas notée."),
        e("statut", "Dispensé", "Disp", "L'élève est dispensé de cette évaluation ; elle ne compte pas."),
        e("statut", "Non rendu", "NR", "Le travail n'a pas été rendu ; selon la règle de l'établissement, il ne compte pas ou compte zéro."),
        e("statut", "Non noté", "NN", "Le professeur a choisi de ne pas noter ce résultat ; il ne compte pas."),
        e("statut", "Annulé", "Ann", "Le résultat ou l'évaluation a été annulé ; il ne compte pas."),
        e("mention", "Provisoire", "", "La moyenne peut encore changer jusqu'à l'arrêt des notes de la classe."),
        e("mention", "Retiré pour vérification", "", "Le professeur ou la direction vérifie les résultats ; ils reviendront une fois vérifiés."),
        e("mention", "Ne compte pas", "", "Évaluation d'entraînement ou diagnostique : elle ne change pas la moyenne."),
        e("mention", "Compte si elle améliore", "", "L'évaluation n'entre dans la moyenne que si elle la fait monter."),
        e("mention", "Bonus", "", "Points ajoutés selon la règle de l'établissement, sans pouvoir faire baisser la moyenne."),
        e("statistique", "Moyenne de la classe", "", "Moyenne des élèves notés. Elle n'est jamais montrée sous un petit effectif, pour ne pas révéler les notes des autres."),
        e("statistique", "Note la plus basse, la plus haute", "", "Extrêmes des élèves notés, sous la même condition d'effectif."),
        e("statistique", "Médiane", "", "La moitié des élèves notés a au moins cette note. Montrée seulement si l'établissement l'a choisi."),
        e("statistique", "Coefficient", "", "Poids d'une évaluation dans la moyenne de la matière, ou d'une matière dans la moyenne générale."),
    ]
}

/// Contrôle d'un message de demande de révision : longueur, aucun caractère de contrôle (hors saut de ligne).
pub fn controler_message_revision(message: &str) -> Result<String, &'static str> {
    let m = message.trim();
    let n = m.chars().count();
    if n < REVISION_MESSAGE_MIN {
        return Err("message trop court : dites ce qui vous semble à revoir");
    }
    if n > REVISION_MESSAGE_MAX {
        return Err("message trop long (1 000 caractères au plus)");
    }
    if m.chars().any(|c| c.is_control() && c != '\n') {
        return Err("caractères interdits dans le message");
    }
    Ok(m.to_string())
}

/// Contrôle d'une réponse.
pub fn controler_reponse_revision(reponse: &str) -> Result<String, &'static str> {
    let m = reponse.trim();
    let n = m.chars().count();
    if n == 0 {
        return Err("réponse vide");
    }
    if n > REVISION_REPONSE_MAX {
        return Err("réponse trop longue (1 000 caractères au plus)");
    }
    if m.chars().any(|c| c.is_control() && c != '\n') {
        return Err("caractères interdits dans la réponse");
    }
    Ok(m.to_string())
}

/// Bornes d'une simulation : barème de 1 à 1 000 points, note entre 0 et le barème, coefficient de 0,001 à 100.
/// Rend `(valeur_m, bareme_m, coef_m)`.
pub fn controler_simulation(b: &SimulationBody) -> Result<(i64, i64, i64), &'static str> {
    let bareme = b.bareme_m.unwrap_or(20_000);
    let coef = b.coef_m.unwrap_or(1_000);
    if !(1_000..=1_000_000).contains(&bareme) {
        return Err("barème invalide (de 1 à 1 000 points)");
    }
    if !(0..=bareme).contains(&b.valeur_m) {
        return Err("la note imaginée doit être comprise entre 0 et le barème");
    }
    if !(1..=100_000).contains(&coef) {
        return Err("coefficient invalide (de 0,001 à 100)");
    }
    Ok((b.valeur_m, bareme, coef))
}

/// Libellé d'une dispense pour la famille (jamais le motif).
pub fn libelle_dispense(nature: &str) -> &'static str {
    match nature {
        "dispense" => "dispensé de cet enseignement",
        "inaptitude_totale" => "inapte : non évalué",
        "inaptitude_partielle" => "inapte en partie : évaluation adaptée",
        _ => "disposition particulière",
    }
}

/// Mois en toutes lettres.
fn mois(m: u32) -> &'static str {
    [
        "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre",
        "novembre", "décembre",
    ]
    .get(m.saturating_sub(1) as usize)
    .copied()
    .unwrap_or("")
}

/// « 5 décembre 2031 ».
pub fn date_fr(d: NaiveDate) -> String {
    use chrono::Datelike;
    let jour = if d.day() == 1 { "1er".to_string() } else { d.day().to_string() };
    format!("{jour} {} {}", mois(d.month()), d.year())
}

/// Phrase « provisoire » du relevé (§10.1).
pub fn mention_provisoire(date_arret: Option<NaiveDate>) -> String {
    match date_arret {
        Some(d) => format!("Moyennes provisoires, susceptibles d'évoluer jusqu'à l'arrêt des notes du {}.", date_fr(d)),
        None => "Moyennes provisoires, susceptibles d'évoluer jusqu'à l'arrêt des notes.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    #[test]
    fn message_de_revision_borne_et_nettoye() {
        assert_eq!(controler_message_revision("  Je crois qu'un exercice a été oublié.  ").unwrap(), "Je crois qu'un exercice a été oublié.");
        assert!(controler_message_revision("ok").is_err());
        assert!(controler_message_revision(&"a".repeat(1001)).is_err());
        assert!(controler_message_revision("bonjour\u{7}").is_err());
        assert!(controler_message_revision("ligne 1\nligne 2").is_ok());
        assert!(controler_reponse_revision("   ").is_err());
        assert!(controler_reponse_revision("Vu, la note est juste.").is_ok());
    }

    #[test]
    fn simulation_bornee() {
        let b = |valeur_m, bareme_m, coef_m| SimulationBody {
            periode_id: Uuid::nil(),
            matiere_id: Uuid::nil(),
            valeur_m,
            bareme_m,
            coef_m,
        };
        assert_eq!(controler_simulation(&b(12_000, None, None)), Ok((12_000, 20_000, 1_000)));
        assert!(controler_simulation(&b(21_000, None, None)).is_err());
        assert!(controler_simulation(&b(-1, None, None)).is_err());
        assert!(controler_simulation(&b(5_000, Some(0), None)).is_err());
        assert!(controler_simulation(&b(5_000, Some(10_000), Some(0))).is_err());
        assert_eq!(controler_simulation(&b(8_000, Some(10_000), Some(2_000))), Ok((8_000, 10_000, 2_000)));
    }

    #[test]
    fn dates_et_mentions_en_francais() {
        assert_eq!(date_fr(d(2031, 12, 5)), "5 décembre 2031");
        assert_eq!(date_fr(d(2032, 3, 1)), "1er mars 2032");
        assert!(mention_provisoire(Some(d(2031, 12, 5))).contains("arrêt des notes du 5 décembre 2031"));
        assert!(mention_provisoire(None).starts_with("Moyennes provisoires"));
    }

    #[test]
    fn glossaire_complet_et_sans_doublon() {
        let g = glossaire();
        let mut termes: Vec<&str> = g.iter().map(|x| x.terme.as_str()).collect();
        termes.sort_unstable();
        termes.dedup();
        assert_eq!(termes.len(), g.len());
        for cat in ["statut", "mention", "statistique"] {
            assert!(g.iter().any(|x| x.categorie == cat), "{cat}");
        }
        assert!(g.iter().any(|x| x.terme == "Absent" && x.definition.contains("jamais zéro")));
        assert_eq!(libelle_dispense("inconnu"), "disposition particulière");
    }
}
