//! Chiffrement des secrets applicatifs au repos (AES-256-GCM via OpenSSL, déjà
//! dépendance du crate — aucune nouvelle dépendance).
//!
//! Format stocké : `enc:v1:` + base64url(nonce 12 o ‖ chiffré ‖ tag 16 o).
//! Une valeur sans ce préfixe est un **secret hérité en clair** : lue telle
//! quelle (rétrocompatibilité) puis rechiffrée par l'appelant.
//!
//! Clé : `ENT_SECRETS_KEY` = 32 octets en hex (64 car.) ou base64/base64url.
//! Fail-closed hors labo/test : absente ou invalide → erreur au démarrage. En
//! labo/test seulement, une clé de développement **publique** est utilisée
//! (elle ne protège rien, elle évite de casser les environnements locaux).
//!
//! Le contenu n'apparaît jamais dans `Debug` ni dans les messages d'erreur.

use base64::Engine as _;
use openssl::symm::{decrypt_aead, encrypt_aead, Cipher};
use rand::RngCore;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const SEALED_PREFIX: &str = "enc:v1:";
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecretBoxError {
    #[error("ENT_SECRETS_KEY absente (32 octets en hex ou base64 requis hors labo/test)")]
    KeyMissing,
    #[error("ENT_SECRETS_KEY invalide (32 octets en hex ou base64 attendus)")]
    KeyInvalid,
    #[error("secret illisible (clé inadaptée ou donnée altérée)")]
    Decrypt,
}

#[derive(Clone)]
pub struct SecretBox {
    key: [u8; 32],
}

impl std::fmt::Debug for SecretBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretBox").field("key", &"[masquée]").finish()
    }
}

/// `true` si la valeur est au format chiffré (sinon : secret hérité en clair).
pub fn is_sealed(stored: &str) -> bool {
    stored.starts_with(SEALED_PREFIX)
}

impl SecretBox {
    /// Construit depuis le texte de clé (hex 64 car., ou base64 std/url sans ou avec padding).
    pub fn from_key_str(raw: &str) -> Result<Self, SecretBoxError> {
        let raw = raw.trim();
        let bytes: Vec<u8> = if raw.len() == 64 && raw.bytes().all(|b| b.is_ascii_hexdigit()) {
            hex::decode(raw).map_err(|_| SecretBoxError::KeyInvalid)?
        } else {
            let t = raw.trim_end_matches('=');
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(t)
                .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(t))
                .map_err(|_| SecretBoxError::KeyInvalid)?
        };
        let key: [u8; 32] = bytes.try_into().map_err(|_| SecretBoxError::KeyInvalid)?;
        Ok(Self { key })
    }

    /// Clé de **développement publique** (labo/test uniquement).
    pub fn dev() -> Self {
        let mut h = Sha256::new();
        h.update(b"scolia-ent-dev-secrets-key-PUBLIC-NOT-SECRET");
        Self {
            key: h.finalize().into(),
        }
    }

    /// Depuis un texte de clé optionnel et le mode d'environnement (pure, testable).
    pub fn resolve(key: Option<&str>, non_prod: bool) -> Result<Self, SecretBoxError> {
        match key.map(str::trim).filter(|k| !k.is_empty()) {
            Some(k) => Self::from_key_str(k),
            None if non_prod => Ok(Self::dev()),
            None => Err(SecretBoxError::KeyMissing),
        }
    }

    /// Depuis `ENT_SECRETS_KEY` et `ENT_ENV`.
    pub fn from_env() -> Result<Self, SecretBoxError> {
        Self::resolve(
            std::env::var("ENT_SECRETS_KEY").ok().as_deref(),
            crate::env_non_prod(),
        )
    }

    /// Chiffre. `aad` lie le chiffré à son usage (un secret recopié ailleurs ne s'ouvre pas).
    pub fn seal(&self, plaintext: &str, aad: &[u8]) -> Result<String, SecretBoxError> {
        let mut nonce = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce);
        let mut tag = [0u8; TAG_LEN];
        let ct = encrypt_aead(
            Cipher::aes_256_gcm(),
            &self.key,
            Some(&nonce),
            aad,
            plaintext.as_bytes(),
            &mut tag,
        )
        .map_err(|_| SecretBoxError::Decrypt)?;
        let mut blob = Vec::with_capacity(NONCE_LEN + ct.len() + TAG_LEN);
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ct);
        blob.extend_from_slice(&tag);
        Ok(format!(
            "{SEALED_PREFIX}{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(blob)
        ))
    }

    /// Déchiffre. Une valeur sans préfixe `enc:v1:` est rendue telle quelle
    /// (héritage en clair).
    pub fn open(&self, stored: &str, aad: &[u8]) -> Result<String, SecretBoxError> {
        let Some(b64) = stored.strip_prefix(SEALED_PREFIX) else {
            return Ok(stored.to_string());
        };
        let blob = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(b64)
            .map_err(|_| SecretBoxError::Decrypt)?;
        if blob.len() < NONCE_LEN + TAG_LEN {
            return Err(SecretBoxError::Decrypt);
        }
        let (nonce, rest) = blob.split_at(NONCE_LEN);
        let (ct, tag) = rest.split_at(rest.len() - TAG_LEN);
        let pt = decrypt_aead(Cipher::aes_256_gcm(), &self.key, Some(nonce), aad, ct, tag)
            .map_err(|_| SecretBoxError::Decrypt)?;
        String::from_utf8(pt).map_err(|_| SecretBoxError::Decrypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const K1: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

    #[test]
    fn aller_retour_et_non_deterministe() {
        let b = SecretBox::from_key_str(K1).unwrap();
        let s1 = b.seal("s3cret", b"ctx").unwrap();
        let s2 = b.seal("s3cret", b"ctx").unwrap();
        assert!(is_sealed(&s1));
        assert!(!s1.contains("s3cret"));
        assert_ne!(s1, s2, "nonce aléatoire");
        assert_eq!(b.open(&s1, b"ctx").unwrap(), "s3cret");
    }

    #[test]
    fn clair_herite_lu_tel_quel() {
        let b = SecretBox::dev();
        assert_eq!(b.open("ancien-secret-en-clair", b"ctx").unwrap(), "ancien-secret-en-clair");
        assert!(!is_sealed("ancien-secret-en-clair"));
    }

    #[test]
    fn altere_mauvaise_cle_ou_mauvais_contexte_refuses() {
        let b = SecretBox::from_key_str(K1).unwrap();
        let s = b.seal("s3cret", b"ctx").unwrap();
        assert_eq!(b.open(&s, b"autre").unwrap_err(), SecretBoxError::Decrypt);
        let autre = SecretBox::dev();
        assert_eq!(autre.open(&s, b"ctx").unwrap_err(), SecretBoxError::Decrypt);
        // Un bit retourné dans le chiffré.
        let mut raw = s.clone().into_bytes();
        let n = raw.len();
        raw[n - 3] = if raw[n - 3] == b'A' { b'B' } else { b'A' };
        let alt = String::from_utf8(raw).unwrap();
        assert_eq!(b.open(&alt, b"ctx").unwrap_err(), SecretBoxError::Decrypt);
        assert_eq!(b.open("enc:v1:@@@", b"ctx").unwrap_err(), SecretBoxError::Decrypt);
        assert_eq!(b.open("enc:v1:AAAA", b"ctx").unwrap_err(), SecretBoxError::Decrypt);
    }

    #[test]
    fn cle_fail_closed_hors_labo() {
        assert_eq!(SecretBox::resolve(None, false).unwrap_err(), SecretBoxError::KeyMissing);
        assert_eq!(SecretBox::resolve(Some("  "), false).unwrap_err(), SecretBoxError::KeyMissing);
        assert_eq!(SecretBox::resolve(Some("trop-court"), false).unwrap_err(), SecretBoxError::KeyInvalid);
        assert_eq!(SecretBox::resolve(Some("trop-court"), true).unwrap_err(), SecretBoxError::KeyInvalid);
        assert!(SecretBox::resolve(None, true).is_ok(), "clé de dev en labo/test");
        assert!(SecretBox::resolve(Some(K1), false).is_ok());
        let b64 = base64::engine::general_purpose::STANDARD.encode(hex::decode(K1).unwrap());
        let a = SecretBox::resolve(Some(&b64), false).unwrap();
        let b = SecretBox::from_key_str(K1).unwrap();
        assert_eq!(a.open(&b.seal("x", b"c").unwrap(), b"c").unwrap(), "x");
    }

    #[test]
    fn debug_ne_montre_pas_la_cle() {
        let d = format!("{:?}", SecretBox::from_key_str(K1).unwrap());
        assert!(!d.contains("0102") && d.contains("masquée"));
    }
}
