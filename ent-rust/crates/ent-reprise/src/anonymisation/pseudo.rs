//! Fabrication des pseudonymes : **déterministes** (même valeur réelle + même clé = même pseudonyme,
//! donc les liens et doublons restent visibles) et **fictifs** (jamais une vraie personne).
//!
//! Les noms sont composés de syllabes inventées ; les e-mails sont en `example.test` ; les numéros
//! de téléphone sont dans les plages fictives réservées par l'ARCEP ; les adresses IP dans les
//! blocs de documentation (RFC 5737) ; les IBAN sont structurellement valides mais fictifs.

use super::cle::{Cle, Flux};
use crate::schema::Genre;
use chrono::{Duration, NaiveDate};

const SYLLABES: &[&str] = &[
    "ka", "lo", "mi", "ra", "tu", "ve", "zo", "ne", "pa", "su", "bi", "do", "fe", "gu", "ha", "jo",
    "li", "mo", "na", "po", "re", "sa", "ti", "vu", "wa", "xe", "yo", "za", "be", "co", "du", "fi",
    "go", "hu", "ji",
];
const FINALES_NOM: &[&str] = &[
    "ec", "ard", "ier", "on", "ault", "oux", "ac", "et", "ens", "ois", "ange", "ur",
];
const FINALES_PRENOM: &[&str] = &["a", "o", "e", "ia", "ou", "el", "an", "ie", "is", "yn"];

/// PNG 1×1 transparent (base64) : image neutre.
pub const PNG_NEUTRE: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";
/// PDF minimal (base64) : document neutre.
pub const PDF_NEUTRE: &str = "JVBERi0xLjEKMSAwIG9iajw8L1R5cGUvQ2F0YWxvZy9QYWdlcyAyIDAgUj4+ZW5kb2JqCjIgMCBvYmo8PC9UeXBlL1BhZ2VzL0tpZHNbMyAwIFJdL0NvdW50IDE+PmVuZG9iagozIDAgb2JqPDwvVHlwZS9QYWdlL1BhcmVudCAyIDAgUi9NZWRpYUJveFswIDAgMjAwIDIwMF0+PmVuZG9iagp0cmFpbGVyPDwvUm9vdCAxIDAgUj4+CiUlRU9GCg==";

fn mot(f: &mut Flux, finales: &[&str]) -> String {
    let n = 2 + f.tirer(2); // 2 ou 3 syllabes
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(SYLLABES[f.tirer(SYLLABES.len())]);
    }
    s.push_str(finales[f.tirer(finales.len())]);
    s
}

fn capitaliser(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(p) => p.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Reproduit la casse du jeton d'origine (MAJUSCULES, minuscules, Capitalisé).
fn casse_comme(modele: &str, s: &str) -> String {
    let lettres: Vec<char> = modele.chars().filter(|c| c.is_alphabetic()).collect();
    if lettres.len() > 1 && lettres.iter().all(|c| c.is_uppercase()) {
        s.to_uppercase()
    } else if !lettres.is_empty() && lettres.iter().all(|c| c.is_lowercase()) {
        s.to_lowercase()
    } else {
        capitaliser(s)
    }
}

fn nom_famille(f: &mut Flux, modele: &str) -> String {
    casse_comme(modele, &capitaliser(&mot(f, FINALES_NOM)))
}

fn prenom(f: &mut Flux, modele: &str) -> String {
    casse_comme(modele, &capitaliser(&mot(f, FINALES_PRENOM)))
}

fn nom_complet(cle: &Cle, v: &str) -> String {
    let mut sortie: Vec<String> = Vec::new();
    for (i, jeton) in v.split_whitespace().enumerate() {
        // Particules et initiales : conservées (« de », « du », « J. »).
        let lettres = jeton.chars().filter(|c| c.is_alphabetic()).count();
        if lettres <= 1 || (jeton.chars().all(|c| c.is_lowercase() || c == '\'') && lettres <= 3) {
            sortie.push(jeton.to_string());
            continue;
        }
        let etiquette = format!("nom_complet/{i}");
        let mut f = Flux::nouveau(cle, &etiquette, v);
        // Composé avec trait d'union : chaque partie est remplacée.
        let parties: Vec<String> = jeton
            .split('-')
            .map(|p| {
                if i == 0 {
                    prenom(&mut f, p)
                } else {
                    nom_famille(&mut f, p)
                }
            })
            .collect();
        sortie.push(parties.join("-"));
    }
    if sortie.is_empty() {
        return v.to_string();
    }
    sortie.join(" ")
}

const DOMAINES_PERSO: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "hotmail.com",
    "hotmail.fr",
    "outlook.com",
    "outlook.fr",
    "live.fr",
    "live.com",
    "msn.com",
    "yahoo.com",
    "yahoo.fr",
    "orange.fr",
    "wanadoo.fr",
    "free.fr",
    "sfr.fr",
    "neuf.fr",
    "laposte.net",
    "bbox.fr",
    "icloud.com",
    "me.com",
    "aol.com",
    "gmx.fr",
    "gmx.com",
    "proton.me",
    "protonmail.com",
    "numericable.fr",
    "club-internet.fr",
    "voila.fr",
    "cegetel.net",
    "aliceadsl.fr",
    "yahoo.co.uk",
];

/// Catégorie du domaine : `ac` (académique et public), `perso`, `entreprise`.
pub fn categorie_domaine(domaine: &str) -> &'static str {
    let d = domaine.trim().to_ascii_lowercase();
    if d.ends_with(".gouv.fr")
        || d.starts_with("ac-")
        || d.contains(".ac-")
        || d.ends_with(".education.fr")
        || d == "education.gouv.fr"
    {
        "ac"
    } else if DOMAINES_PERSO.contains(&d.as_str()) {
        "perso"
    } else {
        "entreprise"
    }
}

fn email(cle: &Cle, v: &str) -> String {
    let (local, domaine) = v.rsplit_once('@').unwrap_or((v, ""));
    let minuscule = v.to_ascii_lowercase();
    let mut f = Flux::nouveau(cle, "email", &minuscule);
    let p = mot(&mut f, FINALES_PRENOM).to_lowercase();
    let n = mot(&mut f, FINALES_NOM).to_lowercase();
    let h: String = (0..4).map(|_| f.chiffre()).collect();
    let _ = local;
    format!("{p}.{n}.{h}@{}.example.test", categorie_domaine(domaine))
}

fn telephone(cle: &Cle, v: &str) -> String {
    let chiffres: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
    if chiffres.len() < 6 {
        return v
            .chars()
            .map(|c| if c.is_ascii_digit() { '0' } else { c })
            .collect();
    }
    let national: String = if let Some(r) = chiffres.strip_prefix("0033") {
        format!("0{r}")
    } else if let Some(r) = chiffres
        .strip_prefix("33")
        .filter(|_| v.trim_start().starts_with('+') || chiffres.len() == 11)
    {
        format!("0{r}")
    } else {
        chiffres.clone()
    };
    let mobile = national.starts_with("06") || national.starts_with("07");
    let mut f = Flux::nouveau(cle, "telephone", &chiffres);
    let fin: String = (0..4).map(|_| f.chiffre()).collect();
    // Plages fictives de l'ARCEP : 06 39 98 xx xx (mobile), 01 99 00 xx xx (fixe).
    let fictif = if mobile {
        format!("063998{fin}")
    } else {
        format!("019900{fin}")
    };
    let international = v.trim_start().starts_with('+') || chiffres.starts_with("0033");
    let sep = if v.contains('.') {
        "."
    } else if v.contains(' ') {
        " "
    } else if v.contains('-') {
        "-"
    } else {
        ""
    };
    let groupes = |s: &str| -> String {
        s.as_bytes()
            .chunks(2)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect::<Vec<_>>()
            .join(sep)
    };
    if international {
        let sans_zero = &fictif[1..];
        let pre = if v.trim_start().starts_with("00") {
            "0033"
        } else {
            "+33"
        };
        format!("{pre}{}{}", if sep.is_empty() { "" } else { " " }, {
            let (a, b) = sans_zero.split_at(1);
            format!("{a}{sep}{}", groupes(b))
        })
    } else {
        groupes(&fictif)
    }
}

/// Même forme : un chiffre reste un chiffre, une lettre reste une lettre (même casse).
fn meme_forme(cle: &Cle, etiquette: &str, v: &str) -> String {
    let mut f = Flux::nouveau(cle, etiquette, v);
    v.chars()
        .map(|c| {
            if c.is_ascii_digit() {
                f.chiffre()
            } else if c.is_ascii_uppercase() {
                f.lettre_minuscule().to_ascii_uppercase()
            } else if c.is_alphabetic() {
                f.lettre_minuscule()
            } else {
                c
            }
        })
        .collect()
}

fn mod97(s: &str) -> u32 {
    let mut r = 0u32;
    for c in s.chars() {
        if let Some(d) = c.to_digit(36) {
            if d < 10 {
                r = (r * 10 + d) % 97;
            } else {
                r = (r * 100 + d) % 97;
            }
        }
    }
    r
}

fn iban(cle: &Cle, v: &str) -> String {
    let compact: String = v
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    let pays: String =
        if compact.len() >= 4 && compact.chars().take(2).all(|c| c.is_ascii_uppercase()) {
            compact[..2].to_string()
        } else {
            "FR".to_string()
        };
    let longueur = compact.len().clamp(15, 34);
    let mut f = Flux::nouveau(cle, "iban", &compact);
    let bban: String = (0..longueur - 4).map(|_| f.chiffre()).collect();
    let controle = 98 - mod97(&format!("{bban}{pays}00"));
    let r = format!("{pays}{controle:02}{bban}");
    if v.contains(' ') {
        r.as_bytes()
            .chunks(4)
            .map(|c| std::str::from_utf8(c).unwrap_or(""))
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        r
    }
}

fn bic(cle: &Cle, v: &str) -> String {
    let compact = v.trim().to_ascii_uppercase();
    let pays: String =
        if compact.len() >= 6 && compact[4..6].chars().all(|c| c.is_ascii_uppercase()) {
            compact[4..6].to_string()
        } else {
            "FR".to_string()
        };
    let mut f = Flux::nouveau(cle, "bic", &compact);
    let banque: String = (0..4)
        .map(|_| f.lettre_minuscule().to_ascii_uppercase())
        .collect();
    let lieu: String = (0..2)
        .map(|_| f.lettre_minuscule().to_ascii_uppercase())
        .collect();
    let mut r = format!("{banque}{pays}{lieu}");
    if compact.len() == 11 {
        r.push_str("XXX");
    }
    r
}

/// Décale une date de 1 à 60 jours (avant ou après), dans le même format. Texte non daté : même forme.
fn date_decalee(cle: &Cle, v: &str) -> String {
    let mut f = Flux::nouveau(cle, "date", v);
    let jours = 1 + f.tirer(60) as i64;
    let delta = if f.tirer(2) == 0 { jours } else { -jours };
    // AAAA-MM-JJ (éventuellement suivi d'une heure).
    if v.len() >= 10 {
        if let Ok(d) = NaiveDate::parse_from_str(&v[..10], "%Y-%m-%d") {
            if let Some(n) = d.checked_add_signed(Duration::days(delta)) {
                return format!("{}{}", n.format("%Y-%m-%d"), &v[10..]);
            }
        }
        if let Ok(d) = NaiveDate::parse_from_str(&v[..10], "%d/%m/%Y") {
            if let Some(n) = d.checked_add_signed(Duration::days(delta)) {
                return format!("{}{}", n.format("%d/%m/%Y"), &v[10..]);
            }
        }
    }
    meme_forme(cle, "date/forme", v)
}

fn adresse(cle: &Cle, v: &str) -> String {
    let mut f = Flux::nouveau(cle, "adresse", v);
    let n = 1 + f.tirer(98);
    let rue = capitaliser(&mot(&mut f, FINALES_NOM));
    format!("{n} rue {rue}")
}

fn code_postal(cle: &Cle, v: &str) -> String {
    let t = v.trim();
    if t.len() == 5 && t.chars().all(|c| c.is_ascii_digit()) {
        let mut f = Flux::nouveau(cle, "code_postal", t);
        let fin: String = (0..3).map(|_| f.chiffre()).collect();
        format!("{}{fin}", &t[..2])
    } else {
        meme_forme(cle, "code_postal", v)
    }
}

fn ip(cle: &Cle, v: &str) -> String {
    let mut f = Flux::nouveau(cle, "ip", v);
    if v.contains(':') {
        return format!("2001:db8::{:x}", f.tirer(0xffff));
    }
    let bloc = ["192.0.2", "198.51.100", "203.0.113"][f.tirer(3)];
    format!("{bloc}.{}", 1 + f.tirer(253))
}

/// Identifiant externe : garde le préfixe jusqu'au premier `_`, remplace le reste à l'identique.
fn identifiant(cle: &Cle, v: &str) -> String {
    match v.split_once('_') {
        Some((p, r)) if !p.is_empty() && p.len() <= 12 => {
            format!("{p}_{}", meme_forme(cle, "identifiant", r))
        }
        _ => meme_forme(cle, "identifiant", v),
    }
}

/// Vrai si `s` est un UUID.
pub fn est_uuid(s: &str) -> bool {
    s.len() == 36
        && s.chars().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

/// Vrai si `s` ressemble à un identifiant technique opaque : UUID, hexadécimal long, ou jeton
/// aléatoire (sans trait d'union) mêlant lettres et chiffres. Un « slug » (`jean-dupont-2`) ne l'est
/// **jamais** : il peut contenir un nom.
pub fn est_identifiant_opaque(s: &str) -> bool {
    if est_uuid(s) {
        return true;
    }
    let n = s.chars().count();
    if n < 16 {
        return false;
    }
    if s.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }
    if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }
    let chiffres = s.chars().filter(|c| c.is_ascii_digit()).count();
    let lettres = s.chars().filter(|c| c.is_ascii_alphabetic()).count();
    let majuscule = s.chars().any(|c| c.is_ascii_uppercase());
    chiffres >= 3 && lettres >= 3 && (majuscule || chiffres * 4 >= n)
}

fn id_souple(cle: &Cle, v: &str) -> String {
    if est_identifiant_opaque(v) {
        return v.to_string();
    }
    meme_forme(cle, "id_souple", v)
}

fn jeton(cle: &Cle, v: &str) -> String {
    let n = v.chars().count().max(16);
    let mut f = Flux::nouveau(cle, "jeton", v);
    (0..n)
        .map(|_| char::from_digit(f.tirer(16) as u32, 16).unwrap_or('0'))
        .collect()
}

fn nom_fichier(cle: &Cle, v: &str) -> String {
    let ext = v
        .rsplit_once('.')
        .map(|(_, e)| e)
        .filter(|e| !e.is_empty() && e.len() <= 8 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(|e| e.to_ascii_lowercase());
    let mut f = Flux::nouveau(cle, "nom_fichier", v);
    let h: String = (0..8)
        .map(|_| char::from_digit(f.tirer(16) as u32, 16).unwrap_or('0'))
        .collect();
    match ext {
        Some(e) => format!("document-{h}.{e}"),
        None => format!("document-{h}"),
    }
}

/// Texte libre : **même longueur et même mise en forme**, mots remplacés (lettres → lettres,
/// chiffres → chiffres, ponctuation et espaces gardés).
pub fn texte(cle: &Cle, v: &str) -> String {
    let mut f = Flux::nouveau(cle, "texte", v);
    v.chars()
        .map(|c| {
            if c.is_ascii_digit() {
                f.chiffre()
            } else if c.is_alphabetic() {
                let l = f.lettre_minuscule();
                if c.is_uppercase() {
                    l.to_ascii_uppercase()
                } else {
                    l
                }
            } else {
                c
            }
        })
        .collect()
}

/// Pseudonyme de `v` pour la nature `genre`. `v` vide reste vide (le NULL est géré par l'appelant).
pub fn pseudonymiser(cle: &Cle, genre: Genre, v: &str) -> String {
    if v.is_empty() {
        return String::new();
    }
    match genre {
        Genre::Nom => {
            let mut f = Flux::nouveau(cle, "nom", v);
            v.split('-')
                .map(|p| nom_famille(&mut f, p))
                .collect::<Vec<_>>()
                .join("-")
        }
        Genre::Prenom => {
            let mut f = Flux::nouveau(cle, "prenom", v);
            v.split('-')
                .map(|p| prenom(&mut f, p))
                .collect::<Vec<_>>()
                .join("-")
        }
        Genre::NomComplet => nom_complet(cle, v),
        Genre::Email => {
            if v.contains('@') {
                email(cle, v)
            } else {
                texte(cle, v)
            }
        }
        Genre::Telephone => telephone(cle, v),
        Genre::Ine => meme_forme(cle, "ine", v),
        Genre::Iban => iban(cle, v),
        Genre::Bic => bic(cle, v),
        Genre::Rum => meme_forme(cle, "rum", v),
        Genre::DateNaissance => date_decalee(cle, v),
        Genre::Ville => {
            let mut f = Flux::nouveau(cle, "ville", v);
            capitaliser(&mot(&mut f, FINALES_NOM))
        }
        Genre::Adresse => adresse(cle, v),
        Genre::CodePostal => code_postal(cle, v),
        Genre::Ip => ip(cle, v),
        Genre::Agent => "Mozilla/5.0 (anonymisé)".to_string(),
        Genre::Identifiant => identifiant(cle, v),
        Genre::IdSouple => id_souple(cle, v),
        Genre::Jeton => jeton(cle, v),
        Genre::Image => PNG_NEUTRE.to_string(),
        Genre::Pdf => PDF_NEUTRE.to_string(),
        Genre::NomFichier => nom_fichier(cle, v),
        Genre::Texte => texte(cle, v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cle() -> Cle {
        Cle::depuis_texte(&"ab".repeat(32)).unwrap()
    }

    #[test]
    fn deterministe_et_different_de_l_original() {
        let c = cle();
        for (g, v) in [
            (Genre::Nom, "Dupont"),
            (Genre::Prenom, "Marie-Claire"),
            (Genre::NomComplet, "Jean DUPONT"),
            (Genre::Email, "marie.dupont@gmail.com"),
            (Genre::Telephone, "06 12 34 56 78"),
            (Genre::Ine, "123456789AB"),
            (Genre::Iban, "FR76 3000 6000 0112 3456 7890 189"),
            (Genre::DateNaissance, "2010-03-14"),
            (Genre::Adresse, "12 rue Victor Hugo"),
        ] {
            let a = pseudonymiser(&c, g, v);
            assert_eq!(a, pseudonymiser(&c, g, v), "{g:?}");
            assert_ne!(a, v, "{g:?}");
        }
    }

    #[test]
    fn deux_valeurs_differentes_deux_pseudonymes() {
        let c = cle();
        assert_ne!(
            pseudonymiser(&c, Genre::Nom, "Dupont"),
            pseudonymiser(&c, Genre::Nom, "Durand")
        );
        assert_ne!(
            pseudonymiser(&c, Genre::Email, "a@x.fr"),
            pseudonymiser(&c, Genre::Email, "b@x.fr")
        );
    }

    #[test]
    fn emails_en_example_test_avec_categorie() {
        let c = cle();
        let e = pseudonymiser(&c, Genre::Email, "parent@gmail.com");
        assert!(e.ends_with("@perso.example.test"), "{e}");
        assert!(
            pseudonymiser(&c, Genre::Email, "prof@ac-versailles.fr").ends_with("@ac.example.test")
        );
        assert!(pseudonymiser(&c, Genre::Email, "x@entreprise-locale.fr")
            .ends_with("@entreprise.example.test"));
        assert!(!pseudonymiser(&c, Genre::Email, "parent@gmail.com").contains("gmail"));
    }

    #[test]
    fn telephone_dans_les_plages_fictives_et_format_garde() {
        let c = cle();
        let t = pseudonymiser(&c, Genre::Telephone, "06 12 34 56 78");
        assert!(t.starts_with("06 39 98 "), "{t}");
        assert_eq!(t.len(), 14);
        let t = pseudonymiser(&c, Genre::Telephone, "0145678901");
        assert!(t.starts_with("019900"), "{t}");
        assert_eq!(t.len(), 10);
        let t = pseudonymiser(&c, Genre::Telephone, "+33 6 12 34 56 78");
        assert!(t.starts_with("+33 6 39 98"), "{t}");
        let t = pseudonymiser(&c, Genre::Telephone, "06.12.34.56.78");
        assert!(t.starts_with("06.39.98."), "{t}");
    }

    #[test]
    fn iban_structurellement_valide() {
        let c = cle();
        let i = pseudonymiser(&c, Genre::Iban, "FR7630006000011234567890189");
        assert!(i.starts_with("FR") && i.len() == 27, "{i}");
        // Contrôle modulo 97 : déplacer les 4 premiers caractères à la fin.
        let r = format!("{}{}", &i[4..], &i[..4]);
        assert_eq!(mod97(&r), 1, "{i}");
        assert_ne!(i, "FR7630006000011234567890189");
        let espaces = pseudonymiser(&c, Genre::Iban, "FR76 3000 6000 0112 3456 7890 189");
        assert_eq!(espaces.split(' ').count(), 7);
    }

    #[test]
    fn date_decalee_de_moins_de_61_jours_meme_format() {
        let c = cle();
        let d = pseudonymiser(&c, Genre::DateNaissance, "2010-03-14");
        let o = NaiveDate::from_ymd_opt(2010, 3, 14).unwrap();
        let n = NaiveDate::parse_from_str(&d, "%Y-%m-%d").unwrap();
        let ecart = (n - o).num_days().abs();
        assert!((1..=60).contains(&ecart), "{d}");
        let d = pseudonymiser(&c, Genre::DateNaissance, "14/03/2010");
        assert!(NaiveDate::parse_from_str(&d, "%d/%m/%Y").is_ok(), "{d}");
        let d = pseudonymiser(&c, Genre::DateNaissance, "2010-03-14T00:00:00Z");
        assert!(d.ends_with("T00:00:00Z"), "{d}");
    }

    #[test]
    fn texte_meme_longueur_et_forme() {
        let c = cle();
        let v = "Rendez-vous le 12/03 avec M. Dupont, 14h30.";
        let t = texte(&c, v);
        assert_eq!(t.chars().count(), v.chars().count());
        assert!(!t.contains("Dupont") && !t.contains("Rendez"));
        assert_eq!(t.matches(',').count(), 1);
        assert_eq!(texte(&c, "é"), texte(&c, "é"));
    }

    #[test]
    fn identifiants_opaques_gardes_les_autres_non() {
        let c = cle();
        let u = "123e4567-e89b-42d3-a456-426614174000";
        assert_eq!(pseudonymiser(&c, Genre::IdSouple, u), u);
        assert_eq!(
            pseudonymiser(&c, Genre::IdSouple, "a1b2c3d4e5f6g7h8i9"),
            "a1b2c3d4e5f6g7h8i9"
        );
        assert_eq!(
            pseudonymiser(&c, Genre::IdSouple, "V1StGXR8_Z5jdHi6B"),
            "V1StGXR8_Z5jdHi6B"
        );
        assert_ne!(
            pseudonymiser(&c, Genre::IdSouple, "marie-dupont-2024-abc"),
            "marie-dupont-2024-abc"
        );
        assert_ne!(
            pseudonymiser(&c, Genre::IdSouple, "dupont-jean"),
            "dupont-jean"
        );
        assert_ne!(
            pseudonymiser(&c, Genre::IdSouple, "marie.dupont@x.fr"),
            "marie.dupont@x.fr"
        );
    }

    #[test]
    fn jeton_unique_par_valeur_et_nom_de_fichier_garde_l_extension() {
        let c = cle();
        assert_ne!(
            pseudonymiser(&c, Genre::Jeton, "a"),
            pseudonymiser(&c, Genre::Jeton, "b")
        );
        assert!(pseudonymiser(&c, Genre::Jeton, "x").len() >= 16);
        let f = pseudonymiser(&c, Genre::NomFichier, "Justificatif Dupont.PDF");
        assert!(f.ends_with(".pdf") && !f.contains("Dupont"), "{f}");
    }

    #[test]
    fn valeur_vide_reste_vide_et_casse_respectee() {
        let c = cle();
        assert_eq!(pseudonymiser(&c, Genre::Nom, ""), "");
        let n = pseudonymiser(&c, Genre::Nom, "DUPONT");
        assert_eq!(n, n.to_uppercase());
        let n = pseudonymiser(&c, Genre::NomComplet, "Jean de LA FONTAINE");
        assert!(n.contains(" de "), "{n}");
    }

    #[test]
    fn code_postal_garde_le_departement() {
        let c = cle();
        let cp = pseudonymiser(&c, Genre::CodePostal, "75011");
        assert!(cp.starts_with("75") && cp.len() == 5, "{cp}");
    }

    #[test]
    fn ip_dans_les_blocs_de_documentation() {
        let c = cle();
        let i = pseudonymiser(&c, Genre::Ip, "81.2.69.160");
        assert!(
            ["192.0.2.", "198.51.100.", "203.0.113."]
                .iter()
                .any(|p| i.starts_with(p)),
            "{i}"
        );
        assert!(pseudonymiser(&c, Genre::Ip, "2a01:e0a::1").starts_with("2001:db8::"));
    }
}
