//! Routes du socle 32, lots ET0–ET1 : espaces de travail des personnels (espace personnel et espaces libres) — dossiers, fichiers à
//! envoi direct à taille signée, versions, corbeille, partages nominatifs, invitations, quota, journal.
//!
//! L'autorité est `ent-db` : chaque commande vérifie le droit D-017 (`espaces.utiliser`, `espaces.creer`, `espaces.partager`)
//! puis le **rôle d'espace** relu en base ; un espace, un dossier ou un fichier hors de portée répond **404** (jamais 403), y
//! compris pour un identifiant d'un autre établissement. Ces routes ne font que traduire HTTP ↔ service ; aucun octet de fichier
//! ne passe par l'API en production (envoi direct vers le magasin à taille signée, téléchargement par lien court de 60 s).
//! L'instant (`Utc::now()`) est lu ici et passé au service, qui n'a pas d'horloge propre.

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use chrono::{NaiveDate, Utc};
use ent_db::espaces::{
    ApercuCorbeilleDossier, Collegue, Contenu, DemandeDepot, DemandePartage, DepotAutorise, EspaceDetail, EspaceResume,
    InvitationsNonVues, LienTelechargement, LigneCorbeille, LigneJournal, ModifEspace, NouveauMembre, NouvelEspace, PartageRecu,
    PartageVue, QuotaEspace, ReponseDepot, ResultatDepot, VersionVue, DossierVue,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

/// Plafond du corps de l'envoi local : 64 Mio de fichier, plus une marge de protocole. Le jeton signe la taille exacte.
const CORPS_ENVOI_MAX: usize = 64 * 1024 * 1024 + 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Espaces, annuaire, badge, quota, partagés avec moi (ETR-106, 110, 111, 150 à 152, 260).
        .route("/espaces", get(lister).post(creer))
        .route("/espaces/mon-espace", post(ouvrir_mon_espace))
        .route("/espaces/collegues", get(collegues))
        .route("/espaces/quota", get(quota))
        .route("/espaces/invitations/non-vues", get(invitations_non_vues))
        .route("/espaces/invitations/vu", post(marquer_vu))
        .route("/espaces/partages/avec-moi", get(partages_avec_moi))
        // Partages nominatifs (ETR-130 à ETR-134).
        .route("/espaces/partages", get(partages_de).post(partager))
        .route("/espaces/partages/{partage_id}", delete(retirer_partage))
        .route("/espaces/partages/{partage_id}/fin", post(modifier_fin))
        .route("/espaces/partages/{partage_id}/quitter", post(quitter_partage))
        // Dépôt direct à taille signée (ETR-114, ETR-161) : l'envoi local n'existe qu'en labo.
        .route("/espaces/depots/{reservation_id}/finaliser", post(finaliser_depot))
        .route("/espaces/depots/{reservation_id}", delete(annuler_depot))
        .route("/espaces/envoi/{jeton}", put(recevoir_envoi_local).layer(DefaultBodyLimit::max(CORPS_ENVOI_MAX)))
        // Dossiers et éléments par identifiant (ETR-112, 118, 120, 122 à 125, 160 à 164).
        .route("/espaces/dossiers/{dossier_id}/renommer", post(renommer_dossier))
        .route("/espaces/dossiers/{dossier_id}/deplacer", post(deplacer_dossier))
        .route("/espaces/dossiers/{dossier_id}/corbeille", post(corbeille_dossier))
        .route("/espaces/elements/{element_id}/renommer", post(renommer_element))
        .route("/espaces/elements/{element_id}/deplacer", post(deplacer_element))
        .route("/espaces/elements/{element_id}/corbeille", post(corbeille_element))
        .route("/espaces/elements/{element_id}/versions", get(versions).post(demander_version))
        .route("/espaces/elements/{element_id}/versions/{numero}/restaurer", post(restaurer_version))
        .route("/espaces/elements/{element_id}/telechargement", get(telechargement))
        .route("/espaces/elements/{element_id}/apercu", get(apercu))
        .route("/espaces/corbeille/restaurer", post(restaurer))
        // Un espace par identifiant.
        .route("/espaces/{espace_id}", get(detail).patch(modifier))
        .route("/espaces/{espace_id}/contenu", get(contenu))
        .route("/espaces/{espace_id}/journal", get(journal))
        .route("/espaces/{espace_id}/membres", post(ajouter_membre))
        .route("/espaces/{espace_id}/membres/{personne_id}", delete(retirer_membre))
        .route("/espaces/{espace_id}/quitter", post(quitter))
        .route("/espaces/{espace_id}/transfert", post(transferer))
        .route("/espaces/{espace_id}/dossiers", post(creer_dossier))
        .route("/espaces/{espace_id}/depots", post(demander_depot))
        .route("/espaces/{espace_id}/corbeille", get(corbeille_lister).delete(vider_corbeille))
}

// ───────────────────────── Espaces ─────────────────────────

async fn lister(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<EspaceResume>>> {
    s.ent.espaces_lister(se.etablissement_id, se.user_id, &se.role, Utc::now()).await.map(Json).map_err(map_db)
}

async fn ouvrir_mon_espace(State(s): State<AppState>, se: EntSession) -> Res<Json<EspaceResume>> {
    s.ent.espaces_ouvrir_mon_espace(se.etablissement_id, se.user_id, &se.role, Utc::now()).await.map(Json).map_err(map_db)
}

async fn creer(State(s): State<AppState>, se: EntSession, Json(n): Json<NouvelEspace>) -> Res<(StatusCode, Json<EspaceResume>)> {
    s.ent
        .espaces_creer_libre(se.etablissement_id, se.user_id, &se.role, n, Utc::now())
        .await
        .map(|e| (StatusCode::CREATED, Json(e)))
        .map_err(map_db)
}

async fn detail(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<EspaceDetail>> {
    s.ent.espaces_detail(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map(Json).map_err(map_db)
}

async fn modifier(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>, Json(m): Json<ModifEspace>) -> Res<Json<EspaceResume>> {
    s.ent.espaces_modifier(se.etablissement_id, se.user_id, &se.role, id, m, Utc::now()).await.map(Json).map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct Recherche {
    q: String,
}

async fn collegues(State(s): State<AppState>, se: EntSession, Query(r): Query<Recherche>) -> Res<Json<Vec<Collegue>>> {
    s.ent.espaces_chercher_collegues(se.etablissement_id, se.user_id, &se.role, &r.q, Utc::now()).await.map(Json).map_err(map_db)
}

async fn quota(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<QuotaEspace>>> {
    s.ent.espaces_quota(se.etablissement_id, se.user_id, &se.role, Utc::now()).await.map(Json).map_err(map_db)
}

async fn invitations_non_vues(State(s): State<AppState>, se: EntSession) -> Res<Json<InvitationsNonVues>> {
    s.ent.espaces_invitations_non_vues(se.etablissement_id, se.user_id, &se.role, Utc::now()).await.map(Json).map_err(map_db)
}

#[derive(Debug, Deserialize, Default)]
struct Vu {
    #[serde(default)]
    espace_id: Option<Uuid>,
    #[serde(default)]
    partage_id: Option<Uuid>,
}

async fn marquer_vu(State(s): State<AppState>, se: EntSession, body: Option<Json<Vu>>) -> Res<Json<i64>> {
    let v = body.map(|Json(v)| v).unwrap_or_default();
    s.ent
        .espaces_marquer_vu(se.etablissement_id, se.user_id, &se.role, v.espace_id, v.partage_id, Utc::now())
        .await
        .map(Json)
        .map_err(map_db)
}

async fn journal(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>, Query(q): Query<Limite>) -> Res<Json<Vec<LigneJournal>>> {
    s.ent
        .espaces_journal(se.etablissement_id, se.user_id, &se.role, id, q.limite.unwrap_or(100), Utc::now())
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct Limite {
    #[serde(default)]
    limite: Option<i64>,
}

// ───────────────────────── Membres ─────────────────────────

async fn ajouter_membre(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>, Json(m): Json<NouveauMembre>) -> Res<StatusCode> {
    s.ent
        .espaces_membre_ajouter(se.etablissement_id, se.user_id, &se.role, id, m, Utc::now())
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn retirer_membre(State(s): State<AppState>, se: EntSession, Path((id, personne)): Path<(Uuid, Uuid)>) -> Res<StatusCode> {
    s.ent
        .espaces_membre_retirer(se.etablissement_id, se.user_id, &se.role, id, personne, Utc::now())
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn quitter(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    s.ent.espaces_quitter(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map(|_| StatusCode::NO_CONTENT).map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct Transfert {
    vers_personne_id: Uuid,
}

async fn transferer(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>, Json(t): Json<Transfert>) -> Res<StatusCode> {
    s.ent
        .espaces_transferer_animation(se.etablissement_id, se.user_id, &se.role, id, t.vers_personne_id, Utc::now())
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

// ───────────────────────── Dossiers et contenu ─────────────────────────

#[derive(Debug, Deserialize)]
struct DossierQuery {
    #[serde(default)]
    dossier_id: Option<Uuid>,
}

async fn contenu(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>, Query(q): Query<DossierQuery>) -> Res<Json<Contenu>> {
    s.ent.espaces_parcourir(se.etablissement_id, se.user_id, &se.role, id, q.dossier_id, Utc::now()).await.map(Json).map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct NouveauDossier {
    #[serde(default)]
    parent_id: Option<Uuid>,
    nom: String,
}

async fn creer_dossier(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(n): Json<NouveauDossier>,
) -> Res<(StatusCode, Json<DossierVue>)> {
    s.ent
        .espaces_dossier_creer(se.etablissement_id, se.user_id, &se.role, id, n.parent_id, &n.nom, Utc::now())
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct Renommage {
    nom: String,
}

#[derive(Debug, Deserialize)]
struct Deplacement {
    #[serde(default)]
    vers: Option<Uuid>,
}

async fn renommer_dossier(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>, Json(r): Json<Renommage>) -> Res<StatusCode> {
    s.ent
        .espaces_dossier_renommer(se.etablissement_id, se.user_id, &se.role, id, &r.nom, Utc::now())
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn deplacer_dossier(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>, Json(d): Json<Deplacement>) -> Res<StatusCode> {
    s.ent
        .espaces_dossier_deplacer(se.etablissement_id, se.user_id, &se.role, id, d.vers, Utc::now())
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

#[derive(Debug, Deserialize, Default)]
struct Confirmation {
    #[serde(default)]
    confirmer: bool,
}

async fn corbeille_dossier(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<Confirmation>>,
) -> Res<Json<ApercuCorbeilleDossier>> {
    let confirmer = body.map(|Json(c)| c.confirmer).unwrap_or(false);
    s.ent
        .espaces_dossier_corbeille(se.etablissement_id, se.user_id, &se.role, id, confirmer, Utc::now())
        .await
        .map(Json)
        .map_err(map_db)
}

async fn renommer_element(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>, Json(r): Json<Renommage>) -> Res<StatusCode> {
    s.ent
        .espaces_element_renommer(se.etablissement_id, se.user_id, &se.role, id, &r.nom, Utc::now())
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn deplacer_element(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>, Json(d): Json<Deplacement>) -> Res<StatusCode> {
    s.ent
        .espaces_element_deplacer(se.etablissement_id, se.user_id, &se.role, id, d.vers, Utc::now())
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn corbeille_element(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    s.ent
        .espaces_element_corbeille(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

// ───────────────────────── Dépôts, versions, téléchargement ─────────────────────────

/// Demande de dépôt : 200 avec l'URL d'envoi à taille signée, ou **409** « nom déjà pris » avec la proposition (jamais d'écrasement).
async fn demander_depot(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>, Json(d): Json<DemandeDepot>) -> Res<Response> {
    let r = s.ent.espaces_demander_depot(se.etablissement_id, se.user_id, &se.role, id, d, Utc::now()).await.map_err(map_db)?;
    Ok(match r {
        ReponseDepot::Autorise(_) => (StatusCode::OK, Json(r)).into_response(),
        ReponseDepot::NomDejaPris { .. } => (StatusCode::CONFLICT, Json(r)).into_response(),
    })
}

#[derive(Debug, Deserialize)]
struct DemandeVersion {
    taille: i64,
    #[serde(default)]
    mime: Option<String>,
    #[serde(default)]
    commentaire: Option<String>,
}

async fn demander_version(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(d): Json<DemandeVersion>,
) -> Res<Json<DepotAutorise>> {
    s.ent
        .espaces_demander_version(se.etablissement_id, se.user_id, &se.role, id, d.taille, d.mime, d.commentaire, Utc::now())
        .await
        .map(Json)
        .map_err(map_db)
}

async fn finaliser_depot(State(s): State<AppState>, se: EntSession, Path(reservation): Path<Uuid>) -> Res<Json<ResultatDepot>> {
    s.ent.espaces_finaliser_depot(se.etablissement_id, se.user_id, &se.role, reservation, Utc::now()).await.map(Json).map_err(map_db)
}

async fn annuler_depot(State(s): State<AppState>, se: EntSession, Path(reservation): Path<Uuid>) -> Res<StatusCode> {
    s.ent
        .espaces_annuler_depot(se.etablissement_id, se.user_id, &se.role, reservation, Utc::now())
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

/// Envoi en **mode local** (labo) : jeton signé (taille comprise) lié à la session ; le corps doit avoir exactement la taille
/// signée (en-tête `Content-Length` compris). En production, le navigateur envoie directement au magasin S3 (taille signée).
async fn recevoir_envoi_local(
    State(s): State<AppState>,
    se: EntSession,
    Path(jeton): Path<String>,
    headers: axum::http::HeaderMap,
    corps: Bytes,
) -> Res<StatusCode> {
    use ent_db::presign::{now_unix, PresignError};
    let claims = s.ent.presigner().verify_upload_local(&jeton, now_unix()).map_err(|e| match e {
        PresignError::Expired => (StatusCode::GONE, "Lien d'envoi expiré.".to_string()),
        PresignError::Disabled => (StatusCode::NOT_FOUND, "Introuvable.".to_string()),
        _ => (StatusCode::FORBIDDEN, "Lien d'envoi invalide.".to_string()),
    })?;
    if claims.user_id != se.user_id || claims.etablissement_id != se.etablissement_id {
        return Err((StatusCode::FORBIDDEN, "Lien d'envoi invalide.".into()));
    }
    // L'en-tête annoncé doit égaler la taille signée (comme le fait le fournisseur S3).
    let annonce = headers.get(axum::http::header::CONTENT_LENGTH).and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
    if annonce != Some(claims.taille) {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "La taille annoncée ne correspond pas à la taille signée.".into()));
    }
    s.ent
        .espaces_recevoir_envoi_local(se.etablissement_id, se.user_id, &claims.key, claims.taille, corps.to_vec(), Utc::now())
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn versions(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<VersionVue>>> {
    s.ent.espaces_versions(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map(Json).map_err(map_db)
}

async fn restaurer_version(State(s): State<AppState>, se: EntSession, Path((id, numero)): Path<(Uuid, i32)>) -> Res<Json<ResultatDepot>> {
    s.ent
        .espaces_version_restaurer(se.etablissement_id, se.user_id, &se.role, id, numero, Utc::now())
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct VersionQuery {
    #[serde(default)]
    version: Option<i32>,
}

async fn telechargement(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<VersionQuery>,
) -> Res<Json<LienTelechargement>> {
    s.ent
        .espaces_lien_telechargement(se.etablissement_id, se.user_id, &se.role, id, q.version, Utc::now())
        .await
        .map(Json)
        .map_err(map_db)
}

/// Aperçu minimal (ET1) : lien court servi en ligne pour les types sûrs ; les autres restent en téléchargement.
async fn apercu(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<VersionQuery>,
) -> Res<Json<LienTelechargement>> {
    s.ent
        .espaces_lien_apercu(se.etablissement_id, se.user_id, &se.role, id, q.version, Utc::now())
        .await
        .map(Json)
        .map_err(map_db)
}

// ───────────────────────── Corbeille ─────────────────────────

async fn corbeille_lister(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<LigneCorbeille>>> {
    s.ent.espaces_corbeille_lister(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map(Json).map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct Restauration {
    genre: String,
    id: Uuid,
}

async fn restaurer(State(s): State<AppState>, se: EntSession, Json(r): Json<Restauration>) -> Res<StatusCode> {
    s.ent
        .espaces_restaurer(se.etablissement_id, se.user_id, &se.role, &r.genre, r.id, Utc::now())
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn vider_corbeille(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<(usize, usize)>> {
    s.ent.espaces_vider_corbeille(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map(Json).map_err(map_db)
}

// ───────────────────────── Partages ─────────────────────────

async fn partager(State(s): State<AppState>, se: EntSession, Json(d): Json<DemandePartage>) -> Res<(StatusCode, Json<Vec<PartageVue>>)> {
    s.ent
        .espaces_partager(se.etablissement_id, se.user_id, &se.role, d, Utc::now())
        .await
        .map(|v| (StatusCode::CREATED, Json(v)))
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct CibleQuery {
    #[serde(default)]
    element_id: Option<Uuid>,
    #[serde(default)]
    dossier_id: Option<Uuid>,
}

async fn partages_de(State(s): State<AppState>, se: EntSession, Query(q): Query<CibleQuery>) -> Res<Json<Vec<PartageVue>>> {
    s.ent
        .espaces_partages_de(se.etablissement_id, se.user_id, &se.role, q.element_id, q.dossier_id, Utc::now())
        .await
        .map(Json)
        .map_err(map_db)
}

async fn partages_avec_moi(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<PartageRecu>>> {
    s.ent.espaces_partages_avec_moi(se.etablissement_id, se.user_id, &se.role, Utc::now()).await.map(Json).map_err(map_db)
}

async fn retirer_partage(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    s.ent.espaces_partage_retirer(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map(|_| StatusCode::NO_CONTENT).map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct FinPartage {
    #[serde(default)]
    fin: Option<NaiveDate>,
}

async fn modifier_fin(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>, Json(f): Json<FinPartage>) -> Res<StatusCode> {
    s.ent
        .espaces_partage_modifier_fin(se.etablissement_id, se.user_id, &se.role, id, f.fin, Utc::now())
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn quitter_partage(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    s.ent.espaces_partage_quitter(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map(|_| StatusCode::NO_CONTENT).map_err(map_db)
}
