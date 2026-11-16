//! Décodage strict des tables clé-valeur `*_attr` (documents aplatis par `ent-attr-codec.ts`).
//!
//! Une ligne est `(propriétaire, chemin, valeur)`. La valeur porte un préfixe de type : `s:` texte,
//! `n:` nombre, `b:` booléen `0`/`1`, `j:` JSON, `null:null`, `undef:`. Tout autre préfixe est refusé.

use super::cellule::{refus, Refus};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeFeuille {
    Texte,
    Nombre,
    Booleen,
    Json,
    Nul,
    Absent,
}

impl TypeFeuille {
    pub fn code(self) -> &'static str {
        match self {
            TypeFeuille::Texte => "s",
            TypeFeuille::Nombre => "n",
            TypeFeuille::Booleen => "b",
            TypeFeuille::Json => "j",
            TypeFeuille::Nul => "null",
            TypeFeuille::Absent => "undef",
        }
    }
}

pub const CHEMIN_MAX: usize = 400;
pub const SEGMENT_MAX: usize = 120;

pub fn decoder_feuille(v: &str) -> Result<TypeFeuille, Refus> {
    if v == "null:null" {
        return Ok(TypeFeuille::Nul);
    }
    if v == "undef:" {
        return Ok(TypeFeuille::Absent);
    }
    if let Some(r) = v.strip_prefix("s:") {
        let _ = r;
        return Ok(TypeFeuille::Texte);
    }
    if let Some(r) = v.strip_prefix("n:") {
        return match r.parse::<f64>() {
            Ok(x) if x.is_finite() => Ok(TypeFeuille::Nombre),
            _ => Err(refus("attr_nombre", "feuille `n:` : nombre fini attendu")),
        };
    }
    if let Some(r) = v.strip_prefix("b:") {
        return match r {
            "0" | "1" => Ok(TypeFeuille::Booleen),
            _ => Err(refus("attr_booleen", "feuille `b:` : 0 ou 1 attendu")),
        };
    }
    if let Some(r) = v.strip_prefix("j:") {
        return serde_json::from_str::<serde_json::Value>(r)
            .map(|_| TypeFeuille::Json)
            .map_err(|_| refus("attr_json", "feuille `j:` : JSON mal formé"));
    }
    Err(refus("attr_prefixe", "préfixe de valeur inconnu (s:, n:, b:, j:, null:null, undef:)"))
}

pub fn valider_chemin(chemin: &str) -> Result<(), Refus> {
    if chemin.is_empty() || chemin.len() > CHEMIN_MAX {
        return Err(refus("attr_chemin", "chemin vide ou trop long"));
    }
    if chemin.chars().any(char::is_control) {
        return Err(refus("attr_chemin", "caractère de contrôle dans le chemin"));
    }
    if chemin == "__root" {
        return Ok(());
    }
    if chemin.split('.').any(|s| s.is_empty() || s.len() > SEGMENT_MAX) {
        return Err(refus("attr_chemin", "segment de chemin vide ou trop long"));
    }
    Ok(())
}

/// `__len` doit porter un entier ; `__root` est une feuille unique (pas d'enfants).
pub fn valider_semantique(chemin: &str, t: TypeFeuille, valeur: &str) -> Result<(), Refus> {
    if chemin.rsplit('.').next() == Some("__len") {
        let ok = t == TypeFeuille::Nombre && valeur[2..].parse::<u32>().is_ok();
        if !ok {
            return Err(refus("attr_len", "`__len` : entier positif attendu"));
        }
    }
    Ok(())
}

/// Vérifie la cohérence des chemins d'un même document : une feuille ne peut pas avoir d'enfants, et
/// `__root` est seul.
#[derive(Default)]
pub struct Documents {
    chemins: BTreeMap<String, BTreeSet<String>>,
}

impl Documents {
    pub fn ajouter(&mut self, proprietaire: &str, chemin: &str) {
        self.chemins.entry(proprietaire.to_string()).or_default().insert(chemin.to_string());
    }

    /// Documents incohérents : (propriétaire, code).
    pub fn conflits(&self) -> Vec<(String, &'static str)> {
        let mut out = Vec::new();
        for (p, set) in &self.chemins {
            if set.contains("__root") && set.len() > 1 {
                out.push((p.clone(), "attr_racine_et_enfants"));
                continue;
            }
            // Un chemin est un préfixe strict d'un autre : feuille portant des enfants.
            let conflit = set
                .iter()
                .any(|c| c.match_indices('.').any(|(i, _)| set.contains(&c[..i])));
            if conflit {
                out.push((p.clone(), "attr_feuille_et_enfants"));
            }
        }
        out
    }

    pub fn nombre(&self) -> usize {
        self.chemins.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feuilles() {
        assert_eq!(decoder_feuille("s:x"), Ok(TypeFeuille::Texte));
        assert_eq!(decoder_feuille("n:12.5"), Ok(TypeFeuille::Nombre));
        assert_eq!(decoder_feuille("b:1"), Ok(TypeFeuille::Booleen));
        assert_eq!(decoder_feuille("j:{\"a\":1}"), Ok(TypeFeuille::Json));
        assert_eq!(decoder_feuille("null:null"), Ok(TypeFeuille::Nul));
        assert_eq!(decoder_feuille("n:abc").unwrap_err().code, "attr_nombre");
        assert_eq!(decoder_feuille("b:2").unwrap_err().code, "attr_booleen");
        assert_eq!(decoder_feuille("j:{").unwrap_err().code, "attr_json");
        assert_eq!(decoder_feuille("x:1").unwrap_err().code, "attr_prefixe");
    }

    #[test]
    fn chemins() {
        assert!(valider_chemin("a.b.0.c").is_ok());
        assert!(valider_chemin("__root").is_ok());
        assert!(valider_chemin("a..b").is_err());
        assert!(valider_chemin("").is_err());
        assert!(valider_chemin("a\nb").is_err());
        assert!(valider_semantique("notes.__len", TypeFeuille::Nombre, "n:3").is_ok());
        assert!(valider_semantique("notes.__len", TypeFeuille::Nombre, "n:-3").is_err());
        assert!(valider_semantique("notes.__len", TypeFeuille::Texte, "s:3").is_err());
    }

    #[test]
    fn documents_incoherents() {
        let mut d = Documents::default();
        d.ajouter("p1", "a");
        d.ajouter("p1", "a.b");
        d.ajouter("p2", "x.y");
        d.ajouter("p2", "x.z");
        d.ajouter("p3", "__root");
        d.ajouter("p3", "k");
        let c = d.conflits();
        assert_eq!(c, vec![("p1".to_string(), "attr_feuille_et_enfants"), ("p3".to_string(), "attr_racine_et_enfants")]);
    }
}
