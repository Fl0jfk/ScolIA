//! Routes `/cahier/c6/*` du lot C6 du socle 20 : continuité (`20_CAHIER_DE_TEXTES.md` §8.4, §12).
//!
//! * `GET /cahier/c6/reports`, `POST /cahier/c6/reports/{id}/decision`, `POST /cahier/c6/reports/lot` : propositions de report
//!   après un changement de séance, traitées une à une ou en lot (droit : saisie sur le cours à la date, ou direction).
//! * `GET` / `POST /cahier/c6/continuites`, `POST /cahier/c6/continuites/{id}/cloture` : continuité d'un élève (PP, CPE,
//!   direction ; l'exclusion temporaire : vie scolaire et direction) ; aucun motif.
//! * `GET /cahier/c6/cours/{id}/bandeau` : élèves en continuité à distance, poste vacant, suppléance, copies à décider.
//! * `GET /cahier/c6/vacants`, `POST /cahier/c6/vacants/travaux` : poste vacant (direction).
//! * `POST /cahier/c6/seances/{id}/travail` : travail en l'absence du professeur ou à distance (fermeture).
//! * `GET /cahier/c6/seances/{id}/remplacement` : vue du remplaçant désigné ; `GET /cahier/c6/cours/{id}/suppleance` : accueil
//!   du suppléant ; `POST /cahier/c6/cours/{id}/copies` : décision sur les copies en attente ;
//!   `GET /cahier/c6/travaux-remplacants` : travaux donnés par un remplaçant pour ma séance.
//!
//! L'établissement vient de la session, jamais de la requête.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_types::cahier_c2::TravailC2Body;
use ent_types::cahier_c6::*;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cahier/c6/reports", get(reports))
        .route("/cahier/c6/reports/lot", post(reports_lot))
        .route("/cahier/c6/reports/{id}/decision", post(report_decision))
        .route("/cahier/c6/continuites", get(continuites).post(continuite_ouvrir))
        .route("/cahier/c6/continuites/{id}/cloture", post(continuite_clore))
        .route("/cahier/c6/cours/{id}/bandeau", get(bandeau))
        .route("/cahier/c6/cours/{id}/suppleance", get(suppleance))
        .route("/cahier/c6/cours/{id}/copies", post(copies_decider))
        .route("/cahier/c6/vacants", get(vacants))
        .route("/cahier/c6/vacants/travaux", post(vacant_publier))
        .route("/cahier/c6/seances/{id}/travail", post(travail_seance))
        .route("/cahier/c6/seances/{id}/remplacement", get(remplacement))
        .route("/cahier/c6/travaux-remplacants", get(travaux_remplacants))
}

async fn reports(State(s): State<AppState>, session: EntSession) -> Res<Json<ReportsDto>> {
    Ok(Json(s.ent.cdt_reports_lister(&acteur(&session)).await.map_err(map_db)?))
}

async fn reports_lot(State(s): State<AppState>, session: EntSession, Json(b): Json<ReportsLotBody>) -> Res<Json<ReportsLotDto>> {
    Ok(Json(s.ent.cdt_reports_lot(&acteur(&session), b).await.map_err(map_db)?))
}

async fn report_decision(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<ReportDecisionBody>) -> Res<Json<ReportDto>> {
    Ok(Json(s.ent.cdt_report_traiter(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn continuites(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<ContinuiteDto>>> {
    Ok(Json(s.ent.cdt_continuites_lister(&acteur(&session)).await.map_err(map_db)?))
}

async fn continuite_ouvrir(State(s): State<AppState>, session: EntSession, Json(b): Json<ContinuiteBody>) -> Res<(StatusCode, Json<ContinuiteDto>)> {
    Ok((StatusCode::CREATED, Json(s.ent.cdt_continuite_ouvrir(&acteur(&session), b).await.map_err(map_db)?)))
}

async fn continuite_clore(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<ContinuiteCloreBody>) -> Res<StatusCode> {
    s.ent.cdt_continuite_clore(&acteur(&session), id, b).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn bandeau(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<BandeauCoursDto>> {
    Ok(Json(s.ent.cdt_cours_bandeau(&acteur(&session), id).await.map_err(map_db)?))
}

async fn suppleance(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<SuppleanceAccueilDto>> {
    Ok(Json(s.ent.cdt_suppleance_accueil(&acteur(&session), id).await.map_err(map_db)?))
}

async fn copies_decider(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<CopiesDecisionBody>) -> Res<Json<CopiesDecisionDto>> {
    Ok(Json(s.ent.cdt_copies_decider(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn vacants(State(s): State<AppState>, session: EntSession) -> Res<Json<VacantsDto>> {
    Ok(Json(s.ent.cdt_vacants_lister(&acteur(&session)).await.map_err(map_db)?))
}

async fn vacant_publier(State(s): State<AppState>, session: EntSession, Json(b): Json<TravailC2Body>) -> Res<(StatusCode, Json<Vec<Uuid>>)> {
    Ok((StatusCode::CREATED, Json(s.ent.cdt_vacant_publier(&acteur(&session), b).await.map_err(map_db)?)))
}

async fn travail_seance(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<TravailSeanceBody>) -> Res<(StatusCode, Json<TravailSeanceDto>)> {
    Ok((StatusCode::CREATED, Json(s.ent.cdt_travail_seance(&acteur(&session), id, b).await.map_err(map_db)?)))
}

async fn remplacement(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<RemplacementVueDto>> {
    Ok(Json(s.ent.cdt_remplacement_vue(&acteur(&session), id).await.map_err(map_db)?))
}

async fn travaux_remplacants(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<TravailDeRemplacantDto>>> {
    Ok(Json(s.ent.cdt_travaux_de_remplacants(&acteur(&session)).await.map_err(map_db)?))
}
