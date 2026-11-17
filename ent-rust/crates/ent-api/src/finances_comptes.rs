//! Routes du socle 24, lots F1 et F2 : journal financier, vue guichet, comptes débiteurs, payeurs
//! désignés, conséquences d'un changement de payeur, élève majeur payeur, prises en charge d'organismes,
//! tâches. Types JSON : `ent_types::finances_comptes`. L'autorité est `ent-db` (`facturation_*.rs`) :
//! droits fins (`facturation.*`, §19.1), établissement de la session, journal et événements.
//!
//! Toute cible absente ou d'un autre établissement répond **404** (jamais un succès) ; une personne qui
//! n'est pas le payeur désigné reçoit le même 404 que pour une proposition inexistante.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::finances_comptes::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::{map_db, tracer_acces};
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // F1 : journal et guichet.
        .route("/finances/journal", get(journal))
        .route("/finances/guichet/recherche", get(guichet_recherche))
        .route("/finances/guichet/comptes/{id}", get(guichet_compte))
        // F2 : comptes.
        .route("/finances/comptes", get(comptes).post(creer_compte_organisme))
        .route("/finances/comptes/reprise", post(reprendre_comptes))
        .route("/finances/comptes/{id}", get(compte))
        .route("/finances/mes-comptes", get(mes_comptes))
        // F2 : payeurs.
        .route("/finances/eleves/{id}/payeurs", get(payeurs))
        .route("/finances/eleves/{id}/designations", post(designer_payeurs))
        .route("/finances/eleves/{id}/payeur-majeur", post(payeur_majeur))
        .route("/finances/designations/{groupe}/accepter", post(accepter_designation))
        .route("/finances/designations/{groupe}/refuser", post(refuser_designation))
        .route("/finances/designations/{groupe}/piece", post(piece_designation))
        .route("/finances/famille/demande-repartition", post(demande_repartition))
        // F2 : conséquences.
        .route("/finances/consequences", get(consequences))
        .route("/finances/consequences/{groupe}/decision", post(decider_consequences))
        // F2 : prises en charge et tâches.
        .route(
            "/finances/prises-en-charge",
            get(prises_en_charge).post(creer_prise_en_charge),
        )
        .route("/finances/prises-en-charge/alertes", post(alertes_prises_en_charge))
        .route("/finances/taches", get(taches))
        .route("/finances/taches/{id}/abandonner", post(abandonner_tache))
}

#[derive(Debug, Deserialize)]
struct LimiteQuery {
    limite: Option<i64>,
}

async fn journal(
    State(st): State<AppState>,
    s: EntSession,
    Query(q): Query<LimiteQuery>,
) -> Res<Json<Vec<JournalLigneDto>>> {
    let r = st
        .ent
        .journal_financier_for_actor(s.etablissement_id, s.user_id, &s.role, q.limite.unwrap_or(100))
        .await
        .map_err(map_db)?;
    tracer_acces(&st, &s, "facturation", "lecture", "Journal financier").await?;
    Ok(Json(r))
}

#[derive(Debug, Deserialize)]
struct RechercheQuery {
    nom: String,
    auth: AuthentificationGuichet,
}

async fn guichet_recherche(
    State(st): State<AppState>,
    s: EntSession,
    Query(q): Query<RechercheQuery>,
) -> Res<Json<Vec<GuichetPayeurDto>>> {
    Ok(Json(
        st.ent
            .guichet_rechercher_for_actor(s.etablissement_id, s.user_id, &s.role, &q.nom, q.auth)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct AuthQuery {
    auth: AuthentificationGuichet,
}

async fn guichet_compte(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<AuthQuery>,
) -> Res<Json<GuichetCompteDto>> {
    let r = st
        .ent
        .guichet_compte_for_actor(s.etablissement_id, s.user_id, &s.role, id, q.auth)
        .await
        .map_err(map_db)?;
    tracer_acces(&st, &s, "facturation", "lecture", "Vue guichet d'un compte").await?;
    Ok(Json(r))
}

async fn comptes(
    State(st): State<AppState>,
    s: EntSession,
    Query(f): Query<ComptesFiltre>,
) -> Res<Json<Vec<CompteDto>>> {
    Ok(Json(
        st.ent
            .comptes_for_actor(s.etablissement_id, s.user_id, &s.role, &f)
            .await
            .map_err(map_db)?,
    ))
}

async fn compte(State(st): State<AppState>, s: EntSession, Path(id): Path<Uuid>) -> Res<Json<CompteDto>> {
    let r = st
        .ent
        .compte_for_actor(s.etablissement_id, s.user_id, &s.role, id)
        .await
        .map_err(map_db)?;
    tracer_acces(&st, &s, "facturation", "lecture", "Compte débiteur").await?;
    Ok(Json(r))
}

async fn creer_compte_organisme(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<CreerCompteOrganismeBody>,
) -> Res<(StatusCode, Json<CompteDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            st.ent
                .creer_compte_organisme_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn reprendre_comptes(State(st): State<AppState>, s: EntSession) -> Res<Json<RapportRepriseDto>> {
    Ok(Json(
        st.ent
            .reprendre_comptes_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn mes_comptes(State(st): State<AppState>, s: EntSession) -> Res<Json<Vec<CompteDto>>> {
    Ok(Json(
        st.ent
            .mes_comptes_for_actor(s.etablissement_id, s.user_id)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct PayeursQuery {
    categorie: Option<String>,
    date: Option<NaiveDate>,
}

async fn payeurs(
    State(st): State<AppState>,
    s: EntSession,
    Path(eleve): Path<Uuid>,
    Query(q): Query<PayeursQuery>,
) -> Res<Json<PayeursDto>> {
    Ok(Json(
        st.ent
            .payeurs_for_actor(
                s.etablissement_id,
                s.user_id,
                &s.role,
                eleve,
                q.categorie.as_deref().unwrap_or("toutes"),
                q.date.unwrap_or_else(domain_eleve::horloge::jour_paris),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn designer_payeurs(
    State(st): State<AppState>,
    s: EntSession,
    Path(eleve): Path<Uuid>,
    Json(b): Json<DesignerPayeursBody>,
) -> Res<(StatusCode, Json<ResultatDesignationDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            st.ent
                .designer_payeurs_for_actor(s.etablissement_id, s.user_id, &s.role, eleve, &b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn payeur_majeur(
    State(st): State<AppState>,
    s: EntSession,
    Path(eleve): Path<Uuid>,
    Json(b): Json<PayeurMajeurBody>,
) -> Res<(StatusCode, Json<ResultatDesignationDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            st.ent
                .transferer_payeur_majeur_for_actor(s.etablissement_id, s.user_id, &s.role, eleve, &b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn accepter_designation(
    State(st): State<AppState>,
    s: EntSession,
    Path(groupe): Path<Uuid>,
) -> Res<Json<ResultatDesignationDto>> {
    Ok(Json(
        st.ent
            .repondre_designation_for_actor(s.etablissement_id, s.user_id, &s.role, groupe, true)
            .await
            .map_err(map_db)?,
    ))
}

async fn refuser_designation(
    State(st): State<AppState>,
    s: EntSession,
    Path(groupe): Path<Uuid>,
) -> Res<Json<ResultatDesignationDto>> {
    Ok(Json(
        st.ent
            .repondre_designation_for_actor(s.etablissement_id, s.user_id, &s.role, groupe, false)
            .await
            .map_err(map_db)?,
    ))
}

async fn piece_designation(
    State(st): State<AppState>,
    s: EntSession,
    Path(groupe): Path<Uuid>,
    Json(b): Json<JoindrePieceBody>,
) -> Res<Json<ResultatDesignationDto>> {
    Ok(Json(
        st.ent
            .joindre_piece_repartition_for_actor(s.etablissement_id, s.user_id, &s.role, groupe, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn demande_repartition(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<DemandeRepartitionBody>,
) -> Res<(StatusCode, Json<TacheDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            st.ent
                .demander_repartition_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

#[derive(Debug, Deserialize)]
struct ConsequencesQuery {
    eleve_id: Option<Uuid>,
}

async fn consequences(
    State(st): State<AppState>,
    s: EntSession,
    Query(q): Query<ConsequencesQuery>,
) -> Res<Json<Vec<ConsequenceDto>>> {
    Ok(Json(
        st.ent
            .consequences_for_actor(s.etablissement_id, s.user_id, &s.role, q.eleve_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn decider_consequences(
    State(st): State<AppState>,
    s: EntSession,
    Path(groupe): Path<Uuid>,
    Json(b): Json<DecisionConsequencesBody>,
) -> Res<Json<ResultatConsequencesDto>> {
    Ok(Json(
        st.ent
            .decider_consequences_for_actor(s.etablissement_id, s.user_id, &s.role, groupe, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn prises_en_charge(State(st): State<AppState>, s: EntSession) -> Res<Json<Vec<PriseEnChargeDto>>> {
    Ok(Json(
        st.ent
            .prises_en_charge_for_actor(s.etablissement_id, s.user_id, &s.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn creer_prise_en_charge(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<PriseEnChargeBody>,
) -> Res<(StatusCode, Json<PriseEnChargeDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            st.ent
                .creer_prise_en_charge_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn alertes_prises_en_charge(
    State(st): State<AppState>,
    s: EntSession,
    Json(b): Json<AlertesPrisesEnChargeBody>,
) -> Res<Json<RapportAlertesDto>> {
    Ok(Json(
        st.ent
            .alerter_prises_en_charge_for_actor(
                s.etablissement_id,
                s.user_id,
                &s.role,
                b.jour.unwrap_or_else(domain_eleve::horloge::jour_paris),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn taches(
    State(st): State<AppState>,
    s: EntSession,
    Query(f): Query<TachesFiltre>,
) -> Res<Json<Vec<TacheDto>>> {
    Ok(Json(
        st.ent
            .taches_for_actor(s.etablissement_id, s.user_id, &s.role, &f)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AbandonBody {
    motif: String,
}

async fn abandonner_tache(
    State(st): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AbandonBody>,
) -> Res<StatusCode> {
    st.ent
        .abandonner_tache_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b.motif)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}
