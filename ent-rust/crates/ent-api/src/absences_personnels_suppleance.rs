//! Routes de l'absence longue et de la suppléance (socle 15, lot A5). Préfixe `/personnels/absences`.
//! L'autorité est `ent-db` (`absences_personnels/suppleance.rs`) : action `personnels.absences.suppleer` (+ lecture
//! d'organisation), nomination réservée à la direction (`valider`) et au droit `services.gerer` du socle 6.
//! Niveau « organisation » : aucune route ne renvoie la nature ni le motif.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_types::absences_personnels::*;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/personnels/absences/{id}/suppleance",
            get(demandes).post(demander),
        )
        .route("/personnels/absences/{id}/suppleant", post(nommer))
        .route("/personnels/absences/{id}/remplacement", get(etat_remplacement))
        .route("/personnels/absences/demandes-suppleance/{id}/reponse", post(repondre))
}

async fn demandes(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<DemandeSuppleanceDto>>> {
    s.ent
        .demandes_suppleance_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn demander(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DemanderSuppleanceBody>,
) -> Res<(StatusCode, Json<DemandeSuppleanceDto>)> {
    s.ent
        .demander_suppleance_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn repondre(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ReponseSuppleanceBody>,
) -> Res<Json<DemandeSuppleanceDto>> {
    s.ent
        .repondre_demande_suppleance_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn nommer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<NommerSuppleantAbsenceBody>,
) -> Res<(StatusCode, Json<NominationSuppleantDto>)> {
    s.ent
        .nommer_suppleant_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn etat_remplacement(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Option<EtatRemplacementDto>>> {
    s.ent
        .etat_remplacement_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}
