//! Routes `/structure/*` du socle 5, lot S7 : périodisations (trimestres, semestres, périodes
//! libres, assignées à des niveaux, MEF, classes) et grille horaire versionnée.
//!
//! Aucune règle ici : droits (`referentiel.lire` / `referentiel.gerer` ; modifier une période
//! commencée et définir la coupure des demi-journées : direction et administrateur), cloisonnement
//! (établissement de la session, jamais du corps), validations et résolution sont dans `ent-db`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::{NaiveDate, NaiveTime};
use ent_db::structure_grilles::structure::GrilleResolue;
use ent_db::structure_grilles::{GrilleModif, GrilleSaisie, GrilleVue};
use ent_db::structure_periodes::{
    CibleSaisie, ControlePeriodisation, ModelePeriodes, PeriodeModif, PeriodeS7Saisie, PeriodeVue,
    PeriodisationSaisie, PeriodisationVue,
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
        .route(
            "/structure/annees/{annee_id}/periodisations",
            get(periodisations).post(periodisation_creer),
        )
        .route(
            "/structure/annees/{annee_id}/periodisations/modeles",
            get(periodisation_modeles),
        )
        .route(
            "/structure/annees/{annee_id}/periodisations/controle",
            get(periodisation_controle),
        )
        .route("/structure/periodisations/{id}", get(periodisation))
        .route(
            "/structure/periodisations/{id}/periodes",
            put(periodisation_periodes),
        )
        .route(
            "/structure/periodisations/{id}/cibles",
            put(periodisation_cibles),
        )
        .route(
            "/structure/periodisations/{id}/archiver",
            post(periodisation_archiver),
        )
        .route("/structure/periodes/{id}", put(periode_modifier))
        .route("/structure/classes/{id}/periodes", get(classe_periodes))
        .route("/structure/grilles", get(grilles).post(grille_versionner))
        .route(
            "/structure/grilles/{id}",
            get(grille).put(grille_remplacer).delete(grille_supprimer),
        )
        .route("/structure/grilles/{code}/coupure", post(grille_coupure))
        .route("/structure/grille", get(grille_a_la_date))
}

// ───── Périodisations ─────

#[derive(Debug, Deserialize)]
struct ArchiveesQuery {
    #[serde(default)]
    archivees: bool,
}

async fn periodisations(
    State(s): State<AppState>,
    session: EntSession,
    Path(annee_id): Path<Uuid>,
    Query(q): Query<ArchiveesQuery>,
) -> Res<Json<Vec<PeriodisationVue>>> {
    Ok(Json(
        s.ent
            .structure_periodisations(&acteur(&session), annee_id, q.archivees)
            .await
            .map_err(map_db)?,
    ))
}

async fn periodisation_creer(
    State(s): State<AppState>,
    session: EntSession,
    Path(annee_id): Path<Uuid>,
    Json(b): Json<PeriodisationSaisie>,
) -> Res<(StatusCode, Json<PeriodisationVue>)> {
    let v = s
        .ent
        .structure_periodisation_creer(&acteur(&session), annee_id, b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn periodisation_modeles(
    State(s): State<AppState>,
    session: EntSession,
    Path(annee_id): Path<Uuid>,
) -> Res<Json<Vec<ModelePeriodes>>> {
    Ok(Json(
        s.ent
            .structure_periodisation_modeles(&acteur(&session), annee_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn periodisation_controle(
    State(s): State<AppState>,
    session: EntSession,
    Path(annee_id): Path<Uuid>,
) -> Res<Json<ControlePeriodisation>> {
    Ok(Json(
        s.ent
            .structure_periodisation_controle(&acteur(&session), annee_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn periodisation(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<PeriodisationVue>> {
    Ok(Json(
        s.ent
            .structure_periodisation(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn periodisation_periodes(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<Vec<PeriodeS7Saisie>>,
) -> Res<Json<PeriodisationVue>> {
    Ok(Json(
        s.ent
            .structure_periodisation_remplacer_periodes(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn periodisation_cibles(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<Vec<CibleSaisie>>,
) -> Res<Json<PeriodisationVue>> {
    Ok(Json(
        s.ent
            .structure_periodisation_assigner(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn periodisation_archiver(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<PeriodisationVue>> {
    Ok(Json(
        s.ent
            .structure_periodisation_archiver(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn periode_modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PeriodeModif>,
) -> Res<Json<PeriodisationVue>> {
    Ok(Json(
        s.ent
            .structure_periode_modifier(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn classe_periodes(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<PeriodeVue>>> {
    Ok(Json(
        s.ent
            .structure_periodes_de_classe(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

// ───── Grilles ─────

async fn grilles(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<GrilleVue>>> {
    Ok(Json(
        s.ent
            .structure_grilles(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}

async fn grille_versionner(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<GrilleSaisie>,
) -> Res<(StatusCode, Json<GrilleVue>)> {
    let v = s
        .ent
        .structure_grille_versionner(&acteur(&session), b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn grille(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<GrilleVue>> {
    Ok(Json(
        s.ent
            .structure_grille(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn grille_remplacer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<GrilleModif>,
) -> Res<Json<GrilleVue>> {
    Ok(Json(
        s.ent
            .structure_grille_remplacer(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn grille_supprimer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .structure_grille_supprimer(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct CoupureBody {
    a_partir_du: NaiveDate,
    coupure: NaiveTime,
    #[serde(default)]
    motif: Option<String>,
}

async fn grille_coupure(
    State(s): State<AppState>,
    session: EntSession,
    Path(code): Path<String>,
    Json(b): Json<CoupureBody>,
) -> Res<Json<GrilleVue>> {
    Ok(Json(
        s.ent
            .structure_grille_coupure(&acteur(&session), &code, b.a_partir_du, b.coupure, b.motif.as_deref())
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct GrilleQuery {
    date: NaiveDate,
    #[serde(default)]
    site_id: Option<Uuid>,
    #[serde(default)]
    niveau_id: Option<Uuid>,
}

async fn grille_a_la_date(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<GrilleQuery>,
) -> Res<Json<Option<GrilleResolue>>> {
    Ok(Json(
        s.ent
            .structure_grille_a_la_date(&acteur(&session), q.date, q.site_id, q.niveau_id)
            .await
            .map_err(map_db)?,
    ))
}
