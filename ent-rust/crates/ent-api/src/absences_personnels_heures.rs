//! Routes des heures constatées, états mensuels, paie de l'OGEC et DSN (socle 15, lot A7).
//! Préfixes `/personnels/heures`, `/personnels/etats-heures`, `/personnels/paie`. L'autorité est `ent-db`
//! (`absences_personnels/heures.rs`) : droit par action **et** périmètre d'employeur (le secrétariat lit l'État, la
//! comptabilité l'OGEC, la direction les deux). Aucune route ne renvoie la nature d'une absence : les éléments de paie
//! portent un code de nature, réservé à la comptabilité et à la direction.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::absences_personnels::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/personnels/heures/moi", get(mes_heures))
        .route("/personnels/heures/moi/annuel", get(mon_etat_annuel))
        .route("/personnels/heures", get(heures).post(saisir))
        .route("/personnels/heures/attestation", post(attester))
        .route("/personnels/heures/validation", post(valider))
        .route("/personnels/heures/annuel/{agent_id}", get(etat_annuel_agent))
        .route("/personnels/heures/{id}/contestation", post(contester))
        .route("/personnels/heures/{id}/correction", post(corriger))
        .route("/personnels/heures/{id}/regularisation", post(regulariser))
        .route("/personnels/etats-heures", post(figer))
        .route("/personnels/etats-heures/{id}", get(etat))
        .route("/personnels/etats-heures/{id}/signature", post(signer))
        .route("/personnels/etats-heures/{id}/export.csv", get(exporter_etat))
        .route("/personnels/paie/elements", get(elements).post(preparer_paie))
        .route("/personnels/paie/export.csv", get(exporter_paie))
        .route("/personnels/paie/dsn", get(dsn))
        .route("/personnels/paie/dsn/{absence_id}/signale", post(dsn_signale))
}

#[derive(Deserialize)]
struct MoisQuery {
    #[serde(default)]
    mois: Option<NaiveDate>,
    #[serde(default)]
    agent_id: Option<Uuid>,
}

async fn mes_heures(State(s): State<AppState>, session: EntSession, Query(q): Query<MoisQuery>) -> Res<Json<MesHeuresDto>> {
    s.ent
        .mes_heures_for_actor(session.etablissement_id, session.user_id, &session.role, q.mois)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn mon_etat_annuel(State(s): State<AppState>, session: EntSession) -> Res<Json<EtatAnnuelDto>> {
    s.ent
        .etat_annuel_agent_for_actor(session.etablissement_id, session.user_id, &session.role, None, None)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn etat_annuel_agent(State(s): State<AppState>, session: EntSession, Path(agent): Path<Uuid>) -> Res<Json<EtatAnnuelDto>> {
    s.ent
        .etat_annuel_agent_for_actor(session.etablissement_id, session.user_id, &session.role, Some(agent), None)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn heures(State(s): State<AppState>, session: EntSession, Query(f): Query<FiltreHeures>) -> Res<Json<Vec<HeureDto>>> {
    s.ent
        .heures_for_actor(session.etablissement_id, session.user_id, &session.role, &f)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn saisir(State(s): State<AppState>, session: EntSession, Json(b): Json<SaisirHeureBody>) -> Res<(StatusCode, Json<HeureDto>)> {
    s.ent
        .saisir_heure_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn attester(State(s): State<AppState>, session: EntSession, Json(b): Json<AttesterBody>) -> Res<(StatusCode, Json<HeureDto>)> {
    s.ent
        .attester_seance_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn valider(State(s): State<AppState>, session: EntSession, Json(b): Json<ValiderHeuresBody>) -> Res<Json<BilanValidationDto>> {
    s.ent
        .valider_heures_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn contester(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<ContesterHeureBody>) -> Res<Json<HeureDto>> {
    s.ent
        .contester_heure_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn corriger(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<CorrigerHeureBody>) -> Res<Json<HeureDto>> {
    s.ent
        .corriger_heure_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn regulariser(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CorrigerHeureBody>,
) -> Res<(StatusCode, Json<RegularisationDto>)> {
    s.ent
        .regulariser_heure_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn figer(State(s): State<AppState>, session: EntSession, Json(b): Json<FigerEtatBody>) -> Res<(StatusCode, Json<EtatMensuelDto>)> {
    s.ent
        .figer_etat_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn etat(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<EtatMensuelDto>> {
    s.ent
        .etat_mensuel_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn signer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<EtatMensuelDto>> {
    s.ent
        .signer_etat_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

fn csv(corps: String, nom: &'static str) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{nom}\"")),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        corps,
    )
}

async fn exporter_etat(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<impl IntoResponse> {
    let c = s
        .ent
        .exporter_etat_csv_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(csv(c, "etat-heures.csv"))
}

async fn elements(State(s): State<AppState>, session: EntSession, Query(q): Query<MoisQuery>) -> Res<Json<Vec<ElementPaieDto>>> {
    let mois = q.mois.ok_or((StatusCode::BAD_REQUEST, "Indiquez le mois (premier jour).".to_string()))?;
    s.ent
        .elements_paie_for_actor(session.etablissement_id, session.user_id, &session.role, mois, q.agent_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn preparer_paie(State(s): State<AppState>, session: EntSession, Json(b): Json<PreparerPaieBody>) -> Res<Json<BilanPaieDto>> {
    s.ent
        .preparer_elements_paie_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn exporter_paie(State(s): State<AppState>, session: EntSession, Query(q): Query<MoisQuery>) -> Res<impl IntoResponse> {
    let mois = q.mois.ok_or((StatusCode::BAD_REQUEST, "Indiquez le mois (premier jour).".to_string()))?;
    let c = s
        .ent
        .exporter_elements_paie_csv_for_actor(session.etablissement_id, session.user_id, &session.role, mois)
        .await
        .map_err(map_db)?;
    Ok(csv(c, "elements-variables-paie.csv"))
}

async fn dsn(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<DsnSuiviDto>>> {
    s.ent
        .dsn_file_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn dsn_signale(
    State(s): State<AppState>,
    session: EntSession,
    Path(absence_id): Path<Uuid>,
    Json(b): Json<DsnSignaleBody>,
) -> Res<Json<DsnSuiviDto>> {
    s.ent
        .marquer_dsn_signale_for_actor(session.etablissement_id, session.user_id, &session.role, absence_id, b.le)
        .await
        .map(Json)
        .map_err(map_db)
}
