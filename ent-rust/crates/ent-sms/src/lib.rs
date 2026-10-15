//! SMS plateforme ENT.
//!
//! - Lab : `ENT_SMS_DRY_RUN=1` (défaut recommandé) → journal mémoire, pas d'API.
//! - Live : `ENT_SMS_LIVE=1` + `SMS_PROVIDER_URL` / `SMS_API_KEY` / `SMS_FROM`
//!   (POST JSON minimal — provider générique).
//!
//! Un échec SMS ne doit **jamais** faire échouer le geste métier.

use std::sync::{Arc, Mutex};

use thiserror::Error;

/// SMS sortant (texte court).
#[derive(Debug, Clone)]
pub struct OutgoingSms {
    pub to: String,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmsSendStatus {
    DryRun,
    Sent,
    SkippedNoConfig,
    Failed,
}

#[derive(Debug, Clone)]
pub struct SmsReceipt {
    pub to: String,
    pub body: String,
    pub status: SmsSendStatus,
    pub error: Option<String>,
    pub message_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum SmsError {
    #[error("numéro destinataire vide")]
    EmptyTo,
    #[error("corps SMS vide")]
    EmptyBody,
    #[error("SMS: {0}")]
    Provider(String),
}

#[derive(Debug, Clone)]
pub struct SmsConfig {
    pub provider_url: String,
    pub api_key: String,
    pub from: String,
}

impl SmsConfig {
    pub fn from_env() -> Option<Self> {
        let provider_url = first_nonempty(&["SMS_PROVIDER_URL", "ENT_SMS_PROVIDER_URL"])?;
        let api_key = first_nonempty(&["SMS_API_KEY", "ENT_SMS_API_KEY"])?;
        let from = first_nonempty(&["SMS_FROM", "ENT_SMS_FROM"]).unwrap_or_else(|| "ScolIA".into());
        Some(Self {
            provider_url,
            api_key,
            from,
        })
    }
}

#[derive(Clone)]
pub struct SmsService {
    config: Option<SmsConfig>,
    live: bool,
    force_dry_run: bool,
    log: Arc<Mutex<Vec<SmsReceipt>>>,
}

impl Default for SmsService {
    fn default() -> Self {
        Self::from_env()
    }
}

impl SmsService {
    pub fn from_env() -> Self {
        Self {
            config: SmsConfig::from_env(),
            live: env_flag("ENT_SMS_LIVE"),
            force_dry_run: env_flag("ENT_SMS_DRY_RUN"),
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn dry_run_only() -> Self {
        Self {
            config: SmsConfig::from_env(),
            live: false,
            force_dry_run: true,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.force_dry_run || self.config.is_some()
    }

    pub fn is_live(&self) -> bool {
        self.live && !self.force_dry_run && self.config.is_some()
    }

    pub fn receipts(&self) -> Vec<SmsReceipt> {
        self.log.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn count_with_body_contains(&self, needle: &str) -> usize {
        self.receipts()
            .into_iter()
            .filter(|r| r.body.contains(needle))
            .count()
    }

    pub async fn send(&self, sms: OutgoingSms) -> Result<SmsReceipt, SmsError> {
        let to = normalize_phone(&sms.to);
        let body = sms.body.trim().to_string();
        if to.is_empty() {
            return Err(SmsError::EmptyTo);
        }
        if body.is_empty() {
            return Err(SmsError::EmptyBody);
        }
        // Garde-fou longueur SMS (concat 3×160 ≈ 480).
        let body = if body.chars().count() > 480 {
            body.chars().take(477).collect::<String>() + "…"
        } else {
            body
        };

        let receipt = if self.is_live() {
            let cfg = self.config.as_ref().expect("is_live implies config");
            match send_provider(cfg, &to, &body).await {
                Ok(message_id) => SmsReceipt {
                    to: to.clone(),
                    body: body.clone(),
                    status: SmsSendStatus::Sent,
                    error: None,
                    message_id,
                },
                Err(e) => SmsReceipt {
                    to: to.clone(),
                    body: body.clone(),
                    status: SmsSendStatus::Failed,
                    error: Some(e.to_string()),
                    message_id: None,
                },
            }
        } else if self.config.is_none() && !self.force_dry_run {
            SmsReceipt {
                to: to.clone(),
                body: body.clone(),
                status: SmsSendStatus::SkippedNoConfig,
                error: None,
                message_id: None,
            }
        } else {
            SmsReceipt {
                to: to.clone(),
                body: body.clone(),
                status: SmsSendStatus::DryRun,
                error: None,
                message_id: Some(format!("drysms-{}", self.receipts().len() + 1)),
            }
        };

        if let Ok(mut g) = self.log.lock() {
            g.push(receipt.clone());
        }
        Ok(receipt)
    }
}

fn env_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let t = v.trim();
            t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

fn first_nonempty(keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Ok(v) = std::env::var(k) {
            let t = v.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

/// Garde chiffres et `+` initial.
pub fn normalize_phone(raw: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (i, c) in t.chars().enumerate() {
        if c.is_ascii_digit() || (i == 0 && c == '+') {
            out.push(c);
        }
    }
    // 06… → +336…
    if out.len() == 10 && out.starts_with('0') {
        out = format!("+33{}", &out[1..]);
    }
    out
}

async fn send_provider(
    cfg: &SmsConfig,
    to: &str,
    body: &str,
) -> Result<Option<String>, SmsError> {
    // Provider générique HTTP — volontairement minimal (pas de crate reqwest
    // obligatoire si non live). Live réel : brancher le SDK du fournisseur.
    let _ = (cfg, to, body);
    Err(SmsError::Provider(
        "provider HTTP live non branché dans ce runtime — utiliser ENT_SMS_DRY_RUN ou brancher le SDK".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_fr_mobile() {
        assert_eq!(normalize_phone("06 12 34 56 78"), "+33612345678");
        assert_eq!(normalize_phone("+33 6 12 34 56 78"), "+33612345678");
    }

    #[tokio::test]
    async fn dry_run_ok() {
        let svc = SmsService::dry_run_only();
        let r = svc
            .send(OutgoingSms {
                to: "0612345678".into(),
                body: "Sanction: retenue 60 min".into(),
            })
            .await
            .unwrap();
        assert_eq!(r.status, SmsSendStatus::DryRun);
        assert_eq!(svc.count_with_body_contains("retenue"), 1);
    }
}
