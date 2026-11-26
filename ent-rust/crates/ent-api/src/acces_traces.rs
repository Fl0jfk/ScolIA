//! Routes `/acces/*` — **traces d'accès** (socle 27, lot G4-1 ; `27_RGPD_ARCHIVES.md` §11).
//!
//! * `GET /acces/consultations` : page « Consultations du dossier » de l'appelant (famille, élève majeur, agent) :
//!   fonction et service, jamais le nom (§11.3) ; rien d'autre que ses propres dossiers ;
//! * `GET /acces/motifs` : liste S-542 des motifs d'une consultation hors périmètre.
//!
//! L'ouverture d'une fiche hors périmètre (`GET /eleves/{id}`) répond 409 `motif_requis` avec cette liste (RGP-191).
//! L'établissement vient de la session.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use ent_types::acces_traces::*;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/acces/consultations", get(consultations))
        .route("/acces/motifs", get(motifs))
}

fn motif_dto(m: ent_db::acces_traces::MotifVue) -> MotifConsultationDto {
    MotifConsultationDto {
        code: m.code,
        libelle: m.libelle,
    }
}

/// Corps de la réponse 409 `motif_requis` (JSON : [`MotifRequisDto`]).
pub(crate) async fn motif_requis(state: &AppState, session: &EntSession) -> (StatusCode, String) {
    let motifs = state
        .ent
        .acces_motifs(session.etablissement_id)
        .await
        .map(|v| v.into_iter().map(motif_dto).collect())
        .unwrap_or_default();
    let corps = MotifRequisDto {
        code: ent_db::acces_traces::MOTIF_REQUIS.to_string(),
        motifs,
    };
    (
        StatusCode::CONFLICT,
        serde_json::to_string(&corps).unwrap_or_else(|_| "motif_requis".to_string()),
    )
}

async fn motifs(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<MotifConsultationDto>>> {
    let v = s
        .ent
        .acces_motifs(session.etablissement_id)
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(motif_dto).collect()))
}

async fn consultations(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<ConsultationsDossierDto>> {
    let c = s
        .ent
        .acces_consultations(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(ConsultationsDossierDto {
        depuis: c.depuis,
        dossiers: c
            .dossiers
            .into_iter()
            .map(|d| DossierConsulteDto {
                sujet_type: d.sujet_type,
                sujet_id: d.sujet_id,
                libelle: d.libelle,
                total: d.total,
                jours: d
                    .jours
                    .into_iter()
                    .map(|j| JourConsultationsDto {
                        jour: j.jour,
                        lignes: j
                            .lignes
                            .into_iter()
                            .map(|l| LigneConsultationDto {
                                le: l.le,
                                fonction: l.fonction,
                                service: l.service,
                                nature: l.nature,
                                section: l.section,
                                motif: l.motif,
                                mention: l.mention,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect(),
    }))
}
