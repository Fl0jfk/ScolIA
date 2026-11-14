//! Routes des rattrapages dus (socle 15, lot A8). Préfixe `/personnels/rattrapages`. L'autorité est `ent-db`
//! (`absences_personnels/rattrapage.rs`) : l'agent ne voit et ne propose que les siens ; la direction confirme, prolonge,
//! requalifie, abandonne ; direction et CPE attestent. Aucune route ne renvoie la nature d'une absence ni un texte libre.

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
        .route("/personnels/rattrapages", get(liste))
        .route("/personnels/rattrapages/moi", get(mes_rattrapages))
        .route(
            "/personnels/rattrapages/eleves-perdues",
            get(eleves_perdues),
        )
        .route("/personnels/rattrapages/{id}", get(un))
        .route("/personnels/rattrapages/{id}/creneaux", post(proposer))
        .route(
            "/personnels/rattrapages/{id}/creneaux/{creneau_id}/confirmation",
            post(confirmer),
        )
        .route(
            "/personnels/rattrapages/{id}/creneaux/{creneau_id}/attestation",
            post(attester),
        )
        .route("/personnels/rattrapages/{id}/prolongation", post(prolonger))
        .route("/personnels/rattrapages/{id}/retenue", post(retenue))
        .route("/personnels/rattrapages/{id}/abandon", post(abandon))
}

#[derive(Deserialize)]
struct ListeQuery {
    /// `true` : sans les rattrapages clos.
    #[serde(default)]
    ouverts: bool,
}

#[derive(Deserialize)]
struct FenetreQuery {
    du: NaiveDate,
    au: NaiveDate,
}

async fn mes_rattrapages(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<RattrapageDto>>> {
    s.ent
        .mes_rattrapages_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn liste(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<ListeQuery>,
) -> Res<Json<Vec<RattrapageDto>>> {
    s.ent
        .rattrapages_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.ouverts,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn eleves_perdues(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<FenetreQuery>,
) -> Res<Json<Vec<HeuresElevesPerduesDto>>> {
    s.ent
        .heures_eleves_perdues_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.du,
            q.au,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn un(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<RattrapageDto>> {
    s.ent
        .rattrapage_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn proposer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ProposerCreneauxBody>,
) -> Res<(StatusCode, Json<RattrapageDto>)> {
    s.ent
        .proposer_creneaux_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn confirmer(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, creneau_id)): Path<(Uuid, Uuid)>,
    Json(b): Json<ConfirmerCreneauBody>,
) -> Res<Json<RattrapageDto>> {
    s.ent
        .confirmer_creneau_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            creneau_id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn attester(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, creneau_id)): Path<(Uuid, Uuid)>,
) -> Res<Json<RattrapageDto>> {
    s.ent
        .attester_creneau_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            creneau_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn prolonger(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ProlongerRattrapageBody>,
) -> Res<Json<RattrapageDto>> {
    s.ent
        .prolonger_rattrapage_for_actor(
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

async fn retenue(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DecisionRattrapageBody>,
) -> Res<Json<RattrapageDto>> {
    s.ent
        .requalifier_en_retenue_for_actor(
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

async fn abandon(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DecisionRattrapageBody>,
) -> Res<Json<RattrapageDto>> {
    s.ent
        .abandonner_rattrapage_for_actor(
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
