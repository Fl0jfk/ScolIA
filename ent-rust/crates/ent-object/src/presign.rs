//! Liens d'accès courts aux fichiers privés (« URL pré-signées »).
//!
//! Deux modes, choisis par `ENT_OBJECT_PRESIGN_MODE` (`local` par défaut, `s3`) :
//!
//! - **local** : jeton HMAC-SHA256 servi par `GET /fichiers/{jeton}` (disque local,
//!   dev et lab). Secret serveur `ENT_OBJECT_SIGNING_KEY` (≥ 32 octets).
//!   **Fail-closed** : sans clé valide, aucune émission, sauf si `ENT_ENV` désigne
//!   explicitement un environnement de labo/test (`lab`, `labo`, `dev`, `local`,
//!   `test`), où une clé éphémère aléatoire (propre au processus) est générée.
//! - **s3** : URL pré-signée AWS Signature V4 (query string) vers un stockage
//!   S3-compatible (Scaleway : `https://s3.{region}.scw.cloud`), avec
//!   `response-content-disposition` et `response-content-type`. Aucun appel réseau.
//!
//! Le jeton local est un *porteur* : quiconque le détient pendant sa courte durée
//! de vie peut lire le fichier. La clé objet n'y figure **plus en clair** (D0 socle 7) :
//! elle est chiffrée par un flux dérivé du secret serveur (HMAC-SHA256 en mode
//! compteur, nonce aléatoire de 16 octets) puis couverte par le MAC. Le jeton est en
//! outre lié à l'utilisateur : la route de service exige une session dont l'identifiant
//! d'utilisateur égale celui du jeton. Le jeton couvre : clé objet,
//! expiration, destinataire (user_id), établissement, disposition, nom affiché,
//! type MIME. Aucun secret n'apparaît dans `Debug` ni dans les erreurs.

use std::fmt;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::assert_safe_key;

type HmacSha256 = Hmac<Sha256>;

/// Taille minimale (octets) du secret de signature local.
pub const MIN_SIGNING_KEY_LEN: usize = 32;
/// Durée de vie par défaut d'un lien.
pub const DEFAULT_TTL_SECS: u64 = 60;
/// Plafond absolu de la durée de vie d'un lien.
pub const MAX_TTL_SECS: u64 = 900;

const TOKEN_VERSION: u8 = 2;
const MAC_DOMAIN: &[u8] = b"ent-object-presign-v1\0";
/// Domaine distinct pour le flux de chiffrement de la clé objet (jamais celui du MAC).
const KEY_STREAM_DOMAIN: &[u8] = b"ent-object-presign-keystream-v2\0";
const NONCE_LEN: usize = 16;
const MAX_FILENAME_BYTES: usize = 255;
const MAX_MIME_BYTES: usize = 127;
const ENVS_NON_PROD: [&str; 5] = ["lab", "labo", "dev", "local", "test"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresignError {
    /// Émission impossible : secret absent / invalide (fail-closed) ou configuration S3 incomplète.
    Disabled,
    /// Paramètre d'émission refusé (clé objet hostile, clé d'un autre établissement…).
    InvalidParam,
    /// Jeton illisible.
    Malformed,
    /// Signature invalide (altération, mauvais secret).
    BadSignature,
    /// Jeton expiré.
    Expired,
}

impl fmt::Display for PresignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Disabled => "liens de fichiers désactivés (configuration absente ou invalide)",
            Self::InvalidParam => "paramètre de lien invalide",
            Self::Malformed => "jeton illisible",
            Self::BadSignature => "signature invalide",
            Self::Expired => "lien expiré",
        })
    }
}

impl std::error::Error for PresignError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Inline,
    Attachment,
}

impl Disposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Attachment => "attachment",
        }
    }
}

/// Ce que l'appelant demande de signer (après contrôle des droits).
#[derive(Debug, Clone)]
pub struct LinkSpec {
    pub key: String,
    pub user_id: Uuid,
    pub etablissement_id: Uuid,
    pub disposition: Disposition,
    pub filename: String,
    pub mime: String,
}

/// Contenu vérifié d'un jeton local.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedClaims {
    pub key: String,
    pub expires_at: i64,
    pub user_id: Uuid,
    pub etablissement_id: Uuid,
    pub disposition: Disposition,
    pub filename: String,
    pub mime: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresignedLink {
    pub url: String,
    pub expires_at: i64,
}

// ─────────────────────────── Règles de contenu ───────────────────────────

/// Types servis `inline` (liste blanche). Tout le reste est forcé en `attachment`.
fn mime_inline_sur(mime: &str) -> bool {
    matches!(
        mime,
        "application/pdf" | "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "text/plain"
    )
}

/// Disposition effective : `inline` seulement pour un type de la liste blanche,
/// jamais pour html / svg / xml (types actifs) ni pour une extension active.
pub fn effective_disposition(requested: Disposition, mime: &str, filename: &str) -> Disposition {
    let m = mime.trim().to_ascii_lowercase();
    let ext = filename
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    let ext_active = matches!(
        ext.as_str(),
        "html" | "htm" | "xhtml" | "svg" | "svgz" | "xml" | "xsl" | "js" | "mjs"
    );
    if requested == Disposition::Inline && mime_inline_sur(&m) && !ext_active {
        Disposition::Inline
    } else {
        Disposition::Attachment
    }
}

/// Nom de fichier affichable : sans séparateur de chemin, guillemet ni caractère
/// de contrôle, borné à 255 octets.
pub fn sanitize_filename(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        let c = if c.is_control() || matches!(c, '/' | '\\' | '"' | ';' | '\u{2028}' | '\u{2029}') {
            '_'
        } else {
            c
        };
        if out.len() + c.len_utf8() > MAX_FILENAME_BYTES {
            break;
        }
        out.push(c);
    }
    let t = out.trim().trim_matches('.').to_string();
    if t.is_empty() {
        "fichier".into()
    } else {
        t
    }
}

/// Type MIME syntaxiquement sûr (`type/sous-type`), sinon `application/octet-stream`.
pub fn sanitize_mime(mime: &str) -> String {
    let base = mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let ok_char = |c: char| c.is_ascii_alphanumeric() || "!#$&^_.+-".contains(c);
    let valide = base.len() <= MAX_MIME_BYTES
        && base.split_once('/').is_some_and(|(t, s)| {
            !t.is_empty() && !s.is_empty() && t.chars().all(ok_char) && s.chars().all(ok_char)
        });
    if valide {
        base
    } else {
        "application/octet-stream".into()
    }
}

/// En-tête `Content-Disposition` (RFC 6266) : repli ASCII + `filename*` UTF-8.
pub fn content_disposition(disposition: Disposition, filename: &str) -> String {
    let nom = sanitize_filename(filename);
    let ascii: String = nom
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!(
        "{}; filename=\"{}\"; filename*=UTF-8''{}",
        disposition.as_str(),
        ascii,
        uri_encode(&nom, true)
    )
}

/// La clé objet appartient-elle à cet établissement ? Formes reconnues (cf.
/// `document_storage_key_is_owned` côté ent-db) : `{etab}/…`, `reinscription/{etab}/…`,
/// `lab/{8 premiers hex de l'établissement}/…`. Refuse toute clé non sûre.
pub fn key_belongs_to_etab(key: &str, etab: Uuid) -> bool {
    if assert_safe_key(key).is_err() {
        return false;
    }
    let e = etab.to_string();
    let segs: Vec<&str> = key.split('/').collect();
    match segs.as_slice() {
        ["reinscription", s, ..] => *s == e,
        ["lab", s, ..] => e.starts_with(*s) && s.len() == 8,
        [s, _, ..] => *s == e,
        _ => false,
    }
}

// ─────────────────────────── Mode local (HMAC) ───────────────────────────

#[derive(Clone)]
struct LocalSigner {
    secret: Vec<u8>,
}

impl fmt::Debug for LocalSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LocalSigner(<secret masqué>)")
    }
}

fn put_field(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    buf.extend_from_slice(bytes);
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.0.len() < n {
            return None;
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Some(a)
    }
    fn field(&mut self) -> Option<&'a [u8]> {
        let l = u16::from_be_bytes(self.take(2)?.try_into().ok()?) as usize;
        self.take(l)
    }
}

impl LocalSigner {
    fn mac(&self, payload: &[u8]) -> HmacSha256 {
        let mut m = <HmacSha256 as Mac>::new_from_slice(&self.secret)
            .expect("HMAC accepte toute longueur de clé");
        m.update(MAC_DOMAIN);
        m.update(payload);
        m
    }

    /// Flux pseudo-aléatoire (HMAC-SHA256 en compteur) lié au nonce, domaine séparé du MAC.
    fn xor_cle(&self, nonce: &[u8], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(data.len());
        for (i, bloc) in data.chunks(32).enumerate() {
            let mut m = <HmacSha256 as Mac>::new_from_slice(&self.secret)
                .expect("HMAC accepte toute longueur de clé");
            m.update(KEY_STREAM_DOMAIN);
            m.update(nonce);
            m.update(&(i as u32).to_be_bytes());
            let ks = m.finalize().into_bytes();
            out.extend(bloc.iter().zip(ks.iter()).map(|(a, b)| a ^ b));
        }
        out
    }

    fn sign(&self, c: &SignedClaims) -> String {
        let mut p = Vec::with_capacity(128);
        p.push(TOKEN_VERSION);
        p.extend_from_slice(&c.expires_at.to_be_bytes());
        p.extend_from_slice(c.user_id.as_bytes());
        p.extend_from_slice(c.etablissement_id.as_bytes());
        p.push(match c.disposition {
            Disposition::Inline => 0,
            Disposition::Attachment => 1,
        });
        let nonce = *Uuid::new_v4().as_bytes();
        debug_assert_eq!(nonce.len(), NONCE_LEN);
        p.extend_from_slice(&nonce);
        put_field(&mut p, &self.xor_cle(&nonce, c.key.as_bytes()));
        put_field(&mut p, c.filename.as_bytes());
        put_field(&mut p, c.mime.as_bytes());
        let tag = self.mac(&p).finalize().into_bytes();
        format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(&p),
            URL_SAFE_NO_PAD.encode(tag)
        )
    }

    fn verify(&self, token: &str, now: i64) -> Result<SignedClaims, PresignError> {
        if token.len() > 2048 {
            return Err(PresignError::Malformed);
        }
        let (p64, t64) = token.split_once('.').ok_or(PresignError::Malformed)?;
        let payload = URL_SAFE_NO_PAD
            .decode(p64)
            .map_err(|_| PresignError::Malformed)?;
        let tag = URL_SAFE_NO_PAD
            .decode(t64)
            .map_err(|_| PresignError::Malformed)?;
        // Comparaison en temps constant (hmac::Mac::verify_slice).
        self.mac(&payload)
            .verify_slice(&tag)
            .map_err(|_| PresignError::BadSignature)?;
        let mut r = Reader(&payload);
        if r.take(1).ok_or(PresignError::Malformed)? != [TOKEN_VERSION] {
            return Err(PresignError::Malformed);
        }
        let exp = i64::from_be_bytes(
            r.take(8)
                .ok_or(PresignError::Malformed)?
                .try_into()
                .map_err(|_| PresignError::Malformed)?,
        );
        let user_id = Uuid::from_slice(r.take(16).ok_or(PresignError::Malformed)?)
            .map_err(|_| PresignError::Malformed)?;
        let etablissement_id = Uuid::from_slice(r.take(16).ok_or(PresignError::Malformed)?)
            .map_err(|_| PresignError::Malformed)?;
        let disposition = match r.take(1).ok_or(PresignError::Malformed)? {
            [0] => Disposition::Inline,
            [1] => Disposition::Attachment,
            _ => return Err(PresignError::Malformed),
        };
        let nonce = r.take(NONCE_LEN).ok_or(PresignError::Malformed)?;
        let cle_chiffree = r.field().ok_or(PresignError::Malformed)?;
        let key = String::from_utf8(self.xor_cle(nonce, cle_chiffree))
            .map_err(|_| PresignError::Malformed)?;
        let mut s = || -> Result<String, PresignError> {
            let b = r.field().ok_or(PresignError::Malformed)?;
            String::from_utf8(b.to_vec()).map_err(|_| PresignError::Malformed)
        };
        let filename = s()?;
        let mime = s()?;
        if !r.0.is_empty() {
            return Err(PresignError::Malformed);
        }
        if exp <= now {
            return Err(PresignError::Expired);
        }
        // Défense en profondeur : même un jeton correctement signé ne sert jamais une clé hostile.
        if assert_safe_key(&key).is_err() {
            return Err(PresignError::InvalidParam);
        }
        Ok(SignedClaims {
            key,
            expires_at: exp,
            user_id,
            etablissement_id,
            disposition,
            filename,
            mime,
        })
    }
}

// ─────────────────────────── Mode S3 (SigV4) ───────────────────────────

/// Configuration S3 (le secret n'apparaît jamais dans `Debug`).
#[derive(Clone)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    secret_key: String,
    /// `scheme://host[:port]` sans slash final (ex. `https://s3.fr-par.scw.cloud`).
    pub endpoint: String,
    /// `true` : `https://{bucket}.{host}/{key}` ; `false` : `https://{host}/{bucket}/{key}`.
    pub virtual_host: bool,
}

impl S3Config {
    pub fn new(
        bucket: &str,
        region: &str,
        access_key: &str,
        secret_key: &str,
        endpoint: Option<&str>,
        virtual_host: bool,
    ) -> Result<Self, PresignError> {
        let endpoint = endpoint
            .map(|e| e.trim().trim_end_matches('/').to_string())
            .filter(|e| !e.is_empty())
            .unwrap_or_else(|| format!("https://s3.{region}.scw.cloud"));
        let host_ok = endpoint
            .strip_prefix("https://")
            .or_else(|| endpoint.strip_prefix("http://"))
            .is_some_and(|h| !h.is_empty() && !h.contains(['/', '?', '#', '@', ' ']));
        if bucket.is_empty()
            || region.is_empty()
            || access_key.is_empty()
            || secret_key.is_empty()
            || !host_ok
            || bucket.contains(['/', ' '])
        {
            return Err(PresignError::Disabled);
        }
        Ok(Self {
            bucket: bucket.into(),
            region: region.into(),
            access_key: access_key.into(),
            secret_key: secret_key.into(),
            endpoint,
            virtual_host,
        })
    }
}

impl S3Config {
    /// `(schéma, hôte[:port], chemin encodé)` de l'objet `key` (style chemin ou hôte virtuel).
    pub(crate) fn cible(&self, key: &str) -> (String, String, String) {
        let (scheme, host_part) = self
            .endpoint
            .split_once("://")
            .expect("endpoint validé à la construction");
        if self.virtual_host {
            (
                scheme.to_string(),
                format!("{}.{}", self.bucket, host_part),
                format!("/{}", uri_encode(key, false)),
            )
        } else {
            (
                scheme.to_string(),
                host_part.to_string(),
                format!(
                    "/{}/{}",
                    uri_encode(&self.bucket, false),
                    uri_encode(key, false)
                ),
            )
        }
    }

    pub(crate) fn secret(&self) -> &str {
        &self.secret_key
    }
}

impl fmt::Debug for S3Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("S3Config")
            .field("bucket", &self.bucket)
            .field("region", &self.region)
            .field("access_key", &self.access_key)
            .field("secret_key", &"<masqué>")
            .field("endpoint", &self.endpoint)
            .field("virtual_host", &self.virtual_host)
            .finish()
    }
}

pub(crate) fn hmac_raw(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut m =
        <HmacSha256 as Mac>::new_from_slice(key).expect("HMAC accepte toute longueur de clé");
    m.update(data);
    m.finalize().into_bytes().to_vec()
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(H[(b >> 4) as usize] as char);
        s.push(H[(b & 15) as usize] as char);
    }
    s
}

/// Encodage URI AWS : tout sauf `A-Za-z0-9-._~` (et `/` si `encode_slash` est faux).
pub(crate) fn uri_encode(s: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric()
            || matches!(b, b'-' | b'.' | b'_' | b'~')
            || (b == b'/' && !encode_slash)
        {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// (année, mois, jour) depuis un nombre de jours depuis 1970-01-01 (algorithme de H. Hinnant).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `(YYYYMMDD, YYYYMMDD'T'HHMMSS'Z')` en UTC.
pub(crate) fn amz_dates(unix: i64) -> (String, String) {
    let (y, m, d) = civil_from_days(unix.div_euclid(86_400));
    let s = unix.rem_euclid(86_400);
    let date = format!("{y:04}{m:02}{d:02}");
    let full = format!("{date}T{:02}{:02}{:02}Z", s / 3600, (s % 3600) / 60, s % 60);
    (date, full)
}

/// URL pré-signée `GET` SigV4 (query string). `expires` en secondes (non plafonné ici :
/// le plafond est appliqué par `Presigner`). `extra` : paramètres supplémentaires signés
/// (`response-content-disposition`, `response-content-type`…).
fn s3_presign_get(
    cfg: &S3Config,
    key: &str,
    extra: &[(&str, &str)],
    now_unix: i64,
    expires: u64,
) -> String {
    let (scheme, host_part) = cfg
        .endpoint
        .split_once("://")
        .expect("endpoint validé à la construction");
    let (host, path) = if cfg.virtual_host {
        (
            format!("{}.{}", cfg.bucket, host_part),
            format!("/{}", uri_encode(key, false)),
        )
    } else {
        (
            host_part.to_string(),
            format!(
                "/{}/{}",
                uri_encode(&cfg.bucket, false),
                uri_encode(key, false)
            ),
        )
    };
    let (date, amz_date) = amz_dates(now_unix);
    let scope = format!("{date}/{}/s3/aws4_request", cfg.region);
    let credential = format!("{}/{}", cfg.access_key, scope);

    let mut params: Vec<(String, String)> = vec![
        ("X-Amz-Algorithm".into(), "AWS4-HMAC-SHA256".into()),
        ("X-Amz-Credential".into(), credential),
        ("X-Amz-Date".into(), amz_date.clone()),
        ("X-Amz-Expires".into(), expires.to_string()),
        ("X-Amz-SignedHeaders".into(), "host".into()),
    ];
    for (k, v) in extra {
        params.push(((*k).to_string(), (*v).to_string()));
    }
    let mut enc: Vec<(String, String)> = params
        .iter()
        .map(|(k, v)| (uri_encode(k, true), uri_encode(v, true)))
        .collect();
    enc.sort();
    let canonical_query = enc
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    let canonical_request =
        format!("GET\n{path}\n{canonical_query}\nhost:{host}\n\nhost\nUNSIGNED-PAYLOAD");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        hex(&Sha256::digest(canonical_request.as_bytes()))
    );
    let k_date = hmac_raw(
        format!("AWS4{}", cfg.secret_key).as_bytes(),
        date.as_bytes(),
    );
    let k_region = hmac_raw(&k_date, cfg.region.as_bytes());
    let k_service = hmac_raw(&k_region, b"s3");
    let k_signing = hmac_raw(&k_service, b"aws4_request");
    let signature = hex(&hmac_raw(&k_signing, string_to_sign.as_bytes()));

    format!("{scheme}://{host}{path}?{canonical_query}&X-Amz-Signature={signature}")
}

// ─────────────────────────── Façade ───────────────────────────

#[derive(Clone)]
enum Backend {
    Disabled,
    Local(LocalSigner),
    S3(S3Config),
}

/// Émetteur de liens courts. Se construit une fois (`from_env`) et se clone à bas coût.
#[derive(Clone)]
pub struct Presigner {
    backend: Backend,
    ttl_secs: u64,
    public_base: Option<String>,
}

impl fmt::Debug for Presigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode = match &self.backend {
            Backend::Disabled => "désactivé",
            Backend::Local(_) => "local",
            Backend::S3(_) => "s3",
        };
        f.debug_struct("Presigner")
            .field("mode", &mode)
            .field("ttl_secs", &self.ttl_secs)
            .finish()
    }
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn clamp_ttl(ttl: u64) -> u64 {
    ttl.clamp(1, MAX_TTL_SECS)
}

impl Presigner {
    /// Émission désactivée (fail-closed).
    pub fn disabled() -> Self {
        Self {
            backend: Backend::Disabled,
            ttl_secs: DEFAULT_TTL_SECS,
            public_base: None,
        }
    }

    /// Mode local avec un secret explicite (≥ 32 octets), durée plafonnée à 15 min.
    pub fn local(secret: &[u8], ttl_secs: u64) -> Result<Self, PresignError> {
        if secret.len() < MIN_SIGNING_KEY_LEN {
            return Err(PresignError::Disabled);
        }
        Ok(Self {
            backend: Backend::Local(LocalSigner {
                secret: secret.to_vec(),
            }),
            ttl_secs: clamp_ttl(ttl_secs),
            public_base: None,
        })
    }

    /// Mode S3 (SigV4).
    pub fn s3(cfg: S3Config, ttl_secs: u64) -> Self {
        Self {
            backend: Backend::S3(cfg),
            ttl_secs: clamp_ttl(ttl_secs),
            public_base: None,
        }
    }

    pub fn with_public_base(mut self, base: Option<&str>) -> Self {
        self.public_base = base
            .map(|b| b.trim().trim_end_matches('/').to_string())
            .filter(|b| !b.is_empty());
        self
    }

    pub fn from_env() -> Self {
        Self::from_vars(|k| std::env::var(k).ok())
    }

    /// Construction depuis un lecteur de variables (testable sans toucher l'environnement).
    pub fn from_vars(get: impl Fn(&str) -> Option<String>) -> Self {
        let var = |k: &str| get(k).filter(|v| !v.trim().is_empty());
        let ttl = var("ENT_OBJECT_LINK_TTL_SECS")
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_TTL_SECS);
        let base = var("ENT_OBJECT_PUBLIC_BASE_URL");
        let mode = var("ENT_OBJECT_PRESIGN_MODE")
            .map(|m| m.trim().to_ascii_lowercase())
            .unwrap_or_else(|| "local".into());
        match mode.as_str() {
            "s3" => {
                let virtual_host = var("ENT_OBJECT_S3_VIRTUAL_HOST").is_some_and(|v| {
                    matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes")
                });
                let cfg = S3Config::new(
                    var("ENT_OBJECT_S3_BUCKET")
                        .as_deref()
                        .unwrap_or("scola-ent"),
                    var("ENT_OBJECT_S3_REGION").as_deref().unwrap_or("fr-par"),
                    var("ENT_OBJECT_S3_ACCESS_KEY").as_deref().unwrap_or(""),
                    var("ENT_OBJECT_S3_SECRET_KEY").as_deref().unwrap_or(""),
                    var("ENT_OBJECT_S3_ENDPOINT").as_deref(),
                    virtual_host,
                );
                match cfg {
                    Ok(c) => Self::s3(c, ttl),
                    Err(_) => Self::disabled(),
                }
            }
            "local" => {
                if let Some(k) = get("ENT_OBJECT_SIGNING_KEY") {
                    if let Ok(p) = Self::local(k.as_bytes(), ttl) {
                        return p.with_public_base(base.as_deref());
                    }
                }
                let non_prod = var("ENT_ENV")
                    .map(|e| e.trim().to_ascii_lowercase())
                    .is_some_and(|e| ENVS_NON_PROD.contains(&e.as_str()));
                if non_prod {
                    // Clé éphémère propre au processus (labo/test uniquement).
                    let mut k = Vec::with_capacity(32);
                    k.extend_from_slice(Uuid::new_v4().as_bytes());
                    k.extend_from_slice(Uuid::new_v4().as_bytes());
                    if let Ok(p) = Self::local(&k, ttl) {
                        return p.with_public_base(base.as_deref());
                    }
                }
                Self::disabled()
            }
            _ => Self::disabled(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self.backend, Backend::Disabled)
    }

    /// `true` si les liens émis sont servis par `GET /fichiers/{jeton}` de cette API.
    pub fn is_local(&self) -> bool {
        matches!(self.backend, Backend::Local(_))
    }

    pub fn ttl_secs(&self) -> u64 {
        self.ttl_secs
    }

    /// Émet un lien. La clé doit être sûre et appartenir à l'établissement du destinataire.
    /// La disposition est durcie (`effective_disposition`), nom et type assainis.
    pub fn issue(&self, spec: &LinkSpec, now: i64) -> Result<PresignedLink, PresignError> {
        if !self.is_enabled() {
            return Err(PresignError::Disabled);
        }
        if !key_belongs_to_etab(&spec.key, spec.etablissement_id) {
            return Err(PresignError::InvalidParam);
        }
        let filename = sanitize_filename(&spec.filename);
        let mime = sanitize_mime(&spec.mime);
        let disposition = effective_disposition(spec.disposition, &mime, &filename);
        let expires_at = now + self.ttl_secs as i64;
        match &self.backend {
            Backend::Disabled => Err(PresignError::Disabled),
            Backend::Local(l) => {
                let token = l.sign(&SignedClaims {
                    key: spec.key.clone(),
                    expires_at,
                    user_id: spec.user_id,
                    etablissement_id: spec.etablissement_id,
                    disposition,
                    filename,
                    mime,
                });
                let path = format!("/fichiers/{token}");
                let url = match &self.public_base {
                    Some(b) => format!("{b}{path}"),
                    None => path,
                };
                Ok(PresignedLink { url, expires_at })
            }
            Backend::S3(cfg) => {
                let cd = content_disposition(disposition, &filename);
                let url = s3_presign_get(
                    cfg,
                    &spec.key,
                    &[
                        ("response-content-disposition", &cd),
                        ("response-content-type", &mime),
                    ],
                    now,
                    self.ttl_secs,
                );
                Ok(PresignedLink { url, expires_at })
            }
        }
    }

    /// Vérifie un jeton local (signature, expiration, clé sûre). Sans état, sans I/O.
    /// La disposition renvoyée est re-durcie.
    pub fn verify_local(&self, token: &str, now: i64) -> Result<SignedClaims, PresignError> {
        let Backend::Local(l) = &self.backend else {
            return Err(PresignError::Disabled);
        };
        let mut c = l.verify(token, now)?;
        c.disposition = effective_disposition(c.disposition, &c.mime, &c.filename);
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"0123456789abcdef0123456789abcdef-secret";
    const NOW: i64 = 1_800_000_000;

    fn spec() -> LinkSpec {
        let etab = Uuid::new_v4();
        LinkSpec {
            key: format!("{etab}/documents/{}.pdf", Uuid::new_v4()),
            user_id: Uuid::new_v4(),
            etablissement_id: etab,
            disposition: Disposition::Inline,
            filename: "Bulletin T1.pdf".into(),
            mime: "application/pdf".into(),
        }
    }

    fn token_de(p: &Presigner, s: &LinkSpec) -> String {
        p.issue(s, NOW)
            .unwrap()
            .url
            .trim_start_matches("/fichiers/")
            .to_string()
    }

    #[test]
    fn jeton_valide_et_contenu() {
        let p = Presigner::local(SECRET, 60).unwrap();
        let s = spec();
        let l = p.issue(&s, NOW).unwrap();
        assert_eq!(l.expires_at, NOW + 60);
        assert!(l.url.starts_with("/fichiers/"));
        let c = p
            .verify_local(l.url.trim_start_matches("/fichiers/"), NOW + 59)
            .unwrap();
        assert_eq!(c.key, s.key);
        assert_eq!(c.user_id, s.user_id);
        assert_eq!(c.etablissement_id, s.etablissement_id);
        assert_eq!(c.disposition, Disposition::Inline);
        assert_eq!(c.filename, "Bulletin T1.pdf");
        assert_eq!(c.mime, "application/pdf");
    }

    #[test]
    fn jeton_ne_revele_plus_la_cle_objet() {
        let p = Presigner::local(SECRET, 60).unwrap();
        let s = spec();
        let t = token_de(&p, &s);
        let (charge, _) = t.split_once('.').unwrap();
        let octets = URL_SAFE_NO_PAD.decode(charge).unwrap();
        let contient = |h: &[u8], n: &[u8]| h.windows(n.len()).any(|w| w == n);
        // Ni en clair dans la charge décodée, ni sous forme de segments identifiants.
        assert!(!contient(&octets, s.key.as_bytes()));
        assert!(!contient(&octets, b"documents"));
        assert!(!t.contains(&s.key));
        // Deux jetons pour la même clé diffèrent (nonce) et rendent la même clé.
        let t2 = token_de(&p, &s);
        assert_ne!(t, t2);
        assert_eq!(p.verify_local(&t, NOW + 1).unwrap().key, s.key);
        assert_eq!(p.verify_local(&t2, NOW + 1).unwrap().key, s.key);
        assert_eq!(p.verify_local(&t, NOW + 1).unwrap().user_id, s.user_id);
    }

    #[test]
    fn jeton_expire() {
        let p = Presigner::local(SECRET, 60).unwrap();
        let t = token_de(&p, &spec());
        assert_eq!(p.verify_local(&t, NOW + 60), Err(PresignError::Expired));
        assert_eq!(p.verify_local(&t, NOW + 3600), Err(PresignError::Expired));
    }

    #[test]
    fn mauvais_secret_refuse() {
        let p = Presigner::local(SECRET, 60).unwrap();
        let autre = Presigner::local(b"un-tout-autre-secret-de-32-octets-ou-plus!", 60).unwrap();
        let t = token_de(&p, &spec());
        assert_eq!(autre.verify_local(&t, NOW), Err(PresignError::BadSignature));
    }

    #[test]
    fn alteration_de_chaque_champ_refusee() {
        let p = Presigner::local(SECRET, 60).unwrap();
        let t = token_de(&p, &spec());
        let (p64, t64) = t.split_once('.').unwrap();
        let payload = URL_SAFE_NO_PAD.decode(p64).unwrap();
        // Chaque octet du contenu (expiration, user, établissement, disposition,
        // clé, nom, type) modifié → signature invalide.
        for i in 0..payload.len() {
            let mut m = payload.clone();
            m[i] ^= 0x01;
            let forge = format!("{}.{}", URL_SAFE_NO_PAD.encode(&m), t64);
            assert!(
                matches!(
                    p.verify_local(&forge, NOW),
                    Err(PresignError::BadSignature) | Err(PresignError::Malformed)
                ),
                "octet {i} altéré accepté"
            );
        }
        // Signature tronquée / étendue / vide.
        let tag = URL_SAFE_NO_PAD.decode(t64).unwrap();
        for tag_alt in [&tag[..31], &[tag.clone(), vec![0]].concat()[..], &[][..]] {
            let forge = format!("{p64}.{}", URL_SAFE_NO_PAD.encode(tag_alt));
            assert!(p.verify_local(&forge, NOW).is_err());
        }
        for j in ["", ".", "a", "a.b", "....", "é.é"] {
            assert!(p.verify_local(j, NOW).is_err(), "{j:?}");
        }
        assert!(p.verify_local(&"A".repeat(5000), NOW).is_err());
    }

    #[test]
    fn jeton_signe_avec_cle_hostile_refuse_a_la_verification() {
        let l = LocalSigner {
            secret: SECRET.to_vec(),
        };
        let p = Presigner::local(SECRET, 60).unwrap();
        let etab = Uuid::new_v4();
        for key in ["../secret", "a/../b", "/etc/passwd", "C:/x", "a\\b", ""] {
            let t = l.sign(&SignedClaims {
                key: key.into(),
                expires_at: NOW + 60,
                user_id: Uuid::new_v4(),
                etablissement_id: etab,
                disposition: Disposition::Attachment,
                filename: "x".into(),
                mime: "text/plain".into(),
            });
            assert_eq!(
                p.verify_local(&t, NOW),
                Err(PresignError::InvalidParam),
                "{key:?}"
            );
        }
    }

    #[test]
    fn emission_refuse_cle_hostile_ou_autre_etablissement() {
        let p = Presigner::local(SECRET, 60).unwrap();
        let mut s = spec();
        for key in ["../x", "a/../b", "/abs", "x\0y", "x:y"] {
            s.key = key.into();
            assert_eq!(p.issue(&s, NOW), Err(PresignError::InvalidParam), "{key:?}");
        }
        let mut s = spec();
        s.key = format!("{}/documents/{}.pdf", Uuid::new_v4(), Uuid::new_v4());
        assert_eq!(p.issue(&s, NOW), Err(PresignError::InvalidParam));
    }

    #[test]
    fn appartenance_etablissement() {
        let e = Uuid::new_v4();
        let autre = Uuid::new_v4();
        let id = Uuid::new_v4();
        assert!(key_belongs_to_etab(&format!("{e}/documents/{id}.pdf"), e));
        assert!(!key_belongs_to_etab(
            &format!("{e}/documents/{id}.pdf"),
            autre
        ));
        assert!(key_belongs_to_etab(
            &format!("reinscription/{e}/{id}/X.pdf"),
            e
        ));
        assert!(!key_belongs_to_etab(
            &format!("reinscription/{e}/{id}/X.pdf"),
            autre
        ));
        let court = &e.to_string()[..8];
        assert!(key_belongs_to_etab(&format!("lab/{court}/x.pdf"), e));
        assert!(!key_belongs_to_etab(&format!("lab/{court}/x.pdf"), autre));
        assert!(!key_belongs_to_etab(
            &format!("lab/{}/x.pdf", &e.to_string()[..4]),
            e
        ));
        assert!(!key_belongs_to_etab(&format!("{e}/../{autre}/x.pdf"), e));
        assert!(!key_belongs_to_etab(&e.to_string(), e));
    }

    #[test]
    fn types_actifs_forces_en_attachment() {
        let p = Presigner::local(SECRET, 60).unwrap();
        for (mime, nom) in [
            ("text/html", "a.html"),
            ("image/svg+xml", "a.svg"),
            ("application/xml", "a.xml"),
            ("application/pdf", "piege.html"),
            ("application/octet-stream", "a.bin"),
            ("TEXT/HTML; charset=utf-8", "a.pdf"),
        ] {
            let mut s = spec();
            s.mime = mime.into();
            s.filename = nom.into();
            let t = token_de(&p, &s);
            let c = p.verify_local(&t, NOW).unwrap();
            assert_eq!(c.disposition, Disposition::Attachment, "{mime} {nom}");
        }
    }

    #[test]
    fn nom_et_mime_assainis() {
        assert_eq!(sanitize_filename("../a\"b\r\nc.pdf"), "_a_b__c.pdf");
        assert_eq!(sanitize_filename(""), "fichier");
        assert_eq!(sanitize_filename("..."), "fichier");
        assert!(sanitize_filename(&"é".repeat(400)).len() <= 255);
        assert_eq!(sanitize_mime("application/pdf"), "application/pdf");
        assert_eq!(sanitize_mime("Text/Plain; charset=utf-8"), "text/plain");
        assert_eq!(
            sanitize_mime("x\r\nSet-Cookie: a=b"),
            "application/octet-stream"
        );
        assert_eq!(sanitize_mime("pas-un-mime"), "application/octet-stream");
        let cd = content_disposition(Disposition::Attachment, "Relevé \"é\".pdf");
        assert!(cd.starts_with("attachment; filename=\""));
        assert!(
            cd.contains("filename*=UTF-8''Relev%C3%A9%20_%C3%A9_.pdf"),
            "{cd}"
        );
        assert!(!cd.contains('\r') && !cd.contains('\n'));
    }

    #[test]
    fn plafond_et_defaut_de_duree() {
        assert_eq!(
            Presigner::local(SECRET, 100_000).unwrap().ttl_secs(),
            MAX_TTL_SECS
        );
        assert_eq!(Presigner::local(SECRET, 0).unwrap().ttl_secs(), 1);
        let p = Presigner::from_vars(|k| match k {
            "ENT_OBJECT_SIGNING_KEY" => Some(String::from_utf8_lossy(SECRET).into()),
            _ => None,
        });
        assert_eq!(p.ttl_secs(), DEFAULT_TTL_SECS);
        let p = Presigner::from_vars(|k| match k {
            "ENT_OBJECT_SIGNING_KEY" => Some(String::from_utf8_lossy(SECRET).into()),
            "ENT_OBJECT_LINK_TTL_SECS" => Some("99999".into()),
            _ => None,
        });
        assert_eq!(p.ttl_secs(), MAX_TTL_SECS);
    }

    #[test]
    fn fail_closed_sans_cle_valide() {
        let vars = |env: Option<&'static str>, cle: Option<&'static str>| {
            Presigner::from_vars(move |k| match k {
                "ENT_ENV" => env.map(String::from),
                "ENT_OBJECT_SIGNING_KEY" => cle.map(String::from),
                _ => None,
            })
        };
        // Production / inconnu / absent : pas d'émission.
        for env in [
            None,
            Some("prod"),
            Some("production"),
            Some(""),
            Some("staging"),
        ] {
            let p = vars(env, None);
            assert!(!p.is_enabled(), "{env:?}");
            assert_eq!(p.issue(&spec(), NOW), Err(PresignError::Disabled));
            assert_eq!(p.verify_local("a.b", NOW), Err(PresignError::Disabled));
        }
        // Clé trop courte en production : toujours désactivé.
        assert!(!vars(Some("prod"), Some("trop-court")).is_enabled());
        // Labo / test : clé éphémère.
        for env in ["lab", "LABO", " dev ", "local", "test"] {
            let p = vars(Some(env), None);
            assert!(p.is_enabled() && p.is_local(), "{env}");
            let s = spec();
            let t = token_de(&p, &s);
            assert!(p.verify_local(&t, NOW).is_ok());
        }
        // Deux clés éphémères sont indépendantes.
        let (a, b) = (vars(Some("test"), None), vars(Some("test"), None));
        let t = token_de(&a, &spec());
        assert_eq!(b.verify_local(&t, NOW), Err(PresignError::BadSignature));
        // Clé valide fournie en production : actif.
        assert!(vars(Some("prod"), Some("0123456789abcdef0123456789abcdef")).is_enabled());
    }

    #[test]
    fn base_publique_et_mode_inconnu() {
        let p = Presigner::from_vars(|k| match k {
            "ENT_OBJECT_SIGNING_KEY" => Some("0123456789abcdef0123456789abcdef".into()),
            "ENT_OBJECT_PUBLIC_BASE_URL" => Some("https://ent.example.fr/".into()),
            _ => None,
        });
        assert!(p
            .issue(&spec(), NOW)
            .unwrap()
            .url
            .starts_with("https://ent.example.fr/fichiers/"));
        let p = Presigner::from_vars(|k| match k {
            "ENT_OBJECT_PRESIGN_MODE" => Some("n-importe-quoi".into()),
            "ENT_ENV" => Some("test".into()),
            _ => None,
        });
        assert!(!p.is_enabled());
    }

    #[test]
    fn debug_ne_divulgue_aucun_secret() {
        let p = Presigner::local(SECRET, 60).unwrap();
        let d = format!("{p:?}");
        assert!(
            !d.contains("secret") && !d.contains("0123456789abcdef"),
            "{d}"
        );
        let cfg = S3Config::new("b", "fr-par", "AK", "SUPER-SECRET-KEY", None, false).unwrap();
        let d = format!("{cfg:?} {:?}", Presigner::s3(cfg.clone(), 60));
        assert!(!d.contains("SUPER-SECRET-KEY"), "{d}");
    }

    // ───────────── SigV4 ─────────────

    /// Vecteur officiel AWS « Authenticating Requests: Using Query Parameters » (GET object).
    #[test]
    fn sigv4_vecteur_officiel_aws() {
        let cfg = S3Config::new(
            "examplebucket",
            "us-east-1",
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            Some("https://s3.amazonaws.com"),
            true,
        )
        .unwrap();
        // 2013-05-24T00:00:00Z
        let url = s3_presign_get(&cfg, "test.txt", &[], 1_369_353_600, 86_400);
        assert_eq!(
            url,
            "https://examplebucket.s3.amazonaws.com/test.txt\
             ?X-Amz-Algorithm=AWS4-HMAC-SHA256\
             &X-Amz-Credential=AKIAIOSFODNN7EXAMPLE%2F20130524%2Fus-east-1%2Fs3%2Faws4_request\
             &X-Amz-Date=20130524T000000Z\
             &X-Amz-Expires=86400\
             &X-Amz-SignedHeaders=host\
             &X-Amz-Signature=aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404"
        );
    }

    #[test]
    fn dates_amz() {
        assert_eq!(
            amz_dates(1_369_353_600),
            ("20130524".into(), "20130524T000000Z".into())
        );
        assert_eq!(amz_dates(0), ("19700101".into(), "19700101T000000Z".into()));
        // 2024-02-29T23:59:59Z (année bissextile)
        assert_eq!(amz_dates(1_709_251_199).1, "20240229T235959Z");
    }

    #[test]
    fn s3_path_style_scaleway_avec_reponse_signee() {
        let cfg = S3Config::new("scola-ent", "fr-par", "SCWAK", "scw-secret", None, false).unwrap();
        let p = Presigner::s3(cfg, 60);
        let mut s = spec();
        s.disposition = Disposition::Attachment;
        s.filename = "Relevé 1.pdf".into();
        let l = p.issue(&s, 1_369_353_600).unwrap();
        assert_eq!(l.expires_at, 1_369_353_660);
        assert!(l.url.starts_with(&format!(
            "https://s3.fr-par.scw.cloud/scola-ent/{}/documents/",
            s.etablissement_id
        )));
        assert!(l.url.contains("X-Amz-Expires=60"));
        assert!(l
            .url
            .contains("&response-content-disposition=attachment%3B%20filename%3D%22Relev"));
        assert!(l.url.contains("&response-content-type=application%2Fpdf&"));
        assert!(l.url.contains("&X-Amz-Signature="));
        assert!(!l.url.contains("scw-secret"));
        // Paramètres triés (ordre octet) : Credential < Date < Expires < SignedHeaders < response-*.
        let q = l.url.split_once('?').unwrap().1;
        let noms: Vec<&str> = q
            .split('&')
            .map(|kv| kv.split('=').next().unwrap())
            .collect();
        let mut tries = noms.clone();
        tries[..noms.len() - 1].sort();
        assert_eq!(noms[..noms.len() - 1], tries[..noms.len() - 1]);
        // Un changement de nom de fichier change la signature.
        let mut s2 = s.clone();
        s2.filename = "autre.pdf".into();
        let l2 = p.issue(&s2, 1_369_353_600).unwrap();
        let sig = |u: &str| u.rsplit_once("X-Amz-Signature=").unwrap().1.to_string();
        assert_ne!(sig(&l.url), sig(&l2.url));
    }

    #[test]
    fn s3_virtual_host_et_config_invalide() {
        let cfg = S3Config::new("scola-ent", "nl-ams", "AK", "SK", None, true).unwrap();
        let u = s3_presign_get(&cfg, "a/b c.pdf", &[], 0, 60);
        assert!(
            u.starts_with("https://scola-ent.s3.nl-ams.scw.cloud/a/b%20c.pdf?"),
            "{u}"
        );
        for (b, r, a, s, e) in [
            ("", "r", "a", "s", None),
            ("b", "", "a", "s", None),
            ("b", "r", "", "s", None),
            ("b", "r", "a", "", None),
            ("b", "r", "a", "s", Some("ftp://x")),
            ("b", "r", "a", "s", Some("https://x/evil?")),
        ] {
            assert!(S3Config::new(b, r, a, s, e, false).is_err());
        }
        let p = Presigner::from_vars(|k| (k == "ENT_OBJECT_PRESIGN_MODE").then(|| "s3".into()));
        assert!(!p.is_enabled(), "S3 sans identifiants : fail-closed");
    }
}
