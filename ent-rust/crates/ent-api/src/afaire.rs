//! Socle 29, lots P2 et P3 : l'à-faire unifié (`docs/socles/29_PORTAILS.md` §8, §21).
//!
//! L'autorité est `ent-db` (`PgEnt::a_faire_de`, cartes d'action) : la route traduit HTTP ↔ service.
//! * `GET /afaire` : l'à-faire de la personne connectée, revérifié à la lecture, jamais mis en cache.
//! * `POST /afaire/{id}/apercu` puis `POST /afaire/{id}/executer` : action en deux gestes, avec
//!   confirmation explicite ; jamais d'action à l'ouverture d'une adresse (aucune route `GET` n'écrit).
//! * `POST /afaire/rapprochement` : rapprochement à la demande (`afaire.rapprocher`).
//!
//! Toute démarche déjà traitée répond **le même code et le même texte**, quel que soit l'auteur
//! de la clôture (R-PO7, R-PO38) ; une tâche qui n'est pas à la personne est introuvable (404).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use ent_db::EntDbError;
use ent_types::afaire::{
    AFaire, ApercuAction, ExecutionAction, RapportRapprochement, ResultatAction,
    MESSAGE_DEJA_TRAITEE,
};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/afaire", get(lire))
        .route("/afaire/rapprochement", post(rapprocher))
        .route("/afaire/{id}/apercu", post(apercu))
        .route("/afaire/{id}/executer", post(executer))
}

/// Traduction d'erreur : « déjà traitée » est toujours 409 avec le même texte.
fn erreur(e: EntDbError) -> (StatusCode, String) {
    if matches!(&e, EntDbError::ConflitDossier(m) if *m == MESSAGE_DEJA_TRAITEE) {
        return (StatusCode::CONFLICT, MESSAGE_DEJA_TRAITEE.to_string());
    }
    map_db(e)
}

async fn lire(
    State(s): State<AppState>,
    session: EntSession,
) -> Result<Json<AFaire>, (StatusCode, String)> {
    s.ent
        .a_faire_de(
            session.etablissement_id,
            session.user_id,
            &session.role,
            Utc::now(),
        )
        .await
        .map(Json)
        .map_err(erreur)
}

async fn apercu(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<ApercuAction>, (StatusCode, String)> {
    s.ent
        .afaire_apercu(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            Utc::now(),
        )
        .await
        .map(Json)
        .map_err(erreur)
}

async fn executer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(corps): Json<ExecutionAction>,
) -> Result<Json<ResultatAction>, (StatusCode, String)> {
    s.ent
        .afaire_executer(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            corps.confirme,
            corps.motif,
            Utc::now(),
        )
        .await
        .map(Json)
        .map_err(erreur)
}

async fn rapprocher(
    State(s): State<AppState>,
    session: EntSession,
) -> Result<Json<RapportRapprochement>, (StatusCode, String)> {
    s.ent
        .rapprocher_afaire_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            Utc::now(),
        )
        .await
        .map(Json)
        .map_err(erreur)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R-PO7, R-PO38 : « déjà traitée » est toujours le même code et le même texte.
    #[test]
    fn deja_traitee_a_un_seul_code_et_un_seul_texte() {
        let (code, texte) = erreur(EntDbError::ConflitDossier(MESSAGE_DEJA_TRAITEE));
        assert_eq!(
            (code, texte.as_str()),
            (StatusCode::CONFLICT, "Cette démarche est déjà traitée.")
        );
    }
}
