//! Auth ENT — cible prod, 100 % Rust.
//!
//! Mot de passe : Argon2id. Session : token opaque (Bearer) ; seul le hash SHA-256
//! est stocké en Postgres (`ent_session`). Pas de Better-Auth, pas de JWT magique.

mod password;
mod passkey;
mod roles;
mod token;

pub use passkey::{
    rp_config_from_env, AuthenticationResult, CreationChallengeResponse, CredentialID, Passkey,
    PasskeyAuthentication, PasskeyEngine, PasskeyError, PasskeyRegistration, PublicKeyCredential,
    RegisterPublicKeyCredential, RequestChallengeResponse, DEFAULT_RP_NAME,
};
pub use password::{hash_password, verify_password, PasswordError};
pub use roles::Role;
pub use token::{
    generate_challenge_token, generate_reset_token, generate_session_token, generate_temp_password,
    hash_token,
};

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthError {
    #[error("e-mail ou mot de passe invalide")]
    InvalidCredentials,
    #[error("e-mail déjà utilisé")]
    EmailTaken,
    #[error("pas membre de cet établissement")]
    NotMember,
    #[error("session invalide ou expirée")]
    InvalidSession,
    #[error("e-mail vide")]
    EmptyEmail,
    #[error("mot de passe trop court (min. 10)")]
    WeakPassword,
    #[error("trop de tentatives de connexion — réessayez plus tard")]
    TooManyAttempts,
    #[error("lien de réinitialisation invalide ou expiré")]
    ResetTokenInvalide,
    #[error(transparent)]
    Password(#[from] PasswordError),
}

pub fn normalize_email(email: &str) -> Result<String, AuthError> {
    let e = email.trim().to_ascii_lowercase();
    if e.is_empty() || !e.contains('@') {
        return Err(AuthError::EmptyEmail);
    }
    Ok(e)
}

pub fn assert_password_policy(password: &str) -> Result<(), AuthError> {
    if password.chars().count() < 10 {
        return Err(AuthError::WeakPassword);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_roundtrip() {
        let hash = hash_password("DevLocalPass1!").expect("hash");
        assert!(verify_password("DevLocalPass1!", &hash).unwrap());
        assert!(!verify_password("wrong-password", &hash).unwrap());
    }

    #[test]
    fn temp_password_respecte_la_politique() {
        let pwd = generate_temp_password();
        assert!(pwd.chars().count() >= 10);
        assert!(assert_password_policy(&pwd).is_ok());
        // Deux tirages diffèrent (aléatoire).
        assert_ne!(pwd, generate_temp_password());
    }

    #[test]
    fn token_hash_stable() {
        let (raw, h1) = generate_session_token();
        let h2 = hash_token(&raw);
        assert_eq!(h1, h2);
        assert_ne!(raw, h1);
    }

    #[test]
    fn reset_token_usage_unique() {
        let (raw1, h1) = generate_reset_token();
        let (raw2, _) = generate_reset_token();
        assert_ne!(raw1, raw2);
        assert_ne!(raw1, h1);
        assert_eq!(hash_token(&raw1), h1);
    }
}
