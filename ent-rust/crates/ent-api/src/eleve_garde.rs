//! E6 (revue de sécurité) : garde central des routes `/eleves/{uuid}[/…]`.
//!
//! Un élève sous confidentialité renforcée n'existe pas pour un membre du personnel qui n'a ni
//! G6 ni lien direct : toute route par identifiant d'élève répond `404` (jamais `403`), même si
//! le service appelé oubliait son propre contrôle. Défense en profondeur : les services gardent
//! leurs filtres. Sans session valide, la requête continue (le handler répond 401).

use axum::extract::{FromRequestParts, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::AppState;

/// Identifiant d'élève d'un chemin `/eleves/<uuid>` ou `/eleves/<uuid>/…`.
pub fn eleve_du_chemin(chemin: &str) -> Option<Uuid> {
    let reste = chemin.strip_prefix("/eleves/")?;
    let segment = reste.split('/').next()?;
    Uuid::parse_str(segment).ok()
}

/// Couche d'`app()`.
pub async fn exiger_eleve_visible(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let Some(eleve) = eleve_du_chemin(req.uri().path()) else {
        return next.run(req).await;
    };
    let (mut parts, body) = req.into_parts();
    let session = match EntSession::from_request_parts(&mut parts, &state).await {
        Ok(s) => s,
        Err(_) => return next.run(Request::from_parts(parts, body)).await,
    };
    match state
        .ent
        .eleve_cache_au_tiers(session.etablissement_id, session.user_id, &session.role, eleve)
        .await
    {
        Ok(false) => next.run(Request::from_parts(parts, body)).await,
        Ok(true) => (StatusCode::NOT_FOUND, "Introuvable.").into_response(),
        // Fermé en cas d'erreur : jamais de fuite par une panne de lecture.
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "Indisponible.").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chemin_eleve() {
        let id = Uuid::from_u128(7);
        assert_eq!(eleve_du_chemin(&format!("/eleves/{id}")), Some(id));
        assert_eq!(eleve_du_chemin(&format!("/eleves/{id}/identite/x")), Some(id));
        assert_eq!(eleve_du_chemin("/eleves/pastilles"), None);
        assert_eq!(eleve_du_chemin("/eleves"), None);
        assert_eq!(eleve_du_chemin("/sante/eleves/x"), None);
    }

    /// F1 : le type servi pour une photo est celui de la signature binaire, jamais un type
    /// actif ; un contenu inconnu est un flux binaire (`nosniff` est posé par la route).
    #[test]
    fn mime_de_photo_par_signature() {
        use crate::routes::mime_image_par_signature as mime;
        assert_eq!(mime(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 16]), "image/jpeg");
        assert_eq!(mime(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0]), "image/png");
        assert_eq!(mime(b"RIFF\x00\x00\x00\x00WEBPVP8 "), "image/webp");
        assert_eq!(mime(b"<html><script>alert(1)</script>"), "application/octet-stream");
        assert_eq!(mime(b"<svg xmlns='http://www.w3.org/2000/svg'/>"), "application/octet-stream");
        assert_eq!(mime(&[]), "application/octet-stream");
    }
}
