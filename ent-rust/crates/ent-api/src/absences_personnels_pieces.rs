//! Routes des pièces, du fil et de la conservation des absences de personnels (socle 15, lot A2).
//! Préfixe `/personnels/absences`. L'autorité est `ent-db` (`absences_personnels/pieces.rs`) : cercle RH
//! (404 uniforme hors cercle), droits `personnels.absences.*` (D-017), journal, purge. Les routes décodent
//! et mappent les erreurs ; aucune ne renvoie un nom de fichier.
//!
//! Un fichier n'est **jamais** servi directement : `POST …/pieces/{piece}/lien` émet un lien pré-signé de
//! 5 minutes au plus, après contrôle du droit, et journalise la lecture (R-AP22).

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use base64::Engine;
use chrono::NaiveDate;
use ent_db::cahier::FichierDepot;
use ent_types::absences_personnels::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    let up = ent_db::upload::PolitiqueFichiers::depuis_env();
    let corps_fichier = DefaultBodyLimit::max(up.limite_corps_base64(ent_db::upload::UsageFichier::Document));
    Router::new()
        .route("/personnels/absences/echeances", get(echeances))
        .route("/personnels/absences/purges-a-venir", get(purges_a_venir))
        .route(
            "/personnels/absences/{id}/pieces",
            get(pieces).post(deposer).layer(corps_fichier),
        )
        .route("/personnels/absences/{id}/pieces/relancer", post(relancer))
        .route("/personnels/absences/{id}/pieces/lectures", get(lectures))
        .route("/personnels/absences/{id}/pieces/{piece_id}/lien", post(lien))
        .route("/personnels/absences/{id}/pieces/{piece_id}/retirer", post(retirer))
        .route("/personnels/absences/{id}/pieces/{piece_id}/purger", post(purger))
        .route("/personnels/absences/{id}/messages", get(fil).post(poster))
        .route("/personnels/absences/{id}/gels", get(gels).post(poser_gel))
        .route("/personnels/absences/{id}/gels/{gel_id}/lever", post(lever_gel))
        .route("/personnels/absences/{id}/confidentialite", put(confidentialite))
}

fn decoder(b64: &str) -> Res<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| (StatusCode::BAD_REQUEST, "contenu_base64 invalide".to_string()))
}

async fn deposer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DeposerPieceBody>,
) -> Res<(StatusCode, Json<PieceAbsenceDto>)> {
    let fichier = FichierDepot {
        nom: b.fichier.nom,
        mime_type: b.fichier.mime_type,
        contenu: decoder(&b.fichier.contenu_base64)?,
    };
    s.ent
        .deposer_piece_absence_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.type_piece,
            b.volet_1_exclu,
            fichier,
        )
        .await
        .map(|p| (StatusCode::CREATED, Json(p)))
        .map_err(map_db)
}

async fn pieces(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<PieceAbsenceDto>>> {
    s.ent
        .pieces_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn lien(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, piece_id)): Path<(Uuid, Uuid)>,
) -> Res<Json<LienPieceDto>> {
    s.ent
        .lien_piece_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id, piece_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn retirer(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, piece_id)): Path<(Uuid, Uuid)>,
) -> Res<Json<PieceAbsenceDto>> {
    s.ent
        .retirer_piece_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id, piece_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn purger(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, piece_id)): Path<(Uuid, Uuid)>,
    Json(b): Json<PurgerPieceBody>,
) -> Res<Json<PieceAbsenceDto>> {
    s.ent
        .purger_piece_absence_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            piece_id,
            &b.motif_code,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn relancer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<u32>> {
    s.ent
        .relancer_piece_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn lectures(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<LecturePieceDto>>> {
    s.ent
        .lectures_pieces_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn fil(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<MessageAbsenceDto>>> {
    s.ent
        .fil_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn poster(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PosterMessageBody>,
) -> Res<(StatusCode, Json<MessageAbsenceDto>)> {
    s.ent
        .poster_message_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b.texte)
        .await
        .map(|m| (StatusCode::CREATED, Json(m)))
        .map_err(map_db)
}

async fn echeances(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<EcheanceAbsenceDto>>> {
    s.ent
        .echeances_absences_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct JusquAu {
    jusqu_au: Option<NaiveDate>,
}

async fn purges_a_venir(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<JusquAu>,
) -> Res<Json<Vec<PurgeAVenirDto>>> {
    // Par défaut : les purges des 90 prochains jours.
    let jusqu_au = q.jusqu_au.unwrap_or_else(|| chrono::Utc::now().date_naive() + chrono::Duration::days(90));
    s.ent
        .purges_a_venir_for_actor(session.etablissement_id, session.user_id, &session.role, jusqu_au)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn gels(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<GelDto>>> {
    s.ent
        .gels_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn poser_gel(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PoserGelBody>,
) -> Res<(StatusCode, Json<GelDto>)> {
    s.ent
        .poser_gel_absence_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.motif,
            b.du,
            b.au,
        )
        .await
        .map(|g| (StatusCode::CREATED, Json(g)))
        .map_err(map_db)
}

async fn lever_gel(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, gel_id)): Path<(Uuid, Uuid)>,
) -> Res<StatusCode> {
    s.ent
        .lever_gel_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id, gel_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn confidentialite(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ConfidentialiteBody>,
) -> Res<StatusCode> {
    s.ent
        .definir_confidentialite_renforcee_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            b.renforcee,
        )
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}
