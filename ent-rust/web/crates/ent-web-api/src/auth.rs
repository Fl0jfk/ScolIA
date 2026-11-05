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
        let r: LoginResponseDto = if self.relais {
            let v: serde_json::Value = self
                .appel_adresse(Methode::Post, "/web/session/connexion", Some(corps))
                .await?;
            decoder_connexion_relais(v)?
        } else {
            self.appel(Methode::Post, "/auth/login", Some(corps))
                .await?
        };
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
        if self.relais {
            self.appel_adresse(Methode::Post, "/web/session/passkey/debut", Some(corps))
                .await
        } else {
            self.appel(Methode::Post, "/auth/passkey/login/start", Some(corps))
                .await
        }
    }

    /// `POST /auth/passkey/login/finish` : assertion signée par l'appareil → session.
    pub async fn connexion_passkey_fin(
        &self,
        jeton_defi: &str,
        assertion: serde_json::Value,
    ) -> Result<Session, ApiError> {
        let corps = serde_json::json!({ "token": jeton_defi, "credential": assertion });
        if self.relais {
            let v: serde_json::Value = self
                .appel_adresse(Methode::Post, "/web/session/passkey/fin", Some(corps))
                .await?;
            decoder_session_relais(v)
        } else {
            self.appel(Methode::Post, "/auth/passkey/login/finish", Some(corps))
                .await
        }
    }

    /// `POST /auth/logout` : révocation de la session en base.
    /// Session web : `POST /web/session/deconnexion` (révocation + effacement du cookie).
    pub async fn deconnexion(&self) -> Result<(), ApiError> {
        let _: serde_json::Value = if self.relais {
            self.appel_adresse(Methode::Post, "/web/session/deconnexion", None)
                .await?
        } else {
            self.appel(Methode::Post, "/auth/logout", None).await?
        };
        Ok(())
    }

    /// `GET /auth/me` : personne connectée.
    pub async fn moi(&self) -> Result<Moi, ApiError> {
        self.appel(Methode::Get, "/auth/me", None).await
    }
}

/// Le relais retire `token` du corps (il est dans le cookie `HttpOnly`) : on le complète
/// par une chaîne vide, jamais par une vraie valeur — le code de la page n'a pas de jeton.
fn decoder_session_relais(mut v: serde_json::Value) -> Result<Session, ApiError> {
    if let Some(o) = v.as_object_mut() {
        o.entry("token").or_insert_with(|| "".into());
    }
    serde_json::from_value(v).map_err(|e| ApiError::Decodage(e.to_string()))
}

fn decoder_connexion_relais(mut v: serde_json::Value) -> Result<LoginResponseDto, ApiError> {
    if let Some(o) = v.as_object_mut() {
        if o.get("mfa_required").and_then(|m| m.as_bool()) != Some(true) {
            o.entry("token").or_insert_with(|| "".into());
        }
    }
    serde_json::from_value(v).map_err(|e| ApiError::Decodage(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION: &str = r#"{"mfa_required":false,"user_id":"6f1c3c80-0000-4000-8000-000000000001",
        "email":"a@b","etablissement_id":"6f1c3c80-0000-4000-8000-000000000002","role":"prof",
        "expires_at":"2026-11-03T00:00:00Z"}"#;

    #[test]
    fn connexion_relais_sans_jeton_dans_le_corps() {
        let v: serde_json::Value = serde_json::from_str(SESSION).unwrap();
        let r = decoder_connexion_relais(v.clone()).unwrap();
        let s = r.session.expect("session");
        assert!(s.token.is_empty(), "aucun jeton côté page");
        assert_eq!(s.email, "a@b");
        assert!(decoder_session_relais(v).unwrap().token.is_empty());
    }

    #[test]
    fn connexion_relais_second_facteur() {
        let v = serde_json::json!({"mfa_required":true,
            "passkey":{"token":"defi","options":{},"expires_at":"2026-11-03T00:00:00Z"}});
        let r = decoder_connexion_relais(v).unwrap();
        assert!(r.mfa_required && r.session.is_none() && r.passkey.is_some());
    }
}
