//! Environnement d'exécution : labo/test explicites vs production (fail-closed).

/// Valeurs de `ENT_ENV` (après trim, insensible à la casse) désignant un
/// environnement explicitement NON productif. Toute autre valeur (absente,
/// vide, `prod`, inconnue) est traitée comme productive. Miroir de
/// `ent_api::ENVS_NON_PROD` (un test d'`ent-api` vérifie la parité).
pub const ENVS_NON_PROD: [&str; 5] = ["lab", "labo", "dev", "local", "test"];

/// Pure : `true` seulement si `env` désigne explicitement un environnement non
/// productif.
pub fn env_non_prod_from(env: Option<&str>) -> bool {
    env.map(|e| e.trim().to_ascii_lowercase())
        .is_some_and(|v| ENVS_NON_PROD.contains(&v.as_str()))
}

/// `ENT_ENV` désigne-t-il explicitement un environnement non productif ?
pub fn env_non_prod() -> bool {
    env_non_prod_from(std::env::var("ENT_ENV").ok().as_deref())
}

/// Égalité en temps constant de deux chaînes (comparaison de secrets / hash).
pub fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_prod_explicite_seulement() {
        for v in ["lab", "LABO", " dev ", "local", "test"] {
            assert!(env_non_prod_from(Some(v)), "{v:?}");
        }
        for v in [
            None,
            Some(""),
            Some("prod"),
            Some("production"),
            Some("staging"),
        ] {
            assert!(!env_non_prod_from(v), "{v:?}");
        }
    }

    #[test]
    fn ct_eq_compare() {
        assert!(ct_eq("abc", "abc"));
        assert!(!ct_eq("abc", "abd"));
        assert!(!ct_eq("abc", "abcd"));
        assert!(ct_eq("", ""));
    }
}
