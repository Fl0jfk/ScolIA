//! Passkeys (WebAuthn) — 2ᵉ facteur **et** connexion sans mot de passe.
//!
//! Choix structurant : l'ENT ne s'appuie **que** sur WebAuthn (passkey /
//! Touch ID / Face ID / clé de sécurité). Pas de TOTP, pas d'application
//! d'authentification tierce : le secret ne quitte jamais l'appareil de
//! l'utilisateur et la vérification est asymétrique (aucun secret partagé
//! serveur).
//!
//! Ce module isole la mécanique cryptographique (challenge, attestation,
//! assertion, compteur d'usage) de la persistance : il ne touche pas à
//! Postgres. La façade `ent-db` stocke les états de challenge et les
//! credentials.

use thiserror::Error;
use uuid::Uuid;
use webauthn_rs::prelude::WebauthnBuilder;

pub use webauthn_rs::prelude::{
    AuthenticationResult, CreationChallengeResponse, CredentialID, Passkey, PasskeyAuthentication,
    PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Url,
};

/// Nom de la « partie » affiché par le navigateur lors de l'enrôlement (ex.
/// « ScolIA ENT »). Configurable par déploiement.
pub const DEFAULT_RP_NAME: &str = "ScolIA ENT";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PasskeyError {
    #[error("configuration WebAuthn invalide (rp_id / rp_origin)")]
    Config,
    #[error("démarrage de l'enrôlement de passkey impossible")]
    RegisterStart,
    #[error("enrôlement de passkey refusé par le vérificateur")]
    RegisterFinish,
    #[error("démarrage de l'authentification par passkey impossible")]
    AuthStart,
    #[error("authentification par passkey refusée")]
    AuthFinish,
    #[error("vérification utilisateur absente (biométrie / code refusé)")]
    UserVerificationRequired,
    #[error("état de passkey illisible")]
    BadState,
}

/// Origine (RP ID + origine complète) lue depuis l'environnement, avec des
/// valeurs de repli sûres pour le développement local.
///
/// - `ENT_RP_ID`   : domaine sans schéma (défaut `localhost`).
/// - `ENT_RP_ORIGIN` : origine complète (défaut `http://localhost:3000`).
/// - `ENT_RP_NAME` : libellé affiché (défaut [`DEFAULT_RP_NAME`]).
pub fn rp_config_from_env() -> (String, String, String) {
    let rp_id = std::env::var("ENT_RP_ID")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "localhost".to_string());
    let rp_origin = std::env::var("ENT_RP_ORIGIN")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "http://localhost:3000".to_string());
    let rp_name = std::env::var("ENT_RP_NAME")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_RP_NAME.to_string());
    (rp_id, rp_origin, rp_name)
}

/// Moteur WebAuthn — construit une fois par processus à partir de la config
/// d'établissement/déploiement.
pub struct PasskeyEngine {
    webauthn: webauthn_rs::Webauthn,
}

impl PasskeyEngine {
    pub fn new(rp_id: &str, rp_origin: &str, rp_name: &str) -> Result<Self, PasskeyError> {
        let origin = Url::parse(rp_origin).map_err(|_| PasskeyError::Config)?;
        let webauthn = WebauthnBuilder::new(rp_id, &origin)
            .map_err(|_| PasskeyError::Config)?
            .rp_name(rp_name)
            .build()
            .map_err(|_| PasskeyError::Config)?;
        Ok(Self { webauthn })
    }

    /// Construit le moteur depuis l'environnement ([`rp_config_from_env`]).
    pub fn from_env() -> Result<Self, PasskeyError> {
        let (rp_id, rp_origin, rp_name) = rp_config_from_env();
        Self::new(&rp_id, &rp_origin, &rp_name)
    }

    /// Ouvre une session d'enrôlement. `exclude` liste les credentials déjà
    /// connus pour éviter qu'un même appareil s'enrôle deux fois.
    pub fn start_registration(
        &self,
        user_id: Uuid,
        user_name: &str,
        user_display_name: &str,
        exclude: Vec<CredentialID>,
    ) -> Result<(CreationChallengeResponse, PasskeyRegistration), PasskeyError> {
        self.webauthn
            .start_passkey_registration(user_id, user_name, user_display_name, Some(exclude))
            .map_err(|_| PasskeyError::RegisterStart)
    }

    /// Vérifie l'attestation et produit la `Passkey` à persister.
    pub fn finish_registration(
        &self,
        credential: &RegisterPublicKeyCredential,
        state: &PasskeyRegistration,
    ) -> Result<Passkey, PasskeyError> {
        self.webauthn
            .finish_passkey_registration(credential, state)
            .map_err(|_| PasskeyError::RegisterFinish)
    }

    /// Ouvre une session d'authentification à partir des passkeys d'un compte.
    pub fn start_authentication(
        &self,
        passkeys: &[Passkey],
    ) -> Result<(RequestChallengeResponse, PasskeyAuthentication), PasskeyError> {
        self.webauthn
            .start_passkey_authentication(passkeys)
            .map_err(|_| PasskeyError::AuthStart)
    }

    /// Vérifie l'assertion. La vérification utilisateur (biométrie / code) est
    /// **exigée** : elle est ce qui distingue un 2ᵉ facteur d'un simple
    /// porteur d'appareil.
    pub fn finish_authentication(
        &self,
        credential: &PublicKeyCredential,
        state: &PasskeyAuthentication,
    ) -> Result<AuthenticationResult, PasskeyError> {
        let res = self
            .webauthn
            .finish_passkey_authentication(credential, state)
            .map_err(|_| PasskeyError::AuthFinish)?;
        if !res.user_verified() {
            return Err(PasskeyError::UserVerificationRequired);
        }
        Ok(res)
    }

    /// Rejoue le compteur d'usage d'une passkey après une authentification
    /// réussie (détection de clonage : le compteur doit progresser).
    pub fn apply_counter(passkey: &mut Passkey, res: &AuthenticationResult) -> bool {
        passkey.update_credential(res).unwrap_or(false)
    }
}

impl std::fmt::Debug for PasskeyEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasskeyEngine").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dev_par_defaut_acceptee() {
        // http://localhost est une origine « secure context » : indispensable
        // pour tester les passkeys en local sans TLS.
        let engine = PasskeyEngine::new("localhost", "http://localhost:3000", DEFAULT_RP_NAME);
        assert!(engine.is_ok(), "origine localhost doit être acceptée");
    }

    #[test]
    fn origine_invalide_refusee() {
        assert_eq!(
            PasskeyEngine::new("localhost", "pas-une-url", DEFAULT_RP_NAME).unwrap_err(),
            PasskeyError::Config
        );
    }

    #[test]
    fn rp_config_env_repli() {
        // Les variables d'environnement ne sont pas posées dans ce test :
        // on doit retomber sur les valeurs de dev.
        std::env::remove_var("ENT_RP_ID");
        std::env::remove_var("ENT_RP_ORIGIN");
        std::env::remove_var("ENT_RP_NAME");
        let (id, origin, name) = rp_config_from_env();
        assert_eq!(id, "localhost");
        assert_eq!(origin, "http://localhost:3000");
        assert_eq!(name, DEFAULT_RP_NAME);
    }

    #[test]
    fn demarrage_enrolement_produit_un_challenge() {
        let engine = PasskeyEngine::new("localhost", "http://localhost:3000", DEFAULT_RP_NAME)
            .expect("engine");
        let uid = Uuid::new_v4();
        let (ccr, _state) = engine
            .start_registration(uid, "claire@ent.local", "Claire", vec![])
            .expect("start registration");
        // Le challenge porte un identifiant de credential utilisateur et un
        // challenge non vide, prêts à être sérialisés vers le navigateur.
        assert_eq!(ccr.public_key.user.id.as_slice(), uid.as_bytes());
        assert!(!ccr.public_key.challenge.is_empty());
    }

    #[test]
    fn enrolement_avec_charge_invalide_refuse() {
        // Un enrôlement fabriqué ne peut pas valider un état de challenge
        // réel : la vérification échoue proprement (pas de panic).
        let engine = PasskeyEngine::new("localhost", "http://localhost:3000", DEFAULT_RP_NAME)
            .expect("engine");
        let uid = Uuid::new_v4();
        let (ccr, state) = engine
            .start_registration(uid, "claire@ent.local", "Claire", vec![])
            .expect("start registration");
        let fake: RegisterPublicKeyCredential = serde_json::from_value(serde_json::json!({
            "id": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "rawId": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "type": "public-key",
            "response": {
                "attestationObject": "AAAAAA",
                "clientDataJSON": "AAAAAA"
            }
        }))
        .expect("fake credential json");
        assert_eq!(
            engine.finish_registration(&fake, &state).unwrap_err(),
            PasskeyError::RegisterFinish
        );
        // Le state reste inutilisable pour un second essai (non consommé) :
        // la persistance à usage unique est de la responsabilité de l'appelant.
        assert!(!ccr.public_key.challenge.is_empty());
    }
}
