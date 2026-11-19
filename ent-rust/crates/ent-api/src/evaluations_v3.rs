//! Routes `/eva/*` du lot V3 « grille et saisie » (socle 21).
//!
//! L'autorité est `ent-db` (`evaluations_v3*.rs`) : droits du cours (titulaire, co-enseignant, suppléant),
//! `notes.lire` / `notes.corriger`, période par élève, versions de cases, moteur de calcul unique. Ces routes
//! ne font que traduire HTTP ↔ service. Une référence d'un autre établissement répond 404 ; famille et élève
//! n'ont jamais la grille (403). Le jour et l'heure de référence sont ceux de l'établissement, posés ici.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::Utc;
use ent_types::evaluations_v3::{
    AnnulerEvaluationV3Body, CreerEvaluationV3Body, EnCoursCorrectionBody, EnseignementV3Dto,
    EvaluationColonneDto, ExplicationDto, GrilleV3Dto, ImportApercuDto, ImportBody,
    LigneHistoriqueV3Dto, ModifierEvaluationV3Body, PeriodeV3Dto, PublierV3Body, SaisieLotBody,
    SaisieLotRetourDto, SousMatiereV3Dto,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/eva/enseignements", get(enseignements))
        .route("/eva/cours/{id}/periodes", get(periodes))
        .route("/eva/cours/{id}/sous-matieres", get(sous_matieres))
        .route("/eva/grille", get(grille))
        .route("/eva/grille/explication", get(explication))
        .route("/eva/saisie", post(saisir))
        .route("/eva/evaluations", post(creer))
        .route("/eva/evaluations/{id}", put(modifier))
        .route("/eva/evaluations/{id}/annuler", post(annuler))
        .route("/eva/evaluations/{id}/publication", post(publication))
        .route(
            "/eva/evaluations/{id}/en-cours-de-correction",
            put(en_cours),
        )
        .route(
            "/eva/evaluations/{id}/eleves/{eleve_id}/historique",
            get(historique),
        )
        .route("/eva/import/apercu", post(apercu_import))
        .route("/eva/import/confirmer", post(confirmer_import))
}

#[derive(Deserialize)]
struct GrilleQuery {
    cours_id: Uuid,
    periode_id: Uuid,
}

#[derive(Deserialize)]
struct ExplicationQuery {
    cours_id: Uuid,
    periode_id: Uuid,
    eleve_id: Uuid,
}

async fn jour(s: &AppState, session: &EntSession) -> Res<chrono::NaiveDate> {
    s.ent
        .aujourdhui_etablissement(session.etablissement_id)
        .await
        .map_err(map_db)
}

async fn enseignements(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<EnseignementV3Dto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_enseignements_for_actor(session.etablissement_id, session.user_id, &session.role, j)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn periodes(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<PeriodeV3Dto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_periodes_for_actor(
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

async fn sous_matieres(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<SousMatiereV3Dto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_sous_matieres_for_actor(
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

async fn grille(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<GrilleQuery>,
) -> Res<Json<GrilleV3Dto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_grille_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.cours_id,
            q.periode_id,
            j,
            Utc::now(),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn explication(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<ExplicationQuery>,
) -> Res<Json<ExplicationDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_explication_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.cours_id,
            q.periode_id,
            q.eleve_id,
            j,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn saisir(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<SaisieLotBody>,
) -> Res<Json<SaisieLotRetourDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_saisir_lot_for_actor(
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

async fn creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerEvaluationV3Body>,
) -> Res<(StatusCode, Json<EvaluationColonneDto>)> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_creer_evaluation_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            Utc::now(),
            &b,
        )
        .await
        .map(|e| (StatusCode::CREATED, Json(e)))
        .map_err(map_db)
}

async fn modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ModifierEvaluationV3Body>,
) -> Res<Json<EvaluationColonneDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_modifier_evaluation_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            Utc::now(),
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn annuler(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AnnulerEvaluationV3Body>,
) -> Res<Json<EvaluationColonneDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_annuler_evaluation_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            Utc::now(),
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn publication(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PublierV3Body>,
) -> Res<Json<EvaluationColonneDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_publier_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            Utc::now(),
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn en_cours(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<EnCoursCorrectionBody>,
) -> Res<Json<EvaluationColonneDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_en_cours_correction_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            Utc::now(),
            id,
            b.en_cours,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn historique(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, eleve_id)): Path<(Uuid, Uuid)>,
) -> Res<Json<Vec<LigneHistoriqueV3Dto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_historique_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            eleve_id,
            j,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn apercu_import(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ImportBody>,
) -> Res<Json<ImportApercuDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_apercu_import_for_actor(
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

async fn confirmer_import(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ImportBody>,
) -> Res<Json<SaisieLotRetourDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v3_confirmer_import_for_actor(
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
