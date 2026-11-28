//! Routes `/eva/v8/*` du lot V8 « fin de période » (socle 21) : aperçu et arrêt des notes (publication de tout,
//! instantané), date d'arrêt propre à une classe, clôture des appréciations, réouverture motivée, clôture de la période
//! (contrôle de 3ᵉ), instantanés versionnés et leurs écarts, rattrapages autorisés, demandes de correction et décision
//! avec l'effet sur les moyennes, correction directe, journal des corrections, état de saisie et relances.
//!
//! L'autorité est `ent-db` (`evaluations_v8.rs`). **Toutes ces routes sont réservées au personnel** : la famille et
//! l'élève reçoivent 403. Le jour de référence est celui de l'établissement.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use ent_types::evaluations_v8::{
    ApercuArretDto, ArreterBody, AutoriserRattrapageBody, ClassesBody, Controle3eDto, CorrectionFaiteDto,
    CorrigerBody, DateArretBody, DeciderCorrectionBody, DemandeCorrectionDto, DemanderCorrectionBody,
    EffetCorrectionDto, EtatSaisieDto, GesteClasseDto, InstantaneDto, InstantanesDto, LigneJournalCorrectionDto,
    RelanceDto, RelancerBody, ReouvertureDto, ReprendreInstantaneBody, RepriseInstantaneDto, RouvrirBody,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/eva/v8/periodes/{p}/classes/{c}/apercu-arret", get(apercu_arret))
        .route("/eva/v8/periodes/{p}/arreter", post(arreter))
        .route("/eva/v8/periodes/{p}/date-arret", post(date_arret))
        .route("/eva/v8/periodes/{p}/clore-appreciations", post(clore_appreciations))
        .route("/eva/v8/periodes/{p}/rouvrir", post(rouvrir))
        .route("/eva/v8/periodes/{p}/classes/{c}/reouvertures", get(reouvertures))
        .route("/eva/v8/periodes/{p}/classes/{c}/controle-3e", get(controle_3e))
        .route("/eva/v8/periodes/{p}/classes/{c}/clore", post(clore_periode))
        .route("/eva/v8/periodes/{p}/classes/{c}/instantanes", get(instantanes).post(reprendre_instantane))
        .route("/eva/v8/periodes/{p}/classes/{c}/instantanes/{v}", get(instantane))
        .route("/eva/v8/periodes/{p}/classes/{c}/corrections", get(journal_corrections))
        .route("/eva/v8/periodes/{p}/classes/{c}/etat-saisie", get(etat_saisie))
        .route("/eva/v8/periodes/{p}/classes/{c}/relances", get(relances).post(relancer))
        .route("/eva/v8/rattrapages", post(autoriser_rattrapage))
        .route("/eva/v8/corrections", post(corriger))
        .route("/eva/v8/corrections/demandes", get(demandes).post(demander))
        .route("/eva/v8/corrections/demandes/{id}/effet", get(effet))
        .route("/eva/v8/corrections/demandes/{id}/decider", post(decider))
        .route("/eva/v8/corrections/demandes/{id}/retirer", post(retirer))
}

async fn jour(s: &AppState, session: &EntSession) -> Res<chrono::NaiveDate> {
    s.ent.aujourdhui_etablissement(session.etablissement_id).await.map_err(map_db)
}

async fn apercu_arret(
    State(s): State<AppState>,
    session: EntSession,
    Path((p, c)): Path<(Uuid, Uuid)>,
) -> Res<Json<ApercuArretDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_apercu_arret_for_actor(session.etablissement_id, session.user_id, &session.role, j, p, c)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn arreter(
    State(s): State<AppState>,
    session: EntSession,
    Path(p): Path<Uuid>,
    Json(b): Json<ArreterBody>,
) -> Res<Json<Vec<GesteClasseDto>>> {
    s.ent
        .eva_v8_arreter_for_actor(session.etablissement_id, session.user_id, &session.role, Utc::now(), p, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn date_arret(
    State(s): State<AppState>,
    session: EntSession,
    Path(p): Path<Uuid>,
    Json(b): Json<DateArretBody>,
) -> Res<StatusCode> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_date_arret_for_actor(session.etablissement_id, session.user_id, &session.role, j, p, &b)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn clore_appreciations(
    State(s): State<AppState>,
    session: EntSession,
    Path(p): Path<Uuid>,
    Json(b): Json<ClassesBody>,
) -> Res<Json<Vec<GesteClasseDto>>> {
    s.ent
        .eva_v8_clore_appreciations_for_actor(session.etablissement_id, session.user_id, &session.role, p, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn rouvrir(
    State(s): State<AppState>,
    session: EntSession,
    Path(p): Path<Uuid>,
    Json(b): Json<RouvrirBody>,
) -> Res<Json<Vec<ReouvertureDto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_rouvrir_for_actor(session.etablissement_id, session.user_id, &session.role, j, p, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn reouvertures(
    State(s): State<AppState>,
    session: EntSession,
    Path((p, c)): Path<(Uuid, Uuid)>,
) -> Res<Json<Vec<ReouvertureDto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_reouvertures_for_actor(session.etablissement_id, session.user_id, &session.role, j, p, c)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn controle_3e(
    State(s): State<AppState>,
    session: EntSession,
    Path((p, c)): Path<(Uuid, Uuid)>,
) -> Res<Json<Controle3eDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_controle_3e_for_actor(session.etablissement_id, session.user_id, &session.role, j, p, c)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn clore_periode(
    State(s): State<AppState>,
    session: EntSession,
    Path((p, c)): Path<(Uuid, Uuid)>,
) -> Res<Json<GesteClasseDto>> {
    s.ent
        .eva_v8_clore_periode_for_actor(session.etablissement_id, session.user_id, &session.role, p, c)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn instantanes(
    State(s): State<AppState>,
    session: EntSession,
    Path((p, c)): Path<(Uuid, Uuid)>,
) -> Res<Json<InstantanesDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_instantanes_for_actor(session.etablissement_id, session.user_id, &session.role, j, p, c)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn reprendre_instantane(
    State(s): State<AppState>,
    session: EntSession,
    Path((p, c)): Path<(Uuid, Uuid)>,
    Json(b): Json<ReprendreInstantaneBody>,
) -> Res<Json<RepriseInstantaneDto>> {
    s.ent
        .eva_v8_reprendre_instantane_for_actor(session.etablissement_id, session.user_id, &session.role, p, c, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct ComparerQuery {
    #[serde(default)]
    comparer_a: Option<i32>,
}

async fn instantane(
    State(s): State<AppState>,
    session: EntSession,
    Path((p, c, v)): Path<(Uuid, Uuid, i32)>,
    Query(q): Query<ComparerQuery>,
) -> Res<Json<InstantaneDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_instantane_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            p,
            c,
            v,
            q.comparer_a,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn journal_corrections(
    State(s): State<AppState>,
    session: EntSession,
    Path((p, c)): Path<(Uuid, Uuid)>,
) -> Res<Json<Vec<LigneJournalCorrectionDto>>> {
    s.ent
        .eva_v8_journal_corrections_for_actor(session.etablissement_id, session.user_id, &session.role, p, c)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn etat_saisie(
    State(s): State<AppState>,
    session: EntSession,
    Path((p, c)): Path<(Uuid, Uuid)>,
) -> Res<Json<EtatSaisieDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_etat_saisie_for_actor(session.etablissement_id, session.user_id, &session.role, j, p, c)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn relances(
    State(s): State<AppState>,
    session: EntSession,
    Path((p, c)): Path<(Uuid, Uuid)>,
) -> Res<Json<Vec<RelanceDto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_relances_for_actor(session.etablissement_id, session.user_id, &session.role, j, p, c)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn relancer(
    State(s): State<AppState>,
    session: EntSession,
    Path((p, c)): Path<(Uuid, Uuid)>,
    Json(b): Json<RelancerBody>,
) -> Res<(StatusCode, Json<Vec<RelanceDto>>)> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_relancer_for_actor(session.etablissement_id, session.user_id, &session.role, j, p, c, &b)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn autoriser_rattrapage(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<AutoriserRattrapageBody>,
) -> Res<StatusCode> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_autoriser_rattrapage_for_actor(session.etablissement_id, session.user_id, &session.role, j, &b)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn corriger(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CorrigerBody>,
) -> Res<Json<CorrectionFaiteDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_corriger_for_actor(session.etablissement_id, session.user_id, &session.role, j, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct DemandesQuery {
    #[serde(default)]
    etat: Option<String>,
}

async fn demandes(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<DemandesQuery>,
) -> Res<Json<Vec<DemandeCorrectionDto>>> {
    let toutes = q.etat.as_deref() == Some("toutes");
    s.ent
        .eva_v8_demandes_for_actor(session.etablissement_id, session.user_id, &session.role, toutes)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn demander(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DemanderCorrectionBody>,
) -> Res<(StatusCode, Json<DemandeCorrectionDto>)> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_demander_correction_for_actor(session.etablissement_id, session.user_id, &session.role, j, &b)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn effet(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<EffetCorrectionDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_effet_demande_for_actor(session.etablissement_id, session.user_id, &session.role, j, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn decider(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DeciderCorrectionBody>,
) -> Res<Json<DemandeCorrectionDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v8_decider_for_actor(session.etablissement_id, session.user_id, &session.role, j, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn retirer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .eva_v8_retirer_demande_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}
