//! Routes `/voyages/*` du lot V2a (socle 42) : l'argent des sorties en **mode suivi** (R-SV44).
//!
//! Aucune règle ici : droits, états, cloisonnement par établissement, montants, ajout seul et journal sont dans
//! `ent-db` (`voyages_argent_repo.rs`, `voyages_argent_flux.rs`, migrations 650 à 652). Aucune route n'accepte un
//! identifiant d'établissement ni un montant sous forme de nombre : les montants sont des **textes** français
//! (`125,50`), analysés par le serveur en centimes entiers. Aucune de ces routes n'émet de facture ni d'écriture
//! comptable : ScolIA décide, répartit, pointe et exporte.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_db::voyages_argent_flux::{AcompteSaisie, AcompteVue, BordereauAttente, BordereauSaisie, BordereauVue};
use ent_db::voyages_argent_v3::{
    AideSaisie, AideVue, DecisionAide, DecisionReport, DecisionValidee, DemandeVue, InternatSynchro, ReportSaisie, ReportVue,
    StatutDemande, AVERTISSEMENT_MOTIF,
};
use ent_db::voyages_argent_repo::{
    AvoirVue, EtatAgrege, ExportFait, FraisSaisie, FraisVue, MaPart, NominatifVue, PaiementsVue, Reglages,
    VersementSaisie, VersementVue,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/voyages/argent/reglages", get(reglages_lire).put(reglages_definir))
        .route("/voyages/argent/bordereaux-en-attente", get(bordereaux_en_attente))
        .route("/voyages/argent/acomptes/alertes", post(acomptes_alertes))
        .route("/voyages/{id}/argent", get(etat))
        .route("/voyages/{id}/argent/paiements", get(paiements))
        .route("/voyages/{id}/argent/mes-versements", get(famille))
        .route("/voyages/{id}/argent/synchroniser", post(synchroniser))
        .route("/voyages/{id}/argent/versements", get(versements).post(verser))
        .route("/voyages/{id}/argent/versements/{vid}/annulation", post(versement_annuler))
        .route("/voyages/{id}/argent/plans", post(plan_accorder))
        .route("/voyages/{id}/argent/bordereaux", get(bordereaux).post(bordereau_saisir))
        .route("/voyages/{id}/argent/bordereaux/{bid}/remise", post(bordereau_remettre))
        .route("/voyages/{id}/argent/bordereaux/lignes/{lid}/pointage", post(bordereau_ligne_pointer))
        .route("/voyages/{id}/argent/acomptes", get(acomptes).post(acompte_creer))
        .route("/voyages/{id}/argent/acomptes/{aid}/paiement", post(acompte_payer))
        .route("/voyages/{id}/argent/acomptes/{aid}/annulation", post(acompte_annuler))
        .route("/voyages/{id}/argent/frais-reels", get(frais).post(frais_saisir))
        .route("/voyages/{id}/argent/frais-reels/{fid}/annulation", post(frais_annuler))
        .route("/voyages/{id}/argent/avoirs", get(avoirs))
        .route("/voyages/{id}/argent/avoirs/annulation-collective", post(avoirs_collectifs))
        .route("/voyages/{id}/argent/avoirs/{aid}/decision", post(avoir_decider))
        // Lot V3 « argent relié » : aides, reports de versement, internat (socle 41).
        .route("/voyages/{id}/argent/aides", get(aides).post(aide_accorder))
        .route("/voyages/{id}/argent/aides/{aid}/annulation", post(aide_annuler))
        .route("/voyages/{id}/argent/reports", get(reports).post(report_effectuer))
        .route("/voyages/{id}/argent/reports/{rid}/annulation", post(report_annuler))
        .route("/voyages/{id}/internat/synchroniser", post(internat_synchroniser))
        // D-069 : seconde validation, retour anticipé, consigne de motif.
        .route("/voyages/{id}/argent/demandes", get(demandes))
        .route("/voyages/{id}/argent/demandes/{did}/validation", post(demande_valider))
        .route("/voyages/{id}/argent/demandes/{did}/refus", post(demande_refuser))
        .route("/voyages/{id}/argent/consigne-motif", get(consigne_motif))
        .route("/voyages/{id}/participants/{pid}/retour-anticipe", post(retour_anticipe))
        .route("/voyages/{id}/argent/export", post(exporter))
        .route("/voyages/{id}/argent/nominatif", post(nominatif_ouvrir).delete(nominatif_fermer))
}

async fn reglages_lire(State(s): State<AppState>, session: EntSession) -> Res<Json<Reglages>> {
    Ok(Json(s.ent.voyages_argent_reglages(&acteur(&session)).await.map_err(map_db)?))
}

async fn reglages_definir(State(s): State<AppState>, session: EntSession, Json(b): Json<Reglages>) -> Res<Json<Reglages>> {
    Ok(Json(s.ent.voyages_argent_reglages_definir(&acteur(&session), b).await.map_err(map_db)?))
}

async fn bordereaux_en_attente(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<BordereauAttente>>> {
    Ok(Json(s.ent.voyages_bordereaux_en_attente(&acteur(&session)).await.map_err(map_db)?))
}

#[derive(Serialize)]
struct Alertes {
    emises: usize,
}

async fn acomptes_alertes(State(s): State<AppState>, session: EntSession) -> Res<Json<Alertes>> {
    Ok(Json(Alertes { emises: s.ent.voyages_acomptes_alertes_declencher(&acteur(&session)).await.map_err(map_db)? }))
}

async fn etat(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<EtatAgrege>> {
    Ok(Json(s.ent.voyage_argent_etat(&acteur(&session), id).await.map_err(map_db)?))
}

/// Le DTO est choisi par le serveur selon `voyages.argent` (E1) : le responsable désigné ne reçoit jamais l'échéancier.
async fn paiements(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<PaiementsVue>> {
    Ok(Json(s.ent.voyage_argent_paiements_vue(&acteur(&session), id).await.map_err(map_db)?))
}

async fn famille(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<MaPart>>> {
    Ok(Json(s.ent.voyage_argent_famille(&acteur(&session), id).await.map_err(map_db)?))
}

async fn synchroniser(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<EtatAgrege>> {
    Ok(Json(s.ent.voyage_argent_synchroniser(&acteur(&session), id).await.map_err(map_db)?))
}

async fn versements(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<VersementVue>>> {
    Ok(Json(s.ent.voyage_versements(&acteur(&session), id).await.map_err(map_db)?))
}

async fn verser(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<VersementSaisie>,
) -> Res<(StatusCode, Json<VersementVue>)> {
    Ok((StatusCode::CREATED, Json(s.ent.voyage_versement_pointer(&acteur(&session), id, b).await.map_err(map_db)?)))
}

#[derive(Deserialize)]
struct MotifBody {
    motif: String,
}

async fn versement_annuler(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, vid)): Path<(Uuid, Uuid)>,
    Json(b): Json<MotifBody>,
) -> Res<(StatusCode, Json<VersementVue>)> {
    Ok((StatusCode::CREATED, Json(s.ent.voyage_versement_annuler(&acteur(&session), id, vid, &b.motif).await.map_err(map_db)?)))
}

#[derive(Deserialize)]
struct PlanBody {
    participant_id: Uuid,
    compte_id: Uuid,
    nb_echeances: i32,
    #[serde(default)]
    motif: Option<String>,
}

async fn plan_accorder(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PlanBody>,
) -> Res<StatusCode> {
    s.ent
        .voyage_plan_accorder(&acteur(&session), id, b.participant_id, b.compte_id, b.nb_echeances, b.motif.as_deref())
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn bordereaux(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<BordereauVue>>> {
    Ok(Json(s.ent.voyage_bordereaux(&acteur(&session), id).await.map_err(map_db)?))
}

async fn bordereau_saisir(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<BordereauSaisie>,
) -> Res<(StatusCode, Json<BordereauVue>)> {
    Ok((StatusCode::CREATED, Json(s.ent.voyage_bordereau_saisir(&acteur(&session), id, b).await.map_err(map_db)?)))
}

#[derive(Deserialize)]
struct RemiseBody {
    /// Les lignes du bordereau effectivement reçues ; les autres sont « manquantes » (M4).
    recues: Vec<Uuid>,
    #[serde(default)]
    note: Option<String>,
}

async fn bordereau_remettre(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, bid)): Path<(Uuid, Uuid)>,
    Json(b): Json<RemiseBody>,
) -> Res<(StatusCode, Json<BordereauVue>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.ent
                .voyage_bordereau_remettre(&acteur(&session), id, bid, b.recues, b.note.as_deref())
                .await
                .map_err(map_db)?,
        ),
    ))
}

#[derive(Deserialize)]
struct PointageBody {
    compte_id: Uuid,
}

async fn bordereau_ligne_pointer(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, lid)): Path<(Uuid, Uuid)>,
    Json(b): Json<PointageBody>,
) -> Res<(StatusCode, Json<VersementVue>)> {
    Ok((
        StatusCode::CREATED,
        Json(s.ent.voyage_bordereau_ligne_pointer(&acteur(&session), id, lid, b.compte_id).await.map_err(map_db)?),
    ))
}

async fn acomptes(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<AcompteVue>>> {
    Ok(Json(s.ent.voyage_acomptes(&acteur(&session), id).await.map_err(map_db)?))
}

async fn acompte_creer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AcompteSaisie>,
) -> Res<(StatusCode, Json<AcompteVue>)> {
    Ok((StatusCode::CREATED, Json(s.ent.voyage_acompte_creer(&acteur(&session), id, b).await.map_err(map_db)?)))
}

#[derive(Deserialize)]
struct PaiementBody {
    #[serde(default)]
    montant: Option<String>,
    #[serde(default)]
    date: Option<NaiveDate>,
    #[serde(default)]
    reference: Option<String>,
}

async fn acompte_payer(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, aid)): Path<(Uuid, Uuid)>,
    Json(b): Json<PaiementBody>,
) -> Res<Json<AcompteVue>> {
    Ok(Json(
        s.ent
            .voyage_acompte_payer(&acteur(&session), id, aid, b.montant.as_deref(), b.date, b.reference.as_deref())
            .await
            .map_err(map_db)?,
    ))
}

async fn acompte_annuler(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, aid)): Path<(Uuid, Uuid)>,
    Json(b): Json<MotifBody>,
) -> Res<StatusCode> {
    s.ent.voyage_acompte_annuler(&acteur(&session), id, aid, &b.motif).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn frais(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<FraisVue>>> {
    Ok(Json(s.ent.voyage_frais(&acteur(&session), id).await.map_err(map_db)?))
}

async fn frais_saisir(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<FraisSaisie>,
) -> Res<(StatusCode, Json<FraisVue>)> {
    Ok((StatusCode::CREATED, Json(s.ent.voyage_frais_saisir(&acteur(&session), id, b).await.map_err(map_db)?)))
}

async fn frais_annuler(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, fid)): Path<(Uuid, Uuid)>,
    Json(b): Json<MotifBody>,
) -> Res<StatusCode> {
    s.ent.voyage_frais_annuler(&acteur(&session), id, fid, &b.motif).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn avoirs(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<AvoirVue>>> {
    Ok(Json(s.ent.voyage_avoirs(&acteur(&session), id).await.map_err(map_db)?))
}

#[derive(Serialize)]
struct Ajoutes {
    ajoutes: usize,
}

async fn avoirs_collectifs(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Ajoutes>> {
    Ok(Json(Ajoutes { ajoutes: s.ent.voyage_avoirs_annulation_collective(&acteur(&session), id).await.map_err(map_db)? }))
}

#[derive(Deserialize)]
struct DecisionBody {
    /// `retenu` ou `ecarte`.
    decision: String,
    #[serde(default)]
    montant_retenu: Option<String>,
    /// Obligatoire pour retenir plus que le montant proposé : `frais_non_dus`, `erreur_de_calcul`, `geste_commercial`, `autre_justifie`.
    #[serde(default)]
    motif_code: Option<String>,
    motif: String,
}

async fn avoir_decider(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, aid)): Path<(Uuid, Uuid)>,
    Json(b): Json<DecisionBody>,
) -> Res<Json<AvoirVue>> {
    let retenir = match b.decision.as_str() {
        "retenu" => true,
        "ecarte" => false,
        _ => return Err((StatusCode::UNPROCESSABLE_ENTITY, "décision inconnue".to_string())),
    };
    Ok(Json(
        s.ent
            .voyage_avoir_decider(&acteur(&session), id, aid, retenir, b.montant_retenu.as_deref(), b.motif_code.as_deref(), &b.motif)
            .await
            .map_err(map_db)?,
    ))
}

async fn exporter(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<ExportFait>> {
    Ok(Json(s.ent.voyage_argent_export(&acteur(&session), id).await.map_err(map_db)?))
}

async fn nominatif_ouvrir(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<MotifBody>,
) -> Res<(StatusCode, Json<NominatifVue>)> {
    Ok((StatusCode::CREATED, Json(s.ent.voyage_nominatif_ouvrir(&acteur(&session), id, &b.motif).await.map_err(map_db)?)))
}

async fn nominatif_fermer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    s.ent.voyage_nominatif_fermer(&acteur(&session), id).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

// ───────────────────────── Lot V3 : aides, reports, internat ─────────────────────────

async fn aides(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<AideVue>>> {
    Ok(Json(s.ent.voyage_aides(&acteur(&session), id).await.map_err(map_db)?))
}

async fn aide_accorder(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AideSaisie>,
) -> Res<(StatusCode, Json<DecisionAide>)> {
    let d = s.ent.voyage_aide_accorder(&acteur(&session), id, b).await.map_err(map_db)?;
    // 201 quand l'aide est écrite ; 202 quand elle attend la seconde personne.
    Ok((statut_http(d.statut), Json(d)))
}

async fn aide_annuler(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, aid)): Path<(Uuid, Uuid)>,
    Json(b): Json<MotifBody>,
) -> Res<(StatusCode, Json<AideVue>)> {
    Ok((StatusCode::CREATED, Json(s.ent.voyage_aide_annuler(&acteur(&session), id, aid, &b.motif).await.map_err(map_db)?)))
}

async fn reports(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<ReportVue>>> {
    Ok(Json(s.ent.voyage_reports(&acteur(&session), id).await.map_err(map_db)?))
}

async fn report_effectuer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ReportSaisie>,
) -> Res<(StatusCode, Json<DecisionReport>)> {
    let d = s.ent.voyage_report_effectuer(&acteur(&session), id, b).await.map_err(map_db)?;
    Ok((statut_http(d.statut), Json(d)))
}

async fn report_annuler(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, rid)): Path<(Uuid, Uuid)>,
    Json(b): Json<MotifBody>,
) -> Res<(StatusCode, Json<ReportVue>)> {
    Ok((StatusCode::CREATED, Json(s.ent.voyage_report_annuler(&acteur(&session), id, rid, &b.motif).await.map_err(map_db)?)))
}

fn statut_http(s: StatutDemande) -> StatusCode {
    match s {
        StatutDemande::Appliquee => StatusCode::CREATED,
        _ => StatusCode::ACCEPTED,
    }
}

async fn demandes(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<DemandeVue>>> {
    Ok(Json(s.ent.voyage_demandes(&acteur(&session), id).await.map_err(map_db)?))
}

async fn demande_valider(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, did)): Path<(Uuid, Uuid)>,
) -> Res<(StatusCode, Json<DecisionValidee>)> {
    Ok((StatusCode::CREATED, Json(s.ent.voyage_demande_valider(&acteur(&session), id, did).await.map_err(map_db)?)))
}

async fn demande_refuser(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, did)): Path<(Uuid, Uuid)>,
    Json(b): Json<MotifBody>,
) -> Res<Json<StatutDemande>> {
    Ok(Json(s.ent.voyage_demande_refuser(&acteur(&session), id, did, &b.motif).await.map_err(map_db)?))
}

#[derive(serde::Serialize)]
struct ConsigneMotif {
    motif_obligatoire: bool,
    avertissement: &'static str,
}

/// F2 : l'écran de saisie du motif d'une aide ou d'un report affiche cet avertissement (aucun détail de situation sociale).
async fn consigne_motif(session: EntSession, Path(_id): Path<Uuid>) -> Json<ConsigneMotif> {
    let _ = session;
    Json(ConsigneMotif {
        motif_obligatoire: true,
        avertissement: AVERTISSEMENT_MOTIF,
    })
}

#[derive(serde::Deserialize)]
struct RetourAnticipeBody {
    motif_code: String,
}

async fn retour_anticipe(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, pid)): Path<(Uuid, Uuid)>,
    Json(b): Json<RetourAnticipeBody>,
) -> Res<Json<InternatSynchro>> {
    Ok(Json(s.ent.voyage_participant_retour_anticipe(&acteur(&session), id, pid, &b.motif_code).await.map_err(map_db)?))
}

async fn internat_synchroniser(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<InternatSynchro>> {
    Ok(Json(s.ent.voyage_internat_synchroniser(&acteur(&session), id).await.map_err(map_db)?))
}
