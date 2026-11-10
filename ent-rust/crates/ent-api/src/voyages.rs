//! Routes `/voyages/*` du socle 42, lot V0a : sorties et voyages scolaires collectifs.
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
use ent_db::voyages_repo::{
    CandidatsClasse, Circulaire, EncadrantVue, EtapeSaisie, EtapeVue, FiltreVoyages, JournalVue,
    ParamsCommande, ParticipantVue, ResultatAjout, ResultatSeance, ResultatSerie, SerieSaisie,
    SortieDetail, SortieLigne, SortieResume, SortieSaisie, TableauDirection, VoyageFamille,
};
use ent_db::voyages::PointAudit;
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
        .route("/voyages/{id}/participants", get(participants).post(participants_ajouter))
        .route("/voyages/{id}/participants.csv", get(participants_csv))
        .route("/voyages/{id}/participants/{pid}/desistement", post(desistement))
        .route("/voyages/{id}/participants/{pid}/retrait", post(retrait))
        .route("/voyages/{id}/encadrants", post(encadrant_ajouter))
        .route("/voyages/{id}/encadrants/{eid}", delete(encadrant_retirer))
        .route("/voyages/{id}/circulaire", post(circulaire))
}

async fn liste(
    State(s): State<AppState>,
    session: EntSession,
    Query(f): Query<FiltreVoyages>,
) -> Res<Json<Vec<SortieResume>>> {
    Ok(Json(s.ent.voyages_liste(&acteur(&session), f).await.map_err(map_db)?))
}

async fn creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<SortieSaisie>,
) -> Res<(StatusCode, Json<SortieLigne>)> {
    let v = s.ent.voyage_creer(&acteur(&session), b).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn serie_creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<SerieSaisie>,
) -> Res<(StatusCode, Json<ResultatSerie>)> {
    let v = s.ent.voyage_serie_creer(&acteur(&session), b).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn serie_valider(
    State(s): State<AppState>,
    session: EntSession,
    Path(serie_id): Path<Uuid>,
) -> Res<Json<Vec<ResultatSeance>>> {
    Ok(Json(
        s.ent.voyage_serie_valider(&acteur(&session), serie_id).await.map_err(map_db)?,
    ))
}

async fn tableau(State(s): State<AppState>, session: EntSession) -> Res<Json<TableauDirection>> {
    Ok(Json(s.ent.voyages_tableau(&acteur(&session)).await.map_err(map_db)?))
}

async fn famille(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<VoyageFamille>>> {
    Ok(Json(s.ent.voyages_famille(&acteur(&session)).await.map_err(map_db)?))
}

async fn detail(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<SortieDetail>> {
    Ok(Json(s.ent.voyage_detail(&acteur(&session), id).await.map_err(map_db)?))
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
    Ok(Json(s.ent.voyage_audit(&acteur(&session), id).await.map_err(map_db)?))
}

async fn journal(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<JournalVue>>> {
    Ok(Json(s.ent.voyage_journal(&acteur(&session), id).await.map_err(map_db)?))
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
        s.ent.voyage_participants(&acteur(&session), id).await.map_err(map_db)?,
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
            (header::CONTENT_DISPOSITION, "attachment; filename=\"participants.csv\""),
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
        .voyage_participant_desister(&acteur(&session), id, pid, &b.motif_code, b.motif_texte.as_deref())
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
        .voyage_participant_retirer(&acteur(&session), id, pid, &b.motif_code, b.motif_texte.as_deref())
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
    Ok(Json(s.ent.voyage_circulaire(&acteur(&session), id).await.map_err(map_db)?))
}
