//! Routes des responsables de groupes et des équipes (socle 6, lot P5). Préfixes `/personnels/groupes`
//! et `/personnels/equipes`.
//!
//! L'autorité est `ent-db` (`personnels_equipes.rs`) : droits (`missions.gerer`, `equipes.gerer`,
//! `personnels.services.lire`), cloisonnement par établissement (404 uniforme), confidentialité des équipes,
//! journal, événements. Les routes ne font que décoder et mapper les erreurs. Types : `ent_types::personnels_equipes`.
//! Une équipe ne donne aucun droit sur les élèves (Q-P9). Le port PRS-74 (`equipe_destinataires`) n'a pas de route :
//! le socle de messagerie l'appelle en Rust.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_types::personnels_equipes::*;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/personnels/groupes/{id}/responsables",
            get(responsables).post(ajouter_responsable),
        )
        .route(
            "/personnels/groupes/responsables/{id}/terminer",
            post(terminer_responsable),
        )
        .route("/personnels/equipes/calculee", get(calculee))
        .route("/personnels/equipes/types", get(types_equipe))
        .route("/personnels/equipes", get(equipes).post(creer_equipe))
        .route("/personnels/equipes/{id}", get(equipe))
        .route("/personnels/equipes/{id}/membres", post(ajouter_membre))
        .route(
            "/personnels/equipes/{id}/membres/{membre}/terminer",
            post(terminer_membre),
        )
        .route("/personnels/equipes/{id}/clore", post(clore))
}

async fn responsables(
    State(state): State<AppState>,
    session: EntSession,
    Path(groupe): Path<Uuid>,
) -> Res<Json<Vec<ResponsableGroupeDto>>> {
    state
        .ent
        .responsables_groupe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            groupe,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn ajouter_responsable(
    State(state): State<AppState>,
    session: EntSession,
    Path(groupe): Path<Uuid>,
    Json(b): Json<AjouterResponsableBody>,
) -> Res<(StatusCode, Json<ResponsableGroupeDto>)> {
    state
        .ent
        .ajouter_responsable_groupe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            groupe,
            &b,
        )
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn terminer_responsable(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TerminerResponsableBody>,
) -> Res<Json<ResponsableGroupeDto>> {
    state
        .ent
        .terminer_responsable_groupe_for_actor(
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

async fn calculee(
    State(state): State<AppState>,
    session: EntSession,
    Query(f): Query<FiltreEquipeCalculee>,
) -> Res<Json<EquipeCalculeeDto>> {
    state
        .ent
        .equipe_calculee_for_actor(session.etablissement_id, session.user_id, &session.role, &f)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn types_equipe(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<TypeEquipeDto>>> {
    state
        .ent
        .types_equipe_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn equipes(
    State(state): State<AppState>,
    session: EntSession,
    Query(f): Query<FiltreEquipes>,
) -> Res<Json<Vec<EquipeDto>>> {
    state
        .ent
        .equipes_for_actor(session.etablissement_id, session.user_id, &session.role, &f)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creer_equipe(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerEquipeBody>,
) -> Res<(StatusCode, Json<EquipeDto>)> {
    state
        .ent
        .creer_equipe_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|e| (StatusCode::CREATED, Json(e)))
        .map_err(map_db)
}

async fn equipe(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<EquipeDto>> {
    state
        .ent
        .equipe_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn ajouter_membre(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<NouveauMembreBody>,
) -> Res<Json<EquipeDto>> {
    state
        .ent
        .ajouter_membre_equipe_for_actor(
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

async fn terminer_membre(
    State(state): State<AppState>,
    session: EntSession,
    Path((id, membre)): Path<(Uuid, Uuid)>,
    Json(b): Json<TerminerMembreBody>,
) -> Res<Json<EquipeDto>> {
    state
        .ent
        .terminer_membre_equipe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            membre,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn clore(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ClorePeriodeBody>,
) -> Res<Json<EquipeDto>> {
    state
        .ent
        .clore_equipe_for_actor(
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
