//! Socle 29, lots P2 et P3 : l'à-faire unifié (`docs/socles/29_PORTAILS.md` §8, §21).
//!
//! L'autorité est `ent-db` (`PgEnt::a_faire_de`, cartes d'action) : la route traduit HTTP ↔ service.
//! * `GET /afaire` : l'à-faire de la personne connectée, revérifié à la lecture, jamais mis en cache.
//! * `POST /afaire/{id}/apercu` puis `POST /afaire/{id}/executer` : action en deux gestes, avec
//!   confirmation explicite ; jamais d'action à l'ouverture d'une adresse (aucune route `GET` n'écrit).
//! * `POST /afaire/rapprochement` : rapprochement à la demande (`afaire.rapprocher`).
//!
//! Lot P4 (files des personnels, §18.3) :
//! * `GET /afaire/files` (« Mes files », des nombres), `GET /afaire/files/{code}` (le contenu d'une file,
//!   avec qui a pris quoi) ;
//! * `POST /afaire/{id}/prendre`, `/relacher`, `/preparee`, `/reattribuer` : prises en charge (la
//!   première gagne, la seconde reçoit 409 « déjà prise ») ;
//! * `GET /afaire/supervision` (nombres, jamais un nom), `GET /afaire/orphelines`,
//!   `POST /afaire/orphelines/{id}/sans-objet`.
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
    AFaire, ApercuAction, CorpsPrise, CorpsReattribution, ExecutionAction, FileAfaire, MesFiles,
    NiveauPrise, Orpheline, RapportRapprochement, ResultatAction, Supervision,
    MESSAGE_DEJA_PRISE, MESSAGE_DEJA_TRAITEE,
};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/afaire", get(lire))
        .route("/afaire/rapprochement", post(rapprocher))
        .route("/afaire/files", get(mes_files))
        .route("/afaire/files/{code}", get(file))
        .route("/afaire/supervision", get(supervision))
        .route("/afaire/orphelines", get(orphelines))
        .route("/afaire/orphelines/{id}/sans-objet", post(clore_orpheline))
        .route("/afaire/{id}/prendre", post(prendre))
        .route("/afaire/{id}/relacher", post(relacher))
        .route("/afaire/{id}/preparee", post(preparee))
        .route("/afaire/{id}/reattribuer", post(reattribuer))
        .route("/afaire/{id}/apercu", post(apercu))
        .route("/afaire/{id}/executer", post(executer))
}

/// Traduction d'erreur : « déjà traitée » est toujours 409 avec le même texte.
fn erreur(e: EntDbError) -> (StatusCode, String) {
    if matches!(&e, EntDbError::ConflitDossier(m) if *m == MESSAGE_DEJA_TRAITEE) {
        return (StatusCode::CONFLICT, MESSAGE_DEJA_TRAITEE.to_string());
    }
    if matches!(&e, EntDbError::ConflitDossier(m) if *m == MESSAGE_DEJA_PRISE) {
        return (StatusCode::CONFLICT, MESSAGE_DEJA_PRISE.to_string());
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
        .afaire_executer_avec(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            corps.confirme,
            corps.motif,
            corps.decision,
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

async fn mes_files(
    State(s): State<AppState>,
    session: EntSession,
) -> Result<Json<MesFiles>, (StatusCode, String)> {
    s.ent
        .mes_files(session.etablissement_id, session.user_id, &session.role, Utc::now())
        .await
        .map(Json)
        .map_err(erreur)
}

async fn file(
    State(s): State<AppState>,
    session: EntSession,
    Path(code): Path<String>,
) -> Result<Json<FileAfaire>, (StatusCode, String)> {
    s.ent
        .file_afaire(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &code,
            Utc::now(),
        )
        .await
        .map(Json)
        .map_err(erreur)
}

async fn prendre(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    corps: Option<Json<CorpsPrise>>,
) -> Result<StatusCode, (StatusCode, String)> {
    let niveau = corps
        .and_then(|Json(c)| c.niveau)
        .unwrap_or(NiveauPrise::Decider);
    s.ent
        .afaire_prendre(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            niveau,
            Utc::now(),
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(erreur)
}

async fn relacher(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    s.ent
        .afaire_relacher(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            Utc::now(),
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(erreur)
}

async fn preparee(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    s.ent
        .afaire_preparee(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            Utc::now(),
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(erreur)
}

async fn reattribuer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(corps): Json<CorpsReattribution>,
) -> Result<StatusCode, (StatusCode, String)> {
    s.ent
        .afaire_reattribuer(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            corps.vers_compte_id,
            corps.niveau.unwrap_or(NiveauPrise::Decider),
            Utc::now(),
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(erreur)
}

async fn supervision(
    State(s): State<AppState>,
    session: EntSession,
) -> Result<Json<Supervision>, (StatusCode, String)> {
    s.ent
        .afaire_supervision(
            session.etablissement_id,
            session.user_id,
            &session.role,
            Utc::now(),
        )
        .await
        .map(Json)
        .map_err(erreur)
}

async fn orphelines(
    State(s): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<Orpheline>>, (StatusCode, String)> {
    s.ent
        .afaire_orphelines(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(erreur)
}

async fn clore_orpheline(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    s.ent
        .afaire_clore_orpheline(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            Utc::now(),
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(erreur)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CPO-36 : « déjà prise » est un 409 au texte fixe, jamais le nom du collègue.
    #[test]
    fn deja_prise_est_un_409_sans_nom() {
        let (code, texte) = erreur(EntDbError::ConflitDossier(MESSAGE_DEJA_PRISE));
        assert_eq!(code, StatusCode::CONFLICT);
        assert_eq!(texte, "Cette tâche est déjà prise en charge par un collègue.");
    }

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
