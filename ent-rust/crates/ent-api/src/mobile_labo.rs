//! Routes de **labo** du lot MB1 (feature `labo` seulement, jamais dans le binaire de production) : elles servent de
//! support aux tests HTTP de l'enveloppe d'erreur codée et de l'`Idempotency-Key` (`docs/socles/11_MB1_SERVEUR_LIVRAISON.md`).
//! Elles ne touchent aucune donnée : un compteur d'exécutions en mémoire, indexé par un nom choisi par l'appelant.
//!
//! Faible F5 (contre-revue MB1) : même dans un binaire de labo, elles ne répondent que si `ENT_ENV` désigne un
//! environnement non productif ([`crate::routes::env_non_prod`]) ; sinon `404`, comme si elles n'existaient pas. Le
//! Dockerfile de production refuse aussi tout binaire qui contient le motif `/mobile/labo`.

use crate::auth::EntSession;
use crate::AppState;
use axum::extract::{Path, Request};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn compteurs() -> &'static Mutex<HashMap<String, u32>> {
    static C: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Deserialize)]
struct EchoBody {
    nom: String,
    /// Durée simulée du traitement (concurrence).
    #[serde(default)]
    delai_ms: u64,
}

#[derive(Serialize)]
struct EchoDto {
    nom: String,
    execution: u32,
}

/// Écriture de labo : incrémente le compteur du nom et renvoie `201` avec le numéro d'exécution.
async fn echo(_s: EntSession, Json(b): Json<EchoBody>) -> (StatusCode, Json<EchoDto>) {
    let n = {
        let mut c = compteurs().lock().unwrap_or_else(|e| e.into_inner());
        let n = c.entry(b.nom.clone()).or_insert(0);
        *n += 1;
        *n
    };
    if b.delai_ms > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(b.delai_ms.min(5_000))).await;
    }
    (
        StatusCode::CREATED,
        Json(EchoDto {
            nom: b.nom,
            execution: n,
        }),
    )
}

/// Nombre d'exécutions d'un nom (lecture).
async fn executions(_s: EntSession, Path(nom): Path<String>) -> Json<EchoDto> {
    let n = compteurs()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&nom)
        .copied()
        .unwrap_or(0);
    Json(EchoDto { nom, execution: n })
}

/// Écriture de labo qui échoue avec le statut demandé et un texte **qui trahit** la cause (il ne doit jamais sortir
/// dans l'enveloppe pour 403 et 404).
async fn echec(_s: EntSession, Path(statut): Path<u16>) -> (StatusCode, String) {
    let code = StatusCode::from_u16(statut).unwrap_or(StatusCode::BAD_REQUEST);
    let texte = match statut {
        403 => "vous êtes visé par une interdiction de contact avec cet élève".to_string(),
        404 => "l'élève existe mais n'est pas dans votre périmètre".to_string(),
        // F1 : un texte de base de données relayé tel quel par une route (il ne doit jamais sortir dans l'enveloppe).
        400 | 409 => "error returned from database: duplicate key value violates unique constraint \"ent_eleve_pkey\""
            .to_string(),
        503 => "pool timed out while waiting for an open connection (sqlx)".to_string(),
        _ => format!("échec simulé {statut}"),
    };
    (code, texte)
}

/// Même échec en lecture (sans écriture : pas d'`Idempotency-Key`).
async fn echec_lecture(s: EntSession, p: Path<u16>) -> (StatusCode, String) {
    echec(s, p).await
}

/// Garde F5 : hors environnement non productif déclaré, les routes de labo répondent `404`.
async fn garde_env(req: Request, next: axum::middleware::Next) -> axum::response::Response {
    if !crate::routes::env_non_prod() {
        return axum::response::IntoResponse::into_response(StatusCode::NOT_FOUND);
    }
    next.run(req).await
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/mobile/labo/echo", post(echo))
        .route("/mobile/labo/echo/{nom}", get(executions))
        .route(
            "/mobile/labo/echec/{statut}",
            post(echec).get(echec_lecture),
        )
        .route_layer(axum::middleware::from_fn(garde_env))
}
