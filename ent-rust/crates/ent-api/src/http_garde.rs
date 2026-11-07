//! Garde-fous HTTP transverses de l'API : en-têtes de non-mise en cache,
//! erreurs internes génériques et span de journal par requête.
//!
//! - `Cache-Control: no-store` + `Pragma: no-cache` sur toute réponse **sauf**
//!   `/static` (polices, scripts : cache légitime) ;
//! - `Referrer-Policy: same-origin` et `Permissions-Policy` minimal ;
//! - toute réponse 500 voit son corps remplacé par un message générique avec
//!   la référence `x-request-id` ; le corps d'origine (qui peut contenir un
//!   message d'erreur technique) n'est écrit que dans le journal ;
//! - le span de requête ne porte que la méthode, le **gabarit** de route
//!   (`MatchedPath`, jamais l'URI brute ni la query : `?token=` des liens de
//!   réinitialisation, `?q=<nom>` des recherches) et l'identifiant de requête.

use axum::body::Body;
use axum::extract::{MatchedPath, Request};
use axum::http::{header, HeaderName, HeaderValue, Response, StatusCode};
use axum::middleware::Next;
use tracing::Span;

const PERMISSIONS_POLICY: &str = "camera=(), microphone=(), geolocation=(), payment=()";

/// Message générique renvoyé au client sur une erreur interne.
pub fn message_interne(request_id: &str) -> String {
    format!("Erreur interne. Référence : {request_id}")
}

/// Format sûr d'un identifiant de requête : `[A-Za-z0-9-]{1,64}`.
pub fn request_id_sur(valeur: &str) -> bool {
    !valeur.is_empty()
        && valeur.len() <= 64
        && valeur.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Retire un `x-request-id` entrant au format douteux (reflété dans le corps
/// des 500, les logs et la réponse) : la couche suivante en génère un UUID.
pub async fn assainir_request_id(mut req: Request, next: Next) -> Response<Body> {
    let sain = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(request_id_sur)
        .unwrap_or(false);
    if !sain {
        req.headers_mut().remove("x-request-id");
    }
    next.run(req).await
}

pub async fn en_tetes_et_erreurs(req: Request, next: Next) -> Response<Body> {
    let statique = req.uri().path().starts_with("/static/");
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| request_id_sur(v))
        .unwrap_or("inconnue")
        .to_string();
    let mut res = next.run(req).await;

    if res.status() == StatusCode::INTERNAL_SERVER_ERROR {
        let (mut parts, body) = res.into_parts();
        let detail = axum::body::to_bytes(body, 4096).await.unwrap_or_default();
        let detail = String::from_utf8_lossy(&detail);
        tracing::error!(
            request_id = %request_id,
            detail = %detail.chars().take(300).collect::<String>(),
            "réponse 500 : corps d'origine remplacé par un message générique"
        );
        parts.headers.remove(header::CONTENT_LENGTH);
        parts.headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        res = Response::from_parts(parts, Body::from(message_interne(&request_id)));
    }

    let h = res.headers_mut();
    // Un handler qui a déjà posé un `Cache-Control` contenant `no-store` (ex. fichiers
    // servis par lien signé : `private, no-store`) le conserve.
    let deja_no_store = h
        .get(header::CACHE_CONTROL)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("no-store"));
    if !statique && !deja_no_store {
        h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        h.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    }
    h.entry(header::REFERRER_POLICY)
        .or_insert(HeaderValue::from_static("same-origin"));
    h.entry(HeaderName::from_static("permissions-policy"))
        .or_insert(HeaderValue::from_static(PERMISSIONS_POLICY));
    res
}

/// Span INFO par requête : méthode, gabarit de route, identifiant de requête.
pub fn span_requete(req: &Request) -> Span {
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or("(non routé)");
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| request_id_sur(v))
        .unwrap_or("-");
    tracing::info_span!(
        "requete",
        methode = %req.method(),
        route = %route,
        request_id = %request_id,
    )
}
