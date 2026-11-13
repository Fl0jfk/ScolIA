//! Routes de l'impact, des tableaux et des files des absences de personnels (socle 15, lot A3). Préfixe
//! `/personnels/absences`. L'autorité est `ent-db` (`absences_personnels/impact.rs`) : droits
//! `personnels.absences.*` (D-017), cercle RH et confidentialité renforcée, 404 uniforme hors cercle. Ces
//! routes sont du niveau « organisation » : aucune ne renvoie la nature ni le motif (R-AP3).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::absences_personnels::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/personnels/absences/tableau", get(tableau))
        .route("/personnels/absences/compteurs", get(compteurs))
        .route("/personnels/absences/a-traiter", get(a_traiter))
        .route("/personnels/absences/presence/{agent_id}", get(presence))
        .route(
            "/personnels/absences/edt-reference",
            get(edt_reference).put(definir_edt_reference),
        )
        .route(
            "/personnels/absences/impacts/{impact_id}/sans-objet",
            post(sans_objet),
        )
        .route("/personnels/absences/{id}/impact", get(impact))
        .route("/personnels/absences/{id}/prochaine-etape", get(prochaine_etape))
        .route("/personnels/absences/{id}/retour", get(retour))
}

#[derive(Debug, Deserialize)]
struct JourQuery {
    jour: Option<NaiveDate>,
}

async fn tableau(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Res<Json<TableauJourDto>> {
    let jour = q.jour.unwrap_or_else(|| chrono::Utc::now().date_naive());
    s.ent
        .tableau_jour_for_actor(session.etablissement_id, session.user_id, &session.role, jour)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn compteurs(State(s): State<AppState>, session: EntSession) -> Res<Json<CompteursTableauDeBordDto>> {
    s.ent
        .compteurs_tableau_de_bord_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn a_traiter(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<AbsenceDto>>> {
    s.ent
        .file_a_traiter_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn presence(
    State(s): State<AppState>,
    session: EntSession,
    Path(agent_id): Path<Uuid>,
) -> Res<Json<PresenceDto>> {
    s.ent
        .present_for_actor(session.etablissement_id, session.user_id, &session.role, agent_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn edt_reference(State(s): State<AppState>, session: EntSession) -> Res<Json<EdtReferenceDto>> {
    s.ent
        .edt_reference_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn definir_edt_reference(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<EdtReferenceBody>,
) -> Res<Json<EdtReferenceDto>> {
    s.ent
        .definir_edt_reference_for_actor(session.etablissement_id, session.user_id, &session.role, b.actif)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn sans_objet(
    State(s): State<AppState>,
    session: EntSession,
    Path(impact_id): Path<Uuid>,
) -> Res<Json<ImpactDto>> {
    s.ent
        .marquer_impact_sans_objet_for_actor(session.etablissement_id, session.user_id, &session.role, impact_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn impact(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<ImpactDto>>> {
    s.ent
        .impact_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn prochaine_etape(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ProchaineEtapeDto>> {
    s.ent
        .prochaine_etape_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn retour(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<ImpactDto>>> {
    s.ent
        .retour_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}
