//! Routes des services d'enseignement et des interventions (socle 6, lot P3). Préfixe
//! `/personnels/services`, `/personnels/interventions`, `/personnels/moi/services`.
//!
//! L'autorité est `ent-db` (`personnels_services.rs`) : droits (`services.gerer`,
//! `personnels.services.lire`), cloisonnement par établissement (404 uniforme), journal, événements.
//! Les routes ne font que décoder et mapper les erreurs. Types JSON : `ent_types::personnels_services`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_types::personnels_services::*;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/personnels/services", get(repartition).post(creer_service))
        .route(
            "/personnels/services/creneaux-sans-service",
            get(creneaux_sans_service),
        )
        .route(
            "/personnels/services/remplacer-agent",
            post(remplacer_agent),
        )
        .route(
            "/personnels/services/{id}",
            get(service).put(modifier_service),
        )
        .route(
            "/personnels/services/{id}/remplacer",
            post(remplacer_service),
        )
        .route("/personnels/services/{id}/clore", post(clore_service))
        .route("/personnels/services/{id}/annuler", post(annuler_service))
        .route("/personnels/services/{id}/ouvrir", post(ouvrir_service))
        .route(
            "/personnels/services/{id}/intervenants",
            post(ajouter_intervenant),
        )
        .route(
            "/personnels/services/{id}/suppleance",
            post(nommer_suppleant),
        )
        .route(
            "/personnels/interventions/{id}/terminer",
            post(terminer_intervention),
        )
        .route(
            "/personnels/interventions/{id}/corriger",
            post(corriger_intervention),
        )
        .route("/personnels/moi/services", get(mes_services))
}

async fn repartition(
    State(state): State<AppState>,
    session: EntSession,
    Query(f): Query<FiltreServices>,
) -> Res<Json<RepartitionDto>> {
    state
        .ent
        .repartition_for_actor(session.etablissement_id, session.user_id, &session.role, &f)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn service(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ServiceDto>> {
    state
        .ent
        .service_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creer_service(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerServiceBody>,
) -> Res<(StatusCode, Json<ServiceDto>)> {
    state
        .ent
        .creer_service_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|s| (StatusCode::CREATED, Json(s)))
        .map_err(map_db)
}

async fn modifier_service(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ModifierServiceBody>,
) -> Res<Json<ServiceDto>> {
    state
        .ent
        .modifier_service_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn remplacer_service(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RemplacerServiceBody>,
) -> Res<(StatusCode, Json<ServiceRemplaceDto>)> {
    state
        .ent
        .remplacer_service_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn clore_service(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<CloreServiceBody>>,
) -> Res<Json<ServiceDto>> {
    let b = body.map(|Json(b)| b).unwrap_or_default();
    state
        .ent
        .clore_service_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn annuler_service(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ServiceDto>> {
    state
        .ent
        .annuler_service_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn ouvrir_service(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ServiceDto>> {
    state
        .ent
        .ouvrir_service_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn ajouter_intervenant(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AjouterIntervenantBody>,
) -> Res<(StatusCode, Json<ServiceDto>)> {
    state
        .ent
        .ajouter_intervenant_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(|s| (StatusCode::CREATED, Json(s)))
        .map_err(map_db)
}

async fn nommer_suppleant(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<NommerSuppleantBody>,
) -> Res<(StatusCode, Json<ServiceDto>)> {
    state
        .ent
        .nommer_suppleant_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(|s| (StatusCode::CREATED, Json(s)))
        .map_err(map_db)
}

async fn terminer_intervention(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<TerminerInterventionBody>>,
) -> Res<Json<ServiceDto>> {
    let b = body.map(|Json(b)| b).unwrap_or_default();
    state
        .ent
        .terminer_intervention_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn corriger_intervention(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CorrigerInterventionBody>,
) -> Res<Json<ServiceDto>> {
    state
        .ent
        .corriger_intervention_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn remplacer_agent(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<RemplacerAgentBody>,
) -> Res<Json<RemplacementAgentDto>> {
    state
        .ent
        .remplacer_agent_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn mes_services(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<MesServicesDto>> {
    state
        .ent
        .mes_services_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creneaux_sans_service(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<CreneauSansServiceDto>>> {
    state
        .ent
        .creneaux_sans_service_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}
