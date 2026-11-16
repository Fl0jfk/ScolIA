//! Routes de l'information des familles (socle 12, lot AS5) : réglages, avis par demi-journée, relance graduée,
//! contacts tracés, courriers postaux, vue de la famille.
//!
//! L'autorité est `ent-db` (`assiduite.suivre`, `assiduite.signaler`, `assiduite.regle_gerer` ; lien famille
//! pour la vue de la famille). **Aucun contrôle de rôle codé en dur ici** : une référence inexistante, d'un
//! autre établissement ou sans droit donne la même réponse (R-AS35). Aucune route n'envoie quoi que ce soit :
//! l'envoi passe par le moteur de notifications du socle 8 (consommateur `assiduite.familles`, tâche de relance).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_db::{
    AvisDemiJourneeAsd, CommunicationFamille, ContactAsd, CourrierAsd, EpisodeSansReponse,
    ParamFamille, PreparerCourrier, RelanceAsd, TracerContact,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Réglages (lecture : la règle d'assiduité datée, AS1, les porte)
        .route("/assiduite/familles/parametres", get(parametres))
        // Avis par demi-journée et relance graduée
        .route("/assiduite/eleves/{eleve_id}/avis", get(avis_eleve))
        .route("/assiduite/relances/sans-reponse", get(sans_reponse))
        .route("/assiduite/episodes/{id}/relances", get(relances_episode))
        // Contacts tracés
        .route("/assiduite/contacts", post(tracer_contact))
        .route("/assiduite/contacts/numeros-faux", get(numeros_faux))
        .route(
            "/assiduite/contacts/{id}/numero-faux/cloturer",
            post(cloturer_numero_faux),
        )
        .route("/assiduite/eleves/{eleve_id}/contacts", get(contacts))
        // Courriers postaux
        .route("/assiduite/courriers", post(preparer_courrier))
        .route("/assiduite/eleves/{eleve_id}/courriers", get(courriers))
        .route("/assiduite/courriers/{id}/valider", post(valider_courrier))
        .route("/assiduite/courriers/{id}/poster", post(poster_courrier))
        .route(
            "/assiduite/courriers/{id}/abandonner",
            post(abandonner_courrier),
        )
        // Famille
        .route(
            "/assiduite/eleves/{eleve_id}/communications",
            get(communications),
        )
}

#[derive(Deserialize)]
struct FiltreAvis {
    du: Option<NaiveDate>,
}

#[derive(Deserialize)]
struct FiltreContacts {
    absence_id: Option<Uuid>,
}

#[derive(Deserialize)]
struct Abandon {
    code: String,
}

async fn parametres(State(state): State<AppState>, session: EntSession) -> Res<Json<ParamFamille>> {
    state
        .ent
        .param_famille_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn avis_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
    Query(f): Query<FiltreAvis>,
) -> Res<Json<Vec<AvisDemiJourneeAsd>>> {
    state
        .ent
        .avis_eleve_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            eleve_id,
            f.du,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn sans_reponse(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<EpisodeSansReponse>>> {
    state
        .ent
        .sans_reponse_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn relances_episode(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<RelanceAsd>>> {
    state
        .ent
        .relances_episode_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn tracer_contact(
    State(state): State<AppState>,
    session: EntSession,
    Json(c): Json<TracerContact>,
) -> Res<(StatusCode, Json<ContactAsd>)> {
    state
        .ent
        .tracer_contact_for_actor(session.etablissement_id, session.user_id, &session.role, c)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(map_db)
}

async fn numeros_faux(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<ContactAsd>>> {
    state
        .ent
        .numeros_faux_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn cloturer_numero_faux(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .cloturer_numero_faux_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(map_db)
}

async fn contacts(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
    Query(f): Query<FiltreContacts>,
) -> Res<Json<Vec<ContactAsd>>> {
    state
        .ent
        .contacts_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            eleve_id,
            f.absence_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn preparer_courrier(
    State(state): State<AppState>,
    session: EntSession,
    Json(p): Json<PreparerCourrier>,
) -> Res<(StatusCode, Json<CourrierAsd>)> {
    state
        .ent
        .preparer_courrier_for_actor(session.etablissement_id, session.user_id, &session.role, p)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(map_db)
}

async fn courriers(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
) -> Res<Json<Vec<CourrierAsd>>> {
    state
        .ent
        .courriers_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            eleve_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn valider_courrier(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<CourrierAsd>> {
    state
        .ent
        .valider_courrier_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn poster_courrier(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<CourrierAsd>> {
    state
        .ent
        .poster_courrier_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn abandonner_courrier(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(a): Json<Abandon>,
) -> Res<Json<CourrierAsd>> {
    state
        .ent
        .abandonner_courrier_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &a.code,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

async fn communications(
    State(state): State<AppState>,
    session: EntSession,
    Path(eleve_id): Path<Uuid>,
) -> Res<Json<Vec<CommunicationFamille>>> {
    state
        .ent
        .communications_famille_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            eleve_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}
