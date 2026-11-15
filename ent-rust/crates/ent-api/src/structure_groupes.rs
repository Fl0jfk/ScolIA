//! Routes `/structure/groupes*`, `/structure/affectations*`, `/structure/eleves/...` du socle 5, lot S6 : groupes,
//! composition datée, changement de classe, règles de composition, dédoublement, correction.
//!
//! Aucune règle ici : droits (`referentiel.lire` pour la liste ; `structure.gerer`, `affectations.gerer`,
//! `groupes.composer`, `groupes.creer_activite` pour l'écriture, `referentiel.gerer` les couvre), cloisonnement par
//! l'établissement de la session, jour de l'établissement et validations sont dans `ent-db`. Les commandes ★
//! (changement de classe, lot, dédoublement, correction) sont des **aperçus par défaut** : `apercu: false` pour écrire.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_db::structure_composition::{
    AAffecter, ApercuChangementClasse, ApplicationRegle, ChangementClasseSaisie,
    CompositionResultat, CompositionSaisie, CorrectionResultat, CorrectionSaisie,
    DedoublementResultat, DedoublementSaisie, DemiGroupesResultat, DemiGroupesSaisie, LotResultat,
    LotSaisie, PropositionsGroupe,
};
use ent_db::structure_groupes::{
    GenerationResultat, GenerationSaisie, GroupeDetail, GroupeFerme, GroupeModif, GroupeSaisie,
    GroupeVue,
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
        .route("/structure/groupes", get(liste).post(creer))
        .route("/structure/groupes/generer", post(generer))
        .route("/structure/groupes/demi-groupes", post(demi_groupes))
        .route("/structure/groupes/activites", post(activite))
        .route("/structure/groupes/{id}", get(detail).put(modifier))
        .route("/structure/groupes/{id}/fermer", post(fermer))
        .route("/structure/groupes/{id}/composition", post(composition))
        .route("/structure/groupes/{id}/propositions", get(propositions))
        .route("/structure/groupes/{id}/regle", post(regle))
        .route("/structure/groupes/{id}/dedoubler", post(dedoubler))
        .route("/structure/eleves/classe", post(changer_classe))
        .route("/structure/affectations/lot", post(lot))
        .route("/structure/affectations/correction", post(correction))
        .route("/structure/a-affecter", get(a_affecter))
}

#[derive(Debug, Deserialize)]
struct ListeQuery {
    annee_id: Option<Uuid>,
    classe_id: Option<Uuid>,
    nature: Option<String>,
    #[serde(default)]
    fermes: bool,
    jour: Option<NaiveDate>,
}

async fn liste(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<ListeQuery>,
) -> Res<Json<Vec<GroupeVue>>> {
    Ok(Json(
        s.ent
            .structure_groupes_liste(
                &acteur(&session),
                q.annee_id,
                q.classe_id,
                q.nature.as_deref(),
                q.fermes,
                q.jour,
            )
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct JourQuery {
    jour: Option<NaiveDate>,
}

async fn detail(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<JourQuery>,
) -> Res<Json<GroupeDetail>> {
    Ok(Json(
        s.ent
            .structure_groupe_detail(&acteur(&session), id, q.jour)
            .await
            .map_err(map_db)?,
    ))
}

async fn creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<GroupeSaisie>,
) -> Res<(StatusCode, Json<GroupeVue>)> {
    let g = s
        .ent
        .structure_groupe_creer(&acteur(&session), b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(g)))
}

async fn activite(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<GroupeSaisie>,
) -> Res<(StatusCode, Json<GroupeVue>)> {
    let g = s
        .ent
        .structure_activite_creer(&acteur(&session), b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(g)))
}

async fn generer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<GenerationSaisie>,
) -> Res<Json<GenerationResultat>> {
    Ok(Json(
        s.ent
            .structure_groupes_generer(&acteur(&session), b)
            .await
            .map_err(map_db)?,
    ))
}

async fn demi_groupes(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DemiGroupesSaisie>,
) -> Res<(StatusCode, Json<DemiGroupesResultat>)> {
    let r = s
        .ent
        .structure_demi_groupes_creer(&acteur(&session), b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<GroupeModif>,
) -> Res<Json<GroupeVue>> {
    Ok(Json(
        s.ent
            .structure_groupe_modifier(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct FermerBody {
    /// Dernier jour du groupe (inclus) ; défaut : aujourd'hui.
    a_la_date: Option<NaiveDate>,
    #[serde(default)]
    apercu: bool,
}

async fn fermer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<FermerBody>,
) -> Res<Json<GroupeFerme>> {
    Ok(Json(
        s.ent
            .structure_groupe_fermer(&acteur(&session), id, b.a_la_date, b.apercu)
            .await
            .map_err(map_db)?,
    ))
}

async fn composition(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CompositionSaisie>,
) -> Res<Json<CompositionResultat>> {
    Ok(Json(
        s.ent
            .structure_composition_modifier(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn propositions(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<JourQuery>,
) -> Res<Json<PropositionsGroupe>> {
    Ok(Json(
        s.ent
            .structure_groupe_propositions(&acteur(&session), id, q.jour)
            .await
            .map_err(map_db)?,
    ))
}

async fn regle(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ApplicationRegle>,
) -> Res<Json<CompositionResultat>> {
    Ok(Json(
        s.ent
            .structure_groupe_regle_appliquer(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn dedoubler(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DedoublementSaisie>,
) -> Res<Json<DedoublementResultat>> {
    Ok(Json(
        s.ent
            .structure_groupe_dedoubler(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn changer_classe(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ChangementClasseSaisie>,
) -> Res<Json<ApercuChangementClasse>> {
    Ok(Json(
        s.ent
            .structure_eleve_classe_changer(&acteur(&session), b)
            .await
            .map_err(map_db)?,
    ))
}

async fn lot(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<LotSaisie>,
) -> Res<Json<LotResultat>> {
    Ok(Json(
        s.ent
            .structure_affectations_lot(&acteur(&session), b)
            .await
            .map_err(map_db)?,
    ))
}

async fn correction(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CorrectionSaisie>,
) -> Res<Json<CorrectionResultat>> {
    Ok(Json(
        s.ent
            .structure_affectation_corriger(&acteur(&session), b)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct AAffecterQuery {
    annee_id: Option<Uuid>,
    jour: Option<NaiveDate>,
}

async fn a_affecter(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AAffecterQuery>,
) -> Res<Json<AAffecter>> {
    Ok(Json(
        s.ent
            .structure_a_affecter(&acteur(&session), q.annee_id, q.jour)
            .await
            .map_err(map_db)?,
    ))
}
