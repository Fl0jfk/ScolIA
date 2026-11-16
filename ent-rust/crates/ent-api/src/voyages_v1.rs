//! Routes `/voyages/*` du lot V1 (socle 42) : campagne d'autorisations, exigences par participant, guichet,
//! attestations de reprise, désaccords, dérogations, relances, ordres de mission, liste nominative.
//!
//! Aucune règle ici : droits, états, cloisonnement, journal et événements sont dans `ent-db`
//! (`voyages_exigences_repo.rs`, `voyages_missions_repo.rs`). Aucune route n'accepte une adresse de
//! destinataire ni la liste des destinataires d'une relance : le serveur les calcule. Le **port d'entrée du
//! socle 18** (`voyages_autorisation_decision_recue`) n'est **pas** exposé en HTTP : c'est un appel de service.

use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use chrono::NaiveDate;
use ent_db::upload::{PolitiqueFichiers, UsageFichier};
use ent_db::voyages_exigences_repo::{
    ApercuRelance, AttestationSaisie, CampagneSaisie, GuichetSaisie, Matrice, ScanSaisie,
};
use ent_db::voyages_missions_repo::{DocumentAdulte, TransmissionSaisie, TransmissionVue};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    let limite = DefaultBodyLimit::max(PolitiqueFichiers::depuis_env().limite_corps_base64(UsageFichier::Document));
    Router::new()
        .route("/voyages/{id}/campagne", post(campagne_ouvrir))
        .route("/voyages/{id}/campagne/clore", post(campagne_clore))
        .route("/voyages/{id}/exigences", get(matrice))
        .route("/voyages/{id}/autorisations/guichet", post(guichet).layer(limite))
        .route("/voyages/{id}/autorisations/attestations", post(attestations))
        .route("/voyages/{id}/desaccords", post(desaccord_declarer))
        .route("/voyages/{id}/desaccords/{pid}/decision", post(desaccord_decider))
        .route("/voyages/{id}/exigences/{pid}/{code}/derogation", post(deroger))
        .route("/voyages/{id}/relance/apercu", get(relance_apercu))
        .route("/voyages/{id}/relance", post(relancer))
        .route("/voyages/relances/traiter", post(relances_traiter))
        .route("/voyages/{id}/ordres-mission", get(ordres_lister).post(ordres_emettre))
        .route("/voyages/{id}/ordres-mission/{did}", get(ordre_lire))
        .route("/voyages/{id}/transmissions", get(transmissions).post(transmission_demander))
        .route("/voyages/{id}/transmissions/{tid}/decision", post(transmission_decider))
}

async fn campagne_ouvrir(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CampagneSaisie>,
) -> Res<(StatusCode, Json<Matrice>)> {
    Ok((StatusCode::CREATED, Json(s.ent.voyage_campagne_ouvrir(&acteur(&session), id, b).await.map_err(map_db)?)))
}

async fn campagne_clore(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    s.ent.voyage_campagne_clore(&acteur(&session), id).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn matrice(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Matrice>> {
    Ok(Json(s.ent.voyage_exigences_matrice(&acteur(&session), id).await.map_err(map_db)?))
}

#[derive(Deserialize)]
struct GuichetBody {
    participant_id: Uuid,
    code: String,
    decision: String,
    signataire_personne_id: Uuid,
    /// Date portée par le coupon papier (jamais la date de saisie).
    signee_le: NaiveDate,
    #[serde(default)]
    scan_nom: Option<String>,
    #[serde(default)]
    scan_base64: Option<String>,
}

async fn guichet(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<GuichetBody>,
) -> Res<Json<Matrice>> {
    let scan = match b.scan_base64.as_deref() {
        None => None,
        Some(b64) => {
            let contenu = base64::engine::general_purpose::STANDARD
                .decode(b64.trim())
                .map_err(|_| (StatusCode::BAD_REQUEST, "scan_base64 invalide".to_string()))?;
            Some(ScanSaisie { nom: b.scan_nom.unwrap_or_else(|| "coupon.pdf".to_string()), mime: None, contenu })
        }
    };
    let g = GuichetSaisie {
        participant_id: b.participant_id,
        code: b.code,
        decision: b.decision,
        signataire_personne_id: b.signataire_personne_id,
        signee_le: b.signee_le,
        scan,
    };
    Ok(Json(s.ent.voyage_autorisation_guichet(&acteur(&session), id, g).await.map_err(map_db)?))
}

async fn attestations(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AttestationSaisie>,
) -> Res<Json<Matrice>> {
    Ok(Json(s.ent.voyage_attestations_masse(&acteur(&session), id, b).await.map_err(map_db)?))
}

#[derive(Deserialize)]
struct DesaccordBody {
    participant_id: Uuid,
    titulaire_personne_id: Uuid,
}

async fn desaccord_declarer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DesaccordBody>,
) -> Res<StatusCode> {
    s.ent.voyage_desaccord_declarer(&acteur(&session), id, b.participant_id, b.titulaire_personne_id).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct MotifBody {
    motif: String,
}

async fn desaccord_decider(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, pid)): Path<(Uuid, Uuid)>,
    Json(b): Json<MotifBody>,
) -> Res<StatusCode> {
    s.ent.voyage_desaccord_decider(&acteur(&session), id, pid, &b.motif).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn deroger(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, pid, code)): Path<(Uuid, Uuid, String)>,
    Json(b): Json<MotifBody>,
) -> Res<StatusCode> {
    s.ent.voyage_exigence_deroger(&acteur(&session), id, pid, &code, &b.motif).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn relance_apercu(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<ApercuRelance>> {
    Ok(Json(s.ent.voyage_relance_apercu(&acteur(&session), id).await.map_err(map_db)?))
}

#[derive(Deserialize)]
struct ConfirmeBody {
    #[serde(default)]
    confirme: bool,
}

async fn relancer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ConfirmeBody>,
) -> Res<Json<ApercuRelance>> {
    Ok(Json(s.ent.voyage_relancer_manquants(&acteur(&session), id, b.confirme).await.map_err(map_db)?))
}

/// Relances automatiques J-21, J-10, J-5 : direction (`voyages.autoriser`) ; aucun ordonnanceur n'est branché.
async fn relances_traiter(State(s): State<AppState>, session: EntSession) -> Res<Json<serde_json::Value>> {
    let a = acteur(&session);
    s.ent.voyages_exiger_direction(&a).await.map_err(map_db)?;
    let n = s.ent.voyages_relances_traiter(a.etablissement_id).await.map_err(map_db)?;
    Ok(Json(serde_json::json!({ "notifies": n })))
}

#[derive(Deserialize)]
struct OrdresBody {
    #[serde(default)]
    prise_en_charge: Option<String>,
}

async fn ordres_emettre(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<OrdresBody>,
) -> Res<(StatusCode, Json<Vec<DocumentAdulte>>)> {
    let v = s.ent.voyage_ordres_mission_emettre(&acteur(&session), id, b.prise_en_charge.as_deref()).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn ordres_lister(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<DocumentAdulte>>> {
    Ok(Json(s.ent.voyage_ordres_mission_lister(&acteur(&session), id).await.map_err(map_db)?))
}

async fn ordre_lire(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, did)): Path<(Uuid, Uuid)>,
) -> Res<axum::response::Response> {
    let (nom, octets) = s.ent.voyage_ordre_mission_lire(&acteur(&session), id, did).await.map_err(map_db)?;
    crate::routes::attachment_response(octets, &nom, "application/pdf")
}

async fn transmissions(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<TransmissionVue>>> {
    Ok(Json(s.ent.voyage_transmissions_lire(&acteur(&session), id).await.map_err(map_db)?))
}

async fn transmission_demander(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TransmissionSaisie>,
) -> Res<(StatusCode, Json<TransmissionVue>)> {
    Ok((StatusCode::CREATED, Json(s.ent.voyage_transmission_demander(&acteur(&session), id, b).await.map_err(map_db)?)))
}

#[derive(Deserialize)]
struct DecisionBody {
    accepter: bool,
    #[serde(default)]
    motif_refus: Option<String>,
}

async fn transmission_decider(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, tid)): Path<(Uuid, Uuid)>,
    Json(b): Json<DecisionBody>,
) -> Res<Json<TransmissionVue>> {
    Ok(Json(
        s.ent.voyage_transmission_decider(&acteur(&session), id, tid, b.accepter, b.motif_refus.as_deref()).await.map_err(map_db)?,
    ))
}
