//! Routes `/cahier/c9/*` du lot C9 du socle 20 : progressions et reconduction (`20_CAHIER_DE_TEXTES.md` §16).
//!
//! * `GET` / `POST /cahier/c9/progressions`, `GET` / `PUT /cahier/c9/progressions/{id}`, `POST …/{id}/archiver` : plan privé.
//! * `POST /cahier/c9/progressions/{id}/partages`, `DELETE …/{id}/partages/{partage}` : partage avec un cours ou une discipline.
//! * `POST /cahier/c9/progressions/{id}/planifier` : entrées en brouillon sur les séances libres d'un cours (`apercu` possible).
//! * `POST /cahier/c9/progressions/{id}/reconduire` : copie privée, sans dates.
//! * `POST /cahier/c9/cours/copier` : copie des entrées de l'appelant d'un cours à l'autre (brouillons).
//!
//! L'établissement vient de la session, jamais de la requête.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use ent_types::cahier_c9::*;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cahier/c9/progressions", get(lister).post(creer))
        .route("/cahier/c9/progressions/{id}", get(lire).put(modifier))
        .route("/cahier/c9/progressions/{id}/archiver", post(archiver))
        .route("/cahier/c9/progressions/{id}/partages", post(partager))
        .route(
            "/cahier/c9/progressions/{id}/partages/{partage}",
            delete(partage_retirer),
        )
        .route("/cahier/c9/progressions/{id}/planifier", post(planifier))
        .route("/cahier/c9/progressions/{id}/reconduire", post(reconduire))
        .route("/cahier/c9/cours/copier", post(copier_cours))
}

async fn lister(State(s): State<AppState>, session: EntSession) -> Res<Json<ProgressionsDto>> {
    Ok(Json(
        s.ent
            .cdt_c9_progressions(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}

async fn creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ProgressionBody>,
) -> Res<(StatusCode, Json<ProgressionDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.ent
                .cdt_c9_progression_creer(&acteur(&session), b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn lire(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ProgressionDto>> {
    Ok(Json(
        s.ent
            .cdt_c9_progression_lire(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ProgressionBody>,
) -> Res<Json<ProgressionDto>> {
    Ok(Json(
        s.ent
            .cdt_c9_progression_modifier(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn archiver(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .cdt_c9_progression_archiver(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn partager(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PartageBody>,
) -> Res<(StatusCode, Json<ProgressionDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.ent
                .cdt_c9_partager(&acteur(&session), id, b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn partage_retirer(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, partage)): Path<(Uuid, Uuid)>,
) -> Res<Json<ProgressionDto>> {
    Ok(Json(
        s.ent
            .cdt_c9_partage_retirer(&acteur(&session), id, partage)
            .await
            .map_err(map_db)?,
    ))
}

async fn planifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PlanifierBody>,
) -> Res<Json<PlanDto>> {
    Ok(Json(
        s.ent
            .cdt_c9_planifier(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn reconduire(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ReconduireBody>,
) -> Res<(StatusCode, Json<ProgressionDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.ent
                .cdt_c9_reconduire(&acteur(&session), id, b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn copier_cours(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CopierCoursBody>,
) -> Res<Json<CopieCoursDto>> {
    Ok(Json(
        s.ent
            .cdt_c9_copier_cours(&acteur(&session), b)
            .await
            .map_err(map_db)?,
    ))
}
