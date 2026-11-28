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
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
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
        // Lot G3b-3 : remise, « Mes données », renvois, clôture consignée, pièces à choisir (Pf7).
        .route(
            "/rgpd/demandes/{id}/extraits/{extrait}/remise/defi",
            post(remise_defi),
        )
        .route(
            "/rgpd/demandes/{id}/extraits/{extrait}/remise",
            post(remettre),
        )
        .route(
            "/rgpd/demandes/{id}/renvois",
            get(renvois).post(tracer_renvoi),
        )
        .route("/rgpd/demandes/{id}/cloture", get(cloture))
        .route("/rgpd/demandes/{id}/pieces-identite", get(pieces_identite))
        .route("/rgpd/mes-donnees", get(mes_donnees))
        .route(
            "/rgpd/mes-donnees/extraits/{extrait}/defi",
            post(telechargement_defi),
        )
        .route(
            "/rgpd/mes-donnees/fichiers/{fichier}/telecharger",
            post(telecharger),
        )
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
        elements_reserves_direction: c.elements_reserves_direction,
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

/// F4 : cette lecture **écrit** (trace de consultation, journal, passage à « périmé »). Un GET à effet de bord ne
/// doit ni être mis en cache (`private, no-store`) ni partir d'une page tierce : une requête que le navigateur
/// déclare `Sec-Fetch-Site: cross-site` (lien ou image sur un autre site, que `SameSite=Lax` laisserait passer) est
/// refusée avant toute écriture. Les applications natives (jeton Bearer) n'envoient pas cet en-tête.
async fn contenu(
    State(s): State<AppState>,
    session: EntSession,
    entetes: axum::http::HeaderMap,
    Path((id, extrait)): Path<(Uuid, Uuid)>,
) -> Res<Response> {
    if entetes
        .get("sec-fetch-site")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("cross-site"))
    {
        return Err((
            StatusCode::FORBIDDEN,
            "lecture d'un extrait refusée depuis un autre site".into(),
        ));
    }
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
    let mut res = Json(contenu_dto(c)).into_response();
    res.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    Ok(res)
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
            b.rang,
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

// ─────────────── Remise, « Mes données », renvois (lot G3b-3) ───────────────

fn defi(d: ent_db::droits_remise::DefiRemise) -> DefiDto {
    DefiDto {
        defi_id: d.defi_id,
        options: d.options,
        intention: d.intention,
        empreinte: d.empreinte,
    }
}

fn fichier(f: ent_db::droits_remise::FichierVue) -> FichierRemisDto {
    FichierRemisDto {
        id: f.id,
        nom: f.nom,
        type_fichier: f.type_,
        octets: f.octets,
        empreinte: f.empreinte,
        disponible_jusqu_le: f.disponible_jusqu_le,
        purge: f.purge,
        telechargements: f.telechargements,
    }
}

fn remise(r: ent_db::droits_remise::RemiseVue, pdf: Option<Vec<u8>>) -> RemiseDto {
    RemiseDto {
        id: r.id,
        extrait_id: r.extrait_id,
        mode: r.mode,
        remis_le: r.remis_le,
        empreinte_extrait: r.empreinte_extrait,
        fichiers: r.fichiers.into_iter().map(fichier).collect(),
        pdf_base64: pdf.map(|p| ent_db::droits_remise::base64_encoder(&p)),
    }
}

fn credential(v: serde_json::Value) -> Res<ent_auth::PublicKeyCredential> {
    serde_json::from_value(v).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "assertion de passkey invalide".to_string(),
        )
    })
}

async fn remise_defi(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, extrait)): Path<(Uuid, Uuid)>,
) -> Res<Json<DefiDto>> {
    let d = s
        .ent
        .droits_remise_defi(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            extrait,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(defi(d)))
}

async fn remettre(
    State(s): State<AppState>,
    session: EntSession,
    Path((id, extrait)): Path<(Uuid, Uuid)>,
    Json(b): Json<RemettreBody>,
) -> Res<Json<RemiseDto>> {
    use ent_db::droits_remise::{Emargement, ModeRemise};
    let mode = match b.mode.as_str() {
        "en_ligne" => ModeRemise::EnLigne,
        "main_propre" => ModeRemise::MainPropre,
        _ => {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                "mode de remise inconnu".into(),
            ))
        }
    };
    let emargement = match (b.emargement_nom, b.emargement_piece_vue) {
        (Some(nom), Some(piece_vue)) => Some(Emargement { nom, piece_vue }),
        _ => None,
    };
    let cred = credential(b.credential)?;
    let f = s
        .ent
        .droits_remettre(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            extrait,
            mode,
            b.defi_id,
            &cred,
            emargement,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(remise(f.remise, f.pdf_a_imprimer)))
}

async fn mes_donnees(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<MonExtraitDto>>> {
    let v = s
        .ent
        .droits_mes_extraits(session.etablissement_id, session.user_id)
        .await
        .map_err(map_db)?;
    Ok(Json(
        v.into_iter()
            .map(|m| MonExtraitDto {
                demande_reference: m.demande_reference,
                remise: remise(m.remise, None),
            })
            .collect(),
    ))
}

async fn telechargement_defi(
    State(s): State<AppState>,
    session: EntSession,
    Path(extrait): Path<Uuid>,
) -> Res<Json<DefiDto>> {
    let d = s
        .ent
        .droits_telechargement_defi(
            session.etablissement_id,
            session.user_id,
            extrait,
            Utc::now(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(defi(d)))
}

/// Téléchargement : octets déchiffrés **au demandeur authentifié seulement**, en base64url dans un JSON que le
/// navigateur n'interprète jamais (la page enregistre le fichier) ; jamais mis en cache.
async fn telecharger(
    State(s): State<AppState>,
    session: EntSession,
    Path(fichier): Path<Uuid>,
    Json(b): Json<TelechargerBody>,
) -> Res<Response> {
    use base64::Engine;
    let cred = credential(b.credential)?;
    let f = s
        .ent
        .droits_telecharger(
            session.etablissement_id,
            session.user_id,
            fichier,
            b.defi_id,
            &cred,
            Utc::now(),
        )
        .await
        .map_err(map_db)?;
    let corps = FichierTelechargeDto {
        nom: f.nom,
        type_fichier: f.type_,
        contenu_base64url: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&f.octets),
    };
    let mut res = Json(corps).into_response();
    let h = res.headers_mut();
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    Ok(res)
}

fn renvoi(r: ent_db::droits_remise::RenvoiVue) -> RenvoiDto {
    RenvoiDto {
        id: r.id,
        nature: r.nature,
        cible: r.cible,
        texte: r.texte,
        cree_le: r.cree_le,
    }
}

async fn renvois(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<RenvoiDto>>> {
    let v = s
        .ent
        .droits_renvois(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?;
    Ok(Json(v.into_iter().map(renvoi).collect()))
}

async fn tracer_renvoi(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RenvoiBody>,
) -> Res<Json<RenvoiDto>> {
    let r = s
        .ent
        .droits_renvoi_tracer(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
            &b.nature,
            &b.cible,
            &b.texte,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(renvoi(r)))
}

async fn cloture(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ClotureDto>> {
    let c = s
        .ent
        .droits_cloture_lire(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(map_db)?
        .ok_or((
            StatusCode::NOT_FOUND,
            "aucune clôture consignée".to_string(),
        ))?;
    Ok(Json(ClotureDto {
        elements: c.0,
        refus: c.1,
    }))
}

async fn pieces_identite(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<PieceChoisissableDto>>> {
    let v = s
        .ent
        .droits_pieces_identite_a_choisir(
            session.etablissement_id,
            session.user_id,
            &session.role,
            id,
        )
        .await
        .map_err(map_db)?;
    Ok(Json(
        v.into_iter()
            .map(|p| PieceChoisissableDto {
                id: p.id,
                deposee_le: p.deposee_le,
                etat: p.etat,
                pour_cette_demande: p.pour_cette_demande,
            })
            .collect(),
    ))
}
