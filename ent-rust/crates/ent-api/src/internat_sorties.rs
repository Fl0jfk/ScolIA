//! Routes `/internat/*` du socle 41, lot I3 « Absences et sorties » : annonces d'absence des familles et décisions du
//! CPE, sorties typées, correspondants, départs du vendredi, contrôle de départ et de retour (remise), autorisations
//! annuelles du socle 18, désaccords entre titulaires.
//!
//! L'établissement et l'acteur viennent de la session, jamais du corps. Aucune règle ici : `ent-db` décide (qualité de
//! titulaire, majorité, désaccords, droits `internat.*`, remise). Les familles et les élèves passent par ces mêmes
//! routes avec leur propre session ; une famille ne voit jamais qui, parmi les autres titulaires, s'oppose.
//! L'heure de chaque commande est celle du serveur. **Aucune route sans session** (R-IT39).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::{NaiveDate, Utc};
use ent_types::internat_sorties::*;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/internat/sortie-types", get(types_sortie))
        .route("/internat/sortie-types/{code}", put(modifier_type))
        .route("/internat/absences", post(annoncer_absence))
        .route("/internat/absences/a-traiter", get(absences_a_traiter))
        .route("/internat/absences/{id}/decision", post(decider_absence))
        .route("/internat/absences/{id}/annulation", post(annuler_absence))
        .route("/internat/absences/{id}/opposition", post(opposer_absence))
        .route("/internat/absences/{id}/arbitrage", post(trancher_absence))
        .route("/internat/eleves/{id}/absences", get(absences_eleve))
        .route("/internat/sorties", post(demander_sortie))
        .route("/internat/sorties/a-traiter", get(sorties_a_traiter))
        .route("/internat/sorties/{id}/reponse", post(repondre_sortie))
        .route("/internat/sorties/{id}/decision", post(decider_sortie))
        .route("/internat/sorties/{id}/annulation", post(annuler_sortie))
        .route("/internat/sorties/{id}/opposition", post(opposer_sortie))
        .route("/internat/sorties/{id}/arbitrage", post(trancher_sortie))
        .route("/internat/sorties/{id}/personne", post(designer_personne))
        .route("/internat/sorties/{id}/retard", post(annoncer_retard))
        .route("/internat/sorties/{id}/depart", post(controler_depart))
        .route("/internat/sorties/{id}/retour", post(controler_retour))
        .route("/internat/eleves/{id}/sorties", get(sorties_eleve))
        .route("/internat/departs/{jour}/preparation", post(preparer_departs))
        .route("/internat/departs/{jour}/remise", get(liste_remise))
        .route("/internat/nuits/{nuit}/sorties-du-soir", get(sorties_du_soir))
        .route("/internat/correspondants", post(designer_correspondant))
        .route("/internat/correspondants/{id}/suspension", post(suspendre_correspondant))
        .route("/internat/correspondants/{id}/reprise", post(reprendre_correspondant))
        .route("/internat/correspondants/{id}/fin", post(terminer_correspondant))
        .route("/internat/eleves/{id}/correspondants", get(correspondants_eleve))
        .route("/internat/absences/{id}/historique", get(historique_absence))
        .route("/internat/sorties/{id}/historique", get(historique_sortie))
        .route("/internat/correspondants/{id}/historique", get(historique_correspondant))
        .route("/internat/eleves/{id}/autorisations", get(autorisations_eleve))
        .route("/internat/autorisations/demande", post(demander_autorisations))
        .route("/internat/autorisations/revocation", post(revoquer_autorisation))
}

/// Bornes raisonnables d'une date reçue d'un client : une date absurde (an 9999, an 0) ferait déborder les calculs de
/// dates de la base (500) ; on la refuse proprement (400).
fn date_bornee(d: NaiveDate) -> Res<NaiveDate> {
    let (min, max) = (
        NaiveDate::from_ymd_opt(2000, 1, 1).unwrap_or(NaiveDate::MIN),
        NaiveDate::from_ymd_opt(2100, 12, 31).unwrap_or(NaiveDate::MAX),
    );
    if d < min || d > max {
        return Err((StatusCode::BAD_REQUEST, "date hors des bornes raisonnables (2000 à 2100)".into()));
    }
    Ok(d)
}

// ───────────────────────── Types de sortie ─────────────────────────

async fn types_sortie(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<SortieTypeDto>>> {
    Ok(Json(s.ent.lister_types_sortie_a(se.etablissement_id).await.map_err(map_db)?))
}

async fn modifier_type(
    State(s): State<AppState>,
    se: EntSession,
    Path(code): Path<String>,
    Json(b): Json<SortieTypeSaisieDto>,
) -> Res<Json<SortieTypeDto>> {
    Ok(Json(
        s.ent
            .modifier_type_sortie_a(se.etablissement_id, se.user_id, &se.role, &code, &b, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

// ───────────────────────── Absences ─────────────────────────

async fn annoncer_absence(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<AbsenceAnnonceDto>,
) -> Res<(StatusCode, Json<AbsenceDto>)> {
    date_bornee(b.du_nuit)?;
    date_bornee(b.au_nuit)?;
    let r = s
        .ent
        .annoncer_absence_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn absences_a_traiter(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<AbsenceDto>>> {
    Ok(Json(s.ent.absences_a_traiter_a(se.etablissement_id, se.user_id, &se.role).await.map_err(map_db)?))
}

async fn absences_eleve(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<AbsenceDto>>> {
    Ok(Json(
        s.ent.absences_eleve_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn decider_absence(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DecisionDto>,
) -> Res<Json<AbsenceDto>> {
    Ok(Json(
        s.ent.decider_absence_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn annuler_absence(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<AbsenceDto>> {
    Ok(Json(
        s.ent.annuler_absence_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn opposer_absence(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<OppositionDto>,
) -> Res<Json<AbsenceDto>> {
    Ok(Json(
        s.ent.opposer_absence_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn trancher_absence(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TrancheDto>,
) -> Res<Json<AbsenceDto>> {
    Ok(Json(
        s.ent.trancher_absence_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now()).await.map_err(map_db)?,
    ))
}

// ───────────────────────── Sorties ─────────────────────────

async fn demander_sortie(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<SortieDemandeDto>,
) -> Res<(StatusCode, Json<SortieDto>)> {
    let r = s
        .ent
        .demander_sortie_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn sorties_a_traiter(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<SortieDto>>> {
    Ok(Json(s.ent.sorties_a_traiter_a(se.etablissement_id, se.user_id, &se.role).await.map_err(map_db)?))
}

async fn sorties_eleve(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<SortieDto>>> {
    Ok(Json(
        s.ent.sorties_eleve_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn repondre_sortie(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AccordTitulaireDto>,
) -> Res<Json<SortieDto>> {
    Ok(Json(
        s.ent.repondre_sortie_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn decider_sortie(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SortieDecisionDto>,
) -> Res<Json<SortieDto>> {
    Ok(Json(
        s.ent.decider_sortie_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn annuler_sortie(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<SortieDto>> {
    Ok(Json(
        s.ent.annuler_sortie_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn opposer_sortie(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<OppositionDto>,
) -> Res<Json<SortieDto>> {
    Ok(Json(
        s.ent.opposer_sortie_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn trancher_sortie(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<TrancheDto>,
) -> Res<Json<SortieDto>> {
    Ok(Json(
        s.ent.trancher_sortie_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn designer_personne(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PersonneSortieDto>,
) -> Res<Json<SortieDto>> {
    Ok(Json(
        s.ent
            .designer_personne_sortie_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn annoncer_retard(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RetardAnnonceDto>,
) -> Res<Json<SortieDto>> {
    Ok(Json(
        s.ent.annoncer_retard_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn controler_depart(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ControleDepartDto>,
) -> Res<Json<SortieDto>> {
    Ok(Json(
        s.ent.controler_depart_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn controler_retour(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ControleRetourDto>,
) -> Res<Json<SortieDto>> {
    Ok(Json(
        s.ent.controler_retour_a(se.etablissement_id, se.user_id, &se.role, id, &b, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn preparer_departs(
    State(s): State<AppState>,
    se: EntSession,
    Path(jour): Path<NaiveDate>,
) -> Res<Json<DepartsPrevusDto>> {
    let jour = date_bornee(jour)?;
    Ok(Json(
        s.ent.preparer_departs_a(se.etablissement_id, se.user_id, &se.role, jour, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn liste_remise(
    State(s): State<AppState>,
    se: EntSession,
    Path(jour): Path<NaiveDate>,
) -> Res<Json<Vec<ListeRemiseLigneDto>>> {
    let jour = date_bornee(jour)?;
    Ok(Json(s.ent.liste_remise_a(se.etablissement_id, se.user_id, &se.role, jour, Utc::now()).await.map_err(map_db)?))
}

async fn sorties_du_soir(
    State(s): State<AppState>,
    se: EntSession,
    Path(nuit): Path<NaiveDate>,
) -> Res<Json<Vec<SortieDuSoirDto>>> {
    let nuit = date_bornee(nuit)?;
    Ok(Json(s.ent.sorties_du_soir_a(se.etablissement_id, se.user_id, &se.role, nuit, Utc::now()).await.map_err(map_db)?))
}

// ───────────────────────── Correspondants ─────────────────────────

async fn designer_correspondant(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<CorrespondantSaisieDto>,
) -> Res<(StatusCode, Json<CorrespondantDto>)> {
    if let Some(d) = b.du {
        date_bornee(d)?;
    }
    if let Some(d) = b.au {
        date_bornee(d)?;
    }
    let r = s
        .ent
        .designer_correspondant_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn correspondants_eleve(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<CorrespondantDto>>> {
    Ok(Json(
        s.ent.correspondants_eleve_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn suspendre_correspondant(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CorrespondantSuspensionDto>,
) -> Res<Json<CorrespondantDto>> {
    Ok(Json(
        s.ent
            .suspendre_correspondant_a(se.etablissement_id, se.user_id, &se.role, id, true, &b, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn reprendre_correspondant(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CorrespondantSuspensionDto>,
) -> Res<Json<CorrespondantDto>> {
    Ok(Json(
        s.ent
            .suspendre_correspondant_a(se.etablissement_id, se.user_id, &se.role, id, false, &b, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn terminer_correspondant(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<CorrespondantDto>> {
    Ok(Json(
        s.ent.terminer_correspondant_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map_err(map_db)?,
    ))
}

// ───────────────────────── Autorisations annuelles ─────────────────────────

async fn autorisations_eleve(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<AutorisationsEleveDto>> {
    Ok(Json(
        s.ent.autorisations_eleve_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn demander_autorisations(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<AutorisationsDemandeDto>,
) -> Res<Json<AutorisationsDemandeesDto>> {
    Ok(Json(s.ent.demander_autorisations_a(se.etablissement_id, se.user_id, &se.role, &b).await.map_err(map_db)?))
}

async fn revoquer_autorisation(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<RevocationDto>,
) -> Res<Json<RevocationResultatDto>> {
    Ok(Json(
        s.ent.revoquer_autorisation_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now()).await.map_err(map_db)?,
    ))
}

// ───────────────────────── Historique ─────────────────────────

async fn historique(s: AppState, se: EntSession, objet: &str, id: Uuid) -> Res<Json<Vec<FaitDto>>> {
    Ok(Json(
        s.ent.historique_a(se.etablissement_id, se.user_id, &se.role, objet, id, Utc::now()).await.map_err(map_db)?,
    ))
}

async fn historique_absence(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<FaitDto>>> {
    historique(s, se, "absence", id).await
}

async fn historique_sortie(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<FaitDto>>> {
    historique(s, se, "sortie", id).await
}

async fn historique_correspondant(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<FaitDto>>> {
    historique(s, se, "correspondant", id).await
}
