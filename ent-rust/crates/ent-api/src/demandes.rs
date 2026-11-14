//! Socle N1, lot N1-4 : routes JSON des **demandes** (`/demandes…`), traduction HTTP ↔ `ent-db`.
//!
//! * **Aucune route publique** (NJ-88) : toutes exigent une session (`EntSession`) ; le demandeur est la personne de la
//!   session, jamais un champ du corps (NJ-84).
//! * L'autorité est `ent-db` (garde d'accès unique : 404 neutre sans lien, NJ-80/NJ-81) ; la route ne décide d'aucun
//!   droit. Les erreurs sont des phrases fixes (NJ-93).
//! * Aucune route `GET` n'écrit, sauf l'ouverture d'une demande de copies prête par son demandeur (« vue »).
//! * Socle 7 absent : le dépôt de copies n'a pas de route (aucun port de documents réel) ; socle 15 absent : les heures
//!   vont à l'export de secours.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_db::demandes::{
    DemandeDemandeur, DemandeTraitant, DepotDemande, EtatDemande, Geste, RefusDemande, Valeur, ValeurSaisie,
};
use ent_db::demandes_hse::{DecisionHse, DepotHse, HeuresAbsentes, LigneHse, OptionsDecision, SeanceSaisie};
use ent_db::demandes_repro::LigneImpression;
use ent_types::demandes::{
    ChampDepot, DecisionHseDto, DemandeFile, DepotDemandeDto, DepotHseDto, EnveloppeHseDto, EtatDemandeDto, GesteDto,
    Guichet, LigneCopiesDto, LigneHseDto, MaDemande, NatureGuichet, PreteDto, RaccourciGuichet, ResultatDecisionHseDto,
    ValeurDepot, ValidationCopiesDto,
};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/demandes", post(deposer))
        .route("/demandes/guichet", get(guichet))
        .route("/demandes/mes", get(mes_demandes))
        .route("/demandes/file", get(ma_file))
        .route("/demandes/hse", post(deposer_hse))
        .route("/demandes/hse/mes", get(mes_hse))
        .route("/demandes/hse/a-decider", get(hse_a_decider))
        .route("/demandes/hse/enveloppe", get(enveloppe_hse))
        .route("/demandes/hse/{id}/confirmer", post(confirmer_hse))
        .route("/demandes/hse/{id}/decider", post(decider_hse))
        .route("/demandes/copies/file", get(file_copies))
        .route("/demandes/copies/mes", get(mes_copies))
        .route("/demandes/copies/a-valider", get(copies_a_valider))
        .route("/demandes/copies/{id}/valider", post(valider_copies))
        .route("/demandes/copies/{id}/impression", post(impression_copies))
        .route("/demandes/copies/{id}/prete", post(prete_copies))
        .route("/demandes/{id}", get(detail))
        .route("/demandes/{id}/prendre", post(prendre))
        .route("/demandes/{id}/annuler", post(annuler))
}

type Echec = (StatusCode, String);

/// Phrase fixe et code HTTP du refus : rien d'autre ne sort (NJ-93).
fn echec(e: RefusDemande) -> Echec {
    (StatusCode::from_u16(e.statut_http()).unwrap_or(StatusCode::SERVICE_UNAVAILABLE), e.message_public().to_string())
}

fn etat(e: EtatDemande) -> EtatDemandeDto {
    EtatDemandeDto { id: e.id, numero: e.numero, statut: e.statut, version: e.version }
}

fn ma_demande(d: DemandeDemandeur) -> MaDemande {
    MaDemande {
        id: d.id,
        numero: d.numero,
        nature_code: d.nature_code,
        statut: d.statut,
        issue: d.issue,
        objet: d.objet,
        depose_le: d.depose_le,
        clos_le: d.clos_le,
        version: d.version,
    }
}

fn demande_file(d: DemandeTraitant) -> DemandeFile {
    DemandeFile {
        id: d.id,
        numero: d.numero,
        nature_code: d.nature_code,
        statut: d.statut,
        confidentialite: d.confidentialite,
        urgence: d.urgence,
        objet: d.objet,
        depose_le: d.depose_le,
        prise: d.attribuee_a_agent_id.is_some(),
        version: d.version,
    }
}

fn valeur(v: ValeurDepot) -> Valeur {
    match v {
        ValeurDepot::Texte(t) => Valeur::Texte(t),
        ValeurDepot::Entier(n) => Valeur::Entier(n),
        ValeurDepot::Decimal(d) => Valeur::Decimal(d),
        ValeurDepot::Date(d) => Valeur::Date(d),
        ValeurDepot::Heure(h) => Valeur::Heure(h),
        ValeurDepot::Bool(b) => Valeur::Bool(b),
        ValeurDepot::Code(c) => Valeur::Code(c),
        ValeurDepot::Ref(r) => Valeur::Ref(r),
    }
}

fn champ(c: ChampDepot) -> ValeurSaisie {
    ValeurSaisie { champ: c.champ, rang: c.rang, valeur: valeur(c.valeur) }
}

/// Formulaire dédié d'une nature : le guichet y renvoie au lieu du formulaire générique.
fn formulaire_de(code: &str) -> Option<String> {
    match code {
        "rh.hse" => Some("/web/demandes/hse".into()),
        "reprographie.tirage" => Some("/web/demandes/copies".into()),
        _ => None,
    }
}

async fn guichet(State(s): State<AppState>, session: EntSession) -> Result<Json<Guichet>, Echec> {
    let c = s.ent.catalogue_for_actor(session.etablissement_id, session.user_id, &session.role).await.map_err(echec)?;
    Ok(Json(Guichet {
        natures: c
            .natures
            .into_iter()
            .map(|n| NatureGuichet {
                formulaire: formulaire_de(n.code),
                code: n.code.to_string(),
                libelle: n.libelle.to_string(),
                mots_cles: n.mots_cles.iter().map(|m| (*m).to_string()).collect(),
                eleve_requis: n.eleve_requis,
                role_traitant: n.role_traitant,
                message_accueil: n.message_accueil,
                delai_traitement_jours: n.delai_traitement_jours,
            })
            .collect(),
        raccourcis: c
            .raccourcis
            .into_iter()
            .map(|r| RaccourciGuichet { code: r.code.to_string(), libelle: r.libelle.to_string(), lien: r.lien.to_string() })
            .collect(),
        signalement_grave: c.signalement_grave.to_string(),
        numeros_urgence: c.numeros_urgence.iter().map(|n| (*n).to_string()).collect(),
    }))
}

async fn deposer(
    State(s): State<AppState>,
    session: EntSession,
    Json(d): Json<DepotDemandeDto>,
) -> Result<(StatusCode, Json<EtatDemandeDto>), Echec> {
    // Le demandeur est la personne de la session : le corps n'a aucun champ d'identité (NJ-84).
    let depot = DepotDemande {
        nature_code: d.nature_code,
        objet: d.objet,
        texte: d.texte,
        valeurs: d.valeurs.into_iter().map(champ).collect(),
        urgence: None,
        eleve_id: d.eleve_id,
        pour_le_compte_de: None,
        canal: None,
        volet: None,
    };
    let sources = s
        .ent
        .sources_demandes_for_actor(session.etablissement_id, session.user_id, &session.role, Some(&depot))
        .await
        .map_err(echec)?;
    let e = s
        .ent
        .deposer_demande_for_actor(session.etablissement_id, session.user_id, &session.role, &sources, depot)
        .await
        .map_err(echec)?;
    Ok((StatusCode::CREATED, Json(etat(e))))
}

async fn mes_demandes(State(s): State<AppState>, session: EntSession) -> Result<Json<Vec<MaDemande>>, Echec> {
    let l = s.ent.mes_demandes_for_actor(session.etablissement_id, session.user_id).await.map_err(echec)?;
    Ok(Json(l.into_iter().map(ma_demande).collect()))
}

async fn ma_file(State(s): State<AppState>, session: EntSession) -> Result<Json<Vec<DemandeFile>>, Echec> {
    let sources = s
        .ent
        .sources_demandes_for_actor(session.etablissement_id, session.user_id, &session.role, None)
        .await
        .map_err(echec)?;
    let l = s
        .ent
        .ma_file_for_actor(session.etablissement_id, session.user_id, &session.role, &sources)
        .await
        .map_err(echec)?;
    Ok(Json(l.into_iter().map(demande_file).collect()))
}

async fn detail(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Result<Json<MaDemande>, Echec> {
    let d = s
        .ent
        .demande_demandeur_for_actor(session.etablissement_id, session.user_id, id)
        .await
        .map_err(echec)?;
    Ok(Json(ma_demande(d)))
}

async fn geste(s: &AppState, session: &EntSession, id: Uuid, v: i32, g: Geste) -> Result<Json<EtatDemandeDto>, Echec> {
    let sources = s
        .ent
        .sources_demandes_for_actor(session.etablissement_id, session.user_id, &session.role, None)
        .await
        .map_err(echec)?;
    let e = s
        .ent
        .agir_sur_demande_for_actor(session.etablissement_id, session.user_id, &session.role, &sources, id, v, g)
        .await
        .map_err(echec)?;
    Ok(Json(etat(e)))
}

async fn prendre(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(g): Json<GesteDto>,
) -> Result<Json<EtatDemandeDto>, Echec> {
    geste(&s, &session, id, g.version, Geste::Prendre).await
}

async fn annuler(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(g): Json<GesteDto>,
) -> Result<Json<EtatDemandeDto>, Echec> {
    geste(&s, &session, id, g.version, Geste::Annuler).await
}

// ───────────────────────── HSE ─────────────────────────

fn ligne_hse(l: LigneHse) -> LigneHseDto {
    LigneHseDto {
        demande_id: l.demande_id,
        numero: l.numero,
        statut: l.statut,
        motif: l.motif,
        volume_demande_quarts: l.volume_demande_quarts,
        volume_accorde_quarts: l.volume_accorde_quarts,
        fait_quarts: l.fait_quarts,
        transmis_quarts: l.transmis_quarts,
        decision: l.decision,
        periode_au: l.periode_au,
    }
}

async fn deposer_hse(
    State(s): State<AppState>,
    session: EntSession,
    Json(d): Json<DepotHseDto>,
) -> Result<(StatusCode, Json<EtatDemandeDto>), Echec> {
    let depot = DepotHse {
        motif: d.motif,
        objet: d.objet,
        precisions: d.precisions,
        volume_quarts: d.volume_quarts,
        periode_du: d.periode_du,
        periode_au: d.periode_au,
        lieu: d.lieu,
        classes: vec![],
        groupes: vec![],
        seances: d
            .seances
            .into_iter()
            .map(|x| SeanceSaisie { jour: x.jour, debut: x.debut, duree_quarts: x.duree_quarts, fait: x.fait, lieu: None })
            .collect(),
        pour_le_compte_de: d.pour_le_compte_de,
    };
    let routage = depot.vers_depot_demande().map_err(echec)?;
    let sources = s
        .ent
        .sources_demandes_for_actor(session.etablissement_id, session.user_id, &session.role, Some(&routage))
        .await
        .map_err(echec)?;
    let e = s
        .ent
        .deposer_hse_for_actor(session.etablissement_id, session.user_id, &session.role, &sources, depot)
        .await
        .map_err(echec)?;
    Ok((StatusCode::CREATED, Json(etat(e))))
}

async fn mes_hse(State(s): State<AppState>, session: EntSession) -> Result<Json<Vec<LigneHseDto>>, Echec> {
    let l = s.ent.mes_hse_for_actor(session.etablissement_id, session.user_id).await.map_err(echec)?;
    Ok(Json(l.into_iter().map(ligne_hse).collect()))
}

async fn hse_a_decider(State(s): State<AppState>, session: EntSession) -> Result<Json<Vec<LigneHseDto>>, Echec> {
    let sources = s
        .ent
        .sources_demandes_for_actor(session.etablissement_id, session.user_id, &session.role, None)
        .await
        .map_err(echec)?;
    let l = s
        .ent
        .hse_a_decider_for_actor(session.etablissement_id, session.user_id, &session.role, &sources)
        .await
        .map_err(echec)?;
    Ok(Json(l.into_iter().map(ligne_hse).collect()))
}

async fn enveloppe_hse(State(s): State<AppState>, session: EntSession) -> Result<Json<EnveloppeHseDto>, Echec> {
    let e = s
        .ent
        .enveloppe_hse_for_actor(session.etablissement_id, session.user_id, &session.role, None)
        .await
        .map_err(echec)?;
    Ok(Json(EnveloppeHseDto {
        saisie: e.saisie,
        total_quarts: e.total_quarts,
        engage_quarts: e.engage_quarts,
        fait_quarts: e.fait_quarts,
        restant_quarts: e.restant_quarts,
    }))
}

async fn confirmer_hse(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, Echec> {
    let sources = s
        .ent
        .sources_demandes_for_actor(session.etablissement_id, session.user_id, &session.role, None)
        .await
        .map_err(echec)?;
    s.ent
        .confirmer_hse_for_actor(session.etablissement_id, session.user_id, &session.role, &sources, id)
        .await
        .map_err(echec)?;
    Ok(Json(serde_json::json!({})))
}

async fn decider_hse(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(d): Json<DecisionHseDto>,
) -> Result<Json<ResultatDecisionHseDto>, Echec> {
    let decision = match (d.decision.as_str(), d.volume_quarts, d.motif.clone()) {
        ("accordee", _, _) => DecisionHse::Accorder,
        ("accordee_partiellement", Some(v), Some(m)) => DecisionHse::AccorderEnPartie { volume_quarts: v, motif: m },
        ("refusee", _, Some(m)) => DecisionHse::Refuser { motif: m },
        _ => return Err(echec(RefusDemande::Invalide("La décision est incomplète."))),
    };
    let sources = s
        .ent
        .sources_demandes_for_actor(session.etablissement_id, session.user_id, &session.role, None)
        .await
        .map_err(echec)?;
    let r = s
        .ent
        .decider_hse_for_actor(
            session.etablissement_id,
            session.user_id,
            &session.role,
            &sources,
            &HeuresAbsentes,
            id,
            d.version,
            decision,
            OptionsDecision { confirmer_depassement: d.confirmer_depassement, motif_depassement: d.motif_depassement },
        )
        .await
        .map_err(echec)?;
    Ok(Json(ResultatDecisionHseDto {
        decision: r.decision,
        volume_accorde_quarts: r.volume_accorde_quarts,
        seances_transmises: r.seances_transmises,
        depassement_enveloppe: r.depassement_enveloppe,
    }))
}

// ───────────────────────── Reprographie ─────────────────────────

fn ligne_copies(l: LigneImpression) -> LigneCopiesDto {
    LigneCopiesDto {
        demande_id: l.demande_id,
        numero: l.numero,
        site_id: l.site_id,
        etat: l.etat,
        format: l.format,
        couleur: l.couleur,
        recto_verso: l.recto_verso,
        agrafage: l.agrafage,
        copies_total: l.copies_total,
        exemplaires_total: l.exemplaires_total,
        pour_le: l.pour_le,
        avertissement_delai: l.avertissement_delai,
        demandeur_nom: l.demandeur_nom,
        retrait_lieu: l.retrait_lieu,
    }
}

async fn file_copies(State(s): State<AppState>, session: EntSession) -> Result<Json<Vec<LigneCopiesDto>>, Echec> {
    let l = s.ent.file_impression_for_actor(session.etablissement_id, session.user_id, &session.role).await.map_err(echec)?;
    Ok(Json(l.into_iter().map(ligne_copies).collect()))
}

async fn mes_copies(State(s): State<AppState>, session: EntSession) -> Result<Json<Vec<LigneCopiesDto>>, Echec> {
    let l = s.ent.mes_repro_for_actor(session.etablissement_id, session.user_id).await.map_err(echec)?;
    Ok(Json(l.into_iter().map(ligne_copies).collect()))
}

async fn copies_a_valider(State(s): State<AppState>, session: EntSession) -> Result<Json<Vec<LigneCopiesDto>>, Echec> {
    let l = s.ent.repro_a_valider_for_actor(session.etablissement_id, session.user_id, &session.role).await.map_err(echec)?;
    Ok(Json(l.into_iter().map(ligne_copies).collect()))
}

async fn valider_copies(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(v): Json<ValidationCopiesDto>,
) -> Result<Json<EtatDemandeDto>, Echec> {
    let e = s
        .ent
        .valider_repro_for_actor(session.etablissement_id, session.user_id, &session.role, id, v.accepter, v.note.as_deref())
        .await
        .map_err(echec)?;
    Ok(Json(etat(e)))
}

async fn impression_copies(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Result<Json<EtatDemandeDto>, Echec> {
    let e = s
        .ent
        .mettre_en_impression_for_actor(session.etablissement_id, session.user_id, &session.role, id)
        .await
        .map_err(echec)?;
    Ok(Json(etat(e)))
}

/// « Prête » : un geste de **session** (NJ-92). Il n'existe aucune route `GET` ni lien signé qui l'accomplisse.
async fn prete_copies(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(p): Json<PreteDto>,
) -> Result<Json<EtatDemandeDto>, Echec> {
    let e = s
        .ent
        .marquer_prete_for_actor(session.etablissement_id, session.user_id, &session.role, id, p.lieu_retrait)
        .await
        .map_err(echec)?;
    Ok(Json(etat(e)))
}
