//! Routes `/internat/*` du socle 41, lot I4 « Vie quotidienne » : sorties de groupe, liaison codée, événements de nuit,
//! temps du soir (études, activités), préoccupation, planning de l'élève.
//!
//! L'établissement et l'acteur viennent de la session, jamais du corps. Aucune règle ici : `ent-db` décide (droits,
//! portée, qualité de titulaire, interdictions, désaccords). L'heure de chaque commande est celle du serveur.
//! **Aucune route sans session** (R-IT39).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::{NaiveDate, Utc};
use ent_types::internat_vie::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

#[derive(Deserialize)]
struct Periode {
    du: NaiveDate,
    au: NaiveDate,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/internat/sorties-groupe",
            post(creer_groupe).get(lister_groupes),
        )
        .route(
            "/internat/sorties-groupe/{id}",
            get(groupe).put(modifier_groupe),
        )
        .route(
            "/internat/sorties-groupe/{id}/soumission",
            post(soumettre_groupe),
        )
        .route(
            "/internat/sorties-groupe/{id}/decision",
            post(decider_groupe),
        )
        .route(
            "/internat/sorties-groupe/{id}/annulation",
            post(annuler_groupe),
        )
        .route(
            "/internat/sorties-groupe/{id}/duplication",
            post(dupliquer_groupe),
        )
        .route(
            "/internat/sorties-groupe/{id}/encadrants",
            post(ajouter_encadrant),
        )
        .route(
            "/internat/sorties-groupe/{id}/encadrants/{user}/retrait",
            post(retirer_encadrant),
        )
        .route("/internat/sorties-groupe/{id}/eleves", post(inviter_eleves))
        .route(
            "/internat/sorties-groupe/{id}/eleves/{eleve}/retrait",
            post(retirer_eleve),
        )
        .route(
            "/internat/sorties-groupe/{id}/eleves/{eleve}/reponse",
            post(repondre_groupe),
        )
        .route(
            "/internat/sorties-groupe/{id}/eleves/{eleve}/accord-hors-ligne",
            post(accord_hors_ligne),
        )
        .route("/internat/sorties-groupe/{id}/depart", post(depart_groupe))
        .route("/internat/sorties-groupe/{id}/retour", post(retour_groupe))
        .route(
            "/internat/sorties-groupe/{id}/retour/annulation",
            post(annuler_retour_groupe),
        )
        .route(
            "/internat/sorties-groupe/{id}/encadrants/remplacement",
            post(remplacer_encadrant),
        )
        .route("/internat/sorties-groupe/{id}/cloture", post(clore_groupe))
        .route("/internat/eleves/{id}/sorties-groupe", get(groupes_eleve))
        .route("/internat/eleves/{id}/planning", get(planning_eleve))
        .route(
            "/internat/liaison-codes",
            get(codes_liaison).post(creer_code_liaison),
        )
        .route("/internat/liaison-codes/{code}", put(modifier_code_liaison))
        .route("/internat/liaison", post(ecrire_liaison))
        .route("/internat/liaison/{id}/geste", post(geste_liaison))
        .route("/internat/eleves/{id}/liaison", get(liaison_eleve))
        .route("/internat/nuits/{nuit}/liaison", get(liaison_nuit))
        .route("/internat/evenements", post(declarer_evenement))
        .route("/internat/evenements/{id}", get(evenement))
        .route(
            "/internat/evenements/{id}/complement",
            post(completer_evenement),
        )
        .route(
            "/internat/evenements/{id}/transmission",
            post(transmettre_evenement),
        )
        .route(
            "/internat/evenements/{id}/intervention",
            post(demander_intervention),
        )
        .route("/internat/nuits/{nuit}/evenements", get(evenements_nuit))
        .route("/internat/preoccupations", post(ouvrir_preoccupation))
        .route("/internat/vigilances", post(poser_vigilance))
        .route("/internat/vigilances/{id}/revue", post(reviser_vigilance))
        .route("/internat/vigilances/{id}/levee", post(lever_vigilance))
        .route("/internat/eleves/{id}/vigilances", get(vigilances_eleve))
        .route("/internat/temps", post(creer_temps).get(lister_temps))
        .route("/internat/temps/{id}/annulation", post(annuler_temps))
        .route("/internat/temps/{id}/inscription", post(inscrire_temps))
        .route(
            "/internat/temps/{id}/desinscription",
            post(desinscrire_temps),
        )
        .route("/internat/temps/{id}/absence", post(absence_etude))
        .route("/internat/temps/{id}/pointage", post(pointer_temps))
        .route(
            "/internat/reglages-vie",
            get(lire_reglages_vie).put(modifier_reglages_vie),
        )
}

/// Bornes raisonnables d'une date reçue d'un client (une date absurde ferait déborder les calculs de la base).
fn date_bornee(d: NaiveDate) -> Res<NaiveDate> {
    let (min, max) = (
        NaiveDate::from_ymd_opt(2000, 1, 1).unwrap_or(NaiveDate::MIN),
        NaiveDate::from_ymd_opt(2100, 12, 31).unwrap_or(NaiveDate::MAX),
    );
    if d < min || d > max {
        return Err((
            StatusCode::BAD_REQUEST,
            "date hors des bornes raisonnables (2000 à 2100)".into(),
        ));
    }
    Ok(d)
}

async fn creer_groupe(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<SortieGroupeSaisieDto>,
) -> Res<(StatusCode, Json<SortieGroupeDto>)> {
    date_bornee(b.debut.date_naive())?;
    date_bornee(b.fin.date_naive())?;
    let r = s
        .ent
        .creer_sortie_groupe_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn groupe(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<SortieGroupeDto>> {
    Ok(Json(
        s.ent
            .sortie_groupe_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn modifier_groupe(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SortieGroupeSaisieDto>,
) -> Res<Json<SortieGroupeDto>> {
    Ok(Json(
        s.ent
            .modifier_sortie_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn soumettre_groupe(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<SortieGroupeDto>> {
    Ok(Json(
        s.ent
            .soumettre_sortie_groupe_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn decider_groupe(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DecisionGroupeDto>,
) -> Res<Json<SortieGroupeDto>> {
    Ok(Json(
        s.ent
            .decider_sortie_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn annuler_groupe(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AnnulationGroupeDto>,
) -> Res<Json<SortieGroupeDto>> {
    Ok(Json(
        s.ent
            .annuler_sortie_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn dupliquer_groupe(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DuplicationGroupeDto>,
) -> Res<(StatusCode, Json<SortieGroupeDto>)> {
    date_bornee(b.debut.date_naive())?;
    date_bornee(b.fin.date_naive())?;
    let r = s
        .ent
        .dupliquer_sortie_groupe_a(
            se.etablissement_id,
            se.user_id,
            &se.role,
            id,
            &b,
            Utc::now(),
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn inviter_eleves(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ElevesAjoutDto>,
) -> Res<Json<SortieGroupeDto>> {
    Ok(Json(
        s.ent
            .inviter_eleves_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn retirer_eleve(
    State(s): State<AppState>,
    se: EntSession,
    Path((id, eleve)): Path<(Uuid, Uuid)>,
) -> Res<Json<SortieGroupeDto>> {
    Ok(Json(
        s.ent
            .retirer_eleve_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                eleve,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn repondre_groupe(
    State(s): State<AppState>,
    se: EntSession,
    Path((id, eleve)): Path<(Uuid, Uuid)>,
    Json(b): Json<ReponseGroupeDto>,
) -> Res<Json<SortieGroupeFamilleDto>> {
    Ok(Json(
        s.ent
            .repondre_sortie_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                eleve,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn accord_hors_ligne(
    State(s): State<AppState>,
    se: EntSession,
    Path((id, eleve)): Path<(Uuid, Uuid)>,
    Json(b): Json<AccordHorsLigneDto>,
) -> Res<Json<SortieGroupeDto>> {
    Ok(Json(
        s.ent
            .accord_hors_ligne_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                eleve,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn depart_groupe(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AppelGroupeDto>,
) -> Res<Json<Vec<ResultatAppelDto>>> {
    Ok(Json(
        s.ent
            .depart_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn retour_groupe(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AppelGroupeDto>,
) -> Res<Json<Vec<ResultatAppelDto>>> {
    Ok(Json(
        s.ent
            .retour_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn clore_groupe(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ClotureGroupeDto>,
) -> Res<Json<SortieGroupeDto>> {
    Ok(Json(
        s.ent
            .clore_sortie_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn planning_eleve(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<PlanningEleveDto>> {
    Ok(Json(
        s.ent
            .planning_eleve_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn creer_code_liaison(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<LiaisonCodeCreationDto>,
) -> Res<(StatusCode, Json<LiaisonCodeDto>)> {
    let r = s
        .ent
        .creer_code_liaison_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn ecrire_liaison(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<LiaisonSaisieDto>,
) -> Res<(StatusCode, Json<LiaisonDto>)> {
    let r = s
        .ent
        .ecrire_liaison_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn geste_liaison(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<LiaisonGesteDto>,
) -> Res<Json<LiaisonDto>> {
    Ok(Json(
        s.ent
            .geste_liaison_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn liaison_eleve(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<LiaisonDto>>> {
    Ok(Json(
        s.ent
            .liaison_eleve_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn declarer_evenement(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<EvenementSaisieDto>,
) -> Res<(StatusCode, Json<EvenementDto>)> {
    let r = s
        .ent
        .declarer_evenement_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn evenement(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<EvenementDto>> {
    Ok(Json(
        s.ent
            .evenement_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn completer_evenement(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ComplementSaisieDto>,
) -> Res<Json<EvenementDto>> {
    Ok(Json(
        s.ent
            .completer_evenement_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn transmettre_evenement(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<EvenementDto>> {
    Ok(Json(
        s.ent
            .transmettre_evenement_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn demander_intervention(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<EvenementDto>> {
    Ok(Json(
        s.ent
            .demander_intervention_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn ouvrir_preoccupation(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<PreoccupationDto>,
) -> Res<(StatusCode, Json<PreoccupationAccuseDto>)> {
    let r = s
        .ent
        .ouvrir_preoccupation_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn creer_temps(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<TempsSaisieDto>,
) -> Res<(StatusCode, Json<TempsDto>)> {
    let r = s
        .ent
        .creer_temps_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn annuler_temps(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<TempsDto>> {
    Ok(Json(
        s.ent
            .annuler_temps_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn inscrire_temps(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<InscriptionTempsDto>,
) -> Res<Json<TempsDto>> {
    Ok(Json(
        s.ent
            .inscrire_temps_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn desinscrire_temps(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<InscriptionTempsDto>,
) -> Res<Json<TempsDto>> {
    Ok(Json(
        s.ent
            .desinscrire_temps_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                b.eleve_id,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn absence_etude(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<InscriptionTempsDto>,
) -> Res<Json<LiaisonDto>> {
    Ok(Json(
        s.ent
            .absence_etude_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn lister_groupes(
    State(s): State<AppState>,
    se: EntSession,
    Query(p): Query<Periode>,
) -> Res<Json<Vec<SortieGroupeResumeDto>>> {
    date_bornee(p.du)?;
    date_bornee(p.au)?;
    Ok(Json(
        s.ent
            .lister_sorties_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                p.du,
                p.au,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn ajouter_encadrant(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<EncadrantAjoutDto>,
) -> Res<Json<SortieGroupeDto>> {
    Ok(Json(
        s.ent
            .ajouter_encadrant_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                b.user_id,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn retirer_encadrant(
    State(s): State<AppState>,
    se: EntSession,
    Path((id, user)): Path<(Uuid, Uuid)>,
) -> Res<Json<SortieGroupeDto>> {
    Ok(Json(
        s.ent
            .retirer_encadrant_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                user,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn groupes_eleve(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<SortieGroupeFamilleDto>>> {
    Ok(Json(
        s.ent
            .sorties_groupe_eleve_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn codes_liaison(
    State(s): State<AppState>,
    se: EntSession,
) -> Res<Json<Vec<LiaisonCodeDto>>> {
    Ok(Json(
        s.ent
            .lister_codes_liaison_a(se.etablissement_id, se.user_id, &se.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn modifier_code_liaison(
    State(s): State<AppState>,
    se: EntSession,
    Path(code): Path<String>,
    Json(b): Json<LiaisonCodeSaisieDto>,
) -> Res<Json<LiaisonCodeDto>> {
    Ok(Json(
        s.ent
            .modifier_code_liaison_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                &code,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn liaison_nuit(
    State(s): State<AppState>,
    se: EntSession,
    Path(nuit): Path<NaiveDate>,
) -> Res<Json<Vec<LiaisonDto>>> {
    date_bornee(nuit)?;
    Ok(Json(
        s.ent
            .liaison_nuit_a(se.etablissement_id, se.user_id, &se.role, nuit, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn evenements_nuit(
    State(s): State<AppState>,
    se: EntSession,
    Path(nuit): Path<NaiveDate>,
) -> Res<Json<Vec<EvenementDto>>> {
    date_bornee(nuit)?;
    Ok(Json(
        s.ent
            .evenements_nuit_a(se.etablissement_id, se.user_id, &se.role, nuit, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn lister_temps(
    State(s): State<AppState>,
    se: EntSession,
    Query(p): Query<Periode>,
) -> Res<Json<Vec<TempsDto>>> {
    date_bornee(p.du)?;
    date_bornee(p.au)?;
    Ok(Json(
        s.ent
            .lister_temps_a(se.etablissement_id, se.user_id, &se.role, p.du, p.au)
            .await
            .map_err(map_db)?,
    ))
}

async fn poser_vigilance(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<VigilanceSaisieDto>,
) -> Res<(StatusCode, Json<VigilanceDto>)> {
    if let Some(d) = b.du {
        date_bornee(d)?;
    }
    if let Some(a) = b.au {
        date_bornee(a)?;
    }
    let r = s
        .ent
        .poser_vigilance_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn reviser_vigilance(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<VigilanceDto>> {
    Ok(Json(
        s.ent
            .reviser_vigilance_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn lever_vigilance(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<VigilanceDto>> {
    Ok(Json(
        s.ent
            .lever_vigilance_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn vigilances_eleve(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<VigilanceDto>>> {
    Ok(Json(
        s.ent
            .vigilances_eleve_a(se.etablissement_id, se.user_id, &se.role, id, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}

async fn annuler_retour_groupe(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<AnnulationRetourDto>,
) -> Res<Json<Vec<ResultatAppelDto>>> {
    Ok(Json(
        s.ent
            .annuler_retour_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn remplacer_encadrant(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RemplacementEncadrantDto>,
) -> Res<Json<SortieGroupeDto>> {
    Ok(Json(
        s.ent
            .remplacer_encadrant_groupe_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn pointer_temps(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PointageTempsDto>,
) -> Res<Json<TempsDto>> {
    Ok(Json(
        s.ent
            .pointer_temps_a(
                se.etablissement_id,
                se.user_id,
                &se.role,
                id,
                &b,
                Utc::now(),
            )
            .await
            .map_err(map_db)?,
    ))
}

async fn lire_reglages_vie(State(s): State<AppState>, se: EntSession) -> Res<Json<ReglagesVieDto>> {
    Ok(Json(
        s.ent
            .lire_reglages_vie_a(se.etablissement_id, se.user_id, &se.role)
            .await
            .map_err(map_db)?,
    ))
}

async fn modifier_reglages_vie(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<ReglagesVieSaisieDto>,
) -> Res<Json<ReglagesVieDto>> {
    Ok(Json(
        s.ent
            .modifier_reglages_vie_a(se.etablissement_id, se.user_id, &se.role, &b, Utc::now())
            .await
            .map_err(map_db)?,
    ))
}
