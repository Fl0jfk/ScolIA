//! Routes des absences collectives (socle 15, lot A9) : grève, tableau de crise, retenue signalée. Préfixe
//! `/personnels/greves`. L'autorité est `ent-db` (`absences_personnels/greve.rs`). La **liste des grévistes** (nature
//! sensible, opinion) n'est lisible que de la direction et du secrétariat : un autre profil est refusé (403), jamais
//! renvoyé vide. Le tableau de crise ne porte que des nombres et des classes.

use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::absences_personnels::*;
use serde::Deserialize;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/personnels/greves", get(liste).post(cocher))
        .route("/personnels/greves/moi", post(se_declarer))
        .route("/personnels/greves/annulation", post(annuler))
        .route("/personnels/greves/crise", get(crise))
        .route("/personnels/greves/retenue.csv", get(retenue))
}

#[derive(Deserialize)]
struct JourQuery {
    jour: NaiveDate,
}

#[derive(Deserialize)]
struct MoisQuery {
    mois: NaiveDate,
}

async fn cocher(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CocherGrevistesBody>,
) -> Res<(StatusCode, Json<CocheGrevistesDto>)> {
    s.ent
        .cocher_grevistes_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn liste(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Res<Json<Vec<GrevisteDto>>> {
    s.ent
        .grevistes_du_jour_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.jour,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn se_declarer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DeclarerGreveBody>,
) -> Res<(StatusCode, Json<GrevisteDto>)> {
    s.ent
        .declarer_greve_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn annuler(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<AnnulerGreveBody>,
) -> Res<Json<AnnulationGreveDto>> {
    s.ent
        .annuler_seances_greve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            b.jour,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn crise(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Res<Json<TableauCriseDto>> {
    s.ent
        .tableau_de_crise_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.jour,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn retenue(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<MoisQuery>,
) -> Res<impl IntoResponse> {
    let c = s
        .ent
        .export_retenue_greve_csv_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.mois,
        )
        .await
        .map_err(map_db)?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"jours-de-greve.csv\"".to_string(),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        c,
    ))
}
