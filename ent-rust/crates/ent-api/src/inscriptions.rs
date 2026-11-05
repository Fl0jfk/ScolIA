//! Routes des admissions, préinscriptions, inscriptions, réinscriptions et
//! imports (socle 2, lot S2-3 ; D-014).
//!
//! L'autorité est `ent-db` (`inscriptions.rs`, `imports.rs`) : droits
//! (`inscriptions.gerer`, `inscriptions.decider`, `imports.appliquer`), liens
//! actifs, e-mail vérifié. Types JSON : `ent_types::inscriptions`.
//!
//! **Routes publiques** (sans session) : `POST /preinscription/demarrer`,
//! `POST /preinscription/verifier`, `POST /inscriptions/invitations/accepter`.
//! Établissement imposé par l'adresse (D-012) ; limitation de débit dans
//! `anti_abus.rs`. **Espace famille** : en-tête `X-Preinscription` (jeton remis
//! après vérification de l'e-mail) ; sans jeton valide → 401, rien n'est affiché.

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use ent_types::inscriptions::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::tenant::{mode_tenant, ModeTenant, TenantHote};
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub const ENTETE_JETON: &str = "x-preinscription";

pub fn routes() -> Router<AppState> {
    Router::new()
        // Personnel : campagnes et instruction.
        .route("/inscriptions/campagnes", get(campagnes).post(creer_campagne))
        .route("/inscriptions/campagnes/{id}", put(modifier_campagne))
        .route("/inscriptions/campagnes/{id}/ouvrir", post(ouvrir_campagne))
        .route("/inscriptions/campagnes/{id}/fermer", post(fermer_campagne))
        .route("/inscriptions/dossiers", get(dossiers))
        .route("/inscriptions/dossiers/{id}", get(dossier))
        .route("/inscriptions/dossiers/{id}/statut", post(statut))
        .route("/inscriptions/dossiers/{id}/pieces/{code}", post(piece))
        .route("/inscriptions/dossiers/{id}/commentaires", post(commenter))
        .route("/inscriptions/dossiers/{id}/relancer", post(relancer))
        .route("/inscriptions/dossiers/{id}/decision", post(decider))
        .route("/inscriptions/dossiers/{id}/valider", post(valider))
        // Responsables connectés : réinscription.
        .route("/inscriptions/famille/reinscriptions", get(reinscriptions))
        .route(
            "/inscriptions/famille/reinscriptions/{id}/confirmer",
            post(confirmer_reinscription),
        )
        .route(
            "/inscriptions/famille/reinscriptions/{id}/depart",
            post(annoncer_depart),
        )
        // Public : invitation reçue par e-mail.
        .route("/inscriptions/invitations/accepter", post(accepter_invitation))
        // Préinscription (public puis jeton).
        .route("/preinscription/demarrer", post(demarrer))
        .route("/preinscription/verifier", post(verifier))
        .route("/preinscription/session", post(jeton_session))
        .route("/preinscription/moi", get(espace))
        .route("/preinscription/dossiers", post(deposer))
        .route("/preinscription/dossiers/{id}", get(dossier_famille))
        .route(
            "/preinscription/dossiers/{id}/pieces/{code}",
            post(piece_famille),
        )
        .route("/preinscription/dossiers/{id}/banque", get(banque))
        .route("/preinscription/dossiers/{id}/mandat", post(mandat))
        .route("/preinscription/dossiers/{id}/mandat/signer", post(signer_mandat))
        // Imports.
        .route(
            "/imports/eleves",
            post(previsualiser_import).layer(DefaultBodyLimit::max(3 * 1024 * 1024)),
        )
        .route("/imports/{id}", get(lot_import))
        .route("/imports/{id}/appliquer", post(appliquer_import))
        .route("/imports/lignes/{id}/traiter", post(traiter_ligne))
}

/// Jeton absent ou invalide → 401 ; le reste suit `map_db`.
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

/// Établissement d'une requête famille : celui de l'adresse ; à défaut (mode
/// souple, adresse inconnue), celui auquel le jeton est lié.
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

// -- Personnel ---------------------------------------------------------------

async fn campagnes(State(state): State<AppState>, s: EntSession) -> Res<Json<Vec<CampagneDto>>> {
    Ok(Json(
        state
            .ent
            .list_campagnes_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map)?,
    ))
}

async fn creer_campagne(
    State(state): State<AppState>,
    s: EntSession,
    Json(b): Json<CampagneBody>,
) -> Res<(StatusCode, Json<CampagneDto>)> {
    let c = state
        .ent
        .creer_campagne_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
        .await
        .map_err(map)?;
    Ok((StatusCode::CREATED, Json(c)))
}

async fn modifier_campagne(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CampagneBody>,
) -> Res<Json<CampagneDto>> {
    Ok(Json(
        state
            .ent
            .modifier_campagne_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

async fn ouvrir_campagne(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<OuvertureCampagneDto>> {
    Ok(Json(
        state
            .ent
            .ouvrir_campagne_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn fermer_campagne(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<CampagneDto>> {
    Ok(Json(
        state
            .ent
            .fermer_campagne_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

#[derive(Deserialize)]
struct FiltreDossiers {
    campagne_id: Option<Uuid>,
    statut: Option<String>,
}

async fn dossiers(
    State(state): State<AppState>,
    s: EntSession,
    Query(f): Query<FiltreDossiers>,
) -> Res<Json<Vec<DossierResumeDto>>> {
    Ok(Json(
        state
            .ent
            .list_dossiers_for_actor(s.etablissement_id, s.user_id, &s.role, f.campagne_id, f.statut)
            .await
            .map_err(map)?,
    ))
}

async fn dossier(State(state): State<AppState>, s: EntSession, Path(id): Path<Uuid>) -> Res<Json<DossierDto>> {
    Ok(Json(
        state
            .ent
            .dossier_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn statut(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<StatutBody>,
) -> Res<Json<DossierDto>> {
    Ok(Json(
        state
            .ent
            .changer_statut_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

async fn piece(
    State(state): State<AppState>,
    s: EntSession,
    Path((id, code)): Path<(Uuid, String)>,
    Json(b): Json<PieceDecisionBody>,
) -> Res<Json<DossierDto>> {
    Ok(Json(
        state
            .ent
            .decider_piece_for_actor(s.etablissement_id, s.user_id, &s.role, id, &code, &b)
            .await
            .map_err(map)?,
    ))
}

async fn commenter(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CommentaireBody>,
) -> Res<Json<DossierDto>> {
    Ok(Json(
        state
            .ent
            .commenter_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

async fn relancer(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<serde_json::Value>> {
    let n = state
        .ent
        .relancer_for_actor(s.etablissement_id, s.user_id, &s.role, id)
        .await
        .map_err(map)?;
    Ok(Json(serde_json::json!({ "pieces_relancees": n })))
}

async fn decider(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DecisionBody>,
) -> Res<Json<DossierDto>> {
    Ok(Json(
        state
            .ent
            .decider_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

async fn valider(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ValidationBody>,
) -> Res<Json<ValidationDto>> {
    Ok(Json(
        state
            .ent
            .valider_inscription_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

// -- Responsables connectés : réinscription -------------------------------------

async fn reinscriptions(State(state): State<AppState>, s: EntSession) -> Res<Json<Vec<DossierDto>>> {
    Ok(Json(
        state
            .ent
            .reinscriptions_famille(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map)?,
    ))
}

async fn confirmer_reinscription(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ConfirmationReinscriptionBody>,
) -> Res<Json<DossierDto>> {
    Ok(Json(
        state
            .ent
            .confirmer_reinscription_famille(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

async fn annoncer_depart(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DepartBody>,
) -> Res<Json<DossierDto>> {
    Ok(Json(
        state
            .ent
            .annoncer_depart_famille(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

// -- Public ------------------------------------------------------------------

async fn accepter_invitation(
    State(state): State<AppState>,
    tenant: TenantHote,
    Json(b): Json<InvitationBody>,
) -> Res<Json<InvitationAccepteeDto>> {
    let refus = || (StatusCode::BAD_REQUEST, "invitation invalide ou expirée".to_string());
    let etab = match tenant.etablissement_id {
        Some(e) => e,
        None if mode_tenant() == ModeTenant::Strict => return Err(refus()),
        None => state
            .ent
            .etablissement_de_invitation(&b.jeton)
            .await
            .map_err(map)?
            .ok_or_else(refus)?,
    };
    Ok(Json(state.ent.accepter_invitation(etab, &b.jeton).await.map_err(map)?))
}

/// Étape 1 : réponse identique que le compte existe ou non (aucune lecture de
/// compte ici ; l'envoi part dans tous les cas).
async fn demarrer(
    State(state): State<AppState>,
    tenant: TenantHote,
    Json(b): Json<DemarrerBody>,
) -> Res<(StatusCode, Json<DemarrerDto>)> {
    let etab = tenant.etablissement_effectif(b.etablissement_id)?;
    let r = state.ent.demarrer_preinscription(etab, &b.email).await.map_err(map)?;
    Ok((StatusCode::ACCEPTED, Json(r)))
}

async fn verifier(
    State(state): State<AppState>,
    tenant: TenantHote,
    Json(b): Json<VerifierBody>,
) -> Res<Json<JetonPreinscriptionDto>> {
    let etab = match tenant.etablissement_id {
        Some(e) => e,
        None if mode_tenant() == ModeTenant::Strict => {
            return Err((StatusCode::NOT_FOUND, "établissement inconnu".into()))
        }
        // Mode souple : l'établissement de la demande (le code reste exigé).
        None => state
            .ent
            .etablissement_de_demande_preinscription(b.demande_id)
            .await
            .map_err(map)?
            .ok_or((StatusCode::BAD_REQUEST, "valeur invalide : code invalide ou expiré".into()))?,
    };
    Ok(Json(
        state
            .ent
            .verifier_preinscription(etab, b.demande_id, &b.code)
            .await
            .map_err(map)?,
    ))
}

/// Variante passkey : une session ouverte dans l'établissement vaut vérification.
async fn jeton_session(State(state): State<AppState>, s: EntSession) -> Res<Json<JetonPreinscriptionDto>> {
    Ok(Json(
        state
            .ent
            .jeton_preinscription_depuis_session(s.etablissement_id, s.user_id)
            .await
            .map_err(map)?,
    ))
}

async fn espace(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
) -> Res<Json<EspacePreinscriptionDto>> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    Ok(Json(state.ent.espace_preinscription(etab, &j).await.map_err(map)?))
}

async fn deposer(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Json(b): Json<PreinscriptionBody>,
) -> Res<(StatusCode, Json<DossierDto>)> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    let d = state.ent.deposer_preinscription(etab, &j, &b).await.map_err(map)?;
    Ok((StatusCode::CREATED, Json(d)))
}

async fn dossier_famille(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Res<Json<DossierDto>> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    Ok(Json(state.ent.dossier_famille(etab, &j, id).await.map_err(map)?))
}

async fn piece_famille(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Path((id, code)): Path<(Uuid, String)>,
    Json(b): Json<PieceDeposeeBody>,
) -> Res<Json<DossierDto>> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    Ok(Json(
        state
            .ent
            .deposer_piece_famille(etab, &j, id, &code, &b.nom_fichier)
            .await
            .map_err(map)?,
    ))
}

async fn banque(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Res<Json<CoordonneesBancairesDto>> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    Ok(Json(
        state
            .ent
            .coordonnees_bancaires_famille(etab, &j, id)
            .await
            .map_err(map)?,
    ))
}

async fn mandat(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(b): Json<MandatBody>,
) -> Res<(StatusCode, Json<MandatDemandeDto>)> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    let m = state
        .ent
        .demander_mandat_famille(etab, &j, id, &b)
        .await
        .map_err(map)?;
    Ok((StatusCode::CREATED, Json(m)))
}

async fn signer_mandat(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Res<Json<MandatDemandeDto>> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    Ok(Json(
        state.ent.signer_mandat_famille(etab, &j, id).await.map_err(map)?,
    ))
}

// -- Imports -------------------------------------------------------------------

async fn previsualiser_import(
    State(state): State<AppState>,
    s: EntSession,
    Json(b): Json<ImportBody>,
) -> Res<(StatusCode, Json<LotImportDto>)> {
    let l = state
        .ent
        .previsualiser_import_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
        .await
        .map_err(map)?;
    Ok((StatusCode::CREATED, Json(l)))
}

async fn lot_import(State(state): State<AppState>, s: EntSession, Path(id): Path<Uuid>) -> Res<Json<LotImportDto>> {
    Ok(Json(
        state
            .ent
            .lot_import_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn appliquer_import(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<LotImportDto>> {
    Ok(Json(
        state
            .ent
            .appliquer_import_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn traiter_ligne(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TraiterLigneBody>,
) -> Res<Json<LigneImportDto>> {
    Ok(Json(
        state
            .ent
            .traiter_ligne_import_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}
