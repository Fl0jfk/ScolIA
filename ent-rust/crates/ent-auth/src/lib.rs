//! Auth ENT — cible prod, 100 % Rust.
//!
//! Mot de passe : Argon2id. Session : token opaque (Bearer) ; seul le hash SHA-256
//! est stocké en Postgres (`ent_session`). Pas de Better-Auth, pas de JWT magique.

mod password;
mod roles;
mod token;

pub use password::{hash_password, verify_password, PasswordError};
pub use roles::Role;
pub use token::{generate_session_token, hash_token};

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
    fn token_hash_stable() {
        let (raw, h1) = generate_session_token();
        let h2 = hash_token(&raw);
        assert_eq!(h1, h2);
        assert_ne!(raw, h1);
    }
}
