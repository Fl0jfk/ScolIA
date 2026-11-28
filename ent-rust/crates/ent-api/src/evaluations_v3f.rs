//! Routes `/eva/v3f/*` du lot V3f (socle 21, complément de V3) : barème détaillé, réussite par exercice, détail par
//! élève, conversion des notes (changement de barème, exercice annulé), duplication de plusieurs évaluations.
//!
//! L'autorité est `ent-db` (`evaluations_v3f.rs`) : droits sur le cours, propriétaire de l'évaluation, période de
//! l'élève, version attendue, empreinte de l'aperçu, élève protégé. Ces routes ne font que traduire HTTP ↔ service.
//! **Tout est réservé au personnel** (la famille et l'élève reçoivent 403), sauf
//! `/eva/v3f/eleves/{eleve}/evaluations/{evaluation}/detail` : le détail d'une note **publiée**, si le professeur l'a
//! montré, sous le lien « voir les notes ». Le jour de référence est celui de l'établissement.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use ent_types::evaluations_v3f::{
    ApercuConversionDto, AppliquerConversionBody, BaremeDetailleDto, ConversionAppliqueeDto,
    ConversionBody, DefinirBaremeBody, DetailFamilleDto, DetailsEvaluationDto, DuplicationDto,
    DupliquerBody, SaisirDetailsBody, SaisirDetailsRetourDto, VisibiliteDetailBody,
};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/eva/v3f/evaluations/{id}/bareme",
            get(bareme).put(definir_bareme),
        )
        .route(
            "/eva/v3f/evaluations/{id}/bareme/visibilite",
            put(visibilite),
        )
        .route(
            "/eva/v3f/evaluations/{id}/details",
            get(details).post(saisir_details),
        )
        .route(
            "/eva/v3f/evaluations/{id}/conversion/apercu",
            post(apercu_conversion),
        )
        .route(
            "/eva/v3f/evaluations/{id}/conversion",
            post(appliquer_conversion),
        )
        .route("/eva/v3f/dupliquer", post(dupliquer))
        .route(
            "/eva/v3f/eleves/{eleve}/evaluations/{evaluation}/detail",
            get(detail_famille),
        )
}

async fn jour(s: &AppState, session: &EntSession) -> Res<chrono::NaiveDate> {
    s.ent
        .aujourdhui_etablissement(session.etablissement_id)
        .await
        .map_err(map_db)
}

async fn bareme(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<BaremeDetailleDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3f_bareme_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn definir_bareme(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DefinirBaremeBody>,
) -> Res<Json<BaremeDetailleDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3f_definir_bareme_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn visibilite(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<VisibiliteDetailBody>,
) -> Res<Json<BaremeDetailleDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3f_visibilite_detail_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn details(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<DetailsEvaluationDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3f_details_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn saisir_details(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SaisirDetailsBody>,
) -> Res<Json<SaisirDetailsRetourDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3f_saisir_details_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn apercu_conversion(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ConversionBody>,
) -> Res<Json<ApercuConversionDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3f_apercu_conversion_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn appliquer_conversion(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AppliquerConversionBody>,
) -> Res<Json<ConversionAppliqueeDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3f_appliquer_conversion_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn dupliquer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DupliquerBody>,
) -> Res<(StatusCode, Json<DuplicationDto>)> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3f_dupliquer_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            &b,
        )
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn detail_famille(
    State(s): State<AppState>,
    session: EntSession,
    Path((eleve, evaluation)): Path<(Uuid, Uuid)>,
) -> Res<Json<DetailFamilleDto>> {
    s.ent
        .eva_v3f_detail_famille_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            eleve,
            evaluation,
        )
        .await
        .map(Json)
        .map_err(map_db)
}
