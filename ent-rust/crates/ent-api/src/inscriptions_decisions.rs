//! Routes du lot I5 (socle 3) : **décisions**, avis internes, déport, commissions, publication, liste d'attente,
//! propositions de place et conditions de confirmation.
//!
//! L'autorité est `ent-db` (`inscriptions_decisions.rs`, `inscriptions_attente.rs`) : droits
//! (`inscriptions.gerer`, `inscriptions.decider`), dossier de l'établissement seulement (un identifiant d'un autre
//! établissement répond « introuvable »), déposant vérifié pour la famille. Types JSON :
//! `ent_types::inscriptions_decisions`. Le temps vient du serveur (`Utc::now()`) et se passe en paramètre au service.
//!
//! * **Personnel** (session) : décider, revoir, historique, courrier, publier (ou programmer), avis, déport,
//!   commissions (séance, dossiers, décisions en lot, publication groupée, clôture), conditions, liste d'attente,
//!   propositions (envoyer sur confirmation, passer son tour, constater l'expiration).
//! * **Famille** (en-tête `X-Preinscription`) : lire la décision **publiée**, répondre à la place proposée.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use ent_types::inscriptions_decisions::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::inscriptions_pieces::{corps, etab_jeton, jeton, map, Res};
use crate::tenant::TenantHote;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Décisions.
        .route(
            "/inscriptions/dossiers/{id}/decisions",
            get(decisions).post(decider),
        )
        .route("/inscriptions/dossiers/{id}/decisions/revoir", post(revoir))
        .route("/inscriptions/decisions/{id}/courrier", get(courrier))
        .route(
            "/inscriptions/decisions/{id}/publier",
            post(publier_decision),
        )
        // Avis et déport.
        .route(
            "/inscriptions/dossiers/{id}/avis",
            get(avis).post(donner_avis),
        )
        .route(
            "/inscriptions/dossiers/{id}/deport",
            get(deport_etat).post(se_deporter),
        )
        // Commissions.
        .route(
            "/inscriptions/commissions",
            get(commissions).post(creer_commission),
        )
        .route("/inscriptions/commissions/{id}", get(commission))
        .route(
            "/inscriptions/commissions/{id}/dossiers",
            post(dossiers_commission),
        )
        .route(
            "/inscriptions/commissions/{id}/decisions",
            post(decisions_en_lot),
        )
        .route(
            "/inscriptions/commissions/{id}/publier",
            post(publier_commission),
        )
        .route(
            "/inscriptions/commissions/{id}/cloturer",
            post(cloturer_commission),
        )
        // Conditions de confirmation.
        .route(
            "/inscriptions/campagnes/{id}/conditions",
            get(conditions_campagne).put(definir_conditions_campagne),
        )
        .route(
            "/inscriptions/dossiers/{id}/conditions",
            get(conditions_dossier),
        )
        .route(
            "/inscriptions/dossiers/{id}/conditions/reevaluer",
            post(reevaluer_conditions),
        )
        .route(
            "/inscriptions/dossiers/{id}/conditions/{code}/remplir",
            post(remplir_condition),
        )
        .route(
            "/inscriptions/dossiers/{id}/conditions/{code}/lever",
            post(lever_condition),
        )
        // Liste d'attente et propositions.
        .route(
            "/inscriptions/offres/{id}/liste-attente",
            get(liste_attente),
        )
        .route(
            "/inscriptions/offres/{id}/liste-attente/designer",
            post(designer_suivants),
        )
        .route("/inscriptions/propositions", get(propositions))
        .route(
            "/inscriptions/propositions/{id}/envoyer",
            post(envoyer_proposition),
        )
        .route(
            "/inscriptions/propositions/{id}/ecarter",
            post(ecarter_proposition),
        )
        .route(
            "/inscriptions/propositions/{id}/constater-expiration",
            post(constater_expiration),
        )
        // Famille (jeton de préinscription).
        .route(
            "/preinscription/dossiers/{id}/decision",
            get(decision_famille),
        )
        .route(
            "/preinscription/dossiers/{id}/proposition/repondre",
            post(repondre_famille),
        )
}

// ───────────────────────────── Décisions ─────────────────────────────

async fn decider(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    b: Result<Json<DecisionCompleteBody>, JsonRejection>,
) -> Res<Json<DecisionDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .decider_complete_for_actor(
                s.etablissement_id,
                s.user_id,
                &s.role,
                id,
                &b,
                false,
                Utc::now(),
            )
            .await
            .map_err(map)?,
    ))
}

async fn revoir(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    b: Result<Json<DecisionCompleteBody>, JsonRejection>,
) -> Res<Json<DecisionDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .decider_complete_for_actor(
                s.etablissement_id,
                s.user_id,
                &s.role,
                id,
                &b,
                true,
                Utc::now(),
            )
            .await
            .map_err(map)?,
    ))
}

async fn decisions(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<DecisionDto>>> {
    Ok(Json(
        state
            .ent
            .decisions_dossier_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn courrier(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<CourrierDto>> {
    Ok(Json(
        state
            .ent
            .courrier_decision_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn publier_decision(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    b: Result<Json<PublierBody>, JsonRejection>,
) -> Res<Json<PublicationDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .publier_decision_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b, Utc::now())
            .await
            .map_err(map)?,
    ))
}

// ───────────────────────────── Avis et déport ─────────────────────────────

async fn donner_avis(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    b: Result<Json<AvisBody>, JsonRejection>,
) -> Res<Json<AvisDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .donner_avis_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

async fn avis(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<AvisDto>>> {
    Ok(Json(
        state
            .ent
            .avis_dossier_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn deport_etat(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<DeportEtatDto>> {
    Ok(Json(
        state
            .ent
            .deport_etat_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn se_deporter(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    b: Result<Json<DeportBody>, JsonRejection>,
) -> Res<Json<DeportEtatDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .se_deporter_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

// ───────────────────────────── Commissions ─────────────────────────────

#[derive(Deserialize)]
struct FiltreCampagne {
    campagne_id: Option<Uuid>,
    statut: Option<String>,
}

async fn creer_commission(
    State(state): State<AppState>,
    s: EntSession,
    b: Result<Json<CommissionBody>, JsonRejection>,
) -> Res<Json<CommissionDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .creer_commission_admission_for_actor(s.etablissement_id, s.user_id, &s.role, &b)
            .await
            .map_err(map)?,
    ))
}

async fn commissions(
    State(state): State<AppState>,
    s: EntSession,
    Query(f): Query<FiltreCampagne>,
) -> Res<Json<Vec<CommissionDto>>> {
    Ok(Json(
        state
            .ent
            .list_commissions_admission_for_actor(
                s.etablissement_id,
                s.user_id,
                &s.role,
                f.campagne_id,
            )
            .await
            .map_err(map)?,
    ))
}

async fn commission(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<CommissionDto>> {
    Ok(Json(
        state
            .ent
            .commission_admission_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn dossiers_commission(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    b: Result<Json<CommissionDossiersBody>, JsonRejection>,
) -> Res<Json<CommissionDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .modifier_dossiers_commission_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

async fn decisions_en_lot(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    b: Result<Json<DecisionsEnLotBody>, JsonRejection>,
) -> Res<Json<RecapitulatifLotDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .decider_en_lot_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b, Utc::now())
            .await
            .map_err(map)?,
    ))
}

async fn publier_commission(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    b: Result<Json<PublierBody>, JsonRejection>,
) -> Res<Json<PublicationDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .publier_commission_for_actor(
                s.etablissement_id,
                s.user_id,
                &s.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map)?,
    ))
}

async fn cloturer_commission(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    b: Result<Json<CommissionClotureBody>, JsonRejection>,
) -> Res<Json<CommissionDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .cloturer_commission_for_actor(
                s.etablissement_id,
                s.user_id,
                &s.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map)?,
    ))
}

// ───────────────────────────── Conditions ─────────────────────────────

async fn conditions_campagne(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<String>>> {
    Ok(Json(
        state
            .ent
            .conditions_campagne_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn definir_conditions_campagne(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    b: Result<Json<ConditionsCampagneBody>, JsonRejection>,
) -> Res<Json<Vec<String>>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .definir_conditions_campagne_for_actor(s.etablissement_id, s.user_id, &s.role, id, &b)
            .await
            .map_err(map)?,
    ))
}

async fn conditions_dossier(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ConditionsDossierDto>> {
    Ok(Json(
        state
            .ent
            .conditions_dossier_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn reevaluer_conditions(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ConditionsDossierDto>> {
    Ok(Json(
        state
            .ent
            .reevaluer_conditions_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn remplir_condition(
    State(state): State<AppState>,
    s: EntSession,
    Path((id, code)): Path<(Uuid, String)>,
    b: Result<Json<RemplirConditionBody>, JsonRejection>,
) -> Res<Json<ConditionsDossierDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .remplir_condition_for_actor(s.etablissement_id, s.user_id, &s.role, id, &code, &b)
            .await
            .map_err(map)?,
    ))
}

async fn lever_condition(
    State(state): State<AppState>,
    s: EntSession,
    Path((id, code)): Path<(Uuid, String)>,
    b: Result<Json<MotifBody>, JsonRejection>,
) -> Res<Json<ConditionsDossierDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .lever_condition_for_actor(s.etablissement_id, s.user_id, &s.role, id, &code, &b)
            .await
            .map_err(map)?,
    ))
}

// ───────────────────────────── Liste d'attente et propositions ─────────────────────────────

async fn liste_attente(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ListeAttenteDto>> {
    Ok(Json(
        state
            .ent
            .liste_attente_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn designer_suivants(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ListeAttenteDto>> {
    Ok(Json(
        state
            .ent
            .designer_suivants_for_actor(s.etablissement_id, s.user_id, &s.role, id)
            .await
            .map_err(map)?,
    ))
}

async fn propositions(
    State(state): State<AppState>,
    s: EntSession,
    Query(f): Query<FiltreCampagne>,
) -> Res<Json<Vec<PropositionDto>>> {
    Ok(Json(
        state
            .ent
            .propositions_for_actor(
                s.etablissement_id,
                s.user_id,
                &s.role,
                f.campagne_id,
                f.statut,
                Utc::now(),
            )
            .await
            .map_err(map)?,
    ))
}

async fn envoyer_proposition(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    b: Result<Json<EnvoyerPropositionBody>, JsonRejection>,
) -> Res<Json<PropositionDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .envoyer_proposition_for_actor(
                s.etablissement_id,
                s.user_id,
                &s.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map)?,
    ))
}

async fn ecarter_proposition(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
    b: Result<Json<MotifBody>, JsonRejection>,
) -> Res<Json<PropositionDto>> {
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .ecarter_proposition_for_actor(
                s.etablissement_id,
                s.user_id,
                &s.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map)?,
    ))
}

async fn constater_expiration(
    State(state): State<AppState>,
    s: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<PropositionDto>> {
    Ok(Json(
        state
            .ent
            .constater_expiration_for_actor(s.etablissement_id, s.user_id, &s.role, id, Utc::now())
            .await
            .map_err(map)?,
    ))
}

// ───────────────────────────── Famille ─────────────────────────────

async fn decision_famille(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Res<Json<DecisionFamilleDto>> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    Ok(Json(
        state
            .ent
            .decision_famille(etab, &j, id)
            .await
            .map_err(map)?,
    ))
}

async fn repondre_famille(
    State(state): State<AppState>,
    tenant: TenantHote,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    b: Result<Json<ReponsePropositionBody>, JsonRejection>,
) -> Res<Json<DecisionFamilleDto>> {
    let j = jeton(&headers)?;
    let etab = etab_jeton(&state, &tenant, &j).await?;
    let b = corps(b)?;
    Ok(Json(
        state
            .ent
            .repondre_proposition_famille(etab, &j, id, &b, Utc::now())
            .await
            .map_err(map)?,
    ))
}
