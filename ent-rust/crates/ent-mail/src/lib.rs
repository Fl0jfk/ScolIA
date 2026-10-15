//! SMTP plateforme ENT — même contrat d'env que `app/lib/tenant-mail.ts`.
//!
//! - Config : `MAILER_*` / `SMTP_*` (From, host, user, pass, port).
//! - Lab : `ENT_MAIL_DRY_RUN=1` (défaut recommandé) → enregistre sans socket.
//! - Live : `ENT_MAIL_LIVE=1` + config complète → envoi réel via lettre.
//!
//! Un échec SMTP ne doit **jamais** faire échouer le geste métier appelant.

use std::sync::{Arc, Mutex};

use thiserror::Error;

mod config;
mod smtp;

pub use config::{SmtpConfig, SmtpConfigError};

/// Pièce jointe optionnelle (PDF courrier, etc.).
#[derive(Debug, Clone)]
pub struct MailAttachment {
    pub filename: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

/// Message sortant (texte brut — HTML plus tard).
#[derive(Debug, Clone)]
pub struct OutgoingMail {
    pub to: String,
    pub subject: String,
    pub body_text: String,
    pub attachments: Vec<MailAttachment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailSendStatus {
    /// Enregistré sans socket (lab / pas de live).
    DryRun,
    /// Socket SMTP OK.
    Sent,
    /// Config absente — rien envoyé.
    SkippedNoConfig,
    /// Tentative live échouée (détail dans `error`).
    Failed,
}

#[derive(Debug, Clone)]
pub struct MailReceipt {
    pub to: String,
    pub subject: String,
    pub status: MailSendStatus,
    pub error: Option<String>,
    pub message_id: Option<String>,
}

#[derive(Debug, Error)]
pub enum MailError {
    #[error("adresse destinataire vide")]
    EmptyTo,
    #[error("sujet vide")]
    EmptySubject,
    #[error("SMTP: {0}")]
    Smtp(String),
}

/// Service mail partageable (PgEnt / lab).
#[derive(Clone)]
pub struct MailService {
    config: Option<SmtpConfig>,
    live: bool,
    force_dry_run: bool,
    log: Arc<Mutex<Vec<MailReceipt>>>,
}

impl Default for MailService {
    fn default() -> Self {
        Self::from_env()
    }
}

impl MailService {
    /// Construit depuis l'environnement process.
    pub fn from_env() -> Self {
        let force_dry_run = env_flag("ENT_MAIL_DRY_RUN");
        let live = env_flag("ENT_MAIL_LIVE");
        let config = SmtpConfig::from_env().ok();
        Self {
            config,
            live,
            force_dry_run,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Lab / tests : dry-run forcé, config optionnelle (pour `is_ready`).
    pub fn dry_run_only() -> Self {
        Self {
            config: SmtpConfig::from_env().ok(),
            live: false,
            force_dry_run: true,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_config(config: SmtpConfig, live: bool) -> Self {
        Self {
            config: Some(config),
            live,
            force_dry_run: !live,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Prêt à « notifier » (dry-run ou live). Toujours vrai en dry-run forcé.
    pub fn is_ready(&self) -> bool {
        self.force_dry_run || self.config.is_some()
    }

    pub fn is_live(&self) -> bool {
        self.live && !self.force_dry_run && self.config.is_some()
    }

    pub fn from_address(&self) -> Option<&str> {
        self.config.as_ref().map(|c| c.from.as_str())
    }

    /// Reçus en mémoire (lab assertions).
    pub fn receipts(&self) -> Vec<MailReceipt> {
        self.log.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn clear_receipts(&self) {
        if let Ok(mut g) = self.log.lock() {
            g.clear();
        }
    }

    pub fn count_with_subject_contains(&self, needle: &str) -> usize {
        self.receipts()
            .into_iter()
            .filter(|r| r.subject.contains(needle))
            .count()
    }

    /// Envoi best-effort. Erreurs métier → `Failed` dans le reçu, pas de panic.
    pub async fn send(&self, mail: OutgoingMail) -> Result<MailReceipt, MailError> {
        let to = mail.to.trim().to_string();
        let subject = mail.subject.trim().to_string();
        if to.is_empty() {
            return Err(MailError::EmptyTo);
        }
        if subject.is_empty() {
            return Err(MailError::EmptySubject);
        }

        let want_live = self.is_live();
        let receipt = if !want_live {
            if self.config.is_none() && !self.force_dry_run {
                MailReceipt {
                    to: to.clone(),
                    subject: subject.clone(),
                    status: MailSendStatus::SkippedNoConfig,
                    error: None,
                    message_id: None,
                }
            } else {
                MailReceipt {
                    to: to.clone(),
                    subject: subject.clone(),
                    status: MailSendStatus::DryRun,
                    error: None,
                    message_id: Some(format!("dryrun-{}", self.receipts().len() + 1)),
                }
            }
        } else {
            let cfg = self.config.as_ref().expect("is_live implies config");
            match smtp::send_smtp(cfg, &mail).await {
                Ok(message_id) => MailReceipt {
                    to: to.clone(),
                    subject: subject.clone(),
                    status: MailSendStatus::Sent,
                    error: None,
                    message_id,
                },
                Err(e) => MailReceipt {
                    to: to.clone(),
                    subject: subject.clone(),
                    status: MailSendStatus::Failed,
                    error: Some(e.to_string()),
                    message_id: None,
                },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dry_run_enregistre() {
        let svc = MailService::dry_run_only();
        let r = svc
            .send(OutgoingMail {
                to: "parent@example.dev".into(),
                subject: "Carnet de liaison — test".into(),
                body_text: "Bonjour".into(),
                attachments: vec![],
            })
            .await
            .unwrap();
        assert_eq!(r.status, MailSendStatus::DryRun);
        assert_eq!(svc.count_with_subject_contains("Carnet"), 1);
    }
}
