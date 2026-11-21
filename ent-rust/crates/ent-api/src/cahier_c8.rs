//! Routes `/cahier/c8/*` du lot C8 du socle 20 : tenue, visa, inspection, exports figés, clôture et archive
//! (`20_CAHIER_DE_TEXTES.md` §14).
//!
//! * Direction : `GET /cahier/c8/tenue`, `GET` / `POST /cahier/c8/visas`, `GET /cahier/c8/visas/{id}`,
//!   `POST /cahier/c8/visas/{id}/observations`, `GET /cahier/c8/cours/{id}/chronologie`.
//! * Professeur : `POST /cahier/c8/observations/{id}/reponses`, `GET /cahier/c8/mon-export`.
//! * Accès d'inspection (direction) : `GET` / `POST /cahier/c8/inspections`, `POST /cahier/c8/inspections/{id}/prolonger`,
//!   `POST /cahier/c8/inspections/{id}/revoquer`, `GET /cahier/c8/inspections/{id}/journal` (aussi du professeur concerné).
//! * Inspecteur : `GET /cahier/c8/inspection/mes-acces`, `GET /cahier/c8/inspection/{acces}/cours/{cours}`.
//! * Exports figés (direction) : `GET` / `POST /cahier/c8/exports`, `GET /cahier/c8/exports/{id}`.
//! * Clôture et archive (direction) : `GET /cahier/c8/cloture/controle`, `POST /cahier/c8/cloture`,
//!   `POST /cahier/c8/cloture/reouvrir`, `GET /cahier/c8/archives`, `GET /cahier/c8/archives/{id}/contenu`,
//!   `POST /cahier/c8/archives/{id}/decision`.
//!
//! L'établissement vient de la session, jamais de la requête.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::cahier_c8::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cahier/c8/tenue", get(tenue))
        .route("/cahier/c8/visas", get(visas).post(viser))
        .route("/cahier/c8/visas/{id}", get(visa_detail))
        .route("/cahier/c8/visas/{id}/observations", post(observer))
        .route("/cahier/c8/observations/{id}/reponses", post(repondre))
        .route("/cahier/c8/cours/{id}/chronologie", get(chronologie))
        .route("/cahier/c8/mon-export", get(mon_export))
        .route(
            "/cahier/c8/inspections",
            get(acces_lister).post(acces_accorder),
        )
        .route(
            "/cahier/c8/inspections/{id}/prolonger",
            post(acces_prolonger),
        )
        .route("/cahier/c8/inspections/{id}/revoquer", post(acces_revoquer))
        .route("/cahier/c8/inspections/{id}/journal", get(acces_journal))
        .route("/cahier/c8/inspection/mes-acces", get(mes_acces))
        .route(
            "/cahier/c8/inspection/{acces}/cours/{cours}",
            get(inspection_lire),
        )
        .route(
            "/cahier/c8/exports",
            get(exports_lister).post(export_produire),
        )
        .route("/cahier/c8/exports/{id}", get(export_lire))
        .route("/cahier/c8/cloture/controle", get(cloture_controle))
        .route("/cahier/c8/cloture", post(cloturer))
        .route("/cahier/c8/cloture/reouvrir", post(rouvrir))
        .route("/cahier/c8/archives", get(archives))
        .route("/cahier/c8/archives/{id}/contenu", get(archive_contenu))
        .route("/cahier/c8/archives/{id}/decision", post(archive_decider))
}

#[derive(Deserialize)]
struct Periode {
    du: NaiveDate,
    au: NaiveDate,
}

#[derive(Deserialize)]
struct VisasQuery {
    cours_id: Option<Uuid>,
}

#[derive(Deserialize)]
struct AnneeQuery {
    annee_id: Uuid,
}

#[derive(Deserialize)]
struct AnneeFacultative {
    annee_id: Option<Uuid>,
}

async fn tenue(
    State(s): State<AppState>,
    session: EntSession,
    Query(p): Query<Periode>,
) -> Res<Json<TenueDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_tenue(&acteur(&session), p.du, p.au)
            .await
            .map_err(map_db)?,
    ))
}

async fn visas(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<VisasQuery>,
) -> Res<Json<Vec<VisaCahierDto>>> {
    Ok(Json(
        s.ent
            .cdt_c8_visas(&acteur(&session), q.cours_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn viser(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<VisaCahierBody>,
) -> Res<(StatusCode, Json<VisaCahierDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.ent
                .cdt_c8_viser(&acteur(&session), b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn visa_detail(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<VisaDetailDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_visa_detail(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn observer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ObservationBody>,
) -> Res<(StatusCode, Json<VisaCahierDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.ent
                .cdt_c8_observer(&acteur(&session), id, b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn repondre(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ObservationBody>,
) -> Res<(StatusCode, Json<VisaCahierDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.ent
                .cdt_c8_repondre(&acteur(&session), id, b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn chronologie(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(p): Query<Periode>,
) -> Res<Json<ChronologieDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_chronologie(&acteur(&session), id, p.du, p.au)
            .await
            .map_err(map_db)?,
    ))
}

async fn mon_export(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AnneeQuery>,
) -> Res<Json<ExportContenuDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_export_professeur(&acteur(&session), q.annee_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn acces_lister(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<AccesInspectionDto>>> {
    Ok(Json(
        s.ent
            .cdt_c8_acces_lister(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}

async fn acces_accorder(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<AccesInspectionBody>,
) -> Res<(StatusCode, Json<AccesInspectionDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.ent
                .cdt_c8_acces_accorder(&acteur(&session), b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn acces_prolonger(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ProlongerBody>,
) -> Res<Json<AccesInspectionDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_acces_prolonger(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn acces_revoquer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<AccesInspectionDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_acces_revoquer(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn acces_journal(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<ConsultationDto>>> {
    Ok(Json(
        s.ent
            .cdt_c8_acces_journal(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn mes_acces(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<AccesInspectionDto>>> {
    Ok(Json(
        s.ent
            .cdt_c8_mes_acces(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}

async fn inspection_lire(
    State(s): State<AppState>,
    session: EntSession,
    Path((acces, cours)): Path<(Uuid, Uuid)>,
) -> Res<Json<CahierInspectionDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_inspection_lire(&acteur(&session), acces, cours)
            .await
            .map_err(map_db)?,
    ))
}

async fn exports_lister(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<ExportDto>>> {
    Ok(Json(
        s.ent
            .cdt_c8_exports_lister(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}

async fn export_produire(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ExportBody>,
) -> Res<(StatusCode, Json<ExportDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.ent
                .cdt_c8_export_produire(&acteur(&session), b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn export_lire(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ExportContenuDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_export_lire(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn cloture_controle(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AnneeQuery>,
) -> Res<Json<ControleClotureDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_cloture_controle(&acteur(&session), q.annee_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn cloturer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ClotureBody>,
) -> Res<Json<ClotureDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_cloturer(&acteur(&session), b)
            .await
            .map_err(map_db)?,
    ))
}

async fn rouvrir(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ReouvrirBody>,
) -> Res<Json<ClotureDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_rouvrir(&acteur(&session), b)
            .await
            .map_err(map_db)?,
    ))
}

async fn archives(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AnneeFacultative>,
) -> Res<Json<Vec<ArchiveDto>>> {
    Ok(Json(
        s.ent
            .cdt_c8_archives(&acteur(&session), q.annee_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn archive_contenu(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ExportContenuDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_archive_contenu(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn archive_decider(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ArchiveDecisionBody>,
) -> Res<Json<ArchiveDto>> {
    Ok(Json(
        s.ent
            .cdt_c8_archive_decider(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}
