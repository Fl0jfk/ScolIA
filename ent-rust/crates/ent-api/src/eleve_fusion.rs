//! Socle 2 (l'élève), lot E6 — routes des doublons et de la fusion de fiches.
//!
//! * ELV-22 : `GET /eleves/doublons`, `POST /eleves/doublons/detecter` (secrétariat, direction :
//!   `eleves.identite.modifier` ou `eleves.fusionner`).
//! * ELV-33 : `POST /eleves/doublons/non-doublon` (« ce sont bien deux élèves »).
//! * ELV-23 : `POST /eleves/fusions/apercu` (rien n'est écrit), `POST /eleves/fusions`
//!   (`eleves.fusionner` : direction, administrateur).
//! * ELV-24 : `GET /eleves/fusions`, `POST /eleves/fusions/{id}/annuler` (30 jours).
//!
//! L'autorité est `ent-db` : chaque commande refait son contrôle (droit à la portée établissement,
//! élèves protégés, conflit d'intérêts) ; les routes ne font que traduire HTTP ↔ service. Une fiche
//! d'un autre établissement répond 404.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_db::eleve_doublons::{BilanDetection, DoublonVue};
use ent_db::eleve_fusion::{
    ApercuFusion, AvertissementFusion, ConflitFusion, DemandeFusion, FusionEleves, LigneApercu,
    LigneModifiee, ResultatDefusion, ResultatFusion,
};
use ent_db::EleveVue;
use ent_types::fusion as dto;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/eleves/doublons", get(lister_doublons))
        .route("/eleves/doublons/detecter", post(detecter_doublons))
        .route("/eleves/doublons/non-doublon", post(declarer_non_doublon))
        .route("/eleves/fusions", get(lister_fusions).post(fusionner))
        .route("/eleves/fusions/apercu", post(apercu_fusion))
        .route("/eleves/fusions/{fusion_id}/annuler", post(annuler_fusion))
}

#[derive(Debug, Deserialize)]
struct LimiteQuery {
    limite: Option<i64>,
}

fn comparaison(v: &EleveVue) -> dto::EleveComparaisonDto {
    dto::EleveComparaisonDto {
        id: v.eleve.id,
        nom: v.eleve.nom.clone(),
        prenom: v.eleve.prenom.clone(),
        classe: v.eleve.classe.clone(),
        statut: v.eleve.status.as_str().to_string(),
        date_naissance: v.eleve.date_naissance,
        ine: v.eleve.ine.clone(),
    }
}

fn doublon_dto(d: &DoublonVue) -> dto::DoublonDto {
    dto::DoublonDto {
        id: d.id,
        score: d.score,
        raisons: d.raisons.clone(),
        detecte_le: d.detecte_le,
        a: comparaison(&d.a),
        b: comparaison(&d.b),
    }
}

fn ligne_dto(l: &LigneApercu) -> dto::LigneApercuDto {
    dto::LigneApercuDto {
        domaine: l.domaine.clone(),
        table: l.table.clone(),
        a_deplacer: l.a_deplacer,
        en_conflit: l.en_conflit,
        restent: l.restent,
    }
}

fn conflit_dto(c: &ConflitFusion) -> dto::ConflitFusionDto {
    dto::ConflitFusionDto { table: c.table.clone(), cle: c.cle.clone(), domaine: c.domaine.clone() }
}

fn conflit_depuis(c: dto::ConflitFusionDto) -> ConflitFusion {
    ConflitFusion { table: c.table, cle: c.cle, domaine: c.domaine }
}

fn avertissement_code(a: AvertissementFusion) -> &'static str {
    match a {
        AvertissementFusion::EleveProtege => "eleve_protege",
        AvertissementFusion::DeclareeDistincte => "declaree_distincte",
        AvertissementFusion::IneDifferents => "ine_differents",
        AvertissementFusion::DeuxComptes => "deux_comptes",
        AvertissementFusion::AbsorbeEncoreScolarise => "absorbe_encore_scolarise",
    }
}

fn fusion_dto(f: &FusionEleves) -> dto::FusionDto {
    dto::FusionDto {
        id: f.id,
        absorbe_id: f.absorbe_id,
        conserve_id: f.conserve_id,
        motif_code: f.motif_code.clone(),
        motif: f.motif.clone(),
        conflits: f.conflits.iter().map(conflit_dto).collect(),
        lignes: f.resume.iter().map(ligne_dto).collect(),
        fusionne_le: f.fusionne_le,
        annule_le: f.annule_le,
        annulable_jusqu_au: f.annulable_jusqu_au,
    }
}

fn modifiee_dto(l: &LigneModifiee) -> dto::LigneModifieeDto {
    dto::LigneModifieeDto {
        domaine: l.domaine.clone(),
        table: l.table.clone(),
        cle: l.cle.clone(),
        raison: l.raison.to_string(),
    }
}

fn apercu_dto(a: &ApercuFusion) -> dto::ApercuFusionDto {
    dto::ApercuFusionDto {
        absorbe: comparaison(&a.absorbe),
        conserve: comparaison(&a.conserve),
        lignes: a.lignes.iter().map(ligne_dto).collect(),
        conflits: a.conflits.iter().map(conflit_dto).collect(),
        differences: a.differences.iter().map(|d| (*d).to_string()).collect(),
        avertissements: a.avertissements.iter().map(|w| avertissement_code(*w).to_string()).collect(),
        possible: a.possible,
    }
}

fn bilan_dto(b: &BilanDetection) -> dto::BilanDetectionDto {
    dto::BilanDetectionDto {
        paires: b.paires,
        nouvelles: b.nouvelles,
        devenues_obsoletes: b.devenues_obsoletes,
    }
}

/// ELV-22 — `GET /eleves/doublons`.
async fn lister_doublons(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<LimiteQuery>,
) -> Res<Json<Vec<dto::DoublonDto>>> {
    session.require_staff()?;
    let l = state
        .ent
        .doublons_pour_acteur(&session.role, session.etablissement_id, session.user_id, q.limite.unwrap_or(100))
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(doublon_dto).collect()))
}

/// ELV-22 — `POST /eleves/doublons/detecter` (relance la détection à la demande).
async fn detecter_doublons(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<dto::BilanDetectionDto>> {
    session.require_staff()?;
    let b = state
        .ent
        .detecter_doublons_pour_acteur(&session.role, session.etablissement_id, session.user_id)
        .await
        .map_err(map_db)?;
    Ok(Json(bilan_dto(&b)))
}

/// ELV-33 — `POST /eleves/doublons/non-doublon`.
async fn declarer_non_doublon(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<dto::NonDoublonBody>,
) -> Res<StatusCode> {
    session.require_staff()?;
    state
        .ent
        .declarer_non_doublon(
            &session.role,
            session.etablissement_id,
            session.user_id,
            b.eleve_a,
            b.eleve_b,
            &b.motif_code,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

/// ELV-23 — `POST /eleves/fusions/apercu`.
async fn apercu_fusion(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<dto::ApercuFusionBody>,
) -> Res<Json<dto::ApercuFusionDto>> {
    session.require_staff()?;
    let a = state
        .ent
        .apercu_fusion_pour_acteur(
            &session.role,
            session.etablissement_id,
            session.user_id,
            b.absorbe_id,
            b.conserve_id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(apercu_dto(&a)))
}

/// ELV-23 ★ — `POST /eleves/fusions`. `409` avec la liste des conflits tant qu'ils ne sont pas
/// tous acquittés (rien n'est écrit).
async fn fusionner(
    State(state): State<AppState>,
    session: EntSession,
    Json(b): Json<dto::FusionnerBody>,
) -> Res<(StatusCode, Json<dto::ResultatFusionDto>)> {
    session.require_staff()?;
    let r = state
        .ent
        .fusionner_eleves_pour_acteur(
            &session.role,
            DemandeFusion {
                etablissement_id: session.etablissement_id,
                absorbe_id: b.absorbe_id,
                conserve_id: b.conserve_id,
                motif_code: b.motif_code,
                motif: b.motif,
                conflits_acquittes: b.conflits_acquittes.into_iter().map(conflit_depuis).collect(),
                acteur: session.user_id,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(match r {
        ResultatFusion::Faite(f) => (
            StatusCode::CREATED,
            Json(dto::ResultatFusionDto { fusion: Some(fusion_dto(&f)), conflits_a_resoudre: Vec::new() }),
        ),
        ResultatFusion::ConflitsAResoudre(c) => (
            StatusCode::CONFLICT,
            Json(dto::ResultatFusionDto {
                fusion: None,
                conflits_a_resoudre: c.iter().map(conflit_dto).collect(),
            }),
        ),
    })
}

/// ELV-24 — `GET /eleves/fusions`.
async fn lister_fusions(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<LimiteQuery>,
) -> Res<Json<Vec<dto::FusionDto>>> {
    session.require_staff()?;
    let l = state
        .ent
        .fusions_pour_acteur(&session.role, session.etablissement_id, session.user_id, q.limite.unwrap_or(50))
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(fusion_dto).collect()))
}

/// ELV-24 ★ — `POST /eleves/fusions/{id}/annuler`. `409` avec les lignes modifiées depuis la
/// fusion (rien n'est restauré).
async fn annuler_fusion(
    State(state): State<AppState>,
    session: EntSession,
    Path(fusion_id): Path<Uuid>,
) -> Res<(StatusCode, Json<dto::ResultatDefusionDto>)> {
    session.require_staff()?;
    let r = state
        .ent
        .defusionner_eleves_pour_acteur(&session.role, session.etablissement_id, session.user_id, fusion_id)
        .await
        .map_err(map_db)?;
    Ok(match r {
        ResultatDefusion::Faite(f) => (
            StatusCode::OK,
            Json(dto::ResultatDefusionDto { fusion: Some(fusion_dto(&f)), lignes_modifiees: Vec::new() }),
        ),
        ResultatDefusion::LignesModifiees(l) => (
            StatusCode::CONFLICT,
            Json(dto::ResultatDefusionDto {
                fusion: None,
                lignes_modifiees: l.iter().map(modifiee_dto).collect(),
            }),
        ),
    })
}
