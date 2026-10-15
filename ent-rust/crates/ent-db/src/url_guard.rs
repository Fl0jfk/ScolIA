//! Garde-fou : aucune connexion Postgres hors localhost pour le chantier ENT Rust.

use thiserror::Error;
use url::Url;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UrlGuardError {
    #[error("DATABASE_URL / ENT_DATABASE_URL manquant ou vide")]
    Missing,
    #[error("URL invalide: {0}")]
    Invalid(String),
    #[error("hôte non local refusé ({host}) — uniquement 127.0.0.1 / localhost")]
    RemoteHost { host: String },
}

/// Accepte uniquement des URL Postgres dont l'hôte est loopback.
pub fn assert_local_postgres_url(raw: &str) -> Result<Url, UrlGuardError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(UrlGuardError::Missing);
    }
    let parsed = Url::parse(trimmed).map_err(|e| UrlGuardError::Invalid(e.to_string()))?;
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    let ok = matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1");
    if !ok {
        return Err(UrlGuardError::RemoteHost { host });
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_loopback() {
        assert!(assert_local_postgres_url(
            "postgres://scola:pwd@127.0.0.1:5432/scola"
        )
        .is_ok());
        assert!(assert_local_postgres_url(
            "postgresql://scola:pwd@localhost:5432/scola"
        )
        .is_ok());
    }

    #[test]
    fn rejects_remote_scaleway_style() {
        let err = assert_local_postgres_url(
            "postgresql://u:p@51.158.57.80:20566/rdb",
        )
        .expect_err("must reject remote");
        match err {
            UrlGuardError::RemoteHost { host } => assert_eq!(host, "51.158.57.80"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(
            assert_local_postgres_url("  ").unwrap_err(),
            UrlGuardError::Missing
        );
    }
}
