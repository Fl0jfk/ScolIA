//! Routes `/voyages/*` du lot V0d (socle 42) : restauration, journal de voyage des familles, reprise des
//! séjours de Next.js.
//!
//! Aucune règle ici : droits, états, cloisonnement, journal et événements sont dans `ent-db`
//! (`voyages_restauration_repo.rs`, `voyages_journal_repo.rs`, `voyages_reprise_repo.rs`). Aucune route
//! n'accepte une adresse de destinataire (VD-06). La route de lecture **par lien nominatif** est publique
//! (pas de session) : elle répond `noindex`, sans cache, sans référent (VD-10).

use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use base64::Engine;
use ent_db::upload::{PolitiqueFichiers, UsageFichier};
use ent_db::voyages_journal_repo::{JournalFamille, JournalVue, PhotoSaisie, PublicationSaisie, PublicationVue};
use ent_db::voyages_reprise_repo::{
    EcartVue, LigneBascule, ParamsReprise, ParticipantReprise, RapportReprise, SejourImport,
};
use ent_db::voyages_restauration_repo::{AttributionSaisie, AvisApercu, RepasSaisie, RepasVue};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    let limite = DefaultBodyLimit::max(PolitiqueFichiers::depuis_env().limite_corps_base64(UsageFichier::Photo) * 4);
    Router::new()
        // Restauration
        .route("/voyages/reglage-restauration", put(reglage_restauration))
        .route("/voyages/{id}/restauration", get(repas_lire).put(repas_ecrire))
        .route("/voyages/{id}/restauration/apercu", get(repas_apercu))
        .route("/voyages/{id}/restauration/attributions", post(repas_attribuer))
        .route("/voyages/{id}/restauration/attributions/{aid}/retirer", post(repas_attribution_retirer))
        .route("/voyages/{id}/restauration/commande", post(repas_passer))
        .route("/voyages/{id}/restauration/avenants", post(repas_avenant))
        .route("/voyages/restauration/avis/traiter", post(repas_avis_traiter))
        // Journal de voyage : gestion
        .route("/voyages/{id}/journal", get(journal_lire))
        .route("/voyages/{id}/journal/activer", post(journal_activer))
        .route("/voyages/{id}/journal/fermer", post(journal_fermer))
        .route("/voyages/{id}/journal/delegues", put(journal_delegues))
        .route("/voyages/{id}/journal/publications", post(journal_publier).layer(limite))
        .route("/voyages/{id}/journal/publications/{pid}/retirer", post(journal_retirer))
        .route("/voyages/{id}/journal/publications/{pid}/bilan", put(journal_garder))
        .route("/voyages/{id}/journal/liens/revoquer", post(journal_liens_revoquer))
        .route("/voyages/journal/purger", post(journal_purger))
        // Journal de voyage : familles
        .route("/voyages/journal-famille", get(journal_famille))
        .route("/voyages/{id}/journal/photos/{phid}", get(journal_photo))
        .route("/voyages/journal/lien/{jeton}", get(journal_par_lien))
        .route("/voyages/journal/lien/{jeton}/photos/{phid}", get(journal_photo_par_lien))
        // Reprise de Next.js
        .route("/voyages/reprise/importer", post(reprise_importer))
        .route("/voyages/reprise/tableau", get(reprise_tableau))
        .route("/voyages/reprise/{rid}/ecarts", get(reprise_ecarts))
        .route("/voyages/reprise/{rid}/a-rapprocher", get(reprise_a_rapprocher))
        .route("/voyages/reprise/{rid}/rapprocher", post(reprise_rapprocher))
        .route("/voyages/reprise/{rid}/porteur", put(reprise_porteur))
        .route("/voyages/reprise/{rid}/pieces", put(reprise_pieces))
        .route("/voyages/reprise/ecarts/{eid}/traiter", post(reprise_ecart_traiter))
        .route("/voyages/reprise/{rid}/deroger", post(reprise_deroger))
        .route("/voyages/reprise/{rid}/basculer", post(reprise_basculer))
        .route("/voyages/reprise/{rid}/retour-arriere", post(reprise_retour_arriere))
}

// ───────────────────────── Restauration ─────────────────────────

#[derive(Deserialize)]
struct ReglageRestaurationBody {
    #[serde(default)]
    boite: Option<String>,
}

async fn reglage_restauration(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ReglageRestaurationBody>,
) -> Res<StatusCode> {
    s.ent.voyages_reglage_restauration_ecrire(&acteur(&session), b.boite.as_deref()).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn repas_lire(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<RepasVue>> {
    Ok(Json(s.ent.voyage_repas_lire(&acteur(&session), id).await.map_err(map_db)?))
}

async fn repas_ecrire(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RepasSaisie>,
) -> Res<Json<RepasVue>> {
    Ok(Json(s.ent.voyage_repas_ecrire(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn repas_apercu(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<AvisApercu>> {
    Ok(Json(s.ent.voyage_repas_apercu(&acteur(&session), id).await.map_err(map_db)?))
}

async fn repas_attribuer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AttributionSaisie>,
) -> Res<Json<RepasVue>> {
    Ok(Json(s.ent.voyage_repas_attribuer(&acteur(&session), id, b).await.map_err(map_db)?))
}

#[derive(Deserialize)]
struct RevBody {
    rev: i32,
}

async fn repas_attribution_retirer(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, aid)): Path<(Uuid, Uuid)>,
    Json(b): Json<RevBody>,
) -> Res<Json<RepasVue>> {
    Ok(Json(s.ent.voyage_repas_attribution_retirer(&acteur(&session), id, aid, b.rev).await.map_err(map_db)?))
}

#[derive(Deserialize)]
struct PasserBody {
    rev: i32,
    #[serde(default)]
    confirme: bool,
}

async fn repas_passer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PasserBody>,
) -> Res<Json<RepasVue>> {
    Ok(Json(s.ent.voyage_repas_passer(&acteur(&session), id, b.rev, b.confirme).await.map_err(map_db)?))
}

#[derive(Deserialize)]
struct AvenantBody {
    rev: i32,
    motif: String,
    #[serde(default)]
    confirme: bool,
}

async fn repas_avenant(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AvenantBody>,
) -> Res<Json<RepasVue>> {
    Ok(Json(
        s.ent.voyage_repas_avenant_emettre(&acteur(&session), id, b.rev, &b.motif, b.confirme).await.map_err(map_db)?,
    ))
}

/// Relance de l'envoi des avis en file (direction : `voyages.autoriser`) ; aucun ordonnanceur n'est branché.
async fn repas_avis_traiter(State(s): State<AppState>, session: EntSession) -> Res<Json<serde_json::Value>> {
    let a = acteur(&session);
    s.ent.voyages_exiger_direction(&a).await.map_err(map_db)?;
    let n = s.ent.voyages_repas_avis_traiter(a.etablissement_id, 50).await.map_err(map_db)?;
    Ok(Json(serde_json::json!({ "partis": n })))
}

// ───────────────────────── Journal de voyage : gestion ─────────────────────────

async fn journal_lire(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<JournalVue>> {
    Ok(Json(s.ent.voyage_journal_lire(&acteur(&session), id).await.map_err(map_db)?))
}

async fn journal_activer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<JournalVue>> {
    Ok(Json(s.ent.voyage_journal_activer(&acteur(&session), id).await.map_err(map_db)?))
}

async fn journal_fermer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<JournalVue>> {
    Ok(Json(s.ent.voyage_journal_fermer(&acteur(&session), id).await.map_err(map_db)?))
}

#[derive(Deserialize)]
struct DeleguesBody {
    encadrants: Vec<Uuid>,
}

async fn journal_delegues(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DeleguesBody>,
) -> Res<Json<JournalVue>> {
    Ok(Json(s.ent.voyage_journal_delegues_definir(&acteur(&session), id, b.encadrants).await.map_err(map_db)?))
}

#[derive(Deserialize)]
struct PhotoBody {
    nom: String,
    base64: String,
    /// `eleve_id` des élèves visibles sur la photo ; vide = « aucun élève identifiable » (à déclarer).
    #[serde(default)]
    eleves: Vec<Uuid>,
}

#[derive(Deserialize)]
struct PublicationBody {
    #[serde(rename = "type")]
    type_: String,
    #[serde(default)]
    nature_point: Option<String>,
    #[serde(default)]
    texte: String,
    #[serde(default)]
    aucun_eleve_identifiable: bool,
    #[serde(default)]
    photos: Vec<PhotoBody>,
}

async fn journal_publier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PublicationBody>,
) -> Res<(StatusCode, Json<PublicationVue>)> {
    let mut photos = Vec::new();
    for p in b.photos {
        let octets = base64::engine::general_purpose::STANDARD
            .decode(p.base64.trim())
            .map_err(|_| (StatusCode::BAD_REQUEST, "photo base64 invalide".to_string()))?;
        photos.push(PhotoSaisie { nom: p.nom, mime: None, contenu: octets, eleves: p.eleves });
    }
    let v = s
        .ent
        .voyage_journal_publier(
            &acteur(&session),
            id,
            PublicationSaisie {
                type_: b.type_,
                nature_point: b.nature_point,
                texte: b.texte,
                aucun_eleve_identifiable: b.aucun_eleve_identifiable,
                photos,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn journal_retirer(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, pid)): Path<(Uuid, Uuid)>,
) -> Res<StatusCode> {
    s.ent.voyage_journal_publication_retirer(&acteur(&session), id, pid).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct GarderBody {
    garder: bool,
}

async fn journal_garder(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, pid)): Path<(Uuid, Uuid)>,
    Json(b): Json<GarderBody>,
) -> Res<StatusCode> {
    s.ent.voyage_journal_publication_garder(&acteur(&session), id, pid, b.garder).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct RevoquerBody {
    #[serde(default)]
    personne_id: Option<Uuid>,
}

async fn journal_liens_revoquer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RevoquerBody>,
) -> Res<Json<serde_json::Value>> {
    let n = s.ent.voyage_journal_liens_revoquer(&acteur(&session), id, b.personne_id).await.map_err(map_db)?;
    Ok(Json(serde_json::json!({ "revoques": n })))
}

/// Purge (fin du séjour + 15 jours) : direction (`voyages.autoriser`) ; aucun ordonnanceur n'est branché.
async fn journal_purger(State(s): State<AppState>, session: EntSession) -> Res<Json<serde_json::Value>> {
    let a = acteur(&session);
    s.ent.voyages_exiger_direction(&a).await.map_err(map_db)?;
    let n = s.ent.voyages_journal_purger(a.etablissement_id).await.map_err(map_db)?;
    Ok(Json(serde_json::json!({ "detruites": n })))
}

// ───────────────────────── Journal de voyage : familles ─────────────────────────

async fn journal_famille(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<JournalFamille>>> {
    Ok(Json(s.ent.voyages_journal_famille(&acteur(&session)).await.map_err(map_db)?))
}

async fn journal_photo(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, phid)): Path<(Uuid, Uuid)>,
) -> Res<Response> {
    let (mime, octets) = s.ent.voyage_journal_photo(&acteur(&session), id, phid).await.map_err(map_db)?;
    crate::routes::attachment_response(octets, "photo", mime_statique(&mime))
}

/// Types d'image servis : la base n'en accepte que trois (contrainte `mime`).
fn mime_statique(m: &str) -> &'static str {
    match m {
        "image/png" => "image/png",
        "image/webp" => "image/webp",
        _ => "image/jpeg",
    }
}

/// Entêtes d'une réponse par lien nominatif : jamais indexée, jamais mise en cache, sans référent.
fn prive(mut r: Response) -> Response {
    let h = r.headers_mut();
    h.insert("x-robots-tag", HeaderValue::from_static("noindex, nofollow, noarchive"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    h.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    r
}

/// Lecture par lien nominatif (famille sans compte) : route PUBLIQUE (pas de session). Un jeton inconnu,
/// expiré, révoqué ou d'un élève qui ne part plus répond 404, comme tout jeton mal formé.
async fn journal_par_lien(State(s): State<AppState>, Path(jeton): Path<String>) -> Res<Response> {
    let j = s.ent.voyage_journal_par_lien(&jeton).await.map_err(map_db)?;
    Ok(prive(Json(j).into_response()))
}

async fn journal_photo_par_lien(
    State(s): State<AppState>,
    Path((jeton, phid)): Path<(String, Uuid)>,
) -> Res<Response> {
    let (mime, octets) = s.ent.voyage_journal_photo_par_lien(&jeton, phid).await.map_err(map_db)?;
    Ok(prive(crate::routes::attachment_response(octets, "photo", mime_statique(&mime))?))
}

// ───────────────────────── Reprise de Next.js ─────────────────────────

#[derive(Deserialize)]
struct ImportBody {
    params: ParamsReprise,
    sejours: Vec<SejourImport>,
}

async fn reprise_importer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ImportBody>,
) -> Res<Json<RapportReprise>> {
    Ok(Json(s.ent.voyages_reprise_importer(&acteur(&session), b.params, b.sejours).await.map_err(map_db)?))
}

async fn reprise_tableau(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<LigneBascule>>> {
    Ok(Json(s.ent.voyages_reprise_tableau(&acteur(&session)).await.map_err(map_db)?))
}

async fn reprise_ecarts(State(s): State<AppState>, session: EntSession, Path(rid): Path<Uuid>) -> Res<Json<Vec<EcartVue>>> {
    Ok(Json(s.ent.voyages_reprise_ecarts(&acteur(&session), rid).await.map_err(map_db)?))
}

async fn reprise_a_rapprocher(
    State(s): State<AppState>,
    session: EntSession,
    Path(rid): Path<Uuid>,
) -> Res<Json<Vec<ParticipantReprise>>> {
    Ok(Json(s.ent.voyages_reprise_a_rapprocher(&acteur(&session), rid).await.map_err(map_db)?))
}

#[derive(Deserialize)]
struct RapprocherBody {
    participant_source_id: String,
    eleve_id: Uuid,
}

async fn reprise_rapprocher(
    State(s): State<AppState>,
    session: EntSession,
    Path(rid): Path<Uuid>,
    Json(b): Json<RapprocherBody>,
) -> Res<StatusCode> {
    s.ent.voyages_reprise_rapprocher(&acteur(&session), rid, &b.participant_source_id, b.eleve_id).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct PorteurBody {
    personne_id: Uuid,
}

async fn reprise_porteur(
    State(s): State<AppState>,
    session: EntSession,
    Path(rid): Path<Uuid>,
    Json(b): Json<PorteurBody>,
) -> Res<StatusCode> {
    s.ent.voyages_reprise_porteur_definir(&acteur(&session), rid, b.personne_id).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct PiecesBody {
    copiees: i32,
}

async fn reprise_pieces(
    State(s): State<AppState>,
    session: EntSession,
    Path(rid): Path<Uuid>,
    Json(b): Json<PiecesBody>,
) -> Res<StatusCode> {
    s.ent.voyages_reprise_pieces_definir(&acteur(&session), rid, b.copiees).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reprise_ecart_traiter(
    State(s): State<AppState>,
    session: EntSession,
    Path(eid): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent.voyages_reprise_ecart_traiter(&acteur(&session), eid).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct DerogerBody {
    motif: String,
}

async fn reprise_deroger(
    State(s): State<AppState>,
    session: EntSession,
    Path(rid): Path<Uuid>,
    Json(b): Json<DerogerBody>,
) -> Res<StatusCode> {
    s.ent.voyages_reprise_deroger(&acteur(&session), rid, &b.motif).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct BasculerBody {
    #[serde(default)]
    confirme: bool,
}

async fn reprise_basculer(
    State(s): State<AppState>,
    session: EntSession,
    Path(rid): Path<Uuid>,
    Json(b): Json<BasculerBody>,
) -> Res<Json<serde_json::Value>> {
    let sortie = s.ent.voyages_reprise_basculer(&acteur(&session), rid, b.confirme).await.map_err(map_db)?;
    Ok(Json(serde_json::json!({ "sortie_id": sortie })))
}

async fn reprise_retour_arriere(
    State(s): State<AppState>,
    session: EntSession,
    Path(rid): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent.voyages_reprise_retour_arriere(&acteur(&session), rid).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}
