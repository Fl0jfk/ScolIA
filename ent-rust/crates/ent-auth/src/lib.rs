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
    #[error("e-mail ou mot de passe trop long")]
    TooLong,
    #[error(transparent)]
    Password(#[from] PasswordError),
}

/// Longueur maximale d'une adresse e-mail (RFC 5321), en octets.
pub const MAX_EMAIL_LEN: usize = 254;
/// Longueur maximale d'un mot de passe saisi, en caractères. Refusé **avant**
/// tout hachage Argon2 (anti-amplification CPU / mémoire).
pub const MAX_PASSWORD_CHARS: usize = 256;

/// Refuse un mot de passe trop long sans le hacher.
pub fn assert_password_max_len(password: &str) -> Result<(), AuthError> {
    // Court-circuit sur les octets (un caractère UTF-8 pèse au plus 4 octets).
    if password.len() > MAX_PASSWORD_CHARS * 4
        || password.chars().take(MAX_PASSWORD_CHARS + 1).count() > MAX_PASSWORD_CHARS
    {
        return Err(AuthError::TooLong);
    }
    Ok(())
}

pub fn normalize_email(email: &str) -> Result<String, AuthError> {
    if email.trim().len() > MAX_EMAIL_LEN {
        return Err(AuthError::TooLong);
    }
    let e = email.trim().to_ascii_lowercase();
    if e.is_empty() || !e.contains('@') {
        return Err(AuthError::EmptyEmail);
    }
    Ok(e)
}

pub fn assert_password_policy(password: &str) -> Result<(), AuthError> {
    assert_password_max_len(password)?;
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
    fn bornes_de_taille_email_et_mot_de_passe() {
        let long_mdp = "a".repeat(MAX_PASSWORD_CHARS + 1);
        assert_eq!(assert_password_max_len(&long_mdp), Err(AuthError::TooLong));
        assert_eq!(assert_password_policy(&long_mdp), Err(AuthError::TooLong));
        assert!(assert_password_max_len(&"a".repeat(MAX_PASSWORD_CHARS)).is_ok());
        // 256 caractères multi-octets : accepté ; 257 : refusé.
        assert!(assert_password_max_len(&"é".repeat(MAX_PASSWORD_CHARS)).is_ok());
        assert_eq!(
            assert_password_max_len(&"é".repeat(MAX_PASSWORD_CHARS + 1)),
            Err(AuthError::TooLong)
        );
        let local = "a".repeat(MAX_EMAIL_LEN - "@x.fr".len());
        assert!(normalize_email(&format!("{local}@x.fr")).is_ok());
        assert_eq!(
            normalize_email(&format!("{local}a@x.fr")),
            Err(AuthError::TooLong)
        );
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
