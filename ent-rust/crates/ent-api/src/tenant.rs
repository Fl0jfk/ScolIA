//! Établissement déduit de l'adresse (socle 1, C2, D-012).
//!
//! L'hôte de la requête (`Host`, ou `X-Forwarded-Host` **seulement** derrière un
//! proxy de confiance, `ENT_TRUST_PROXY=1`) est résolu en établissement via
//! `ent_etablissement_domaine`. Mode choisi par `ENT_TENANT_PAR_HOTE` :
//!
//! - `strict` : hôte inconnu → 404 (sauf `/health`, `/ready`, `/metrics`) ;
//!   l'`etablissement_id` envoyé par le client est ignoré ; une session d'un
//!   autre établissement que l'hôte → 401 ;
//! - `souple` (défaut) : hôte connu → comme `strict` ; hôte inconnu ou absent →
//!   comportement antérieur (`etablissement_id` fourni par le client).

use crate::AppState;
use axum::extract::{FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

/// Chemins jamais soumis à la résolution (sondes et métriques).
const EXEMPTES: &[&str] = &["/health", "/ready", "/metrics"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeTenant {
    Strict,
    Souple,
}

/// Lu à chaque requête (comme `ENT_TRUST_PROXY`) : valeur inconnue → souple.
pub fn mode_tenant() -> ModeTenant {
    match std::env::var("ENT_TENANT_PAR_HOTE")
        .map(|v| v.trim().to_ascii_lowercase())
        .as_deref()
    {
        Ok("strict") => ModeTenant::Strict,
        _ => ModeTenant::Souple,
    }
}

/// `ENT_TRUST_PROXY=1` : la requête arrive d'un proxy de confiance qui pose
/// lui-même les en-têtes `X-Forwarded-*` (même règle que la garde par IP).
pub(crate) fn proxy_de_confiance() -> bool {
    std::env::var("ENT_TRUST_PROXY")
        .map(|v| matches!(v.trim(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Nom d'hôte brut de la requête. `X-Forwarded-Host` (première valeur) n'est lu
/// que derrière un proxy de confiance : sinon un client pourrait choisir son
/// établissement. Repli : `Host`, puis l'autorité de l'URI (HTTP/2).
fn hote_brut(headers: &HeaderMap, uri: &Uri) -> Option<String> {
    let lire = |nom: &str| {
        headers
            .get(nom)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    if proxy_de_confiance() {
        if let Some(h) = lire("x-forwarded-host") {
            return Some(h);
        }
    }
    lire("host").or_else(|| uri.authority().map(|a| a.as_str().to_string()))
}

/// Établissement imposé par l'adresse de la requête (`None` : hôte inconnu,
/// possible seulement en mode souple).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TenantHote {
    pub etablissement_id: Option<Uuid>,
}

impl TenantHote {
    async fn resoudre(
        state: &AppState,
        headers: &HeaderMap,
        uri: &Uri,
    ) -> Result<Self, (StatusCode, String)> {
        let etablissement_id = match hote_brut(headers, uri) {
            Some(h) => state.ent.resoudre_domaine(&h).await.map_err(|e| {
                crate::routes::erreur_infra(&e).unwrap_or((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Erreur interne.".to_string(),
                ))
            })?,
            None => None,
        };
        Ok(Self { etablissement_id })
    }

    /// Établissement à utiliser pour une route publique / de connexion :
    /// celui de l'hôte s'il est connu (la valeur du client est alors ignorée),
    /// sinon (mode souple) celui fourni par le client.
    pub fn etablissement_effectif(
        &self,
        client: Option<Uuid>,
    ) -> Result<Uuid, (StatusCode, String)> {
        if let Some(e) = self.etablissement_id {
            if client.is_some_and(|c| c != e) {
                tracing::debug!("etablissement_id du client ignoré : imposé par l'hôte");
            }
            return Ok(e);
        }
        if mode_tenant() == ModeTenant::Strict {
            return Err(hote_inconnu());
        }
        client.ok_or((StatusCode::BAD_REQUEST, "établissement requis".to_string()))
    }

    /// Une session (ou une connexion qui vient d'aboutir) appartient-elle à
    /// l'établissement de l'hôte ? Vrai si l'hôte n'impose rien (mode souple).
    pub fn accepte(&self, etablissement_id: Uuid) -> bool {
        self.etablissement_id.is_none_or(|e| e == etablissement_id)
    }
}

fn hote_inconnu() -> (StatusCode, String) {
    (
        StatusCode::NOT_FOUND,
        "établissement inconnu pour cette adresse".to_string(),
    )
}

/// Résout l'établissement une fois par requête et le range dans les extensions ;
/// en mode strict, un hôte inconnu s'arrête ici (404).
pub async fn resoudre_tenant(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    if EXEMPTES.contains(&req.uri().path()) {
        return next.run(req).await;
    }
    let tenant = match TenantHote::resoudre(&state, req.headers(), req.uri()).await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };
    if tenant.etablissement_id.is_none() && mode_tenant() == ModeTenant::Strict {
        return hote_inconnu().into_response();
    }
    req.extensions_mut().insert(tenant);
    next.run(req).await
}

impl FromRequestParts<AppState> for TenantHote {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(t) = parts.extensions.get::<TenantHote>() {
            return Ok(*t);
        }
        // Hors middleware (ne devrait pas arriver avec `app()`) : on résout ici.
        let t = Self::resoudre(state, &parts.headers, &parts.uri).await?;
        if t.etablissement_id.is_none() && mode_tenant() == ModeTenant::Strict {
            return Err(hote_inconnu());
        }
        parts.extensions.insert(t);
        Ok(t)
    }
}
