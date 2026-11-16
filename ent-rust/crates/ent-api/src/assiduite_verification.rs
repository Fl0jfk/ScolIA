//! Routes de la vérification et de la file du matin (socle 12, lot AS6) : file classée, appel à la place de
//! l'intervenant, « prévenir maintenant », alertes (« je prends », traitement), drapeau de vigilance.
//!
//! L'autorité est `ent-db` (`appel.verifier` et `absences.lire` pour la file, `appel.faire_tout`,
//! `assiduite.alerter`, `assiduite.suivre`, `absences.qualifier`). **Aucun contrôle de rôle codé en dur ici** :
//! une référence inexistante, d'un autre établissement ou sans droit donne la même réponse (R-AS35). Aucune route
//! n'envoie quoi que ce soit : « prévenir maintenant » émet un avis du moteur de notifications du socle 8 (U3).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{NaiveDate, Utc};
use ent_db::{AlerteAsd, FileDuMatin, PrevenirMaintenant, ResultatALaPlace, ResultatPrevenir, VigilanceAsd};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/assiduite/file-du-matin", get(file_du_matin))
        .route("/assiduite/appels/a-la-place", post(appel_a_la_place))
        .route("/assiduite/eleves/{eleve_id}/prevenir-maintenant", post(prevenir_maintenant))
        .route("/assiduite/alertes", get(alertes))
        .route("/assiduite/alertes/{id}/prendre", post(prendre))
        .route("/assiduite/alertes/{id}/traiter", post(traiter))
        .route("/assiduite/vigilances", get(vigilances))
        .route("/assiduite/eleves/{eleve_id}/vigilance", post(poser_vigilance))
        .route("/assiduite/eleves/{eleve_id}/vigilance/lever", post(lever_vigilance))
}

#[derive(Deserialize)]
struct FiltreJour {
    jour: Option<NaiveDate>,
}

#[derive(Deserialize)]
struct ALaPlace {
    creneau_id: Uuid,
    jour: NaiveDate,
    raison_code: String,
}

#[derive(Deserialize)]
struct Raison {
    raison_code: String,
}

#[derive(Deserialize)]
struct Cloture {
    code: String,
}

async fn file_du_matin(
    State(state): State<AppState>,
    session: EntSession,
    Query(f): Query<FiltreJour>,
) -> Res<Json<FileDuMatin>> {
    state
        .ent
        .file_du_matin_for_actor(session.etablissement_id, session.user_id, &session.role, f.jour, Utc::now())
        .await
        .map(Json)
        .map_err(map_db)
}

async fn appel_a_la_place(
    State(state): State<AppState>,
    session: EntSession,
    Json(c): Json<ALaPlace>,
) -> Res<(StatusCode, Json<ResultatALaPlace>)> {
    state
        .ent
        .appel_a_la_place_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            c.creneau_id,
            c.jour,
            &c.raison_code,
        )
        .await
        .map(|r| (StatusCode::OK, Json(r)))
        .map_err(map_db)
}

async fn prevenir_maintenant(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
    Json(r): Json<Raison>,
) -> Res<Json<ResultatPrevenir>> {
    state
        .ent
        .prevenir_maintenant_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            PrevenirMaintenant {
                eleve_id,
                raison_code: r.raison_code,
            },
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn alertes(
    State(state): State<AppState>,
    session: EntSession,
    Query(f): Query<FiltreJour>,
) -> Res<Json<Vec<AlerteAsd>>> {
    let jour = match f.jour {
        Some(j) => j,
        None => state
            .ent
            .aujourdhui_etablissement(session.etablissement_id)
            .await
            .map_err(map_db)?,
    };
    state
        .ent
        .alertes_for_actor(session.etablissement_id, session.user_id, &session.role, jour)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn prendre(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<AlerteAsd>> {
    state
        .ent
        .alerte_prendre_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn traiter(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(c): Json<Cloture>,
) -> Res<Json<AlerteAsd>> {
    state
        .ent
        .alerte_traiter_for_actor(session.etablissement_id, session.user_id, &session.role, id, &c.code)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn vigilances(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<VigilanceAsd>>> {
    state
        .ent
        .vigilances_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn poser_vigilance(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .vigilance_poser_for_actor(session.etablissement_id, session.user_id, &session.role, eleve_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn lever_vigilance(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .vigilance_lever_for_actor(session.etablissement_id, session.user_id, &session.role, eleve_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}
