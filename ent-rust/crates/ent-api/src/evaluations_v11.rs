//! Routes `/eva/v11/*` du lot V11 « alertes précoces », partie déterministe (socle 21, §13.3, §13.8, R-V40, S-118) :
//! réglage des seuils, alertes d'une classe et d'un cours, « ce qui a changé depuis le dernier conseil », suites
//! humaines, exclusions écrites de l'IA, journal des consultations.
//!
//! L'autorité est `ent-db` (`evaluations_v11.rs`) : direction (`periodes.verrouiller`), professeur principal, CPE
//! référent, professeur du cours. Ces routes ne font que traduire HTTP ↔ service. **Personnel seulement** : la famille
//! et l'élève reçoivent 403. Le jour de référence est celui de l'établissement. Aucun modèle de langage.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::evaluations_v11::{
    AlertesClasseDto, AlertesCoursDto, ChangementsEleveDto, ConsultationAlerteDto,
    DefinirReglageAlertesBody, ExclusionIaDto, PoserSuiteBody, ReglageAlertesDto, SuiteDto,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/eva/v11/reglage-alertes",
            get(reglage).put(definir_reglage),
        )
        .route("/eva/v11/classes/{id}/alertes", get(alertes_classe))
        .route("/eva/v11/cours/{id}/alertes", get(alertes_cours))
        .route("/eva/v11/eleves/{id}/changements", get(changements))
        .route("/eva/v11/eleves/{id}/suites", get(suites).post(poser_suite))
        .route("/eva/v11/ia/exclusions", get(exclusions))
        .route("/eva/v11/consultations", get(consultations))
}

async fn jour(s: &AppState, session: &EntSession) -> Res<NaiveDate> {
    s.ent
        .aujourdhui_etablissement(session.etablissement_id)
        .await
        .map_err(map_db)
}

#[derive(Deserialize)]
struct AnneeQuery {
    annee: Uuid,
}

#[derive(Deserialize)]
struct PeriodeQuery {
    periode: Uuid,
}

#[derive(Deserialize)]
struct FenetreQuery {
    du: Option<NaiveDate>,
    au: Option<NaiveDate>,
}

async fn reglage(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AnneeQuery>,
) -> Res<Json<ReglageAlertesDto>> {
    s.ent
        .eva_v11_reglage_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.annee,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn definir_reglage(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DefinirReglageAlertesBody>,
) -> Res<Json<ReglageAlertesDto>> {
    s.ent
        .eva_v11_definir_reglage_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn alertes_classe(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<AlertesClasseDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v11_alertes_classe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            q.periode,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn alertes_cours(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<AlertesCoursDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v11_alertes_cours_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            q.periode,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn changements(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<ChangementsEleveDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v11_changements_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            q.periode,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn suites(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<Vec<SuiteDto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v11_suites_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            q.periode,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn poser_suite(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PoserSuiteBody>,
) -> Res<(StatusCode, Json<SuiteDto>)> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v11_poser_suite_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            &b,
        )
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn exclusions(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<ExclusionIaDto>>> {
    s.ent
        .eva_v11_exclusions_ia_for_actor(&session.role)
        .map(Json)
        .map_err(map_db)
}

async fn consultations(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<FenetreQuery>,
) -> Res<Json<Vec<ConsultationAlerteDto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v11_consultations_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            q.du,
            q.au,
        )
        .await
        .map(Json)
        .map_err(map_db)
}
