//! Routes `/voyages/*` du socle 42, lots V0a (sorties) et V0b (budget, prix annoncé, PDF, rappels,
//! synthèse APEL) : sorties et voyages scolaires collectifs.
//!
//! Aucune règle ici (R-SV1) : droits (`voyages.*` et droits du porteur), états, préconditions,
//! version, cloisonnement (l'établissement vient de la session, jamais du corps), journal et
//! événements sont dans `ent-db` (`voyages.rs`, `voyages_repo.rs`). Le navigateur n'envoie jamais un
//! dossier entier : chaque transition est une **commande** (`/voyages/{id}/commandes/{commande}`).

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use ent_db::voyages::Commande;
use ent_db::voyages::PointAudit;
use ent_db::voyages_budget_repo::{
    BudgetVue, DocumentVue, LigneSaisie, ParamsBudget, SyntheseApel,
};
use ent_db::voyages_rappels::RappelVue;
use ent_db::voyages_repo::{
    CandidatsClasse, Circulaire, EncadrantVue, EtapeSaisie, EtapeVue, FiltreVoyages, JournalVue,
    ParamsCommande, ParticipantVue, ResultatAjout, ResultatSeance, ResultatSerie, SerieSaisie,
    SortieDetail, SortieLigne, SortieResume, SortieSaisie, TableauDirection, VoyageFamille,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/voyages", get(liste).post(creer))
        .route("/voyages/series", post(serie_creer))
        .route("/voyages/series/{serie_id}/valider", post(serie_valider))
        .route("/voyages/tableau", get(tableau))
        .route("/voyages/famille", get(famille))
        .route("/voyages/{id}", get(detail).put(modifier))
        .route("/voyages/{id}/commandes/{commande}", post(commande))
        .route("/voyages/{id}/porteur", put(porteur))
        .route("/voyages/{id}/programme", put(programme))
        .route("/voyages/{id}/audit", get(audit))
        .route("/voyages/{id}/journal", get(journal))
        .route("/voyages/{id}/candidats", get(candidats))
        .route(
            "/voyages/{id}/participants",
            get(participants).post(participants_ajouter),
        )
        .route("/voyages/{id}/participants.csv", get(participants_csv))
        .route(
            "/voyages/{id}/participants/{pid}/desistement",
            post(desistement),
        )
        .route("/voyages/{id}/participants/{pid}/retrait", post(retrait))
        .route("/voyages/{id}/encadrants", post(encadrant_ajouter))
        .route("/voyages/{id}/encadrants/{eid}", delete(encadrant_retirer))
        .route("/voyages/{id}/circulaire", post(circulaire))
        // Lot V0b : budget versionné en centimes, prix annoncé figé, PDF, rappels, synthèse APEL.
        .route("/voyages/{id}/budget", get(budget))
        .route("/voyages/{id}/budget/versions", post(budget_version_creer))
        .route(
            "/voyages/{id}/budget/versions/{numero}",
            put(budget_parametrer).delete(budget_version_abandonner),
        )
        .route(
            "/voyages/{id}/budget/versions/{numero}/lignes",
            post(budget_ligne_ajouter),
        )
        .route(
            "/voyages/{id}/budget/versions/{numero}/lignes/{lid}",
            put(budget_ligne_modifier).delete(budget_ligne_supprimer),
        )
        .route(
            "/voyages/{id}/budget/versions/{numero}/valider-revision",
            post(budget_valider_revision),
        )
        .route("/voyages/{id}/prix/hausse", post(prix_hausser))
        .route("/voyages/{id}/budget/pdf", post(budget_pdf))
        .route("/voyages/{id}/circulaire/pdf", post(circulaire_pdf))
        .route("/voyages/{id}/documents", get(documents))
        .route("/voyages/{id}/documents/{doc}", get(document))
        .route("/voyages/{id}/rappels", get(rappels))
        .route("/voyages/rappels/traiter", post(rappels_traiter))
        .route("/voyages/apel", get(apel))
        .route("/voyages/apel.csv", get(apel_csv))
}

async fn liste(
    State(s): State<AppState>,
    session: EntSession,
    Query(f): Query<FiltreVoyages>,
) -> Res<Json<Vec<SortieResume>>> {
    Ok(Json(
        s.ent
            .voyages_liste(&acteur(&session), f)
            .await
            .map_err(map_db)?,
    ))
}

async fn creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<SortieSaisie>,
) -> Res<(StatusCode, Json<SortieLigne>)> {
    let v = s
        .ent
        .voyage_creer(&acteur(&session), b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn serie_creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<SerieSaisie>,
) -> Res<(StatusCode, Json<ResultatSerie>)> {
    let v = s
        .ent
        .voyage_serie_creer(&acteur(&session), b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn serie_valider(
    State(s): State<AppState>,
    session: EntSession,
    Path(serie_id): Path<Uuid>,
) -> Res<Json<Vec<ResultatSeance>>> {
    Ok(Json(
        s.ent
            .voyage_serie_valider(&acteur(&session), serie_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn tableau(State(s): State<AppState>, session: EntSession) -> Res<Json<TableauDirection>> {
    Ok(Json(
        s.ent
            .voyages_tableau(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}

async fn famille(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<VoyageFamille>>> {
    Ok(Json(
        s.ent
            .voyages_famille(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}

async fn detail(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<SortieDetail>> {
    Ok(Json(
        s.ent
            .voyage_detail(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct ModifBody {
    /// Version lue : refusée si le dossier a changé depuis (verrou optimiste).
    version: i32,
    #[serde(flatten)]
    saisie: SortieSaisie,
}

async fn modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ModifBody>,
) -> Res<Json<SortieLigne>> {
    Ok(Json(
        s.ent
            .voyage_modifier(&acteur(&session), id, b.version, b.saisie)
            .await
            .map_err(map_db)?,
    ))
}

fn commande_de(code: &str) -> Option<Commande> {
    Some(match code {
        "soumettre" => Commande::Soumettre,
        "valider-pedagogie" => Commande::ValiderPedagogie,
        "demander-modifications" => Commande::DemanderModifications,
        "annuler-demande" => Commande::AnnulerDemande,
        "renvoyer" => Commande::Renvoyer,
        "refuser" => Commande::Refuser,
        "demander-signature" => Commande::DemanderSignature,
        "signer-devis" => Commande::SignerDevis,
        "passer-finances" => Commande::PasserAuxFinances,
        "valider-budget" => Commande::ValiderBudget,
        "autoriser" => Commande::Autoriser,
        "confirmer-liste" => Commande::ConfirmerListe,
        "rouvrir" => Commande::Rouvrir,
        "annuler" => Commande::Annuler,
        "requalifier" => Commande::Requalifier,
        _ => return None,
    })
}

async fn commande(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, cmd)): Path<(Uuid, String)>,
    body: Option<Json<ParamsCommande>>,
) -> Res<Json<SortieLigne>> {
    let cmd = commande_de(&cmd).ok_or((StatusCode::NOT_FOUND, "commande inconnue".to_string()))?;
    let p = body.map(|Json(p)| p).unwrap_or_default();
    Ok(Json(
        s.ent
            .voyage_commande(&acteur(&session), id, cmd, p)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct PorteurBody {
    /// Agent du personnel (annuaire du socle 6).
    agent_id: Uuid,
}

async fn porteur(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PorteurBody>,
) -> Res<Json<SortieLigne>> {
    Ok(Json(
        s.ent
            .voyage_changer_porteur(&acteur(&session), id, b.agent_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn programme(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<Vec<EtapeSaisie>>,
) -> Res<Json<Vec<EtapeVue>>> {
    Ok(Json(
        s.ent
            .voyage_etapes_remplacer(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn audit(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<PointAudit>>> {
    Ok(Json(
        s.ent
            .voyage_audit(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn journal(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<JournalVue>>> {
    Ok(Json(
        s.ent
            .voyage_journal(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct CandidatsQuery {
    #[serde(default)]
    classe_id: Option<Uuid>,
}

async fn candidats(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<CandidatsQuery>,
) -> Res<Json<Vec<CandidatsClasse>>> {
    Ok(Json(
        s.ent
            .voyage_candidats(&acteur(&session), id, q.classe_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn participants(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<ParticipantVue>>> {
    Ok(Json(
        s.ent
            .voyage_participants(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct AjoutBody {
    eleve_ids: Vec<Uuid>,
}

async fn participants_ajouter(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AjoutBody>,
) -> Res<Json<ResultatAjout>> {
    Ok(Json(
        s.ent
            .voyage_participants_ajouter(&acteur(&session), id, b.eleve_ids)
            .await
            .map_err(map_db)?,
    ))
}

async fn participants_csv(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<impl IntoResponse> {
    let csv = s
        .ent
        .voyage_participants_csv(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"participants.csv\"",
            ),
            (header::CACHE_CONTROL, "no-store"),
        ],
        csv,
    ))
}

#[derive(Debug, Deserialize)]
struct MotifBody {
    motif_code: String,
    #[serde(default)]
    motif_texte: Option<String>,
}

async fn desistement(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, pid)): Path<(Uuid, Uuid)>,
    Json(b): Json<MotifBody>,
) -> Res<StatusCode> {
    s.ent
        .voyage_participant_desister(
            &acteur(&session),
            id,
            pid,
            &b.motif_code,
            b.motif_texte.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn retrait(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, pid)): Path<(Uuid, Uuid)>,
    Json(b): Json<MotifBody>,
) -> Res<StatusCode> {
    s.ent
        .voyage_participant_retirer(
            &acteur(&session),
            id,
            pid,
            &b.motif_code,
            b.motif_texte.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct EncadrantBody {
    /// Agent du personnel (annuaire du socle 6) : jamais un nom libre.
    agent_id: Uuid,
    role: String,
}

async fn encadrant_ajouter(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<EncadrantBody>,
) -> Res<(StatusCode, Json<EncadrantVue>)> {
    let v = s
        .ent
        .voyage_encadrant_ajouter(&acteur(&session), id, b.agent_id, &b.role)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn encadrant_retirer(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, eid)): Path<(Uuid, Uuid)>,
) -> Res<StatusCode> {
    s.ent
        .voyage_encadrant_retirer(&acteur(&session), id, eid)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn circulaire(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Circulaire>> {
    Ok(Json(
        s.ent
            .voyage_circulaire(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

// ───────────────────────── Lot V0b : budget, prix annoncé, PDF, rappels, APEL ─────────────────────────

async fn budget(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<BudgetVue>> {
    Ok(Json(
        s.ent
            .voyage_budget(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Default, Deserialize)]
struct VersionBody {
    #[serde(default)]
    motif: Option<String>,
}

async fn budget_version_creer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<VersionBody>>,
) -> Res<(StatusCode, Json<BudgetVue>)> {
    let b = body.map(|Json(b)| b).unwrap_or_default();
    let v = s
        .ent
        .voyage_budget_version_creer(&acteur(&session), id, b.motif.as_deref())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

/// Version d'écritures lue par l'appelant (verrou optimiste du budget).
#[derive(Debug, Deserialize)]
struct RevQuery {
    rev: i32,
}

async fn budget_version_abandonner(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, numero)): Path<(Uuid, i32)>,
    Query(q): Query<RevQuery>,
) -> Res<Json<BudgetVue>> {
    Ok(Json(
        s.ent
            .voyage_budget_version_abandonner(&acteur(&session), id, numero, q.rev)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct ParamsBody {
    rev: i32,
    #[serde(flatten)]
    params: ParamsBudget,
}

async fn budget_parametrer(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, numero)): Path<(Uuid, i32)>,
    Json(b): Json<ParamsBody>,
) -> Res<Json<BudgetVue>> {
    Ok(Json(
        s.ent
            .voyage_budget_parametrer(&acteur(&session), id, numero, b.rev, b.params)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct LigneBody {
    rev: i32,
    #[serde(flatten)]
    ligne: LigneSaisie,
}

async fn budget_ligne_ajouter(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, numero)): Path<(Uuid, i32)>,
    Json(b): Json<LigneBody>,
) -> Res<(StatusCode, Json<BudgetVue>)> {
    let v = s
        .ent
        .voyage_budget_ligne_ajouter(&acteur(&session), id, numero, b.rev, b.ligne)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn budget_ligne_modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, numero, lid)): Path<(Uuid, i32, Uuid)>,
    Json(b): Json<LigneBody>,
) -> Res<Json<BudgetVue>> {
    Ok(Json(
        s.ent
            .voyage_budget_ligne_modifier(&acteur(&session), id, numero, b.rev, lid, b.ligne)
            .await
            .map_err(map_db)?,
    ))
}

async fn budget_ligne_supprimer(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, numero, lid)): Path<(Uuid, i32, Uuid)>,
    Query(q): Query<RevQuery>,
) -> Res<Json<BudgetVue>> {
    Ok(Json(
        s.ent
            .voyage_budget_ligne_supprimer(&acteur(&session), id, numero, q.rev, lid)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct RevBody {
    rev: i32,
}

async fn budget_valider_revision(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, numero)): Path<(Uuid, i32)>,
    Json(b): Json<RevBody>,
) -> Res<Json<BudgetVue>> {
    Ok(Json(
        s.ent
            .voyage_budget_valider_revision(&acteur(&session), id, numero, b.rev)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct HausseBody {
    motif: String,
}

async fn prix_hausser(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<HausseBody>,
) -> Res<Json<BudgetVue>> {
    Ok(Json(
        s.ent
            .voyage_prix_hausser(&acteur(&session), id, &b.motif)
            .await
            .map_err(map_db)?,
    ))
}

async fn budget_pdf(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<(StatusCode, Json<DocumentVue>)> {
    let d = s
        .ent
        .voyage_budget_pdf(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(d)))
}

async fn circulaire_pdf(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<(StatusCode, Json<DocumentVue>)> {
    let d = s
        .ent
        .voyage_circulaire_pdf(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(d)))
}

async fn documents(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<DocumentVue>>> {
    Ok(Json(
        s.ent
            .voyage_documents(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

/// Le document sort **par l'application**, après contrôle du droit et de l'empreinte : jamais
/// d'URL publique du magasin d'objets (R-SV28).
async fn document(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, doc)): Path<(Uuid, Uuid)>,
) -> Res<axum::response::Response> {
    let (nom, octets) = s
        .ent
        .voyage_document_lire(&acteur(&session), id, doc)
        .await
        .map_err(map_db)?;
    crate::routes::attachment_response(octets, &nom, "application/pdf")
}

async fn rappels(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<RappelVue>>> {
    Ok(Json(
        s.ent
            .voyage_rappels(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, serde::Serialize)]
struct RappelsCrees {
    crees: usize,
}

async fn rappels_traiter(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<RappelsCrees>> {
    let crees = s
        .ent
        .voyages_rappels_declencher(&acteur(&session), chrono::Utc::now())
        .await
        .map_err(map_db)?;
    Ok(Json(RappelsCrees { crees }))
}

#[derive(Debug, Deserialize)]
struct AnneeQuery {
    #[serde(default)]
    annee_id: Option<Uuid>,
}

async fn apel(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AnneeQuery>,
) -> Res<Json<SyntheseApel>> {
    Ok(Json(
        s.ent
            .voyages_synthese_apel(&acteur(&session), q.annee_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn apel_csv(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AnneeQuery>,
) -> Res<impl IntoResponse> {
    let csv = s
        .ent
        .voyages_synthese_apel_csv(&acteur(&session), q.annee_id)
        .await
        .map_err(map_db)?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"synthese-apel.csv\"",
            ),
            (header::CACHE_CONTROL, "no-store"),
        ],
        csv,
    ))
}
