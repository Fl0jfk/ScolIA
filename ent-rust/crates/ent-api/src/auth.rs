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
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub etablissement_id: Uuid,
    pub actor_user_id: String,
    pub email: String,
    pub role: String,
    /// Web ou mobile (D-011).
    pub client: ent_db::ClientSession,
    /// Authentification qui a ouvert la session (mot de passe, passkey, SSO).
    pub assurance: ent_db::AssuranceSession,
    /// Complète, ou limitée à l'enrôlement d'une passkey (D-009).
    pub portee: ent_db::PorteeSession,
}

/// Routes ouvertes à une session « enrôlement seulement » (passkey obligatoire,
/// pas encore enrôlée) : de quoi savoir qui l'on est, enrôler, se déconnecter.
pub const ROUTES_ENROLEMENT: &[&str] = &[
    "/auth/me",
    "/auth/logout",
    "/auth/passkeys",
    "/auth/passkey/register/start",
    "/auth/passkey/register/finish",
];

/// Message stable (le client l'utilise pour afficher l'écran d'enrôlement).
pub const ENROLEMENT_REQUIS: &str = "passkey_enrolement_requis";

impl EntSession {
    pub fn role_parsed(&self) -> Option<Role> {
        Role::parse(&self.role)
    }

    /// Personnel uniquement (ni parent ni élève).
    pub fn require_staff(&self) -> Result<(), (StatusCode, String)> {
        if self.role_parsed().map(|r| r.is_staff()).unwrap_or(false) {
            Ok(())
        } else {
            Err((StatusCode::FORBIDDEN, "réservé au personnel".into()))
        }
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

    /// Sauvegardes / PRA : inventaire d'intégrité et vérification de
    /// restauration — Direction / Admin uniquement, sur son établissement.
    pub fn require_sauvegarde(&self) -> Result<(), (StatusCode, String)> {
        if PgEnt::role_can_sauvegarde(&self.role) {
            Ok(())
        } else {
            Err((
                StatusCode::FORBIDDEN,
                "rôle insuffisant pour les sauvegardes / PRA".into(),
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
            .map_err(|e| match e {
                ent_db::EntDbError::Auth(_) => (StatusCode::UNAUTHORIZED, e.to_string()),
                autre => crate::routes::erreur_infra(&autre).unwrap_or((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Erreur interne.".to_string(),
                )),
            })?;

        // Socle 1 (A2) : une session d'enrôlement n'ouvre aucune donnée.
        if info.portee == ent_db::PorteeSession::Enrolement
            && !ROUTES_ENROLEMENT.contains(&parts.uri.path())
        {
            return Err((StatusCode::FORBIDDEN, ENROLEMENT_REQUIS.into()));
        }

        Ok(Self {
            session_id: info.session_id,
            user_id: info.user_id,
            etablissement_id: info.etablissement_id,
            actor_user_id: info.user_id.to_string(),
            email: info.email,
            role: info.role,
            client: info.client,
            assurance: info.assurance,
            portee: info.portee,
        })
    }
}
