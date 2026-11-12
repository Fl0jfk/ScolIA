//! Routes `/internat/*` du socle 41, lot I1 : hébergement (zones, chambres, lits), séjours et
//! rythmes, affectations sous contraintes, dérogations, plan, paramètres.
//!
//! L'établissement vient de la session, jamais du corps. Les droits (`internat.gerer`, délégations
//! D-017 comprises) et toutes les règles (régime daté, public de la chambre, interdiction de
//! contact, capacité) sont vérifiés par `ent-db` à chaque appel ; ici, aucune règle.
//! **Aucune route sans session** : ni lien public, ni jeton (corrige NJ-22 et NJ-51 côté Rust).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, post, put};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::internat::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/internat/zones", get(zones).post(creer_zone))
        .route("/internat/zones/{id}", patch(modifier_zone))
        .route("/internat/chambres", get(chambres).post(creer_chambre))
        .route("/internat/chambres/{id}/lits", get(lits))
        .route(
            "/internat/lits/{id}/neutralisation",
            put(neutraliser_lit).delete(retablir_lit),
        )
        .route("/internat/eleves/{id}/sejours", get(sejours))
        .route(
            "/internat/eleves/{id}/sejours/synchroniser",
            post(synchroniser_sejours),
        )
        .route("/internat/sejours/{id}/rythmes", post(definir_rythme))
        .route("/internat/affectations", post(affecter))
        .route(
            "/internat/eleves/{id}/changement-chambre",
            post(changer_chambre),
        )
        .route("/internat/derogations", post(accorder_derogation))
        .route(
            "/internat/interdictions-contact",
            post(poser_interdiction_contact),
        )
        .route("/internat/plan", get(plan))
        .route("/internat/parametres", get(parametres).put(modifier_parametres))
}

#[derive(Debug, Deserialize)]
struct PlanQuery {
    jour: Option<NaiveDate>,
}

async fn zones(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<ZoneDto>>> {
    Ok(Json(
        s.ent
            .lister_zones_internat_for_actor(se.etablissement_id, se.user_id, &se.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn creer_zone(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<ZoneSaisieDto>,
) -> Res<(StatusCode, Json<ZoneDto>)> {
    let z = s
        .ent
        .creer_zone_internat_for_actor(se.etablissement_id, se.user_id, &se.role, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(z)))
}

async fn modifier_zone(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ZoneModifDto>,
) -> Res<Json<ZoneModifieeDto>> {
    Ok(Json(
        s.ent
            .modifier_zone_internat_for_actor(se.etablissement_id, se.user_id, &se.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn chambres(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<ChambreEtendueDto>>> {
    Ok(Json(
        s.ent
            .lister_chambres_etendues_for_actor(se.etablissement_id, se.user_id, &se.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn creer_chambre(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<ChambreSaisieDto>,
) -> Res<(StatusCode, Json<ChambreEtendueDto>)> {
    let c = s
        .ent
        .creer_chambre_etendue_for_actor(se.etablissement_id, se.user_id, &se.role, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(c)))
}

async fn lits(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<LitDto>>> {
    Ok(Json(
        s.ent
            .lister_lits_internat_for_actor(se.etablissement_id, se.user_id, &se.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn neutraliser_lit(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<NeutraliserLitDto>,
) -> Res<StatusCode> {
    s.ent
        .neutraliser_lit_internat_for_actor(se.etablissement_id, se.user_id, &se.role, id, &b)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn retablir_lit(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .retablir_lit_internat_for_actor(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn sejours(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<SejourDto>>> {
    Ok(Json(
        s.ent
            .lister_sejours_internat_for_actor(se.etablissement_id, se.user_id, &se.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn synchroniser_sejours(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<SejourDto>>> {
    Ok(Json(
        s.ent
            .synchroniser_sejours_internat_for_actor(se.etablissement_id, se.user_id, &se.role, id)
            .await
            .map_err(map_db)?,
    ))
}

async fn definir_rythme(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RythmeSaisieDto>,
) -> Res<(StatusCode, Json<SejourDto>)> {
    let j = s
        .ent
        .definir_rythme_internat_for_actor(se.etablissement_id, se.user_id, &se.role, id, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(j)))
}

async fn affecter(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<AffectationSaisieDto>,
) -> Res<(StatusCode, Json<AffectationHebergementDto>)> {
    let a = s
        .ent
        .affecter_lit_internat_for_actor(se.etablissement_id, se.user_id, &se.role, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(a)))
}

async fn changer_chambre(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ChangementChambreDto>,
) -> Res<Json<AffectationHebergementDto>> {
    Ok(Json(
        s.ent
            .changer_chambre_internat_for_actor(se.etablissement_id, se.user_id, &se.role, id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn accorder_derogation(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<DerogationSaisieDto>,
) -> Res<(StatusCode, Json<DerogationDto>)> {
    let d = s
        .ent
        .accorder_derogation_internat_for_actor(se.etablissement_id, se.user_id, &se.role, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(d)))
}

async fn poser_interdiction_contact(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<InterdictionContactSaisieDto>,
) -> Res<(StatusCode, Json<InterdictionContactDto>)> {
    let d = s
        .ent
        .poser_interdiction_contact_internat_for_actor(
            se.etablissement_id,
            se.user_id,
            &se.role,
            &b,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(d)))
}

async fn plan(
    State(s): State<AppState>,
    se: EntSession,
    Query(q): Query<PlanQuery>,
) -> Res<Json<PlanDto>> {
    let jour = match q.jour {
        Some(j) => j,
        None => s
            .ent
            .nuit_courante_internat(se.etablissement_id)
            .await
            .map_err(map_db)?,
    };
    Ok(Json(
        s.ent
            .plan_internat_for_actor(se.etablissement_id, se.user_id, &se.role, jour)
            .await
            .map_err(map_db)?,
    ))
}

async fn parametres(State(s): State<AppState>, se: EntSession) -> Res<Json<ParametresInternatDto>> {
    Ok(Json(
        s.ent
            .parametres_internat_for_actor(se.etablissement_id, se.user_id, &se.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn modifier_parametres(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<ParametresInternatModifDto>,
) -> Res<Json<ParametresInternatDto>> {
    Ok(Json(
        s.ent
            .modifier_parametres_internat_for_actor(se.etablissement_id, se.user_id, &se.role, &b)
            .await
            .map_err(map_db)?,
    ))
}
