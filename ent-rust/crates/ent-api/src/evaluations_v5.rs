//! Routes `/eva/v5/*` du lot V5 « parcours » (socle 21) : résultats externes, enseignements suivis hors de
//! l'établissement, parcours d'un élève dans une période.
//!
//! L'autorité est `ent-db` (`evaluations_v5.rs`) : droit personnel, droit sur le cours, élève visible (socle 2), période
//! de l'élève, attribution et journal. Ces routes ne font que traduire HTTP ↔ service. **Écritures et parcours : personnel
//! seulement** (la famille et l'élève reçoivent 403). La seule lecture ouverte à la famille et à l'élève est
//! `/eva/v5/eleves/{id}/externes-famille` : résultats externes **publiés**, comptés, non retirés, sans auteur ni motif,
//! sous le lien « voir les notes ». Le jour de référence est celui de l'établissement.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_types::evaluations_v5::{
    CompterExterneBody, DeclarerEnseignementExterneBody, DeclarerResultatExterneBody,
    EnseignementExterneDto, ParcoursPeriodeDto, PublierMoyenneExterneBody, ResultatExterneDto,
    ResultatExterneFamilleDto, RetirerExterneBody, RetirerMoyenneExterneBody,
    SaisirMoyenneExterneBody,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/eva/v5/externes", post(declarer_externe))
        .route("/eva/v5/externes/{id}/compter", post(compter_externe))
        .route("/eva/v5/externes/{id}/publier", post(publier_externe))
        .route("/eva/v5/externes/{id}/retirer", post(retirer_externe))
        .route("/eva/v5/eleves/{id}/externes", get(externes_eleve))
        .route(
            "/eva/v5/eleves/{id}/externes-famille",
            get(externes_famille),
        )
        .route(
            "/eva/v5/enseignements-externes",
            post(declarer_enseignement),
        )
        .route(
            "/eva/v5/enseignements-externes/{id}/moyennes",
            post(saisir_moyenne),
        )
        .route(
            "/eva/v5/moyennes-externes/{id}/publier",
            post(publier_moyenne),
        )
        .route(
            "/eva/v5/moyennes-externes/{id}/retirer-publication",
            post(retirer_publication_moyenne),
        )
        .route(
            "/eva/v5/eleves/{id}/enseignements-externes",
            get(enseignements_eleve),
        )
        .route("/eva/v5/eleves/{id}/parcours", get(parcours))
}

async fn jour(s: &AppState, session: &EntSession) -> Res<chrono::NaiveDate> {
    s.ent
        .aujourdhui_etablissement(session.etablissement_id)
        .await
        .map_err(map_db)
}

async fn declarer_externe(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DeclarerResultatExterneBody>,
) -> Res<(StatusCode, Json<ResultatExterneDto>)> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v5_declarer_externe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            &b,
        )
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn compter_externe(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CompterExterneBody>,
) -> Res<Json<ResultatExterneDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v5_compter_externe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn publier_externe(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ResultatExterneDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v5_publier_externe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn retirer_externe(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RetirerExterneBody>,
) -> Res<Json<ResultatExterneDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v5_retirer_externe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn externes_eleve(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<ResultatExterneDto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v5_externes_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn externes_famille(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<ResultatExterneFamilleDto>>> {
    s.ent
        .eva_v5_externes_famille_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn declarer_enseignement(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DeclarerEnseignementExterneBody>,
) -> Res<(StatusCode, Json<EnseignementExterneDto>)> {
    s.ent
        .eva_v5_declarer_enseignement_externe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &b,
        )
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn saisir_moyenne(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SaisirMoyenneExterneBody>,
) -> Res<Json<EnseignementExterneDto>> {
    s.ent
        .eva_v5_saisir_moyenne_externe_for_actor(
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

async fn publier_moyenne(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PublierMoyenneExterneBody>,
) -> Res<Json<EnseignementExterneDto>> {
    s.ent
        .eva_v5_publier_moyenne_externe_for_actor(
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

async fn retirer_publication_moyenne(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RetirerMoyenneExterneBody>,
) -> Res<Json<EnseignementExterneDto>> {
    s.ent
        .eva_v5_retirer_publication_moyenne_externe_for_actor(
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

async fn enseignements_eleve(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<EnseignementExterneDto>>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v5_enseignements_externes_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct ParcoursQuery {
    periode_id: Uuid,
}

async fn parcours(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<ParcoursQuery>,
) -> Res<Json<ParcoursPeriodeDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v5_parcours_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            id,
            q.periode_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}
