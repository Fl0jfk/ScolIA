//! Routes `/eva/v14/*` du lot V14 « cohabitation de l'an 1 » (socle 21, EVA-233 à EVA-237) : dépôt et rapprochement des
//! bulletins du logiciel tiers, contrôle et distribution, lecture famille, export de l'assiduité, bandeau « où sont les
//! notes ».
//!
//! L'autorité est `ent-db` (`evaluations_v14.rs`) : personnel, droit, puis existence. Ces routes ne font que traduire
//! HTTP ↔ service. Ouvertes à la famille et à l'élève : `GET /eva/v14/eleves/{id}/bulletins` (liste vide sans le droit
//! « voir les notes »), `GET /eva/v14/bulletins/{id}/pdf` (404 hors bulletin distribué lisible) et
//! `GET /eva/v14/cohabitation`. Tout le reste répond 403 à la famille et à l'élève, quel que soit le corps.

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use ent_types::evaluations_v14::{
    BulletinTiersDto, BulletinTiersFamilleDto, CohabitationDto, DeposerBulletinTiersBody,
    ExportAssiduiteTiersDto, LotTiersDto, MotifBulletinTiersBody, OuvrirLotBody,
    RapportDistributionDto, RapprocherBulletinBody, ReglerCohabitationBody,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::{attachment_response, map_db};
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

fn corps_fichier() -> DefaultBodyLimit {
    let up = ent_db::upload::PolitiqueFichiers::depuis_env();
    DefaultBodyLimit::max(up.limite_corps_base64(ent_db::upload::UsageFichier::Document))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/eva/v14/lots", get(lots).post(ouvrir_lot))
        .route("/eva/v14/lots/{id}", get(lot))
        .route(
            "/eva/v14/lots/{id}/fichiers",
            post(deposer).layer(corps_fichier()),
        )
        .route("/eva/v14/lots/{id}/controle", get(controler))
        .route("/eva/v14/lots/{id}/distribuer", post(distribuer))
        .route("/eva/v14/bulletins/{id}/rapprocher", post(rapprocher))
        .route("/eva/v14/bulletins/{id}/ecarter", post(ecarter))
        .route("/eva/v14/bulletins/{id}/retirer", post(retirer))
        .route("/eva/v14/bulletins/{id}/apercu", get(apercu))
        .route("/eva/v14/bulletins/{id}/pdf", get(pdf))
        .route("/eva/v14/eleves/{id}/bulletins", get(bulletins_eleve))
        .route(
            "/eva/v14/cohabitation",
            get(cohabitation).put(regler_cohabitation),
        )
        .route("/eva/v14/assiduite", get(exporter_assiduite))
}

async fn jour(s: &AppState, session: &EntSession) -> Res<chrono::NaiveDate> {
    s.ent
        .aujourdhui_etablissement(session.etablissement_id)
        .await
        .map_err(map_db)
}

#[derive(Deserialize)]
struct PeriodeQuery {
    periode_id: Uuid,
}

async fn lots(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<Vec<LotTiersDto>>> {
    s.ent
        .eva_v14_lots_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.periode_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn ouvrir_lot(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<OuvrirLotBody>,
) -> Res<(StatusCode, Json<LotTiersDto>)> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v14_ouvrir_lot_for_actor(
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

#[derive(Serialize)]
struct LotDetail {
    lot: LotTiersDto,
    bulletins: Vec<BulletinTiersDto>,
}

async fn lot(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<LotDetail>> {
    s.ent
        .eva_v14_lot_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(|(lot, bulletins)| Json(LotDetail { lot, bulletins }))
        .map_err(map_db)
}

async fn deposer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DeposerBulletinTiersBody>,
) -> Res<(StatusCode, Json<BulletinTiersDto>)> {
    // Le droit est vérifié par le service AVANT toute écriture ; le décodage ne fait que préparer les octets.
    let contenu = base64::engine::general_purpose::STANDARD
        .decode(b.contenu_base64.trim())
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "contenu_base64 invalide".to_string(),
            )
        })?;
    s.ent
        .eva_v14_deposer_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
            &contenu,
        )
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn controler(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<RapportDistributionDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v14_controler_for_actor(
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

async fn distribuer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<RapportDistributionDto>> {
    let j = jour(&s, &session).await?;
    s.ent
        .eva_v14_distribuer_for_actor(
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

async fn rapprocher(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RapprocherBulletinBody>,
) -> Res<Json<BulletinTiersDto>> {
    s.ent
        .eva_v14_rapprocher_for_actor(
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

async fn ecarter(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<MotifBulletinTiersBody>,
) -> Res<Json<BulletinTiersDto>> {
    s.ent
        .eva_v14_ecarter_for_actor(
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

async fn retirer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<MotifBulletinTiersBody>,
) -> Res<Json<BulletinTiersDto>> {
    s.ent
        .eva_v14_retirer_for_actor(
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

async fn apercu(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Response> {
    let (nom, octets) = s
        .ent
        .eva_v14_apercu_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    attachment_response(octets, &nom, "application/pdf")
}

async fn pdf(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Response> {
    let (nom, octets) = s
        .ent
        .eva_v14_pdf_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    attachment_response(octets, &nom, "application/pdf")
}

async fn bulletins_eleve(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<BulletinTiersFamilleDto>>> {
    s.ent
        .eva_v14_bulletins_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

/// Le bandeau de l'établissement de la session ; `null` sans réglage.
async fn cohabitation(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Option<CohabitationDto>>> {
    s.ent
        .eva_v14_cohabitation(session.etablissement_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn regler_cohabitation(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ReglerCohabitationBody>,
) -> Res<Json<CohabitationDto>> {
    s.ent
        .eva_v14_regler_cohabitation_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct AssiduiteQuery {
    periode_id: Uuid,
    classe_id: Uuid,
    /// `csv` : fichier pour le tableur ; sinon JSON.
    #[serde(default)]
    format: Option<String>,
}

async fn exporter_assiduite(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AssiduiteQuery>,
) -> Res<Response> {
    let j = jour(&s, &session).await?;
    let (export, classe) = s
        .ent
        .eva_v14_exporter_assiduite_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            j,
            q.periode_id,
            q.classe_id,
        )
        .await
        .map_err(map_db)?;
    if q.format.as_deref() == Some("csv") {
        let csv = ent_db::evaluations_v14::csv_assiduite_tiers(&export, &classe);
        let nom = format!(
            "assiduite-{}-{}.csv",
            export.du.format("%Y%m%d"),
            export.au.format("%Y%m%d")
        );
        return attachment_response(csv.into_bytes(), &nom, "text/csv; charset=utf-8");
    }
    Ok(Json::<ExportAssiduiteTiersDto>(export).into_response())
}
