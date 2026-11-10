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
use crate::routes::{base_url_publique, map_compte_sensible, map_db};
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
        // Lot P2 : invitations (PRS-20 à 23), accueil (24 à 26), casquettes (29).
        .route(
            "/personnels/agents/{id}/invitations",
            get(invitations_agent).post(inviter_agent),
        )
        .route("/personnels/invitations/lot", post(inviter_lot))
        .route(
            "/personnels/invitations/{id}/annuler",
            post(annuler_invitation),
        )
        .route(
            "/personnels/invitations/{id}/relancer",
            post(relancer_invitation),
        )
        .route("/invitations/personnel/consulter", post(consulter_invitation))
        .route("/invitations/personnel/accepter", post(accepter_invitation))
        .route("/personnels/moi/arrivee", get(preparer_arrivee))
        .route("/auth/casquettes", get(casquettes))
        .route("/auth/casquette", post(basculer_casquette))
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

// ---------------------------------------------------------------------------------------------
// Lot P2 : invitations, accueil, casquettes
// ---------------------------------------------------------------------------------------------

fn map_p2(e: ent_db::EntDbError) -> (StatusCode, String) {
    map_compte_sensible(&e).unwrap_or_else(|| map_db(e))
}

/// 201 : invitation par e-mail (jamais de jeton ni d'adresse en clair) ou par QR code (le seul
/// corps qui porte le lien, affiché une fois à l'écran du secrétariat).
async fn inviter_agent(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<InviterAgentBody>,
) -> Response {
    match state
        .ent
        .inviter_agent_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b,
            &base_url_publique(),
        )
        .await
    {
        Ok(ent_db::InvitationEmise::Email(d)) => (StatusCode::CREATED, Json(d)).into_response(),
        Ok(ent_db::InvitationEmise::Qr(q)) => (StatusCode::CREATED, Json(q)).into_response(),
        Err(e) => map_p2(e).into_response(),
    }
}

async fn invitations_agent(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<InvitationDto>>> {
    let v = state
        .ent
        .invitations_agent_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_p2)?;
    Ok(Json(v))
}

async fn annuler_invitation(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<InvitationDto>> {
    let v = state
        .ent
        .annuler_invitation_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_p2)?;
    Ok(Json(v))
}

/// Relancer une invitation par QR code renvoie un nouveau lien (seul corps qui en porte un).
async fn relancer_invitation(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Response {
    match state
        .ent
        .relancer_invitation_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &base_url_publique(),
        )
        .await
    {
        Ok(ent_db::InvitationEmise::Email(d)) => (StatusCode::CREATED, Json(d)).into_response(),
        Ok(ent_db::InvitationEmise::Qr(q)) => (StatusCode::CREATED, Json(q)).into_response(),
        Err(e) => map_p2(e).into_response(),
    }
}

async fn inviter_lot(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<InvitationsLotBody>,
) -> Res<Json<Vec<InvitationLotLigneDto>>> {
    let v = state
        .ent
        .inviter_lot_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &b,
            &base_url_publique(),
        )
        .await
        .map_err(map_p2)?;
    Ok(Json(v))
}

/// Public : le jeton est la preuve. Toute invitation non valide reçoit la même réponse 400.
async fn consulter_invitation(
    State(state): State<AppState>,
    Json(b): Json<InvitationJetonBody>,
) -> Res<Json<InvitationApercuDto>> {
    match state.ent.invitation_personnel_apercu(&b.token).await {
        Ok(a) => Ok(Json(a)),
        Err(ent_db::EntDbError::Refus(_, m)) => Err((StatusCode::BAD_REQUEST, m.to_string())),
        Err(e) => Err(map_p2(e)),
    }
}

/// Public. Identité existante : l'appelant présente la session du compte concerné (`Authorization`)
/// et `confirmer_rattachement: true`. Aucune session n'est ouverte par cette route.
async fn accepter_invitation(
    State(state): State<AppState>,
    tenant: crate::tenant::TenantHote,
    headers: axum::http::HeaderMap,
    Json(b): Json<AccepterInvitationBody>,
) -> Response {
    use ent_db::ErreurAcceptation as E;
    let session = match headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        Some(h) => state.ent.resolve_session(h).await.ok(),
        None => None,
    };
    // Sur l'adresse d'un établissement, seule l'invitation de cet établissement est acceptée.
    let etab_jeton = b.token.get(..32).and_then(|p| Uuid::parse_str(p).ok());
    if let Some(e) = etab_jeton {
        if !tenant.accepte(e) {
            return (StatusCode::BAD_REQUEST, ent_db::personnels_acces::MSG_INVITATION_INVALIDE.to_string())
                .into_response();
        }
    }
    match state
        .ent
        .accepter_invitation_personnel(&b, session.as_ref())
        .await
    {
        Ok(d) => (StatusCode::OK, Json(d)).into_response(),
        Err(E::Invalide) => (
            StatusCode::BAD_REQUEST,
            ent_db::personnels_acces::MSG_INVITATION_INVALIDE.to_string(),
        )
            .into_response(),
        Err(E::ConnexionRequise) => (
            StatusCode::UNAUTHORIZED,
            "connexion_requise : connectez-vous avec le compte concerné, puis recommencez."
                .to_string(),
        )
            .into_response(),
        Err(E::ConfirmationRequise) => (
            StatusCode::CONFLICT,
            "confirmation_requise : confirmez explicitement le rattachement de votre identité."
                .to_string(),
        )
            .into_response(),
        Err(E::Conflit(m)) => (StatusCode::CONFLICT, m.to_string()).into_response(),
        Err(E::MotDePasse(m)) => (StatusCode::UNPROCESSABLE_ENTITY, m).into_response(),
        Err(E::Db(e)) => map_p2(e).into_response(),
    }
}

async fn preparer_arrivee(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<PreparerArriveeDto>> {
    let ctx = ctx_session(&session);
    let v = state.ent.preparer_arrivee(&ctx).await.map_err(map_p2)?;
    Ok(Json(v))
}

async fn casquettes(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<CasquettesDto>> {
    let ctx = ctx_session(&session);
    let v = state
        .ent
        .casquettes_de_la_session(&ctx)
        .await
        .map_err(map_p2)?;
    Ok(Json(v))
}

async fn basculer_casquette(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<BasculerCasquetteBody>,
) -> Res<Json<CasquettesDto>> {
    let ctx = ctx_session(&session);
    let v = state
        .ent
        .basculer_casquette(&ctx, &b.casquette)
        .await
        .map_err(map_p2)?;
    Ok(Json(v))
}

fn ctx_session(s: &EntSession) -> ent_db::CtxSession {
    ent_db::CtxSession {
        session_id: s.session_id,
        user_id: s.user_id,
        etablissement_id: s.etablissement_id,
        casquette: s.casquette.clone(),
        assurance: s.assurance,
        portee: s.portee,
    }
}
