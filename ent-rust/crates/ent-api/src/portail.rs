//! Socle 29, lot P1 : `GET /portail/aujourdhui`, la page d'accueil composée côté serveur.
//!
//! L'autorité est `ent-db` (`PgEnt::portail_aujourdhui`) : chaque tuile est rendue par le service
//! propriétaire sous les droits de la session ; la route ne fait que traduire HTTP ↔ service.
//! Le jour est celui de l'établissement (`maintenant_local`), jamais la date UTC.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use ent_db::PgEnt;
use ent_types::portail::PortailAujourdhui;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/portail/aujourdhui", get(aujourdhui))
}

async fn aujourdhui(
    State(state): State<AppState>,
    session: EntSession,
) -> Result<Json<PortailAujourdhui>, (StatusCode, String)> {
    state
        .ent
        .portail_aujourdhui(
            session.etablissement_id,
            session.user_id,
            &session.role,
            PgEnt::maintenant_local(),
        )
        .await
        .map(Json)
        .map_err(map_db)
}
