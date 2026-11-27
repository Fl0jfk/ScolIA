//! Vérification d'`id_token` JWT (RS256) contre un JWKS.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use openssl::bn::BigNum;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::Verifier;
use serde::Deserialize;

use crate::OidcError;

/// Une clé publique du JWKS (on ne gère que RSA : `kty = "RSA"`).
#[derive(Debug, Clone, Deserialize)]
pub struct Jwk {
    pub kty: String,
    #[serde(default)]
    pub kid: Option<String>,
    #[serde(default)]
    pub alg: Option<String>,
    #[serde(default)]
    pub n: Option<String>,
    #[serde(default)]
    pub e: Option<String>,
    #[serde(default, rename = "use")]
    pub usage: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

/// Claims d'un `id_token` — on ne lit que ce dont l'ENT a besoin.
#[derive(Debug, Clone, Deserialize)]
pub struct IdTokenClaims {
    pub iss: String,
    /// `aud` peut être une chaîne ou un tableau (spec OIDC).
    pub aud: serde_json::Value,
    pub exp: i64,
    #[serde(default)]
    pub nonce: Option<String>,
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: Option<bool>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub preferred_username: Option<String>,
}

impl IdTokenClaims {
    pub fn audiences(&self) -> Vec<String> {
        match &self.aud {
            serde_json::Value::String(s) => vec![s.clone()],
            serde_json::Value::Array(a) => a
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        }
    }
}

fn b64url(input: &str) -> Result<Vec<u8>, OidcError> {
    URL_SAFE_NO_PAD
        .decode(input.trim_end_matches('='))
        .map_err(|_| OidcError::MalformedToken)
}

/// Choisit la clé de vérification : par `kid` si l'en-tête en fournit un,
/// sinon l'unique clé RSA du JWKS (utile aux IdP qui n'émettent qu'une clé).
/// Un `kid` inconnu est un **refus** (pas de repli silencieux).
fn select_jwk<'a>(jwks: &'a Jwks, kid: Option<&str>) -> Result<&'a Jwk, OidcError> {
    let rsa_keys: Vec<&Jwk> = jwks
        .keys
        .iter()
        .filter(|k| k.kty.eq_ignore_ascii_case("RSA"))
        .collect();
    match kid {
        Some(kid) => rsa_keys
            .iter()
            .find(|k| k.kid.as_deref() == Some(kid))
            .copied()
            .ok_or(OidcError::UnknownKey),
        None => match rsa_keys.as_slice() {
            [only] => Ok(only),
            [] => Err(OidcError::UnknownKey),
            _ => Err(OidcError::UnknownKey),
        },
    }
}

/// Vérifie une signature RSA-SHA256 avec la clé publique (n, e) d'un JWK.
pub fn verify_rs256(jwk: &Jwk, signing_input: &[u8], signature: &[u8]) -> Result<(), OidcError> {
    let n = jwk.n.as_deref().ok_or(OidcError::UnknownKey)?;
    let e = jwk.e.as_deref().ok_or(OidcError::UnknownKey)?;
    let n = b64url(n)?;
    let e = b64url(e)?;
    let bn_n = BigNum::from_slice(&n).map_err(|_| OidcError::UnknownKey)?;
    let bn_e = BigNum::from_slice(&e).map_err(|_| OidcError::UnknownKey)?;
    let rsa = Rsa::from_public_components(bn_n, bn_e).map_err(|_| OidcError::UnknownKey)?;
    let pkey = PKey::from_rsa(rsa).map_err(|_| OidcError::UnknownKey)?;
    let mut verifier =
        Verifier::new(MessageDigest::sha256(), &pkey).map_err(|_| OidcError::UnknownKey)?;
    verifier
        .update(signing_input)
        .map_err(|_| OidcError::Signature)?;
    if verifier.verify(signature).map_err(|_| OidcError::Signature)? {
        Ok(())
    } else {
        Err(OidcError::Signature)
    }
}

/// Vérifie un `id_token` : structure, algorithme (RS256 **seulement**),
/// signature JWKS, puis `iss` / `aud` / `exp` / `nonce`.
pub fn verify_id_token(
    id_token: &str,
    jwks: &Jwks,
    issuer: &str,
    client_id: &str,
    expected_nonce: Option<&str>,
    now: i64,
) -> Result<IdTokenClaims, OidcError> {
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() != 3 {
        return Err(OidcError::MalformedToken);
    }
    let header: serde_json::Value =
        serde_json::from_slice(&b64url(parts[0])?).map_err(|_| OidcError::MalformedToken)?;
    let alg = header
        .get("alg")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if alg != "RS256" {
        return Err(OidcError::UnsupportedAlg(alg.to_string()));
    }
    let kid = header.get("kid").and_then(|v| v.as_str());
    let signature = b64url(parts[2])?;
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let jwk = select_jwk(jwks, kid)?;
    verify_rs256(jwk, signing_input.as_bytes(), &signature)?;

    let claims: IdTokenClaims =
        serde_json::from_slice(&b64url(parts[1])?).map_err(|_| OidcError::MalformedToken)?;
    if claims.iss != issuer {
        return Err(OidcError::BadIssuer);
    }
    if !claims.audiences().iter().any(|a| a == client_id) {
        return Err(OidcError::BadAudience);
    }
    if claims.exp <= now {
        return Err(OidcError::Expired);
    }
    if let Some(expected) = expected_nonce {
        if claims.nonce.as_deref() != Some(expected) {
            return Err(OidcError::BadNonce);
        }
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestIdp;

    const NOW: i64 = 1_800_000_000;

    #[test]
    fn id_token_valide_accepte() {
        let idp = TestIdp::new();
        let jwt = idp.sign_id_token("nonce-1", NOW + 300);
        let claims = verify_id_token(
            &jwt,
            &idp.jwks(),
            &idp.issuer,
            &idp.client_id,
            Some("nonce-1"),
            NOW,
        )
        .expect("id_token valide");
        assert_eq!(claims.sub, idp.subject);
        assert_eq!(claims.email.as_deref(), Some(idp.email.as_str()));
    }

    #[test]
    fn signature_alteree_refusee() {
        let idp = TestIdp::new();
        let jwt = idp.sign_id_token("n", NOW + 300);
        let mut parts: Vec<String> = jwt.split('.').map(str::to_string).collect();
        // On garde l'en-tête et les claims mais on casse la signature.
        parts[2] = crate::random_token();
        let casse = parts.join(".");
        assert_eq!(
            verify_id_token(
                &casse,
                &idp.jwks(),
                &idp.issuer,
                &idp.client_id,
                None,
                NOW
            )
            .unwrap_err(),
            OidcError::Signature
        );
    }

    #[test]
    fn nonce_et_audience_et_issuer_verifies() {
        let idp = TestIdp::new();
        let jwt = idp.sign_id_token("attendu", NOW + 300);
        assert_eq!(
            verify_id_token(&jwt, &idp.jwks(), &idp.issuer, &idp.client_id, Some("autre"), NOW)
                .unwrap_err(),
            OidcError::BadNonce
        );
        assert_eq!(
            verify_id_token(&jwt, &idp.jwks(), &idp.issuer, "mauvais-client", None, NOW)
                .unwrap_err(),
            OidcError::BadAudience
        );
        assert_eq!(
            verify_id_token(&jwt, &idp.jwks(), "https://autre-idp", &idp.client_id, None, NOW)
                .unwrap_err(),
            OidcError::BadIssuer
        );
    }

    #[test]
    fn expiration_refusee() {
        let idp = TestIdp::new();
        let jwt = idp.sign_id_token("n", NOW - 1);
        assert_eq!(
            verify_id_token(&jwt, &idp.jwks(), &idp.issuer, &idp.client_id, None, NOW).unwrap_err(),
            OidcError::Expired
        );
    }

    #[test]
    fn algorithme_non_rs256_refuse() {
        let idp = TestIdp::new();
        // En-tête forgé en HS256 : doit être refusé **avant** toute signature.
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(br#"{"iss":"x","aud":"y","exp":9999999999,"sub":"s"}"#);
        let forged = format!("{header}.{payload}.AAAA");
        assert_eq!(
            verify_id_token(&forged, &idp.jwks(), "x", "y", None, NOW).unwrap_err(),
            OidcError::UnsupportedAlg("HS256".into())
        );
    }

    #[test]
    fn kid_inconnu_refuse() {
        let idp = TestIdp::new();
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","kid":"inconnu"}"#);
        let payload = URL_SAFE_NO_PAD.encode(br#"{"iss":"x","aud":"y","exp":9999999999,"sub":"s"}"#);
        let forged = format!("{header}.{payload}.AAAA");
        assert_eq!(
            verify_id_token(&forged, &idp.jwks(), "x", "y", None, NOW).unwrap_err(),
            OidcError::UnknownKey
        );
    }
}
