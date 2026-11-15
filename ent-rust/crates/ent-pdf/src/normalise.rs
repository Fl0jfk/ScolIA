//! Normalisation NFC maison pour le latin, le grec et le cyrillique : réordonnancement
//! canonique des marques combinantes puis composition d'après la table générée
//! (`compositions.rs`). Sert aussi à retrouver la lettre de base d'un caractère absent de la
//! police (repli lisible).

use crate::compositions::{CCC, COMPOSITIONS};

fn ccc(m: char) -> u8 {
    let u = m as u32;
    if (0x300..0x370).contains(&u) {
        CCC[(u - 0x300) as usize]
    } else {
        0
    }
}

pub(crate) fn est_marque(c: char) -> bool {
    ('\u{300}'..='\u{36f}').contains(&c)
}

fn composer(base: char, marque: char) -> Option<char> {
    let (b, m) = (
        u16::try_from(base as u32).ok()?,
        u16::try_from(marque as u32).ok()?,
    );
    let i = COMPOSITIONS
        .binary_search_by(|&(x, y, _)| (x, y).cmp(&(b, m)))
        .ok()?;
    char::from_u32(COMPOSITIONS[i].2 as u32)
}

/// Décomposition d'une lettre composée : (base, marque).
pub(crate) fn decomposer(c: char) -> Option<(char, char)> {
    let u = u16::try_from(c as u32).ok()?;
    COMPOSITIONS
        .iter()
        .find(|e| e.2 == u)
        .and_then(|e| Some((char::from_u32(e.0 as u32)?, char::from_u32(e.1 as u32)?)))
}

/// Lettre de base la plus simple (accents retirés récursivement).
pub(crate) fn lettre_de_base(c: char) -> char {
    let mut c = c;
    while let Some((b, _)) = decomposer(c) {
        c = b;
    }
    c
}

/// Forme NFC (pour les compositions de la table) : les marques suivant une lettre sont
/// triées par classe de combinaison puis composées quand c'est possible.
pub(crate) fn nfc(s: &str) -> Vec<char> {
    let mut sortie: Vec<char> = Vec::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(base) = it.next() {
        if est_marque(base) {
            sortie.push(base); // marque orpheline : conservée telle quelle
            continue;
        }
        let mut marques: Vec<char> = Vec::new();
        while let Some(&n) = it.peek() {
            if est_marque(n) {
                marques.push(n);
                it.next();
            } else {
                break;
            }
        }
        marques.sort_by_key(|&m| ccc(m)); // tri stable : l'ordre relatif à classe égale est gardé
        let mut starter = base;
        let mut restantes: Vec<char> = Vec::new();
        let mut derniere_classe = 0u8;
        for m in marques {
            let c = ccc(m);
            let bloquee = !restantes.is_empty() && derniere_classe >= c;
            match (bloquee, composer(starter, m)) {
                (false, Some(x)) => starter = x,
                _ => {
                    restantes.push(m);
                    derniere_classe = c;
                }
            }
        }
        sortie.push(starter);
        sortie.extend(restantes);
    }
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: Vec<char>) -> String {
        v.into_iter().collect()
    }

    #[test]
    fn compositions_courantes() {
        assert_eq!(s(nfc("e\u{301}le\u{300}ve")), "élève");
        assert_eq!(s(nfc("c\u{327}a")), "ça");
        assert_eq!(s(nfc("S\u{326}tefan T\u{326}aran")), "Ștefan Țaran");
    }

    #[test]
    fn vietnamien_toutes_les_formes_d_ecriture() {
        // Nguyễn : NFD canonique, ordre inverse des marques, et déjà composé.
        assert_eq!(s(nfc("Nguye\u{302}\u{303}n")), "Nguyễn");
        // Deux marques de même classe ne se réordonnent pas (NFC strict) : ẽ + circonflexe.
        assert_eq!(
            s(nfc("Nguye\u{303}\u{302}n")),
            "Nguye\u{303}\u{302}n".replacen("e\u{303}", "ẽ", 1)
        );
        assert_eq!(s(nfc("Nguyễn")), "Nguyễn");
        // ệ = e + point souscrit + circonflexe, quel que soit l'ordre de saisie.
        assert_eq!(s(nfc("e\u{323}\u{302}")), "ệ");
        assert_eq!(s(nfc("e\u{302}\u{323}")), "ệ");
        // Hằng : a + brève + grave ; Thị : i + point souscrit ; ơ ư (cornes).
        assert_eq!(s(nfc("Ha\u{306}\u{300}ng")), "Hằng");
        assert_eq!(s(nfc("Thi\u{323}")), "Thị");
        assert_eq!(s(nfc("o\u{31b}\u{301}")), "ớ");
    }

    #[test]
    fn grec_cyrillique_et_autres_latins() {
        assert_eq!(s(nfc("\u{3b9}\u{308}")), "ϊ");
        assert_eq!(s(nfc("\u{3b5}\u{301}")), "έ");
        assert_eq!(s(nfc("\u{438}\u{306}")), "й");
        assert_eq!(s(nfc("S\u{327}u\u{308}kru\u{308}")), "Şükrü");
        assert_eq!(s(nfc("L\u{b7}")), "L\u{b7}");
    }

    #[test]
    fn base_et_decomposition() {
        assert_eq!(lettre_de_base('ễ'), 'e');
        assert_eq!(lettre_de_base('Ș'), 'S');
        assert_eq!(decomposer('é'), Some(('e', '\u{301}')));
        assert_eq!(decomposer('z'), None);
    }

    #[test]
    fn marque_sans_composition_reste() {
        assert_eq!(s(nfc("x\u{301}")), "x\u{301}");
        assert_eq!(s(nfc("\u{301}")), "\u{301}");
    }
}
