//! Routes de la vie scolaire complète (socle 2, lot S2-4).
//!
//! L'autorité est `ent-db` (moteur de droits D-017 : `absences.declarer`,
//! `appel.verifier`, `punitions.donner`, `discipline.gerer`, `assiduite.signaler`,
//! `absences.lire`) : ces routes ne font que traduire HTTP ↔ service. Une
//! référence d'un autre établissement répond 404.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::{NaiveDate, NaiveTime};
use ent_db::{
    DeclarationAbsence, DeclarerAbsence, DecompteMois, DemiJourneeEleve, DonnerPunition,
    EffacementRapport, EngagerProcedure, ModeEffacement, PgEnt, ProcedureDisciplinaire, Punition,
    PunitionType, RegleAssiduite, RelanceAppel, SignalementAssiduite, TableauJour,
    VerificationAppels,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Journée vie scolaire / accueil
        .route("/vs/tableau-jour", get(tableau_jour))
        .route("/vs/verification-appels", get(verification_appels))
        .route("/vs/appels/relancer", post(relancer_appel))
        .route("/appels/{id}/attendus-absents", get(attendus_absents))
        // Déclarations
        .route("/vs/declarations/accueil", post(declarer_accueil))
        .route("/vs/declarations/famille", post(declarer_famille))
        .route("/vs/declarations/{id}/annuler", post(annuler_declaration))
        .route("/eleves/{id}/declarations-absence", get(declarations_eleve))
        // Assiduité
        .route("/eleves/{id}/demi-journees", get(demi_journees_eleve))
        .route("/vs/decompte", get(decompte))
        .route("/vs/regle-assiduite", get(lire_regle).put(definir_regle))
        .route("/vs/signalements", get(signalements))
        .route("/vs/signalements/proposer", post(proposer_signalements))
        .route("/vs/signalements/{id}/decider", post(decider_signalement))
        // Punitions (≠ sanctions)
        .route("/vs/punition-types", get(types_punition))
        .route("/vs/punition-types/amorcer", post(amorcer_punitions))
        .route("/eleves/{id}/punitions", get(punitions_eleve).post(donner_punition))
        .route("/vs/punitions/{id}/clore", post(clore_punition))
        // Discipline
        .route("/vs/sanction-types/{id}/categorie", put(categoriser_type))
        .route("/vs/regles-effacement/{categorie}", put(definir_effacement))
        .route("/vs/sanctions/effacer", post(effacer_sanctions))
        .route("/vs/procedures", post(engager_procedure))
        .route("/eleves/{id}/procedures", get(procedures_eleve))
        .route("/vs/procedures/{id}/observations", post(observations))
        .route("/vs/procedures/{id}/convoquer", post(convoquer))
        .route("/vs/procedures/{id}/decider", post(decider_procedure))
        .route("/vs/procedures/{id}/classer", post(classer_procedure))
}

fn ent(state: &AppState) -> &PgEnt {
    &state.ent
}

#[derive(Deserialize)]
struct JourQuery {
    jour: Option<NaiveDate>,
}

#[derive(Deserialize)]
struct MoisQuery {
    mois: Option<NaiveDate>,
}

fn aujourdhui_local() -> NaiveDate {
    PgEnt::maintenant_local().date()
}

async fn tableau_jour(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Res<Json<TableauJour>> {
    let jour = q.jour.unwrap_or_else(aujourdhui_local);
    ent(&state)
        .tableau_du_jour_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            jour,
            PgEnt::maintenant_local(),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn verification_appels(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Res<Json<VerificationAppels>> {
    let jour = q.jour.unwrap_or_else(aujourdhui_local);
    ent(&state)
        .verifier_appels_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            jour,
            PgEnt::maintenant_local(),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct RelanceBody {
    creneau_id: Uuid,
    jour: NaiveDate,
}

async fn relancer_appel(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<RelanceBody>,
) -> Res<(StatusCode, Json<RelanceAppel>)> {
    ent(&state)
        .relancer_appel(
            session.etablissement_id,
            session.user_id,
            &session.role,
            b.creneau_id,
            b.jour,
        )
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn attendus_absents(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<Uuid>>> {
    session.require_faire_appel()?;
    ent(&state)
        .attendus_absents_appel(session.etablissement_id, id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

// ─── Déclarations ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct DeclarationBody {
    eleve_id: Uuid,
    debut_jour: NaiveDate,
    debut_heure: Option<NaiveTime>,
    fin_jour: NaiveDate,
    fin_heure: Option<NaiveTime>,
    motif: String,
    commentaire: Option<String>,
    declarant_nom: Option<String>,
    declarant_qualite: Option<String>,
    piece_document_id: Option<Uuid>,
}

impl DeclarationBody {
    fn vers_cmd(self, etablissement_id: Uuid) -> DeclarerAbsence {
        DeclarerAbsence {
            etablissement_id,
            eleve_id: self.eleve_id,
            debut_jour: self.debut_jour,
            debut_heure: self.debut_heure,
            fin_jour: self.fin_jour,
            fin_heure: self.fin_heure,
            motif: self.motif,
            commentaire: self.commentaire,
            declarant_nom: self.declarant_nom,
            declarant_qualite: self.declarant_qualite,
            piece_document_id: self.piece_document_id,
        }
    }
}

async fn declarer_accueil(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<DeclarationBody>,
) -> Res<(StatusCode, Json<DeclarationAbsence>)> {
    ent(&state)
        .declarer_absence_accueil(
            session.user_id,
            &session.role,
            b.vers_cmd(session.etablissement_id),
        )
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn declarer_famille(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<DeclarationBody>,
) -> Res<(StatusCode, Json<DeclarationAbsence>)> {
    ent(&state)
        .declarer_absence_famille(
            session.user_id,
            &session.role,
            b.vers_cmd(session.etablissement_id),
        )
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn annuler_declaration(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    ent(&state)
        .annuler_declaration(session.etablissement_id, id, session.user_id, &session.role)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn declarations_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<DeclarationAbsence>>> {
    ent(&state)
        .declarations_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

// ─── Assiduité ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PeriodeQuery {
    du: NaiveDate,
    au: NaiveDate,
}

async fn demi_journees_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<PeriodeQuery>,
) -> Res<Json<Vec<DemiJourneeEleve>>> {
    ent(&state)
        .demi_journees_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            q.du,
            q.au,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn decompte(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<MoisQuery>,
) -> Res<Json<DecompteMois>> {
    ent(&state)
        .decompte_mensuel_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.mois.unwrap_or_else(aujourdhui_local),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn lire_regle(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<RegleAssiduite>> {
    session.require_staff()?;
    ent(&state)
        .regle_assiduite(session.etablissement_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn definir_regle(
    State(state): State<AppState>,
    session: EntSession,
    Json(r): Json<RegleAssiduite>,
) -> Res<Json<RegleAssiduite>> {
    ent(&state)
        .definir_regle_assiduite(session.etablissement_id, session.user_id, &session.role, r)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn signalements(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<MoisQuery>,
) -> Res<Json<Vec<SignalementAssiduite>>> {
    ent(&state)
        .signalements_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.mois.unwrap_or_else(aujourdhui_local),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct MoisBody {
    mois: NaiveDate,
}

async fn proposer_signalements(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<MoisBody>,
) -> Res<Json<Vec<SignalementAssiduite>>> {
    ent(&state)
        .proposer_signalements(session.etablissement_id, session.user_id, &session.role, b.mois)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct DecisionBody {
    /// `valider` | `refuser` | `emettre`.
    action: String,
}

async fn decider_signalement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DecisionBody>,
) -> Res<Json<SignalementAssiduite>> {
    ent(&state)
        .decider_signalement(
            session.etablissement_id,
            id,
            session.user_id,
            &session.role,
            &b.action,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

// ─── Punitions ───────────────────────────────────────────────────────────────

async fn types_punition(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<PunitionType>>> {
    session.require_staff()?;
    ent(&state)
        .types_punition_ou_amorce(session.etablissement_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn amorcer_punitions(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<PunitionType>>> {
    ent(&state)
        .amorcer_types_punition(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct PunitionBody {
    type_id: Uuid,
    jour: NaiveDate,
    motif: String,
    echeance: Option<NaiveDate>,
    duree_minutes: Option<i32>,
    incident_id: Option<Uuid>,
}

async fn donner_punition(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
    Json(b): Json<PunitionBody>,
) -> Res<(StatusCode, Json<Punition>)> {
    ent(&state)
        .donner_punition(
            session.user_id,
            &session.role,
            DonnerPunition {
                etablissement_id: session.etablissement_id,
                eleve_id,
                type_id: b.type_id,
                jour: b.jour,
                motif: b.motif,
                echeance: b.echeance,
                duree_minutes: b.duree_minutes,
                incident_id: b.incident_id,
            },
        )
        .await
        .map(|p| (StatusCode::CREATED, Json(p)))
        .map_err(map_db)
}

async fn punitions_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<Punition>>> {
    ent(&state)
        .punitions_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct StatutBody {
    /// `effectuee` | `annulee`.
    statut: String,
}

async fn clore_punition(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<StatutBody>,
) -> Res<Json<Punition>> {
    ent(&state)
        .clore_punition(
            session.etablissement_id,
            id,
            session.user_id,
            &session.role,
            &b.statut,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

// ─── Discipline ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CategorieBody {
    categorie: String,
}

async fn categoriser_type(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CategorieBody>,
) -> Res<StatusCode> {
    ent(&state)
        .categoriser_type_sanction(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.categorie,
        )
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct EffacementBody {
    /// `fin_annee_scolaire` | `delai_mois` | `jamais`.
    mode: String,
    delai_mois: Option<u32>,
}

async fn definir_effacement(
    State(state): State<AppState>,
    session: EntSession,
    Path(categorie): Path<String>,
    Json(b): Json<EffacementBody>,
) -> Res<StatusCode> {
    let mode = match (b.mode.as_str(), b.delai_mois) {
        ("fin_annee_scolaire", _) => ModeEffacement::FinAnneeScolaire,
        ("delai_mois", Some(n)) => ModeEffacement::DelaiMois(n),
        ("jamais", _) => ModeEffacement::Jamais,
        _ => return Err((StatusCode::BAD_REQUEST, "mode d'effacement invalide".into())),
    };
    ent(&state)
        .definir_regle_effacement(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &categorie,
            mode,
        )
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn effacer_sanctions(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Res<Json<EffacementRapport>> {
    ent(&state)
        .effacer_sanctions_echues_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            q.jour.unwrap_or_else(aujourdhui_local),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct ProcedureBody {
    eleve_id: Uuid,
    incident_id: Option<Uuid>,
    instance: String,
    faits: String,
    delai_convocation_jours: Option<i32>,
}

async fn engager_procedure(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<ProcedureBody>,
) -> Res<(StatusCode, Json<ProcedureDisciplinaire>)> {
    ent(&state)
        .engager_procedure(
            session.user_id,
            &session.role,
            EngagerProcedure {
                etablissement_id: session.etablissement_id,
                eleve_id: b.eleve_id,
                incident_id: b.incident_id,
                instance: b.instance,
                faits: b.faits,
                delai_convocation_jours: b.delai_convocation_jours,
            },
        )
        .await
        .map(|p| (StatusCode::CREATED, Json(p)))
        .map_err(map_db)
}

async fn procedures_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<ProcedureDisciplinaire>>> {
    ent(&state)
        .procedures_eleve_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct ObservationsBody {
    le: NaiveDate,
}

async fn observations(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ObservationsBody>,
) -> Res<Json<ProcedureDisciplinaire>> {
    ent(&state)
        .recueillir_observations(session.etablissement_id, id, session.user_id, &session.role, b.le)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct ConvocationBody {
    convocation_envoyee_le: NaiveDate,
    conseil_le: NaiveDate,
}

async fn convoquer(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ConvocationBody>,
) -> Res<Json<ProcedureDisciplinaire>> {
    ent(&state)
        .convoquer_conseil(
            session.etablissement_id,
            id,
            session.user_id,
            &session.role,
            b.convocation_envoyee_le,
            b.conseil_le,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct DecisionProcedureBody {
    decision: String,
    decision_le: NaiveDate,
    sanction_id: Option<Uuid>,
}

async fn decider_procedure(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DecisionProcedureBody>,
) -> Res<Json<ProcedureDisciplinaire>> {
    ent(&state)
        .decider_procedure(
            session.etablissement_id,
            id,
            session.user_id,
            &session.role,
            &b.decision,
            b.decision_le,
            b.sanction_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn classer_procedure(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ProcedureDisciplinaire>> {
    ent(&state)
        .classer_procedure(session.etablissement_id, id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}
