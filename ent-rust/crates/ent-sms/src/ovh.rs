//! Connecteur SMS **OVH** (D-004) : API REST OVHcloud `POST /sms/{service}/jobs`.
//!
//! - Signature des requêtes : en-têtes `X-Ovh-Application`, `X-Ovh-Consumer`,
//!   `X-Ovh-Timestamp`, `X-Ovh-Signature = "$1$" + sha1_hex(AS+CK+METHODE+URL+CORPS+TS)`
//!   (documentation officielle de l'API OVH, « first steps with the OVH API »).
//! - Horloge : l'horodatage est calé sur celui du serveur OVH (`GET /auth/time`,
//!   sans authentification), l'écart est mémorisé.
//! - **Aucune clé dans le code** : tout vient de variables d'environnement
//!   (`ENT_SMS_OVH_*`). `Debug` masque les secrets.
//! - Le transport HTTP est une interface ([`OvhTransport`]) : les tests utilisent
//!   un faux transport, **aucun appel réseau réel** vers OVH.

use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::Duration;

use sha1::{Digest, Sha1};

use crate::SmsError;

/// Points d'accès OVH connus (liste fermée : pas d'URL libre = pas de SSRF).
pub fn endpoint_url(nom: &str) -> Option<&'static str> {
    match nom.trim().to_ascii_lowercase().as_str() {
        "ovh-eu" | "eu" => Some("https://eu.api.ovh.com/1.0"),
        "ovh-ca" | "ca" => Some("https://ca.api.ovh.com/1.0"),
        "ovh-us" | "us" => Some("https://api.us.ovhcloud.com/1.0"),
        _ => None,
    }
}

/// Configuration du connecteur (secrets masqués dans `Debug`).
#[derive(Clone, PartialEq, Eq)]
pub struct OvhConfig {
    pub endpoint: &'static str,
    pub application_key: String,
    pub application_secret: String,
    pub consumer_key: String,
    /// Nom du compte SMS (`sms-xx12345-1`).
    pub service_name: String,
    /// Expéditeur (alphanumérique validé chez OVH).
    pub sender: String,
    /// Messages de service (non publicitaires) : pas de mention « STOP ».
    pub no_stop_clause: bool,
}

impl std::fmt::Debug for OvhConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OvhConfig")
            .field("endpoint", &self.endpoint)
            .field("application_key", &"[masqué]")
            .field("application_secret", &"[masqué]")
            .field("consumer_key", &"[masqué]")
            .field("service_name", &self.service_name)
            .field("sender", &self.sender)
            .field("no_stop_clause", &self.no_stop_clause)
            .finish()
    }
}

/// Variables d'environnement lues (documentées dans le runbook de production).
pub const VARIABLES: [&str; 7] = [
    "ENT_SMS_OVH_ENDPOINT",
    "ENT_SMS_OVH_APPLICATION_KEY",
    "ENT_SMS_OVH_APPLICATION_SECRET",
    "ENT_SMS_OVH_CONSUMER_KEY",
    "ENT_SMS_OVH_SERVICE",
    "ENT_SMS_OVH_SENDER",
    "ENT_SMS_OVH_NO_STOP_CLAUSE",
];

fn var(nom: &str) -> Option<String> {
    std::env::var(nom)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl OvhConfig {
    /// Lit l'environnement. `Ok(None)` : aucune variable OVH posée.
    /// `Err` : configuration **incomplète ou invalide** (jamais de repli silencieux).
    pub fn from_env() -> Result<Option<Self>, String> {
        let posees = VARIABLES.iter().filter(|v| var(v).is_some()).count();
        if posees == 0 {
            return Ok(None);
        }
        let requise = |nom: &str| var(nom).ok_or_else(|| format!("{nom} manquante"));
        let endpoint_nom = var("ENT_SMS_OVH_ENDPOINT").unwrap_or_else(|| "ovh-eu".into());
        let endpoint = endpoint_url(&endpoint_nom).ok_or_else(|| {
            "ENT_SMS_OVH_ENDPOINT inconnu (ovh-eu, ovh-ca ou ovh-us)".to_string()
        })?;
        let cfg = Self {
            endpoint,
            application_key: requise("ENT_SMS_OVH_APPLICATION_KEY")?,
            application_secret: requise("ENT_SMS_OVH_APPLICATION_SECRET")?,
            consumer_key: requise("ENT_SMS_OVH_CONSUMER_KEY")?,
            service_name: requise("ENT_SMS_OVH_SERVICE")?,
            sender: requise("ENT_SMS_OVH_SENDER")?,
            no_stop_clause: !matches!(
                var("ENT_SMS_OVH_NO_STOP_CLAUSE").as_deref(),
                Some("0") | Some("false")
            ),
        };
        cfg.valider()?;
        Ok(Some(cfg))
    }

    fn valider(&self) -> Result<(), String> {
        // Le nom de service entre dans l'URL : liste blanche stricte.
        if !self
            .service_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err("ENT_SMS_OVH_SERVICE contient des caractères non autorisés".into());
        }
        if self.sender.chars().count() > 11
            || !self.sender.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return Err("ENT_SMS_OVH_SENDER : 11 caractères alphanumériques au plus".into());
        }
        Ok(())
    }
}

/// Requête HTTP à transporter.
#[derive(Debug, Clone)]
pub struct OvhRequete {
    pub methode: &'static str,
    pub url: String,
    pub en_tetes: Vec<(String, String)>,
    pub corps: String,
}

#[derive(Debug, Clone)]
pub struct OvhReponse {
    pub statut: u16,
    pub corps: String,
}

pub type FutureTransport<'a> =
    Pin<Box<dyn Future<Output = Result<OvhReponse, String>> + Send + 'a>>;

/// Transport HTTP (réel : [`TransportHttp`] ; tests : faux transport).
pub trait OvhTransport: Send + Sync {
    fn envoyer<'a>(&'a self, requete: OvhRequete) -> FutureTransport<'a>;
}

/// Transport réel (HTTPS, délai borné, pas de redirection).
pub struct TransportHttp {
    client: reqwest::Client,
}

impl TransportHttp {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .https_only(true)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { client })
    }
}

impl OvhTransport for TransportHttp {
    fn envoyer<'a>(&'a self, r: OvhRequete) -> FutureTransport<'a> {
        Box::pin(async move {
            let mut b = match r.methode {
                "GET" => self.client.get(&r.url),
                "POST" => self.client.post(&r.url).body(r.corps.clone()),
                autre => return Err(format!("méthode non gérée : {autre}")),
            };
            for (k, v) in &r.en_tetes {
                b = b.header(k, v);
            }
            let rep = b.send().await.map_err(|e| e.without_url().to_string())?;
            let statut = rep.status().as_u16();
            let corps = rep.text().await.map_err(|e| e.without_url().to_string())?;
            Ok(OvhReponse { statut, corps })
        })
    }
}

/// Signature OVH : `"$1$" + sha1_hex("AS+CK+METHODE+URL+CORPS+TIMESTAMP")`.
pub fn signature(
    application_secret: &str,
    consumer_key: &str,
    methode: &str,
    url: &str,
    corps: &str,
    horodatage: i64,
) -> String {
    let mut h = Sha1::new();
    h.update(format!(
        "{application_secret}+{consumer_key}+{methode}+{url}+{corps}+{horodatage}"
    ));
    let mut s = String::from("$1$");
    for o in h.finalize() {
        s.push_str(&format!("{o:02x}"));
    }
    s
}

/// Client OVH (un par processus).
pub struct OvhClient {
    config: OvhConfig,
    transport: Box<dyn OvhTransport>,
    /// Écart (secondes) horloge OVH − horloge locale, une fois connu.
    ecart: Mutex<Option<i64>>,
}

impl std::fmt::Debug for OvhClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OvhClient").field("config", &self.config).finish()
    }
}

fn maintenant_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl OvhClient {
    pub fn new(config: OvhConfig, transport: Box<dyn OvhTransport>) -> Self {
        Self {
            config,
            transport,
            ecart: Mutex::new(None),
        }
    }

    /// Client réel depuis l'environnement ; `Ok(None)` si OVH n'est pas configuré.
    pub fn from_env() -> Result<Option<Self>, String> {
        let Some(cfg) = OvhConfig::from_env()? else {
            return Ok(None);
        };
        Ok(Some(Self::new(cfg, Box::new(TransportHttp::new()?))))
    }

    pub fn config(&self) -> &OvhConfig {
        &self.config
    }

    async fn horodatage(&self) -> Result<i64, SmsError> {
        let connu = self.ecart.lock().ok().and_then(|g| *g);
        if let Some(e) = connu {
            return Ok(maintenant_unix() + e);
        }
        let rep = self
            .transport
            .envoyer(OvhRequete {
                methode: "GET",
                url: format!("{}/auth/time", self.config.endpoint),
                en_tetes: vec![],
                corps: String::new(),
            })
            .await
            .map_err(|e| SmsError::Provider(format!("OVH /auth/time : {e}")))?;
        let serveur: i64 = rep
            .corps
            .trim()
            .parse()
            .map_err(|_| SmsError::Provider("OVH /auth/time : réponse illisible".into()))?;
        if rep.statut != 200 {
            return Err(SmsError::Provider(format!("OVH /auth/time : HTTP {}", rep.statut)));
        }
        let ecart = serveur - maintenant_unix();
        if let Ok(mut g) = self.ecart.lock() {
            *g = Some(ecart);
        }
        Ok(serveur)
    }

    /// Envoie un SMS à un destinataire (E.164). Retourne l'identifiant OVH du job.
    pub async fn envoyer(&self, to: &str, message: &str) -> Result<String, SmsError> {
        let ts = self.horodatage().await?;
        let url = format!(
            "{}/sms/{}/jobs",
            self.config.endpoint, self.config.service_name
        );
        let corps = serde_json_minimal(
            message,
            to,
            &self.config.sender,
            self.config.no_stop_clause,
        );
        let sig = signature(
            &self.config.application_secret,
            &self.config.consumer_key,
            "POST",
            &url,
            &corps,
            ts,
        );
        let rep = self
            .transport
            .envoyer(OvhRequete {
                methode: "POST",
                url,
                en_tetes: vec![
                    ("X-Ovh-Application".into(), self.config.application_key.clone()),
                    ("X-Ovh-Consumer".into(), self.config.consumer_key.clone()),
                    ("X-Ovh-Timestamp".into(), ts.to_string()),
                    ("X-Ovh-Signature".into(), sig),
                    ("Content-Type".into(), "application/json".into()),
                ],
                corps,
            })
            .await
            .map_err(|e| SmsError::Provider(format!("OVH : {e}")))?;
        interpreter_reponse(rep.statut, &rep.corps)
    }
}

/// Corps JSON du job (sérialisation manuelle : aucune dépendance de plus).
fn serde_json_minimal(message: &str, to: &str, sender: &str, no_stop: bool) -> String {
    format!(
        "{{\"message\":{},\"receivers\":[{}],\"sender\":{},\"noStopClause\":{},\"priority\":\"high\",\"coding\":\"{}\"}}",
        json_chaine(message),
        json_chaine(to),
        json_chaine(sender),
        no_stop,
        codage_pour(message)
    )
}

/// Alphabet par défaut GSM 03.38 et son extension (euro, crochets, accolades…).
const GSM_BASE: &str = "@£$¥èéùìòÇ\nØø\rÅåΔ_ΦΓΛΩΠΨΣΘΞÆæßÉ !\"#¤%&'()*+,-./0123456789:;<=>?¡ABCDEFGHIJKLMNOPQRSTUVWXYZÄÖÑÜ§¿abcdefghijklmnopqrstuvwxyzäöñüà";
const GSM_EXT: &str = "^{}\\[~]|€\u{c}";

/// Codage choisi selon le texte (socle 8 N0b, i) : `7bit` quand tous les caractères sont dans
/// l'alphabet GSM, sinon `8bit` — un « ç » majuscule, un guillemet français ou une apostrophe
/// typographique ne sont plus déformés par un `7bit` forcé. Hypothèse à confirmer à l'essai
/// OVH : le fournisseur choisit alors lui-même l'encodage 16 bits (voir N5).
pub fn codage_pour(message: &str) -> &'static str {
    if message.chars().all(|c| GSM_BASE.contains(c) || GSM_EXT.contains(c)) {
        "7bit"
    } else {
        "8bit"
    }
}

fn json_chaine(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Interprète la réponse d'un `POST /sms/{service}/jobs`.
pub fn interpreter_reponse(statut: u16, corps: &str) -> Result<String, SmsError> {
    if !(200..300).contains(&statut) {
        // Le corps d'erreur OVH est court et sans secret, mais borné par prudence.
        let extrait: String = corps.chars().take(200).collect();
        return Err(SmsError::Provider(format!("OVH HTTP {statut} : {extrait}")));
    }
    let v: serde_json_lite::Valeur = serde_json_lite::parse(corps)
        .ok_or_else(|| SmsError::Provider("OVH : réponse JSON illisible".into()))?;
    if v.nb_valides == 0 {
        return Err(SmsError::Provider(
            "OVH : destinataire rejeté (aucun destinataire valide)".into(),
        ));
    }
    v.premier_id
        .map(|i| i.to_string())
        .ok_or_else(|| SmsError::Provider("OVH : aucun identifiant de job renvoyé".into()))
}

/// Extraction minimale de `ids` et `validReceivers` (réponse OVH).
mod serde_json_lite {
    pub struct Valeur {
        pub premier_id: Option<u64>,
        pub nb_valides: usize,
    }

    fn tableau<'a>(s: &'a str, cle: &str) -> Option<&'a str> {
        let motif = format!("\"{cle}\"");
        let d = s.find(&motif)? + motif.len();
        let reste = &s[d..];
        let o = reste.find('[')?;
        let f = reste[o..].find(']')? + o;
        Some(&reste[o + 1..f])
    }

    pub fn parse(s: &str) -> Option<Valeur> {
        let s = s.trim();
        if !s.starts_with('{') {
            return None;
        }
        let ids = tableau(s, "ids")?;
        let premier_id = ids
            .split(',')
            .map(|x| x.trim())
            .find(|x| !x.is_empty())
            .and_then(|x| x.parse::<u64>().ok());
        let valides = tableau(s, "validReceivers").unwrap_or("");
        let nb_valides = valides.split(',').filter(|x| !x.trim().is_empty()).count();
        Some(Valeur {
            premier_id,
            nb_valides,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn codage_selon_le_texte() {
        use super::codage_pour;
        assert_eq!(codage_pour("Absence de Léa le 12/11 : ok."), "7bit");
        assert_eq!(codage_pour("Total 12 € [urgent]"), "7bit");
        assert_eq!(codage_pour("Information de l’école"), "8bit");
        assert_eq!(codage_pour("« Vie scolaire »"), "8bit");
        assert_eq!(codage_pour("Le garçon est là"), "8bit");
        assert_eq!(codage_pour("ÇA va"), "7bit");
    }

    use super::*;
    use std::sync::{Arc, Mutex as StdMutex};

    #[test]
    fn signature_valeur_connue() {
        // Valeur de référence calculée indépendamment (hashlib.sha1 de Python).
        let s = signature(
            "AS",
            "CK",
            "POST",
            "https://eu.api.ovh.com/1.0/sms/sms-ab1-1/jobs",
            "{\"message\":\"Bonjour\"}",
            1366560945,
        );
        assert!(s.starts_with("$1$"));
        assert_eq!(s.len(), 3 + 40);
        assert_eq!(s, "$1$c4b0d11419e3534dd3bc7c90bf7e3a08f8d15234");
    }

    struct Faux {
        vues: Arc<StdMutex<Vec<OvhRequete>>>,
        reponse: OvhReponse,
    }
    impl OvhTransport for Faux {
        fn envoyer<'a>(&'a self, r: OvhRequete) -> FutureTransport<'a> {
            self.vues.lock().unwrap().push(r.clone());
            let rep = if r.url.ends_with("/auth/time") {
                OvhReponse {
                    statut: 200,
                    corps: "1700000000".into(),
                }
            } else {
                self.reponse.clone()
            };
            Box::pin(async move { Ok(rep) })
        }
    }

    fn cfg() -> OvhConfig {
        OvhConfig {
            endpoint: "https://eu.api.ovh.com/1.0",
            application_key: "AK".into(),
            application_secret: "AS".into(),
            consumer_key: "CK".into(),
            service_name: "sms-ab1-1".into(),
            sender: "ScolIA".into(),
            no_stop_clause: true,
        }
    }

    #[tokio::test]
    async fn envoi_signe_et_identifiant_lu() {
        let vues = Arc::new(StdMutex::new(Vec::new()));
        let c = OvhClient::new(
            cfg(),
            Box::new(Faux {
                vues: vues.clone(),
                reponse: OvhReponse {
                    statut: 200,
                    corps: "{\"totalCreditsRemoved\":1,\"invalidReceivers\":[],\"ids\":[4242],\"validReceivers\":[\"+33612345678\"]}".into(),
                },
            }),
        );
        let id = c.envoyer("+33612345678", "Bonjour \"à tous\"").await.unwrap();
        assert_eq!(id, "4242");
        let v = vues.lock().unwrap();
        assert_eq!(v.len(), 2, "horloge puis envoi");
        let post = &v[1];
        assert_eq!(post.methode, "POST");
        assert!(post.url.ends_with("/sms/sms-ab1-1/jobs"));
        let en = |n: &str| post.en_tetes.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
        assert_eq!(en("X-Ovh-Application").as_deref(), Some("AK"));
        assert_eq!(en("X-Ovh-Consumer").as_deref(), Some("CK"));
        let ts: i64 = en("X-Ovh-Timestamp").unwrap().parse().unwrap();
        assert!(ts >= 1_700_000_000, "horloge calée sur celle d'OVH");
        let attendu = signature("AS", "CK", "POST", &post.url, &post.corps, ts);
        assert_eq!(en("X-Ovh-Signature").unwrap(), attendu);
        assert!(post.corps.contains("\\\"à tous\\\""), "échappement JSON");
        assert!(post.corps.contains("\"receivers\":[\"+33612345678\"]"));
    }

    #[tokio::test]
    async fn rejets_et_erreurs() {
        let faux = |statut, corps: &str| {
            OvhClient::new(
                cfg(),
                Box::new(Faux {
                    vues: Arc::new(StdMutex::new(Vec::new())),
                    reponse: OvhReponse {
                        statut,
                        corps: corps.into(),
                    },
                }),
            )
        };
        assert!(faux(403, "{\"message\":\"This call has not been granted\"}")
            .envoyer("+33612345678", "x")
            .await
            .is_err());
        assert!(faux(200, "{\"ids\":[],\"validReceivers\":[],\"invalidReceivers\":[\"+33\"]}")
            .envoyer("+33", "x")
            .await
            .is_err());
        assert!(faux(200, "pas du json").envoyer("+33612345678", "x").await.is_err());
    }

    #[test]
    fn config_masque_les_secrets_et_valide() {
        let d = format!("{:?}", cfg());
        assert!(!d.contains("\"AS\"") && !d.contains("CK") && !d.contains("\"AK\""));
        let mut c = cfg();
        c.service_name = "sms/../x".into();
        assert!(c.valider().is_err());
        let mut c = cfg();
        c.sender = "Un expéditeur trop long".into();
        assert!(c.valider().is_err());
        assert!(endpoint_url("http://evil.example").is_none());
    }
}
