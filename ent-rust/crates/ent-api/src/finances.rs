//! Routes des finances reliées (socle 2, lot S2-9 ; FIN-01 → FIN-17) et des
//! déclencheurs de scolarité (changement de régime, sortie).
//!
//! L'autorité est `ent-db` (`finances.rs`, `finances_sepa.rs`) : droits
//! `facturation.lire` / `facturation.gerer` / `bourses.gerer` / `eleves.gerer`
//! (moteur D-017), établissement de la session, pièces des seuls foyers payeurs
//! pour l'espace famille. Types JSON : `ent_types::finances`.
//!
//! Paiement en ligne (FIN-09) : interface seulement, **désactivée** (Q5 non
//! décidée) ; la route d'intention répond 409.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::finances::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::{attachment_response, map_db, tracer_acces};
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Paramétrage (FIN-01, FIN-05, FIN-07).
        .route(
            "/finances/parametres",
            get(parametres).put(modifier_parametres),
        )
        .route("/finances/articles", get(articles).post(creer_article))
        .route("/finances/articles/{id}", put(modifier_article))
        .route("/finances/tarifs", get(tarifs).post(creer_tarif))
        .route("/finances/tarifs/{id}", delete(supprimer_tarif))
        .route(
            "/finances/reductions",
            get(reductions).post(creer_reduction),
        )
        .route("/finances/reductions/{id}/eleves", post(accorder_reduction))
        .route("/finances/bourses/baremes", get(baremes).post(poser_bareme))
        .route(
            "/finances/echeancier-type",
            get(echeancier_type).put(poser_echeancier_type),
        )
        // Génération et facturation (FIN-02 → FIN-06).
        .route("/finances/generation", post(generer))
        .route("/finances/consommations", post(consommation))
        .route("/finances/factures", get(factures))
        .route("/finances/factures/emettre", post(emettre))
        .route("/finances/factures/{id}", get(facture))
        .route("/finances/factures/{id}/avoir", post(avoir))
        // SEPA (FIN-11, FIN-12).
        .route("/finances/prelevements", get(prelevements))
        .route("/finances/prelevements/preparer", post(preparer))
        .route("/finances/prelevements/export", post(exporter_prelevements))
        .route("/finances/prelevements/execution", post(execution))
        .route("/finances/prelevements/{id}/rejet", post(rejet))
        .route("/finances/rejets", get(rejets))
        // Règlements, comptes (FIN-08, FIN-17).
        .route("/finances/reglements", post(reglement))
        .route("/finances/foyers/{id}/compte", get(compte_foyer))
        .route("/finances/foyers/{id}/mode-paiement", put(mode_paiement))
        .route("/finances/famille", get(famille))
        // Relances (FIN-13) et export comptable (FIN-16).
        .route("/finances/relances", get(relances))
        .route("/finances/relances/proposer", post(proposer_relances))
        .route("/finances/relances/{id}/confirmer", post(confirmer_relance))
        .route("/finances/relances/{id}/annuler", post(annuler_relance))
        .route(
            "/finances/exports-comptables",
            get(exports).post(exporter_comptabilite),
        )
        // Paiement en ligne (FIN-09) : désactivé.
        .route("/finances/paiement-en-ligne", get(etat_paiement))
        .route(
            "/finances/paiement-en-ligne/intention",
            post(intention_paiement),
        )
        // Scolarité (D3) : déclencheurs de facturation.
        .route("/scolarite/eleves/{id}/regime", post(regime))
        .route("/scolarite/eleves/{id}/sortie", post(sortie))
}

#[derive(Deserialize)]
struct FiltreAnnee {
    annee: Option<String>,
}

#[derive(Deserialize)]
struct FiltreFactures {
    statut: Option<String>,
    annee: Option<String>,
    foyer: Option<Uuid>,
}

#[derive(Deserialize)]
struct FiltrePrelevements {
    statut: Option<String>,
    date: Option<NaiveDate>,
}

#[derive(Deserialize)]
struct FiltreStatut {
    statut: Option<String>,
}

// -- Paramétrage -------------------------------------------------------------

async fn parametres(State(st): State<AppState>, s: EntSession) -> Res<Json<ParametresFinancesDto>> {
    Ok(Json(
        st.ent
            .parametres_finances_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn modifier_parametres(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<ParametresFinancesBody>,
) -> Res<Json<ParametresFinancesDto>> {
    Ok(Json(
        st.ent
            .modifier_parametres_finances_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn articles(State(st): State<AppState>, s: EntSession) -> Res<Json<Vec<ArticleDto>>> {
    Ok(Json(
        st.ent
            .articles_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn creer_article(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<ArticleBody>,
) -> Res<(StatusCode, Json<ArticleDto>)> {
    let a = st
        .ent
        .enregistrer_article_for_actor(s.etablissement_id, s.user_id, &s.role, None, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(a)))
}

async fn modifier_article(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ArticleBody>,
) -> Res<Json<ArticleDto>> {
    Ok(Json(
        st.ent
            .enregistrer_article_for_actor(s.etablissement_id, s.user_id, &s.role, Some(id), &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn tarifs(
    State(st): State<AppState>,
    s: EntSession,
    Query(f): Query<FiltreAnnee>,
) -> Res<Json<Vec<TarifDto>>> {
    Ok(Json(
        st.ent
            .tarifs_for_actor(s.etablissement_id, s.user_id, &s.role, f.annee)
            .await
            .map_err(map_db)?,
    ))
}

async fn creer_tarif(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<TarifBody>,
) -> Res<(StatusCode, Json<TarifDto>)> {
    let t = st
        .ent
        .creer_tarif_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(t)))
}

async fn supprimer_tarif(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    st.ent
        .supprimer_tarif_for_actor(s.etablissement_id, s.user_id, &s.role, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reductions(
    State(st): State<AppState>,
    s: EntSession,
    Query(f): Query<FiltreAnnee>,
) -> Res<Json<Vec<ReductionDto>>> {
    Ok(Json(
        st.ent
            .reductions_for_actor(s.etablissement_id, s.user_id, &s.role, f.annee)
            .await
            .map_err(map_db)?,
    ))
}

async fn creer_reduction(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<ReductionBody>,
) -> Res<(StatusCode, Json<ReductionDto>)> {
    let r = st
        .ent
        .creer_reduction_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn accorder_reduction(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<EleveReductionBody>,
) -> Res<StatusCode> {
    st.ent
        .accorder_reduction_for_actor(s.etablissement_id, s.user_id, &s.role, id, b.eleve_id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn baremes(
    State(st): State<AppState>,
    s: EntSession,
    Query(f): Query<FiltreAnnee>,
) -> Res<Json<Vec<BaremeBourseDto>>> {
    Ok(Json(
        st.ent
            .baremes_bourses_for_actor(s.etablissement_id, s.user_id, &s.role, f.annee)
            .await
            .map_err(map_db)?,
    ))
}

async fn poser_bareme(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<BaremeBourseBody>,
) -> Res<Json<BaremeBourseDto>> {
    Ok(Json(
        st.ent
            .poser_bareme_bourse_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn echeancier_type(
    State(st): State<AppState>,
    s: EntSession,
    Query(f): Query<FiltreAnnee>,
) -> Res<Json<EcheancierTypeDto>> {
    let annee = f
        .annee
        .ok_or((StatusCode::BAD_REQUEST, "paramètre annee requis".into()))?;
    Ok(Json(
        st.ent
            .echeancier_type_for_actor(s.etablissement_id, s.user_id, &s.role, &annee)
            .await
            .map_err(map_db)?,
    ))
}

async fn poser_echeancier_type(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<EcheancierTypeDto>,
) -> Res<Json<EcheancierTypeDto>> {
    Ok(Json(
        st.ent
            .poser_echeancier_type_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

// -- Génération, factures, avoirs ---------------------------------------------

async fn generer(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<GenerationBody>,
) -> Res<Json<RapportGenerationDto>> {
    Ok(Json(
        st.ent
            .generer_annee_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn consommation(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<ConsommationBody>,
) -> Res<Json<ResultatSourceDto>> {
    Ok(Json(
        st.ent
            .facturer_consommation_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn factures(
    State(st): State<AppState>,
    s: EntSession,
    Query(f): Query<FiltreFactures>,
) -> Res<Json<Vec<FactureResumeDto>>> {
    Ok(Json(
        st.ent
            .factures_for_actor(
                s.etablissement_id,
                s.user_id,
                &s.role,
                f.statut,
                f.annee,
                f.foyer,
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn facture(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<FactureDetailDto>> {
    Ok(Json(
        st.ent
            .facture_detail_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn emettre(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<EmissionBody>,
) -> Res<Json<RapportEmissionDto>> {
    Ok(Json(
        st.ent
            .emettre_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn avoir(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AvoirBody>,
) -> Res<(StatusCode, Json<FactureDetailDto>)> {
    let d = st
        .ent
        .creer_avoir_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(d)))
}

// -- SEPA ---------------------------------------------------------------------

async fn prelevements(
    State(st): State<AppState>,
    s: EntSession,
    Query(f): Query<FiltrePrelevements>,
) -> Res<Json<Vec<PrelevementDto>>> {
    Ok(Json(
        st.ent
            .prelevements_for_actor(s.etablissement_id, s.user_id, &s.role, f.statut, f.date)
            .await
            .map_err(map_db)?,
    ))
}

async fn preparer(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<PreparationBody>,
) -> Res<Json<RapportPreparationDto>> {
    Ok(Json(
        st.ent
            .preparer_prelevements_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn exporter_prelevements(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<ExportPrelevementsBody>,
) -> Result<Response, (StatusCode, String)> {
    let (contenu, nature) = st
        .ent
        .exporter_prelevements_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
        .await
        .map_err(map_db)?;
    tracer_acces(&st, &s, "export", "sepa", "Export des prélèvements SEPA").await;
    if nature == "pain008" {
        attachment_response(
            contenu,
            &format!("prelevements-{}.xml", b.date_execution),
            "application/xml; charset=utf-8",
        )
    } else {
        attachment_response(
            contenu,
            &format!("prelevements-{}.csv", b.date_execution),
            "text/csv; charset=utf-8",
        )
    }
}

async fn execution(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<ExecutionBody>,
) -> Res<Json<RapportExecutionDto>> {
    Ok(Json(
        st.ent
            .constater_execution_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn rejet(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RejetBody>,
) -> Res<Json<RejetDto>> {
    Ok(Json(
        st.ent
            .traiter_rejet_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn rejets(State(st): State<AppState>, s: EntSession) -> Res<Json<Vec<RejetDto>>> {
    Ok(Json(
        st.ent
            .rejets_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map_db)?,
    ))
}

// -- Règlements, comptes ------------------------------------------------------

async fn reglement(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<ReglementBody>,
) -> Res<(StatusCode, Json<ReglementDto>)> {
    let r = st
        .ent
        .enregistrer_reglement_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn compte_foyer(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<CompteFoyerDto>> {
    Ok(Json(
        st.ent
            .compte_foyer_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn mode_paiement(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ModePaiementBody>,
) -> Res<Json<CompteFoyerDto>> {
    Ok(Json(
        st.ent
            .modifier_mode_paiement_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

/// Espace famille : seulement les foyers dont la personne est payeuse.
async fn famille(State(st): State<AppState>, s: EntSession) -> Res<Json<EspaceFamilleFinancesDto>> {
    Ok(Json(
        st.ent
            .espace_finances_famille(s.etablissement_id, s.user_id)
            .await
            .map_err(map_db)?,
    ))
}

// -- Relances, export comptable ---------------------------------------------

async fn relances(
    State(st): State<AppState>,
    s: EntSession,
    Query(f): Query<FiltreStatut>,
) -> Res<Json<Vec<RelanceDto>>> {
    Ok(Json(
        st.ent
            .relances_for_actor(s.etablissement_id, s.user_id, &s.role, f.statut)
            .await
            .map_err(map_db)?,
    ))
}

async fn proposer_relances(
    State(st): State<AppState>,
    s: EntSession,
) -> Res<Json<RapportRelancesDto>> {
    Ok(Json(
        st.ent
            .proposer_relances_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn confirmer_relance(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<RelanceDto>> {
    Ok(Json(
        st.ent
            .decider_relance_for_actor(s.etablissement_id, s.user_id, &s.role, id, true)
            .await
            .map_err(map_db)?,
    ))
}

async fn annuler_relance(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<RelanceDto>> {
    Ok(Json(
        st.ent
            .decider_relance_for_actor(s.etablissement_id, s.user_id, &s.role, id, false)
            .await
            .map_err(map_db)?,
    ))
}

async fn exports(State(st): State<AppState>, s: EntSession) -> Res<Json<Vec<ExportComptableDto>>> {
    Ok(Json(
        st.ent
            .exports_comptables_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn exporter_comptabilite(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<ExportComptableBody>,
) -> Res<(StatusCode, Json<ExportComptableDto>)> {
    let e = st
        .ent
        .exporter_comptabilite_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
        .await
        .map_err(map_db)?;
    tracer_acces(&st, &s, "export", "comptabilite", "Export comptable").await;
    Ok((StatusCode::CREATED, Json(e)))
}

// -- Paiement en ligne (désactivé) ---------------------------------------------

async fn etat_paiement(_s: EntSession) -> Json<EtatPaiementEnLigneDto> {
    Json(ent_db::paiement_en_ligne::etat_dto(
        &ent_db::paiement_en_ligne::PaiementDesactive,
    ))
}

async fn intention_paiement(
    _s: EntSession,
    Json(_b): Json<IntentionPaiementBody>,
) -> Res<Json<EtatPaiementEnLigneDto>> {
    Err((
        StatusCode::CONFLICT,
        ent_db::paiement_en_ligne::ErreurPaiement::Desactive.to_string(),
    ))
}

// -- Scolarité -------------------------------------------------------------------

async fn regime(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RegimeBody>,
) -> Res<Json<EvenementScolariteDto>> {
    Ok(Json(
        st.ent
            .changer_regime_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn sortie(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SortieBody>,
) -> Res<Json<EvenementScolariteDto>> {
    Ok(Json(
        st.ent
            .enregistrer_sortie_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}
