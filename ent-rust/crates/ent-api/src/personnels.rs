//! Routes des personnels (socle 6, lot P1) : agents, engagements, fonctions, disciplines,
//! qualifications, choix de l'agent, annuaire. Préfixe `/personnels`.
//!
//! L'autorité est `ent-db` (`personnels.rs`, `personnels_fiche.rs`) : droits (`personnels.*`),
//! cloisonnement par établissement (404 uniforme pour une référence d'un autre établissement),
//! journal, événements. Les routes ne font que décoder et mapper les erreurs.
//! Types JSON partagés avec le frontend : `ent_types::personnels`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use ent_db::personnels::FiltreAnnuaire;
use ent_db::ResultatCreationAgent;
use ent_types::personnels::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/personnels/annuaire", get(annuaire))
        .route(
            "/personnels/fonctions",
            get(fonctions_catalogue).post(creer_fonction),
        )
        .route("/personnels/agents", post(creer_agent))
        .route("/personnels/agents/{id}", get(fiche))
        .route("/personnels/agents/{id}/identite", put(modifier_identite))
        .route(
            "/personnels/agents/{id}/engagements",
            post(ajouter_engagement),
        )
        .route("/personnels/agents/{id}/fonctions", post(ajouter_fonction))
        .route(
            "/personnels/agents/{id}/disciplines",
            post(ajouter_discipline),
        )
        .route(
            "/personnels/agents/{id}/qualifications",
            post(enregistrer_qualification),
        )
        .route(
            "/personnels/agents/{id}/identifiants",
            post(ajouter_identifiant),
        )
        .route(
            "/personnels/agents/{id}/identite-restreinte",
            put(identite_restreinte),
        )
        .route("/personnels/agents/{id}/mineur", put(personnel_mineur))
        .route("/personnels/engagements/{id}", put(modifier_engagement))
        .route(
            "/personnels/engagements/{id}/prolonger",
            post(prolonger_engagement),
        )
        .route(
            "/personnels/engagements/{id}/terminer",
            post(terminer_engagement),
        )
        .route(
            "/personnels/engagements/{id}/corriger",
            post(corriger_engagement),
        )
        .route(
            "/personnels/engagements/{id}/suspendre",
            post(suspendre_engagement),
        )
        .route(
            "/personnels/engagements/{id}/reprendre",
            post(reprendre_engagement),
        )
        .route(
            "/personnels/fonctions-agent/{id}/terminer",
            post(terminer_fonction),
        )
        .route(
            "/personnels/disciplines-agent/{id}/terminer",
            post(terminer_discipline),
        )
        .route(
            "/personnels/qualifications/{id}/annuler",
            post(annuler_qualification),
        )
        .route(
            "/personnels/identifiants/{id}/clore",
            post(clore_identifiant),
        )
        .route("/personnels/fusion", post(fusionner_agents))
        .route(
            "/personnels/moi/contact-urgence",
            put(definir_contact_urgence),
        )
        .route(
            "/personnels/moi/contact-urgence/{ordre}",
            delete(retirer_contact_urgence),
        )
        .route("/personnels/moi/photo", put(definir_photo))
        .route("/personnels/moi/pause-notifications", put(definir_pause))
}

#[derive(Deserialize)]
struct AnnuaireQuery {
    q: Option<String>,
    fonction: Option<String>,
    /// États séparés par des virgules (`actif,suspendu`) ; défaut : `actif`.
    etat: Option<String>,
    limite: Option<i64>,
}

async fn annuaire(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<AnnuaireQuery>,
) -> Res<Json<Vec<AgentResumeDto>>> {
    let filtre = FiltreAnnuaire {
        q: q.q,
        fonction_code: q.fonction,
        etats: q
            .etat
            .map(|e| {
                e.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default(),
        limite: q.limite,
    };
    let v = state
        .ent
        .annuaire_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &filtre,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(v))
}

async fn fonctions_catalogue(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<FonctionCatalogueDto>>> {
    let v = state
        .ent
        .fonctions_catalogue_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(v))
}

async fn creer_fonction(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerFonctionBody>,
) -> Res<(StatusCode, Json<FonctionCatalogueDto>)> {
    let f = state
        .ent
        .creer_fonction_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(f)))
}

/// 201 avec l'agent créé, ou 409 `correspondances_a_confirmer` : aucune écriture tant que la direction
/// n'a pas choisi une personne existante (`personne_id`) ou confirmé la création (`confirmer_nouvelle`).
async fn creer_agent(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerAgentBody>,
) -> Response {
    match state
        .ent
        .creer_agent_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
    {
        Ok(ResultatCreationAgent::Cree(c)) => (StatusCode::CREATED, Json(c)).into_response(),
        Ok(ResultatCreationAgent::AConfirmer(correspondances)) => (
            StatusCode::CONFLICT,
            Json(CorrespondancesAConfirmerDto {
                code: "correspondances_a_confirmer".into(),
                correspondances,
            }),
        )
            .into_response(),
        Err(e) => map_db(e).into_response(),
    }
}

async fn fiche(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<FicheAgentDto>> {
    let f = state
        .ent
        .fiche_agent_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(f))
}

async fn modifier_identite(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ModifierIdentiteBody>,
) -> Res<Json<IdentiteAgentDto>> {
    let i = state
        .ent
        .modifier_identite_agent_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(i))
}

async fn ajouter_engagement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<EngagementBody>,
) -> Res<(StatusCode, Json<EngagementDto>)> {
    let e = state
        .ent
        .ajouter_engagement_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(e)))
}

async fn modifier_engagement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ModifierEngagementBody>,
) -> Res<Json<EngagementDto>> {
    let e = state
        .ent
        .modifier_engagement_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(e))
}

async fn prolonger_engagement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ProlongerEngagementBody>,
) -> Res<Json<EngagementDto>> {
    let e = state
        .ent
        .prolonger_engagement_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(e))
}

async fn terminer_engagement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TerminerEngagementBody>,
) -> Res<Json<EngagementDto>> {
    let e = state
        .ent
        .terminer_engagement_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(e))
}

async fn corriger_engagement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .corriger_engagement_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn suspendre_engagement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SuspendreEngagementBody>,
) -> Res<Json<EngagementDto>> {
    let e = state
        .ent
        .suspendre_engagement_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(e))
}

async fn reprendre_engagement(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ReprendreEngagementBody>,
) -> Res<Json<EngagementDto>> {
    let e = state
        .ent
        .reprendre_engagement_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(e))
}

async fn ajouter_fonction(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AgentFonctionBody>,
) -> Res<(StatusCode, Json<AgentFonctionDto>)> {
    let f = state
        .ent
        .ajouter_fonction_agent_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(f)))
}

async fn terminer_fonction(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TerminerLigneBody>,
) -> Res<StatusCode> {
    state
        .ent
        .terminer_fonction_agent_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn ajouter_discipline(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AgentDisciplineBody>,
) -> Res<(StatusCode, Json<AgentDisciplineDto>)> {
    let d = state
        .ent
        .ajouter_discipline_agent_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(d)))
}

async fn terminer_discipline(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TerminerLigneBody>,
) -> Res<StatusCode> {
    state
        .ent
        .terminer_discipline_agent_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn enregistrer_qualification(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<QualificationBody>,
) -> Res<(StatusCode, Json<QualificationDto>)> {
    let q = state
        .ent
        .enregistrer_qualification_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(q)))
}

async fn annuler_qualification(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .annuler_qualification_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn ajouter_identifiant(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<IdentifiantAgentBody>,
) -> Res<(StatusCode, Json<IdentifiantAgentDto>)> {
    let i = state
        .ent
        .ajouter_identifiant_agent_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(i)))
}

async fn clore_identifiant(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .clore_identifiant_agent_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn identite_restreinte(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<IdentiteRestreinteBody>,
) -> Res<StatusCode> {
    state
        .ent
        .definir_identite_restreinte_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn personnel_mineur(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PersonnelMineurBody>,
) -> Res<StatusCode> {
    state
        .ent
        .definir_personnel_mineur_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn fusionner_agents(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<FusionnerAgentsBody>,
) -> Res<(StatusCode, Json<FusionAgentsDto>)> {
    let f = state
        .ent
        .fusionner_agents_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(f)))
}

async fn definir_contact_urgence(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<ContactUrgenceAgentBody>,
) -> Res<Json<ContactUrgenceAgentDto>> {
    let c = state
        .ent
        .definir_contact_urgence_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(c))
}

async fn retirer_contact_urgence(
    State(state): State<AppState>,
    session: EntSession,
    Path(ordre): Path<i16>,
) -> Res<StatusCode> {
    state
        .ent
        .retirer_contact_urgence_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            ordre,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn definir_photo(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<PhotoAgentBody>,
) -> Res<StatusCode> {
    state
        .ent
        .definir_photo_agent_for_actor(session.etablissement_id, session.user_id, &session.role, &b)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn definir_pause(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<PauseNotificationsBody>,
) -> Res<StatusCode> {
    state
        .ent
        .definir_pause_notifications_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}
