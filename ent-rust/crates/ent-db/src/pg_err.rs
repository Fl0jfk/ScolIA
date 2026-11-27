//! Classification **commune** des erreurs Postgres en erreurs de domaine.
//!
//! | Cas sqlx / code PG     | Variante de domaine              | HTTP (API) |
//! |------------------------|----------------------------------|------------|
//! | `RowNotFound`          | `DomainError::NotFound`          | 404        |
//! | `23505` unicité        | `DomainError::Conflit`           | 409        |
//! | `23503` clé étrangère  | `DomainError::ReferenceInconnue` | 400        |
//! | `23514` / `23502`      | `DomainError::ValeurInvalide`    | 400        |
//! | classe `22` (donnée)   | `DomainError::ValeurInvalide`    | 400        |
//! | classe `40` (concurrence) | `DomainError::Reessayer`      | 503        |
//! | tout le reste (panne)  | `DomainError::Infrastructure`    | 500 générique |
//!
//! Une panne d'infrastructure ne doit jamais ressembler à un « introuvable » :
//! elle est journalisée via `tracing` (code SQLSTATE seulement, jamais le texte
//! de l'erreur qui peut citer des valeurs) et remontée en erreur interne.

use domain_eleve::DomainError;

pub fn map_sqlx(scope: &str, err: sqlx::Error) -> DomainError {
    match &err {
        sqlx::Error::RowNotFound => return DomainError::NotFound,
        sqlx::Error::Database(db) => {
            let code = db.code().map(|c| c.to_string()).unwrap_or_default();
            let mapped = classer_sqlstate(&code);
            match &mapped {
                DomainError::Infrastructure => {
                    tracing::error!(scope, sqlstate = %code, "erreur base de données");
                }
                _ => tracing::warn!(scope, sqlstate = %code, "erreur base classée en erreur de domaine"),
            }
            return mapped;
        }
        _ => {}
    }
    tracing::error!(scope, kind = %kind(&err), "erreur d'infrastructure base de données");
    DomainError::Infrastructure
}

/// Classification pure d'un SQLSTATE (testable sans base).
pub fn classer_sqlstate(code: &str) -> DomainError {
    match code {
        "23505" => DomainError::Conflit,
        "23503" => DomainError::ReferenceInconnue,
        "23514" | "23502" => DomainError::ValeurInvalide,
        // Classe 22 : donnée invalide (chaîne trop longue, hors plage, date,
        // syntaxe, octet NUL…) = entrée du client, pas une panne.
        c if c.starts_with("22") => DomainError::ValeurInvalide,
        // Classe 40 : sérialisation / interblocage, l'appelant peut réessayer.
        c if c.starts_with("40") => DomainError::Reessayer,
        _ => DomainError::Infrastructure,
    }
}

fn kind(err: &sqlx::Error) -> &'static str {
    match err {
        sqlx::Error::Io(_) => "io",
        sqlx::Error::PoolTimedOut => "pool_timeout",
        sqlx::Error::PoolClosed => "pool_closed",
        sqlx::Error::Tls(_) => "tls",
        sqlx::Error::Protocol(_) => "protocol",
        sqlx::Error::ColumnNotFound(_) | sqlx::Error::ColumnDecode { .. } => "colonne",
        sqlx::Error::Decode(_) => "decode",
        _ => "autre",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_sqlstate() {
        for c in ["22001", "22003", "22007", "22008", "22P02", "22021", "23514", "23502"] {
            assert!(matches!(classer_sqlstate(c), DomainError::ValeurInvalide), "{c}");
        }
        for c in ["40001", "40P01"] {
            assert!(matches!(classer_sqlstate(c), DomainError::Reessayer), "{c}");
        }
        assert!(matches!(classer_sqlstate("23505"), DomainError::Conflit));
        assert!(matches!(classer_sqlstate("23503"), DomainError::ReferenceInconnue));
        for c in ["", "08006", "42P01", "53300", "57P01", "XX000"] {
            assert!(matches!(classer_sqlstate(c), DomainError::Infrastructure), "{c}");
        }
    }
}
