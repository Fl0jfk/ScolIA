//! PKCE (RFC 7636, méthode `S256`) et jetons aléatoires (`state`, `nonce`).

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Jeton aléatoire base64url (32 octets) — utilisé pour `state` et `nonce`.
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Paire PKCE `(code_verifier, code_challenge)` avec `code_challenge =
/// BASE64URL(SHA256(code_verifier))`.
pub fn generate_pkce() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let verifier = URL_SAFE_NO_PAD.encode(bytes);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_derive_du_verifier() {
        let (verifier, challenge) = generate_pkce();
        // Longueur RFC 7636 : verifier 43-128 caractères, challenge 43.
        assert!(verifier.len() >= 43 && verifier.len() <= 128);
        assert_eq!(challenge.len(), 43);
        let attendu = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert_eq!(challenge, attendu);
    }

    #[test]
    fn jetons_aleatoires_distincts() {
        let a = random_token();
        let b = random_token();
        assert_ne!(a, b);
        assert_eq!(a.len(), 43);
    }
}
