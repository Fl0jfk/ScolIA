//! Unicode → WinAnsiEncoding (CP1252), avec repli lisible.
//!
//! Les polices standard 14 ne sont pas incorporées : le texte est codé sur un octet selon
//! `/Encoding /WinAnsiEncoding`. Tout caractère français courant est couvert ; ce que la table
//! ne connaît pas est translittéré (`ł` → `l`, `→` → `->`) ou, à défaut, devient `?`.

/// Octet WinAnsi d'un caractère, s'il existe directement.
pub fn octet(c: char) -> Option<u8> {
    let u = c as u32;
    Some(match u {
        0x20..=0x7e => u as u8,
        0xa0..=0xac | 0xae..=0xff => u as u8,
        0x20ac => 0x80,
        0x201a => 0x82,
        0x0192 => 0x83,
        0x201e => 0x84,
        0x2026 => 0x85,
        0x2020 => 0x86,
        0x2021 => 0x87,
        0x02c6 => 0x88,
        0x2030 => 0x89,
        0x0160 => 0x8a,
        0x2039 => 0x8b,
        0x0152 => 0x8c,
        0x017d => 0x8e,
        0x2018 => 0x91,
        0x2019 => 0x92,
        0x201c => 0x93,
        0x201d => 0x94,
        0x2022 => 0x95,
        0x2013 => 0x96,
        0x2014 => 0x97,
        0x02dc => 0x98,
        0x2122 => 0x99,
        0x0161 => 0x9a,
        0x203a => 0x9b,
        0x0153 => 0x9c,
        0x017e => 0x9e,
        0x0178 => 0x9f,
        _ => return None,
    })
}

/// Caractère Unicode d'un octet WinAnsi (pour relire un PDF produit, dans les tests).
pub fn depuis_octet(b: u8) -> char {
    const HAUT: [char; 32] = [
        '€', '\u{fffd}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{fffd}', 'Ž',
        '\u{fffd}', '\u{fffd}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ',
        '\u{fffd}', 'ž', 'Ÿ',
    ];
    match b {
        0x80..=0x9f => HAUT[(b - 0x80) as usize],
        _ => b as char,
    }
}

const BASES_ETENDU_A: &str = "AaAaAaCcCcCcCcDdDdEeEeEeEeEeGgGgGgGgHhHhIiIiIiIiIiIiJjKkkLlLlLlLlLlNnNnNnnNnOoOoOoOoRrRrRrSsSsSsSsTtTtTtUuUuUuUuUuUuWwYyYZzZzZzs";

const COMBINAISONS: [(char, &str, &str); 7] = [
    ('\u{301}', "AEIOUYaeiouy", "ÁÉÍÓÚÝáéíóúý"),
    ('\u{300}', "AEIOUaeiou", "ÀÈÌÒÙàèìòù"),
    ('\u{302}', "AEIOUaeiou", "ÂÊÎÔÛâêîôû"),
    ('\u{308}', "AEIOUYaeiouy", "ÄËÏÖÜŸäëïöüÿ"),
    ('\u{327}', "Cc", "Çç"),
    ('\u{303}', "ANOano", "ÃÑÕãñõ"),
    ('\u{30a}', "Aa", "Åå"),
];

fn composer(base: char, marque: char) -> Option<char> {
    for (m, bases, composes) in COMBINAISONS {
        if m == marque {
            let i = bases.chars().position(|b| b == base)?;
            return composes.chars().nth(i);
        }
    }
    None
}

fn repli(c: char) -> Option<&'static str> {
    Some(match c {
        '\u{2000}'..='\u{200a}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\t' => " ",
        '\u{200b}'..='\u{200d}' | '\u{feff}' | '\u{ad}' | '\u{2060}' => "",
        '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2212}' => "-",
        '\u{2015}' => "—",
        '\u{2032}' | '\u{02bc}' | '\u{2035}' => "'",
        '\u{2033}' | '\u{201f}' | '\u{201b}' => "\"",
        'ᵉ' => "e",
        'ʳ' => "r",
        'ˢ' => "s",
        'ᵈ' => "d",
        '→' => "->",
        '←' => "<-",
        '↔' => "<->",
        '≤' => "<=",
        '≥' => ">=",
        '≠' => "!=",
        '≈' => "~",
        '№' => "No",
        '℃' => "°C",
        '·' => "·",
        '\u{2022}' => "•",
        _ => return None,
    })
}

/// Encode un texte en octets WinAnsi. `\n` est conservé (la mise en page le traite) ;
/// les autres caractères de contrôle deviennent des espaces.
pub fn encoder(s: &str) -> Vec<u8> {
    let mut sortie = Vec::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(mut c) = it.next() {
        // Lettres décomposées (NFD) : composées quand c'est possible.
        while let Some(&n) = it.peek() {
            if ('\u{300}'..='\u{36f}').contains(&n) {
                it.next();
                if let Some(x) = composer(c, n) {
                    c = x;
                }
            } else {
                break;
            }
        }
        if c == '\n' {
            sortie.push(b'\n');
        } else if c == '\r' {
            // ignoré (CRLF) : le `\n` qui suit fait la coupure
        } else if c.is_control() {
            sortie.push(b' ');
        } else if let Some(b) = octet(c) {
            sortie.push(b);
        } else if let Some(r) = repli(c) {
            for rc in r.chars() {
                sortie.push(octet(rc).unwrap_or(b'?'));
            }
        } else if ('\u{100}'..='\u{17f}').contains(&c) {
            sortie.push(BASES_ETENDU_A.as_bytes()[(c as u32 - 0x100) as usize]);
        } else if ('\u{300}'..='\u{36f}').contains(&c) {
            // marque combinante isolée : sans effet
        } else {
            sortie.push(b'?');
        }
    }
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_etendue_a_longueur() {
        assert_eq!(BASES_ETENDU_A.len(), 128);
    }

    #[test]
    fn tous_les_caracteres_francais() {
        for c in "éèêëàâçôùûüïîœŒæÆ«»€–—’…ÉÈÀÇÔÙÎÏŸ°œ".chars() {
            assert!(octet(c).is_some(), "{c} doit avoir un octet WinAnsi");
        }
        assert_eq!(octet('é'), Some(0xe9));
        assert_eq!(octet('€'), Some(0x80));
        assert_eq!(octet('œ'), Some(0x9c));
        assert_eq!(octet('Œ'), Some(0x8c));
        assert_eq!(octet('’'), Some(0x92));
        assert_eq!(octet('—'), Some(0x97));
        assert_eq!(octet('«'), Some(0xab));
        assert_eq!(octet('æ'), Some(0xe6));
    }

    #[test]
    fn aller_retour() {
        let s = "Élève Hélène « œuvre » – 12 € … ’ ç à ù ü ï";
        let o = encoder(s);
        let r: String = o.iter().map(|&b| depuis_octet(b)).collect();
        assert_eq!(r, s);
    }

    #[test]
    fn repli_lisible() {
        assert_eq!(encoder("Łódź"), vec![b'L', 0xf3, b'd', b'z']);
        assert_eq!(encoder("a→b"), b"a->b".to_vec());
        assert_eq!(encoder("x\u{2011}y"), b"x-y".to_vec());
        assert_eq!(encoder("a\u{200b}b"), b"ab".to_vec());
        assert_eq!(encoder("日本"), b"??".to_vec());
        assert_eq!(encoder("4ᵉ B, 1ʳᵉ"), b"4e B, 1re".to_vec());
        assert_eq!(encoder("a\tb\r\nc"), b"a b\nc".to_vec());
    }

    #[test]
    fn nfd_recompose() {
        assert_eq!(encoder("e\u{301}le\u{300}ve"), vec![0xe9, b'l', 0xe8, b'v', b'e']);
        assert_eq!(encoder("c\u{327}"), vec![0xe7]);
    }
}
