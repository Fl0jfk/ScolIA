//! Normalisation des valeurs comparées par le rapprochement (lot R3). Fonctions **pures**, sans horloge.
//!
//! Même esprit que `domain-eleve::doublon::cle_nom` (socle 2) : la clé ne sert qu'à **comparer** deux
//! valeurs entre elles, jamais à les stocker. Aucune ressemblance floue n'est appliquée ici : la distance
//! d'édition ne sert qu'à *proposer* (jamais à rapprocher d'elle-même).

/// Clé de comparaison d'un nom ou d'un prénom : minuscules, sans accents, ponctuation et traits d'union
/// réduits à une espace.
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

/// Mots d'une clé de nom (déjà passée par [`cle_nom`]).
pub fn mots(cle: &str) -> Vec<&str> {
    cle.split(' ').filter(|m| !m.is_empty()).collect()
}

/// INE normalisé : majuscules, sans espace, 9 à 16 caractères alphanumériques (même règle que
/// `domain-eleve::ine`). `None` si la valeur est vide ou de forme impossible : un INE illisible ne
/// rapproche **jamais** rien.
pub fn cle_ine(brut: &str) -> Option<String> {
    let ine: String = brut
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    ((9..=16).contains(&ine.len()) && ine.chars().all(|c| c.is_ascii_alphanumeric())).then_some(ine)
}

/// Fin d'un INE pour l'affichage minimisé (« …6789A ») : jamais l'INE entier dans la file.
pub fn ine_fin(brut: &str) -> Option<String> {
    let ine = cle_ine(brut)?;
    let n = ine.chars().count();
    Some(format!(
        "…{}",
        ine.chars().skip(n.saturating_sub(4)).collect::<String>()
    ))
}

/// E-mail normalisé : minuscules, sans espaces ; **pas** de suppression des points ni des « + » (T0 §6.6).
/// `None` si la forme est impossible (pas de « @ » unique, partie vide).
pub fn email_normalise(brut: &str) -> Option<String> {
    let e = brut.trim().to_lowercase();
    let (local, domaine) = e.split_once('@')?;
    let ok = !local.is_empty()
        && domaine.contains('.')
        && !domaine.starts_with('.')
        && !domaine.ends_with('.')
        && !domaine.contains('@')
        && !e.chars().any(char::is_whitespace);
    ok.then_some(e)
}

/// E-mail masqué pour l'affichage minimisé (« j***@domaine.fr »).
pub fn email_masque(brut: &str) -> Option<String> {
    let e = email_normalise(brut)?;
    let (local, domaine) = e.split_once('@')?;
    let premier: String = local.chars().take(1).collect();
    Some(format!("{premier}***@{domaine}"))
}

/// Téléphone : les 9 derniers chiffres (indicatif et zéro initial ignorés). `None` en dessous de 9 chiffres.
pub fn cle_telephone(brut: &str) -> Option<String> {
    let chiffres: Vec<char> = brut.chars().filter(char::is_ascii_digit).collect();
    (chiffres.len() >= 9).then(|| chiffres[chiffres.len() - 9..].iter().collect())
}

/// Distance d'édition bornée : `None` au-delà de `max`.
pub fn distance_bornee(a: &str, b: &str, max: usize) -> Option<usize> {
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

/// Niveau canonique d'un mot de libellé de classe (« seconde », « 2de », « 2nd » → « 2nde »).
fn niveau_canonique(mot: &str) -> Option<&'static str> {
    Some(match mot {
        "6e" | "6eme" | "sixieme" => "6e",
        "5e" | "5eme" | "cinquieme" => "5e",
        "4e" | "4eme" | "quatrieme" => "4e",
        "3e" | "3eme" | "troisieme" => "3e",
        "2nde" | "2de" | "2nd" | "seconde" | "2e" => "2nde",
        "1ere" | "1re" | "premiere" | "1e" => "1ere",
        "tle" | "terminale" | "term" | "tl" => "tle",
        _ => return None,
    })
}

/// Clé de comparaison d'un libellé de classe : « 2nde B », « 2de B », « Seconde B », « seconde b »
/// donnent la même clé. Les libellés qui ne portent pas de niveau connu sont comparés tels quels
/// (« 2B » ne devient **pas** « 2nde B » : c'est une proposition de synonyme à faire relire, jamais
/// une déduction).
pub fn cle_classe(brut: &str) -> String {
    let base = cle_nom(brut);
    let mots: Vec<String> = base
        .split(' ')
        .filter(|m| !m.is_empty() && *m != "classe" && *m != "de" && *m != "la")
        .map(|m| niveau_canonique(m).map_or_else(|| m.to_string(), str::to_string))
        .collect();
    mots.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_sans_accents_ni_ponctuation() {
        assert_eq!(cle_nom("  Éloïse-Marie  "), "eloise marie");
        assert_eq!(cle_nom("D'Aubigné"), "d aubigne");
        assert_eq!(mots("jean pierre"), vec!["jean", "pierre"]);
    }

    #[test]
    fn ine_forme_et_masque() {
        assert_eq!(cle_ine(" 1234 567 890a ").as_deref(), Some("1234567890A"));
        assert_eq!(cle_ine("court"), None);
        assert_eq!(cle_ine("12345-67890"), None);
        assert_eq!(cle_ine(""), None);
        assert_eq!(ine_fin("1234567890A").as_deref(), Some("…890A"));
        assert_eq!(ine_fin("n'importe quoi"), None);
    }

    #[test]
    fn emails_conserves_a_l_identique_sauf_casse() {
        assert_eq!(
            email_normalise(" Jean.Dupont+ecole@Example.TEST ").as_deref(),
            Some("jean.dupont+ecole@example.test")
        );
        assert_eq!(email_normalise("pas un mail"), None);
        assert_eq!(email_normalise("a@b"), None);
        assert_eq!(email_normalise("a@@b.fr"), None);
        assert_eq!(
            email_masque("Jean@example.test").as_deref(),
            Some("j***@example.test")
        );
    }

    #[test]
    fn telephones_par_les_neuf_derniers_chiffres() {
        assert_eq!(cle_telephone("06 12 34 56 78"), Some("612345678".into()));
        assert_eq!(cle_telephone("+33 6 12 34 56 78"), Some("612345678".into()));
        assert_eq!(cle_telephone("12 34"), None);
    }

    #[test]
    fn distance_bornee_au_dela_de_la_borne() {
        assert_eq!(distance_bornee("martin", "martim", 1), Some(1));
        assert_eq!(distance_bornee("martin", "dupont", 1), None);
        assert_eq!(distance_bornee("abc", "abc", 1), Some(0));
    }

    #[test]
    fn classes_synonymes_courants_donnent_la_meme_cle() {
        let a = cle_classe("2nde B");
        assert_eq!(a, cle_classe("Seconde B"));
        assert_eq!(a, cle_classe("2de b"));
        assert_eq!(a, cle_classe("classe de 2NDE  B"));
        assert_eq!(cle_classe("Sixième A"), cle_classe("6ème A"));
        assert_eq!(cle_classe("Terminale S1"), cle_classe("Tle S1"));
        // « 2B » n'est jamais déduit : il reste distinct.
        assert_ne!(cle_classe("2B"), a);
        assert_ne!(cle_classe("2nde B"), cle_classe("2nde C"));
    }
}
