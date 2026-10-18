//! SSO OIDC (OpenID Connect) — délégation d'identité à un IdP.
//!
//! Ce crate porte **toute** la mécanique de fédération, sans base de données :
//! découverte (`.well-known/openid-configuration`), PKCE (S256), génération de
//! `state`/`nonce`, échange `authorization_code` → tokens, puis vérification
//! **RS256** de l'`id_token` contre le JWKS de l'IdP (`iss`, `aud`, `exp`,
//! `nonce`).
//!
//! Deux points structurants :
//!
//! - **Aucun algorithme accepté en dehors de RS256** : on refuse `none`,
//!   `HS256` et tout ce que le client pourrait imposer (confusion d'algorithme).
//!   La clé est choisie par `kid` dans le JWKS, jamais par l'en-tête seul.
//! - **Le transport est une dépendance injectable** ([`OidcTransport`]) : le
//!   code de production parle HTTP (reqwest), les tests pilotent un IdP en
//!   mémoire ([`testing::TestIdp`]) — la logique est donc testable hors réseau.
//!
//! Ce crate ne décide **pas** qui a le droit d'entrer : il rend une
//! [`OidcIdentity`] vérifiée. Le rattachement au compte local (et le refus des
//! comptes inconnus) est une décision de `ent-db`.

mod jwt;
mod pkce;
mod provider;
pub mod testing;
mod transport;

pub use jwt::{verify_id_token, IdTokenClaims, Jwk, Jwks};
pub use pkce::{generate_pkce, random_token};
pub use provider::{
    build_authorization_url, discover, exchange_authorization_code, OidcDiscovery, OidcIdentity,
    OidcProviderConfig,
};
pub use transport::{HttpOidcTransport, OidcTransport};

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OidcError {
    #[error("réseau OIDC indisponible : {0}")]
    Network(String),
    #[error("l'IdP a répondu HTTP {status}")]
    Http { status: u16 },
    #[error("découverte OIDC invalide : {0}")]
    Discovery(String),
    #[error("id_token illisible")]
    MalformedToken,
    #[error("algorithme de signature refusé : {0} (RS256 attendu)")]
    UnsupportedAlg(String),
    #[error("clé de signature introuvable dans le JWKS")]
    UnknownKey,
    #[error("signature de l'id_token invalide")]
    Signature,
    #[error("émetteur (iss) inattendu")]
    BadIssuer,
    #[error("audience (aud) inattendue")]
    BadAudience,
    #[error("id_token expiré")]
    Expired,
    #[error("nonce OIDC inattendu (rejeu ?)")]
    BadNonce,
    #[error("réponse de l'IdP sans id_token")]
    NoIdToken,
    #[error("e-mail non vérifié par l'IdP")]
    EmailNotVerified,
}
