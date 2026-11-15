//! Socle 29, lot P5 : recherche globale (`docs/socles/29_PORTAILS.md` §12.1).
//!
//! L'autorité est `ent-db` (`PgEnt::rechercher_global`) : la route traduit HTTP ↔ service.
//! * `GET /recherche?q=` : sources fédérées, 10 résultats par source, 40 au total, côté serveur. Une recherche
//!   bloquée répond `429` au texte fixe (aucun chiffre) ; passé 75 % du plafond, la réponse est ralentie.
//! * `GET /recherche/blocages` (administrateur) ; `POST /recherche/{compte}/debloquer` (`recherche.debloquer`) ;
//!   `POST /recherche/debloquer-moi` (une fois par jour, motif de liste fermée).
//!
//! **Le texte de la recherche n'est jamais journalisé** : la route ne le trace pas, le service ne le stocke pas.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use ent_db::EntDbError;
use ent_types::recherche::{
    BlocageRecherche, CorpsDeblocageSoi, ReponseRecherche, MESSAGE_RECHERCHE_BLOQUEE,
};
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/recherche", get(rechercher))
        .route("/recherche/blocages", get(blocages))
        .route("/recherche/debloquer-moi", post(debloquer_moi))
        .route("/recherche/{compte}/debloquer", post(debloquer))
}

/// Ralentissement appliqué passé 75 % du plafond quotidien.
const RALENTISSEMENT: Duration = Duration::from_millis(800);

fn erreur(e: EntDbError) -> (StatusCode, String) {
    if matches!(&e, EntDbError::ConflitDossier(m) if *m == MESSAGE_RECHERCHE_BLOQUEE) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            MESSAGE_RECHERCHE_BLOQUEE.to_string(),
        );
    }
    map_db(e)
}

#[derive(Debug, Deserialize)]
struct Recherche {
    q: Option<String>,
}

async fn rechercher(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<Recherche>,
) -> Result<Json<ReponseRecherche>, (StatusCode, String)> {
    let r = s
        .ent
        .rechercher_global(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &q.q.unwrap_or_default(),
            Utc::now(),
        )
        .await
        .map_err(erreur)?;
    if r.ralenti {
        tokio::time::sleep(RALENTISSEMENT).await;
    }
    Ok(Json(r))
}

async fn blocages(
    State(s): State<AppState>,
    session: EntSession,
) -> Result<Json<Vec<BlocageRecherche>>, (StatusCode, String)> {
    s.ent
        .rechercher_blocages(
            session.etablissement_id,
            session.user_id,
            &session.role,
            Utc::now(),
        )
        .await
        .map(Json)
        .map_err(erreur)
}

async fn debloquer(
    State(s): State<AppState>,
    session: EntSession,
    Path(compte): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    s.ent
        .rechercher_debloquer(
            session.etablissement_id,
            session.user_id,
            &session.role,
            compte,
            Utc::now(),
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(erreur)
}

async fn debloquer_moi(
    State(s): State<AppState>,
    session: EntSession,
    Json(corps): Json<CorpsDeblocageSoi>,
) -> Result<StatusCode, (StatusCode, String)> {
    s.ent
        .rechercher_debloquer_soi(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &corps.motif,
            Utc::now(),
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(erreur)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une recherche bloquée a un seul code et un seul texte, sans chiffre ni seuil.
    #[test]
    fn bloquee_est_un_429_sans_chiffre() {
        let (code, texte) = erreur(EntDbError::ConflitDossier(MESSAGE_RECHERCHE_BLOQUEE));
        assert_eq!(code, StatusCode::TOO_MANY_REQUESTS);
        assert!(!texte.chars().any(|c| c.is_ascii_digit()), "{texte}");
    }
}
