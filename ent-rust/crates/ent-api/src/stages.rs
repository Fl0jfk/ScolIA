//! Routes des stages (socle 39, lot ST1 (a)). Préfixe `/stages`.
//!
//! L'autorité est `ent-db` (`stages_st1.rs`) : droits D-017 (`stages.parametrer`,
//! `stages.organismes.gerer`, `stages.periodes.gerer`, `stages.lire`), cloisonnement par
//! établissement (404 uniforme), verrou optimiste, événements dans la transaction. Les routes ne
//! font que décoder et mapper les erreurs. Types : `ent_types::stages`. Les routes historiques
//! (`/entreprises`, `/conventions-stage`) restent dans `routes.rs`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use ent_types::stages::*;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/stages/dispositifs", get(dispositifs))
        .route("/stages/dispositifs/{code}", put(activer_dispositif))
        .route("/stages/parametres", get(parametres).put(modifier_parametres))
        .route("/stages/organismes/{id}", get(organisme).put(modifier_organisme))
        .route("/stages/organismes/{id}/etat", post(changer_etat))
        .route("/stages/organismes/{id}/sites", get(sites).post(creer_site))
        .route("/stages/sites/{id}", put(modifier_site))
        .route("/stages/organismes/{id}/contacts", get(contacts).post(creer_contact))
        .route("/stages/contacts/{id}", put(modifier_contact))
        .route("/stages/contacts/{id}/desactiver", post(desactiver_contact))
        .route("/stages/periodes", get(periodes).post(creer_periode))
        .route("/stages/periodes/{id}", put(modifier_periode))
        .route("/stages/periodes/{id}/publier", post(publier_periode))
        .route("/stages/periodes/{id}/fermer", post(fermer_periode))
}

async fn dispositifs(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<DispositifStageDto>>> {
    s.ent.dispositifs_stage_for_actor(se.etablissement_id, se.user_id, &se.role).await.map(Json).map_err(map_db)
}

async fn activer_dispositif(
    State(s): State<AppState>,
    se: EntSession,
    Path(code): Path<String>,
    Json(b): Json<ActiverDispositifBody>,
) -> Res<StatusCode> {
    s.ent
        .activer_dispositif_stage_for_actor(se.etablissement_id, se.user_id, &se.role, &code, b.actif)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn parametres(State(s): State<AppState>, se: EntSession) -> Res<Json<ParametresStageDto>> {
    s.ent.parametres_stage_for_actor(se.etablissement_id, se.user_id, &se.role).await.map(Json).map_err(map_db)
}

async fn modifier_parametres(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<ModifierParametresStageBody>,
) -> Res<Json<ParametresStageDto>> {
    s.ent
        .modifier_parametres_stage_for_actor(se.etablissement_id, se.user_id, &se.role, b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn organisme(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<OrganismeStageDto>> {
    s.ent.organisme_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id).await.map(Json).map_err(map_db)
}

async fn modifier_organisme(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ModifierOrganismeBody>,
) -> Res<Json<OrganismeStageDto>> {
    s.ent
        .modifier_organisme_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id, b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn changer_etat(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ChangerEtatOrganismeBody>,
) -> Res<Json<OrganismeStageDto>> {
    s.ent
        .changer_etat_organisme_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id, b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn sites(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<SiteStageDto>>> {
    s.ent.sites_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id).await.map(Json).map_err(map_db)
}

async fn creer_site(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SiteStageBody>,
) -> Res<(StatusCode, Json<SiteStageDto>)> {
    s.ent
        .creer_site_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id, b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn modifier_site(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SiteStageBody>,
) -> Res<Json<SiteStageDto>> {
    s.ent
        .modifier_site_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id, b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn contacts(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<ContactStageDto>>> {
    s.ent.contacts_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id).await.map(Json).map_err(map_db)
}

async fn creer_contact(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ContactStageBody>,
) -> Res<(StatusCode, Json<ContactStageDto>)> {
    s.ent
        .creer_contact_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id, b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn modifier_contact(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ContactStageBody>,
) -> Res<Json<ContactStageDto>> {
    s.ent
        .modifier_contact_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id, b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn desactiver_contact(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<VersionBody>,
) -> Res<StatusCode> {
    s.ent
        .desactiver_contact_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id, b.version)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn periodes(
    State(s): State<AppState>,
    se: EntSession,
    Query(f): Query<FiltrePeriodesStage>,
) -> Res<Json<Vec<PeriodeStageDto>>> {
    s.ent.periodes_stage_for_actor(se.etablissement_id, se.user_id, &se.role, &f).await.map(Json).map_err(map_db)
}

async fn creer_periode(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<CreerPeriodeStageBody>,
) -> Res<(StatusCode, Json<PeriodeStageDto>)> {
    s.ent
        .creer_periode_stage_for_actor(se.etablissement_id, se.user_id, &se.role, b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn modifier_periode(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ModifierPeriodeStageBody>,
) -> Res<Json<PeriodeStageDto>> {
    s.ent
        .modifier_periode_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id, b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn publier_periode(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<VersionBody>,
) -> Res<Json<PeriodeStageDto>> {
    s.ent
        .publier_periode_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id, b.version)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn fermer_periode(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<VersionBody>,
) -> Res<Json<PeriodeStageDto>> {
    s.ent
        .fermer_periode_stage_for_actor(se.etablissement_id, se.user_id, &se.role, id, b.version)
        .await
        .map(Json)
        .map_err(map_db)
}
