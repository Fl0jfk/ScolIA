//! Routes du socle 18, lots FO2–FO3 : formulaires (création par les personnels, versions, publication, validation, réponse des
//! familles et des élèves, questions « accord ») et accords sur objets (règle, désaccord, dossier de preuve).
//!
//! L'autorité est `ent-db` (moteur de droits D-017 : `formulaires.creer`, `formulaires.valider`, `formulaires.gerer`,
//! `autorisations.gerer`, `autorisations.parametrer`) ; côté famille et élève, le répondant est **calculé** par le lien
//! communicant et le compte de l'élève, jamais choisi. Ces routes ne font que traduire HTTP ↔ service. Une référence d'un autre
//! établissement ou d'une autre famille répond 404. L'ancien module (`communication_s28.rs`, préfixe `/formulaires`) reste en
//! service jusqu'à son retrait (FO13) : le nouveau moteur est sous `/frm`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use ent_db::accords::{DecisionSaisie, MaDecision};
use ent_db::accords_fo3::{DesaccordVue, DossierPreuve};
use ent_db::frm_moteur::{
    ApercuAudience, BilanPublication, CritereAudience, FormulaireVue, NouveauFormulaire,
    NouvelleCondition, NouvelleQuestion, NouvelleSection, StructureVue,
};
use ent_db::frm_reponse::{ARepondre, RecuReponse, ReponseSaisie};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Création (FRM-120 à FRM-139, FRM-145, FRM-149) : personnels, selon `formulaires.creer` et la portée.
        .route("/frm", get(lister).post(creer))
        .route("/frm/modeles", get(modeles))
        .route("/frm/modeles/{code}", post(depuis_modele))
        .route("/frm/a-repondre", get(a_repondre))
        .route("/frm/{id}", get(lire).delete(supprimer))
        .route("/frm/{id}/reglages", post(regler))
        .route("/frm/{id}/structure", get(structure))
        .route("/frm/{id}/sections", post(ajouter_section))
        .route("/frm/{id}/questions", post(ajouter_question))
        .route(
            "/frm/{id}/questions/{question_id}",
            delete(retirer_question),
        )
        .route(
            "/frm/{id}/questions/{question_id}/libelle",
            post(corriger_libelle),
        )
        .route("/frm/{id}/conditions", post(ajouter_condition))
        .route(
            "/frm/{id}/audience",
            get(apercu_audience).put(definir_audience),
        )
        .route("/frm/{id}/publier", post(publier))
        .route("/frm/{id}/valider", post(valider))
        .route("/frm/{id}/refuser-validation", post(refuser_validation))
        .route("/frm/{id}/retirer-validation", post(retirer_validation))
        .route("/frm/{id}/versions", post(nouvelle_version))
        .route("/frm/{id}/dupliquer", post(dupliquer))
        .route("/frm/{id}/clore", post(clore))
        .route("/frm/{id}/rouvrir", post(rouvrir))
        .route("/frm/{id}/archiver", post(archiver))
        .route(
            "/frm/{id}/co-auteurs/{user_id}",
            put(ajouter_co_auteur).delete(retirer_co_auteur),
        )
        // Répondre : le répondant est calculé (titulaire communicant, élève selon l'unité et la capacité).
        .route("/frm/{id}/eleves/{eleve_id}", get(pour_repondre))
        .route("/frm/{id}/repondre", post(repondre))
        // Accords sur objets (FO3) : règle, désaccord, dossier de preuve.
        .route("/accords/objets/{code}/regle", post(relever_regle))
        .route("/accords/objets/{code}/documents", post(lier_document))
        .route("/accords/desaccords", post(declarer_desaccord))
        .route("/accords/desaccords/{id}/lever", post(lever_desaccord))
        .route("/accords/eleves/{eleve_id}/desaccords", get(desaccords))
        .route("/accords/decider-signe", post(decider_signe))
        .route(
            "/accords/decisions/{id}/dossier-preuve",
            get(dossier_preuve),
        )
}

// ─── Création ────────────────────────────────────────────────────────────────────────────────────────

async fn lister(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<FormulaireVue>>> {
    s.ent
        .frm_lister(se.etablissement_id, se.user_id, &se.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creer(
    State(s): State<AppState>,
    se: EntSession,
    Json(n): Json<NouveauFormulaire>,
) -> Res<(StatusCode, Json<FormulaireVue>)> {
    s.ent
        .frm_creer(se.etablissement_id, se.user_id, &se.role, n)
        .await
        .map(|f| (StatusCode::CREATED, Json(f)))
        .map_err(map_db)
}

#[derive(Debug, Serialize)]
struct ModeleVue {
    code: String,
    titre: String,
    description: String,
    nature: String,
    unite: String,
}

async fn modeles(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<ModeleVue>>> {
    s.ent
        .frm_modeles(se.etablissement_id, se.user_id, &se.role)
        .await
        .map(|l| {
            Json(
                l.into_iter()
                    .map(|(code, titre, description, nature, unite)| ModeleVue {
                        code,
                        titre,
                        description,
                        nature,
                        unite,
                    })
                    .collect(),
            )
        })
        .map_err(map_db)
}

#[derive(Debug, Default, Deserialize)]
struct CorpsModele {
    #[serde(default)]
    titre: Option<String>,
}

async fn depuis_modele(
    State(s): State<AppState>,
    se: EntSession,
    Path(code): Path<String>,
    body: Option<Json<CorpsModele>>,
) -> Res<(StatusCode, Json<FormulaireVue>)> {
    let titre = body.and_then(|Json(b)| b.titre);
    s.ent
        .frm_creer_depuis_modele(se.etablissement_id, se.user_id, &se.role, &code, titre)
        .await
        .map(|f| (StatusCode::CREATED, Json(f)))
        .map_err(map_db)
}

async fn lire(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<FormulaireVue>> {
    s.ent
        .frm_lire(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn supprimer(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .frm_supprimer_brouillon(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct Reglages {
    #[serde(default)]
    titre: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    ouverture: Option<DateTime<Utc>>,
    #[serde(default)]
    echeance: Option<DateTime<Utc>>,
    #[serde(default)]
    relances: Option<String>,
}

async fn regler(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(r): Json<Reglages>,
) -> Res<Json<FormulaireVue>> {
    s.ent
        .frm_regler(
            se.etablissement_id,
            se.user_id,
            &se.role,
            id,
            r.titre,
            r.description,
            r.ouverture,
            r.echeance,
            r.relances,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct ParamsStructure {
    #[serde(default)]
    numero: Option<i32>,
}

async fn structure(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Query(p): Query<ParamsStructure>,
) -> Res<Json<StructureVue>> {
    s.ent
        .frm_structure(se.etablissement_id, se.user_id, &se.role, id, p.numero)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Debug, Serialize)]
struct Cree {
    id: Uuid,
}

async fn ajouter_section(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(n): Json<NouvelleSection>,
) -> Res<(StatusCode, Json<Cree>)> {
    s.ent
        .frm_ajouter_section(se.etablissement_id, se.user_id, &se.role, id, n)
        .await
        .map(|id| (StatusCode::CREATED, Json(Cree { id })))
        .map_err(map_db)
}

async fn ajouter_question(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(q): Json<NouvelleQuestion>,
) -> Res<(StatusCode, Json<Cree>)> {
    s.ent
        .frm_ajouter_question(se.etablissement_id, se.user_id, &se.role, id, q)
        .await
        .map(|id| (StatusCode::CREATED, Json(Cree { id })))
        .map_err(map_db)
}

async fn retirer_question(
    State(s): State<AppState>,
    se: EntSession,
    Path((id, question_id)): Path<(Uuid, Uuid)>,
) -> Res<StatusCode> {
    s.ent
        .frm_retirer_question(se.etablissement_id, se.user_id, &se.role, id, question_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct Libelle {
    libelle: String,
    #[serde(default)]
    aide: Option<String>,
}

async fn corriger_libelle(
    State(s): State<AppState>,
    se: EntSession,
    Path((id, question_id)): Path<(Uuid, Uuid)>,
    Json(l): Json<Libelle>,
) -> Res<StatusCode> {
    s.ent
        .frm_corriger_libelle(
            se.etablissement_id,
            se.user_id,
            &se.role,
            id,
            question_id,
            &l.libelle,
            l.aide.as_deref(),
        )
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn ajouter_condition(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(c): Json<NouvelleCondition>,
) -> Res<(StatusCode, Json<Cree>)> {
    s.ent
        .frm_ajouter_condition(se.etablissement_id, se.user_id, &se.role, id, c)
        .await
        .map(|id| (StatusCode::CREATED, Json(Cree { id })))
        .map_err(map_db)
}

async fn apercu_audience(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ApercuAudience>> {
    s.ent
        .frm_apercu_audience(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn definir_audience(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(c): Json<Vec<CritereAudience>>,
) -> Res<Json<ApercuAudience>> {
    s.ent
        .frm_definir_audience(se.etablissement_id, se.user_id, &se.role, id, c)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Debug, Default, Deserialize)]
struct Publication {
    /// `forme` ou `fond` ; absent : `fond` (prudence), sauf pour une première version.
    #[serde(default)]
    changement: Option<String>,
}

async fn publier(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<Publication>>,
) -> Res<Json<BilanPublication>> {
    let changement = body.and_then(|Json(p)| p.changement);
    s.ent
        .frm_publier(
            se.etablissement_id,
            se.user_id,
            &se.role,
            id,
            changement.as_deref(),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn valider(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<Publication>>,
) -> Res<Json<BilanPublication>> {
    let changement = body.and_then(|Json(p)| p.changement);
    s.ent
        .frm_valider(
            se.etablissement_id,
            se.user_id,
            &se.role,
            id,
            changement.as_deref(),
        )
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct Motif {
    motif: String,
}

async fn refuser_validation(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(m): Json<Motif>,
) -> Res<StatusCode> {
    s.ent
        .frm_refuser_validation(se.etablissement_id, se.user_id, &se.role, id, &m.motif)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn retirer_validation(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .frm_retirer_de_la_validation(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn nouvelle_version(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<(StatusCode, Json<StructureVue>)> {
    s.ent
        .frm_nouvelle_version(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(|v| (StatusCode::CREATED, Json(v)))
        .map_err(map_db)
}

async fn dupliquer(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<(StatusCode, Json<FormulaireVue>)> {
    s.ent
        .frm_dupliquer(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(|f| (StatusCode::CREATED, Json(f)))
        .map_err(map_db)
}

async fn clore(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    s.ent
        .frm_clore(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn rouvrir(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .frm_rouvrir(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn archiver(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .frm_archiver(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn ajouter_co_auteur(
    State(s): State<AppState>,
    se: EntSession,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> Res<StatusCode> {
    s.ent
        .frm_ajouter_co_auteur(se.etablissement_id, se.user_id, &se.role, id, user_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn retirer_co_auteur(
    State(s): State<AppState>,
    se: EntSession,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> Res<StatusCode> {
    s.ent
        .frm_retirer_co_auteur(se.etablissement_id, se.user_id, &se.role, id, user_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

// ─── Répondre ────────────────────────────────────────────────────────────────────────────────────────

async fn a_repondre(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<ARepondre>>> {
    s.ent
        .frm_a_repondre(se.etablissement_id, se.user_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn pour_repondre(
    State(s): State<AppState>,
    se: EntSession,
    Path((id, eleve_id)): Path<(Uuid, Uuid)>,
) -> Res<Json<StructureVue>> {
    s.ent
        .frm_formulaire_pour_repondre(se.etablissement_id, se.user_id, id, eleve_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn repondre(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(r): Json<ReponseSaisie>,
) -> Res<Json<RecuReponse>> {
    s.ent
        .frm_repondre(se.etablissement_id, se.user_id, id, r)
        .await
        .map(Json)
        .map_err(map_db)
}

// ─── Accords sur objets (FO3) ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct Regle {
    regle: String,
}

async fn relever_regle(
    State(s): State<AppState>,
    se: EntSession,
    Path(code): Path<String>,
    Json(r): Json<Regle>,
) -> Res<Json<ent_db::accords::ObjetAccord>> {
    s.ent
        .accords_relever_regle(se.etablissement_id, se.user_id, &se.role, &code, &r.regle)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct LienDocument {
    eleve_id: Uuid,
    document_id: Uuid,
}

async fn lier_document(
    State(s): State<AppState>,
    se: EntSession,
    Path(code): Path<String>,
    Json(l): Json<LienDocument>,
) -> Res<StatusCode> {
    s.ent
        .accords_lier_document(
            se.etablissement_id,
            se.user_id,
            &se.role,
            &code,
            l.eleve_id,
            l.document_id,
        )
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct Declaration {
    eleve_id: Uuid,
    /// `image`, `sortie` ou `general`.
    objet: String,
}

async fn declarer_desaccord(
    State(s): State<AppState>,
    se: EntSession,
    Json(d): Json<Declaration>,
) -> Res<(StatusCode, Json<DesaccordVue>)> {
    s.ent
        .accords_declarer_desaccord(
            se.etablissement_id,
            se.user_id,
            &se.role,
            d.eleve_id,
            &d.objet,
        )
        .await
        .map(|v| (StatusCode::CREATED, Json(v)))
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct Levee {
    piece_document_id: Uuid,
    motif: String,
}

async fn lever_desaccord(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(l): Json<Levee>,
) -> Res<StatusCode> {
    s.ent
        .accords_lever_desaccord(
            se.etablissement_id,
            se.user_id,
            &se.role,
            id,
            l.piece_document_id,
            &l.motif,
        )
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn desaccords(
    State(s): State<AppState>,
    se: EntSession,
    Path(eleve_id): Path<Uuid>,
) -> Res<Json<Vec<DesaccordVue>>> {
    s.ent
        .accords_desaccords(se.etablissement_id, se.user_id, &se.role, eleve_id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Debug, Deserialize)]
struct DecisionSignee {
    #[serde(flatten)]
    decision: DecisionSaisie,
    signature_id: Uuid,
}

async fn decider_signe(
    State(s): State<AppState>,
    se: EntSession,
    Json(d): Json<DecisionSignee>,
) -> Res<Json<MaDecision>> {
    s.ent
        .accords_decider_signe(se.etablissement_id, se.user_id, d.decision, d.signature_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn dossier_preuve(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<DossierPreuve>> {
    s.ent
        .accords_dossier_preuve(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}
