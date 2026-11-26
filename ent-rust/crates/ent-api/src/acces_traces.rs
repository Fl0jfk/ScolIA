//! Routes `/acces/*` — **traces d'accès** (socle 27, lot G4-1 ; `27_RGPD_ARCHIVES.md` §11).
//!
//! * `GET /acces/consultations` : page « Consultations du dossier » de l'appelant (famille, élève majeur, agent) :
//!   fonction et service, jamais le nom (§11.3) ; rien d'autre que ses propres dossiers ;
//! * `GET /acces/motifs` : liste S-542 des motifs d'une consultation hors périmètre.
//!
//! L'ouverture d'une fiche hors périmètre (`GET /eleves/{id}`) répond 409 `motif_requis` avec cette liste (RGP-191).
//! L'établissement vient de la session.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use chrono::Utc;
use uuid::Uuid;
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
        .route("/acces/reglages", get(reglages).post(modifier_reglage))
        .route("/acces/controles", get(controles))
        .route("/acces/controles/defi", post(controle_defi))
        .route("/acces/controles/ouvrir", post(controle_ouvrir))
        .route("/acces/controles/{id}/traces", get(controle_traces))
        .route("/acces/controles/{id}/clore", post(controle_clore))
        .route("/acces/alertes", get(alertes))
        .route("/acces/alertes/{id}/suite", post(alerte_suite))
        .route("/acces/traces/demandes/{id}", get(traces_demande))
        .route("/acces/differer", post(differer))
        .route("/acces/demandes/{id}/nom-agent", post(nom_agent))
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

// ─────────────── Lot G4-2 : réglages, contrôles, alertes, traces d'un sujet ───────────────

fn lignes_reglage(v: Vec<ent_db::acces_controle::LigneReglage>) -> Vec<LigneReglageDto> {
    v.into_iter()
        .map(|l| LigneReglageDto {
            code: l.code.to_string(),
            libelle: l.libelle.to_string(),
            valeur: l.valeur,
            defaut: l.defaut,
            min: l.min,
            max: l.max,
        })
        .collect()
}

async fn reglages(State(s): State<AppState>, session: EntSession) -> Res<Json<ReglagesAccesDto>> {
    let (e, u, r) = (session.etablissement_id, session.user_id, &session.role);
    let lignes = s.ent.acces_reglages_lire(e, u, r).await.map_err(map_db)?;
    let historique = s
        .ent
        .acces_reglages_historique(e, u, r)
        .await
        .map_err(map_db)?;
    Ok(Json(ReglagesAccesDto {
        lignes: lignes_reglage(lignes),
        historique: historique
            .into_iter()
            .map(|h| ChangementReglageDto {
                reglage: h.reglage,
                ancienne: h.ancienne,
                nouvelle: h.nouvelle,
                le: h.le,
            })
            .collect(),
    }))
}

async fn modifier_reglage(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ReglageAccesBody>,
) -> Res<Json<ReglagesAccesDto>> {
    let (e, u, r) = (session.etablissement_id, session.user_id, &session.role);
    // Les deux réglages de l'extrait (lot G3b) passent par la même route : un seul écran de réglages.
    if b.reglage == "plafond_depots_publics_jour" || b.reglage == "extrait_disponible_jours" {
        s.ent
            .droits_reglage_modifier(e, u, r, &b.reglage, b.valeur)
            .await
            .map_err(map_db)?;
    } else {
        s.ent
            .acces_reglage_modifier(e, u, r, &b.reglage, b.valeur)
            .await
            .map_err(map_db)?;
    }
    reglages(State(s), session).await
}

fn controle_dto(c: ent_db::acces_controle::ControleVue) -> ControleDto {
    ControleDto {
        id: c.id,
        agent_compte_id: c.agent_compte_id,
        agent_nom: c.agent_nom,
        motif_code: c.motif_code,
        motif_texte: c.motif_texte,
        ouvert_le: c.ouvert_le,
        echeance_le: c.echeance_le,
        clos_le: c.clos_le,
        issue: c.issue,
        ouvert: c.ouvert,
    }
}

fn trace_dto(l: ent_db::acces_controle::TraceLigne) -> TraceLigneDto {
    TraceLigneDto {
        id: l.id,
        le: l.le,
        fonction: l.fonction,
        service: l.service,
        nature: l.nature,
        section: l.section,
        hors_perimetre: l.hors_perimetre,
        perimetre_temporaire: l.perimetre_temporaire,
        motif: l.motif,
        sujet: l.sujet,
        differee: l.differee,
        nom_communique: l.nom_communique,
        agent_nom: l.agent_nom,
    }
}

async fn controles(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<ControleDto>>> {
    let v = s
        .ent
        .acces_controles_a(
            session.etablissement_id,
            session.user_id,
            &session.role,
            Utc::now(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(controle_dto).collect()))
}

async fn controle_defi(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ControleDemandeBody>,
) -> Res<Json<ControleDefiDto>> {
    let d = s
        .ent
        .acces_controle_defi_a(
            session.etablissement_id,
            session.user_id,
            &session.role,
            b.agent_compte_id,
            &b.motif_code,
            &b.motif_texte,
            b.jours,
            Utc::now(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(ControleDefiDto {
        defi_id: d.defi_id,
        options: d.options,
        intention: d.intention,
    }))
}

async fn controle_ouvrir(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ControleOuvrirBody>,
) -> Res<Json<ControleDto>> {
    let cred: ent_auth::PublicKeyCredential = serde_json::from_value(b.credential).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "assertion de passkey invalide".to_string(),
        )
    })?;
    let d = b.demande;
    let c = s
        .ent
        .acces_controle_ouvrir_a(
            session.etablissement_id,
            session.user_id,
            &session.role,
            d.agent_compte_id,
            &d.motif_code,
            &d.motif_texte,
            d.jours,
            b.defi_id,
            &cred,
            Utc::now(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(controle_dto(c)))
}

async fn controle_traces(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<TracesControleDto>> {
    let (c, l) = s
        .ent
        .acces_traces_controle_a(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            Utc::now(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(TracesControleDto {
        controle: controle_dto(c),
        lignes: l.into_iter().map(trace_dto).collect(),
    }))
}

async fn controle_clore(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ControleCloreBody>,
) -> Res<Json<ControleDto>> {
    let c = s
        .ent
        .acces_controle_clore_a(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.issue,
            Utc::now(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(controle_dto(c)))
}

async fn alertes(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<AlerteAccesDto>>> {
    let v = s
        .ent
        .acces_alertes_lister(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(
        v.into_iter()
            .map(|a| AlerteAccesDto {
                id: a.id,
                regle: a.regle,
                libelle: a.libelle,
                agent_compte_id: a.agent_compte_id,
                agent_nom: a.agent_nom,
                fenetre_debut: a.fenetre_debut,
                fenetre_fin: a.fenetre_fin,
                nombre: a.nombre,
                cree_le: a.cree_le,
                suite: a.suite,
                suite_le: a.suite_le,
                controle_id: a.controle_id,
            })
            .collect(),
    ))
}

async fn alerte_suite(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AlerteSuiteBody>,
) -> Res<StatusCode> {
    s.ent
        .acces_alerte_suite_a(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.suite,
            Utc::now(),
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn traces_demande(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<TracesSujetDto>>> {
    let v = s
        .ent
        .acces_traces_demande_a(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            Utc::now(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        v.into_iter()
            .map(|t| TracesSujetDto {
                sujet_type: t.sujet_type,
                sujet_id: t.sujet_id,
                libelle: t.libelle,
                lignes: t.lignes.into_iter().map(trace_dto).collect(),
            })
            .collect(),
    ))
}

async fn differer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DifferBody>,
) -> Res<StatusCode> {
    s.ent
        .acces_differer_a(
            session.etablissement_id,
            session.user_id,
            &session.role,
            b.trace_id,
            &b.motif,
            b.jours,
            Utc::now(),
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn nom_agent(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<NomAgentBody>,
) -> Res<StatusCode> {
    s.ent
        .acces_nom_agent_arbitrer_a(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            b.trace_id,
            &b.motif,
            Utc::now(),
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}
