use rand::RngCore;
use sha2::{Digest, Sha256};

/// Génère un token opaque (hex) + son hash à stocker.
pub fn generate_session_token() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let raw = hex::encode(bytes);
    let hashed = hash_token(&raw);
    (raw, hashed)
}

pub fn hash_token(raw_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    hex::encode(hasher.finalize())
}
