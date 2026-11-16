//! Routes du tableau de bord de la vie scolaire en panneaux (socle 12, lot AS14b-bis).
//!
//! Le tableau **réutilise** la file du matin (`/assiduite/file-du-matin`, AS6) et les actions des écrans AS14b ; ces
//! routes n'ajoutent que : les exclus de cours du jour, l'heure de bascule (réglage d'établissement) et la
//! disposition personnelle de l'utilisateur. L'autorité est `ent-db` : **aucun contrôle de rôle codé en dur ici**.
//! La disposition est toujours celle de l'utilisateur de la session (jamais un identifiant d'utilisateur en entrée).

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use chrono::{NaiveTime, Utc};
use ent_db::{ConfigTableau, ExclusDuJour, PreferencePanneau};
use serde::{Deserialize, Serialize};

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/assiduite/exclus-du-jour", get(exclus_du_jour))
        .route("/assiduite/tableau/configuration", get(configuration))
        .route("/assiduite/tableau/heure-bascule", put(regler_heure))
        .route("/assiduite/tableau/preference", put(poser_preference).delete(effacer_preference))
}

#[derive(Deserialize, Serialize)]
struct HeureBascule {
    heure: NaiveTime,
}

#[derive(Deserialize)]
struct Preference {
    panneaux: Vec<PreferencePanneau>,
}

async fn exclus_du_jour(State(state): State<AppState>, session: EntSession) -> Res<Json<ExclusDuJour>> {
    state
        .ent
        .exclus_du_jour_for_actor(session.etablissement_id, session.user_id, &session.role, Utc::now())
        .await
        .map(Json)
        .map_err(map_db)
}

async fn configuration(State(state): State<AppState>, session: EntSession) -> Res<Json<ConfigTableau>> {
    state
        .ent
        .config_tableau_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn regler_heure(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<HeureBascule>,
) -> Res<Json<HeureBascule>> {
    state
        .ent
        .regler_heure_bascule_for_actor(session.etablissement_id, session.user_id, &session.role, b.heure)
        .await
        .map(|heure| Json(HeureBascule { heure }))
        .map_err(map_db)
}

async fn poser_preference(
    State(state): State<AppState>,
    session: EntSession,
    Json(p): Json<Preference>,
) -> Res<Json<Vec<PreferencePanneau>>> {
    state
        .ent
        .poser_preference_tableau_for_actor(session.etablissement_id, session.user_id, &session.role, p.panneaux)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn effacer_preference(State(state): State<AppState>, session: EntSession) -> Res<StatusCode> {
    state
        .ent
        .effacer_preference_tableau_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}
