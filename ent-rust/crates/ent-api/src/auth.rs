//! Session ENT — tenant **toujours** issu de `ent_session` (Postgres).
//! Auth 100 % Rust (argon2 + token opaque). Pas de Better-Auth / NextAuth.

use crate::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use ent_auth::Role;
use ent_db::PgEnt;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct EntSession {
    pub user_id: Uuid,
    pub etablissement_id: Uuid,
    pub actor_user_id: String,
    pub email: String,
    pub role: String,
}

impl EntSession {
    pub fn role_parsed(&self) -> Option<Role> {
        Role::parse(&self.role)
    }

    pub fn require_manage_eleves(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_manage_eleves(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour gérer les élèves".into(),
            ))
        }
    }

    pub fn require_read_effectif(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_read_effectif(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour l'effectif".into(),
            ))
        }
    }

    pub fn require_manage_matieres(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_manage_matieres(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour gérer les matières".into(),
            ))
        }
    }

    pub fn require_saisir_notes(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_saisir_notes(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour saisir une note".into(),
            ))
        }
    }

    pub fn require_faire_appel(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_faire_appel(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour faire l'appel".into(),
            ))
        }
    }

    pub fn require_ecrire_carnet(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_ecrire_carnet(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour écrire au carnet".into(),
            ))
        }
    }

    pub fn require_saisir_infirmerie(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_saisir_infirmerie(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour l'infirmerie".into(),
            ))
        }
    }

    pub fn require_saisir_psy(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_saisir_psy(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour la psychologie scolaire".into(),
            ))
        }
    }

    pub fn require_manage_edt(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_manage_edt(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour gérer l'EDT".into(),
            ))
        }
    }

    pub fn require_saisir_sanctions(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_saisir_sanctions(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour la commission / sanctions VS".into(),
            ))
        }
    }

    pub fn require_declarer_incident(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_declarer_incident(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour déclarer un incident".into(),
            ))
        }
    }

    pub fn require_manage_vs_referentiels(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_manage_vs_referentiels(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour gérer les référentiels VS".into(),
            ))
        }
    }
}

impl FromRequestParts<AppState> for EntSession {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "authorization manquante".into()))?;

        let info = state
            .ent
            .resolve_session(header)
            .await
            .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

        Ok(Self {
            user_id: info.user_id,
            etablissement_id: info.etablissement_id,
            actor_user_id: info.user_id.to_string(),
            email: info.email,
            role: info.role,
        })
    }
}
