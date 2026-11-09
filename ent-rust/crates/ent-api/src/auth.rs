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

    /// D-017 (R2) : pré-filtre d'une route — l'acteur doit avoir `action` avec au
    /// moins une portée (rôle de base ou rôles paramétrés). Le contrôle par élève
    /// reste fait par le service (`autoriser_eleve`).
    pub async fn require_action(
        &self,
        ent: &PgEnt,
        action: ent_auth::Action,
        refus: &str,
    ) -> Result<(), (StatusCode, String)> {
        match ent
            .a_action_quelconque(self.etablissement_id, self.user_id, &self.role, action)
            .await
        {
            Ok(true) => Ok(()),
            Ok(false) => Err((StatusCode::FORBIDDEN, refus.to_string())),
            Err(e) => Err(crate::routes::map_db(e)),
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

/// R2 (D-017) : charge **une fois par requête** les droits effectifs de la
/// personne authentifiée (profil de base ∪ rôles attribués, lus en base : effet
/// immédiat) et exécute le handler dans ce contexte. Les contrôles de gestion
/// des services (`role_can_*`, `require_*`) y lisent leur décision : une seule
/// règle, celle du moteur. Sans session valide ou en cas d'erreur de lecture, la
/// requête passe **sans** contexte : repli sur le modèle du profil (droits
/// historiques), jamais plus. La session résolue est mémorisée pour l'extracteur.
pub async fn contexte_droits(
    axum::extract::State(state): axum::extract::State<AppState>,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let Some(header) = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
    else {
        return next.run(req).await;
    };
    let Ok(info) = state.ent.resolve_session(&header).await else {
        return next.run(req).await;
    };
    let ctx = state
        .ent
        .charger_droits_acteur(info.etablissement_id, info.user_id, &info.role)
        .await;
    req.extensions_mut().insert(SessionPreResolue(info));
    match ctx {
        Ok(Some(ctx)) => PgEnt::avec_droits_acteur(ctx, next.run(req)).await,
        _ => next.run(req).await,
    }
}

/// Session déjà résolue par [`contexte_droits`] (évite une seconde lecture).
#[derive(Clone)]
pub struct SessionPreResolue(pub ent_db::SessionInfo);

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

        let info = if let Some(SessionPreResolue(info)) = parts.extensions.get().cloned() {
            info
        } else {
            state
                .ent
                .resolve_session(header)
                .await
                .map_err(|e| match e {
                    ent_db::EntDbError::Auth(_) => (StatusCode::UNAUTHORIZED, e.to_string()),
                    autre => crate::routes::erreur_infra(&autre).unwrap_or((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Erreur interne.".to_string(),
                    )),
                })?
        };

        // C2 (D-012) : sur l'adresse d'un établissement, seule une session de cet
        // établissement est acceptée (jeton d'un autre établissement → 401).
        let tenant = crate::tenant::TenantHote::from_request_parts(parts, state).await?;
        if !tenant.accepte(info.etablissement_id) {
            return Err((
                StatusCode::UNAUTHORIZED,
                "session d'un autre établissement".into(),
            ));
        }

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
