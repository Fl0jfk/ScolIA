//! Identifiant national élève (INE) — validation format (P0).

use crate::DomainError;

/// INE : 9–16 caractères alphanumériques (majuscules), hors espaces.
/// Format national réel ~11 ; labo / imports tolèrent une plage élargie.
pub fn normalize_ine(raw: &str) -> Result<String, DomainError> {
    let ine: String = raw
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if !(9..=16).contains(&ine.len()) {
        return Err(DomainError::IneInvalide);
    }
    if !ine.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(DomainError::IneInvalide);
    }
    Ok(ine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise() {
        assert_eq!(normalize_ine("ab1234567").unwrap(), "AB1234567");
        assert!(normalize_ine("short").is_err());
    }
}
