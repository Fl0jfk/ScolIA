//! Décodage **strict** d'une cellule du dump selon le type SQL attendu.
//!
//! Une valeur hors domaine n'est jamais corrigée ni ignorée : elle produit un [`Refus`] (code stable +
//! message sans valeur). Le texte brut est conservé tel quel pour la zone de reprise (colonnes texte).

use chrono::{DateTime, NaiveDate};

/// Raison de refus : un code stable (clé de comptage) et un message qui ne cite jamais la valeur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refus {
    pub code: &'static str,
    pub message: String,
}

pub fn refus(code: &'static str, message: impl Into<String>) -> Refus {
    Refus {
        code,
        message: message.into(),
    }
}

/// Valeur comprise.
#[derive(Debug, Clone, PartialEq)]
pub enum Valeur {
    Nul,
    Texte,
    Entier(i64),
    Decimal,
    Reel(f64),
    Booleen(bool),
    Uuid,
    Instant,
    Jour,
    Json(serde_json::Value),
    Tableau(Vec<String>),
}

pub const TEXTE_MAX: usize = 4_000_000;

/// Décode `brut` pour une colonne de type `type_sql`.
pub fn decoder(type_sql: &str, nullable: bool, brut: Option<&str>) -> Result<Valeur, Refus> {
    let Some(v) = brut else {
        return if nullable {
            Ok(Valeur::Nul)
        } else {
            Err(refus("null_interdit", "valeur absente dans une colonne obligatoire"))
        };
    };
    match type_sql {
        "text" => {
            if v.len() > TEXTE_MAX {
                return Err(refus("texte_trop_long", "texte au-delà de la limite"));
            }
            Ok(Valeur::Texte)
        }
        "uuid" => {
            if !est_uuid(v) {
                return Err(refus("uuid_invalide", "identifiant UUID mal formé"));
            }
            if v == "00000000-0000-0000-0000-000000000000" {
                return Err(refus("uuid_nul", "identifiant UUID nul"));
            }
            Ok(Valeur::Uuid)
        }
        "timestamp with time zone" => {
            let d = DateTime::parse_from_str(v, "%Y-%m-%d %H:%M:%S%.f%#z")
                .or_else(|_| DateTime::parse_from_rfc3339(v))
                .map_err(|_| refus("instant_invalide", "horodatage mal formé"))?;
            borne_annee(chrono::Datelike::year(&d), "instant_hors_domaine")?;
            Ok(Valeur::Instant)
        }
        "date" => {
            let d = NaiveDate::parse_from_str(v, "%Y-%m-%d")
                .map_err(|_| refus("jour_invalide", "date mal formée"))?;
            borne_annee(chrono::Datelike::year(&d), "jour_hors_domaine")?;
            Ok(Valeur::Jour)
        }
        "integer" => v
            .parse::<i32>()
            .map(|n| Valeur::Entier(i64::from(n)))
            .map_err(|_| refus("entier_invalide", "entier 32 bits attendu")),
        "bigint" => v
            .parse::<i64>()
            .map(Valeur::Entier)
            .map_err(|_| refus("entier_invalide", "entier 64 bits attendu")),
        "boolean" => match v {
            "t" => Ok(Valeur::Booleen(true)),
            "f" => Ok(Valeur::Booleen(false)),
            _ => Err(refus("booleen_invalide", "booléen `t` ou `f` attendu")),
        },
        "numeric" => {
            if est_decimal(v) {
                Ok(Valeur::Decimal)
            } else {
                Err(refus("decimal_invalide", "nombre décimal mal formé"))
            }
        }
        "double precision" => match v.parse::<f64>() {
            Ok(x) if x.is_finite() => Ok(Valeur::Reel(x)),
            _ => Err(refus("reel_invalide", "nombre réel fini attendu")),
        },
        "jsonb" | "json" => serde_json::from_str(v)
            .map(Valeur::Json)
            .map_err(|_| refus("json_invalide", "JSON mal formé")),
        "_text" | "text[]" => tableau_texte(v).map(Valeur::Tableau),
        autre => Err(refus(
            "type_sql_inconnu",
            format!("type SQL non géré par le décodeur : {autre}"),
        )),
    }
}

fn borne_annee(annee: i32, code: &'static str) -> Result<(), Refus> {
    if (1900..=2100).contains(&annee) {
        Ok(())
    } else {
        Err(refus(code, "année hors de 1900–2100"))
    }
}

pub fn est_uuid(v: &str) -> bool {
    let b = v.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_hexdigit(),
        })
}

fn est_decimal(v: &str) -> bool {
    let s = v.strip_prefix('-').unwrap_or(v);
    let (ent, frac) = match s.split_once('.') {
        Some((e, f)) => (e, Some(f)),
        None => (s, None),
    };
    !ent.is_empty()
        && ent.len() <= 20
        && ent.bytes().all(|c| c.is_ascii_digit())
        && frac.is_none_or(|f| !f.is_empty() && f.len() <= 10 && f.bytes().all(|c| c.is_ascii_digit()))
}

/// Littéral de tableau PostgreSQL à une dimension : `{a,"b c",...}`. Un élément NULL est refusé.
pub fn tableau_texte(v: &str) -> Result<Vec<String>, Refus> {
    let mal = || refus("tableau_invalide", "littéral de tableau mal formé");
    let corps = v
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .ok_or_else(mal)?;
    let mut out = Vec::new();
    if corps.is_empty() {
        return Ok(out);
    }
    let mut it = corps.chars().peekable();
    loop {
        let mut elem = String::new();
        if it.peek() == Some(&'"') {
            it.next();
            loop {
                match it.next() {
                    Some('\\') => elem.push(it.next().ok_or_else(mal)?),
                    Some('"') => break,
                    Some(c) => elem.push(c),
                    None => return Err(mal()),
                }
            }
        } else {
            while let Some(&c) = it.peek() {
                if c == ',' {
                    break;
                }
                if matches!(c, '{' | '}' | '"' | '\\') {
                    return Err(mal());
                }
                elem.push(c);
                it.next();
            }
            if elem.eq_ignore_ascii_case("NULL") {
                return Err(refus("element_nul", "élément NULL dans un tableau"));
            }
            if elem.is_empty() {
                return Err(mal());
            }
        }
        out.push(elem);
        match it.next() {
            Some(',') => continue,
            None => return Ok(out),
            _ => return Err(mal()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(t: &str, v: &str) -> bool {
        decoder(t, false, Some(v)).is_ok()
    }

    #[test]
    fn types_simples() {
        assert!(ok("uuid", "0e7ab115-0000-4000-8000-00000000f1c7"));
        assert!(!ok("uuid", "pas-un-uuid"));
        assert!(!ok("uuid", "00000000-0000-0000-0000-000000000000"));
        assert!(ok("integer", "-12"));
        assert!(!ok("integer", "2147483648"));
        assert!(ok("bigint", "2147483648"));
        assert!(ok("boolean", "t") && ok("boolean", "f"));
        assert!(!ok("boolean", "true"));
        assert!(ok("numeric", "10.50") && ok("numeric", "-3"));
        assert!(!ok("numeric", "NaN") && !ok("numeric", "1e5") && !ok("numeric", "1."));
        assert!(ok("double precision", "0.5") && !ok("double precision", "inf"));
        assert!(ok("date", "2026-09-01") && !ok("date", "2026-02-30") && !ok("date", "0001-01-01"));
    }

    #[test]
    fn instants() {
        assert!(ok("timestamp with time zone", "2026-09-01 08:30:00+00"));
        assert!(ok("timestamp with time zone", "2026-09-01 08:30:00.123456+02"));
        assert!(ok("timestamp with time zone", "2026-09-01T08:30:00Z"));
        assert!(!ok("timestamp with time zone", "infinity"));
        assert!(!ok("timestamp with time zone", "2026-09-01"));
        assert!(!ok("timestamp with time zone", "3026-09-01 08:30:00+00"));
    }

    #[test]
    fn null_selon_nullable() {
        assert_eq!(decoder("text", true, None), Ok(Valeur::Nul));
        assert_eq!(decoder("text", false, None).unwrap_err().code, "null_interdit");
    }

    #[test]
    fn tableaux() {
        assert_eq!(tableau_texte("{}").unwrap(), Vec::<String>::new());
        assert_eq!(tableau_texte("{a,\"b c\",\"d\\\"e\"}").unwrap(), vec!["a", "b c", "d\"e"]);
        assert_eq!(tableau_texte("{a,NULL}").unwrap_err().code, "element_nul");
        assert!(tableau_texte("a,b").is_err());
        assert!(tableau_texte("{a,,b}").is_err());
        assert!(tableau_texte("{\"a}").is_err());
    }

    #[test]
    fn type_inconnu_et_json() {
        assert_eq!(decoder("money", false, Some("1")).unwrap_err().code, "type_sql_inconnu");
        assert!(ok("jsonb", "{\"a\":1}"));
        assert_eq!(decoder("jsonb", false, Some("{a")).unwrap_err().code, "json_invalide");
    }
}
