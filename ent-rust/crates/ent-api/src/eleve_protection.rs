//! Socle 2 (l'élève), lot E7 — routes de la confidentialité renforcée, des photos, des coordonnées
//! propres de l'élève et du journal de consultation.
//!
//! * ELV-15 : `GET /eleves/confidentialite` (élèves protégés, G6), `GET|POST|PUT
//!   /eleves/{id}/confidentialite`, `POST /eleves/{id}/confidentialite/lever`.
//! * Journal : `GET /eleves/{id}/consultations` (`audit.lire`).
//! * ELV-16 / ELV-42 : `GET|POST /eleves/{id}/photos`, `GET /eleves/photos/a-valider`,
//!   `GET /eleves/photos/manquantes`, `POST /eleves/photos/{photo}/valider|rejeter|retirer`,
//!   `POST|DELETE /eleves/{id}/photo/refus`.
//! * ELV-17 : `GET|POST /eleves/{id}/coordonnees`, `POST /eleves/coordonnees/{coord}/decision|
//!   retirer|usages|verification|verifier`.
//!
//! L'autorité est `ent-db` : chaque commande refait son contrôle (droit, élève protégé, conflit
//! d'intérêts, lien de la famille, majorité) ; les routes ne font que traduire HTTP ↔ service. Une
//! cible absente ou d'un autre établissement répond `404`, jamais un succès.

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use ent_db::eleve_confidentialite::{ConsultationVue, MesureConfidentialiteVue, PoserConfidentialite};
use ent_db::eleve_coordonnee::{AjouterCoordonneeEleve, CoordonneeEleveVue};
use ent_db::eleve_photo::{PhotoVue, ProposerPhoto};
use ent_types::eleve_protection as dto;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    // Les photos portent leur contenu en base64 : même plafond de corps que la route héritée.
    let up = ent_db::upload::PolitiqueFichiers::depuis_env();
    let plafond_photo = DefaultBodyLimit::max(up.limite_corps_base64(ent_db::upload::UsageFichier::Photo));
    Router::new()
        .route("/eleves/confidentialite", get(lister_proteges))
        .route(
            "/eleves/{id}/confidentialite",
            get(lire_mesures).post(poser_mesure).put(modifier_mesure),
        )
        .route("/eleves/{id}/confidentialite/lever", post(lever_mesure))
        .route("/eleves/{id}/consultations", get(consultations))
        .route("/eleves/{id}/photos", get(photos_eleve).post(deposer_photo).layer(plafond_photo))
        .route("/eleves/photos/a-valider", get(photos_a_valider))
        .route("/eleves/photos/manquantes", get(photos_manquantes))
        .route("/eleves/photos/{photo}/valider", post(valider_photo))
        .route("/eleves/photos/{photo}/rejeter", post(rejeter_photo))
        .route("/eleves/photos/{photo}/retirer", post(retirer_photo))
        .route("/eleves/{id}/photo/refus", post(enregistrer_refus).delete(lever_refus))
        .route("/eleves/{id}/coordonnees", get(coordonnees_eleve).post(ajouter_coordonnee))
        .route("/eleves/coordonnees/{coord}/decision", post(decider_coordonnee))
        .route("/eleves/coordonnees/{coord}/retirer", post(retirer_coordonnee))
        .route("/eleves/coordonnees/{coord}/usages", post(usages_coordonnee))
        .route("/eleves/coordonnees/{coord}/verification", post(demander_verification))
        .route("/eleves/coordonnees/{coord}/verifier", post(verifier_coordonnee))
}

#[derive(Debug, Deserialize)]
struct LimiteQuery {
    limite: Option<i64>,
}

fn mesure_dto(m: &MesureConfidentialiteVue) -> dto::MesureConfidentialiteDto {
    dto::MesureConfidentialiteDto {
        id: m.id,
        eleve_id: m.eleve_id,
        mesures: m.mesures.clone(),
        du: m.du,
        au: m.au,
        decide_par: m.decide_par,
        decide_le: m.decide_le,
        leve_par: m.leve_par,
        leve_le: m.leve_le,
        leve_motif_code: m.leve_motif_code.clone(),
        motif: m.motif.clone(),
    }
}

fn consultation_dto(c: &ConsultationVue) -> dto::ConsultationDto {
    dto::ConsultationDto {
        id: c.id,
        eleve_id: c.eleve_id,
        acteur: c.acteur,
        action: c.action.clone(),
        libelle: c.libelle.clone(),
        quand: c.quand,
    }
}

fn photo_dto(p: &PhotoVue) -> dto::PhotoDto {
    dto::PhotoDto {
        id: p.id,
        eleve_id: p.eleve_id,
        source: p.source.clone(),
        statut: p.statut.clone(),
        mime: p.mime.clone(),
        taille_octets: p.taille_octets,
        prise_le: p.prise_le,
        propose_par: p.propose_par,
        propose_le: p.propose_le,
        decide_par: p.decide_par,
        decide_le: p.decide_le,
        motif_code: p.motif_code.clone(),
    }
}

fn coordonnee_dto(c: &CoordonneeEleveVue) -> dto::CoordonneeEleveDto {
    dto::CoordonneeEleveDto {
        id: c.id,
        eleve_id: c.eleve_id,
        type_: c.type_.clone(),
        valeur: c.valeur.clone(),
        usages: c.usages.clone(),
        source: c.source.clone(),
        statut: c.statut.clone(),
        verifiee_le: c.verifiee_le,
        du: c.du,
        au: c.au,
        cree_le: c.cree_le,
        decide_le: c.decide_le,
        motif_code: c.motif_code.clone(),
    }
}

fn pose(session: &EntSession, eleve: Uuid, b: dto::PoserConfidentialiteBody) -> PoserConfidentialite {
    PoserConfidentialite {
        etablissement_id: session.etablissement_id,
        eleve_id: eleve,
        motif: b.motif,
        mesures: b.mesures,
        du: b.du,
        au: b.au,
        acteur: session.user_id,
    }
}

// ───────────────────────────── ELV-15 ─────────────────────────────

/// `GET /eleves/confidentialite` — élèves actuellement protégés (G6 partout).
async fn lister_proteges(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<LimiteQuery>,
) -> Res<Json<Vec<dto::MesureConfidentialiteDto>>> {
    session.require_staff()?;
    let l = state
        .ent
        .eleves_proteges_pour_acteur(&session.role, session.etablissement_id, session.user_id, q.limite.unwrap_or(200))
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(mesure_dto).collect()))
}

/// `GET /eleves/{id}/confidentialite` — mesures de l'élève, motif descellé (G6 ; `404` sinon).
async fn lire_mesures(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<dto::MesureConfidentialiteDto>>> {
    session.require_staff()?;
    let l = state
        .ent
        .mesures_confidentialite_pour_acteur(&session.role, session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(mesure_dto).collect()))
}

/// `POST /eleves/{id}/confidentialite` ★ — pose la mesure (le motif n'est pas renvoyé).
async fn poser_mesure(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<dto::PoserConfidentialiteBody>,
) -> Res<(StatusCode, Json<dto::MesureConfidentialiteDto>)> {
    session.require_staff()?;
    let m = state.ent.poser_confidentialite(&session.role, pose(&session, id, b)).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(mesure_dto(&m))))
}

/// `PUT /eleves/{id}/confidentialite` ★ — remplace la mesure en vigueur.
async fn modifier_mesure(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<dto::PoserConfidentialiteBody>,
) -> Res<Json<dto::MesureConfidentialiteDto>> {
    session.require_staff()?;
    let m = state.ent.modifier_confidentialite(&session.role, pose(&session, id, b)).await.map_err(map_db)?;
    Ok(Json(mesure_dto(&m)))
}

/// `POST /eleves/{id}/confidentialite/lever` ★.
async fn lever_mesure(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<dto::LeverConfidentialiteBody>,
) -> Res<Json<dto::LeveeResultatDto>> {
    session.require_staff()?;
    let n = state
        .ent
        .lever_confidentialite(&session.role, session.etablissement_id, session.user_id, id, &b.motif_code)
        .await
        .map_err(map_db)?;
    Ok(Json(dto::LeveeResultatDto { levees: n }))
}

/// `GET /eleves/{id}/consultations` — journal de consultation (`audit.lire`).
async fn consultations(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<LimiteQuery>,
) -> Res<Json<Vec<dto::ConsultationDto>>> {
    session.require_staff()?;
    let l = state
        .ent
        .consultations_pour_acteur(&session.role, session.etablissement_id, session.user_id, id, q.limite.unwrap_or(100))
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(consultation_dto).collect()))
}

// ───────────────────────────── ELV-16, ELV-42 ─────────────────────────────

/// `GET /eleves/{id}/photos` — historique des photos (`eleves.photo.gerer`).
async fn photos_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<dto::PhotoDto>>> {
    session.require_staff()?;
    let l = state
        .ent
        .photos_eleve_pour_acteur(&session.role, session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(photo_dto).collect()))
}

/// `POST /eleves/{id}/photos` — dépose une photo (personnel, famille liée, élève lui-même).
async fn deposer_photo(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<dto::DeposerPhotoBody>,
) -> Res<(StatusCode, Json<dto::PhotoDto>)> {
    let octets = base64::engine::general_purpose::STANDARD
        .decode(b.contenu_base64.trim())
        .map_err(|_| (StatusCode::BAD_REQUEST, "contenu_base64 invalide".into()))?;
    let p = state
        .ent
        .proposer_photo(
            &session.role,
            ProposerPhoto {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                acteur: session.user_id,
                content_type: b.mime_type,
                octets,
                prise_le: b.prise_le,
                valider_aussitot: b.valider_aussitot,
                photographe: b.photographe,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(photo_dto(&p))))
}

/// `GET /eleves/photos/a-valider` — file des photos proposées.
async fn photos_a_valider(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<LimiteQuery>,
) -> Res<Json<Vec<dto::PhotoDto>>> {
    session.require_staff()?;
    let l = state
        .ent
        .photos_a_valider_pour_acteur(&session.role, session.etablissement_id, session.user_id, q.limite.unwrap_or(100))
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(photo_dto).collect()))
}

/// `GET /eleves/photos/manquantes` — élèves scolarisés sans photo ni refus.
async fn photos_manquantes(
    State(state): State<AppState>,
    session: EntSession,
    Query(q): Query<LimiteQuery>,
) -> Res<Json<dto::ElevesDto>> {
    session.require_staff()?;
    let eleves = state
        .ent
        .eleves_sans_photo_pour_acteur(&session.role, session.etablissement_id, session.user_id, q.limite.unwrap_or(500))
        .await
        .map_err(map_db)?;
    Ok(Json(dto::ElevesDto { eleves }))
}

/// `POST /eleves/photos/{photo}/valider`.
async fn valider_photo(
    State(state): State<AppState>,
    session: EntSession,
    Path(photo): Path<Uuid>,
) -> Res<Json<dto::PhotoDto>> {
    session.require_staff()?;
    let p = state
        .ent
        .valider_photo(&session.role, session.etablissement_id, session.user_id, photo)
        .await
        .map_err(map_db)?;
    Ok(Json(photo_dto(&p)))
}

/// `POST /eleves/photos/{photo}/rejeter` (motif codé).
async fn rejeter_photo(
    State(state): State<AppState>,
    session: EntSession,
    Path(photo): Path<Uuid>,
    Json(b): Json<dto::MotifBody>,
) -> Res<Json<dto::PhotoDto>> {
    session.require_staff()?;
    let p = state
        .ent
        .rejeter_photo(&session.role, session.etablissement_id, session.user_id, photo, &b.motif_code)
        .await
        .map_err(map_db)?;
    Ok(Json(photo_dto(&p)))
}

/// `POST /eleves/photos/{photo}/retirer` (motif codé).
async fn retirer_photo(
    State(state): State<AppState>,
    session: EntSession,
    Path(photo): Path<Uuid>,
    Json(b): Json<dto::MotifBody>,
) -> Res<Json<dto::PhotoDto>> {
    session.require_staff()?;
    let p = state
        .ent
        .retirer_photo(&session.role, session.etablissement_id, session.user_id, photo, &b.motif_code)
        .await
        .map_err(map_db)?;
    Ok(Json(photo_dto(&p)))
}

/// `POST /eleves/{id}/photo/refus` — refus de la photo administrative (famille, élève, secrétariat).
async fn enregistrer_refus(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .enregistrer_refus_photo(&session.role, session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /eleves/{id}/photo/refus` — lève le refus.
async fn lever_refus(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    state
        .ent
        .lever_refus_photo(&session.role, session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

// ───────────────────────────── ELV-17 ─────────────────────────────

/// `GET /eleves/{id}/coordonnees`.
async fn coordonnees_eleve(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<dto::CoordonneeEleveDto>>> {
    let l = state
        .ent
        .coordonnees_eleve_pour_acteur(&session.role, session.etablissement_id, session.user_id, id)
        .await
        .map_err(map_db)?;
    Ok(Json(l.iter().map(coordonnee_dto).collect()))
}

/// `POST /eleves/{id}/coordonnees` — ajoute (personnel, élève) ou propose (famille).
async fn ajouter_coordonnee(
    State(state): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<dto::AjouterCoordonneeBody>,
) -> Res<(StatusCode, Json<dto::CoordonneeEleveDto>)> {
    let c = state
        .ent
        .ajouter_coordonnee_eleve(
            &session.role,
            AjouterCoordonneeEleve {
                etablissement_id: session.etablissement_id,
                eleve_id: id,
                acteur: session.user_id,
                type_: b.type_,
                valeur: b.valeur,
                usages: b.usages,
            },
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(coordonnee_dto(&c))))
}

/// `POST /eleves/coordonnees/{coord}/decision` — accepter ou refuser une proposition.
async fn decider_coordonnee(
    State(state): State<AppState>,
    session: EntSession,
    Path(coord): Path<Uuid>,
    Json(b): Json<dto::DecisionCoordonneeBody>,
) -> Res<Json<dto::CoordonneeEleveDto>> {
    session.require_staff()?;
    let c = state
        .ent
        .decider_coordonnee_eleve(
            &session.role,
            session.etablissement_id,
            session.user_id,
            coord,
            b.accepter,
            b.motif_code.as_deref(),
        )
        .await
        .map_err(map_db)?;
    Ok(Json(coordonnee_dto(&c)))
}

/// `POST /eleves/coordonnees/{coord}/retirer` (motif codé).
async fn retirer_coordonnee(
    State(state): State<AppState>,
    session: EntSession,
    Path(coord): Path<Uuid>,
    Json(b): Json<dto::MotifBody>,
) -> Res<Json<dto::CoordonneeEleveDto>> {
    let c = state
        .ent
        .retirer_coordonnee_eleve(&session.role, session.etablissement_id, session.user_id, coord, &b.motif_code)
        .await
        .map_err(map_db)?;
    Ok(Json(coordonnee_dto(&c)))
}

/// `POST /eleves/coordonnees/{coord}/usages`.
async fn usages_coordonnee(
    State(state): State<AppState>,
    session: EntSession,
    Path(coord): Path<Uuid>,
    Json(b): Json<dto::UsagesBody>,
) -> Res<Json<dto::CoordonneeEleveDto>> {
    let c = state
        .ent
        .modifier_usages_coordonnee_eleve(&session.role, session.etablissement_id, session.user_id, coord, &b.usages)
        .await
        .map_err(map_db)?;
    Ok(Json(coordonnee_dto(&c)))
}

/// `POST /eleves/coordonnees/{coord}/verification` — envoie un code (jamais renvoyé).
async fn demander_verification(
    State(state): State<AppState>,
    session: EntSession,
    Path(coord): Path<Uuid>,
) -> Res<Json<dto::VerificationEnvoyeeDto>> {
    let v = state
        .ent
        .demander_verification_coordonnee_eleve(&session.role, session.etablissement_id, session.user_id, coord)
        .await
        .map_err(map_db)?;
    Ok(Json(dto::VerificationEnvoyeeDto {
        coordonnee_id: v.coordonnee_id,
        canal: v.canal.to_string(),
        expire_le: v.expire_le,
    }))
}

/// `POST /eleves/coordonnees/{coord}/verifier` — confirme le code (erreur unique).
async fn verifier_coordonnee(
    State(state): State<AppState>,
    session: EntSession,
    Path(coord): Path<Uuid>,
    Json(b): Json<dto::CodeBody>,
) -> Res<Json<dto::CoordonneeEleveDto>> {
    let c = state
        .ent
        .verifier_coordonnee_eleve(&session.role, session.etablissement_id, session.user_id, coord, &b.code)
        .await
        .map_err(map_db)?;
    Ok(Json(coordonnee_dto(&c)))
}
