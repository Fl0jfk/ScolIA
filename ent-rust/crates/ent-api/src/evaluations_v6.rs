//! Routes `/eva/v6/*` du lot V6 « appréciations » (socle 21) : saisie (cours, générale, vie scolaire, intermédiaire,
//! dispositif), saisie en série d'un cours avec les moyennes à côté, contrôles de cohérence, historique des versions,
//! relecture et remarques aux auteurs, banque de phrases personnelle, mots interdits de l'établissement.
//!
//! L'autorité est `ent-db` (`evaluations_v6.rs`) : droit sur le cours ou sur la classe, élève visible (socle 2), fenêtre
//! selon l'état de la période de la classe de l'élève, versions et journal. **Toutes ces routes sont réservées au
//! personnel** : la famille et l'élève reçoivent 403 (ils lisent les appréciations publiées par les lectures existantes,
//! vue `ent_note_appreciation_v`). Le jour de référence est celui de l'établissement.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use ent_types::evaluations_v6::{
    AjouterMotInterditBody, AjouterPhraseBody, AppreciationsEleveDto, JeterBrouillonBody,
    MotInterditDto, PhraseBanqueDto, RemarqueBody, RemarqueDto, SaisieAppreciationDto,
    SaisirAppreciationBody, SerieCoursDto, VersionAppreciationDto,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/eva/v6/appreciations", post(saisir))
        .route("/eva/v6/appreciations/{id}/jeter", post(jeter))
        .route("/eva/v6/appreciations/{id}/historique", get(historique))
        .route("/eva/v6/appreciations/{id}/remarques", post(remarquer))
        .route("/eva/v6/cours/{id}/serie", get(serie))
        .route("/eva/v6/eleves/{id}/appreciations", get(appreciations_eleve))
        .route("/eva/v6/remarques", get(mes_remarques))
        .route("/eva/v6/remarques/{id}/traiter", post(traiter_remarque))
        .route("/eva/v6/banque", get(banque).post(ajouter_phrase))
        .route("/eva/v6/banque/{id}/retirer", post(retirer_phrase))
        .route(
            "/eva/v6/mots-interdits",
            get(mots_interdits).post(ajouter_mot),
        )
        .route("/eva/v6/mots-interdits/{id}/retirer", post(retirer_mot))
}

#[derive(Debug, Deserialize)]
struct PeriodeQuery {
    periode_id: Uuid,
}

async fn jour(s: &AppState, session: &EntSession) -> Res<chrono::NaiveDate> {
    s.ent
        .aujourdhui_etablissement(session.etablissement_id)
        .await
        .map_err(map_db)
}

async fn saisir(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<SaisirAppreciationBody>,
) -> Res<Json<SaisieAppreciationDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v6_saisir_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn jeter(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<JeterBrouillonBody>,
) -> Res<StatusCode> {
    s.ent
        .eva_v6_jeter_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn historique(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<VersionAppreciationDto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v6_historique_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            j,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn remarquer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RemarqueBody>,
) -> Res<(StatusCode, Json<RemarqueDto>)> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v6_remarquer_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            j,
            &b,
        )
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn serie(
    State(s): State<AppState>,
    session: EntSession,
    Path(cours): Path<Uuid>,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<SerieCoursDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v6_serie_cours_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            cours,
            q.periode_id,
            j,
            Utc::now(),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn appreciations_eleve(
    State(s): State<AppState>,
    session: EntSession,
    Path(eleve): Path<Uuid>,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<AppreciationsEleveDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v6_appreciations_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            eleve,
            q.periode_id,
            j,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn mes_remarques(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<RemarqueDto>>> {
    s.ent
        .eva_v6_mes_remarques_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn traiter_remarque(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .eva_v6_traiter_remarque_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn banque(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<PhraseBanqueDto>>> {
    s.ent
        .eva_v6_banque_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn ajouter_phrase(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<AjouterPhraseBody>,
) -> Res<Json<PhraseBanqueDto>> {
    s.ent
        .eva_v6_ajouter_phrase_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn retirer_phrase(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .eva_v6_retirer_phrase_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn mots_interdits(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<MotInterditDto>>> {
    s.ent
        .eva_v6_mots_interdits_for_actor(session.etablissement_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn ajouter_mot(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<AjouterMotInterditBody>,
) -> Res<Json<MotInterditDto>> {
    s.ent
        .eva_v6_ajouter_mot_interdit_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn retirer_mot(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .eva_v6_retirer_mot_interdit_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}
