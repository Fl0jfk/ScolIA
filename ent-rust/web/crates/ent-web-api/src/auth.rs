//! Service d'authentification : routes existantes d'`ent-api`, inchangées.

use uuid::Uuid;

use crate::{
    ApiClient, ApiError, Connexion, LoginResponseDto, Methode, Moi, PasskeyChallengeDto, Session,
};

impl ApiClient {
    /// `POST /auth/login` (mot de passe). Si le compte a une passkey, renvoie le défi
    /// du second facteur à signer puis à transmettre à [`ApiClient::connexion_passkey_fin`].
    pub async fn connexion(
        &self,
        email: &str,
        mot_de_passe: &str,
        etablissement_id: Uuid,
    ) -> Result<Connexion, ApiError> {
        let corps = serde_json::json!({
            "email": email,
            "password": mot_de_passe,
            "etablissement_id": etablissement_id,
        });
        let r: LoginResponseDto = self
            .appel(Methode::Post, "/auth/login", Some(corps))
            .await?;
        match (r.mfa_required, r.session, r.passkey) {
            (false, Some(s), _) => Ok(Connexion::Ouverte(s)),
            (true, _, Some(defi)) => Ok(Connexion::PasskeyRequise(Box::new(defi))),
            _ => Err(ApiError::Decodage("réponse de connexion incomplète".into())),
        }
    }

    /// `POST /auth/passkey/login/start` : connexion sans mot de passe (défi WebAuthn).
    pub async fn connexion_passkey_debut(
        &self,
        email: &str,
        etablissement_id: Uuid,
    ) -> Result<PasskeyChallengeDto, ApiError> {
        let corps = serde_json::json!({ "email": email, "etablissement_id": etablissement_id });
        self.appel(Methode::Post, "/auth/passkey/login/start", Some(corps))
            .await
    }

    /// `POST /auth/passkey/login/finish` : assertion signée par l'appareil → session.
    pub async fn connexion_passkey_fin(
        &self,
        jeton_defi: &str,
        assertion: serde_json::Value,
    ) -> Result<Session, ApiError> {
        let corps = serde_json::json!({ "token": jeton_defi, "credential": assertion });
        self.appel(Methode::Post, "/auth/passkey/login/finish", Some(corps))
            .await
    }

    /// `POST /auth/logout` : révocation de la session en base.
    pub async fn deconnexion(&self) -> Result<(), ApiError> {
        let _: serde_json::Value = self.appel(Methode::Post, "/auth/logout", None).await?;
        Ok(())
    }

    /// `GET /auth/me` : personne connectée.
    pub async fn moi(&self) -> Result<Moi, ApiError> {
        self.appel(Methode::Get, "/auth/me", None).await
    }
}
