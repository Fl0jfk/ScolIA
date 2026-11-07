//! Routes des évaluations, conseils, bulletins et livrets (socle 2, lot S2-7).
//!
//! L'autorité est `ent-db` (moteur de droits D-017 : `notes.lire`, `notes.saisir`,
//! `conseils.gerer`, `periodes.verrouiller`, `bulletins.publier`, `notes.corriger`,
//! `livrets.exporter`, `matieres.gerer`, `competences.evaluer`, lien famille) : ces routes
//! ne font que traduire HTTP ↔ service. Une référence d'un autre établissement répond 404.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use ent_types::evaluations::{
    AppreciationBody, AppreciationDto, AvisVoeuBody, BilanCycleBody, BilanCycleDto,
    BulletinDetailDto, BulletinDto, CoefMatiereBody, ConseilDto, CorrigerNoteBody,
    CreerConseilBody, CreerDevoirGrilleBody, DecisionConseilBody, DecisionConseilDto,
    DevoirGrilleDto, ExportLivretDto, GenererBulletinsBody, GrilleDto, PeriodeNotesDto,
    PreparationConseilDto, PublierBulletinsBody, RapportBulletinsDto, RapportSaisieDto,
    ReglesCalculDto, ReleveEleveDto, ReponseFamilleBody, SaisieGrilleBody, ServiceEvaluationDto,
    StatutPeriodeBody, VoeuAvisDto, VueClasseDto,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::{attachment_response, map_db};
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Référentiels et règles
        .route("/evaluations/services", get(services))
        .route("/evaluations/periodes", get(periodes))
        .route("/evaluations/periodes/{id}/statut", put(statut_periode))
        .route("/evaluations/regles", get(regles).put(definir_regles))
        .route("/evaluations/coefficients", put(coefficient))
        // Grille
        .route("/evaluations/grille", get(grille))
        .route("/evaluations/devoirs", post(creer_devoir))
        .route("/evaluations/devoirs/{id}/resultats", put(saisir))
        .route("/evaluations/devoirs/{id}/publier", post(publier_devoir))
        .route("/evaluations/devoirs/{id}/corriger", post(corriger))
        // Vues
        .route("/evaluations/classes/{id}", get(vue_classe))
        .route("/evaluations/eleves/{id}/releve", get(releve))
        // Appréciations et conseil
        .route("/evaluations/appreciations", post(appreciation))
        .route("/evaluations/conseils", get(conseils).post(creer_conseil))
        .route("/evaluations/conseils/{id}/preparation", get(preparation))
        .route("/evaluations/conseils/{id}/decisions", put(decider))
        .route("/evaluations/conseils/{id}/avis-voeux", put(avis_voeu))
        // Orientation et socle
        .route("/evaluations/eleves/{id}/orientation", get(orientation))
        .route("/evaluations/avis/{id}/reponse", post(reponse_avis))
        .route("/evaluations/bilans-cycle", post(saisir_bilan))
        .route("/evaluations/bilans-cycle/{id}/valider", post(valider_bilan))
        .route("/evaluations/eleves/{id}/bilans-cycle", get(bilans_eleve))
        // Bulletins
        .route("/evaluations/bulletins", get(bulletins_classe))
        .route("/evaluations/bulletins/generer", post(generer_bulletins))
        .route("/evaluations/bulletins/publier", post(publier_bulletins))
        .route("/evaluations/bulletins/{id}", get(bulletin))
        .route("/evaluations/bulletins/{id}/pdf", get(bulletin_pdf))
        .route("/evaluations/eleves/{id}/bulletins", get(bulletins_eleve))
        // Livrets
        .route("/evaluations/exports", post(exporter))
}

#[derive(Deserialize)]
struct GrilleQuery {
    service_id: Uuid,
    periode_id: Uuid,
}

#[derive(Deserialize)]
struct PeriodeQuery {
    periode_id: Uuid,
}

#[derive(Deserialize)]
struct ClasseQuery {
    classe_id: Option<Uuid>,
}

#[derive(Deserialize)]
struct ClassePeriodeQuery {
    classe_id: Uuid,
    periode_id: Uuid,
}

#[derive(Deserialize)]
struct ExportBody {
    format: String,
    classe_id: Uuid,
    periode_id: Uuid,
}

async fn services(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<ServiceEvaluationDto>>> {
    s.ent
        .services_evaluables_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn periodes(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<PeriodeNotesDto>>> {
    s.ent.periodes_notes(session.etablissement_id).await.map(Json).map_err(map_db)
}

async fn statut_periode(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<StatutPeriodeBody>,
) -> Res<Json<PeriodeNotesDto>> {
    s.ent
        .changer_statut_periode_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b.statut)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn regles(State(s): State<AppState>, session: EntSession) -> Res<Json<ReglesCalculDto>> {
    s.ent
        .regles_calcul_for_actor(session.etablissement_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn definir_regles(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ReglesCalculDto>,
) -> Res<Json<ReglesCalculDto>> {
    s.ent
        .definir_regles_calcul_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn coefficient(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CoefMatiereBody>,
) -> Res<StatusCode> {
    s.ent
        .definir_coef_matiere_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn grille(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<GrilleQuery>,
) -> Res<Json<GrilleDto>> {
    s.ent
        .grille_for_actor(session.etablissement_id, session.user_id, &session.role, q.service_id, q.periode_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creer_devoir(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerDevoirGrilleBody>,
) -> Res<(StatusCode, Json<DevoirGrilleDto>)> {
    s.ent
        .creer_devoir_grille_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn saisir(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SaisieGrilleBody>,
) -> Res<Json<RapportSaisieDto>> {
    s.ent
        .saisir_grille_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn publier_devoir(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<DevoirGrilleDto>> {
    s.ent
        .publier_devoir_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn corriger(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CorrigerNoteBody>,
) -> Res<StatusCode> {
    s.ent
        .corriger_note_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn vue_classe(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<VueClasseDto>> {
    s.ent
        .vue_classe_for_actor(session.etablissement_id, session.user_id, &session.role, id, q.periode_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn releve(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<ReleveEleveDto>> {
    s.ent
        .releve_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id, q.periode_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn appreciation(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<AppreciationBody>,
) -> Res<Json<AppreciationDto>> {
    s.ent
        .saisir_appreciation_eva_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn conseils(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<ClasseQuery>,
) -> Res<Json<Vec<ConseilDto>>> {
    s.ent
        .conseils_eva_for_actor(session.etablissement_id, session.user_id, &session.role, q.classe_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creer_conseil(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerConseilBody>,
) -> Res<(StatusCode, Json<ConseilDto>)> {
    s.ent
        .creer_conseil_eva_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(map_db)
}

async fn preparation(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<PreparationConseilDto>> {
    s.ent
        .preparation_conseil_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn decider(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DecisionConseilBody>,
) -> Res<Json<DecisionConseilDto>> {
    s.ent
        .decider_conseil_eva_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn avis_voeu(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AvisVoeuBody>,
) -> Res<Json<VoeuAvisDto>> {
    s.ent
        .avis_voeu_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn orientation(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<VoeuAvisDto>>> {
    s.ent
        .orientation_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn reponse_avis(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ReponseFamilleBody>,
) -> Res<StatusCode> {
    s.ent
        .repondre_avis_for_actor(session.etablissement_id, session.user_id, &session.role, id, &b.reponse)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn saisir_bilan(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<BilanCycleBody>,
) -> Res<Json<BilanCycleDto>> {
    s.ent
        .saisir_bilan_cycle_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn valider_bilan(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<BilanCycleDto>> {
    s.ent
        .valider_bilan_cycle_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn bilans_eleve(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<BilanCycleDto>>> {
    s.ent
        .bilans_cycle_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn bulletins_classe(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<ClassePeriodeQuery>,
) -> Res<Json<Vec<BulletinDto>>> {
    s.ent
        .bulletins_classe_for_actor(session.etablissement_id, session.user_id, &session.role, q.classe_id, q.periode_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn generer_bulletins(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<GenererBulletinsBody>,
) -> Res<Json<RapportBulletinsDto>> {
    s.ent
        .generer_bulletins_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn publier_bulletins(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<PublierBulletinsBody>,
) -> Res<Json<RapportBulletinsDto>> {
    s.ent
        .publier_bulletins_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn bulletin(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<BulletinDetailDto>> {
    s.ent
        .bulletin_detail_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn bulletin_pdf(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Response> {
    let (nom, octets) = s
        .ent
        .bulletin_pdf_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    attachment_response(octets, &nom, "application/pdf")
}

async fn bulletins_eleve(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<BulletinDto>>> {
    s.ent
        .bulletins_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn exporter(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ExportBody>,
) -> Res<Json<ExportLivretDto>> {
    s.ent
        .exporter_livret_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            b.format.trim(),
            b.classe_id,
            b.periode_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}
