//! Routes des **pièces réelles** (lot I3) et des **accords des responsables** (lot I4) du socle 3.
//!
//! L'autorité est `ent-db` (`inscriptions_pieces.rs`, `inscriptions_accords.rs`) : droits
//! (`inscriptions.gerer`, `inscriptions.decider`), déposant vérifié, autorité parentale relue à chaque réponse.
//! Types JSON : `ent_types::inscriptions_pieces`.
//!
//! * **Personnel** (session) : règles de pièces d'une campagne, pièces attendues d'un dossier, dépôt au guichet,
//!   contrôle, lecture du fichier (PDF désarmé, jamais l'original), accords, désaccord, levée avec pièce.
//! * **Famille** (en-tête `X-Preinscription`, comme le reste de l'espace) : voir ses pièces, déposer un PDF ou des
//!   photos (base64, dans l'ordre des pages ; le type réel est contrôlé sur le contenu), reprendre une pièce de
//!   foyer déjà fournie, répondre à un accord depuis son compte.
//! * **Public sans compte** : `POST /accords/{id}/consulter|code|repondre`. Le jeton du lien (dérivé d'un secret
//!   serveur, jamais stocké) et le code reçu par e-mail font foi ; réponses identiques pour tout lien refusé et
//!   pour un accord comme pour un désaccord ; débit limité (`anti_abus.rs`).

use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use ent_types::inscriptions_pieces::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::tenant::{mode_tenant, ModeTenant, TenantHote};
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

/// Corps maximal d'un dépôt : 45 Mio de fichiers (limite du socle 7) en base64.
const DEPOT_CORPS_MAX: usize = 64 * 1024 * 1024;
const ENTETE_JETON: &str = crate::inscriptions::ENTETE_JETON;

pub fn routes() -> Router<AppState> {
    let depot = DefaultBodyLimit::max(DEPOT_CORPS_MAX);
    Router::new()
        // Personnel : règles, pièces, accords.
        .route(
            "/inscriptions/campagnes/{id}/pieces-regles",
            get(regles).put(definir_regles),
        )
        .route("/inscriptions/dossiers/{id}/pieces-attendues", get(pieces))
        .route(
            "/inscriptions/dossiers/{id}/pieces/{code}/fichiers",
            post(deposer_guichet).layer(depot),
        )
        .route("/inscriptions/dossiers/{id}/pieces/{code}/controle", post(controler))
        .route("/inscriptions/dossiers/{id}/pieces/{code}/reutiliser", post(reutiliser))
        .route("/inscriptions/dossiers/{id}/pieces/{code}/fichier", get(lire_fichier))
        .route("/inscriptions/dossiers/{id}/accords", get(accords))
        .route("/inscriptions/dossiers/{id}/accords/demander", post(demander_accords))
        .route("/inscriptions/dossiers/{id}/accords/papier", post(accord_papier))
        .route("/inscriptions/dossiers/{id}/accords/desaccord", post(desaccord))
        .route("/inscriptions/dossiers/{id}/accords/dispenser", post(dispenser))
        .route("/inscriptions/dossiers/{id}/accords/lever", post(lever))
        // Famille (jeton de préinscription).
        .route("/preinscription/dossiers/{id}/pieces-attendues", get(pieces_famille))
        .route(
            "/preinscription/dossiers/{id}/pieces/{code}/fichiers",
            post(deposer_famille).layer(depot),
        )
        .route("/preinscription/dossiers/{id}/pieces/{code}/reutiliser", post(reutiliser_famille))
        .route("/preinscription/dossiers/{id}/accord", post(accord_compte))
        // Public : le lien reçu par e-mail fait foi.
        .route("/accords/{id}/consulter", post(accord_consulter))
        .route("/accords/{id}/code", post(accord_code))
        .route("/accords/{id}/repondre", post(accord_repondre))
}

fn map(err: ent_db::EntDbError) -> (StatusCode, String) {
    match &err {
        ent_db::EntDbError::Auth(ent_auth::AuthError::InvalidSession) => (
            StatusCode::UNAUTHORIZED,
            "session de préinscription absente ou expirée".into(),
        ),
        _ => map_db(err),
    }
}

fn jeton(headers: &HeaderMap) -> Res<String> {
    headers
        .get(ENTETE_JETON)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 256)
        .map(str::to_string)
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "session de préinscription absente ou expirée".into(),
        ))
}

fn corps<T>(b: Result<Json<T>, JsonRejection>) -> Res<T> {
    b.map(|Json(v)| v).map_err(|e| (e.status(), e.body_text()))
}

async fn etab_jeton(state: &AppState, tenant: &TenantHote, j: &str) -> Res<Uuid> {
    if let Some(e) = tenant.etablissement_id {
        return Ok(e);
    }
    if mode_tenant() == ModeTenant::Strict {
        return Err((StatusCode::NOT_FOUND, "établissement inconnu".into()));
    }
    state
        .ent
        .etablissement_du_jeton_preinscription(j)
        .await
        .map_err(map)?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "session de préinscription absente ou expirée".into(),
        ))
}

/// Décode les fichiers d'un dépôt (base64 standard). Le type réel est contrôlé par `ent-db` sur le contenu.
fn decoder(b: &DepotFichiersBody) -> Res<Vec<Vec<u8>>> {
    if b.fichiers.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "un PDF, ou de une à cinquante photos".into()));
    }
    b.fichiers
        .iter()
        .map(|f| {
            base64::engine::general_purpose::STANDARD
                .decode(f.trim())
                .map_err(|_| (StatusCode::BAD_REQUEST, "fichier illisible (base64 attendu)".into()))
        })
        .collect()
}

// ───────────────────────────── Personnel ─────────────────────────────

async fn regles(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<ReglePieceDto>>> {
    Ok(Json(
        state
            .ent
            .regles_pieces_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn definir_regles(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ReglesPiecesBody>,
) -> Res<Json<Vec<ReglePieceDto>>> {
    Ok(Json(
        state
            .ent
            .definir_regles_pieces_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b.regles)
            .await
            .map_err(map)?,
    ))
}

async fn pieces(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<PieceAttendueDto>>> {
    Ok(Json(
        state
            .ent
            .pieces_dossier_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn deposer_guichet(
    State(state): State<AppState>,
    s: EntSession,
    Path((id, code)): Path<(Uuid, String)>,
    Json(b): Json<DepotFichiersBody>,
) -> Res<Json<DepotPieceDto>> {
    let fichiers = decoder(&b)?;
    Ok(Json(
        state
            .ent
            .deposer_fichiers_guichet_for_actor(s.etablissement_id, s.user_id, &s.role, id, &code, fichiers)
            .await
            .map_err(map)?,
    ))
}

async fn controler(
    State(state): State<AppState>,
    s: EntSession,
    Path((id, code)): Path<(Uuid, String)>,
    Json(b): Json<ControlePieceBody>,
) -> Res<Json<Vec<PieceAttendueDto>>> {
    Ok(Json(
        state
            .ent
            .controler_piece_inscription_for_actor(s.etablissement_id, s.user_id, &s.role, id, &code, &b)
            .await
            .map_err(map)?,
    ))
}

async fn reutiliser(
    State(state): State<AppState>,
    s: EntSession,
    Path((id, code)): Path<(Uuid, String)>,
    Json(b): Json<ReutilisationPieceBody>,
) -> Res<Json<Vec<PieceAttendueDto>>> {
    Ok(Json(
        state
            .ent
            .reutiliser_piece_dossier_for_actor(s.etablissement_id, s.user_id, &s.role, id, &code, b.source_dossier_id)
            .await
            .map_err(map)?,
    ))
}

/// Le fichier **servi** (PDF désarmé) : en pièce jointe, jamais interprété par le navigateur, jamais mis en cache.
async fn lire_fichier(
    State(state): State<AppState>,
    s: EntSession,
    Path((id, code)): Path<(Uuid, String)>,
) -> Res<Response> {
    let p = state
        .ent
        .lire_piece_inscription_for_actor(s.etablissement_id, s.user_id, &s.role, id, &code)
        .await
        .map_err(map)?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"piece.pdf\""),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        p.octets,
    )
        .into_response())
}

async fn accords(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<AccordsDossierDto>> {
    Ok(Json(
        state
            .ent
            .accords_dossier_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn demander_accords(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DemanderAccordsBody>,
) -> Res<Json<usize>> {
    Ok(Json(
        state
            .ent
            .demander_accords_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

async fn accord_papier(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AccordPapierBody>,
) -> Res<Json<AccordsDossierDto>> {
    Ok(Json(
        state
            .ent
            .saisir_accord_papier_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

async fn desaccord(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DesaccordBody>,
) -> Res<Json<AccordsDossierDto>> {
    Ok(Json(
        state
            .ent
            .declarer_desaccord_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

async fn dispenser(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DispenseAccordBody>,
) -> Res<Json<AccordsDossierDto>> {
    Ok(Json(
        state
            .ent
            .dispenser_accord_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

async fn lever(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<LeverDesaccordBody>,
) -> Res<Json<AccordsDossierDto>> {
    Ok(Json(
        state
            .ent
            .lever_desaccord_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

// ───────────────────────────── Famille ─────────────────────────────

async fn pieces_famille(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<PieceAttendueDto>>> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    Ok(Json(
        state.ent.pieces_dossier_famille(etab, &j, id).await.map_err(map)?,
    ))
}

async fn deposer_famille(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Path((id, code)): Path<(Uuid, String)>,
    b: Result<Json<DepotFichiersBody>, JsonRejection>,
) -> Res<Json<DepotPieceDto>> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    let fichiers = decoder(&corps(b)?)?;
    Ok(Json(
        state
            .ent
            .deposer_fichiers_famille(etab, &j, id, &code, fichiers)
            .await
            .map_err(map)?,
    ))
}

async fn reutiliser_famille(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Path((id, code)): Path<(Uuid, String)>,
    b: Result<Json<ReutilisationPieceBody>, JsonRejection>,
) -> Res<Json<Vec<PieceAttendueDto>>> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .reutiliser_piece_famille(etab, &j, id, &code, b.source_dossier_id)
            .await
            .map_err(map)?,
    ))
}

#[derive(Deserialize)]
struct ReponseCompteBody {
    /// `donne` ou `refuse`.
    reponse: String,
}

async fn accord_compte(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    b: Result<Json<ReponseCompteBody>, JsonRejection>,
) -> Res<Json<ReponseEnregistreeDto>> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    let b = corps(b)?;
    state
        .ent
        .repondre_accord_compte(etab, &j, id, &b.reponse)
        .await
        .map_err(map)?;
    Ok(Json(message_enregistre()))
}

// ───────────────────────────── Public, sans compte ─────────────────────────────

fn message_enregistre() -> ReponseEnregistreeDto {
    ReponseEnregistreeDto {
        message: "Votre réponse a été enregistrée. L'établissement vous tiendra informé(e).".into(),
    }
}

fn lien_invalide() -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, "lien invalide ou expiré".into())
}

/// Établissement d'un lien d'accord : celui de l'adresse ; à défaut (mode souple), celui de la demande.
async fn etab_accord(state: &AppState, tenant: &TenantHote, demande: Uuid) -> Res<Uuid> {
    if let Some(e) = tenant.etablissement_id {
        return Ok(e);
    }
    if mode_tenant() == ModeTenant::Strict {
        return Err(lien_invalide());
    }
    state
        .ent
        .etablissement_de_accord(demande)
        .await
        .map_err(map)?
        .ok_or_else(lien_invalide)
}

async fn accord_consulter(
    State(state): State<AppState>,
    tenant: TenantHote,
    Path(id): Path<Uuid>,
    Json(b): Json<JetonAccordBody>,
) -> Res<Json<AccordPublicDto>> {
    let etab = etab_accord(&state, &tenant, id).await?;
    Ok(Json(state.ent.accord_public(etab, id, &b.jeton).await.map_err(map)?))
}

async fn accord_code(
    State(state): State<AppState>,
    tenant: TenantHote,
    Path(id): Path<Uuid>,
    Json(b): Json<JetonAccordBody>,
) -> Res<Json<CodeEnvoyeDto>> {
    let etab = etab_accord(&state, &tenant, id).await?;
    // Réponse identique que l'envoi ait eu lieu ou non pour une cause « lien » : aucune indication.
    match state.ent.envoyer_code_accord(etab, id, &b.jeton).await {
        Ok(()) => {}
        Err(e) => {
            let (st, msg) = map(e);
            if st != StatusCode::BAD_REQUEST && st != StatusCode::UNPROCESSABLE_ENTITY {
                return Err((st, msg));
            }
        }
    }
    Ok(Json(CodeEnvoyeDto {
        message: "Si ce lien est valide, un code vient d'être envoyé à l'adresse de votre dossier.".into(),
    }))
}

async fn accord_repondre(
    State(state): State<AppState>,
    tenant: TenantHote,
    Path(id): Path<Uuid>,
    Json(b): Json<ReponseAccordBody>,
) -> Res<Json<ReponseEnregistreeDto>> {
    let etab = etab_accord(&state, &tenant, id).await?;
    state
        .ent
        .repondre_accord(etab, id, &b.jeton, &b.code, &b.reponse)
        .await
        .map_err(map)?;
    // Même message pour un accord et pour un désaccord : rien ne dit ce qu'a répondu l'autre parent.
    Ok(Json(message_enregistre()))
}
