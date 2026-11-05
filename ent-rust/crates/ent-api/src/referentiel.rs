//! Routes `/referentiel/*` : années, classes, groupes, affectations, services d'enseignement.
//!
//! Droits (autorité : `ent-db`, qui re-vérifie le rôle) : écriture Admin / Direction /
//! Administratif ; lecture tout le personnel ; parents et élèves : refusé. Les listes
//! d'élèves passent par la minimisation d'identité (`PgEnt::niveau_identite`).
//! Une référence d'un autre établissement répond 404, comme une référence inconnue.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use domain_eleve::today_utc;
use ent_auth::Action;
use ent_db::{
    Acteur, Affectation, AnneeScolaire, Classe, ClasseResume, CreerService, Groupe, PgEnt,
    RapportAmorcage, ServiceEnseignement,
};
use ent_types::referentiel::{
    AffectationDto, AffecterEleveBody, AnneeScolaireDto, ClasseDetailDto, ClasseDto,
    CreerAnneeBody, CreerClasseBody, CreerGroupeBody, CreerServiceBody, EleveClasseDto,
    GroupeDto, RapportAmorcageDto, ServiceDto,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/referentiel/annees", get(list_annees).post(creer_annee))
        .route("/referentiel/classes", get(list_classes).post(creer_classe))
        .route("/referentiel/classes/{id}", get(get_classe))
        .route(
            "/referentiel/classes/{id}/affectations",
            post(affecter_eleve),
        )
        .route("/referentiel/groupes", post(creer_groupe))
        .route("/referentiel/services", post(creer_service))
        .route("/referentiel/amorcer", post(amorcer))
        .route("/referentiel/mes-classes", get(mes_classes))
}

#[derive(Debug, Deserialize)]
struct ClassesQuery {
    annee_id: Option<Uuid>,
    jour: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
struct JourQuery {
    jour: Option<NaiveDate>,
}

fn annee_dto(a: &AnneeScolaire) -> AnneeScolaireDto {
    AnneeScolaireDto {
        id: a.id,
        libelle: a.libelle.clone(),
        date_debut: a.date_debut,
        date_fin: a.date_fin,
        courante: a.courante,
    }
}

fn classe_dto(c: &ClasseResume, labels: &HashMap<Uuid, String>) -> ClasseDto {
    ClasseDto {
        id: c.id,
        annee_id: c.annee_id,
        code: c.code.clone(),
        libelle: c.libelle.clone(),
        niveau: c.niveau.clone(),
        effectif: c.effectif,
        professeur_principal_id: c.professeur_principal_id,
        professeur_principal_label: c
            .professeur_principal_id
            .and_then(|p| labels.get(&p).cloned()),
    }
}

fn groupe_dto(g: &Groupe) -> GroupeDto {
    GroupeDto {
        id: g.id,
        annee_id: g.annee_id,
        code: g.code.clone(),
        libelle: g.libelle.clone(),
        classe_id: g.classe_id,
    }
}

fn service_dto(s: &ServiceEnseignement, labels: &HashMap<Uuid, String>) -> ServiceDto {
    ServiceDto {
        id: s.id,
        annee_id: s.annee_id,
        prof_user_id: s.prof_user_id,
        prof_label: labels.get(&s.prof_user_id).cloned(),
        matiere_id: s.matiere_id,
        matiere_libelle: s.matiere_libelle.clone(),
        classe_id: s.classe_id,
        groupe_id: s.groupe_id,
        professeur_principal: s.professeur_principal,
    }
}

fn affectation_dto(a: &Affectation) -> AffectationDto {
    AffectationDto {
        id: a.id,
        eleve_id: a.eleve_id,
        classe_id: a.classe_id,
        groupe_id: a.groupe_id,
        date_debut: a.date_debut,
        date_fin: a.date_fin,
    }
}

fn classe_simple_dto(c: &Classe) -> ClasseDto {
    ClasseDto {
        id: c.id,
        annee_id: c.annee_id,
        code: c.code.clone(),
        libelle: c.libelle.clone(),
        niveau: c.niveau.clone(),
        effectif: 0,
        professeur_principal_id: None,
        professeur_principal_label: None,
    }
}

async fn libelles_classes(state: &AppState, etablissement_id: Uuid, list: &[ClasseResume]) -> Res<HashMap<Uuid, String>> {
    let ids: Vec<Uuid> = list
        .iter()
        .filter_map(|c| c.professeur_principal_id)
        .collect();
    state.ent.user_labels(etablissement_id, &ids).await.map_err(map_db)
}

fn classe_introuvable() -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, "classe introuvable".into())
}

/// Autorisation par le moteur de droits (D-017). Renvoie le marqueur « autorisé par le
/// moteur » à transmettre aux fonctions de `ent-db` (qui conservent leur garde historique).
pub(crate) async fn role_moteur(
    state: &AppState,
    session: &EntSession,
    action: Action,
) -> Res<&'static str> {
    let acteur = acteur(session);
    if state.ent.ref_peut(&acteur, action).await.map_err(map_db)? {
        Ok(if action == Action::ReferentielGerer {
            ent_db::ROLE_AUTORISE_MOTEUR_GESTION
        } else {
            ent_db::ROLE_AUTORISE_MOTEUR_LECTURE
        })
    } else {
        Err((StatusCode::FORBIDDEN, "accès refusé pour ce rôle".into()))
    }
}

pub(crate) fn acteur(session: &EntSession) -> Acteur<'_> {
    Acteur {
        etablissement_id: session.etablissement_id,
        user_id: session.user_id,
        role: &session.role,
    }
}

async fn refus_gestion(state: &AppState, session: &EntSession) -> Res<()> {
    role_moteur(state, session, Action::ReferentielGerer).await.map(|_| ())
}

async fn list_annees(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<AnneeScolaireDto>>> {
    let role = role_moteur(&state, &session, Action::ReferentielLire).await?;
    let list = state
        .ent
        .referentiel_annees(role, session.etablissement_id)
        .await
        .map_err(map_db)?;
    Ok(Json(list.iter().map(annee_dto).collect()))
}

async fn creer_annee(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerAnneeBody>,
) -> Res<(StatusCode, Json<AnneeScolaireDto>)> {
    let role = role_moteur(&state, &session, Action::ReferentielGerer).await?;
    let a = state
        .ent
        .referentiel_creer_annee(
            role,
            session.etablissement_id,
            &body.libelle,
            body.date_debut,
            body.date_fin,
            body.courante,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(annee_dto(&a))))
}

async fn list_classes(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<ClassesQuery>,
) -> Res<Json<Vec<ClasseDto>>> {
    let role = role_moteur(&state, &session, Action::ReferentielLire).await?;
    let list = state
        .ent
        .referentiel_classes(
            role,
            session.etablissement_id,
            q.annee_id,
            q.jour.unwrap_or_else(today_utc),
        )
        .await
        .map_err(map_db)?;
    let labels = libelles_classes(&state, session.etablissement_id, &list).await?;
    Ok(Json(list.iter().map(|c| classe_dto(c, &labels)).collect()))
}

async fn creer_classe(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerClasseBody>,
) -> Res<(StatusCode, Json<ClasseDto>)> {
    let role = role_moteur(&state, &session, Action::ReferentielGerer).await?;
    let c = state
        .ent
        .referentiel_creer_classe(
            role,
            session.etablissement_id,
            body.annee_id,
            &body.code,
            body.libelle.as_deref(),
            body.niveau.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(classe_simple_dto(&c))))
}

async fn get_classe(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<JourQuery>,
) -> Res<Json<ClasseDetailDto>> {
    let role = role_moteur(&state, &session, Action::ReferentielLire).await?;
    let jour = q.jour.unwrap_or_else(today_utc);
    let resume = state
        .ent
        .referentiel_classe_resume(role, session.etablissement_id, id, jour)
        .await
        .map_err(map_db)?
        .ok_or_else(classe_introuvable)?;
    let niveau = PgEnt::niveau_identite(&session.role);
    let eleves = state
        .ent
        .eleves_de_la_classe(session.etablissement_id, id, jour)
        .await
        .map_err(map_db)?;
    let services = state
        .ent
        .referentiel_services_de_classe(session.etablissement_id, id)
        .await
        .map_err(map_db)?;
    let groupes = state
        .ent
        .referentiel_groupes_de_classe(session.etablissement_id, id)
        .await
        .map_err(map_db)?;
    let mut ids: Vec<Uuid> = services.iter().map(|s| s.prof_user_id).collect();
    ids.extend(resume.professeur_principal_id);
    let labels = state.ent.user_labels(session.etablissement_id, &ids).await.map_err(map_db)?;
    Ok(Json(ClasseDetailDto {
        classe: classe_dto(&resume, &labels),
        jour,
        eleves: eleves
            .into_iter()
            .map(|mut e| {
                e.minimise(niveau);
                EleveClasseDto {
                    id: e.id,
                    nom: e.nom,
                    prenom: e.prenom,
                    ine: e.ine,
                    date_naissance: e.date_naissance,
                    status: e.status.as_str().to_string(),
                }
            })
            .collect(),
        services: services.iter().map(|s| service_dto(s, &labels)).collect(),
        groupes: groupes.iter().map(groupe_dto).collect(),
    }))
}

async fn affecter_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<AffecterEleveBody>,
) -> Res<(StatusCode, Json<AffectationDto>)> {
    // Droit vérifié avant toute lecture : un rôle sans droit ne sonde pas les identifiants.
    let role = role_moteur(&state, &session, Action::ReferentielGerer).await?;
    let a = state
        .ent
        .referentiel_affecter_eleve_classe(
            role,
            session.etablissement_id,
            id,
            body.eleve_id,
            body.date_debut,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(affectation_dto(&a))))
}

async fn creer_groupe(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerGroupeBody>,
) -> Res<(StatusCode, Json<GroupeDto>)> {
    let role = role_moteur(&state, &session, Action::ReferentielGerer).await?;
    let g = state
        .ent
        .referentiel_creer_groupe(
            role,
            session.etablissement_id,
            body.annee_id,
            &body.code,
            body.libelle.as_deref(),
            body.classe_id,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(groupe_dto(&g))))
}

async fn creer_service(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<CreerServiceBody>,
) -> Res<(StatusCode, Json<ServiceDto>)> {
    let role = role_moteur(&state, &session, Action::ReferentielGerer).await?;
    let s = state
        .ent
        .referentiel_creer_service(
            role,
            session.etablissement_id,
            CreerService {
                prof_user_id: body.prof_user_id,
                matiere_id: body.matiere_id,
                classe_id: body.classe_id,
                groupe_id: body.groupe_id,
                professeur_principal: body.professeur_principal,
            },
        )
        .await
        .map_err(map_db)?;
    let labels = state
        .ent
        .user_labels(session.etablissement_id, &[s.prof_user_id])
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(service_dto(&s, &labels))))
}

async fn amorcer(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<RapportAmorcageDto>> {
    refus_gestion(&state, &session).await?;
    let r: RapportAmorcage = state
        .ent
        .amorcer_referentiel(session.etablissement_id)
        .await
        .map_err(map_db)?;
    Ok(Json(RapportAmorcageDto {
        annee_id: r.annee_id,
        annee_libelle: r.annee_libelle,
        classes_creees: r.classes_creees,
        affectations_creees: r.affectations_creees,
        services_crees: r.services_crees,
        divergences: r.divergences,
    }))
}

/// Professeur : ses classes (via ses services). Autre personnel : toutes les classes.
async fn mes_classes(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Res<Json<Vec<ClasseDto>>> {
    session.require_staff()?;
    let jour = q.jour.unwrap_or_else(today_utc);
    let est_prof = matches!(
        ent_auth::Role::parse(&session.role),
        Some(ent_auth::Role::Prof)
    );
    let list = if est_prof {
        state
            .ent
            .classes_du_prof(session.etablissement_id, session.user_id, jour)
            .await
    } else {
        state
            .ent
            .referentiel_classes(&session.role, session.etablissement_id, None, jour)
            .await
    }
    .map_err(map_db)?;
    let labels = libelles_classes(&state, session.etablissement_id, &list).await?;
    Ok(Json(list.iter().map(|c| classe_dto(c, &labels)).collect()))
}
