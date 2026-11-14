//! Socle 2 (l'élève), lot E6 — détection de doublons d'élèves (ELV-22, ELV-33).
//!
//! Logique **pure** : deux identités en entrée, un score et des raisons en sortie. La base
//! propose les paires candidates (clé de recherche trigramme, INE identique, même date de
//! naissance) ; ce module décide si la paire mérite l'attention de la direction.
//!
//! Principes :
//! * un **INE identique** est le signal le plus fort (c'est le déclencheur habituel d'un doublon,
//!   cf. §7.3 : une collision d'INE n'échoue pas en silence) ;
//! * deux INE **différents** n'excluent pas un doublon (un INE mal recopié existe) mais baissent
//!   le score ;
//! * **jumeaux** : même nom, même date de naissance, prénoms différents → jamais proposés ;
//! * une date de naissance **inconnue, imprécise ou contestée** ne confirme ni n'infirme ;
//! * le seuil d'inscription est volontairement prudent : mieux vaut une fausse alerte qu'une
//!   fiche dupliquée, et ELV-33 permet de mémoriser « ce sont bien deux élèves ».

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Score minimal pour inscrire une paire dans la liste des doublons probables.
pub const SEUIL_DOUBLON: u8 = 60;

/// Ce que la détection sait d'un élève (désignation, état civil, INE actif).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentiteDoublon {
    pub id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub nom_naissance: String,
    pub prenoms: Vec<String>,
    pub date_naissance: Option<NaiveDate>,
    /// `jour`, `mois`, `annee` ou `inconnue` (R-E1).
    pub precision: String,
    /// `declaree`, `justifiee`, `presumee` ou `contestee` (R-E1).
    pub statut_date: String,
    pub ine: Option<String>,
}

/// Pourquoi une paire est proposée (codes stables, affichés par l'écran).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RaisonDoublon {
    IneIdentique,
    IneDifferent,
    IdentiteExacte,
    NomPrenomExacts,
    NomPrenomProches,
    NomEtPrenomInverses,
    MemeNaissance,
    NaissanceProche,
    NaissanceInconnue,
}

impl RaisonDoublon {
    pub fn code(self) -> &'static str {
        match self {
            Self::IneIdentique => "ine_identique",
            Self::IneDifferent => "ine_different",
            Self::IdentiteExacte => "identite_exacte",
            Self::NomPrenomExacts => "nom_prenom_exacts",
            Self::NomPrenomProches => "nom_prenom_proches",
            Self::NomEtPrenomInverses => "nom_et_prenom_inverses",
            Self::MemeNaissance => "meme_naissance",
            Self::NaissanceProche => "naissance_proche",
            Self::NaissanceInconnue => "naissance_inconnue",
        }
    }
}

/// Résultat de la comparaison d'une paire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreDoublon {
    pub score: u8,
    pub raisons: Vec<RaisonDoublon>,
}

/// Clé de comparaison d'un nom ou d'un prénom : minuscules, sans accents, ponctuation et
/// traits d'union réduits à une espace. Même esprit que `ent_cle_recherche` côté SQL (qui reste
/// la référence de la recherche) ; ici elle ne sert qu'à comparer deux valeurs entre elles.
pub fn cle_nom(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => out.push('a'),
            'é' | 'è' | 'ê' | 'ë' => out.push('e'),
            'î' | 'ï' | 'í' | 'ì' => out.push('i'),
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => out.push('o'),
            'ù' | 'û' | 'ü' | 'ú' => out.push('u'),
            'ç' => out.push('c'),
            'ÿ' | 'ý' => out.push('y'),
            'ñ' => out.push('n'),
            'œ' => out.push_str("oe"),
            'æ' => out.push_str("ae"),
            'ß' => out.push_str("ss"),
            c if c.is_alphanumeric() => out.push(c),
            _ => out.push(' '),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Distance d'édition (Levenshtein), bornée : renvoie `None` au-delà de `max`.
fn distance_bornee(a: &str, b: &str, max: usize) -> Option<usize> {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.len().abs_diff(b.len()) > max {
        return None;
    }
    let mut prec: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cour = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cout = usize::from(ca != cb);
            cour.push((prec[j] + cout).min(prec[j + 1] + 1).min(cour[j] + 1));
        }
        prec = cour;
    }
    let d = prec[b.len()];
    (d <= max).then_some(d)
}

/// Écart toléré selon la longueur : aucun pour un mot très court (Léa ≠ Léo, Noa ≠ Noé), une
/// faute pour un mot moyen (Yanis / Yannis), deux pour un mot long (Mohamed / Mohammed).
fn tolerance(a: &str, b: &str) -> usize {
    match a.chars().count().min(b.chars().count()) {
        0..=3 => 0,
        4..=7 => 1,
        _ => 2,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ressemblance {
    Non,
    Proche,
    Exact,
}

fn compare(a: &str, b: &str) -> Ressemblance {
    if a.is_empty() || b.is_empty() {
        return Ressemblance::Non;
    }
    if a == b {
        return Ressemblance::Exact;
    }
    match distance_bornee(a, b, tolerance(a, b)) {
        Some(_) => Ressemblance::Proche,
        None => Ressemblance::Non,
    }
}

/// Meilleure ressemblance entre deux ensembles de valeurs (nom d'usage / de naissance, prénom
/// affiché / prénoms d'état civil).
fn meilleure(a: &[String], b: &[String]) -> Ressemblance {
    let mut m = Ressemblance::Non;
    for x in a {
        for y in b {
            match compare(x, y) {
                Ressemblance::Exact => return Ressemblance::Exact,
                Ressemblance::Proche => m = Ressemblance::Proche,
                Ressemblance::Non => {}
            }
        }
    }
    m
}

fn noms(i: &IdentiteDoublon) -> Vec<String> {
    let mut v = vec![cle_nom(&i.nom), cle_nom(&i.nom_naissance)];
    v.retain(|s| !s.is_empty());
    v.sort();
    v.dedup();
    v
}

fn prenoms(i: &IdentiteDoublon) -> Vec<String> {
    let mut v = vec![cle_nom(&i.prenom)];
    v.extend(i.prenoms.iter().map(|p| cle_nom(p)));
    // Le premier prénom d'un prénom composé : « Jean Pierre » ~ « Jean ».
    let premiers: Vec<String> = v
        .iter()
        .filter_map(|p| p.split(' ').next().map(str::to_string))
        .filter(|p| !p.is_empty())
        .collect();
    v.extend(premiers);
    v.retain(|s| !s.is_empty());
    v.sort();
    v.dedup();
    v
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Naissance {
    Egale,
    /// Jour et mois permutés, ou même année avec précision réduite.
    Proche,
    /// Au moins une des deux dates ne dit rien (inconnue, imprécise, contestée).
    Inconnue,
    Differente,
}

fn utilisable(i: &IdentiteDoublon) -> Option<NaiveDate> {
    if i.precision == "jour" && i.statut_date != "contestee" {
        i.date_naissance
    } else {
        None
    }
}

fn naissance(a: &IdentiteDoublon, b: &IdentiteDoublon) -> Naissance {
    match (utilisable(a), utilisable(b)) {
        (Some(x), Some(y)) if x == y => Naissance::Egale,
        (Some(x), Some(y)) => {
            // 03/04 saisi 04/03 : permutation jour / mois (cas fréquent d'une saisie à l'étranger).
            if x.year() == y.year() && x.day() == y.month() && x.month() == y.day() {
                Naissance::Proche
            } else {
                Naissance::Differente
            }
        }
        _ => {
            // Une année seule n'infirme pas : elle doit seulement ne pas contredire.
            match (a.date_naissance, b.date_naissance) {
                (Some(x), Some(y))
                    if (a.precision == "annee" || b.precision == "annee")
                        && a.statut_date != "contestee"
                        && b.statut_date != "contestee"
                        && x.year() != y.year() =>
                {
                    Naissance::Differente
                }
                _ => Naissance::Inconnue,
            }
        }
    }
}

fn ine_normalise(i: &IdentiteDoublon) -> Option<String> {
    i.ine
        .as_ref()
        .map(|s| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_uppercase())
        .filter(|s| !s.is_empty())
}

/// Compare deux élèves ; `None` si la paire ne mérite pas d'être proposée (score sous le seuil
/// ou paire évidemment distincte).
pub fn evaluer(a: &IdentiteDoublon, b: &IdentiteDoublon) -> Option<ScoreDoublon> {
    if a.id == b.id {
        return None;
    }
    let mut raisons = Vec::new();
    let (na, nb) = (noms(a), noms(b));
    let (pa, pb) = (prenoms(a), prenoms(b));
    let nom = meilleure(&na, &nb);
    let prenom = meilleure(&pa, &pb);
    // Nom et prénom inversés (saisie « Jean Dupont » / « Dupont Jean »).
    let inverses = meilleure(&na, &pb) != Ressemblance::Non
        && meilleure(&pa, &nb) != Ressemblance::Non
        && !(nom != Ressemblance::Non && prenom != Ressemblance::Non);
    let naiss = naissance(a, b);

    let ine_a = ine_normalise(a);
    let ine_b = ine_normalise(b);
    let ine = match (&ine_a, &ine_b) {
        (Some(x), Some(y)) if x == y => Some(true),
        (Some(_), Some(_)) => Some(false),
        _ => None,
    };

    let identite_exacte = nom == Ressemblance::Exact && prenom == Ressemblance::Exact;
    let identite_proche = nom != Ressemblance::Non && prenom != Ressemblance::Non;

    let mut score: i32 = if ine == Some(true) {
        raisons.push(RaisonDoublon::IneIdentique);
        if identite_exacte && naiss == Naissance::Egale {
            100
        } else {
            92
        }
    } else if identite_exacte && naiss == Naissance::Egale {
        raisons.push(RaisonDoublon::IdentiteExacte);
        100
    } else if inverses && naiss == Naissance::Egale {
        raisons.push(RaisonDoublon::NomEtPrenomInverses);
        90
    } else if identite_proche && naiss == Naissance::Egale {
        raisons.push(RaisonDoublon::NomPrenomProches);
        85
    } else if identite_exacte && naiss == Naissance::Proche {
        raisons.push(RaisonDoublon::NomPrenomExacts);
        75
    } else if identite_exacte && naiss == Naissance::Inconnue {
        raisons.push(RaisonDoublon::NomPrenomExacts);
        70
    } else if identite_proche && naiss == Naissance::Inconnue {
        raisons.push(RaisonDoublon::NomPrenomProches);
        62
    } else {
        // Même nom, prénoms différents (jumeaux, fratrie), ou même identité à une autre date de
        // naissance (homonymes) : ce ne sont pas des doublons.
        return None;
    };
    match naiss {
        Naissance::Egale => raisons.push(RaisonDoublon::MemeNaissance),
        Naissance::Proche => raisons.push(RaisonDoublon::NaissanceProche),
        Naissance::Inconnue => raisons.push(RaisonDoublon::NaissanceInconnue),
        Naissance::Differente => {}
    }
    if ine == Some(false) {
        raisons.push(RaisonDoublon::IneDifferent);
        score -= 15;
    }
    let score = score.clamp(0, 100) as u8;
    (score >= SEUIL_DOUBLON).then_some(ScoreDoublon { score, raisons })
}

/// Ordre canonique d'une paire (`a < b`) : une paire n'existe qu'une fois.
pub fn paire_ordonnee(x: Uuid, y: Uuid) -> (Uuid, Uuid) {
    if x < y {
        (x, y)
    } else {
        (y, x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el(nom: &str, prenom: &str, naiss: Option<(i32, u32, u32)>, ine: Option<&str>) -> IdentiteDoublon {
        IdentiteDoublon {
            id: Uuid::new_v4(),
            nom: nom.into(),
            prenom: prenom.into(),
            nom_naissance: nom.into(),
            prenoms: vec![prenom.into()],
            date_naissance: naiss.map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).unwrap()),
            precision: if naiss.is_some() { "jour" } else { "inconnue" }.into(),
            statut_date: "declaree".into(),
            ine: ine.map(str::to_string),
        }
    }

    #[test]
    fn cle_nom_accents_majuscules_ligatures() {
        assert_eq!(cle_nom("ÉLODIE"), "elodie");
        assert_eq!(cle_nom("  Jean-Pierre  D'Arc "), "jean pierre d arc");
        assert_eq!(cle_nom("Œuvre"), "oeuvre");
        assert_eq!(cle_nom("Strauß"), "strauss");
    }

    #[test]
    fn identite_exacte_meme_naissance_vaut_cent() {
        let a = el("Dupont", "Jean", Some((2012, 3, 4)), None);
        let b = el("DUPONT", "jean", Some((2012, 3, 4)), None);
        let s = evaluer(&a, &b).unwrap();
        assert_eq!(s.score, 100);
        assert!(s.raisons.contains(&RaisonDoublon::IdentiteExacte));
    }

    #[test]
    fn faute_de_frappe_reste_un_doublon() {
        let a = el("Benali", "Yanis", Some((2012, 3, 4)), None);
        let b = el("Benali", "Yannis", Some((2012, 3, 4)), None);
        assert_eq!(evaluer(&a, &b).unwrap().score, 85);
        let c = el("Mohamed", "Karim", Some((2011, 1, 2)), None);
        let d = el("Mohammed", "Karim", Some((2011, 1, 2)), None);
        assert!(evaluer(&c, &d).is_some());
    }

    #[test]
    fn jumeaux_jamais_proposes() {
        let lea = el("Martin", "Léa", Some((2012, 5, 6)), None);
        let leo = el("Martin", "Léo", Some((2012, 5, 6)), None);
        assert!(evaluer(&lea, &leo).is_none(), "Léa / Léo : prénoms trop courts pour une faute");
        let noa = el("Martin", "Noa", Some((2012, 5, 6)), None);
        let noe = el("Martin", "Noé", Some((2012, 5, 6)), None);
        assert!(evaluer(&noa, &noe).is_none());
        let lucas = el("Martin", "Lucas", Some((2012, 5, 6)), None);
        let lucie = el("Martin", "Lucie", Some((2012, 5, 6)), None);
        assert!(evaluer(&lucas, &lucie).is_none());
    }

    #[test]
    fn homonymes_nes_un_autre_jour_pas_proposes() {
        let a = el("Dupont", "Jean", Some((2012, 3, 4)), None);
        let b = el("Dupont", "Jean", Some((2013, 9, 10)), None);
        assert!(evaluer(&a, &b).is_none());
    }

    #[test]
    fn ine_identique_suffit() {
        let a = el("Dupont", "Jean", Some((2012, 3, 4)), Some("123456789AB"));
        let b = el("Durand", "Paul", None, Some("123456789ab"));
        let s = evaluer(&a, &b).unwrap();
        assert_eq!(s.score, 92);
        assert_eq!(s.raisons[0], RaisonDoublon::IneIdentique);
    }

    #[test]
    fn ine_differents_baissent_le_score_sans_exclure() {
        let a = el("Dupont", "Jean", Some((2012, 3, 4)), Some("123456789AB"));
        let b = el("Dupont", "Jean", Some((2012, 3, 4)), Some("123456789AC"));
        let s = evaluer(&a, &b).unwrap();
        assert_eq!(s.score, 85);
        assert!(s.raisons.contains(&RaisonDoublon::IneDifferent));
    }

    #[test]
    fn nom_et_prenom_inverses() {
        let a = el("Dupont", "Jean", Some((2012, 3, 4)), None);
        let b = el("Jean", "Dupont", Some((2012, 3, 4)), None);
        assert_eq!(evaluer(&a, &b).unwrap().raisons[0], RaisonDoublon::NomEtPrenomInverses);
    }

    #[test]
    fn jour_et_mois_permutes() {
        let a = el("Dupont", "Jean", Some((2012, 3, 4)), None);
        let b = el("Dupont", "Jean", Some((2012, 4, 3)), None);
        let s = evaluer(&a, &b).unwrap();
        assert_eq!(s.score, 75);
        assert!(s.raisons.contains(&RaisonDoublon::NaissanceProche));
    }

    #[test]
    fn naissance_inconnue_ou_contestee_ne_tranche_pas() {
        let a = el("Dupont", "Jean", None, None);
        let b = el("Dupont", "Jean", Some((2012, 3, 4)), None);
        assert_eq!(evaluer(&a, &b).unwrap().score, 70);
        let mut c = el("Dupont", "Jean", Some((2010, 1, 1)), None);
        c.statut_date = "contestee".into();
        assert_eq!(evaluer(&c, &b).unwrap().score, 70, "date contestée = inconnue");
        // Année seule en désaccord : ce sont deux élèves.
        let mut d = el("Dupont", "Jean", Some((2009, 1, 1)), None);
        d.precision = "annee".into();
        assert!(evaluer(&d, &b).is_none());
    }

    #[test]
    fn nom_de_naissance_et_usage() {
        let mut a = el("Martin", "Sam", Some((2008, 2, 2)), None);
        a.nom_naissance = "Dupuis".into();
        a.prenoms = vec!["Samantha".into(), "Sam".into()];
        let mut b = el("Dupuis", "Samantha", Some((2008, 2, 2)), None);
        b.nom_naissance = "Dupuis".into();
        // Le nom d'usage de l'un est le nom de naissance de l'autre.
        assert!(evaluer(&a, &b).is_some());
    }

    #[test]
    fn paire_ordonnee_est_symetrique() {
        let (x, y) = (Uuid::new_v4(), Uuid::new_v4());
        assert_eq!(paire_ordonnee(x, y), paire_ordonnee(y, x));
        let a = el("Dupont", "Jean", Some((2012, 3, 4)), None);
        let b = el("Dupont", "Jean", Some((2012, 3, 4)), None);
        assert_eq!(evaluer(&a, &b).map(|s| s.score), evaluer(&b, &a).map(|s| s.score));
        assert!(evaluer(&a, &a).is_none());
    }
}
