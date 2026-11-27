//! Routes `/structure/*` du socle 5, lot S5 : matières, offre d'enseignements, règles de choix,
//! incompatibilités, prérequis, enseignements suivis par l'élève.
//!
//! Aucune règle ici : droits (`referentiel.lire` / `referentiel.gerer`, périmètre D-022 pour la
//! lecture des enseignements d'un élève), cloisonnement (établissement de la session, jamais du
//! corps), validations et règles de choix sont dans `ent-db`. Un choix refusé par une règle
//! bloquante répond **409** avec la liste des écarts (rien n'est écrit) ; un aperçu répond 200.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_db::structure_enseignements::{
    ApercuFermeture, ComparaisonReprise, DecisionSaisie, EnseignementVue, HistoLigne, Incoherence,
    Modification, ResultatModification,
};
use ent_db::structure_offre::offre::Violation;
use ent_db::structure_offre::{
    IncompatibiliteSaisie, IncompatibiliteVue, MatiereDetail, MatiereModif, MatiereSaisie,
    OffreDetail, OffreModif, OffreSaisie, PrerequisSaisie, RegleSaisie, RegleVue,
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/structure/matieres", get(matieres).post(matiere_creer))
        .route("/structure/matieres/{id}", put(matiere_modifier))
        .route("/structure/matieres/{id}/archiver", post(matiere_archiver))
        .route(
            "/structure/matieres/{id}/desarchiver",
            post(matiere_desarchiver),
        )
        .route("/structure/offres", get(offres).post(offre_creer))
        .route("/structure/offres/{id}", get(offre).put(offre_modifier))
        .route("/structure/offres/{id}/fermer", post(offre_fermer))
        .route("/structure/offres/{id}/rouvrir", post(offre_rouvrir))
        .route(
            "/structure/offres/{id}/groupes",
            post(offre_groupe_associer),
        )
        .route(
            "/structure/offres/{id}/groupes/{groupe_id}",
            delete(offre_groupe_dissocier),
        )
        .route("/structure/offres/{id}/prerequis", post(prerequis_ajouter))
        .route("/structure/prerequis/{id}", delete(prerequis_retirer))
        .route(
            "/structure/regles-choix",
            get(regles_choix).post(regle_creer),
        )
        .route(
            "/structure/regles-choix/{id}/archiver",
            post(regle_archiver),
        )
        .route(
            "/structure/incompatibilites",
            get(incompatibilites).post(incompatibilite_declarer),
        )
        .route(
            "/structure/incompatibilites/{id}/archiver",
            post(incompatibilite_archiver),
        )
        .route(
            "/structure/eleves/{id}/enseignements",
            get(enseignements_eleve).post(enseignements_modifier),
        )
        .route(
            "/structure/eleves/{id}/enseignements/controle",
            get(enseignements_controle),
        )
        .route(
            "/structure/enseignements/{id}/historique",
            get(enseignement_historique),
        )
        .route(
            "/structure/enseignements/{id}/decision",
            post(enseignement_decider),
        )
        .route(
            "/structure/enseignements-incoherences",
            get(enseignements_incoherences),
        )
        .route(
            "/structure/enseignements-reprise",
            get(enseignements_reprise),
        )
}

// ───── Matières ─────

#[derive(Debug, Deserialize)]
struct MatieresQuery {
    #[serde(default)]
    archivees: bool,
}

async fn matieres(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<MatieresQuery>,
) -> Res<Json<Vec<MatiereDetail>>> {
    Ok(Json(
        s.ent
            .structure_matieres(&acteur(&session), q.archivees)
            .await
            .map_err(map_db)?,
    ))
}

async fn matiere_creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<MatiereSaisie>,
) -> Res<(StatusCode, Json<MatiereDetail>)> {
    let m = s
        .ent
        .structure_matiere_creer(&acteur(&session), b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(m)))
}

async fn matiere_modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<MatiereModif>,
) -> Res<Json<MatiereDetail>> {
    Ok(Json(
        s.ent
            .structure_matiere_modifier(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn matiere_archiver(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<MatiereDetail>> {
    Ok(Json(
        s.ent
            .structure_matiere_archiver(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn matiere_desarchiver(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<MatiereDetail>> {
    Ok(Json(
        s.ent
            .structure_matiere_desarchiver(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

// ───── Offre ─────

#[derive(Debug, Deserialize)]
struct AnneeQuery {
    annee_id: Option<Uuid>,
}

async fn offres(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AnneeQuery>,
) -> Res<Json<Vec<OffreDetail>>> {
    Ok(Json(
        s.ent
            .structure_offres(&acteur(&session), q.annee_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn offre(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<OffreDetail>> {
    Ok(Json(
        s.ent
            .structure_offre(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn offre_creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<OffreSaisie>,
) -> Res<(StatusCode, Json<OffreDetail>)> {
    let o = s
        .ent
        .structure_offre_creer(&acteur(&session), b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(o)))
}

async fn offre_modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<OffreModif>,
) -> Res<Json<OffreDetail>> {
    Ok(Json(
        s.ent
            .structure_offre_modifier(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct FermetureBody {
    motif: String,
    effet: Option<NaiveDate>,
    /// Sans `confirmer`, aperçu seulement (qui est touché), rien n'est écrit.
    #[serde(default)]
    confirmer: bool,
}

async fn offre_fermer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<FermetureBody>,
) -> Res<Json<ApercuFermeture>> {
    Ok(Json(
        s.ent
            .structure_offre_fermer(&acteur(&session), id, &b.motif, b.effet, b.confirmer)
            .await
            .map_err(map_db)?,
    ))
}

async fn offre_rouvrir(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<OffreDetail>> {
    Ok(Json(
        s.ent
            .structure_offre_rouvrir(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct GroupeBody {
    groupe_id: Uuid,
}

async fn offre_groupe_associer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<GroupeBody>,
) -> Res<Json<OffreDetail>> {
    Ok(Json(
        s.ent
            .structure_offre_groupe_associer(&acteur(&session), id, b.groupe_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn offre_groupe_dissocier(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, groupe_id)): Path<(Uuid, Uuid)>,
) -> Res<Json<OffreDetail>> {
    Ok(Json(
        s.ent
            .structure_offre_groupe_dissocier(&acteur(&session), id, groupe_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn prerequis_ajouter(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PrerequisSaisie>,
) -> Res<Json<OffreDetail>> {
    Ok(Json(
        s.ent
            .structure_prerequis_ajouter(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn prerequis_retirer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .structure_prerequis_retirer(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

// ───── Règles de choix et incompatibilités ─────

#[derive(Debug, Deserialize)]
struct AnneeRequise {
    annee_id: Uuid,
}

async fn regles_choix(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AnneeRequise>,
) -> Res<Json<Vec<RegleVue>>> {
    Ok(Json(
        s.ent
            .structure_regles_choix(&acteur(&session), q.annee_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn regle_creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<RegleSaisie>,
) -> Res<(StatusCode, Json<RegleVue>)> {
    let r = s
        .ent
        .structure_regle_creer(&acteur(&session), b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn regle_archiver(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .structure_regle_archiver(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn incompatibilites(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AnneeRequise>,
) -> Res<Json<Vec<IncompatibiliteVue>>> {
    Ok(Json(
        s.ent
            .structure_incompatibilites(&acteur(&session), q.annee_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn incompatibilite_declarer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<IncompatibiliteSaisie>,
) -> Res<(StatusCode, Json<IncompatibiliteVue>)> {
    let v = s
        .ent
        .structure_incompatibilite_declarer(&acteur(&session), b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

async fn incompatibilite_archiver(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .structure_incompatibilite_archiver(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

// ───── Enseignements suivis ─────

#[derive(Debug, Deserialize)]
struct EnseignementsQuery {
    annee_id: Option<Uuid>,
    /// Seulement les enseignements suivis ou en attente (sans l'historique quitté, refusé, retiré).
    #[serde(default)]
    actifs: bool,
}

async fn enseignements_eleve(
    State(s): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
    Query(q): Query<EnseignementsQuery>,
) -> Res<Json<Vec<EnseignementVue>>> {
    Ok(Json(
        s.ent
            .structure_enseignements_eleve(&acteur(&session), eleve_id, q.annee_id, q.actifs)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct ModificationBody {
    annee_id: Uuid,
    #[serde(flatten)]
    modification: Modification,
}

async fn enseignements_modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
    Json(b): Json<ModificationBody>,
) -> Result<Json<ResultatModification>, (StatusCode, Json<Value>)> {
    let r = s
        .ent
        .structure_enseignements_modifier(&acteur(&session), eleve_id, b.annee_id, b.modification)
        .await
        .map_err(|e| {
            let (st, msg) = map_db(e);
            (st, Json(serde_json::json!({ "erreur": msg })))
        })?;
    if r.ecrit || r.apercu {
        Ok(Json(r))
    } else {
        // Règle bloquante : rien n'est écrit ; la liste des écarts accompagne le refus.
        Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "erreur": "choix refusé par une règle de l'offre",
                "violations": r.violations,
                "enseignements": r.enseignements,
            })),
        ))
    }
}

#[derive(Debug, Deserialize)]
struct ControleQuery {
    annee_id: Uuid,
}

async fn enseignements_controle(
    State(s): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
    Query(q): Query<ControleQuery>,
) -> Res<Json<Vec<Violation>>> {
    Ok(Json(
        s.ent
            .structure_enseignements_controle_regles(&acteur(&session), q.annee_id, eleve_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn enseignement_historique(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<HistoLigne>>> {
    Ok(Json(
        s.ent
            .structure_enseignement_historique(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn enseignement_decider(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DecisionSaisie>,
) -> Res<Json<EnseignementVue>> {
    Ok(Json(
        s.ent
            .structure_enseignement_decider(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct IncoherencesQuery {
    annee_id: Uuid,
    jour: Option<NaiveDate>,
}

async fn enseignements_incoherences(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<IncoherencesQuery>,
) -> Res<Json<Vec<Incoherence>>> {
    Ok(Json(
        s.ent
            .structure_enseignements_incoherences(&acteur(&session), q.annee_id, q.jour)
            .await
            .map_err(map_db)?,
    ))
}

async fn enseignements_reprise(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<ComparaisonReprise>> {
    Ok(Json(
        s.ent
            .structure_enseignements_comparaison_reprise(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}
