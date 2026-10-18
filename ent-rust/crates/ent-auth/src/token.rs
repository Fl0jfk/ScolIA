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

/// Mot de passe temporaire aléatoire (≥ 16 caractères, classes mélangées).
/// À communiquer à l'utilisateur, qui devra le changer via `POST /auth/password`.
pub fn generate_temp_password() -> String {
    const CHARS: &[u8] = b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789!@#%*?";
    let mut bytes = [0u8; 20];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| CHARS[(*b as usize) % CHARS.len()] as char)
        .collect()
}

pub fn hash_token(raw_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Jeton de réinitialisation de mot de passe (usage unique, TTL côté base).
/// Même format que les sessions : 32 octets aléatoires en hex + hash SHA-256.
pub fn generate_reset_token() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let raw = hex::encode(bytes);
    let hashed = hash_token(&raw);
    (raw, hashed)
}

/// Jeton de challenge (enrôlement / authentification passkey). Usage unique :
/// la ligne de challenge est consommée à la fin du parcours.
pub fn generate_challenge_token() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let raw = hex::encode(bytes);
    let hashed = hash_token(&raw);
    (raw, hashed)
}
