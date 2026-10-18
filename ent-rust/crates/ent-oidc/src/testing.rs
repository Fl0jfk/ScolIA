//! IdP de test **en mémoire** : clé RSA éphémère, JWKS, découverte et
//! signature d'`id_token` RS256.
//!
//! Vit dans le crate (et pas dans un `#[cfg(test)]`) pour que les tests L1
//! (`ent-db`) et L2 (`ent-api`) pilotent le **même** IdP sur le **vrai** code de
//! production. Aucune socket : pas de réseau dans les tests.
//!
//! Labo/tests uniquement — jamais utilisé par le runtime de production.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::Signer;
use sha2::{Digest, Sha256};

use crate::jwt::Jwks;
use crate::transport::OidcTransport;
use crate::OidcError;

/// Expiration très lointaine (2100) : les id_tokens du faux IdP restent valides
/// quel que soit l'horloge du test.
const FAR_FUTURE_EXP: i64 = 4_102_444_800;

#[derive(Clone)]
pub struct TestIdp {
    key: Arc<Rsa<openssl::pkey::Private>>,
    pub kid: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub subject: String,
    pub email: String,
    pub name: String,
}

impl TestIdp {
    pub fn new() -> Self {
        let key = Rsa::generate(2048).expect("clé RSA de test");
        Self {
            key: Arc::new(key),
            kid: "test-key-1".to_string(),
            issuer: "https://idp.test/oidc".to_string(),
            client_id: "client-ent".to_string(),
            client_secret: "secret-ent".to_string(),
            redirect_uri: "http://localhost:3000/auth/oidc/callback".to_string(),
            subject: "idp-subject-42".to_string(),
            email: "claire@idp.test".to_string(),
            name: "Claire IdP".to_string(),
        }
    }

    pub fn authorization_endpoint(&self) -> String {
        format!("{}/authorize", self.issuer)
    }
    pub fn token_endpoint(&self) -> String {
        format!("{}/token", self.issuer)
    }
    pub fn jwks_uri(&self) -> String {
        format!("{}/jwks", self.issuer)
    }

    pub fn jwks_value(&self) -> serde_json::Value {
        let n = self.key.n().to_vec();
        let e = self.key.e().to_vec();
        serde_json::json!({
            "keys": [{
                "kty": "RSA",
                "use": "sig",
                "alg": "RS256",
                "kid": self.kid,
                "n": URL_SAFE_NO_PAD.encode(n),
                "e": URL_SAFE_NO_PAD.encode(e),
            }]
        })
    }

    pub fn jwks(&self) -> Jwks {
        serde_json::from_value(self.jwks_value()).expect("JWKS de test")
    }

    pub fn discovery_value(&self) -> serde_json::Value {
        serde_json::json!({
            "issuer": self.issuer,
            "authorization_endpoint": self.authorization_endpoint(),
            "token_endpoint": self.token_endpoint(),
            "jwks_uri": self.jwks_uri(),
            "id_token_signing_alg_values_supported": ["RS256"],
        })
    }

    /// Signe un `id_token` RS256 avec le `nonce` et l'expiration fournis.
    pub fn sign_id_token(&self, nonce: &str, exp: i64) -> String {
        let header = serde_json::json!({ "alg": "RS256", "typ": "JWT", "kid": self.kid });
        let claims = serde_json::json!({
            "iss": self.issuer,
            "aud": self.client_id,
            "sub": self.subject,
            "exp": exp,
            "iat": exp - 3600,
            "nonce": nonce,
            "email": self.email,
            "email_verified": true,
            "name": self.name,
        });
        let signing_input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap()),
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
        );
        let pkey = PKey::from_rsa((*self.key).clone()).expect("pkey");
        let mut signer = Signer::new(MessageDigest::sha256(), &pkey).expect("signer");
        signer.update(signing_input.as_bytes()).expect("update");
        let signature = signer.sign_to_vec().expect("signature");
        format!("{}.{}", signing_input, URL_SAFE_NO_PAD.encode(signature))
    }

    /// `id_token` avec un e-mail non vérifié (doit être refusé).
    pub fn sign_id_token_email_non_verifie(&self, nonce: &str) -> String {
        let header = serde_json::json!({ "alg": "RS256", "typ": "JWT", "kid": self.kid });
        let claims = serde_json::json!({
            "iss": self.issuer,
            "aud": self.client_id,
            "sub": self.subject,
            "exp": FAR_FUTURE_EXP,
            "nonce": nonce,
            "email": self.email,
            "email_verified": false,
        });
        let signing_input = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap()),
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
        );
        let pkey = PKey::from_rsa((*self.key).clone()).expect("pkey");
        let mut signer = Signer::new(MessageDigest::sha256(), &pkey).expect("signer");
        signer.update(signing_input.as_bytes()).expect("update");
        let signature = signer.sign_to_vec().expect("signature");
        format!("{}.{}", signing_input, URL_SAFE_NO_PAD.encode(signature))
    }
}

impl Default for TestIdp {
    fn default() -> Self {
        Self::new()
    }
}

/// Transport simulé : répond au document de découverte, au JWKS et à l'échange
/// de code. Peut aussi jouer les cas d'erreur (PKCE invalide, découverte
/// incomplète, id_token absent, nonce non armé).
pub struct FakeOidcTransport {
    idp: TestIdp,
    armed_nonce: Mutex<Option<String>>,
    expected_challenge: Mutex<Option<String>>,
    suppress_id_token: AtomicBool,
    break_discovery: AtomicBool,
}

impl FakeOidcTransport {
    pub fn new(idp: TestIdp) -> Self {
        Self {
            idp,
            armed_nonce: Mutex::new(None),
            expected_challenge: Mutex::new(None),
            suppress_id_token: AtomicBool::new(false),
            break_discovery: AtomicBool::new(false),
        }
    }

    pub fn idp(&self) -> &TestIdp {
        &self.idp
    }

    /// Nonce que le faux IdP inscrira dans l'`id_token` (c'est celui que l'ENT a
    /// envoyé dans l'URL d'autorisation).
    pub fn arm_nonce(&self, nonce: &str) {
        *self.armed_nonce.lock().unwrap() = Some(nonce.to_string());
    }

    /// Vérifie le PKCE comme le ferait un vrai IdP : `SHA256(verifier)` doit
    /// égaler le `code_challenge` envoyé à l'autorisation.
    pub fn arm_code_challenge(&self, challenge: &str) {
        *self.expected_challenge.lock().unwrap() = Some(challenge.to_string());
    }

    pub fn suppress_id_token(&self) {
        self.suppress_id_token.store(true, Ordering::SeqCst);
    }

    pub fn break_discovery(&self) {
        self.break_discovery.store(true, Ordering::SeqCst);
    }
}

#[async_trait]
impl OidcTransport for FakeOidcTransport {
    async fn get_json(&self, url: &str) -> Result<serde_json::Value, OidcError> {
        if url == format!("{}/.well-known/openid-configuration", self.idp.issuer) {
            if self.break_discovery.load(Ordering::SeqCst) {
                // Document amputé du token_endpoint : la découverte doit refuser.
                return Ok(serde_json::json!({
                    "issuer": self.idp.issuer,
                    "authorization_endpoint": self.idp.authorization_endpoint(),
                    "jwks_uri": self.idp.jwks_uri(),
                }));
            }
            return Ok(self.idp.discovery_value());
        }
        if url == self.idp.jwks_uri() {
            return Ok(self.idp.jwks_value());
        }
        Err(OidcError::Network(format!("URL inattendue : {url}")))
    }

    async fn post_form_json(
        &self,
        url: &str,
        form: &[(String, String)],
    ) -> Result<serde_json::Value, OidcError> {
        if url != self.idp.token_endpoint() {
            return Err(OidcError::Network(format!("URL inattendue : {url}")));
        }
        let get = |k: &str| {
            form.iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.clone())
        };
        if get("client_id").as_deref() != Some(self.idp.client_id.as_str()) {
            return Err(OidcError::Http { status: 401 });
        }
        if let Some(expected) = self.expected_challenge.lock().unwrap().clone() {
            let verifier = get("code_verifier").ok_or(OidcError::Http { status: 400 })?;
            let derived = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
            if derived != expected {
                return Err(OidcError::Http { status: 400 });
            }
        }
        if self.suppress_id_token.load(Ordering::SeqCst) {
            return Ok(serde_json::json!({ "access_token": "at", "token_type": "Bearer" }));
        }
        let nonce = self
            .armed_nonce
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| "nonce-non-arme".to_string());
        Ok(serde_json::json!({
            "access_token": "at",
            "token_type": "Bearer",
            "id_token": self.idp.sign_id_token(&nonce, FAR_FUTURE_EXP),
        }))
    }
}
