//! Routes de l'assiduité (socle 12, lots AS1 et AS2) : référentiels, règle d'assiduité datée, motifs,
//! activités appelées, appel v2 (activités, pointage, validation, corrections, délégation,
//! exclusion de cours), situations connues (localisation, retard annoncé).
//!
//! L'autorité est `ent-db` (moteur de droits D-017 : `appel.faire`, `appel.faire_tout`,
//! `appel.corriger`, `appel.deleguer`, `absences.lire_motif`, `localisation.poser`,
//! `assiduite.regle_gerer`, `absences.declarer`) ; ces routes ne font que traduire HTTP ↔ service.
//! **Aucun contrôle de rôle codé en dur ici** (DA-18) : une route ne répond que ce que le service
//! autorise, et une référence d'un autre établissement ou inexistante donne la même réponse (R-AS35).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_db::{
    ActiviteAppelee, AnnoncerRetard, AppelV2, CorrectionLigne, HistoLigne, IncoherenceV2,
    LigneAppelV2, MotifAsd, NatureMotif, OuvrirActivite, PoserLocalisation, ProfilAcademique,
    RegleAsd, SaisieLigne, SaisieMotif, SaisieRegle, SituationAsd, ValidationAppel, VersionMotif,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Référentiels, règle, motifs, activités (AS1)
        .route("/assiduite/natures-motif", get(natures))
        .route("/assiduite/profils-academiques", get(profils))
        .route("/assiduite/motifs", get(motifs).post(creer_motif))
        .route("/assiduite/motifs/{id}", put(modifier_motif))
        .route("/assiduite/motifs/{id}/versions", get(versions_motif))
        .route("/assiduite/motifs-famille", get(motifs_famille))
        .route("/assiduite/regles", get(regles).post(definir_regle))
        .route("/assiduite/regle-en-vigueur", get(regle_en_vigueur))
        .route("/assiduite/activites-appelees", get(activites).put(definir_activite))
        // Appel v2 (AS2)
        .route("/appels/activites", post(ouvrir_activite))
        .route("/appels/{id}/v2", get(appel_v2))
        .route("/appels/{id}/v2/lignes", post(pointer))
        .route("/appels/{id}/v2/valider", post(valider))
        .route("/appels/{id}/v2/corrections", post(corriger))
        .route("/appels/{id}/lignes/{eleve_id}/historique", get(historique))
        .route("/appels/{id}/delegation", post(deleguer).delete(revoquer))
        .route("/appels/{id}/exclusions", post(exclure))
        .route("/appels/{id}/arrivees-vs", post(arrivee_vs))
        .route("/appels/{id}/incoherences", get(incoherences))
        // Situations connues (AS2)
        .route("/assiduite/situations", get(situations_jour))
        .route("/assiduite/situations/localisation", post(localiser))
        .route("/assiduite/situations/retard-annonce", post(annoncer_retard))
        .route("/assiduite/situations/{id}/lever", post(lever_situation))
        .route("/assiduite/situations/{id}/texte", get(texte_situation))
}

/// Canal de saisie de la session : web ou mobile (le hors ligne est annoncé par le client, AS10).
fn canal(session: &EntSession) -> &'static str {
    match session.client {
        ent_db::ClientSession::Web => "web",
        _ => "mobile",
    }
}

async fn avec_noms(state: &AppState, etablissement_id: Uuid, mut a: AppelV2) -> Res<Json<AppelV2>> {
    let ids: Vec<Uuid> = a.lignes.iter().map(|l| l.eleve_id).collect();
    let noms = state.ent.eleve_labels(etablissement_id, &ids).await.map_err(map_db)?;
    for l in &mut a.lignes {
        l.eleve_label = noms.get(&l.eleve_id).cloned();
    }
    Ok(Json(a))
}

// ─── AS1 ─────────────────────────────────────────────────────────────────────

async fn natures(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<NatureMotif>>> {
    state.ent.natures_motif_for_actor(session.etablissement_id, session.user_id, &session.role).await.map(Json).map_err(map_db)
}

async fn profils(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<ProfilAcademique>>> {
    state.ent.profils_academiques_for_actor(session.etablissement_id, session.user_id, &session.role).await.map(Json).map_err(map_db)
}

async fn motifs(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<MotifAsd>>> {
    state.ent.motifs_for_actor(session.etablissement_id, session.user_id, &session.role).await.map(Json).map_err(map_db)
}

/// Motifs proposés à la famille : le service ne rend que les motifs actifs et visibles (jamais le
/// motif technique) ; réservé aux comptes famille et élève majeur, qui déclarent et justifient.
async fn motifs_famille(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<MotifAsd>>> {
    state.ent.motifs_famille(session.etablissement_id).await.map(Json).map_err(map_db)
}

async fn creer_motif(State(state): State<AppState>, session: EntSession, Json(s): Json<SaisieMotif>) -> Res<(StatusCode, Json<MotifAsd>)> {
    state.ent.creer_motif_for_actor(session.etablissement_id, session.user_id, &session.role, s).await.map(|m| (StatusCode::CREATED, Json(m))).map_err(map_db)
}

async fn modifier_motif(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(s): Json<SaisieMotif>) -> Res<Json<MotifAsd>> {
    state.ent.modifier_motif_for_actor(session.etablissement_id, session.user_id, &session.role, id, s).await.map(Json).map_err(map_db)
}

async fn versions_motif(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<VersionMotif>>> {
    state.ent.versions_motif_for_actor(session.etablissement_id, session.user_id, &session.role, id).await.map(Json).map_err(map_db)
}

async fn regles(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<RegleAsd>>> {
    state.ent.regles_asd_for_actor(session.etablissement_id, session.user_id, &session.role).await.map(Json).map_err(map_db)
}

async fn definir_regle(State(state): State<AppState>, session: EntSession, Json(s): Json<SaisieRegle>) -> Res<(StatusCode, Json<RegleAsd>)> {
    state.ent.definir_regle_asd_for_actor(session.etablissement_id, session.user_id, &session.role, s).await.map(|r| (StatusCode::CREATED, Json(r))).map_err(map_db)
}

#[derive(Deserialize)]
struct JourQuery {
    jour: Option<NaiveDate>,
}

async fn regle_en_vigueur(State(state): State<AppState>, session: EntSession, Query(q): Query<JourQuery>) -> Res<Json<RegleAsd>> {
    let jour = match q.jour {
        Some(j) => j,
        None => state.ent.aujourdhui_etablissement(session.etablissement_id).await.map_err(map_db)?,
    };
    state.ent.regle_asd_en_vigueur_for_actor(session.etablissement_id, session.user_id, &session.role, jour).await.map(Json).map_err(map_db)
}

async fn activites(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<ActiviteAppelee>>> {
    state.ent.activites_appelees_for_actor(session.etablissement_id, session.user_id, &session.role).await.map(Json).map_err(map_db)
}

#[derive(Deserialize)]
struct ActiviteBody {
    type_activite: String,
    active: bool,
}

async fn definir_activite(State(state): State<AppState>, session: EntSession, Json(b): Json<ActiviteBody>) -> Res<StatusCode> {
    state.ent.definir_activite_appelee_for_actor(session.etablissement_id, session.user_id, &session.role, &b.type_activite, b.active).await.map(|_| StatusCode::NO_CONTENT).map_err(map_db)
}

// ─── AS2 : appel v2 ──────────────────────────────────────────────────────────

async fn ouvrir_activite(State(state): State<AppState>, session: EntSession, Json(o): Json<OuvrirActivite>) -> Res<Json<AppelV2>> {
    let a = state.ent.ouvrir_appel_activite(session.etablissement_id, session.user_id, &session.role, o).await.map_err(map_db)?;
    avec_noms(&state, session.etablissement_id, a).await
}

async fn appel_v2(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<AppelV2>> {
    let a = state.ent.appel_v2_lu(session.etablissement_id, id, session.user_id, &session.role).await.map_err(map_db)?;
    avec_noms(&state, session.etablissement_id, a).await
}

#[derive(Deserialize)]
struct PointerBody {
    eleve_id: Uuid,
    #[serde(flatten)]
    saisie: SaisieLigne,
}

async fn pointer(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<PointerBody>) -> Res<Json<LigneAppelV2>> {
    state.ent.pointer_ligne_v2(session.etablissement_id, id, b.eleve_id, session.user_id, &session.role, b.saisie, canal(&session)).await.map(Json).map_err(map_db)
}

async fn valider(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(mut v): Json<ValidationAppel>) -> Res<Json<AppelV2>> {
    // Le canal vient de la session ; seul « hors ligne » est annoncé par le client (AS10).
    let hors_ligne = v.canal.as_deref() == Some("hors_ligne");
    v.canal = Some(if hors_ligne { "hors_ligne".into() } else { canal(&session).into() });
    let a = state.ent.valider_appel_v2(session.etablissement_id, id, session.user_id, &session.role, v).await.map_err(map_db)?;
    avec_noms(&state, session.etablissement_id, a).await
}

#[derive(Deserialize)]
struct CorrigerBody {
    eleve_id: Uuid,
    #[serde(flatten)]
    correction: CorrectionLigne,
}

async fn corriger(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<CorrigerBody>) -> Res<Json<LigneAppelV2>> {
    state.ent.corriger_ligne_for_actor(session.etablissement_id, id, b.eleve_id, session.user_id, &session.role, b.correction, canal(&session)).await.map(Json).map_err(map_db)
}

async fn historique(State(state): State<AppState>, session: EntSession, Path((id, eleve_id)): Path<(Uuid, Uuid)>) -> Res<Json<Vec<HistoLigne>>> {
    state.ent.historique_ligne_for_actor(session.etablissement_id, id, eleve_id, session.user_id, &session.role).await.map(Json).map_err(map_db)
}

#[derive(Deserialize)]
struct DelegationBody {
    a_user: Uuid,
    motif: Option<String>,
}

async fn deleguer(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<DelegationBody>) -> Res<StatusCode> {
    state.ent.deleguer_appel_for_actor(session.etablissement_id, id, session.user_id, &session.role, b.a_user, b.motif.as_deref()).await.map(|_| StatusCode::NO_CONTENT).map_err(map_db)
}

async fn revoquer(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    state.ent.revoquer_delegation_for_actor(session.etablissement_id, id, session.user_id, &session.role).await.map(|_| StatusCode::NO_CONTENT).map_err(map_db)
}

#[derive(Deserialize)]
struct EleveBody {
    eleve_id: Uuid,
}

async fn exclure(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<EleveBody>) -> Res<Json<LigneAppelV2>> {
    state.ent.exclure_du_cours_for_actor(session.etablissement_id, id, b.eleve_id, session.user_id, &session.role, canal(&session)).await.map(Json).map_err(map_db)
}

async fn arrivee_vs(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<EleveBody>) -> Res<StatusCode> {
    state.ent.confirmer_arrivee_vs_for_actor(session.etablissement_id, id, b.eleve_id, session.user_id, &session.role).await.map(|_| StatusCode::NO_CONTENT).map_err(map_db)
}

async fn incoherences(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<IncoherenceV2>>> {
    state.ent.incoherences_appel_for_actor(session.etablissement_id, id, session.user_id, &session.role).await.map(Json).map_err(map_db)
}

// ─── AS2 : situations connues ────────────────────────────────────────────────

async fn situations_jour(State(state): State<AppState>, session: EntSession, Query(q): Query<JourQuery>) -> Res<Json<Vec<SituationAsd>>> {
    let jour = match q.jour {
        Some(j) => j,
        None => state.ent.aujourdhui_etablissement(session.etablissement_id).await.map_err(map_db)?,
    };
    state.ent.situations_du_jour_for_actor(session.etablissement_id, session.user_id, &session.role, jour).await.map(Json).map_err(map_db)
}

async fn localiser(State(state): State<AppState>, session: EntSession, Json(p): Json<PoserLocalisation>) -> Res<(StatusCode, Json<SituationAsd>)> {
    state.ent.poser_localisation_for_actor(session.etablissement_id, session.user_id, &session.role, p).await.map(|s| (StatusCode::CREATED, Json(s))).map_err(map_db)
}

async fn annoncer_retard(State(state): State<AppState>, session: EntSession, Json(a): Json<AnnoncerRetard>) -> Res<(StatusCode, Json<SituationAsd>)> {
    state.ent.annoncer_retard_for_actor(session.etablissement_id, session.user_id, &session.role, a).await.map(|s| (StatusCode::CREATED, Json(s))).map_err(map_db)
}

#[derive(Deserialize)]
struct LeverBody {
    #[serde(default)]
    venu: bool,
}

async fn lever_situation(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<LeverBody>) -> Res<StatusCode> {
    state.ent.lever_situation_for_actor(session.etablissement_id, session.user_id, &session.role, id, b.venu).await.map(|_| StatusCode::NO_CONTENT).map_err(map_db)
}

async fn texte_situation(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<String>>> {
    state.ent.texte_situation_for_actor(session.etablissement_id, session.user_id, &session.role, id).await.map(Json).map_err(map_db)
}
