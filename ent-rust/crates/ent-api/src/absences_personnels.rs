//! Routes des absences des personnels (socle 15, lot A1). Préfixe `/personnels/absences`.
//!
//! L'autorité est `ent-db` (`absences_personnels.rs`) : droits (`personnels.absences.*`, D-017),
//! cloisonnement par établissement (404 uniforme hors du cercle RH), journal, événements. Les routes ne
//! font que décoder et mapper les erreurs. Types JSON : `ent_types::absences_personnels`.
//!
//! Deux niveaux de lecture, **deux routes** : `/organisation` (sans nature, R-AP3) et `/{id}` (cercle RH).
//! Le port « absences validées non remplacées » (`/non-remplacees`, socles 10 et 19) ne porte ni motif,
//! ni nature, ni nom d'agent.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::absences_personnels::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/personnels/absences", post(declarer))
        .route("/personnels/absences/signalements", post(signaler))
        .route("/personnels/absences/natures", get(natures))
        .route("/personnels/absences/moi", get(mes_absences))
        .route("/personnels/absences/organisation", get(organisation))
        .route("/personnels/absences/a-valider", get(a_valider))
        .route("/personnels/absences/non-remplacees", get(non_remplacees))
        .route(
            "/personnels/absences/circuit-regles",
            get(regles_circuit).post(creer_regle_circuit),
        )
        .route(
            "/personnels/absences/circuit-regles/{id}",
            delete(retirer_regle_circuit),
        )
        .route("/personnels/absences/{id}", get(absence))
        .route("/personnels/absences/{id}/historique", get(historique))
        .route("/personnels/absences/{id}/confirmer", post(confirmer))
        .route(
            "/personnels/absences/{id}/rappel-confirme",
            post(confirmer_par_rappel),
        )
        .route("/personnels/absences/{id}/valider", post(valider))
        .route("/personnels/absences/{id}/refuser", post(refuser))
        .route("/personnels/absences/{id}/precision", post(precision))
        .route("/personnels/absences/{id}/traiter", post(traiter))
        .route("/personnels/absences/{id}/clore", post(clore))
        .route("/personnels/absences/{id}/rouvrir", post(rouvrir))
        .route("/personnels/absences/{id}/annuler", post(annuler))
        .route("/personnels/absences/{id}/requalifier", post(requalifier))
        .route("/personnels/absences/{id}/prolonger", post(prolonger))
        .route("/personnels/absences/{id}/ecourter", post(ecourter))
        .route("/personnels/absences/{id}/arret-recu", post(arret_recu))
        .route(
            "/personnels/absences/{id}/decision-externe",
            post(decision_externe),
        )
}

#[derive(Debug, Deserialize)]
struct Periode {
    du: Option<NaiveDate>,
    au: Option<NaiveDate>,
}

fn aujourdhui() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

async fn declarer(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<DeclarerAbsenceBody>,
) -> Res<(StatusCode, Json<DeclarationDto>)> {
    state
        .ent
        .declarer_absence_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn signaler(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<SignalerAbsenceBody>,
) -> Res<(StatusCode, Json<DeclarationDto>)> {
    state
        .ent
        .signaler_absence_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(map_db)
}

async fn natures(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<NatureDto>>> {
    state
        .ent
        .natures_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn mes_absences(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<AbsenceDto>>> {
    state
        .ent
        .mes_absences_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn organisation(
    State(state): State<AppState>,
    session: EntSession,
    Query(p): Query<Periode>,
) -> Res<Json<Vec<AbsenceOrganisationDto>>> {
    let du = p.du.unwrap_or_else(aujourdhui);
    let au = p.au.unwrap_or(du);
    state
        .ent
        .absences_organisation_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            du,
            au,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn a_valider(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<AbsenceDto>>> {
    state
        .ent
        .file_a_valider_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn non_remplacees(
    State(state): State<AppState>,
    session: EntSession,
    Query(p): Query<Periode>,
) -> Res<Json<Vec<SeanceSansEnseignantDto>>> {
    let du = p.du.unwrap_or_else(aujourdhui);
    let au = p.au.unwrap_or(du);
    state
        .ent
        .absences_validees_non_remplacees_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            du,
            au,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn regles_circuit(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<RegleCircuitDto>>> {
    state
        .ent
        .regles_circuit_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creer_regle_circuit(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerRegleCircuitBody>,
) -> Res<(StatusCode, Json<RegleCircuitDto>)> {
    state
        .ent
        .creer_regle_circuit_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

async fn retirer_regle_circuit(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .retirer_regle_circuit_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn absence(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<AbsenceDto>> {
    state
        .ent
        .absence_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn historique(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<JournalAbsenceDto>>> {
    state
        .ent
        .historique_absence_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn confirmer(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<ConfirmerBody>>,
) -> Res<Json<AbsenceDto>> {
    let b = body.map(|Json(b)| b).unwrap_or_default();
    state
        .ent
        .confirmer_absence_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
            false,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn confirmer_par_rappel(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<ConfirmerBody>>,
) -> Res<Json<AbsenceDto>> {
    let b = body.map(|Json(b)| b).unwrap_or_default();
    state
        .ent
        .confirmer_absence_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
            true,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

macro_rules! decision_route {
    ($nom:ident, $fonction:ident) => {
        async fn $nom(
            State(state): State<AppState>,
            session: EntSession,
            Path(id): Path<Uuid>,
            body: Option<Json<DecisionBody>>,
        ) -> Res<Json<AbsenceDto>> {
            let b = body.map(|Json(b)| b).unwrap_or_default();
            state
                .ent
                .$fonction(
                    session.etablissement_id,
                    session.user_id,
                    &session.role,
                    id,
                    &b,
                )
                .await
                .map(Json)
                .map_err(map_db)
        }
    };
}

decision_route!(valider, valider_absence_for_actor);
decision_route!(refuser, refuser_absence_for_actor);
decision_route!(precision, demander_precision_for_actor);
decision_route!(traiter, traiter_absence_for_actor);
decision_route!(clore, clore_absence_for_actor);
decision_route!(rouvrir, rouvrir_absence_for_actor);
decision_route!(annuler, annuler_absence_for_actor);

async fn requalifier(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RequalifierBody>,
) -> Res<Json<AbsenceDto>> {
    state
        .ent
        .requalifier_absence_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn prolonger(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ChangerFinBody>,
) -> Res<Json<AbsenceDto>> {
    state
        .ent
        .prolonger_absence_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn ecourter(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ChangerFinBody>,
) -> Res<Json<AbsenceDto>> {
    state
        .ent
        .ecourter_absence_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn arret_recu(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<ArretRecuBody>>,
) -> Res<Json<AbsenceDto>> {
    let b = body.map(|Json(b)| b).unwrap_or_default();
    state
        .ent
        .noter_arret_recu_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn decision_externe(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DecisionExterneBody>,
) -> Res<Json<AbsenceDto>> {
    state
        .ent
        .enregistrer_decision_externe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map(Json)
        .map_err(map_db)
}
