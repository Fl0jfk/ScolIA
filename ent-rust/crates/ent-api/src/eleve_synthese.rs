//! Socle 2 (l'élève), lot E8 — routes des pastilles et de la synthèse de la fiche (le service
//! d'E9b). `GET /eleves/{id}/synthese`, `GET /eleves/{id}/pastilles`, `GET /eleves/pastilles`
//! (compteurs), `GET /eleves/pastilles/catalogue`, `GET /eleves/pastilles/{code}` (liste de travail).
//!
//! Personnel seulement. L'autorité est `ent-db` : une pastille ne se montre que si l'acteur détient,
//! sur cet élève, l'une des actions du catalogue ; un élève absent, hors de la portée de l'annuaire
//! ou protégé pour ce tiers répond `404`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use domain_eleve::{CATALOGUE_INDICATEURS, CONTRATS_TUILES};
use ent_db::eleve_indicateurs::{EtatTuile, PastilleVue, SyntheseEleve};
use ent_types::eleve_synthese as dto;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/eleves/pastilles", get(compteurs))
        .route("/eleves/pastilles/catalogue", get(catalogue))
        .route("/eleves/pastilles/{code}", get(liste_par_pastille))
        .route("/eleves/{id}/synthese", get(synthese))
        .route("/eleves/{id}/pastilles", get(pastilles))
}

#[derive(Debug, Deserialize)]
struct LimiteQuery {
    limite: Option<i64>,
}

fn pastille_dto(p: &PastilleVue) -> dto::PastilleDto {
    dto::PastilleDto {
        code: p.code.to_string(),
        domaine: p.domaine.to_string(),
        libelle: p.libelle.to_string(),
        icone: p.icone.to_string(),
        gravite: p.gravite.code().to_string(),
        valeur_code: p.valeur_code.clone(),
        valeur_date: p.valeur_date,
        du: p.du,
    }
}

fn synthese_dto(s: &SyntheseEleve) -> dto::SyntheseDto {
    dto::SyntheseDto {
        eleve_id: s.eleve_id,
        pastilles: s.pastilles.iter().map(pastille_dto).collect(),
        tuiles: s
            .tuiles
            .iter()
            .map(|t| dto::TuileDto {
                domaine: t.domaine.to_string(),
                libelle: t.libelle.to_string(),
                rang: t.rang,
                lien: t.lien.clone(),
                etat: match t.etat {
                    EtatTuile::Visible => "visible",
                    EtatTuile::Masquee(_) => "masquee",
                }
                .to_string(),
                raison: match t.etat {
                    EtatTuile::Visible => None,
                    EtatTuile::Masquee(r) => Some(r.code().to_string()),
                },
                pastilles: t.pastilles.iter().map(pastille_dto).collect(),
            })
            .collect(),
    }
}

/// `GET /eleves/{id}/synthese` — pastilles visibles et tuiles par domaine.
async fn synthese(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<dto::SyntheseDto>> {
    session.require_staff()?;
    let s = state
        .ent
        .synthese_eleve_pour_acteur(&session.role, session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?;
    Ok(Json(synthese_dto(&s)))
}

/// `GET /eleves/{id}/pastilles` — pastilles visibles seulement (en-tête de la fiche).
async fn pastilles(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<dto::PastilleDto>>> {
    session.require_staff()?;
    let l = state
        .ent
        .pastilles_eleve_pour_acteur(&session.role, session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(pastille_dto).collect()))
}

/// `GET /eleves/pastilles` — combien d'élèves par pastille (échelle de l'établissement).
async fn compteurs(State(state): State<AppState>, session: EntSession) -> Res<Json<Vec<dto::CompteurPastilleDto>>> {
    session.require_staff()?;
    let l = state
        .ent
        .compteurs_pastilles_pour_acteur(&session.role, session.etablissement_id, session.user_id)
        .await
        .map_err(map_db)?;
    Ok(Json(
        l.into_iter()
            .filter_map(|(code, nombre)| {
                let d = domain_eleve::indicateur_catalogue(code)?;
                Some(dto::CompteurPastilleDto {
                    code: code.to_string(),
                    libelle: d.libelle.to_string(),
                    gravite: d.gravite.code().to_string(),
                    nombre,
                })
            })
            .collect(),
    ))
}

/// `GET /eleves/pastilles/catalogue` — le catalogue du logiciel et le contrat des tuiles (aucune
/// donnée d'élève : lisible de tout le personnel).
async fn catalogue(session: EntSession) -> Res<Json<dto::CatalogueDto>> {
    session.require_staff()?;
    Ok(Json(dto::CatalogueDto {
        indicateurs: CATALOGUE_INDICATEURS
            .iter()
            .map(|d| dto::IndicateurCatalogueDto {
                code: d.code.to_string(),
                domaine: d.domaine.to_string(),
                libelle: d.libelle.to_string(),
                icone: d.icone.to_string(),
                gravite: d.gravite.code().to_string(),
            })
            .collect(),
        tuiles: CONTRATS_TUILES
            .iter()
            .map(|c| dto::ContratTuileDto {
                domaine: c.domaine.to_string(),
                libelle: c.libelle.to_string(),
                rang: c.rang,
                personnel_seul: c.personnel_seul,
            })
            .collect(),
    }))
}

/// `GET /eleves/pastilles/{code}` — liste de travail (« sans INE », « PAI en cours »…).
async fn liste_par_pastille(
    State(state): State<AppState>,
    session: EntSession,
    Path(code): Path<String>,
    Query(q): Query<LimiteQuery>,
) -> Res<Json<dto::ElevesPastilleDto>> {
    session.require_staff()?;
    let eleves = state
        .ent
        .eleves_par_pastille_pour_acteur(&session.role, session.etablissement_id, session.user_id, &code, q.limite.unwrap_or(500))
        .await
        .map_err(map_db)?;
    Ok(Json(dto::ElevesPastilleDto { code, eleves }))
}
