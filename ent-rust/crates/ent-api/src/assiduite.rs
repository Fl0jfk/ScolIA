//! Routes de l'assiduité (socle 12, lots AS1 et AS2) : référentiels, règle d'assiduité datée, motifs,
//! activités appelées, appel v2 (activités, pointage, validation, corrections, délégation,
//! exclusion de cours), situations connues (localisation, retard annoncé).
//!
//! L'autorité est `ent-db` (moteur de droits D-017 : `appel.faire`, `appel.faire_tout`,
//! `appel.corriger`, `appel.deleguer`, `absences.lire_motif`, `localisation.poser`,
//! `assiduite.regle_gerer`, `absences.declarer`) ; ces routes ne font que traduire HTTP ↔ service.
//! **Aucun contrôle de rôle codé en dur ici** (DA-18) : une route ne répond que ce que le service
//! autorise, et une référence d'un autre établissement ou inexistante donne la même réponse (R-AS35).

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::NaiveDate;
use base64::Engine;
use ent_db::cahier::FichierDepot;
use ent_db::{
    ActiviteAppelee, AnnoncerRetard, ApercuMasse, AppelV2, BilanMasse, CorrectionLigne, DecisionDemande,
    DemandeAsd, DemandeAutorisation, DepotJustificatif, DetailEpisode, EpisodeAsd, EpisodeVueFamille,
    ArretMois, EpisodeVueProfesseur, FiltreEpisodes, HistoLigne, IncoherenceV2, JustificatifAsd, LienJustificatif,
    LigneAppelV2, MotifAsd, NatureMotif, OuvrirActivite, PoserLocalisation, ProfilAcademique,
    ProposerQualification, PropositionAsd, QualifierEpisode, Rectification, RegleAsd, RequalifierEpisode, SaisieLigne,
    SaisieMotif, SaisieRegle, SelectionMasse, SituationAsd, ValidationAppel, VersionMotif,
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
        // Décompte versionné : arrêt du mois, rectifications postérieures (AS4)
        .route("/assiduite/mois/{mois}/arreter", post(arreter_mois))
        .route("/assiduite/mois/{mois}/rectifications", get(rectifications))
        .route("/assiduite/rectifications/{id}/vue", post(rectification_vue))
        // Épisodes, qualification, justificatifs, demandes (AS3)
        .route("/assiduite/episodes", get(episodes))
        .route("/assiduite/episodes/masse/apercu", post(apercu_masse))
        .route("/assiduite/episodes/masse", post(qualifier_masse))
        .route("/assiduite/episodes/{id}", get(episode))
        .route("/assiduite/episodes/{id}/qualification", post(qualifier))
        .route("/assiduite/episodes/{id}/requalification", post(requalifier))
        .route("/assiduite/episodes/{id}/propositions", post(proposer))
        .route("/assiduite/mes-episodes", get(mes_episodes))
        .route("/assiduite/eleves/{eleve_id}/episodes", get(episodes_famille))
        .route("/assiduite/episodes/{id}/justificatifs", get(justificatifs).post(deposer_justificatif).layer(corps_fichier()))
        .route("/assiduite/episodes/{id}/justificatifs/lectures", get(lectures_justificatifs))
        .route("/assiduite/episodes/{id}/justificatifs/{jid}/lien", post(lien_justificatif))
        .route("/assiduite/episodes/{id}/justificatifs/{jid}/retirer", post(retirer_justificatif))
        .route("/assiduite/demandes", get(demandes).post(demander_autorisation))
        .route("/assiduite/demandes/{id}/decision", post(decider_demande))
        .route("/assiduite/demandes/{id}/retirer", post(retirer_demande))
}

fn corps_fichier() -> DefaultBodyLimit {
    let up = ent_db::upload::PolitiqueFichiers::depuis_env();
    DefaultBodyLimit::max(up.limite_corps_base64(ent_db::upload::UsageFichier::Document))
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

// ─── AS3 : épisodes, qualification, justificatifs, demandes ──────────────────

async fn avec_noms_episodes(state: &AppState, etab: Uuid, mut v: Vec<EpisodeAsd>) -> Res<Json<Vec<EpisodeAsd>>> {
    let ids: Vec<Uuid> = v.iter().map(|e| e.eleve_id).collect();
    let noms = state.ent.eleve_labels(etab, &ids).await.map_err(map_db)?;
    for e in &mut v {
        e.eleve_label = noms.get(&e.eleve_id).cloned();
    }
    Ok(Json(v))
}

async fn episodes(State(state): State<AppState>, session: EntSession, Query(f): Query<FiltreEpisodes>) -> Res<Json<Vec<EpisodeAsd>>> {
    let v = state.ent.episodes_for_actor(session.etablissement_id, session.user_id, &session.role, f).await.map_err(map_db)?;
    avec_noms_episodes(&state, session.etablissement_id, v).await
}

async fn episode(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<DetailEpisode>> {
    state.ent.episode_for_actor(session.etablissement_id, session.user_id, &session.role, id).await.map(Json).map_err(map_db)
}

async fn qualifier(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(q): Json<QualifierEpisode>) -> Res<Json<EpisodeAsd>> {
    state.ent.qualifier_episode_for_actor(session.etablissement_id, session.user_id, &session.role, id, q).await.map(Json).map_err(map_db)
}

async fn requalifier(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(q): Json<RequalifierEpisode>) -> Res<Json<EpisodeAsd>> {
    state.ent.requalifier_episode_for_actor(session.etablissement_id, session.user_id, &session.role, id, q).await.map(Json).map_err(map_db)
}

async fn proposer(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(p): Json<ProposerQualification>) -> Res<(StatusCode, Json<PropositionAsd>)> {
    state.ent.proposer_qualification_for_actor(session.etablissement_id, session.user_id, &session.role, id, p).await.map(|r| (StatusCode::CREATED, Json(r))).map_err(map_db)
}

async fn apercu_masse(State(state): State<AppState>, session: EntSession, Json(s): Json<SelectionMasse>) -> Res<Json<ApercuMasse>> {
    state.ent.apercu_masse_for_actor(session.etablissement_id, session.user_id, &session.role, &s).await.map(Json).map_err(map_db)
}

async fn qualifier_masse(State(state): State<AppState>, session: EntSession, Json(s): Json<SelectionMasse>) -> Res<Json<BilanMasse>> {
    state.ent.qualifier_masse_for_actor(session.etablissement_id, session.user_id, &session.role, s).await.map(Json).map_err(map_db)
}

/// Vue du professeur : l'état de régularisation des épisodes de ses séances, sans motif (R-AS21).
async fn mes_episodes(State(state): State<AppState>, session: EntSession, Query(q): Query<JourQuery>) -> Res<Json<Vec<EpisodeVueProfesseur>>> {
    let depuis = match q.jour {
        Some(j) => j,
        None => state.ent.aujourdhui_etablissement(session.etablissement_id).await.map_err(map_db)? - chrono::Duration::days(14),
    };
    state.ent.episodes_de_mes_seances_for_actor(session.etablissement_id, session.user_id, &session.role, depuis).await.map(Json).map_err(map_db)
}

async fn episodes_famille(State(state): State<AppState>, session: EntSession, Path(eleve_id): Path<Uuid>) -> Res<Json<Vec<EpisodeVueFamille>>> {
    state.ent.episodes_famille_for_actor(session.etablissement_id, session.user_id, &session.role, eleve_id).await.map(Json).map_err(map_db)
}

async fn justificatifs(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<JustificatifAsd>>> {
    state.ent.justificatifs_for_actor(session.etablissement_id, session.user_id, &session.role, id).await.map(Json).map_err(map_db)
}

#[derive(Deserialize)]
struct FichierJson {
    nom: String,
    mime_type: String,
    contenu_base64: String,
}

#[derive(Deserialize)]
struct DeposerJustificatifBody {
    texte: Option<String>,
    medical: Option<bool>,
    #[serde(default)]
    vu_non_conserve: bool,
    fichier: Option<FichierJson>,
}

/// Dépôt d'un justificatif : le type et la clé d'objet sont décidés par le serveur, jamais par le client.
async fn deposer_justificatif(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<DeposerJustificatifBody>) -> Res<(StatusCode, Json<JustificatifAsd>)> {
    let fichier = match b.fichier {
        Some(f) => Some(FichierDepot {
            nom: f.nom,
            mime_type: f.mime_type,
            contenu: base64::engine::general_purpose::STANDARD
                .decode(f.contenu_base64.trim())
                .map_err(|_| (StatusCode::BAD_REQUEST, "contenu_base64 invalide".to_string()))?,
        }),
        None => None,
    };
    let depot = DepotJustificatif { texte: b.texte, medical: b.medical, vu_non_conserve: b.vu_non_conserve };
    state.ent.deposer_justificatif_for_actor(session.etablissement_id, session.user_id, &session.role, id, depot, fichier).await.map(|j| (StatusCode::CREATED, Json(j))).map_err(map_db)
}

async fn lien_justificatif(State(state): State<AppState>, session: EntSession, Path((id, jid)): Path<(Uuid, Uuid)>) -> Res<Json<LienJustificatif>> {
    state.ent.lien_justificatif_for_actor(session.etablissement_id, session.user_id, &session.role, id, jid).await.map(Json).map_err(map_db)
}

async fn retirer_justificatif(State(state): State<AppState>, session: EntSession, Path((id, jid)): Path<(Uuid, Uuid)>) -> Res<Json<JustificatifAsd>> {
    state.ent.retirer_justificatif_for_actor(session.etablissement_id, session.user_id, &session.role, id, jid).await.map(Json).map_err(map_db)
}

async fn lectures_justificatifs(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<(Uuid, Uuid, bool, chrono::DateTime<chrono::Utc>)>>> {
    state.ent.lectures_justificatifs_for_actor(session.etablissement_id, session.user_id, &session.role, id).await.map(Json).map_err(map_db)
}

#[derive(Deserialize)]
struct DemandesQuery {
    eleve_id: Option<Uuid>,
}

async fn demandes(State(state): State<AppState>, session: EntSession, Query(q): Query<DemandesQuery>) -> Res<Json<Vec<DemandeAsd>>> {
    state.ent.demandes_for_actor(session.etablissement_id, session.user_id, &session.role, q.eleve_id).await.map(Json).map_err(map_db)
}

async fn demander_autorisation(State(state): State<AppState>, session: EntSession, Json(d): Json<DemandeAutorisation>) -> Res<(StatusCode, Json<DemandeAsd>)> {
    state.ent.demander_autorisation_for_actor(session.etablissement_id, session.user_id, &session.role, d).await.map(|r| (StatusCode::CREATED, Json(r))).map_err(map_db)
}

async fn decider_demande(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(d): Json<DecisionDemande>) -> Res<Json<DemandeAsd>> {
    state.ent.decider_demande_for_actor(session.etablissement_id, session.user_id, &session.role, id, d).await.map(Json).map_err(map_db)
}

async fn retirer_demande(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<DemandeAsd>> {
    state.ent.retirer_demande_for_actor(session.etablissement_id, session.user_id, id).await.map(Json).map_err(map_db)
}

// ── Décompte versionné (AS4) ────────────────────────────────────────────────────────────────────

/// Arrête un mois terminé (`assiduite.regle_gerer`, direction) : décompte figé, règles figées. Idempotent.
async fn arreter_mois(State(state): State<AppState>, session: EntSession, Path(mois): Path<NaiveDate>) -> Res<Json<ArretMois>> {
    state.ent.arreter_mois_for_actor(session.etablissement_id, session.user_id, &session.role, mois).await.map(Json).map_err(map_db)
}

/// Rectifications postérieures à l'arrêt d'un mois (`assiduite.signaler`).
async fn rectifications(State(state): State<AppState>, session: EntSession, Path(mois): Path<NaiveDate>) -> Res<Json<Vec<Rectification>>> {
    state.ent.rectifications_for_actor(session.etablissement_id, session.user_id, &session.role, mois).await.map(Json).map_err(map_db)
}

async fn rectification_vue(State(state): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    state.ent.rectification_vue_for_actor(session.etablissement_id, session.user_id, &session.role, id).await.map(|()| StatusCode::NO_CONTENT).map_err(map_db)
}
