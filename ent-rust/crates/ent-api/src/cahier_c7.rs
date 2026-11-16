//! Routes `/cahier/c7/*` du lot C7 du socle 20 : adaptations (`20_CAHIER_DE_TEXTES.md` §13).
//!
//! * Professeur en saisie : `GET` / `POST /cahier/c7/travaux/{id}/variantes`, `PUT /cahier/c7/variantes/{id}`,
//!   `POST` / `DELETE /cahier/c7/variantes/{id}/eleves`, `GET /cahier/c7/cours/{id}/pastilles`,
//!   `POST /cahier/c7/travaux/{id}/amenagements` (aperçu sans `?confirmer=true`), `GET /cahier/c7/travaux/{id}/notes-accompagnement`.
//! * AESH (élèves accompagnés à la date seulement) : `GET /cahier/c7/aesh/eleves`, `GET /cahier/c7/aesh/agenda`,
//!   `GET /cahier/c7/aesh/cahier`, `POST /cahier/c7/aesh/notes`.
//! * Élève : `GET` / `PUT /cahier/c7/affichage` (réglages d'affichage, jamais lus d'un adulte).
//!
//! Aucune route d'élève ni de famille ne porte d'indicateur de variante : l'élève lit sa version dans l'agenda (C3).
//! L'établissement vient de la session, jamais de la requête.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::cahier_c3::AgendaDto;
use ent_types::cahier_c6::EntreeRemplacementDto;
use ent_types::cahier_c7::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cahier/c7/travaux/{id}/variantes", get(variantes).post(variante_creer))
        .route("/cahier/c7/variantes/{id}", put(variante_modifier))
        .route("/cahier/c7/variantes/{id}/eleves", post(variante_attribuer).delete(variante_retirer))
        .route("/cahier/c7/cours/{id}/pastilles", get(pastilles))
        .route("/cahier/c7/travaux/{id}/amenagements", post(amenagements))
        .route("/cahier/c7/travaux/{id}/notes-accompagnement", get(notes_travail))
        .route("/cahier/c7/entrees/{id}/notes-accompagnement", get(notes_entree))
        .route("/cahier/c7/aesh/eleves", get(aesh_eleves))
        .route("/cahier/c7/aesh/agenda", get(aesh_agenda))
        .route("/cahier/c7/aesh/cahier", get(aesh_cahier))
        .route("/cahier/c7/aesh/notes", post(aesh_note))
        .route("/cahier/c7/affichage", get(affichage).put(affichage_regler))
}

async fn variantes(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<VarianteDto>>> {
    Ok(Json(s.ent.cdt_variantes_lister(&acteur(&session), id).await.map_err(map_db)?))
}

async fn variante_creer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<VarianteBody>) -> Res<(StatusCode, Json<VarianteDto>)> {
    Ok((StatusCode::CREATED, Json(s.ent.cdt_variante_creer(&acteur(&session), id, b).await.map_err(map_db)?)))
}

async fn variante_modifier(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<VarianteBody>) -> Res<Json<VarianteDto>> {
    Ok(Json(s.ent.cdt_variante_modifier(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn variante_attribuer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<AttributionBody>) -> Res<Json<VarianteDto>> {
    Ok(Json(s.ent.cdt_variante_attribuer(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn variante_retirer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<AttributionBody>) -> Res<Json<VarianteDto>> {
    Ok(Json(s.ent.cdt_variante_retirer(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn pastilles(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<PastillesCoursDto>> {
    Ok(Json(s.ent.cdt_pastilles_cours(&acteur(&session), id).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize)]
struct ConfirmerQuery {
    #[serde(default)]
    confirmer: bool,
}

async fn amenagements(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Query(q): Query<ConfirmerQuery>) -> Res<Json<ApplicationAmenagementsDto>> {
    Ok(Json(s.ent.cdt_amenagements_appliquer(&acteur(&session), id, q.confirmer).await.map_err(map_db)?))
}

async fn notes_travail(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<NoteAccompagnementDto>>> {
    Ok(Json(s.ent.cdt_notes_accompagnement_lire(&acteur(&session), Some(id), None).await.map_err(map_db)?))
}

async fn notes_entree(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<NoteAccompagnementDto>>> {
    Ok(Json(s.ent.cdt_notes_accompagnement_lire(&acteur(&session), None, Some(id)).await.map_err(map_db)?))
}

async fn aesh_eleves(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<EleveAccompagneDto>>> {
    Ok(Json(s.ent.cdt_aesh_eleves(&acteur(&session)).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize)]
struct AgendaQuery {
    eleve_id: Uuid,
    du: NaiveDate,
    au: NaiveDate,
}

async fn aesh_agenda(State(s): State<AppState>, session: EntSession, Query(q): Query<AgendaQuery>) -> Res<Json<AgendaDto>> {
    Ok(Json(s.ent.cdt_aesh_agenda(&acteur(&session), q.eleve_id, q.du, q.au).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize)]
struct CahierQuery {
    eleve_id: Uuid,
    cours_id: Uuid,
}

async fn aesh_cahier(State(s): State<AppState>, session: EntSession, Query(q): Query<CahierQuery>) -> Res<Json<Vec<EntreeRemplacementDto>>> {
    Ok(Json(s.ent.cdt_aesh_cahier(&acteur(&session), q.eleve_id, q.cours_id).await.map_err(map_db)?))
}

async fn aesh_note(State(s): State<AppState>, session: EntSession, Json(b): Json<NoteAccompagnementBody>) -> Res<(StatusCode, Json<NoteAccompagnementDto>)> {
    Ok((StatusCode::CREATED, Json(s.ent.cdt_note_accompagnement_ajouter(&acteur(&session), b).await.map_err(map_db)?)))
}

async fn affichage(State(s): State<AppState>, session: EntSession) -> Res<Json<AffichageDto>> {
    Ok(Json(s.ent.cdt_affichage_lire(&acteur(&session)).await.map_err(map_db)?))
}

async fn affichage_regler(State(s): State<AppState>, session: EntSession, Json(b): Json<AffichageDto>) -> Res<Json<AffichageDto>> {
    Ok(Json(s.ent.cdt_affichage_regler(&acteur(&session), b).await.map_err(map_db)?))
}
