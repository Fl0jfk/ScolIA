//! Routes de réponse et de sollicitation des absences de personnels (socle 15, lot A4). Préfixe
//! `/personnels/absences`. L'autorité est `ent-db` (`absences_personnels/reponse.rs`) : droit d'organiser
//! (`edt.gerer` + lecture d'organisation), réponse du seul agent sollicité, exclusion en base des plages qui
//! se chevauchent. Niveau « organisation » : aucune route ne renvoie la nature ni le motif.

use axum::extract::{Path, State};
use axum::http::StatusCode;
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
        .route("/personnels/absences/mes-sollicitations", get(mes_sollicitations))
        .route("/personnels/absences/impacts/{impact_id}/propositions", get(propositions))
        .route("/personnels/absences/impacts/{impact_id}/solliciter", post(solliciter))
        .route("/personnels/absences/impacts/{impact_id}/appel-volontaires", post(appel_volontaires))
        .route("/personnels/absences/impacts/{impact_id}/designer", post(designer))
        .route("/personnels/absences/impacts/{impact_id}/lever", post(lever))
        .route("/personnels/absences/impacts/{impact_id}/annuler-seance", post(annuler_seance))
        .route("/personnels/absences/sollicitations/{id}/accepter", post(accepter))
        .route("/personnels/absences/sollicitations/{id}/refuser", post(refuser))
        .route("/personnels/absences/sollicitations/{id}/accuser", post(accuser))
}

async fn mes_sollicitations(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<SollicitationDto>>> {
    s.ent
        .mes_sollicitations_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn propositions(
    State(s): State<AppState>,
    session: EntSession,
    Path(impact_id): Path<Uuid>,
) -> Res<Json<Vec<PropositionDto>>> {
    s.ent
        .propositions_impact_for_actor(session.etablissement_id, session.user_id, &session.role, impact_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn solliciter(
    State(s): State<AppState>,
    session: EntSession,
    Path(impact_id): Path<Uuid>,
    Json(b): Json<SolliciterBody>,
) -> Res<(StatusCode, Json<SollicitationDto>)> {
    s.ent
        .solliciter_for_actor(session.etablissement_id, session.user_id, &session.role, impact_id, b.agent_id)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn appel_volontaires(
    State(s): State<AppState>,
    session: EntSession,
    Path(impact_id): Path<Uuid>,
    Json(b): Json<AppelVolontairesBody>,
) -> Res<(StatusCode, Json<Vec<SollicitationDto>>)> {
    s.ent
        .appel_volontaires_for_actor(session.etablissement_id, session.user_id, &session.role, impact_id, &b.agents)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn designer(
    State(s): State<AppState>,
    session: EntSession,
    Path(impact_id): Path<Uuid>,
    Json(b): Json<SolliciterBody>,
) -> Res<(StatusCode, Json<SollicitationDto>)> {
    s.ent
        .designer_for_actor(session.etablissement_id, session.user_id, &session.role, impact_id, b.agent_id)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn lever(
    State(s): State<AppState>,
    session: EntSession,
    Path(impact_id): Path<Uuid>,
) -> Res<Json<ImpactDto>> {
    s.ent
        .lever_reponse_for_actor(session.etablissement_id, session.user_id, &session.role, impact_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn annuler_seance(
    State(s): State<AppState>,
    session: EntSession,
    Path(impact_id): Path<Uuid>,
    Json(b): Json<AnnulerSeanceBody>,
) -> Res<Json<ImpactDto>> {
    s.ent
        .annuler_seance_for_actor(session.etablissement_id, session.user_id, &session.role, impact_id, b.sortie_anticipee)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn accepter(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<SollicitationDto>> {
    s.ent
        .accepter_sollicitation_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn refuser(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RefuserSollicitationBody>,
) -> Res<Json<SollicitationDto>> {
    s.ent
        .refuser_sollicitation_for_actor(session.etablissement_id, session.user_id, &session.role, id, b.refus_code.as_deref())
        .await
        .map(Json)
        .map_err(map_db)
}

async fn accuser(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<SollicitationDto>> {
    s.ent
        .accuser_reception_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}
