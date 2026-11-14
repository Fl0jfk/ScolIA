//! Détection d'homonymes / dedup élèves (P1).
//!
//! Matching normalisé nom+prénom (+ date de naissance optionnelle).

use crate::DomainError;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HomonymeCandidat {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub date_naissance: Option<NaiveDate>,
    pub classe: Option<String>,
    pub score: u8,
}

#[derive(Debug, Clone)]
pub struct RechercherHomonymes {
    pub etablissement_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub date_naissance: Option<NaiveDate>,
}

pub fn normalize_identite(s: &str) -> String {
    s.trim()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ÿ' => 'y',
            'ñ' => 'n',
            other => other,
        })
        .collect::<String>()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Score 0–100 : 100 = exact nom+prénom+naissance, 80 = nom+prénom, 60 = nom seul proche.
pub fn score_homonyme(
    needle_nom: &str,
    needle_prenom: &str,
    needle_naissance: Option<NaiveDate>,
    cand_nom: &str,
    cand_prenom: &str,
    cand_naissance: Option<NaiveDate>,
) -> u8 {
    let n_nom = normalize_identite(needle_nom);
    let n_pre = normalize_identite(needle_prenom);
    let c_nom = normalize_identite(cand_nom);
    let c_pre = normalize_identite(cand_prenom);
    if n_nom.is_empty() || n_pre.is_empty() {
        return 0;
    }
    let nom_ok = n_nom == c_nom;
    let pre_ok = n_pre == c_pre;
    let naissance_ok = match (needle_naissance, cand_naissance) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    };
    if nom_ok && pre_ok && naissance_ok {
        100
    } else if nom_ok && pre_ok {
        80
    } else if nom_ok && (c_pre.starts_with(&n_pre) || n_pre.starts_with(&c_pre)) {
        65
    } else if nom_ok {
        50
    } else {
        0
    }
}

pub fn plan_rechercher_homonymes(
    cmd: RechercherHomonymes,
) -> Result<(String, String), DomainError> {
    if cmd.etablissement_id.is_nil() {
        return Err(DomainError::Event(
            ent_events::EventBusError::MissingEtablissement,
        ));
    }
    let nom = cmd.nom.trim().to_string();
    let prenom = cmd.prenom.trim().to_string();
    if nom.is_empty() || prenom.is_empty() {
        return Err(DomainError::HomonymeInvalide);
    }
    Ok((nom, prenom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accents_et_naissance() {
        assert_eq!(normalize_identite("Hélène"), "helene");
        let s = score_homonyme(
            "Dupont",
            "Jean",
            Some(NaiveDate::from_ymd_opt(2010, 1, 1).unwrap()),
            "DUPONT",
            "jean",
            Some(NaiveDate::from_ymd_opt(2010, 1, 1).unwrap()),
        );
        assert_eq!(s, 100);
        let s2 = score_homonyme("Dupont", "Jean", None, "Dupont", "Jean", None);
        assert_eq!(s2, 80);
    }
}
