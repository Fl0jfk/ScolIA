//! Routes des délégations par objet, de l'intérim de direction et des pouvoirs de signature (socle 6, lot P6).
//! Préfixes `/personnels/delegations` et `/personnels/signature`.
//!
//! L'autorité est `ent-db` (`personnels_delegations.rs`) : droits (`delegations.gerer`, `signature.habiliter`, fonction de
//! chef d'établissement), anti-escalade, cloisonnement par établissement (404 uniforme), journal, événements. Les routes
//! ne font que décoder et mapper les erreurs. Types : `ent_types::personnels_delegations`. La révocation (PRS-83) reste
//! `POST /delegations/{id}/revoquer` (S2-5), qui ferme aussi ce que la délégation a produit. Le port `peut_signer` du
//! socle 7 n'a PAS de route : le socle 7 l'appelle en Rust (`PgEnt::peut_signer`).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_types::personnels_delegations::*;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/personnels/delegations", get(lister))
        .route("/personnels/delegations/mission", post(deleguer_mission))
        .route(
            "/personnels/delegations/signature",
            post(deleguer_signature),
        )
        .route("/personnels/delegations/interim", post(organiser_interim))
        .route(
            "/personnels/signature/habilitations",
            get(habilitations).post(habiliter),
        )
        .route(
            "/personnels/signature/habilitations/{id}/retirer",
            post(retirer),
        )
        .route("/personnels/moi/signature", get(mes_pouvoirs))
}

async fn lister(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<DelegationObjetDto>>> {
    state
        .ent
        .delegations_objets_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn deleguer_mission(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<DeleguerMissionBody>,
) -> Res<(StatusCode, Json<DelegationObjetDto>)> {
    state
        .ent
        .deleguer_mission_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn deleguer_signature(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<DeleguerSignatureBody>,
) -> Res<(StatusCode, Json<DelegationObjetDto>)> {
    state
        .ent
        .deleguer_signature_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn organiser_interim(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<OrganiserInterimBody>,
) -> Res<(StatusCode, Json<DelegationObjetDto>)> {
    state
        .ent
        .organiser_interim_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn habilitations(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<HabilitationDto>>> {
    state
        .ent
        .habilitations_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn habiliter(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<HabiliterBody>,
) -> Res<(StatusCode, Json<HabilitationDto>)> {
    state
        .ent
        .habiliter_signature_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|h| (StatusCode::CREATED, Json(h)))
        .map_err(map_db)
}

async fn retirer(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<HabilitationDto>> {
    state
        .ent
        .retirer_habilitation_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn mes_pouvoirs(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<HabilitationDto>>> {
    state
        .ent
        .mes_pouvoirs_signature(session.etablissement_id, session.user_id)
        .await
        .map(Json)
        .map_err(map_db)
}
