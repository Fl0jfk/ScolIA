//! Socle 2 (l'élève), lot E5 — routes des propositions et du paramétrage des champs.
//!
//! * ELV-18 / ELV-40 : `POST /eleves/{id}/propositions` — un responsable légal (lien actif) ou
//!   l'élève majeur ou émancipé propose une valeur ; le personnel signale une erreur vue sur le
//!   terrain. Dans tous les cas : **une proposition, jamais une modification directe**.
//! * ELV-19 : `GET /eleves/propositions`, `GET /eleves/propositions/{id}`,
//!   `POST /eleves/propositions/{id}/decision` (secrétariat) ; `POST …/retrait` (le déposant).
//! * ELV-26 : `GET /eleves/parametrage`, `PUT /eleves/parametrage/champs/{champ}`,
//!   `POST /eleves/parametrage/profil`, `PUT /eleves/parametrage/regles-auto` (administrateur).
//!
//! L'autorité est `ent-db` : chaque commande refait son contrôle (droit établissement, lien
//! actif, majorité), les routes ne font que traduire HTTP ↔ service. Une proposition d'un autre
//! établissement répond 404 ; le déposant ne voit que ses propres propositions.

use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use domain_eleve::{
    champ_catalogue, CibleProposition, ModificationLocale, NatureVersion, ProfilEtablissement,
    SaisieFamille, SourceFoi, SourceProposition,
};
use ent_db::param_eleve::{ChampParametre, ModifChamp, ParametrageEleve};
use ent_db::propositions::{
    DecisionProposition, DepotProposition, FiltrePropositions, PropositionEleve, ResultatDepot,
};
use ent_types::propositions as dto;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/eleves/propositions", get(lister_propositions))
        .route("/eleves/propositions/{proposition_id}", get(lire_proposition))
        .route(
            "/eleves/propositions/{proposition_id}/decision",
            post(decider_proposition),
        )
        .route(
            "/eleves/propositions/{proposition_id}/retrait",
            post(retirer_proposition),
        )
        .route(
            "/eleves/{id}/propositions",
            get(propositions_de_l_eleve).post(deposer_proposition),
        )
        .route("/eleves/{id}/propositions/champs", get(champs_signalables))
        .route("/eleves/parametrage", get(lire_parametrage))
        .route("/eleves/parametrage/champs/{champ}", put(modifier_champ))
        .route("/eleves/parametrage/profil", post(appliquer_profil))
        .route("/eleves/parametrage/regles-auto", put(definir_regle_auto))
}

fn bad(m: &str) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, m.to_string())
}

/// Un compte de la famille : responsable légal ou élève (le service contrôle le lien actif, la
/// majorité ou l'émancipation).
fn est_famille(session: &EntSession) -> bool {
    matches!(session.role.as_str(), "parent" | "eleve")
}

fn proposition_dto(p: &PropositionEleve, interne: bool) -> dto::PropositionDto {
    let e = &p.entete;
    dto::PropositionDto {
        id: e.id,
        eleve_id: e.eleve_id,
        cible: e.cible.clone(),
        source: e.source.clone(),
        origine: e.origine.clone().filter(|_| interne),
        depose_par: e.depose_par.filter(|_| interne),
        piece_document_id: e.piece_document_id,
        commentaire: e.commentaire.clone(),
        statut: e.statut.clone(),
        regle_auto: e.regle_auto.clone().filter(|_| interne),
        decision_code: e.decision_code.clone(),
        decision_motif: e.decision_motif.clone(),
        decide_le: e.decide_le,
        cree_le: e.cree_le,
        expire_le: e.expire_le,
        champs: p
            .champs
            .iter()
            .map(|c| dto::ChampPropositionDto {
                champ: c.champ.clone(),
                actuel: c.actuel.clone().filter(|_| interne),
                propose: c.propose.clone(),
                decision: c.decision.clone(),
            })
            .collect(),
    }
}

fn depot_dto(r: &ResultatDepot, interne: bool) -> dto::ResultatDepotDto {
    dto::ResultatDepotDto {
        en_attente: r.en_attente.as_ref().map(|p| proposition_dto(p, interne)),
        appliquee_auto: r.appliquee_auto.as_ref().map(|p| proposition_dto(p, interne)),
        champs_identiques: if interne { r.champs_identiques.clone() } else { Vec::new() },
        raisons_non_auto: if interne {
            r.raisons_non_auto
                .iter()
                .map(|(c, m)| dto::RaisonNonAutoDto { champ: c.clone(), raison: (*m).to_string() })
                .collect()
        } else {
            Vec::new()
        },
    }
}

fn cible_des_champs(champs: &BTreeMap<String, serde_json::Value>) -> Res<CibleProposition> {
    let premier = champs.keys().next().ok_or_else(|| bad("aucun champ proposé"))?;
    Ok(champ_catalogue(premier)
        .ok_or_else(|| bad("champ inconnu du catalogue"))?
        .cible)
}

/// ELV-18 (famille, élève majeur) ou ELV-40 (personnel) — `POST /eleves/{id}/propositions`.
async fn deposer_proposition(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<dto::DeposerPropositionBody>,
) -> Res<(StatusCode, Json<dto::ResultatDepotDto>)> {
    let cible = cible_des_champs(&body.champs)?;
    let cmd = DepotProposition {
        etablissement_id: session.etablissement_id,
        eleve_id: id,
        cible,
        // Fixées par le service selon le déposant : jamais par le client.
        source: SourceProposition::Famille,
        origine: None,
        source_ref: None,
        champs: body.champs.into_iter().collect(),
        piece_document_id: body.piece_document_id,
        commentaire: body.commentaire,
        expire_le: None,
        depose_par: None,
    };
    let famille = est_famille(&session);
    let r = if famille {
        state
            .ent
            .deposer_proposition_famille(&session.role, cmd, session.user_id)
            .await
    } else {
        session.require_staff()?;
        state
            .ent
            .deposer_signalement_personnel(&session.role, cmd, session.user_id)
            .await
    }
    .map_err(map_db)?;
    // Ni la famille ni un personnel qui ne traite pas les propositions ne voient la valeur de
    // la fiche (état civil, identifiants, versions sensibles : groupes G2, G3, G6) ni le détail
    // des règles internes : le dépôt ne donne aucun droit de lecture.
    let interne = !famille
        && state
            .ent
            .a_action_quelconque(
                session.etablissement_id,
                session.user_id,
                &session.role,
                ent_auth::Action::ElevesPropositionsTraiter,
            )
            .await
            .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(depot_dto(&r, interne))))
}

/// `GET /eleves/{id}/propositions` — le déposant : les siennes ; le secrétariat : celles de
/// l'élève.
async fn propositions_de_l_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<dto::PropositionDto>>> {
    if est_famille(&session) {
        let l = state
            .ent
            .mes_propositions(&session.role, session.etablissement_id, session.user_id, id)
            .await
            .map_err(map_db)?;
        return Ok(Json(l.iter().map(|p| proposition_dto(p, false)).collect()));
    }
    session.require_staff()?;
    let l = state
        .ent
        .propositions_pour_acteur(
            &session.role,
            session.etablissement_id,
            session.user_id,
            FiltrePropositions { eleve_id: Some(id), ..Default::default() },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(|p| proposition_dto(p, true)).collect()))
}

/// `GET /eleves/{id}/propositions/champs` — ce que l'établissement ouvre à cette famille.
async fn champs_signalables(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<dto::ChampSignalableDto>>> {
    let l = state
        .ent
        .champs_signalables(&session.role, session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?;
    Ok(Json(
        l.into_iter()
            .map(|c| dto::ChampSignalableDto {
                code: c.code.into(),
                libelle: c.libelle.into(),
                piece_requise: c.piece_requise,
                sensible: c.sensible,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
struct FiltreQuery {
    statut: Option<String>,
    eleve_id: Option<Uuid>,
    source: Option<String>,
    champ: Option<String>,
    limite: Option<i64>,
    avant: Option<DateTime<Utc>>,
}

/// ELV-19 — `GET /eleves/propositions` : la file du secrétariat.
async fn lister_propositions(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<FiltreQuery>,
) -> Res<Json<Vec<dto::PropositionDto>>> {
    session.require_staff()?;
    let l = state
        .ent
        .propositions_pour_acteur(
            &session.role,
            session.etablissement_id,
            session.user_id,
            FiltrePropositions {
                statut: q.statut,
                eleve_id: q.eleve_id,
                source: q.source,
                champ: q.champ,
                limite: q.limite,
                avant: q.avant,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(|p| proposition_dto(p, true)).collect()))
}

/// ELV-19 — `GET /eleves/propositions/{id}`.
async fn lire_proposition(
    State(state): State<AppState>,
    session: EntSession,
    Path(pid): Path<Uuid>,
) -> Res<Json<dto::PropositionDto>> {
    session.require_staff()?;
    let p = state
        .ent
        .proposition_pour_acteur(&session.role, session.etablissement_id, session.user_id, pid)
        .await
        .map_err(map_db)?;
    Ok(Json(proposition_dto(&p, true)))
}

/// ELV-19 — `POST /eleves/propositions/{id}/decision`.
async fn decider_proposition(
    State(state): State<AppState>,
    session: EntSession,
    Path(pid): Path<Uuid>,
    Json(body): Json<dto::DecisionPropositionBody>,
) -> Res<Json<dto::PropositionDto>> {
    session.require_staff()?;
    // Une personne choisit entre correction et changement d'état civil ; « import » est
    // réservé aux flux machine (déduit de la source, jamais choisi).
    let nature = match body.nature.as_deref() {
        None => None,
        Some(n) => match NatureVersion::parse(n) {
            Some(n @ (NatureVersion::Correction | NatureVersion::ChangementEtatCivil)) => Some(n),
            _ => return Err(bad("nature : correction ou changement_etat_civil")),
        },
    };
    let p = state
        .ent
        .decider_proposition(
            &session.role,
            DecisionProposition {
                proposition_id: pid,
                etablissement_id: session.etablissement_id,
                acteur: session.user_id,
                champs_acceptes: body.champs_acceptes,
                nature,
                motif_code: body.motif_code,
                motif: body.motif,
                piece_document_id: body.piece_document_id,
                effet_du: body.effet_du,
                refus_code: body.refus_code,
                refus_motif: body.refus_motif,
            },
        )
        .await
        .map_err(map_db)?;
    Ok(Json(proposition_dto(&p, true)))
}

/// `POST /eleves/propositions/{id}/retrait` — le déposant retire sa proposition en attente
/// (toute autre proposition répond 404, comme une proposition inexistante).
async fn retirer_proposition(
    State(state): State<AppState>,
    session: EntSession,
    Path(pid): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .retirer_proposition(session.etablissement_id, session.user_id, pid)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

// ───────────────────────── Paramétrage (ELV-26) ─────────────────────────

fn champ_param_dto(c: &ChampParametre) -> dto::ChampParametreDto {
    dto::ChampParametreDto {
        code: c.code.into(),
        libelle: c.libelle.into(),
        visible: c.reglage.visible,
        obligatoire_a: c.reglage.obligatoire_a.clone(),
        saisie_famille: c.reglage.saisie_famille.code().into(),
        source_qui_fait_foi: c.reglage.source_qui_fait_foi.code().into(),
        modification_locale: c.reglage.modification_locale.code().into(),
        verrouille: c.reglage.verrouille,
        regle: c.regle,
        regles_auto: c.regles_auto.clone(),
    }
}

fn parametrage_dto(p: &ParametrageEleve) -> dto::ParametrageEleveDto {
    dto::ParametrageEleveDto {
        profil: p.profil.clone(),
        profils_proposes: ProfilEtablissement::PROPOSES
            .iter()
            .map(|x| dto::ProfilPropose { code: x.code().into(), libelle: x.libelle().into() })
            .collect(),
        champs: p.champs.iter().map(champ_param_dto).collect(),
    }
}

/// ELV-26 — `GET /eleves/parametrage`.
async fn lire_parametrage(
    State(state): State<AppState>,
    session: EntSession,
) -> Res<Json<dto::ParametrageEleveDto>> {
    session.require_staff()?;
    let p = state
        .ent
        .parametrage_eleve(&session.role, session.etablissement_id, session.user_id)
        .await
        .map_err(map_db)?;
    Ok(Json(parametrage_dto(&p)))
}

/// ELV-26 — `PUT /eleves/parametrage/champs/{champ}`.
async fn modifier_champ(
    State(state): State<AppState>,
    session: EntSession,
    Path(champ): Path<String>,
    Json(body): Json<dto::ModifierChampBody>,
) -> Res<Json<dto::ChampParametreDto>> {
    session.require_staff()?;
    let m = ModifChamp {
        champ,
        visible: body.visible,
        obligatoire_a: body.obligatoire_a,
        saisie_famille: match body.saisie_famille.as_deref() {
            None => None,
            Some(s) => Some(SaisieFamille::parse(s).ok_or_else(|| bad("saisie_famille inconnue"))?),
        },
        source_qui_fait_foi: match body.source_qui_fait_foi.as_deref() {
            None => None,
            Some(s) => Some(SourceFoi::parse(s).ok_or_else(|| bad("source_qui_fait_foi inconnue"))?),
        },
        modification_locale: match body.modification_locale.as_deref() {
            None => None,
            Some(s) => {
                Some(ModificationLocale::parse(s).ok_or_else(|| bad("modification_locale inconnue"))?)
            }
        },
        verrouille: body.verrouille,
    };
    let c = state
        .ent
        .parametrer_champ(&session.role, session.etablissement_id, session.user_id, m)
        .await
        .map_err(map_db)?;
    Ok(Json(champ_param_dto(&c)))
}

/// ELV-26 — `POST /eleves/parametrage/profil` : applique un préréglage (remplace les réglages).
async fn appliquer_profil(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<dto::AppliquerProfilBody>,
) -> Res<Json<dto::ParametrageEleveDto>> {
    session.require_staff()?;
    let profil = ProfilEtablissement::parse(&body.profil).ok_or_else(|| bad("profil inconnu"))?;
    let p = state
        .ent
        .appliquer_profil(&session.role, session.etablissement_id, session.user_id, profil)
        .await
        .map_err(map_db)?;
    Ok(Json(parametrage_dto(&p)))
}

/// ELV-26 — `PUT /eleves/parametrage/regles-auto`.
async fn definir_regle_auto(
    State(state): State<AppState>,
    session: EntSession,
    Json(body): Json<dto::RegleAutoBody>,
) -> Res<StatusCode> {
    session.require_staff()?;
    let source = SourceProposition::parse(&body.source).ok_or_else(|| bad("source inconnue"))?;
    state
        .ent
        .definir_regle_auto(
            &session.role,
            session.etablissement_id,
            session.user_id,
            &body.champ,
            source,
            body.actif,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}
