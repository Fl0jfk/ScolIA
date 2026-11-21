//! Routes des missions et du périmètre (socle 6, lot P4). Préfixe `/personnels/missions`,
//! `/personnels/moi/missions`, `/personnels/acces`.
//!
//! L'autorité est `ent-db` (`personnels_missions.rs`) : droits (`missions.gerer`, `missions.types.gerer`,
//! `acces.revue`, `personnels.services.lire`), cloisonnement par établissement (404 uniforme), journal,
//! événements. Les routes ne font que décoder et mapper les erreurs. Types : `ent_types::personnels_missions`.
//! Le port PRS-67 (types réservés à un autre socle) n'a PAS de route : un autre socle l'appelle en Rust.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_types::personnels_missions::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/personnels/missions", get(missions).post(nommer))
        .route("/personnels/missions/types", get(types).post(creer_type))
        .route(
            "/personnels/missions/professeurs-principaux",
            get(tableau_pp).post(nommer_pp_lot),
        )
        .route("/personnels/missions/reconduire", post(reconduire))
        .route("/personnels/missions/simuler", post(simuler))
        .route("/personnels/missions/{id}/terminer", post(terminer))
        .route("/personnels/missions/{id}/transferer", post(transferer))
        .route("/personnels/missions/{id}/annuler", post(annuler))
        .route("/personnels/moi/missions", get(mes_missions))
        .route("/personnels/acces/expliquer", get(expliquer))
}

async fn missions(
    State(state): State<AppState>,
    session: EntSession,
    Query(f): Query<FiltreMissions>,
) -> Res<Json<Vec<MissionDto>>> {
    state
        .ent
        .missions_for_actor(session.etablissement_id, session.user_id, &session.role, &f)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn types(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<TypeMissionDto>>> {
    state
        .ent
        .types_mission_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creer_type(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerTypeMissionBody>,
) -> Res<(StatusCode, Json<TypeMissionDto>)> {
    state
        .ent
        .creer_type_mission_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|t| (StatusCode::CREATED, Json(t)))
        .map_err(map_db)
}

async fn nommer(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<NommerMissionBody>,
) -> Res<(StatusCode, Json<MissionDto>)> {
    state
        .ent
        .nommer_mission_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|m| (StatusCode::CREATED, Json(m)))
        .map_err(map_db)
}

#[derive(Deserialize)]
struct AnneeQuery {
    #[serde(default)]
    annee_id: Option<Uuid>,
}

async fn tableau_pp(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<AnneeQuery>,
) -> Res<Json<TableauPpDto>> {
    state
        .ent
        .tableau_pp_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.annee_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn nommer_pp_lot(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<NommerPpLotBody>,
) -> Res<(StatusCode, Json<LotMissionsDto>)> {
    state
        .ent
        .nommer_pp_lot_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|m| (StatusCode::CREATED, Json(m)))
        .map_err(map_db)
}

async fn terminer(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TerminerMissionBody>,
) -> Res<Json<MissionDto>> {
    state
        .ent
        .terminer_mission_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn transferer(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TransfererMissionBody>,
) -> Res<(StatusCode, Json<MissionDto>)> {
    state
        .ent
        .transferer_mission_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(|m| (StatusCode::CREATED, Json(m)))
        .map_err(map_db)
}

async fn annuler(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<MissionDto>> {
    state
        .ent
        .annuler_mission_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn reconduire(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<ReconduireMissionsBody>,
) -> Res<Json<ReconductionDto>> {
    state
        .ent
        .reconduire_missions_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn simuler(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<SimulerMissionBody>,
) -> Res<Json<SimulationDto>> {
    state
        .ent
        .simuler_mission_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn mes_missions(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<MissionDto>>> {
    state
        .ent
        .mes_missions_for_actor(session.etablissement_id, session.user_id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct ExpliquerQuery {
    agent_id: Uuid,
    eleve_id: Uuid,
}

async fn expliquer(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<ExpliquerQuery>,
) -> Res<Json<ExplicationAccesDto>> {
    state
        .ent
        .expliquer_acces_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.agent_id,
            q.eleve_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}
