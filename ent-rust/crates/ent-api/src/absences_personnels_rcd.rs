//! Routes du remplacement de courte durée (socle 15, lot A6). Préfixe `/personnels/rcd`.
//! L'autorité est `ent-db` (`absences_personnels/rcd.rs`) : plan et vivant selon `personnels.rcd.gerer`, remontée
//! selon `personnels.rcd.remontee`, engagement de soi-même sans droit particulier. Aucune route ne renvoie la nature
//! d'une absence ni l'identité d'une personne dans une remontée (R-02).

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_types::absences_personnels::*;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/personnels/rcd/plans", get(plans).post(creer_plan))
        .route("/personnels/rcd/plan-courant", get(plan_courant))
        .route("/personnels/rcd/plans/{id}", axum::routing::put(modifier_plan))
        .route("/personnels/rcd/plans/{id}/version", post(versionner_plan))
        .route("/personnels/rcd/plans/{id}/procedure", post(procedure_plan))
        .route("/personnels/rcd/plans/{id}/campagne", post(lancer_campagne))
        .route("/personnels/rcd/plans/{id}/comptes-rendus", post(compte_rendu))
        .route("/personnels/rcd/engagements", post(engager))
        .route("/personnels/rcd/engagements/{id}", axum::routing::put(modifier_engagement))
        .route("/personnels/rcd/mon-compteur", get(mon_compteur))
        .route("/personnels/rcd/vivier", get(vivier))
        .route("/personnels/rcd/remontees", get(remontees).post(preparer))
        .route("/personnels/rcd/remontees/{id}/controle", post(controler))
        .route("/personnels/rcd/remontees/{id}/rejet", post(rejeter))
        .route("/personnels/rcd/remontees/{id}/transmission", post(transmettre))
        .route("/personnels/rcd/remontees/{id}/export.csv", get(exporter))
}

async fn plans(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<PlanRcdDto>>> {
    s.ent
        .plans_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, None)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn plan_courant(State(s): State<AppState>, session: EntSession) -> Res<Json<Option<PlanRcdDto>>> {
    s.ent
        .plan_courant_rcd_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creer_plan(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<PlanRcdBody>,
) -> Res<(StatusCode, Json<PlanRcdDto>)> {
    s.ent
        .creer_plan_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn modifier_plan(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PlanRcdBody>,
) -> Res<Json<PlanRcdDto>> {
    s.ent
        .modifier_plan_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn versionner_plan(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PlanRcdBody>,
) -> Res<(StatusCode, Json<PlanRcdDto>)> {
    s.ent
        .versionner_plan_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn procedure_plan(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ProcedurePlanRcdBody>,
) -> Res<Json<PlanRcdDto>> {
    s.ent
        .procedure_plan_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn lancer_campagne(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<PlanRcdDto>> {
    s.ent
        .lancer_campagne_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn compte_rendu(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CompteRenduRcdBody>,
) -> Res<(StatusCode, Json<CompteRenduRcdDto>)> {
    s.ent
        .produire_compte_rendu_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn engager(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<EngagementRcdBody>,
) -> Res<(StatusCode, Json<EngagementRcdDto>)> {
    s.ent
        .engager_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn modifier_engagement(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ModifierEngagementRcdBody>,
) -> Res<Json<EngagementRcdDto>> {
    s.ent
        .modifier_engagement_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn mon_compteur(State(s): State<AppState>, session: EntSession) -> Res<Json<Option<CompteurRcdDto>>> {
    s.ent
        .mon_compteur_rcd_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn vivier(State(s): State<AppState>, session: EntSession) -> Res<Json<VivierRcdDto>> {
    s.ent
        .vivier_rcd_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn remontees(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<RemonteeRcdDto>>> {
    s.ent
        .remontees_rcd_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn preparer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<PreparerRemonteeBody>,
) -> Res<(StatusCode, Json<RemonteeRcdDto>)> {
    s.ent
        .preparer_remontee_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn controler(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<RemonteeRcdDto>> {
    s.ent
        .controler_remontee_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn rejeter(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<RemonteeRcdDto>> {
    s.ent
        .rejeter_remontee_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn transmettre(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<RemonteeRcdDto>> {
    s.ent
        .transmettre_remontee_rcd_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn exporter(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<impl IntoResponse> {
    let csv = s
        .ent
        .exporter_remontee_rcd_csv_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"remontee-rcd.csv\""),
            (header::CACHE_CONTROL, "no-store"),
        ],
        csv,
    ))
}
