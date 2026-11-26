//! Routes `/rgpd/demandes/{id}/extraits/*` et `/rgpd/reglages` — **extrait d'accès** (socle 27, lot G3b ;
//! `27_RGPD_ARCHIVES.md` §9.4, §12.2).
//!
//! * `GET …/extraits` (structure), `POST …/extraits` (assemblage), `POST …/sujets` (fratrie, RGP-302) ;
//! * `GET …/extraits/{extrait}/contenu` : lecture du contenu, **tracée avant d'être servie** (R-RG8) ;
//! * `POST …/extraits/{extrait}/incomplet` (arbitrage R-RG18), `…/dpo` (soumission), `…/avis` (avis du DPO) ;
//! * `GET …/extraits/{extrait}/masquages`, `POST …/masquages` (ajout), `POST …/masquages/{id}` (décision) ;
//! * `GET|POST /rgpd/reglages`.
//!
//! L'établissement vient de la session. Les refus sont neutres : 403 sans le droit, 404 pour un objet
//! d'un autre établissement ou une raison de marquage qu'on n'a pas à connaître.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_types::rgpd_extrait::*;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/rgpd/demandes/{id}/extraits",
            get(extraits).post(assembler),
        )
        .route("/rgpd/demandes/{id}/sujets", post(ajouter_sujet))
        .route(
            "/rgpd/demandes/{id}/extraits/{extrait}/contenu",
            get(contenu),
        )
        .route(
            "/rgpd/demandes/{id}/extraits/{extrait}/incomplet",
            post(arbitrer_incomplet),
        )
        .route(
            "/rgpd/demandes/{id}/extraits/{extrait}/dpo",
            post(soumettre_dpo),
        )
        .route(
            "/rgpd/demandes/{id}/extraits/{extrait}/avis",
            post(avis_dpo),
        )
        .route(
            "/rgpd/demandes/{id}/extraits/{extrait}/masquages",
            get(masquages).post(ajouter_masquage),
        )
        .route(
            "/rgpd/demandes/{id}/masquages/{masquage}",
            post(decider_masquage),
        )
        .route("/rgpd/reglages", get(reglages).post(modifier_reglage))
}

fn section(s: ent_db::droits_extrait::SectionVue) -> SectionExtraitDto {
    SectionExtraitDto {
        fournisseur: s.fournisseur,
        socle: s.socle,
        statut: s.statut,
        motif_code: s.motif_code,
        lignes: s.lignes,
        remise_incomplete_motif: s.remise_incomplete_motif,
    }
}

fn extrait(v: ent_db::droits_extrait::ExtraitVue) -> ExtraitDto {
    ExtraitDto {
        id: v.id,
        demande_id: v.demande_id,
        sujet_type: v.sujet_type,
        sujet_id: v.sujet_id,
        version: v.version,
        assemble_le: v.assemble_le,
        complet: v.complet,
        perime: v.perime,
        perime_motif: v.perime_motif,
        nb_sections: v.nb_sections,
        nb_lignes: v.nb_lignes,
        empreinte: v.empreinte,
        sections: v.sections.into_iter().map(section).collect(),
        soumis_dpo: v.soumis_dpo,
        avis_dpo_code: v.avis_dpo_code,
        avis_dpo_texte: v.avis_dpo_texte,
        remissible_en_l_etat: v.remissible_en_l_etat,
        masquages_a_decider: v.masquages_a_decider,
    }
}

fn marquage(m: ent_db::droits_extrait_masquage::MarquageVue) -> MarquageDto {
    MarquageDto {
        id: m.id,
        fournisseur: m.fournisseur,
        table: m.table,
        ligne_id: m.ligne_id,
        rang: m.rang,
        colonne: m.colonne,
        raison: m.raison,
        propose_par: m.propose_par,
        decision: m.decision,
        motif: m.motif,
        decide_le: m.decide_le,
        levee_reservee_direction: m.levee_reservee_direction,
    }
}

fn contenu_dto(c: ent_db::droits_extrait::ContenuExtrait) -> ContenuExtraitDto {
    ContenuExtraitDto {
        sujet_type: c.sujet_type,
        sujet_id: c.sujet_id,
        periode_du: c.periode_du,
        periode_au: c.periode_au,
        sections: c
            .sections
            .into_iter()
            .map(|s| SectionContenuDto {
                fournisseur: s.fournisseur,
                socle: s.socle,
                statut: s.statut.code().to_string(),
                motif_code: s.motif_code,
                tables: s
                    .tables
                    .into_iter()
                    .map(|t| TableExtraitDto {
                        nom: t.nom,
                        lignes: t
                            .lignes
                            .into_iter()
                            .map(|l| LigneExtraitDto {
                                id: l.id,
                                valeurs: l.valeurs,
                            })
                            .collect(),
                    })
                    .collect(),
                exclusions: s
                    .exclusions
                    .into_iter()
                    .map(|e| ExclusionDto {
                        table: e.table,
                        motif: e.motif,
                    })
                    .collect(),
                lignes: s.lignes,
            })
            .collect(),
        complet: c.complet,
        empreinte: c.empreinte,
        nb_lignes: c.nb_lignes,
        destinataires_internes: c.destinataires_internes,
        destinataires_externes: c.destinataires_externes,
        soustraitants: c.soustraitants,
        marquages: c.marquages.into_iter().map(marquage).collect(),
    }
}

async fn extraits(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<ExtraitDto>>> {
    let v = s
        .ent
        .droits_extraits(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(extrait).collect()))
}

async fn assembler(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AssemblerBody>,
) -> Res<Json<Vec<ExtraitDto>>> {
    let sujet = match (b.sujet_type.as_deref(), b.sujet_id) {
        (Some(t), Some(i)) => Some((t, i)),
        (None, None) => None,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "sujet_type et sujet_id vont ensemble".into(),
            ))
        }
    };
    let v = s
        .ent
        .droits_extrait_assembler(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            sujet,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(extrait).collect()))
}

async fn ajouter_sujet(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SujetBody>,
) -> Res<StatusCode> {
    s.ent
        .droits_sujet_ajouter(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.sujet_type,
            b.sujet_id,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn contenu(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, extrait)): Path<(Uuid, Uuid)>,
) -> Res<Json<ContenuExtraitDto>> {
    let c = s
        .ent
        .droits_extrait_lire(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            extrait,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(contenu_dto(c)))
}

async fn arbitrer_incomplet(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, extrait)): Path<(Uuid, Uuid)>,
    Json(b): Json<ArbitrageIncompletBody>,
) -> Res<Json<ExtraitDto>> {
    let d: Vec<(&str, &str)> = b
        .decisions
        .iter()
        .map(|x| (x.fournisseur.as_str(), x.motif.as_str()))
        .collect();
    let v = s
        .ent
        .droits_extrait_arbitrer_incomplet(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            extrait,
            &d,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(extrait_dto(v)))
}

fn extrait_dto(v: ent_db::droits_extrait::ExtraitVue) -> ExtraitDto {
    extrait(v)
}

async fn soumettre_dpo(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, extrait)): Path<(Uuid, Uuid)>,
) -> Res<Json<ExtraitDto>> {
    let v = s
        .ent
        .droits_extrait_soumettre_dpo(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            extrait,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(extrait_dto(v)))
}

async fn avis_dpo(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, extrait)): Path<(Uuid, Uuid)>,
    Json(b): Json<AvisDpoBody>,
) -> Res<Json<ExtraitDto>> {
    let v = s
        .ent
        .droits_extrait_avis_dpo(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            extrait,
            &b.avis,
            &b.texte,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(extrait_dto(v)))
}

async fn masquages(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, extrait)): Path<(Uuid, Uuid)>,
) -> Res<Json<Vec<MarquageDto>>> {
    let v = s
        .ent
        .droits_masquages(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            extrait,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(marquage).collect()))
}

async fn ajouter_masquage(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, extrait)): Path<(Uuid, Uuid)>,
    Json(b): Json<AjoutMasquageBody>,
) -> Res<Json<Vec<MarquageDto>>> {
    let v = s
        .ent
        .droits_masquage_ajouter(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            extrait,
            &b.fournisseur,
            &b.table,
            b.ligne_id,
            b.colonne.as_deref(),
            &b.motif,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(marquage).collect()))
}

async fn decider_masquage(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, masquage_id)): Path<(Uuid, Uuid)>,
    Json(b): Json<DecisionMasquageBody>,
) -> Res<Json<Vec<MarquageDto>>> {
    let v = s
        .ent
        .droits_masquage_decider(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            masquage_id,
            &b.decision,
            b.motif.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(marquage).collect()))
}

async fn reglages(State(s): State<AppState>, session: EntSession) -> Res<Json<ReglageRgpdDto>> {
    let r = s
        .ent
        .droits_reglage_lire(session.etablissement_id, session.user_id, &session.role)
        .await
        .map_err(map_db)?;
    Ok(Json(ReglageRgpdDto {
        plafond_depots_publics_jour: r.plafond_depots_publics_jour,
        extrait_disponible_jours: r.extrait_disponible_jours,
    }))
}

async fn modifier_reglage(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ReglageRgpdBody>,
) -> Res<Json<ReglageRgpdDto>> {
    let r = s
        .ent
        .droits_reglage_modifier(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &b.reglage,
            b.valeur,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(ReglageRgpdDto {
        plafond_depots_publics_jour: r.plafond_depots_publics_jour,
        extrait_disponible_jours: r.extrait_disponible_jours,
    }))
}
