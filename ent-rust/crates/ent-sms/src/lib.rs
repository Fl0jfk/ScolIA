//! SMS plateforme ENT.
//!
//! - Lab : `ENT_SMS_DRY_RUN=1` (défaut recommandé) → journal mémoire, pas d'API.
//! - Live : `ENT_SMS_LIVE=1` + **OVH** (D-004, [`ovh`], variables `ENT_SMS_OVH_*`)
//!   ou, à défaut, `SMS_PROVIDER_URL` / `SMS_API_KEY` / `SMS_FROM` (provider
//!   générique, non branché).
//! - Fail-closed : [`SmsService::from_env_strict`] refuse de démarrer hors labo si
//!   `ENT_SMS_LIVE=1` sans configuration OVH valide (jamais de repli silencieux).
//!
//! Un échec SMS ne doit **jamais** faire échouer le geste métier.

pub mod ovh;

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

#[derive(Clone)]
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
    ovh: Option<Arc<ovh::OvhClient>>,
    /// Erreur de configuration OVH relevée à la lecture de l'environnement.
    erreur_config: Option<String>,
    log: Arc<Mutex<Vec<SmsReceipt>>>,
}

impl Default for SmsService {
    fn default() -> Self {
        Self::from_env()
    }
}

impl SmsService {
    pub fn from_env() -> Self {
        let (ovh, erreur_config) = match ovh::OvhClient::from_env() {
            Ok(c) => (c.map(Arc::new), None),
            Err(e) => (None, Some(e)),
        };
        Self {
            config: SmsConfig::from_env(),
            live: env_flag("ENT_SMS_LIVE"),
            force_dry_run: env_flag("ENT_SMS_DRY_RUN"),
            ovh,
            erreur_config,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Démarrage **fail-closed** : si `ENT_SMS_LIVE=1` (envoi réel demandé) et que
    /// la configuration OVH est absente ou invalide, hors labo, le démarrage est
    /// refusé au lieu de retomber silencieusement en mode simulé. Sans
    /// `ENT_SMS_LIVE`, le mode simulé reste le défaut.
    pub fn from_env_strict(non_prod: bool) -> Result<Self, String> {
        let s = Self::from_env();
        if s.live && !s.force_dry_run && !non_prod {
            if let Some(e) = &s.erreur_config {
                return Err(format!("ENT_SMS_LIVE=1 mais configuration OVH invalide : {e}"));
            }
            if s.ovh.is_none() {
                return Err(
                    "ENT_SMS_LIVE=1 mais aucune configuration OVH (variables ENT_SMS_OVH_*)".into(),
                );
            }
        }
        Ok(s)
    }

    /// Service branché sur un client OVH donné (tests : faux transport).
    pub fn avec_ovh(client: ovh::OvhClient, live: bool) -> Self {
        Self {
            config: None,
            live,
            force_dry_run: false,
            ovh: Some(Arc::new(client)),
            erreur_config: None,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Erreur de configuration relevée à l'initialisation, s'il y en a une.
    pub fn erreur_config(&self) -> Option<&str> {
        self.erreur_config.as_deref()
    }

    /// Fournisseur effectivement utilisé pour un envoi réel.
    pub fn fournisseur(&self) -> &'static str {
        if self.ovh.is_some() {
            "ovh"
        } else if self.config.is_some() {
            "generique"
        } else {
            "aucun"
        }
    }

    pub fn dry_run_only() -> Self {
        Self {
            config: SmsConfig::from_env(),
            live: false,
            force_dry_run: true,
            ovh: None,
            erreur_config: None,
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.force_dry_run || self.config.is_some() || self.ovh.is_some()
    }

    pub fn is_live(&self) -> bool {
        self.live && !self.force_dry_run && (self.config.is_some() || self.ovh.is_some())
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
            let envoi = match (&self.ovh, &self.config) {
                (Some(ovh), _) => ovh.envoyer(&to, &body).await.map(Some),
                (None, Some(cfg)) => send_provider(cfg, &to, &body).await,
                (None, None) => unreachable!("is_live implique un fournisseur"),
            };
            match envoi {
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
        } else if self.config.is_none() && self.ovh.is_none() && !self.force_dry_run {
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

impl std::fmt::Debug for SmsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmsConfig")
            .field("provider_url", &self.provider_url)
            .field("api_key", &"[masqué]")
            .field("from", &self.from)
            .finish()
    }
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

    struct FauxOvh;
    impl ovh::OvhTransport for FauxOvh {
        fn envoyer<'a>(&'a self, r: ovh::OvhRequete) -> ovh::FutureTransport<'a> {
            let corps = if r.url.ends_with("/auth/time") {
                "1700000000".to_string()
            } else {
                "{\"ids\":[77],\"validReceivers\":[\"+33612345678\"],\"invalidReceivers\":[]}".to_string()
            };
            Box::pin(async move { Ok(ovh::OvhReponse { statut: 200, corps }) })
        }
    }

    fn client_ovh() -> ovh::OvhClient {
        ovh::OvhClient::new(
            ovh::OvhConfig {
                endpoint: "https://eu.api.ovh.com/1.0",
                application_key: "AK".into(),
                application_secret: "AS".into(),
                consumer_key: "CK".into(),
                service_name: "sms-ab1-1".into(),
                sender: "ScolIA".into(),
                no_stop_clause: true,
            },
            Box::new(FauxOvh),
        )
    }

    #[tokio::test]
    async fn ovh_live_via_faux_transport_et_simule_par_defaut() {
        let live = SmsService::avec_ovh(client_ovh(), true);
        assert!(live.is_live());
        assert_eq!(live.fournisseur(), "ovh");
        let r = live
            .send(OutgoingSms { to: "0612345678".into(), body: "Réunion jeudi".into() })
            .await
            .unwrap();
        assert_eq!(r.status, SmsSendStatus::Sent);
        assert_eq!(r.message_id.as_deref(), Some("77"));
        // Sans ENT_SMS_LIVE : configuré mais simulé (aucun appel au transport).
        let simule = SmsService::avec_ovh(client_ovh(), false);
        assert!(!simule.is_live());
        let r = simule
            .send(OutgoingSms { to: "0612345678".into(), body: "x".into() })
            .await
            .unwrap();
        assert_eq!(r.status, SmsSendStatus::DryRun);
    }
}
