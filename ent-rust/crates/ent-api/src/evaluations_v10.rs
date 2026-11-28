//! Routes `/eva/v10/*` du lot V10 « pilotage et statistiques » (socle 21) : évaluations communes et leur comparaison,
//! écarts à l'examen blanc, statistiques de période, répartition par nature, harmonisation d'un niveau, charge d'une
//! classe, évaluations non publiées, journal des consultations.
//!
//! L'autorité est `ent-db` (`evaluations_v10.rs`) : direction (`periodes.verrouiller`), professeur du cours, professeur
//! principal ; journal des consultations (R-V44). Ces routes ne font que traduire HTTP ↔ service. **Personnel
//! seulement** : la famille et l'élève reçoivent 403. Le jour de référence est celui de l'établissement.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::evaluations_v10::{
    AxeStatistiques, ChargeClasseDto, ComparaisonCommuneDto, CommuneDto, ConsultationDto, CreerCommuneBody,
    EcartsExamenDto, EvaluationsBody, HarmonisationDto, NonPublieesDto, RepartitionNaturesDto,
    StatistiquesPeriodeDto,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/eva/v10/communes", get(communes).post(creer_commune))
        .route("/eva/v10/communes/{id}", get(commune))
        .route("/eva/v10/communes/{id}/rattacher", post(rattacher))
        .route("/eva/v10/communes/{id}/detacher", post(detacher))
        .route("/eva/v10/communes/{id}/comparaison", get(comparaison))
        .route("/eva/v10/communes/{id}/ecarts-examen", get(ecarts_examen))
        .route("/eva/v10/periodes/{id}/statistiques", get(statistiques))
        .route("/eva/v10/periodes/{id}/natures", get(natures))
        .route(
            "/eva/v10/periodes/{id}/niveaux/{niveau}/harmonisation",
            get(harmonisation),
        )
        .route("/eva/v10/classes/{id}/charge", get(charge))
        .route("/eva/v10/non-publiees", get(non_publiees))
        .route("/eva/v10/consultations", get(consultations))
}

async fn jour(s: &AppState, session: &EntSession) -> Res<NaiveDate> {
    s.ent
        .aujourdhui_etablissement(session.etablissement_id)
        .await
        .map_err(map_db)
}

async fn communes(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<CommuneDto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v10_communes_for_actor(session.etablissement_id, session.user_id, &session.role, j)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creer_commune(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerCommuneBody>,
) -> Res<(StatusCode, Json<CommuneDto>)> {
    s.ent
        .eva_v10_creer_commune_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn commune(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<CommuneDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v10_commune_for_actor(session.etablissement_id, session.user_id, &session.role, j, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn rattacher(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<EvaluationsBody>,
) -> Res<Json<CommuneDto>> {
    s.ent
        .eva_v10_rattacher_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn detacher(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<EvaluationsBody>,
) -> Res<Json<CommuneDto>> {
    s.ent
        .eva_v10_detacher_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn comparaison(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ComparaisonCommuneDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v10_comparer_commune_for_actor(session.etablissement_id, session.user_id, &session.role, j, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn ecarts_examen(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<EcartsExamenDto>> {
    s.ent
        .eva_v10_ecarts_examen_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct AxeQuery {
    axe: Option<AxeStatistiques>,
}

async fn statistiques(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<AxeQuery>,
) -> Res<Json<StatistiquesPeriodeDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v10_statistiques_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            q.axe.unwrap_or(AxeStatistiques::Matiere),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn natures(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<RepartitionNaturesDto>> {
    s.ent
        .eva_v10_natures_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn harmonisation(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, niveau)): Path<(Uuid, Uuid)>,
) -> Res<Json<HarmonisationDto>> {
    s.ent
        .eva_v10_harmonisation_for_actor(session.etablissement_id, session.user_id, &session.role, id, niveau)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct FenetreQuery {
    du: NaiveDate,
    au: NaiveDate,
}

async fn charge(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<FenetreQuery>,
) -> Res<Json<ChargeClasseDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v10_charge_for_actor(session.etablissement_id, session.user_id, &session.role, j, id, q.du, q.au)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct DelaiQuery {
    delai_jours: Option<i64>,
}

async fn non_publiees(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<DelaiQuery>,
) -> Res<Json<NonPublieesDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v10_non_publiees_for_actor(session.etablissement_id, session.user_id, &session.role, j, q.delai_jours)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct ActeurQuery {
    acteur_id: Option<Uuid>,
}

async fn consultations(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<ActeurQuery>,
) -> Res<Json<Vec<ConsultationDto>>> {
    s.ent
        .eva_v10_consultations_for_actor(session.etablissement_id, session.user_id, &session.role, q.acteur_id)
        .await
        .map(Json)
        .map_err(map_db)
}
