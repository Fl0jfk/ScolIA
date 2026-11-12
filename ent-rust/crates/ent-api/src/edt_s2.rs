//! Routes du lot S2-5 : emploi du temps (administration, semaine, remplacements,
//! annulations, déplacements, annulation en masse), absences de personnel et délégations.
//!
//! L'autorité est `ent-db` (moteur de droits D-017 : `edt.gerer`, `edt.annuler_masse`,
//! `delegations.gerer`, délégations datées comprises) : ces routes ne font que traduire
//! HTTP ↔ service. L'établissement vient de la session ; une référence d'un autre
//! établissement répond 404. Un créneau en conflit répond 409 avec la liste des conflits.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_db::{
    AbsenceSeances, Candidat, ChangementEdt, CibleEdt, CmdAnnulationMasse, CmdDeplacement,
    CmdRemplacement, ConflitEdt, CreneauAdmin, CreneauSaisie, Delegation, DelegationSaisie,
    RapportMasse, ResultatCreneau, ResultatException, SemaineEdt,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Vue de la semaine (personnel, classe, salle, élève) et changements visibles.
        .route("/edt/semaine", get(semaine))
        .route("/eleves/{id}/edt/changements", get(changements))
        // Administration des créneaux (EDT-01/02) et conflits.
        .route(
            "/edt/admin/creneaux",
            get(creneaux_admin).post(creer_creneau),
        )
        .route(
            "/edt/admin/creneaux/{id}",
            put(modifier_creneau).delete(supprimer_creneau),
        )
        .route("/edt/admin/conflits", get(conflits))
        .route("/edt/admin/salles/{id}", put(modifier_salle))
        // Absences de personnel → séances touchées → remplacements (RH-04, EDT-05).
        .route("/edt/absences", get(absences))
        .route("/edt/absences/{id}/seances", get(absence_seances))
        .route("/edt/remplacants", get(remplacants))
        .route("/edt/remplacements", post(organiser_remplacement))
        // Exceptions (EDT-04) et annulation en masse (EDT-08).
        .route("/edt/annulations", post(annuler_seance))
        .route("/edt/changements-salle", post(changer_salle))
        .route("/edt/deplacements", post(deplacer_seance))
        .route("/edt/exceptions-levee", delete(lever_exception))
        .route("/edt/annulation-masse", post(annulation_masse))
        // Délégations datées (ADM-04).
        .route("/delegations", get(delegations).post(accorder_delegation))
        .route("/delegations/mes", get(mes_delegations))
        .route("/delegations/{id}/revoquer", post(revoquer_delegation))
}

// ───── Semaine ─────

#[derive(Debug, Deserialize)]
struct SemaineQuery {
    /// `personnel` (défaut : soi), `classe`, `salle`, `eleve`.
    #[serde(default)]
    cible: Option<String>,
    #[serde(default)]
    id: Option<Uuid>,
    #[serde(default)]
    salle: Option<String>,
    #[serde(default)]
    jour: Option<NaiveDate>,
}

async fn semaine(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<SemaineQuery>,
) -> Res<Json<SemaineEdt>> {
    let mauvaise = || (StatusCode::BAD_REQUEST, "cible invalide".to_string());
    let cible = match q.cible.as_deref().unwrap_or("personnel") {
        "personnel" => CibleEdt::Personnel(q.id.unwrap_or(session.user_id)),
        "classe" => CibleEdt::Classe(q.id.ok_or_else(mauvaise)?),
        "salle" => CibleEdt::Salle(
            q.salle
                .clone()
                .filter(|x| !x.trim().is_empty())
                .ok_or_else(mauvaise)?,
        ),
        "eleve" => CibleEdt::Eleve(q.id.ok_or_else(mauvaise)?),
        _ => return Err(mauvaise()),
    };
    // D8 (socle 10 T0) : jour de l'établissement (fuseau du paramètre), pas le jour UTC.
    let jour = match q.jour {
        Some(j) => j,
        None => s
            .ent
            .aujourdhui_etablissement(session.etablissement_id)
            .await
            .map_err(map_db)?,
    };
    Ok(Json(
        s.ent
            .edt_semaine(&acteur(&session), cible, jour)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct Periode {
    du: NaiveDate,
    au: NaiveDate,
}

async fn changements(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(p): Query<Periode>,
) -> Res<Json<Vec<ChangementEdt>>> {
    Ok(Json(
        s.ent
            .edt_changements(&acteur(&session), id, p.du, p.au)
            .await
            .map_err(map_db)?,
    ))
}

// ───── Créneaux ─────

#[derive(Debug, Deserialize)]
struct FiltreCreneaux {
    #[serde(default)]
    classe: Option<String>,
    #[serde(default)]
    prof: Option<Uuid>,
    #[serde(default)]
    salle: Option<String>,
}

async fn creneaux_admin(
    State(s): State<AppState>,
    session: EntSession,
    Query(f): Query<FiltreCreneaux>,
) -> Res<Json<Vec<CreneauAdmin>>> {
    Ok(Json(
        s.ent
            .edt_creneaux_admin(
                &acteur(&session),
                f.classe.as_deref(),
                f.prof,
                f.salle.as_deref(),
            )
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize, Default)]
struct ApercuQuery {
    #[serde(default)]
    apercu: bool,
}

#[derive(Debug, Serialize)]
struct ConflitsReponse {
    message: &'static str,
    conflits: Vec<ConflitEdt>,
}

fn reponse_creneau(
    r: ResultatCreneau,
    cree: bool,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ConflitsReponse>)> {
    match r {
        ResultatCreneau::Enregistre { creneau } => Ok((
            if cree && creneau.is_some() {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            },
            Json(serde_json::json!({ "resultat": "enregistre", "creneau": creneau })),
        )),
        ResultatCreneau::Conflits { conflits } => Err((
            StatusCode::CONFLICT,
            Json(ConflitsReponse {
                message: "conflit d'emploi du temps",
                conflits,
            }),
        )),
    }
}

async fn creer_creneau(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<ApercuQuery>,
    Json(body): Json<CreneauSaisie>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    match s
        .ent
        .edt_creneau_creer(&acteur(&session), &body, q.apercu)
        .await
    {
        Ok(r) => match reponse_creneau(r, true) {
            Ok(ok) => ok.into_response(),
            Err(conflit) => conflit.into_response(),
        },
        Err(e) => map_db(e).into_response(),
    }
}

async fn modifier_creneau(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<ApercuQuery>,
    Json(body): Json<CreneauSaisie>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    match s
        .ent
        .edt_creneau_modifier(&acteur(&session), id, &body, q.apercu)
        .await
    {
        Ok(r) => match reponse_creneau(r, false) {
            Ok(ok) => ok.into_response(),
            Err(conflit) => conflit.into_response(),
        },
        Err(e) => map_db(e).into_response(),
    }
}

async fn supprimer_creneau(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<StatusCode> {
    s.ent
        .edt_creneau_supprimer(&acteur(&session), id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn conflits(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<ConflitEdt>>> {
    Ok(Json(
        s.ent
            .edt_conflits(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct SalleBody {
    libelle: String,
    #[serde(default)]
    capacite: Option<i32>,
    #[serde(default = "vrai")]
    actif: bool,
}

fn vrai() -> bool {
    true
}

async fn modifier_salle(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SalleBody>,
) -> Res<StatusCode> {
    s.ent
        .edt_salle_modifier(&acteur(&session), id, &b.libelle, b.capacite, b.actif)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

// ───── Absences, remplaçants, remplacements ─────

async fn absences(
    State(s): State<AppState>,
    session: EntSession,
    Query(p): Query<Periode>,
) -> Res<Json<Vec<AbsenceSeances>>> {
    Ok(Json(
        s.ent
            .edt_absences(&acteur(&session), p.du, p.au)
            .await
            .map_err(map_db)?,
    ))
}

async fn absence_seances(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<AbsenceSeances>> {
    Ok(Json(
        s.ent
            .edt_absence_seances(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct RemplacantsQuery {
    creneau_id: Uuid,
    jour: NaiveDate,
    #[serde(default)]
    tous: bool,
}

async fn remplacants(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<RemplacantsQuery>,
) -> Res<Json<Vec<Candidat>>> {
    Ok(Json(
        s.ent
            .edt_candidats_remplacants(&acteur(&session), q.creneau_id, q.jour, q.tous)
            .await
            .map_err(map_db)?,
    ))
}

async fn organiser_remplacement(
    State(s): State<AppState>,
    session: EntSession,
    Json(cmd): Json<CmdRemplacement>,
) -> Res<(StatusCode, Json<ResultatException>)> {
    let r = s
        .ent
        .edt_organiser_remplacement(&acteur(&session), &cmd)
        .await
        .map_err(map_db)?;
    Ok((
        if r.deja_fait {
            StatusCode::OK
        } else {
            StatusCode::CREATED
        },
        Json(r),
    ))
}

// ───── Exceptions ─────

#[derive(Debug, Deserialize)]
struct AnnulationBody {
    creneau_id: Uuid,
    jour: NaiveDate,
    #[serde(default)]
    absence_rh_id: Option<Uuid>,
}

async fn annuler_seance(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<AnnulationBody>,
) -> Res<(StatusCode, Json<ResultatException>)> {
    let r = s
        .ent
        .edt_annuler_seance(&acteur(&session), b.creneau_id, b.jour, b.absence_rh_id)
        .await
        .map_err(map_db)?;
    Ok((
        if r.deja_fait {
            StatusCode::OK
        } else {
            StatusCode::CREATED
        },
        Json(r),
    ))
}

#[derive(Debug, Deserialize)]
struct SalleChangeBody {
    creneau_id: Uuid,
    jour: NaiveDate,
    salle: String,
}

async fn changer_salle(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<SalleChangeBody>,
) -> Res<(StatusCode, Json<ResultatException>)> {
    let r = s
        .ent
        .edt_changer_salle(&acteur(&session), b.creneau_id, b.jour, &b.salle)
        .await
        .map_err(map_db)?;
    Ok((
        if r.deja_fait {
            StatusCode::OK
        } else {
            StatusCode::CREATED
        },
        Json(r),
    ))
}

async fn deplacer_seance(
    State(s): State<AppState>,
    session: EntSession,
    Json(cmd): Json<CmdDeplacement>,
) -> Res<(StatusCode, Json<ResultatException>)> {
    let r = s
        .ent
        .edt_deplacer_seance(&acteur(&session), &cmd)
        .await
        .map_err(map_db)?;
    Ok((
        if r.deja_fait {
            StatusCode::OK
        } else {
            StatusCode::CREATED
        },
        Json(r),
    ))
}

#[derive(Debug, Deserialize)]
struct LeverQuery {
    creneau_id: Uuid,
    jour: NaiveDate,
}

async fn lever_exception(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<LeverQuery>,
) -> Res<StatusCode> {
    let leve = s
        .ent
        .edt_lever_exception(&acteur(&session), q.creneau_id, q.jour)
        .await
        .map_err(map_db)?;
    Ok(if leve {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    })
}

async fn annulation_masse(
    State(s): State<AppState>,
    session: EntSession,
    Json(cmd): Json<CmdAnnulationMasse>,
) -> Res<(StatusCode, Json<RapportMasse>)> {
    let r = s
        .ent
        .edt_annulation_masse(&acteur(&session), &cmd)
        .await
        .map_err(map_db)?;
    let code = if r.apercu || r.deja_applique {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((code, Json(r)))
}

// ───── Délégations ─────

#[derive(Debug, Deserialize)]
struct DelegationsQuery {
    #[serde(default)]
    en_cours: bool,
}

async fn delegations(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<DelegationsQuery>,
) -> Res<Json<Vec<Delegation>>> {
    Ok(Json(
        s.ent
            .delegations_lister(&acteur(&session), q.en_cours)
            .await
            .map_err(map_db)?,
    ))
}

async fn mes_delegations(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<Delegation>>> {
    Ok(Json(
        s.ent
            .delegations_mes(&acteur(&session))
            .await
            .map_err(map_db)?,
    ))
}

async fn accorder_delegation(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<DelegationSaisie>,
) -> Res<(StatusCode, Json<Delegation>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.ent
                .delegation_accorder(&acteur(&session), &b)
                .await
                .map_err(map_db)?,
        ),
    ))
}

async fn revoquer_delegation(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Delegation>> {
    Ok(Json(
        s.ent
            .delegation_revoquer(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}
