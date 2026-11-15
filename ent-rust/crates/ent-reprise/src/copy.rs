//! Format texte `COPY` de PostgreSQL (celui des dumps `pg_dump`) : décodage et encodage d'une cellule.
//!
//! Une ligne = valeurs séparées par une tabulation ; `\N` = NULL ; échappements `\\ \t \n \r \b \f \v`,
//! octal `\NNN` et hexadécimal `\xHH`.

/// Sépare une ligne de données en cellules **décodées** (`None` = NULL).
pub fn decouper_ligne(ligne: &str) -> Vec<Option<String>> {
    ligne.split('\t').map(decoder_cellule).collect()
}

/// Décode une cellule brute (telle qu'écrite dans le dump).
pub fn decoder_cellule(brut: &str) -> Option<String> {
    if brut == "\\N" {
        return None;
    }
    if !brut.contains('\\') {
        return Some(brut.to_string());
    }
    let mut octets: Vec<u8> = Vec::with_capacity(brut.len());
    let b = brut.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\\' || i + 1 >= b.len() {
            octets.push(b[i]);
            i += 1;
            continue;
        }
        let c = b[i + 1];
        match c {
            b'b' => {
                octets.push(8);
                i += 2;
            }
            b'f' => {
                octets.push(12);
                i += 2;
            }
            b'n' => {
                octets.push(b'\n');
                i += 2;
            }
            b'r' => {
                octets.push(b'\r');
                i += 2;
            }
            b't' => {
                octets.push(b'\t');
                i += 2;
            }
            b'v' => {
                octets.push(11);
                i += 2;
            }
            b'0'..=b'7' => {
                let mut v = 0u32;
                let mut n = 0;
                let mut j = i + 1;
                while j < b.len() && n < 3 && (b'0'..=b'7').contains(&b[j]) {
                    v = v * 8 + u32::from(b[j] - b'0');
                    j += 1;
                    n += 1;
                }
                octets.push((v & 0xFF) as u8);
                i = j;
            }
            b'x' if i + 2 < b.len() && b[i + 2].is_ascii_hexdigit() => {
                let mut v = 0u32;
                let mut n = 0;
                let mut j = i + 2;
                while j < b.len() && n < 2 && b[j].is_ascii_hexdigit() {
                    v = v * 16 + (b[j] as char).to_digit(16).unwrap_or(0);
                    j += 1;
                    n += 1;
                }
                octets.push(v as u8);
                i = j;
            }
            other => {
                octets.push(other);
                i += 2;
            }
        }
    }
    Some(String::from_utf8_lossy(&octets).into_owned())
}

/// Encode une cellule pour l'écriture dans un dump texte.
pub fn encoder_cellule(valeur: Option<&str>) -> String {
    let Some(v) = valeur else {
        return "\\N".to_string();
    };
    let mut s = String::with_capacity(v.len() + 2);
    for c in v.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '\t' => s.push_str("\\t"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\u{8}' => s.push_str("\\b"),
            '\u{c}' => s.push_str("\\f"),
            '\u{b}' => s.push_str("\\v"),
            c => s.push(c),
        }
    }
    s
}

/// Encode une ligne complète.
pub fn encoder_ligne(cellules: &[Option<String>]) -> String {
    let mut s = String::new();
    for (i, c) in cellules.iter().enumerate() {
        if i > 0 {
            s.push('\t');
        }
        s.push_str(&encoder_cellule(c.as_deref()));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_et_vide_sont_distincts() {
        assert_eq!(decoder_cellule("\\N"), None);
        assert_eq!(decoder_cellule(""), Some(String::new()));
        assert_eq!(encoder_cellule(None), "\\N");
        assert_eq!(encoder_cellule(Some("")), "");
    }

    #[test]
    fn echappements_aller_retour() {
        for v in [
            "a\tb",
            "ligne1\nligne2",
            "c:\\dossier",
            "\\N",
            "tab\t\\n littéral",
            "é\u{8}\u{c}\u{b}\r",
        ] {
            let enc = encoder_cellule(Some(v));
            assert!(!enc.contains('\t') && !enc.contains('\n'));
            assert_eq!(decoder_cellule(&enc).as_deref(), Some(v), "{v:?}");
        }
    }

    #[test]
    fn octal_et_hexa() {
        assert_eq!(decoder_cellule("\\101\\x42").as_deref(), Some("AB"));
        assert_eq!(decoder_cellule("\\303\\251").as_deref(), Some("é"));
    }

    #[test]
    fn ligne_decoupee() {
        let l = decouper_ligne("a\t\\N\tb\\tc");
        assert_eq!(l, vec![Some("a".into()), None, Some("b\tc".into())]);
        assert_eq!(encoder_ligne(&l), "a\t\\N\tb\\tc");
    }
}
