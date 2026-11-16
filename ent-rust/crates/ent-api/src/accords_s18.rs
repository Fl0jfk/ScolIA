//! Routes du socle 18, lots FO0–FO1 : objets d'accord, versions, décisions, retrait, « mes accords », port `verifier`,
//! reprise de l'existant.
//!
//! L'autorité est `ent-db` (moteur de droits D-017 : `autorisations.lire`, `autorisations.gerer`,
//! `autorisations.parametrer`) ; côté famille et élève, l'accès passe par le **lien communicant** et par le compte
//! (le décideur est calculé, jamais choisi). Ces routes ne font que traduire HTTP ↔ service. Une référence d'un autre
//! établissement ou d'une autre famille répond 404. Une famille ne reçoit jamais la décision d'un autre titulaire.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_db::accords::{
    BilanReprise, DecisionDetail, DecisionSaisie, DemandeVerification, EntreeCatalogue, MaDecision, MesAccordsSujet,
    NouvelObjet, ObjetAccord, RedactionVersion, UsageAccord, VerdictEleve, VerdictSujet, VersionAccord,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Catalogue et objets (FRM-100 à FRM-109) : direction et secrétariat.
        .route("/accords/catalogue", get(catalogue))
        .route("/accords/objets", get(objets).post(creer_objet))
        .route("/accords/objets/{code}/activer", post(activer))
        .route("/accords/objets/{code}/desactiver", post(desactiver))
        .route("/accords/objets/{code}/usages", get(usages))
        .route("/accords/objets/{code}/versions", get(historique).post(rediger))
        .route("/accords/objets/{code}/versions/publier", post(publier))
        .route("/accords/objets/{code}/versions/{numero}/relire", post(relire))
        .route("/accords/objets/{code}/apercu/{profil}", get(apercu))
        .route("/accords/objets/{code}/eleves/{eleve_id}/decisions", get(decisions))
        // Décider, retirer, mes accords (FRM-113 à FRM-117) : le décideur est calculé.
        .route("/accords/mes", get(mes_accords))
        .route("/accords/decider", post(decider))
        .route("/accords/retirer", post(retirer))
        // Port `verifier` pour un acteur humain, et grille d'une classe.
        .route("/accords/verifier", post(verifier))
        .route("/classes/{classe_id}/autorisations", get(grille_classe))
        // Reprise de l'existant (rejouable).
        .route("/accords/reprise/formulaires", post(reprise_formulaires))
        .route("/accords/reprise/consentements", post(reprise_consentements))
}

async fn catalogue(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<EntreeCatalogue>>> {
    s.ent.accords_catalogue(se.etablissement_id, se.user_id, &se.role).await.map(Json).map_err(map_db)
}

async fn objets(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<ObjetAccord>>> {
    s.ent.accords_objets(se.etablissement_id, se.user_id, &se.role).await.map(Json).map_err(map_db)
}

async fn creer_objet(
    State(s): State<AppState>,
    se: EntSession,
    Json(n): Json<NouvelObjet>,
) -> Res<(StatusCode, Json<ObjetAccord>)> {
    s.ent
        .accords_creer_objet(se.etablissement_id, se.user_id, &se.role, n)
        .await
        .map(|o| (StatusCode::CREATED, Json(o)))
        .map_err(map_db)
}

async fn activer(State(s): State<AppState>, se: EntSession, Path(code): Path<String>) -> Res<Json<ObjetAccord>> {
    s.ent.accords_activer_objet(se.etablissement_id, se.user_id, &se.role, &code).await.map(Json).map_err(map_db)
}

async fn desactiver(State(s): State<AppState>, se: EntSession, Path(code): Path<String>) -> Res<StatusCode> {
    s.ent
        .accords_desactiver_objet(se.etablissement_id, se.user_id, &se.role, &code)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn usages(State(s): State<AppState>, se: EntSession, Path(code): Path<String>) -> Res<Json<Vec<UsageAccord>>> {
    s.ent.accords_usages(se.etablissement_id, se.user_id, &se.role, &code).await.map(Json).map_err(map_db)
}

async fn historique(State(s): State<AppState>, se: EntSession, Path(code): Path<String>) -> Res<Json<Vec<VersionAccord>>> {
    s.ent.accords_historique(se.etablissement_id, se.user_id, &se.role, &code).await.map(Json).map_err(map_db)
}

async fn rediger(
    State(s): State<AppState>,
    se: EntSession,
    Path(code): Path<String>,
    Json(r): Json<RedactionVersion>,
) -> Res<Json<VersionAccord>> {
    s.ent.accords_rediger_version(se.etablissement_id, se.user_id, &se.role, &code, r).await.map(Json).map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct Publication {
    /// `forme`, `fond` ou `restriction` ; absent : `fond` (prudence), sauf pour une première version.
    #[serde(default)]
    changement: Option<String>,
}

async fn publier(
    State(s): State<AppState>,
    se: EntSession,
    Path(code): Path<String>,
    body: Option<Json<Publication>>,
) -> Res<Json<VersionAccord>> {
    let changement = body.and_then(|Json(p)| p.changement);
    s.ent
        .accords_publier_version(se.etablissement_id, se.user_id, &se.role, &code, changement.as_deref())
        .await
        .map(Json)
        .map_err(map_db)
}

async fn relire(
    State(s): State<AppState>,
    se: EntSession,
    Path((code, numero)): Path<(String, i32)>,
) -> Res<Json<VersionAccord>> {
    s.ent.accords_relire_version(se.etablissement_id, se.user_id, &se.role, &code, numero).await.map(Json).map_err(map_db)
}

#[derive(Debug, Serialize)]
struct Apercu {
    texte: String,
    usages: Vec<UsageAccord>,
    texte_provisoire: bool,
}

async fn apercu(
    State(s): State<AppState>,
    se: EntSession,
    Path((code, profil)): Path<(String, String)>,
) -> Res<Json<Apercu>> {
    s.ent
        .accords_apercu(se.etablissement_id, se.user_id, &se.role, &code, &profil)
        .await
        .map(|(texte, usages, texte_provisoire)| Json(Apercu { texte, usages, texte_provisoire }))
        .map_err(map_db)
}

async fn decisions(
    State(s): State<AppState>,
    se: EntSession,
    Path((code, eleve_id)): Path<(String, Uuid)>,
) -> Res<Json<Vec<DecisionDetail>>> {
    s.ent
        .accords_lire_decisions(se.etablissement_id, se.user_id, &se.role, &code, eleve_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn mes_accords(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<MesAccordsSujet>>> {
    s.ent.accords_mes_accords(se.etablissement_id, se.user_id).await.map(Json).map_err(map_db)
}

async fn decider(State(s): State<AppState>, se: EntSession, Json(d): Json<DecisionSaisie>) -> Res<Json<MaDecision>> {
    s.ent.accords_decider(se.etablissement_id, se.user_id, d).await.map(Json).map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct Retrait {
    objet_code: String,
    #[serde(default)]
    eleve_id: Option<Uuid>,
    /// Usages à retirer ; absent : tout l'accord.
    #[serde(default)]
    usages: Option<Vec<String>>,
}

async fn retirer(State(s): State<AppState>, se: EntSession, Json(r): Json<Retrait>) -> Res<Json<MaDecision>> {
    s.ent
        .accords_retirer(se.etablissement_id, se.user_id, &r.objet_code, r.eleve_id, r.usages)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn verifier(
    State(s): State<AppState>,
    se: EntSession,
    Json(d): Json<DemandeVerification>,
) -> Res<Json<Vec<VerdictSujet>>> {
    s.ent.accords_verifier_pour_acteur(se.etablissement_id, se.user_id, &se.role, &d).await.map(Json).map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct ParamsGrille {
    objet: String,
    usage: String,
}

async fn grille_classe(
    State(s): State<AppState>,
    se: EntSession,
    Path(classe_id): Path<Uuid>,
    Query(p): Query<ParamsGrille>,
) -> Res<Json<Vec<VerdictEleve>>> {
    s.ent
        .accords_verdicts_classe(se.etablissement_id, se.user_id, &se.role, &p.objet, &p.usage, classe_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn reprise_formulaires(State(s): State<AppState>, se: EntSession) -> Res<Json<BilanReprise>> {
    s.ent.accords_reprendre_formulaires(se.etablissement_id, se.user_id, &se.role).await.map(Json).map_err(map_db)
}

async fn reprise_consentements(State(s): State<AppState>, se: EntSession) -> Res<Json<BilanReprise>> {
    s.ent
        .accords_reprendre_consentements_personnes(se.etablissement_id, se.user_id, &se.role)
        .await
        .map(Json)
        .map_err(map_db)
}
