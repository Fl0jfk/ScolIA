//! Transport HTTP du SSO — trait injectable + implémentation réelle (reqwest).

use std::time::Duration;

use async_trait::async_trait;

use crate::OidcError;

/// Accès réseau minimal dont le SSO a besoin (découverte, JWKS, échange de
/// code). Isolé pour que la logique soit testable sans réseau.
#[async_trait]
pub trait OidcTransport: Send + Sync {
    async fn get_json(&self, url: &str) -> Result<serde_json::Value, OidcError>;
    async fn post_form_json(
        &self,
        url: &str,
        form: &[(String, String)],
    ) -> Result<serde_json::Value, OidcError>;
}

/// Implémentation réelle : reqwest + rustls, délais courts (un IdP lent ne doit
/// pas immobiliser un worker).
pub struct HttpOidcTransport {
    client: reqwest::Client,
}

impl HttpOidcTransport {
    pub fn new(timeout_secs: u64) -> Result<Self, OidcError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent("ScolIA-ENT/1.0 (+oidc)")
            .build()
            .map_err(|e| OidcError::Network(e.to_string()))?;
        Ok(Self { client })
    }

    /// Délai par défaut (10 s) — surchargeable via `ENT_OIDC_TIMEOUT_SECS`.
    pub fn from_env() -> Self {
        let secs = std::env::var("ENT_OIDC_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|s| *s > 0)
            .unwrap_or(10);
        Self::new(secs).unwrap_or_else(|_| Self {
            client: reqwest::Client::new(),
        })
    }

    async fn parse(res: reqwest::Response) -> Result<serde_json::Value, OidcError> {
        let status = res.status();
        let text = res
            .text()
            .await
            .map_err(|e| OidcError::Network(e.to_string()))?;
        if !status.is_success() {
            return Err(OidcError::Http {
                status: status.as_u16(),
            });
        }
        serde_json::from_str(&text)
            .map_err(|_| OidcError::Discovery(format!("réponse non JSON ({})", text.len())))
    }
}

#[async_trait]
impl OidcTransport for HttpOidcTransport {
    async fn get_json(&self, url: &str) -> Result<serde_json::Value, OidcError> {
        let res = self
            .client
            .get(url)
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|e| OidcError::Network(e.to_string()))?;
        Self::parse(res).await
    }

    async fn post_form_json(
        &self,
        url: &str,
        form: &[(String, String)],
    ) -> Result<serde_json::Value, OidcError> {
        let res = self
            .client
            .post(url)
            .header("accept", "application/json")
            .form(form)
            .send()
            .await
            .map_err(|e| OidcError::Network(e.to_string()))?;
        Self::parse(res).await
    }
}
