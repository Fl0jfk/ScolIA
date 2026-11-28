//! Routes `/eva/v7/*` du lot V7 « famille et élève » (socle 21) : relevé expliqué, à venir, projet d'évaluation,
//! simulation, demandes de révision, glossaire.
//!
//! L'autorité est `ent-db` (`evaluations_v7.rs`) : rôle, lien « voir les notes » ou compte de l'élève, élève visible
//! (socle 2), publication, période de l'élève. Ces routes ne font que traduire HTTP ↔ service. **Lectures de l'élève :
//! famille et élève seulement** (le personnel reçoit 403) ; **simulation : l'élève seulement** ; **réponse aux demandes :
//! personnel seulement** (professeur du cours ou direction). Le jour de référence est celui de l'établissement.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_types::evaluations_v7::{
    glossaire, AVenirDto, DemanderRevisionBody, EntreeGlossaireDto, ProjetEvaluationDto, ReleveV7Dto,
    RepondreRevisionBody, RevisionDto, RevisionPersonnelDto, SimulationBody, SimulationDto,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/eva/v7/eleves/{id}/releve", get(releve))
        .route("/eva/v7/eleves/{id}/a-venir", get(a_venir))
        .route("/eva/v7/eleves/{id}/projet", get(projet))
        .route("/eva/v7/eleves/{id}/simulation", post(simuler))
        .route("/eva/v7/eleves/{id}/revisions", get(mes_revisions))
        .route("/eva/v7/revisions", post(demander).get(revisions_personnel))
        .route("/eva/v7/revisions/{id}/retirer", post(retirer))
        .route("/eva/v7/revisions/{id}/repondre", post(repondre))
        .route("/eva/v7/glossaire", get(lire_glossaire))
}

#[derive(Deserialize)]
struct PeriodeQuery {
    periode_id: Uuid,
}

#[derive(Deserialize)]
struct RevisionsQuery {
    /// `toutes` pour voir aussi les demandes closes ; ouvertes seulement par défaut.
    #[serde(default)]
    etat: Option<String>,
}

async fn jour(s: &AppState, session: &EntSession) -> Res<chrono::NaiveDate> {
    s.ent
        .aujourdhui_etablissement(session.etablissement_id)
        .await
        .map_err(map_db)
}

async fn releve(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<ReleveV7Dto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v7_releve_for_actor(session.etablissement_id, session.user_id, &session.role, id, q.periode_id, j)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn a_venir(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<AVenirDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v7_a_venir_for_actor(session.etablissement_id, session.user_id, &session.role, id, j)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn projet(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<ProjetEvaluationDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v7_projet_for_actor(session.etablissement_id, session.user_id, &session.role, id, q.periode_id, j)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn simuler(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SimulationBody>,
) -> Res<Json<SimulationDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v7_simuler_for_actor(session.etablissement_id, session.user_id, &session.role, id, j, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn mes_revisions(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<RevisionDto>>> {
    s.ent
        .eva_v7_mes_revisions_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn demander(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DemanderRevisionBody>,
) -> Res<(StatusCode, Json<RevisionDto>)> {
    s.ent
        .eva_v7_demander_revision_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn revisions_personnel(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<RevisionsQuery>,
) -> Res<Json<Vec<RevisionPersonnelDto>>> {
    let j = jour(&s, &session).await?;
    let ouvertes = q.etat.as_deref() != Some("toutes");
    s.ent
        .eva_v7_revisions_personnel_for_actor(session.etablissement_id, session.user_id, &session.role, j, ouvertes)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn retirer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    s.ent
        .eva_v7_retirer_revision_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn repondre(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RepondreRevisionBody>,
) -> Res<Json<RevisionPersonnelDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v7_repondre_revision_for_actor(session.etablissement_id, session.user_id, &session.role, j, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

/// Statique, pour tout compte connecté de l'établissement.
async fn lire_glossaire(_session: EntSession) -> Json<Vec<EntreeGlossaireDto>> {
    Json(glossaire())
}
