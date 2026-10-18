//! Orchestration SSO : découverte, autorisation (PKCE) et échange de code.

use serde::{Deserialize, Serialize};

use crate::jwt::{verify_id_token, Jwks};
use crate::transport::OidcTransport;
use crate::OidcError;

/// Document de découverte OIDC (`/.well-known/openid-configuration`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcDiscovery {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    #[serde(default)]
    pub id_token_signing_alg_values_supported: Vec<String>,
}

/// Paramétrage d'un fournisseur côté établissement.
#[derive(Debug, Clone)]
pub struct OidcProviderConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Vec<String>,
    pub redirect_uri: String,
}

impl OidcProviderConfig {
    /// Portée demandée : `openid` est **toujours** présent (sinon ce n'est pas
    /// de l'OpenID Connect).
    pub fn scope_string(&self) -> String {
        let mut scopes = self.scopes.clone();
        if !scopes.iter().any(|s| s == "openid") {
            scopes.insert(0, "openid".to_string());
        }
        scopes.join(" ")
    }
}

/// Identité **vérifiée** rendue par l'IdP (l'ENT décide ensuite du rattachement).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcIdentity {
    pub subject: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

/// Interroge l'IdP : endpoints + clés de signature.
pub async fn discover(
    transport: &dyn OidcTransport,
    issuer: &str,
) -> Result<OidcDiscovery, OidcError> {
    let url = format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/')
    );
    let value = transport.get_json(&url).await?;
    let discovery: OidcDiscovery = serde_json::from_value(value)
        .map_err(|e| OidcError::Discovery(format!("document illisible : {e}")))?;
    if discovery.authorization_endpoint.is_empty()
        || discovery.token_endpoint.is_empty()
        || discovery.jwks_uri.is_empty()
    {
        return Err(OidcError::Discovery(
            "authorization/token/jwks manquant".into(),
        ));
    }
    Ok(discovery)
}

/// Construit l'URL d'autorisation (code + PKCE S256 + state + nonce).
pub fn build_authorization_url(
    discovery: &OidcDiscovery,
    config: &OidcProviderConfig,
    state: &str,
    nonce: &str,
    code_challenge: &str,
) -> String {
    let mut q = url::form_urlencoded::Serializer::new(String::new());
    q.append_pair("response_type", "code");
    q.append_pair("client_id", &config.client_id);
    q.append_pair("redirect_uri", &config.redirect_uri);
    q.append_pair("scope", &config.scope_string());
    q.append_pair("state", state);
    q.append_pair("nonce", nonce);
    q.append_pair("code_challenge", code_challenge);
    q.append_pair("code_challenge_method", "S256");
    let query = q.finish().replace('+', "%20");
    let sep = if discovery.authorization_endpoint.contains('?') {
        '&'
    } else {
        '?'
    };
    format!("{}{}{}", discovery.authorization_endpoint, sep, query)
}

#[derive(Debug, Deserialize)]
struct TokenEndpointResponse {
    #[serde(default)]
    id_token: Option<String>,
}

/// Échange le `code` contre des tokens puis **vérifie** l'`id_token`
/// (signature RS256 + iss/aud/exp/nonce). `expected_nonce` vient de l'état
/// serveur du parcours : un id_token sans le bon nonce est un rejeu.
pub async fn exchange_authorization_code(
    transport: &dyn OidcTransport,
    discovery: &OidcDiscovery,
    config: &OidcProviderConfig,
    code: &str,
    code_verifier: &str,
    expected_nonce: &str,
    now: i64,
) -> Result<OidcIdentity, OidcError> {
    let form = vec![
        ("grant_type".to_string(), "authorization_code".to_string()),
        ("code".to_string(), code.to_string()),
        ("redirect_uri".to_string(), config.redirect_uri.clone()),
        ("client_id".to_string(), config.client_id.clone()),
        ("client_secret".to_string(), config.client_secret.clone()),
        ("code_verifier".to_string(), code_verifier.to_string()),
    ];
    let value = transport
        .post_form_json(&discovery.token_endpoint, &form)
        .await?;
    let tokens: TokenEndpointResponse = serde_json::from_value(value)
        .map_err(|e| OidcError::Discovery(format!("réponse token illisible : {e}")))?;
    let id_token = tokens.id_token.ok_or(OidcError::NoIdToken)?;

    let jwks_value = transport.get_json(&discovery.jwks_uri).await?;
    let jwks: Jwks = serde_json::from_value(jwks_value)
        .map_err(|e| OidcError::Discovery(format!("JWKS illisible : {e}")))?;

    let claims = verify_id_token(
        &id_token,
        &jwks,
        &discovery.issuer,
        &config.client_id,
        Some(expected_nonce),
        now,
    )?;

    // Un e-mail explicitement marqué non vérifié ne sert pas de clé de
    // rattachement (mais un IdP qui n'émet pas le claim reste accepté : le
    // rattachement par `sub` est la voie normale).
    if claims.email_verified == Some(false) {
        return Err(OidcError::EmailNotVerified);
    }

    Ok(OidcIdentity {
        subject: claims.sub,
        email: claims.email,
        name: claims.name.or(claims.preferred_username),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeOidcTransport, TestIdp};

    fn config(idp: &TestIdp) -> OidcProviderConfig {
        OidcProviderConfig {
            issuer: idp.issuer.clone(),
            client_id: idp.client_id.clone(),
            client_secret: idp.client_secret.clone(),
            scopes: vec!["openid".into(), "email".into(), "profile".into()],
            redirect_uri: idp.redirect_uri.clone(),
        }
    }

    #[tokio::test]
    async fn decouverte_puis_autorisation_puis_echange() {
        let idp = TestIdp::new();
        let transport = FakeOidcTransport::new(idp.clone());

        let discovery = discover(&transport, &idp.issuer).await.expect("découverte");
        assert_eq!(discovery.token_endpoint, idp.token_endpoint());

        let (verifier, challenge) = crate::generate_pkce();
        let url = build_authorization_url(&discovery, &config(&idp), "state-1", "nonce-1", &challenge);
        assert!(url.starts_with(&idp.authorization_endpoint()));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("scope=openid%20email%20profile"));

        transport.arm_nonce("nonce-1");
        let identity = exchange_authorization_code(
            &transport,
            &discovery,
            &config(&idp),
            "code-1",
            &verifier,
            "nonce-1",
            1_800_000_000,
        )
        .await
        .expect("échange");
        assert_eq!(identity.subject, idp.subject);
        assert_eq!(identity.email.as_deref(), Some(idp.email.as_str()));
    }

    #[tokio::test]
    async fn nonce_du_token_ne_correspond_pas_au_state() {
        let idp = TestIdp::new();
        let transport = FakeOidcTransport::new(idp.clone());
        let discovery = discover(&transport, &idp.issuer).await.expect("découverte");
        transport.arm_nonce("nonce-emis-par-idp");
        let err = exchange_authorization_code(
            &transport,
            &discovery,
            &config(&idp),
            "code-1",
            "verifier",
            "nonce-attendu-par-ent",
            1_800_000_000,
        )
        .await
        .unwrap_err();
        assert_eq!(err, OidcError::BadNonce);
    }

    #[tokio::test]
    async fn token_sans_id_token_refuse() {
        let idp = TestIdp::new();
        let transport = FakeOidcTransport::new(idp.clone());
        let discovery = discover(&transport, &idp.issuer).await.expect("découverte");
        transport.suppress_id_token();
        let err = exchange_authorization_code(
            &transport,
            &discovery,
            &config(&idp),
            "code-1",
            "verifier",
            "nonce",
            1_800_000_000,
        )
        .await
        .unwrap_err();
        assert_eq!(err, OidcError::NoIdToken);
    }

    #[tokio::test]
    async fn decouverte_incomplete_refusee() {
        let idp = TestIdp::new();
        let transport = FakeOidcTransport::new(idp.clone());
        transport.break_discovery();
        assert!(matches!(
            discover(&transport, &idp.issuer).await.unwrap_err(),
            OidcError::Discovery(_)
        ));
    }
}
