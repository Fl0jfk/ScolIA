//! Routes `/voyages/*` du lot V0c (socle 42) : annuaire des prestataires, demandes de devis, devis,
//! commandes, avenants, boîte de réception des devis (file de tri).
//!
//! Aucune règle ici : droits, états, cloisonnement, journal et événements sont dans `ent-db`
//! (`voyages_prestataires_repo.rs`, `voyages_courrier_repo.rs`). **Aucune route n'accepte une adresse de
//! destinataire** : les envois visent une adresse de l'annuaire (corrige NJ-11). Le dépôt de courrier
//! entrant n'est **pas** exposé en HTTP (corrige NJ-28).

use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use base64::Engine;
use ent_db::upload::{PolitiqueFichiers, UsageFichier};
use ent_db::voyages_budget_repo::BudgetVue;
use ent_db::voyages_courrier_repo::{CourrierVue, PieceCourrierVue};
use ent_db::voyages_prestataires_repo::{
    AchatsVue, AvenantSaisie, CommandeVue, ConfirmationCommande, ContactsTransport, DemandeSaisie, DevisSaisie,
    DevisVue, EnvoiVue, PieceSaisie, PrestataireChoix, PrestataireSaisie, PrestataireVue,
};
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
        .route("/voyages/prestataires", get(prestataires).post(prestataire_creer))
        .route("/voyages/prestataires/{pid}", put(prestataire_modifier))
        .route("/voyages/prestataires/{pid}/adresses", post(adresse_ajouter))
        .route("/voyages/prestataires/{pid}/adresses/{aid}", delete(adresse_desactiver))
        .route("/voyages/reglage", put(reglage))
        .route("/voyages/envois/traiter", post(envois_traiter))
        .route("/voyages/courrier/tri", get(tri))
        .route("/voyages/courrier/tri/compte", get(tri_compte))
        .route("/voyages/courrier/{cid}/rattacher", post(rattacher))
        .route("/voyages/courrier/{cid}/ignorer", post(ignorer))
        .route("/voyages/courrier/{cid}/nouveau-prestataire", post(nouveau_prestataire))
        .route("/voyages/courrier/{cid}/pieces", get(courrier_pieces))
        .route("/voyages/courrier/{cid}/pieces/{pid}", get(courrier_piece))
        .route("/voyages/{id}/prestataires", get(choix))
        .route("/voyages/{id}/achats", get(achats))
        .route("/voyages/{id}/devis/demandes", post(demander))
        .route("/voyages/{id}/devis/relances", post(relancer))
        .route("/voyages/{id}/devis", post(devis_ajouter).layer(limite))
        .route("/voyages/{id}/devis/{did}/montant", post(devis_montant))
        .route("/voyages/{id}/devis/{did}/choisir", post(devis_choisir))
        .route("/voyages/{id}/devis/{did}/ecarter", post(devis_ecarter))
        .route("/voyages/{id}/devis/{did}/piece", get(devis_piece))
        .route("/voyages/{id}/budget/versions/{numero}/lignes-devis", post(ligne_devis))
        .route("/voyages/{id}/commandes-achat/{cid}/confirmer", post(commande_confirmer))
        .route("/voyages/{id}/commandes-achat/{cid}/avenants", post(avenant))
        .route("/voyages/{id}/contacts-transport", get(contacts))
}

async fn prestataires(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<PrestataireVue>>> {
    Ok(Json(s.ent.prestataires_liste(&acteur(&session)).await.map_err(map_db)?))
}

async fn prestataire_creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<PrestataireSaisie>,
) -> Res<(StatusCode, Json<PrestataireVue>)> {
    let v = s.ent.prestataire_creer(&acteur(&session), b).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn prestataire_modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(pid): Path<Uuid>,
    Json(b): Json<PrestataireSaisie>,
) -> Res<Json<PrestataireVue>> {
    Ok(Json(s.ent.prestataire_modifier(&acteur(&session), pid, b).await.map_err(map_db)?))
}

#[derive(Deserialize)]
struct AdresseBody {
    email: String,
}

async fn adresse_ajouter(
    State(s): State<AppState>,
    session: EntSession,
    Path(pid): Path<Uuid>,
    Json(b): Json<AdresseBody>,
) -> Res<Json<PrestataireVue>> {
    Ok(Json(s.ent.prestataire_adresse_ajouter(&acteur(&session), pid, &b.email).await.map_err(map_db)?))
}

async fn adresse_desactiver(
    State(s): State<AppState>,
    session: EntSession,
    Path((pid, aid)): Path<(Uuid, Uuid)>,
) -> Res<Json<PrestataireVue>> {
    Ok(Json(s.ent.prestataire_adresse_desactiver(&acteur(&session), pid, aid).await.map_err(map_db)?))
}

#[derive(Deserialize)]
struct ReglageBody {
    #[serde(default)]
    boite_comptabilite: Option<String>,
    #[serde(default)]
    pied_de_page: Option<String>,
}

async fn reglage(State(s): State<AppState>, session: EntSession, Json(b): Json<ReglageBody>) -> Res<StatusCode> {
    s.ent
        .sorties_reglage_ecrire(&acteur(&session), b.boite_comptabilite.as_deref(), b.pied_de_page.as_deref())
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Serialize)]
struct Traites {
    partis: usize,
}

async fn envois_traiter(State(s): State<AppState>, session: EntSession) -> Res<Json<Traites>> {
    Ok(Json(Traites { partis: s.ent.voyage_envois_file(&acteur(&session)).await.map_err(map_db)? }))
}

async fn tri(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<CourrierVue>>> {
    Ok(Json(s.ent.courrier_tri_liste(&acteur(&session)).await.map_err(map_db)?))
}

#[derive(serde::Serialize)]
struct Compte {
    a_trier: i64,
}

async fn tri_compte(State(s): State<AppState>, session: EntSession) -> Res<Json<Compte>> {
    Ok(Json(Compte { a_trier: s.ent.courrier_tri_compte(&acteur(&session)).await.map_err(map_db)? }))
}

#[derive(Deserialize)]
struct RattacherBody {
    sortie_id: Uuid,
    prestataire_id: Uuid,
}

async fn rattacher(
    State(s): State<AppState>,
    session: EntSession,
    Path(cid): Path<Uuid>,
    Json(b): Json<RattacherBody>,
) -> Res<(StatusCode, Json<serde_json::Value>)> {
    let devis = s
        .ent
        .courrier_rattacher(&acteur(&session), cid, b.sortie_id, b.prestataire_id)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "devis_id": devis }))))
}

#[derive(Deserialize)]
struct IgnorerBody {
    motif: String,
}

async fn ignorer(
    State(s): State<AppState>,
    session: EntSession,
    Path(cid): Path<Uuid>,
    Json(b): Json<IgnorerBody>,
) -> Res<StatusCode> {
    s.ent.courrier_ignorer(&acteur(&session), cid, &b.motif).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn nouveau_prestataire(
    State(s): State<AppState>,
    session: EntSession,
    Path(cid): Path<Uuid>,
    Json(b): Json<PrestataireSaisie>,
) -> Res<(StatusCode, Json<PrestataireVue>)> {
    let v = s.ent.courrier_nouveau_prestataire(&acteur(&session), cid, b).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn courrier_pieces(
    State(s): State<AppState>,
    session: EntSession,
    Path(cid): Path<Uuid>,
) -> Res<Json<Vec<PieceCourrierVue>>> {
    Ok(Json(s.ent.courrier_pieces(&acteur(&session), cid).await.map_err(map_db)?))
}

async fn courrier_piece(
    State(s): State<AppState>,
    session: EntSession,
    Path((cid, pid)): Path<(Uuid, Uuid)>,
) -> Res<axum::response::Response> {
    let (nom, mime, octets) = s.ent.courrier_piece_lire(&acteur(&session), cid, pid).await.map_err(map_db)?;
    // Pièce d'origine inconnue : toujours servie en téléchargement, jamais interprétée par le navigateur.
    let type_ = if mime == "application/pdf" { "application/pdf" } else { "application/octet-stream" };
    crate::routes::attachment_response(octets, &nom, type_)
}

async fn choix(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<PrestataireChoix>>> {
    Ok(Json(s.ent.voyage_prestataires_choix(&acteur(&session), id).await.map_err(map_db)?))
}

async fn achats(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<AchatsVue>> {
    Ok(Json(s.ent.voyage_achats(&acteur(&session), id).await.map_err(map_db)?))
}

async fn demander(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DemandeSaisie>,
) -> Res<(StatusCode, Json<Vec<EnvoiVue>>)> {
    let v = s.ent.voyage_devis_demander(&acteur(&session), id, b).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
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
) -> Res<(StatusCode, Json<Vec<EnvoiVue>>)> {
    let v = s.ent.voyage_devis_relancer(&acteur(&session), id, b.confirme).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

#[derive(Deserialize)]
struct DevisBody {
    #[serde(flatten)]
    devis: DevisSaisie,
    #[serde(default)]
    piece_nom: Option<String>,
    #[serde(default)]
    piece_base64: Option<String>,
}

async fn devis_ajouter(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DevisBody>,
) -> Res<(StatusCode, Json<DevisVue>)> {
    let mut d = b.devis;
    if let Some(b64) = b.piece_base64.as_deref() {
        let octets = base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .map_err(|_| (StatusCode::BAD_REQUEST, "piece_base64 invalide".to_string()))?;
        d.piece = Some(PieceSaisie {
            nom: b.piece_nom.unwrap_or_else(|| "devis.pdf".to_string()),
            contenu: octets,
            mime: None,
        });
    }
    let v = s.ent.voyage_devis_ajouter(&acteur(&session), id, d).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

#[derive(Deserialize)]
struct MontantBody {
    rev: i32,
    montant: String,
}

async fn devis_montant(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, did)): Path<(Uuid, Uuid)>,
    Json(b): Json<MontantBody>,
) -> Res<Json<DevisVue>> {
    Ok(Json(s.ent.voyage_devis_confirmer_montant(&acteur(&session), id, did, b.rev, &b.montant).await.map_err(map_db)?))
}

#[derive(Deserialize)]
struct RevBody {
    rev: i32,
}

async fn devis_choisir(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, did)): Path<(Uuid, Uuid)>,
    Json(b): Json<RevBody>,
) -> Res<Json<Vec<DevisVue>>> {
    Ok(Json(s.ent.voyage_devis_choisir(&acteur(&session), id, did, b.rev).await.map_err(map_db)?))
}

async fn devis_ecarter(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, did)): Path<(Uuid, Uuid)>,
) -> Res<Json<Vec<DevisVue>>> {
    Ok(Json(s.ent.voyage_devis_ecarter(&acteur(&session), id, did).await.map_err(map_db)?))
}

async fn devis_piece(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, did)): Path<(Uuid, Uuid)>,
) -> Res<axum::response::Response> {
    let (nom, octets) = s.ent.voyage_devis_piece_lire(&acteur(&session), id, did).await.map_err(map_db)?;
    crate::routes::attachment_response(octets, &nom, "application/pdf")
}

#[derive(Deserialize)]
struct LigneDevisBody {
    rev: i32,
    devis_id: Uuid,
}

async fn ligne_devis(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, numero)): Path<(Uuid, i32)>,
    Json(b): Json<LigneDevisBody>,
) -> Res<Json<BudgetVue>> {
    Ok(Json(
        s.ent
            .voyage_budget_ligne_depuis_devis(&acteur(&session), id, numero, b.rev, b.devis_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn commande_confirmer(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, cid)): Path<(Uuid, Uuid)>,
    Json(b): Json<ConfirmationCommande>,
) -> Res<Json<CommandeVue>> {
    Ok(Json(s.ent.voyage_commande_confirmer(&acteur(&session), id, cid, b).await.map_err(map_db)?))
}

async fn avenant(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, cid)): Path<(Uuid, Uuid)>,
    Json(b): Json<AvenantSaisie>,
) -> Res<(StatusCode, Json<CommandeVue>)> {
    let v = s.ent.voyage_avenant_emettre(&acteur(&session), id, cid, b).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn contacts(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<ContactsTransport>>> {
    Ok(Json(s.ent.voyage_contacts_transport(&acteur(&session), id).await.map_err(map_db)?))
}
