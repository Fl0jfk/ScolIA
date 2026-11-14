//! Push notifications ENT (famille / élève / staff).
//!
//! - Lab : `ENT_PUSH_DRY_RUN=1` → journal mémoire, pas d'APNs/FCM.
//! - Live : `ENT_PUSH_LIVE=1` + secrets provider (branchement ultérieur).
//!
//! Un échec push ne doit **jamais** faire échouer le geste métier.

use std::sync::{Arc, Mutex};

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushPlatform {
    Ios,
    Android,
    Web,
}

impl PushPlatform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
            Self::Web => "web",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ios" | "apns" => Some(Self::Ios),
            "android" | "fcm" => Some(Self::Android),
            "web" => Some(Self::Web),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutgoingPush {
    pub token: String,
    pub platform: PushPlatform,
    pub title: String,
    pub body: String,
    pub data_json: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushSendStatus {
    DryRun,
    Sent,
    SkippedNoConfig,
    Failed,
}

#[derive(Debug, Clone)]
pub struct PushReceipt {
    pub token: String,
    pub platform: PushPlatform,
    pub title: String,
    pub body: String,
    pub status: PushSendStatus,
    pub error: Option<String>,
    pub message_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum PushError {
    #[error("token vide")]
    EmptyToken,
    #[error("titre vide")]
    EmptyTitle,
    #[error("push: {0}")]
    Provider(String),
}

#[derive(Clone)]
pub struct PushService {
    live: bool,
    force_dry_run: bool,
    /// Présence d'une config provider (clés FCM/APNs) — pour live futur.
    configured: bool,
    log: Arc<Mutex<Vec<PushReceipt>>>,
}

impl Default for PushService {
    fn default() -> Self {
        Self::from_env()
    }
}

impl PushService {
    pub fn from_env() -> Self {
        let configured = first_nonempty(&["FCM_SERVER_KEY", "APNS_KEY_ID", "ENT_PUSH_KEY"]).is_some();
        Self {
            live: env_flag("ENT_PUSH_LIVE"),
            force_dry_run: env_flag("ENT_PUSH_DRY_RUN"),
            configured,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn dry_run_only() -> Self {
        Self {
            live: false,
            force_dry_run: true,
            configured: first_nonempty(&["FCM_SERVER_KEY", "APNS_KEY_ID", "ENT_PUSH_KEY"]).is_some(),
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.force_dry_run || self.configured
    }

    pub fn is_live(&self) -> bool {
        self.live && !self.force_dry_run && self.configured
    }

    pub fn receipts(&self) -> Vec<PushReceipt> {
        self.log.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn count_with_title_contains(&self, needle: &str) -> usize {
        self.receipts()
            .into_iter()
            .filter(|r| r.title.contains(needle))
            .count()
    }

    pub async fn send(&self, push: OutgoingPush) -> Result<PushReceipt, PushError> {
        let token = push.token.trim().to_string();
        let title = push.title.trim().to_string();
        let body = push.body.trim().to_string();
        if token.is_empty() {
            return Err(PushError::EmptyToken);
        }
        if title.is_empty() {
            return Err(PushError::EmptyTitle);
        }

        let receipt = if self.is_live() {
            // Provider réel (FCM/APNs) — branchement SDK ultérieur.
            PushReceipt {
                token: token.clone(),
                platform: push.platform,
                title: title.clone(),
                body: body.clone(),
                status: PushSendStatus::Failed,
                error: Some(
                    "provider FCM/APNs live non branché — ENT_PUSH_DRY_RUN ou brancher le SDK"
                        .into(),
                ),
                message_id: None,
            }
        } else if !self.configured && !self.force_dry_run {
            PushReceipt {
                token: token.clone(),
                platform: push.platform,
                title: title.clone(),
                body: body.clone(),
                status: PushSendStatus::SkippedNoConfig,
                error: None,
                message_id: None,
            }
        } else {
            PushReceipt {
                token: token.clone(),
                platform: push.platform,
                title: title.clone(),
                body: body.clone(),
                status: PushSendStatus::DryRun,
                error: None,
                message_id: Some(format!("drypush-{}", self.receipts().len() + 1)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dry_run_ok() {
        let svc = PushService::dry_run_only();
        let r = svc
            .send(OutgoingPush {
                token: "lab-token-lily".into(),
                platform: PushPlatform::Ios,
                title: "Carnet de liaison".into(),
                body: "Nouvelle entrée".into(),
                data_json: None,
            })
            .await
            .unwrap();
        assert_eq!(r.status, PushSendStatus::DryRun);
        assert_eq!(svc.count_with_title_contains("Carnet"), 1);
    }
}
