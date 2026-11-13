//! Routes `/cahier/*` du lot S2-6 : cahier de textes, travail à faire / à rendre, copies,
//! agenda, charge de travail, visa (CDT-01 → CDT-10).
//!
//! Droits : moteur D-017 (`cahier.lire`, `cahier.ecrire`, `cahier.viser`) vérifiés par
//! `ent-db` à chaque appel, plus identité de l'élève / lien du responsable. L'établissement
//! vient de la session, jamais du corps. Fichiers : décodés ici, contrôlés (D2) par `ent-db`,
//! jamais servis directement : `POST /cahier/pieces/{id}/lien` émet un lien signé court.

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use base64::Engine;
use chrono::NaiveDate;
use ent_db::cahier::{Cible, FichierDepot, NouveauTravail, ParentPiece};
use ent_types::cahier::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    let up = ent_db::upload::PolitiqueFichiers::depuis_env();
    let corps_fichier = DefaultBodyLimit::max(
        up.limite_corps_base64(ent_db::upload::UsageFichier::Copie)
            .max(up.limite_corps_base64(ent_db::upload::UsageFichier::PieceCahier)),
    );
    Router::new()
        .route("/cahier/seances", get(seances))
        .route("/cahier/seances/{id}/contenu", put(renseigner_seance))
        .route("/cahier/mon-cahier", get(cahier_eleve))
        .route("/cahier/travaux", get(travaux_cible).post(donner_travail))
        .route(
            "/cahier/travaux/{id}",
            get(travail).put(modifier_travail).delete(supprimer_travail),
        )
        .route("/cahier/travaux/{id}/historique", get(historique))
        .route("/cahier/travaux/{id}/fait", put(marquer_fait))
        .route("/cahier/travaux/{id}/suivi", get(suivi))
        .route(
            "/cahier/travaux/{id}/pieces",
            post(piece_travail).layer(corps_fichier.clone()),
        )
        .route(
            "/cahier/contenus/{id}/pieces",
            post(piece_seance).layer(corps_fichier.clone()),
        )
        .route("/cahier/pieces/{id}/lien", post(lien_piece))
        .route("/cahier/pieces/{id}", axum::routing::delete(retirer_piece))
        .route("/cahier/travaux/{id}/retrait", post(retirer_travail))
        .route(
            "/cahier/travaux/{id}/remise",
            post(rendre).layer(corps_fichier.clone()),
        )
        .route(
            "/cahier/remises/{id}/retour",
            post(retour).layer(corps_fichier),
        )
        .route("/cahier/agenda", get(agenda))
        .route("/cahier/charge", get(charge))
        .route("/cahier/visas", get(visas).post(viser))
        .route("/cahier/reprise", get(reprise_etat))
}

fn decoder(f: FichierDepotBody) -> Res<FichierDepot> {
    let contenu = base64::engine::general_purpose::STANDARD
        .decode(f.contenu_base64.trim())
        .map_err(|_| (StatusCode::BAD_REQUEST, "contenu_base64 invalide".to_string()))?;
    Ok(FichierDepot {
        nom: f.nom,
        mime_type: f.mime_type,
        contenu,
    })
}

// ───── CDT-01 ─────

#[derive(Debug, Deserialize)]
struct SeancesQuery {
    classe_id: Option<Uuid>,
    du: NaiveDate,
    au: NaiveDate,
}

async fn seances(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<SeancesQuery>,
) -> Res<Json<Vec<SeanceCahierDto>>> {
    Ok(Json(
        s.ent
            .cahier_seances(&acteur(&session), q.classe_id, q.du, q.au)
            .await
            .map_err(map_db)?,
    ))
}

async fn renseigner_seance(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ContenuSeanceBody>,
) -> Res<Json<SeanceContenuDto>> {
    Ok(Json(
        s.ent
            .cahier_renseigner_seance(&acteur(&session), id, &b.contenu, &b.ressources)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct EleveQuery {
    eleve_id: Option<Uuid>,
    du: NaiveDate,
    au: NaiveDate,
}

async fn cahier_eleve(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<EleveQuery>,
) -> Res<Json<Vec<SeanceCahierDto>>> {
    Ok(Json(
        s.ent
            .cahier_seances_eleve(&acteur(&session), q.eleve_id, q.du, q.au)
            .await
            .map_err(map_db)?,
    ))
}

// ───── CDT-02 / 03 / 05 ─────

#[derive(Debug, Deserialize)]
struct CibleQuery {
    classe_id: Option<Uuid>,
    groupe_id: Option<Uuid>,
    du: NaiveDate,
    au: NaiveDate,
    seuil_minutes: Option<i32>,
    seuil_controles: Option<i32>,
}

async fn travaux_cible(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<CibleQuery>,
) -> Res<Json<Vec<TravailDto>>> {
    Ok(Json(
        s.ent
            .cahier_travaux_cible(
                &acteur(&session),
                Cible {
                    classe_id: q.classe_id,
                    groupe_id: q.groupe_id,
                },
                q.du,
                q.au,
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn donner_travail(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerTravailBody>,
) -> Res<(StatusCode, Json<TravailCreeDto>)> {
    let cible = match (b.classe_id, b.groupe_id) {
        (None, None) => None,
        (classe_id, groupe_id) => Some(Cible { classe_id, groupe_id }),
    };
    let n = NouveauTravail {
        seance_id: b.seance_id,
        cible,
        matiere_libelle: b.matiere_libelle,
        type_travail: b.type_travail.unwrap_or_else(|| "a_faire".to_string()),
        titre: b.titre,
        consigne: b.consigne,
        pour_le: b.pour_le,
        duree_estimee_min: b.duree_estimee_min,
        date_limite: b.date_limite,
        acceptation_retard: b.acceptation_retard.unwrap_or(true),
        formats_acceptes: b.formats_acceptes,
    };
    let cree = s
        .ent
        .cahier_donner_travail(&acteur(&session), n)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(cree)))
}

async fn travail(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<AgendaItemDto>> {
    Ok(Json(
        s.ent.cahier_travail(&acteur(&session), id).await.map_err(map_db)?,
    ))
}

async fn modifier_travail(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ModifierTravailBody>,
) -> Res<Json<TravailDto>> {
    Ok(Json(
        s.ent
            .cahier_modifier_travail(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

async fn supprimer_travail(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .cahier_supprimer_travail(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Serialize)]
struct VersionHistorique {
    version: i32,
    action: String,
    created_at: chrono::DateTime<chrono::Utc>,
    instantane: serde_json::Value,
}

async fn historique(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<VersionHistorique>>> {
    let v = s
        .ent
        .cahier_historique_travail(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok(Json(
        v.into_iter()
            .map(|(version, action, created_at, instantane)| VersionHistorique {
                version,
                action,
                created_at,
                instantane,
            })
            .collect(),
    ))
}

// ───── CDT-04 ─────

async fn piece_travail(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<FichierDepotBody>,
) -> Res<(StatusCode, Json<PieceCahierDto>)> {
    let f = decoder(b)?;
    let p = s
        .ent
        .cahier_joindre_piece(&acteur(&session), ParentPiece::Travail(id), f)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(p)))
}

async fn piece_seance(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<FichierDepotBody>,
) -> Res<(StatusCode, Json<PieceCahierDto>)> {
    let f = decoder(b)?;
    let p = s
        .ent
        .cahier_joindre_piece(&acteur(&session), ParentPiece::Seance(id), f)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(p)))
}

/// Lot C1 : état de la reprise de l'ancien cahier (direction).
async fn reprise_etat(State(s): State<AppState>, session: EntSession) -> Res<Json<RepriseEtatDto>> {
    Ok(Json(s.ent.cahier_reprise_etat(&acteur(&session)).await.map_err(map_db)?))
}

async fn retirer_piece(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .cahier_retirer_piece(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn retirer_travail(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RetraitBody>,
) -> Res<StatusCode> {
    s.ent
        .cahier_retirer_travail(&acteur(&session), id, &b.motif)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn lien_piece(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<LienCahierDto>> {
    let l = s
        .ent
        .cahier_lien_piece(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok(Json(LienCahierDto {
        url: l.url,
        expires_at: l.expires_at,
    }))
}

// ───── CDT-06 / 07 ─────

async fn rendre(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RemiseBody>,
) -> Res<(StatusCode, Json<RemiseDto>)> {
    let f = b.fichier.map(decoder).transpose()?;
    let r = s
        .ent
        .cahier_rendre(&acteur(&session), id, &b.commentaire, f)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn suivi(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<SuiviTravailDto>> {
    Ok(Json(
        s.ent
            .cahier_suivi_travail(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn retour(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RetourBody>,
) -> Res<Json<RemiseDto>> {
    let f = b.fichier.map(decoder).transpose()?;
    Ok(Json(
        s.ent
            .cahier_retour(&acteur(&session), id, &b.commentaire, b.note_id, f)
            .await
            .map_err(map_db)?,
    ))
}

// ───── CDT-08 / 09 ─────

async fn agenda(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<EleveQuery>,
) -> Res<Json<Vec<AgendaItemDto>>> {
    Ok(Json(
        s.ent
            .cahier_agenda(&acteur(&session), q.eleve_id, q.du, q.au)
            .await
            .map_err(map_db)?,
    ))
}

async fn marquer_fait(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<FaitBody>,
) -> Res<StatusCode> {
    s.ent
        .cahier_marquer_fait(&acteur(&session), id, b.fait)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

// ───── Charge et visa ─────

async fn charge(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<CibleQuery>,
) -> Res<Json<ChargeDto>> {
    Ok(Json(
        s.ent
            .cahier_charge(
                &acteur(&session),
                Cible {
                    classe_id: q.classe_id,
                    groupe_id: q.groupe_id,
                },
                q.du,
                q.au,
                q.seuil_minutes,
                q.seuil_controles,
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn viser(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<VisaBody>,
) -> Res<(StatusCode, Json<VisaDto>)> {
    let v = s.ent.cahier_viser(&acteur(&session), b).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(v)))
}

#[derive(Debug, Deserialize)]
struct VisasQuery {
    classe_id: Uuid,
}

async fn visas(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<VisasQuery>,
) -> Res<Json<Vec<VisaDto>>> {
    Ok(Json(
        s.ent
            .cahier_visas(&acteur(&session), q.classe_id)
            .await
            .map_err(map_db)?,
    ))
}
