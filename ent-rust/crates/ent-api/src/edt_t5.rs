//! Routes du lot T5 du socle 10 : publication des versions d'emploi du temps (EDT-150 à EDT-156).
//!
//! L'autorité est `ent-db` (`edt.publication_lire`, `edt.publier`, `edt.revenir_version`, `edt.forcer_conflit`) : ces routes ne
//! font que traduire HTTP ↔ service. L'établissement vient de la session ; une version d'un autre établissement répond 404.
//! Des contrôles bloquants répondent **409** avec la liste (codes, nombres, identifiants : jamais un nom de personne).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::{DateTime, NaiveDate, Utc};
use ent_db::edt_publication::{
    ApercuPublication, CmdIndividuelle, CmdPublier, DifferencesVersions, RapportControles,
    ResultatIndividuelle, ResultatPublication, VersionHistorique,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/edt/annees/{id}/publications", get(historique))
        .route("/edt/versions/difference", get(difference))
        .route("/edt/versions/{id}/controles", get(controler))
        .route("/edt/versions/{id}/apercu", get(apercu))
        .route("/edt/versions/{id}/publier", post(publier))
        .route("/edt/versions/{id}/diffuser", post(diffuser))
        .route("/edt/versions/{id}/diffusion", post(regler_diffusion))
        .route("/edt/versions/{id}/annonce", delete(retirer_annonce))
        .route("/edt/versions/{id}/retour-arriere", post(retour_arriere))
        .route("/edt/publications/diffuser-echues", post(diffuser_echues))
        .route("/edt/individuelles", post(individuelle))
}

#[derive(Debug, Deserialize)]
struct FenetreQuery {
    #[serde(default)]
    a_partir_du: Option<NaiveDate>,
    #[serde(default)]
    au: Option<NaiveDate>,
}

async fn controler(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<FenetreQuery>,
) -> Res<Json<RapportControles>> {
    Ok(Json(
        s.ent
            .edt_publication_controler(&acteur(&session), id, q.a_partir_du, q.au)
            .await
            .map_err(map_db)?,
    ))
}

async fn apercu(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<FenetreQuery>,
) -> Res<Json<ApercuPublication>> {
    Ok(Json(
        s.ent
            .edt_publication_apercu(&acteur(&session), id, q.a_partir_du, q.au)
            .await
            .map_err(map_db)?,
    ))
}

/// 201 : publiée (ou annoncée) ; 200 : déjà faite ; 409 : contrôles bloquants (liste dans le corps).
fn reponse(r: ResultatPublication) -> (StatusCode, Json<ResultatPublication>) {
    let code = if !r.bloquants.is_empty() {
        StatusCode::CONFLICT
    } else if r.deja_fait {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    (code, Json(r))
}

async fn publier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(mut cmd): Json<CmdPublier>,
) -> Res<(StatusCode, Json<ResultatPublication>)> {
    cmd.version_id = id;
    Ok(reponse(
        s.ent
            .edt_publier(&acteur(&session), &cmd)
            .await
            .map_err(map_db)?,
    ))
}

async fn diffuser(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<(StatusCode, Json<ResultatPublication>)> {
    Ok(reponse(
        s.ent
            .edt_diffuser(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct DiffusionBody {
    diffusion_le: DateTime<Utc>,
}

async fn regler_diffusion(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DiffusionBody>,
) -> Res<(StatusCode, Json<ResultatPublication>)> {
    Ok(reponse(
        s.ent
            .edt_diffusion_regler(&acteur(&session), id, b.diffusion_le)
            .await
            .map_err(map_db)?,
    ))
}

async fn retirer_annonce(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .edt_annonce_retirer(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Default, Deserialize)]
struct RetourBody {
    #[serde(default)]
    a_partir_du: Option<NaiveDate>,
}

async fn retour_arriere(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<RetourBody>>,
) -> Res<(StatusCode, Json<ResultatPublication>)> {
    let b = body.map(|j| j.0).unwrap_or_default();
    Ok(reponse(
        s.ent
            .edt_retour_arriere(&acteur(&session), id, b.a_partir_du)
            .await
            .map_err(map_db)?,
    ))
}

async fn diffuser_echues(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<Uuid>>> {
    Ok(Json(
        s.ent
            .edt_diffuser_echues(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}

async fn historique(
    State(s): State<AppState>,
    session: EntSession,
    Path(annee): Path<Uuid>,
) -> Res<Json<Vec<VersionHistorique>>> {
    Ok(Json(
        s.ent
            .edt_publications_historique(&acteur(&session), annee)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct DifferenceQuery {
    de: Uuid,
    vers: Uuid,
}

async fn difference(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<DifferenceQuery>,
) -> Res<Json<DifferencesVersions>> {
    Ok(Json(
        s.ent
            .edt_versions_difference(&acteur(&session), q.de, q.vers)
            .await
            .map_err(map_db)?,
    ))
}

async fn individuelle(
    State(s): State<AppState>,
    session: EntSession,
    Json(cmd): Json<CmdIndividuelle>,
) -> Res<(StatusCode, Json<ResultatIndividuelle>)> {
    let r = s
        .ent
        .edt_publier_individuelle(&acteur(&session), &cmd)
        .await
        .map_err(map_db)?;
    let code = if !r.conflits.is_empty() {
        StatusCode::CONFLICT
    } else if r.deja_fait {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((code, Json(r)))
}
