//! Routes des rôles paramétrables (D-017, `docs/ROLES_ET_DROITS.md`).
//!
//! L'autorité est `ent-db` (action `roles.gerer`, anti-escalade, garde-fous).
//! Une référence d'un autre établissement répond 404, comme une référence inconnue.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_auth::{Action, Portee};
use ent_db::{Droit, RoleEtab, RoleSaisie};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/roles/catalogue", get(catalogue))
        .route("/roles", get(list_roles).post(creer_role))
        .route("/roles/{id}", put(modifier_role).delete(supprimer_role))
        .route(
            "/roles/{id}/membres/{user_id}",
            put(attribuer_role).delete(retirer_role),
        )
        .route("/roles/mes-droits", get(mes_droits))
        .route(
            "/referentiel/groupes/{id}/responsables",
            put(definir_responsables),
        )
}

#[derive(Serialize)]
struct ActionDto {
    code: &'static str,
    libelle: &'static str,
    portees: Vec<&'static str>,
}

#[derive(Serialize)]
struct PorteeDto {
    code: &'static str,
    libelle: &'static str,
}

#[derive(Serialize)]
struct CatalogueDto {
    actions: Vec<ActionDto>,
    portees: Vec<PorteeDto>,
}

/// Catalogue fixe des actions et portées (lecture : tout le personnel).
async fn catalogue(session: EntSession) -> Res<Json<CatalogueDto>> {
    session.require_staff()?;
    Ok(Json(CatalogueDto {
        actions: Action::TOUTES
            .iter()
            .map(|a| ActionDto {
                code: a.code(),
                libelle: a.libelle(),
                portees: a.portees_admises().iter().map(|p| p.code()).collect(),
            })
            .collect(),
        portees: Portee::TOUTES
            .iter()
            .map(|p| PorteeDto {
                code: p.code(),
                libelle: p.libelle(),
            })
            .collect(),
    }))
}

#[derive(Serialize, Deserialize)]
struct DroitDto {
    action: String,
    portee: String,
    #[serde(default)]
    cibles: Vec<Uuid>,
}

#[derive(Serialize)]
struct RoleDto {
    id: Uuid,
    nom: String,
    description: Option<String>,
    modele: Option<String>,
    droits: Vec<DroitDto>,
    membres: Vec<Uuid>,
}

#[derive(Deserialize)]
struct RoleBody {
    nom: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    modele: Option<String>,
    #[serde(default)]
    droits: Vec<DroitDto>,
}

#[derive(Deserialize, Default)]
struct AttributionBody {
    #[serde(default)]
    fin: Option<NaiveDate>,
}

#[derive(Deserialize)]
struct ResponsablesBody {
    responsables: Vec<Uuid>,
}

fn droit_dto(d: &Droit) -> DroitDto {
    DroitDto {
        action: d.action.code().into(),
        portee: d.portee.code().into(),
        cibles: d.cibles.clone(),
    }
}

fn role_dto(r: RoleEtab) -> RoleDto {
    RoleDto {
        id: r.id,
        nom: r.nom,
        description: r.description,
        modele: r.modele,
        droits: r.droits.iter().map(droit_dto).collect(),
        membres: r.membres,
    }
}

fn saisie(body: RoleBody) -> Res<RoleSaisie> {
    let mut droits = Vec::with_capacity(body.droits.len());
    for d in body.droits {
        let action = Action::parse(&d.action)
            .ok_or((StatusCode::BAD_REQUEST, "action inconnue".to_string()))?;
        let portee = Portee::parse(&d.portee)
            .ok_or((StatusCode::BAD_REQUEST, "portée inconnue".to_string()))?;
        droits.push(Droit {
            action,
            portee,
            cibles: d.cibles,
        });
    }
    Ok(RoleSaisie {
        nom: body.nom,
        description: body.description,
        modele: body.modele,
        droits,
    })
}

async fn list_roles(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<RoleDto>>> {
    let roles = state
        .ent
        .list_roles_for_actor(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(roles.into_iter().map(role_dto).collect()))
}

async fn creer_role(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<RoleBody>,
) -> Res<(StatusCode, Json<RoleDto>)> {
    let r = state
        .ent
        .creer_role_for_actor(session.etablissement_id, session.user_id, &session.role, saisie(body)?)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(role_dto(r))))
}

async fn modifier_role(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<RoleBody>,
) -> Res<Json<RoleDto>> {
    let r = state
        .ent
        .modifier_role_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            saisie(body)?,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(role_dto(r)))
}

async fn supprimer_role(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .supprimer_role_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn attribuer_role(
    State(state): State<AppState>,
    session: EntSession,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
    body: Option<Json<AttributionBody>>,
) -> Res<StatusCode> {
    let fin = body.map(|Json(b)| b.fin).unwrap_or_default();
    state
        .ent
        .attribuer_role_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            user_id,
            fin,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn retirer_role(
    State(state): State<AppState>,
    session: EntSession,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> Res<StatusCode> {
    let ok = state
        .ent
        .retirer_role_for_actor(session.etablissement_id, session.user_id, &session.role, id, user_id)
        .await
        .map_err(map_db)?;
    if ok {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "attribution introuvable".into()))
    }
}

/// Droits effectifs de la personne connectée (pour adapter l'interface).
async fn mes_droits(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<DroitDto>>> {
    let d = state
        .ent
        .droits_effectifs(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(d.iter().map(droit_dto).collect()))
}

async fn definir_responsables(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ResponsablesBody>,
) -> Res<Json<Vec<Uuid>>> {
    let r = state
        .ent
        .definir_responsables_groupe_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &body.responsables,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(r))
}
