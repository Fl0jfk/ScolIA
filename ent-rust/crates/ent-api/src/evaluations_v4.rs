//! Routes `/eva/v4/*` du lot V4 « absences, dispenses, aménagements, rattrapages » (socle 21).
//!
//! L'autorité est `ent-db` (`evaluations_v4.rs`) : droits du cours, `notes.corriger`, élève visible (socle 2), période par
//! élève, écriture commune d'un résultat. Ces routes ne font que traduire HTTP ↔ service. **Personnel seulement** : la
//! famille et l'élève reçoivent 403 sur toutes ; une évaluation d'un autre établissement répond 404. Le jour de référence
//! est celui de l'établissement.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use ent_types::evaluations_v4::{
    AbsenceRepeteeDto, AmenagementsEvaluationDto, AnnulerRattrapageBody, AppliquerPropositionsBody,
    AttribuerZeroBody, DeclarerDispenseBody, DeplacerRattrapageBody, DispenseDto,
    NoterRattrapageBody, PlanifierRattrapageBody, PropositionAppliqueeDto, PropositionsDto,
    RattrapageDto, ReconvoquerBody, ZeroAttribueDto,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/eva/v4/evaluations/{id}/propositions", get(propositions))
        .route("/eva/v4/propositions/appliquer", post(appliquer))
        .route("/eva/v4/evaluations/{id}/amenagements", get(amenagements))
        .route("/eva/v4/evaluations/{id}/rattrapages", get(rattrapages))
        .route("/eva/v4/rattrapages", post(planifier))
        .route("/eva/v4/rattrapages/{id}", put(deplacer))
        .route("/eva/v4/rattrapages/{id}/resultat", post(noter))
        .route("/eva/v4/rattrapages/{id}/annuler", post(annuler))
        .route("/eva/v4/rattrapages/{id}/reconvoquer", post(reconvoquer))
        .route("/eva/v4/remplacements", post(convoquer_remplacement))
        .route("/eva/v4/rattrapages/{id}/zero", post(zero))
        .route("/eva/v4/dispenses", post(declarer_dispense).get(dispenses))
        .route("/eva/v4/dispenses/{id}/retirer", post(retirer_dispense))
        .route("/eva/v4/absences-repetees", get(absences_repetees))
}

async fn jour(s: &AppState, session: &EntSession) -> Res<chrono::NaiveDate> {
    s.ent
        .aujourdhui_etablissement(session.etablissement_id)
        .await
        .map_err(map_db)
}

async fn propositions(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<PropositionsDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v4_propositions_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            j,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn appliquer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<AppliquerPropositionsBody>,
) -> Res<Json<Vec<PropositionAppliqueeDto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v4_appliquer_propositions_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn amenagements(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<AmenagementsEvaluationDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v4_amenagements_evaluation_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            j,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn rattrapages(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<RattrapageDto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v4_rattrapages_evaluation_for_actor(
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

async fn planifier(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<PlanifierRattrapageBody>,
) -> Res<(StatusCode, Json<Vec<RattrapageDto>>)> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v4_planifier_rattrapage_for_actor(
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

async fn convoquer_remplacement(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<PlanifierRattrapageBody>,
) -> Res<(StatusCode, Json<Vec<RattrapageDto>>)> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v4_convoquer_remplacement_for_actor(
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

async fn deplacer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DeplacerRattrapageBody>,
) -> Res<Json<RattrapageDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v4_deplacer_rattrapage_for_actor(
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

async fn noter(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<NoterRattrapageBody>,
) -> Res<Json<RattrapageDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v4_noter_rattrapage_for_actor(
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

async fn annuler(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AnnulerRattrapageBody>,
) -> Res<Json<RattrapageDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v4_annuler_rattrapage_for_actor(
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

async fn reconvoquer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ReconvoquerBody>,
) -> Res<(StatusCode, Json<RattrapageDto>)> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v4_reconvoquer_for_actor(
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

async fn zero(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AttribuerZeroBody>,
) -> Res<Json<ZeroAttribueDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v4_attribuer_zero_for_actor(
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

async fn declarer_dispense(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DeclarerDispenseBody>,
) -> Res<(StatusCode, Json<DispenseDto>)> {
    s.ent
        .eva_v4_declarer_dispense_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &b,
        )
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

#[derive(Deserialize)]
struct DispensesQuery {
    eleve_id: Uuid,
}

async fn dispenses(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<DispensesQuery>,
) -> Res<Json<Vec<DispenseDto>>> {
    s.ent
        .eva_v4_dispenses_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.eleve_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn retirer_dispense(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .eva_v4_retirer_dispense_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct RepeteesQuery {
    periode_id: Uuid,
    cours_id: Option<Uuid>,
}

async fn absences_repetees(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<RepeteesQuery>,
) -> Res<Json<Vec<AbsenceRepeteeDto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v4_absences_repetees_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            q.periode_id,
            q.cours_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}
