//! Routes `/cahier/c4/*` du lot C4 du socle 20 : la charge de travail (`20_CAHIER_DE_TEXTES.md` §11).
//!
//! * `POST /cahier/c4/apercu` : au moment de donner un travail, la charge des élèves réels du ou des cours (nombres et
//!   natures seulement), l'alerte éventuelle et deux dates alternatives. Droit : saisie dans chaque cours, à la date.
//! * `GET /cahier/c4/charge` : la charge d'un élève, pour lui-même ou ses responsables habilités.
//! * `GET /cahier/c4/classes/{id}/calendrier` : le calendrier d'une classe (équipe, PP, direction : `cahier.lire`).
//! * `POST /cahier/c4/travaux/{id}/alerte-ignoree` et `GET /cahier/c4/alertes` : les dérogations, comptées.
//! * `GET` / `PUT /cahier/c4/reperes` : les repères à trois positions de l'établissement (réglage : `cahier.parametrer`).
//!
//! L'établissement vient de la session, jamais de la requête ; ni nom d'élève, ni titre d'un travail d'un collègue, ni matière
//! hors de la classe du cours ne sortent d'ici.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::cahier_c4::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cahier/c4/reperes", get(reperes).put(reperes_regler))
        .route("/cahier/c4/apercu", post(apercu))
        .route("/cahier/c4/charge", get(charge))
        .route("/cahier/c4/classes/{id}/calendrier", get(calendrier))
        .route("/cahier/c4/travaux/{id}/alerte-ignoree", post(alerte_ignoree))
        .route("/cahier/c4/alertes", get(alertes))
}

#[derive(Debug, Deserialize)]
struct PeriodeQuery {
    du: NaiveDate,
    au: NaiveDate,
}

#[derive(Debug, Deserialize)]
struct ChargeQuery {
    eleve_id: Option<Uuid>,
    du: NaiveDate,
    au: NaiveDate,
}

async fn reperes(State(s): State<AppState>, session: EntSession) -> Res<Json<ReperesDto>> {
    Ok(Json(s.ent.cdt_reperes(&acteur(&session)).await.map_err(map_db)?))
}

async fn reperes_regler(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<PositionBody>,
) -> Res<Json<ReperesDto>> {
    Ok(Json(s.ent.cdt_reperes_regler(&acteur(&session), &b.position).await.map_err(map_db)?))
}

async fn apercu(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ApercuBody>,
) -> Res<Json<ApercuChargeDto>> {
    Ok(Json(s.ent.cdt_charge_apercu(&acteur(&session), b).await.map_err(map_db)?))
}

async fn charge(State(s): State<AppState>, session: EntSession, Query(q): Query<ChargeQuery>) -> Res<Json<ChargeEleveDto>> {
    Ok(Json(s.ent.cdt_charge_eleve(&acteur(&session), q.eleve_id, q.du, q.au).await.map_err(map_db)?))
}

async fn calendrier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<CalendrierClasseDto>> {
    Ok(Json(s.ent.cdt_calendrier_classe(&acteur(&session), id, q.du, q.au).await.map_err(map_db)?))
}

async fn alerte_ignoree(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<AlerteChargeDto>>> {
    Ok(Json(s.ent.cdt_alerte_ignorer(&acteur(&session), id).await.map_err(map_db)?))
}

async fn alertes(State(s): State<AppState>, session: EntSession, Query(q): Query<PeriodeQuery>) -> Res<Json<AlertesIgnoreesDto>> {
    Ok(Json(s.ent.cdt_alertes_ignorees(&acteur(&session), q.du, q.au).await.map_err(map_db)?))
}
