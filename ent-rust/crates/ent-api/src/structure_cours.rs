//! Routes `/structure/cours*` du socle 5, lot S4 : l'objet « cours » (ce qui est enseigné à qui),
//! les postes vacants, la pose d'un service sur un cours et l'état de la reprise.
//!
//! Aucune règle ici : droits (`referentiel.lire` pour la liste, `referentiel.gerer` pour le détail
//! avec les professeurs et pour toute écriture), cloisonnement par l'établissement de la session
//! et validations sont dans `ent-db`. Les membres individuels (inclusions et exclusions) restent
//! sous `/referentiel/cours/{id}/membres` (lot S1) et acceptent désormais l'identifiant du cours.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_db::structure_cours::{
    CoursDetail, CoursFerme, CoursModif, CoursSaisie, CoursVue, EtatReprise, ServiceDuCours,
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
        .route("/structure/cours", get(liste).post(creer))
        .route("/structure/cours/vacants", get(vacants))
        .route("/structure/cours/reprise", get(reprise))
        .route("/structure/cours/{id}", get(detail).put(modifier))
        .route("/structure/cours/{id}/fermer", post(fermer))
        .route("/structure/cours/{id}/services", post(pourvoir))
}

#[derive(Debug, Deserialize)]
struct ListeQuery {
    annee_id: Option<Uuid>,
    classe_id: Option<Uuid>,
    groupe_id: Option<Uuid>,
    #[serde(default)]
    vacants: bool,
    #[serde(default)]
    fermes: bool,
}

async fn liste(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<ListeQuery>,
) -> Res<Json<Vec<CoursVue>>> {
    Ok(Json(
        s.ent
            .structure_cours_liste(
                &acteur(&session),
                q.annee_id,
                q.classe_id,
                q.groupe_id,
                q.vacants,
                q.fermes,
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn vacants(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<CoursVue>>> {
    Ok(Json(
        s.ent
            .structure_cours_vacants(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}

async fn reprise(State(s): State<AppState>, session: EntSession) -> Res<Json<EtatReprise>> {
    Ok(Json(
        s.ent
            .structure_cours_reprise(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}

async fn detail(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<CoursDetail>> {
    Ok(Json(
        s.ent
            .structure_cours_detail(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CoursSaisie>,
) -> Res<(StatusCode, Json<CoursVue>)> {
    let c = s
        .ent
        .structure_cours_creer(&acteur(&session), b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(c)))
}

async fn modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CoursModif>,
) -> Res<Json<CoursVue>> {
    Ok(Json(
        s.ent
            .structure_cours_modifier(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct FermerBody {
    a_la_date: NaiveDate,
}

async fn fermer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<FermerBody>,
) -> Res<Json<CoursFerme>> {
    Ok(Json(
        s.ent
            .structure_cours_fermer(&acteur(&session), id, b.a_la_date)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct PourvoirBody {
    prof_user_id: Uuid,
    #[serde(default)]
    professeur_principal: bool,
}

async fn pourvoir(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PourvoirBody>,
) -> Res<(StatusCode, Json<ServiceDuCours>)> {
    let r = s
        .ent
        .structure_cours_pourvoir(
            &acteur(&session),
            id,
            b.prof_user_id,
            b.professeur_principal,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}
