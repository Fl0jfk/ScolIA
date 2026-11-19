//! Routes `/internat/*` du socle 41, lot I2 « La nuit » : attendu, nuits exceptionnelles, retraits,
//! listes, appels (ouverture, pointage, clôture, validation, correction, PDF), cas et procédure,
//! transmission du matin, effectif de nuit.
//!
//! L'établissement et l'acteur viennent de la session, jamais du corps. Aucune règle ici : `ent-db`
//! vérifie les droits (`internat.lire`, `internat.appel`, `internat.appel.valider`, `internat.gerer`),
//! la portée sur les élèves (R-IT30), l'attendu (R-IT14) et la clôture (R-IT16). L'heure de chaque
//! commande est celle du serveur. **Aucune route sans session.**

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{NaiveDate, Utc};
use ent_types::internat_nuit::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::{attachment_response, map_db};
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/internat/nuits/{nuit}/attendu", get(attendu))
        .route("/internat/nuits/{nuit}/effectif", get(effectif))
        .route("/internat/nuits/{nuit}/transmission", post(transmettre))
        .route("/internat/nuits-exceptionnelles", post(poser_nuit_exceptionnelle))
        .route("/internat/nuits-exceptionnelles/{id}/annulation", post(annuler_nuit_exceptionnelle))
        .route("/internat/retraits", post(poser_retrait))
        .route("/internat/retraits/{id}/annulation", post(annuler_retrait))
        .route("/internat/listes", get(listes).post(creer_liste))
        .route("/internat/listes/{id}", axum::routing::put(modifier_liste))
        .route("/internat/appels", get(appels).post(ouvrir_appel))
        .route("/internat/appels/{id}", get(vue_appel))
        .route("/internat/appels/{id}/historique", get(historique))
        .route("/internat/appels/{id}/pdf", get(pdf))
        .route("/internat/appels/{id}/pointages", post(pointer))
        .route("/internat/appels/{id}/tous-presents", post(tous_presents))
        .route("/internat/appels/{id}/cloture", post(cloturer))
        .route("/internat/appels/{id}/validation", post(valider))
        .route("/internat/cas", get(cas))
        .route("/internat/cas/{id}", get(cas_vue))
        .route("/internat/cas/{id}/etapes", post(cas_etape))
        .route("/internat/cas/{id}/issue", post(cas_issue))
        .route("/internat/procedure", get(procedure).put(publier_procedure))
        .route("/internat/lectures-sensibles", get(lectures))
}

/// Bornes raisonnables d'une date de nuit reçue d'un client : une date absurde (an 9999, an 0) ferait déborder
/// les calculs de dates de la base (500) ; on la refuse proprement (400).
fn nuit_bornee(nuit: NaiveDate) -> Res<NaiveDate> {
    let (min, max) = (
        NaiveDate::from_ymd_opt(2000, 1, 1).unwrap_or(NaiveDate::MIN),
        NaiveDate::from_ymd_opt(2100, 12, 31).unwrap_or(NaiveDate::MAX),
    );
    if nuit < min || nuit > max {
        return Err((
            StatusCode::BAD_REQUEST,
            "date de nuit hors des bornes raisonnables (2000 à 2100)".into(),
        ));
    }
    Ok(nuit)
}

#[derive(Debug, Deserialize)]
struct NuitQuery {
    nuit: NaiveDate,
}

#[derive(Debug, Deserialize)]
struct ListeQuery {
    #[serde(default = "vrai")]
    actif: bool,
}

fn vrai() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct LimiteQuery {
    limite: Option<i64>,
}

async fn attendu(State(s): State<AppState>, se: EntSession, Path(nuit): Path<NaiveDate>) -> Res<Json<AttenduDto>> {
    let nuit = nuit_bornee(nuit)?;
    Ok(Json(
        s.ent
            .attendu_nuit_a(se.etablissement_id, se.user_id, &se.role, nuit, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn effectif(State(s): State<AppState>, se: EntSession, Path(nuit): Path<NaiveDate>) -> Res<Json<EffectifRepasDto>> {
    let nuit = nuit_bornee(nuit)?;
    Ok(Json(
        s.ent
            .effectif_repas_a(se.etablissement_id, se.user_id, &se.role, nuit, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn transmettre(
    State(s): State<AppState>,
    se: EntSession,
    Path(nuit): Path<NaiveDate>,
    corps: Bytes,
) -> Res<Json<TransmissionDto>> {
    let nuit = nuit_bornee(nuit)?;
    // Corps facultatif : absent, vide, ou blanc (même avec `Content-Type: application/json`), c'est « sans motif » ;
    // `Option<Json<_>>` refuserait un corps vide quand l'en-tête JSON est présent.
    let motif = if corps.iter().all(u8::is_ascii_whitespace) {
        None
    } else {
        serde_json::from_slice::<TransmissionSaisieDto>(&corps)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("corps JSON invalide : {e}")))?
            .motif_rectification
    };
    Ok(Json(
        s.ent
            .transmettre_matin_a(se.etablissement_id, se.user_id, &se.role, nuit, motif.as_deref(), Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn poser_nuit_exceptionnelle(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<NuitExceptionnelleSaisieDto>,
) -> Res<(StatusCode, Json<NuitExceptionnelleDto>)> {
    let r = s
        .ent
        .poser_nuit_exceptionnelle_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn annuler_nuit_exceptionnelle(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    s.ent
        .annuler_nuit_exceptionnelle_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn poser_retrait(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<RetraitSaisieDto>,
) -> Res<(StatusCode, Json<RetraitDto>)> {
    let r = s
        .ent
        .poser_retrait_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn annuler_retrait(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    s.ent
        .annuler_retrait_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn listes(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<ListeDto>>> {
    Ok(Json(s.ent.lister_listes_a(se.etablissement_id, se.user_id, &se.role, Utc::now()).await.map_err(map_db)?))
}

async fn creer_liste(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<ListeSaisieDto>,
) -> Res<(StatusCode, Json<ListeDto>)> {
    let r = s
        .ent
        .creer_liste_a(se.etablissement_id, se.user_id, &se.role, &b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn modifier_liste(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<ListeQuery>,
    Json(b): Json<ListeSaisieDto>,
) -> Res<Json<ListeDto>> {
    Ok(Json(
        s.ent
            .modifier_liste_a(se.etablissement_id, se.user_id, &se.role, id, &b, q.actif)
            .await
            .map_err(map_db)?,
    ))
}

async fn appels(State(s): State<AppState>, se: EntSession, Query(q): Query<NuitQuery>) -> Res<Json<Vec<AppelDto>>> {
    Ok(Json(
        s.ent
            .lister_appels_a(se.etablissement_id, se.user_id, &se.role, nuit_bornee(q.nuit)?, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn ouvrir_appel(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<AppelOuvertureDto>,
) -> Res<(StatusCode, Json<AppelVueDto>)> {
    if let Some(n) = b.nuit {
        nuit_bornee(n)?;
    }
    let r = s
        .ent
        .ouvrir_appel_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn vue_appel(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<AppelVueDto>> {
    Ok(Json(
        s.ent
            .vue_appel_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn historique(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<HistoriqueFaitDto>>> {
    Ok(Json(
        s.ent
            .historique_appel_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn pdf(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Response> {
    let octets = s
        .ent
        .pdf_appel_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
        .await
        .map_err(map_db)?;
    attachment_response(octets, "appel-nuit.pdf", "application/pdf")
}

async fn pointer(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PointageSaisieDto>,
) -> Res<(StatusCode, Json<PointageResultatDto>)> {
    let r = s
        .ent
        .pointer_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn tous_presents(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ToutPresentSaisieDto>,
) -> Res<Json<Vec<PointageResultatDto>>> {
    Ok(Json(
        s.ent
            .tous_presents_chambre_a(se.etablissement_id, se.user_id, &se.role, id, b.chambre_id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn cloturer(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<ClotureResultatDto>> {
    Ok(Json(
        s.ent
            .cloturer_appel_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn valider(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<ValidationResultatDto>> {
    Ok(Json(
        s.ent
            .valider_appel_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn cas(State(s): State<AppState>, se: EntSession, Query(q): Query<NuitQuery>) -> Res<Json<Vec<CasDto>>> {
    Ok(Json(
        s.ent
            .lister_cas_a(se.etablissement_id, se.user_id, &se.role, nuit_bornee(q.nuit)?, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn cas_vue(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<CasVueDto>> {
    Ok(Json(
        s.ent
            .cas_vue_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn cas_etape(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CasEtapeSaisieDto>,
) -> Res<StatusCode> {
    s.ent
        .cas_etape_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok(StatusCode::CREATED)
}

async fn cas_issue(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CasIssueSaisieDto>,
) -> Res<Json<CasDto>> {
    Ok(Json(
        s.ent
            .cas_issue_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn procedure(State(s): State<AppState>, se: EntSession) -> Res<Json<ProcedureDto>> {
    Ok(Json(
        s.ent
            .lire_procedure_a(se.etablissement_id, se.user_id, &se.role, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn publier_procedure(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<ProcedureSaisieDto>,
) -> Res<Json<ProcedureDto>> {
    Ok(Json(
        s.ent
            .publier_procedure_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn lectures(State(s): State<AppState>, se: EntSession, Query(q): Query<LimiteQuery>) -> Res<Json<Vec<LectureSensibleDto>>> {
    Ok(Json(
        s.ent
            .lectures_sensibles_a(se.etablissement_id, se.user_id, &se.role, q.limite.unwrap_or(100), Utc::now())
            .await
            .map_err(map_db)?,
    ))
}
