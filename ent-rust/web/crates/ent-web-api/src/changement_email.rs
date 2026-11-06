//! Changement d'adresse e-mail d'un compte (revue de sécurité RF-01) : routes **publiques**
//! `POST /auth/email/confirmer` (lien reçu à la NOUVELLE adresse) et `POST /auth/email/annuler`
//! (lien reçu à l'ANCIENNE adresse). Le jeton fait foi ; réponse 204 sans corps.

use crate::{ApiClient, ApiError, Methode};

impl ApiClient {
    /// `POST /auth/email/confirmer` : la nouvelle adresse devient celle du compte.
    pub async fn confirmer_changement_email(&self, jeton: &str) -> Result<(), ApiError> {
        let corps = serde_json::json!({ "token": jeton });
        let _: serde_json::Value = self
            .appel(Methode::Post, "/auth/email/confirmer", Some(corps))
            .await?;
        Ok(())
    }

    /// `POST /auth/email/annuler` : annule la demande, ou rétablit l'ancienne adresse.
    pub async fn annuler_changement_email(&self, jeton: &str) -> Result<(), ApiError> {
        let corps = serde_json::json!({ "token": jeton });
        let _: serde_json::Value = self
            .appel(Methode::Post, "/auth/email/annuler", Some(corps))
            .await?;
        Ok(())
    }
}
