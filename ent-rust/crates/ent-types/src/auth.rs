//! Session (contrat de `POST /auth/login`, `GET /auth/me`). Copie fidèle de
//! `ent-api/src/routes.rs`.
//!
//! Note : `LoginBody` contient un mot de passe — ne jamais l'écrire dans un journal
//! (`Debug` est volontairement implémenté à la main pour le masquer).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Corps de `POST /auth/login`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginBody {
    pub email: String,
    pub password: String,
    pub etablissement_id: Uuid,
}

impl std::fmt::Debug for LoginBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginBody")
            .field("email", &self.email)
            .field("password", &"<masqué>")
            .field("etablissement_id", &self.etablissement_id)
            .finish()
    }
}

/// Session ouverte (jeton opaque à envoyer en `Authorization: Bearer`).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthSessionDto {
    pub token: String,
    pub user_id: Uuid,
    pub email: String,
    pub etablissement_id: Uuid,
    pub role: String,
    pub expires_at: DateTime<Utc>,
}

impl std::fmt::Debug for AuthSessionDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthSessionDto")
            .field("token", &"<masqué>")
            .field("user_id", &self.user_id)
            .field("email", &self.email)
            .field("etablissement_id", &self.etablissement_id)
            .field("role", &self.role)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// Défi passkey renvoyé quand le compte exige le second facteur.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PasskeyChallengeDto {
    pub token: String,
    pub options: serde_json::Value,
    pub expires_at: DateTime<Utc>,
}

/// Réponse de `POST /auth/login` : session à plat (`flatten`) ou défi passkey.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoginResponseDto {
    pub mfa_required: bool,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub session: Option<AuthSessionDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passkey: Option<PasskeyChallengeDto>,
}

/// Réponse de `GET /auth/me`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeDto {
    pub user_id: Uuid,
    pub email: String,
    pub etablissement_id: Uuid,
    pub role: String,
    /// Présent si le compte est une face élève liée.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eleve_id: Option<Uuid>,
    /// Nombre de passkeys enrôlées.
    pub passkey_count: i64,
}
