//! Routes du socle 24, lot F3 : catalogue d'articles qualifiés (nature légale, TVA), versions de grille
//! (brouillon, proposée, votée, en vigueur, correction), tranches de quotient, simulateur de scénarios,
//! familles types, état des contributions. Types JSON : `ent_types::finances_grilles`. L'autorité est
//! `ent-db` (`facturation_grilles.rs`) : droits fins (`facturation.*`, §19.1), établissement de la session,
//! journal et événements.
//!
//! Toute cible absente ou d'un autre établissement répond **404** (jamais un succès). Aucune de ces routes
//! ne renvoie ni nom ni coordonnée de personne.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use ent_types::finances_grilles::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Catalogue.
        .route("/finances/catalogue/modeles", get(modeles))
        .route(
            "/finances/catalogue/articles",
            get(articles).post(article_depuis_modele),
        )
        .route(
            "/finances/catalogue/articles/{id}/qualifier",
            post(qualifier_article),
        )
        .route(
            "/finances/catalogue/articles/{id}/usage",
            get(usage_article),
        )
        .route(
            "/finances/grilles/reglages",
            get(reglages).post(definir_reglages),
        )
        // Versions de grille.
        .route("/finances/grilles", get(grilles).post(creer_grille))
        .route("/finances/grilles/{id}", get(grille))
        .route("/finances/grilles/{id}/tarifs", post(ajouter_tarif))
        .route(
            "/finances/grilles/{id}/tarifs/{tarif}",
            put(modifier_tarif).delete(retirer_tarif),
        )
        .route("/finances/grilles/{id}/tranches", put(definir_tranches))
        .route("/finances/grilles/{id}/hausse", post(hausse))
        .route("/finances/grilles/{id}/controles", get(controles))
        .route("/finances/grilles/{id}/proposer", post(proposer))
        .route(
            "/finances/grilles/{id}/retour-brouillon",
            post(retour_brouillon),
        )
        .route("/finances/grilles/{id}/abandonner", post(abandonner))
        .route("/finances/grilles/{id}/voter", post(voter))
        .route(
            "/finances/grilles/{id}/mise-en-vigueur",
            post(mise_en_vigueur),
        )
        .route("/finances/grilles/{id}/correction", post(corriger))
        .route("/finances/grilles/{id}/publier", post(publier))
        .route("/finances/grilles/{id}/familles-types", get(effet_familles))
        // Simulateur.
        .route(
            "/finances/familles-types",
            get(familles_types).post(creer_famille_type),
        )
        .route(
            "/finances/familles-types/{id}/desactiver",
            post(desactiver_famille_type),
        )
        .route(
            "/finances/familles-types/{id}",
            delete(supprimer_famille_type),
        )
        .route(
            "/finances/simulations",
            get(simulations).post(creer_simulation),
        )
        .route("/finances/simulations/comparaison", get(comparer))
        .route("/finances/simulations/{id}", delete(supprimer_simulation))
        .route("/finances/simulations/{id}/calculer", post(calculer))
        .route("/finances/simulations/{id}/export", get(exporter))
        // État des contributions.
        .route("/finances/contributions", get(contributions))
}

#[derive(Debug, Deserialize)]
struct AnneeQuery {
    annee: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ComparaisonQuery {
    a: Uuid,
    b: Uuid,
}

async fn modeles(State(st): State<AppState>, s: EntSession) -> Res<Json<Vec<ModeleArticleDto>>> {
    Ok(Json(
        st.ent
            .modeles_articles_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn articles(State(st): State<AppState>, s: EntSession) -> Res<Json<Vec<ArticleQualifieDto>>> {
    Ok(Json(
        st.ent
            .articles_qualifies_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn article_depuis_modele(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<ArticleDepuisModeleBody>,
) -> Res<Json<ArticleQualifieDto>> {
    Ok(Json(
        st.ent
            .article_depuis_modele_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn qualifier_article(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<QualifierArticleBody>,
) -> Res<Json<ArticleQualifieDto>> {
    Ok(Json(
        st.ent
            .qualifier_article_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn usage_article(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ArticleUsageDto>> {
    Ok(Json(
        st.ent
            .usage_article_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn reglages(State(st): State<AppState>, s: EntSession) -> Res<Json<ReglagesGrillesDto>> {
    Ok(Json(
        st.ent
            .reglages_grilles_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn definir_reglages(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<ReglagesGrillesBody>,
) -> Res<Json<ReglagesGrillesDto>> {
    Ok(Json(
        st.ent
            .definir_reglages_grilles_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn grilles(
    State(st): State<AppState>,
    s: EntSession,
    Query(q): Query<AnneeQuery>,
) -> Res<Json<Vec<GrilleVersionDto>>> {
    Ok(Json(
        st.ent
            .grilles_for_actor(s.etablissement_id, s.user_id, &s.role, q.annee)
            .await
            .map_err(map_db)?,
    ))
}

async fn creer_grille(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<CreerGrilleBody>,
) -> Res<Json<GrilleVersionDto>> {
    Ok(Json(
        st.ent
            .creer_grille_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn grille(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<GrilleDetailDto>> {
    Ok(Json(
        st.ent
            .grille_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn ajouter_tarif(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TarifGrilleBody>,
) -> Res<Json<TarifGrilleDto>> {
    Ok(Json(
        st.ent
            .ajouter_tarif_grille_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn modifier_tarif(
    State(st): State<AppState>,
    s: EntSession,
    Path((id, tarif)): Path<(Uuid, Uuid)>,
    Json(b): Json<TarifGrilleBody>,
) -> Res<Json<TarifGrilleDto>> {
    Ok(Json(
        st.ent
            .modifier_tarif_grille_for_actor(s.etablissement_id, s.user_id, &s.role, id, tarif, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn definir_tranches(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TranchesBody>,
) -> Res<Json<Vec<TrancheQfDto>>> {
    Ok(Json(
        st.ent
            .definir_tranches_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn hausse(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<HausseBody>,
) -> Res<Json<ApercuHausseDto>> {
    Ok(Json(
        st.ent
            .appliquer_hausse_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn controles(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ControlesGrilleDto>> {
    Ok(Json(
        st.ent
            .controles_grille_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn proposer(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<GrilleVersionDto>> {
    Ok(Json(
        st.ent
            .proposer_grille_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn retour_brouillon(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RetourBrouillonBody>,
) -> Res<Json<GrilleVersionDto>> {
    Ok(Json(
        st.ent
            .retour_brouillon_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn abandonner(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RetourBrouillonBody>,
) -> Res<Json<GrilleVersionDto>> {
    Ok(Json(
        st.ent
            .abandonner_grille_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn voter(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<VoteGrilleBody>,
) -> Res<Json<GrilleVersionDto>> {
    Ok(Json(
        st.ent
            .voter_grille_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn mise_en_vigueur(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<MiseEnVigueurBody>,
) -> Res<Json<GrilleVersionDto>> {
    Ok(Json(
        st.ent
            .mettre_en_vigueur_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn corriger(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CorrectionGrilleBody>,
) -> Res<Json<GrilleVersionDto>> {
    Ok(Json(
        st.ent
            .corriger_grille_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn publier(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<PublicationGrilleDto>> {
    Ok(Json(
        st.ent
            .publier_grille_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn effet_familles(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<EffetFamilleTypeDto>>> {
    Ok(Json(
        st.ent
            .effet_familles_types_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn familles_types(
    State(st): State<AppState>,
    s: EntSession,
) -> Res<Json<Vec<FamilleTypeDto>>> {
    Ok(Json(
        st.ent
            .familles_types_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn creer_famille_type(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<FamilleTypeBody>,
) -> Res<Json<FamilleTypeDto>> {
    Ok(Json(
        st.ent
            .creer_famille_type_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn simulations(
    State(st): State<AppState>,
    s: EntSession,
) -> Res<Json<Vec<SimulationResumeDto>>> {
    Ok(Json(
        st.ent
            .simulations_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn creer_simulation(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<SimulationBody>,
) -> Res<Json<ResultatSimulationDto>> {
    Ok(Json(
        st.ent
            .creer_simulation_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn comparer(
    State(st): State<AppState>,
    s: EntSession,
    Query(q): Query<ComparaisonQuery>,
) -> Res<Json<ComparaisonSimulationsDto>> {
    Ok(Json(
        st.ent
            .comparer_simulations_for_actor(s.etablissement_id, s.user_id, &s.role, q.a, q.b)
            .await
            .map_err(map_db)?,
    ))
}

async fn calculer(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ResultatSimulationDto>> {
    Ok(Json(
        st.ent
            .calculer_simulation_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn exporter(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ExportConseilDto>> {
    Ok(Json(
        st.ent
            .exporter_conseil_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn retirer_tarif(
    State(st): State<AppState>,
    s: EntSession,
    Path((id, tarif)): Path<(Uuid, Uuid)>,
) -> Res<StatusCode> {
    st.ent
        .retirer_tarif_grille_for_actor(s.etablissement_id, s.user_id, &s.role, id, tarif)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn desactiver_famille_type(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    st.ent
        .desactiver_famille_type_for_actor(s.etablissement_id, s.user_id, &s.role, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn supprimer_famille_type(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    st.ent
        .supprimer_famille_type_for_actor(s.etablissement_id, s.user_id, &s.role, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn supprimer_simulation(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    st.ent
        .supprimer_simulation_for_actor(s.etablissement_id, s.user_id, &s.role, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn contributions(
    State(st): State<AppState>,
    s: EntSession,
    Query(q): Query<AnneeQuery>,
) -> Res<Json<EtatContributionsDto>> {
    let annee = q.annee.ok_or((
        StatusCode::UNPROCESSABLE_ENTITY,
        "année scolaire attendue (AAAA-AAAA)".to_string(),
    ))?;
    Ok(Json(
        st.ent
            .etat_contributions_for_actor(s.etablissement_id, s.user_id, &s.role, &annee)
            .await
            .map_err(map_db)?,
    ))
}
