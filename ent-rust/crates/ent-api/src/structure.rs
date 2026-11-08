//! Routes `/structure/*` du socle 5, lot S2 : profil, fuseau, préréglages, paramètres, sites,
//! bâtiments, salles, lieux extérieurs, disponibilités, indisponibilités, import de salles.
//!
//! Aucune règle ici : les droits (`referentiel.lire` / `referentiel.gerer`), le cloisonnement
//! (établissement de la session, jamais du corps) et les validations sont dans `ent-db`
//! (`structure_etab`, `structure_lieux`). Une référence d'un autre établissement répond 404.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::{DateTime, NaiveDate, Utc};
use ent_db::structure_etab::{
    ApercuPrereglage, JournalLigne, ParametreVue, Prereglage, Profil, ProfilSaisie,
};
use ent_db::structure_lieux::{
    Batiment, Cible, Disponibilite, DisponibiliteSaisie, FiltreSalles, Indisponibilite, Lieu, LieuSaisie,
    RapportImportSalles, RapportIndisponibilite, SalleDetail, SalleSaisie, SiteDetail, SiteModif,
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/structure/profil", get(profil).put(profil_definir))
        .route("/structure/fuseau", put(fuseau_definir))
        .route("/structure/prereglages", get(prereglages))
        .route("/structure/prereglages/{code}/apercu", post(prereglage_apercu))
        .route("/structure/prereglages/{code}/appliquer", post(prereglage_appliquer))
        .route("/structure/parametres", get(parametres))
        .route("/structure/parametres/{cle}", put(parametre_definir))
        .route("/structure/journal", get(journal))
        .route("/structure/sites", get(sites))
        .route("/structure/sites/{id}", put(site_modifier))
        .route("/structure/sites/{id}/fermer", post(site_fermer))
        .route("/structure/sites/{id}/rouvrir", post(site_rouvrir))
        .route("/structure/batiments", get(batiments).post(batiment_creer))
        .route("/structure/batiments/{id}/fermer", post(batiment_fermer))
        .route("/structure/lieux", get(lieux).post(lieu_creer))
        .route("/structure/lieux/{id}", put(lieu_modifier))
        .route("/structure/lieux/{id}/fermer", post(lieu_fermer))
        .route("/structure/salles", get(salles).post(salle_creer))
        .route("/structure/salles/import", post(salles_importer))
        .route("/structure/salles/{id}", put(salle_modifier))
        .route("/structure/salles/{id}/fermer", post(salle_fermer))
        .route("/structure/salles/{id}/rattacher", post(salle_rattacher))
        .route("/structure/salles/{id}/disponibilites", get(dispo_salle).post(dispo_salle_ajouter))
        .route("/structure/lieux/{id}/disponibilites", get(dispo_lieu).post(dispo_lieu_ajouter))
        .route("/structure/disponibilites/{id}/annuler", post(dispo_annuler))
        .route("/structure/salles/{id}/indisponibilites", get(indispo_salle).post(indispo_salle_declarer))
        .route("/structure/lieux/{id}/indisponibilites", get(indispo_lieu).post(indispo_lieu_declarer))
        .route("/structure/indisponibilites/lot", post(indispo_lot))
        .route("/structure/indisponibilites/{id}/annuler", post(indispo_annuler))
}

// ───── Profil, fuseau, préréglages, paramètres, journal ─────

async fn profil(State(s): State<AppState>, session: EntSession) -> Res<Json<Option<Profil>>> {
    Ok(Json(s.ent.structure_profil(&acteur(&session)).await.map_err(map_db)?))
}

async fn profil_definir(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ProfilSaisie>,
) -> Res<Json<Profil>> {
    Ok(Json(s.ent.structure_profil_definir(&acteur(&session), b).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize)]
struct FuseauBody {
    fuseau: String,
    /// À vrai pour confirmer un changement pendant une année courante.
    #[serde(default)]
    confirmer: bool,
}

async fn fuseau_definir(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<FuseauBody>,
) -> Res<Json<Value>> {
    let f = s
        .ent
        .structure_fuseau_definir(&acteur(&session), &b.fuseau, b.confirmer)
        .await
        .map_err(map_db)?;
    Ok(Json(serde_json::json!({ "fuseau": f })))
}

async fn prereglages(State(s): State<AppState>, _session: EntSession) -> Json<Vec<Prereglage>> {
    Json(s.ent.structure_prereglages())
}

async fn prereglage_apercu(
    State(s): State<AppState>,
    session: EntSession,
    Path(code): Path<String>,
) -> Res<Json<ApercuPrereglage>> {
    Ok(Json(s.ent.structure_prereglage_appliquer(&acteur(&session), &code, true).await.map_err(map_db)?))
}

async fn prereglage_appliquer(
    State(s): State<AppState>,
    session: EntSession,
    Path(code): Path<String>,
) -> Res<Json<ApercuPrereglage>> {
    Ok(Json(s.ent.structure_prereglage_appliquer(&acteur(&session), &code, false).await.map_err(map_db)?))
}

async fn parametres(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<ParametreVue>>> {
    Ok(Json(s.ent.structure_parametres(&acteur(&session)).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize)]
struct ParametreBody {
    valeur: Value,
    version_attendue: Option<i32>,
}

async fn parametre_definir(
    State(s): State<AppState>,
    session: EntSession,
    Path(cle): Path<String>,
    Json(b): Json<ParametreBody>,
) -> Res<Json<ParametreVue>> {
    Ok(Json(
        s.ent
            .structure_parametre_definir(&acteur(&session), &cle, b.valeur, b.version_attendue)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct JournalQuery {
    objet: String,
    #[serde(rename = "ref")]
    objet_ref: Option<String>,
    limite: Option<i64>,
}

async fn journal(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<JournalQuery>,
) -> Res<Json<Vec<JournalLigne>>> {
    Ok(Json(
        s.ent
            .structure_journal(&acteur(&session), &q.objet, q.objet_ref.as_deref(), q.limite.unwrap_or(100))
            .await
            .map_err(map_db)?,
    ))
}

// ───── Sites ─────

async fn sites(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<SiteDetail>>> {
    Ok(Json(s.ent.structure_sites(&acteur(&session)).await.map_err(map_db)?))
}

async fn site_modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SiteModif>,
) -> Res<Json<SiteDetail>> {
    Ok(Json(s.ent.structure_site_modifier(&acteur(&session), id, b).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize)]
struct FermerBody {
    ferme_le: NaiveDate,
}

async fn site_fermer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<FermerBody>,
) -> Res<Json<SiteDetail>> {
    Ok(Json(s.ent.structure_site_fermer(&acteur(&session), id, b.ferme_le).await.map_err(map_db)?))
}

async fn site_rouvrir(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<SiteDetail>> {
    Ok(Json(s.ent.structure_site_rouvrir(&acteur(&session), id).await.map_err(map_db)?))
}

// ───── Bâtiments ─────

#[derive(Debug, Deserialize)]
struct BatimentsQuery {
    site_id: Option<Uuid>,
}

async fn batiments(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<BatimentsQuery>,
) -> Res<Json<Vec<Batiment>>> {
    Ok(Json(s.ent.structure_batiments(&acteur(&session), q.site_id).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize)]
struct BatimentBody {
    site_id: Uuid,
    code: String,
    libelle: String,
}

async fn batiment_creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<BatimentBody>,
) -> Res<(StatusCode, Json<Batiment>)> {
    let bat = s
        .ent
        .structure_batiment_creer(&acteur(&session), b.site_id, &b.code, &b.libelle)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(bat)))
}

async fn batiment_fermer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<FermerBody>,
) -> Res<Json<Batiment>> {
    Ok(Json(s.ent.structure_batiment_fermer(&acteur(&session), id, b.ferme_le).await.map_err(map_db)?))
}

// ───── Lieux extérieurs ─────

async fn lieux(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<Lieu>>> {
    Ok(Json(s.ent.structure_lieux(&acteur(&session)).await.map_err(map_db)?))
}

async fn lieu_creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<LieuSaisie>,
) -> Res<(StatusCode, Json<Lieu>)> {
    let l = s.ent.structure_lieu_creer(&acteur(&session), b).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(l)))
}

async fn lieu_modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<LieuSaisie>,
) -> Res<Json<Lieu>> {
    Ok(Json(s.ent.structure_lieu_modifier(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn lieu_fermer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<FermerBody>,
) -> Res<Json<Lieu>> {
    Ok(Json(s.ent.structure_lieu_fermer(&acteur(&session), id, b.ferme_le).await.map_err(map_db)?))
}

// ───── Salles ─────

async fn salles(
    State(s): State<AppState>,
    session: EntSession,
    Query(f): Query<FiltreSalles>,
) -> Res<Json<Vec<SalleDetail>>> {
    Ok(Json(s.ent.structure_salles(&acteur(&session), &f).await.map_err(map_db)?))
}

async fn salle_creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<SalleSaisie>,
) -> Res<(StatusCode, Json<SalleDetail>)> {
    let r = s.ent.structure_salle_creer(&acteur(&session), b).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn salle_modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<SalleSaisie>,
) -> Res<Json<SalleDetail>> {
    Ok(Json(s.ent.structure_salle_modifier(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn salle_fermer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<FermerBody>,
) -> Res<Json<SalleDetail>> {
    Ok(Json(s.ent.structure_salle_fermer(&acteur(&session), id, b.ferme_le).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize)]
struct RattacherBody {
    site_id: Uuid,
}

async fn salle_rattacher(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<RattacherBody>,
) -> Res<Json<SalleDetail>> {
    Ok(Json(s.ent.structure_salle_rattacher(&acteur(&session), id, b.site_id).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize)]
struct ImportBody {
    csv: String,
    /// Par défaut : aperçu (rien n'est écrit tant que l'appelant ne le demande pas explicitement).
    #[serde(default = "vrai")]
    apercu: bool,
}

fn vrai() -> bool {
    true
}

async fn salles_importer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ImportBody>,
) -> Res<Json<RapportImportSalles>> {
    Ok(Json(s.ent.structure_salles_importer(&acteur(&session), &b.csv, b.apercu).await.map_err(map_db)?))
}

// ───── Disponibilités et indisponibilités ─────

async fn dispo_salle(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<Disponibilite>>> {
    Ok(Json(s.ent.structure_disponibilites(&acteur(&session), Cible::Salle(id)).await.map_err(map_db)?))
}

async fn dispo_lieu(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<Disponibilite>>> {
    Ok(Json(s.ent.structure_disponibilites(&acteur(&session), Cible::Lieu(id)).await.map_err(map_db)?))
}

async fn dispo_salle_ajouter(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DisponibiliteSaisie>,
) -> Res<(StatusCode, Json<Disponibilite>)> {
    let d = s.ent.structure_disponibilite_ajouter(&acteur(&session), Cible::Salle(id), b).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(d)))
}

async fn dispo_lieu_ajouter(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DisponibiliteSaisie>,
) -> Res<(StatusCode, Json<Disponibilite>)> {
    let d = s.ent.structure_disponibilite_ajouter(&acteur(&session), Cible::Lieu(id), b).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(d)))
}

async fn dispo_annuler(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    s.ent.structure_disponibilite_annuler(&acteur(&session), id).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct DepuisQuery {
    depuis: Option<DateTime<Utc>>,
}

async fn indispo_salle(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<DepuisQuery>,
) -> Res<Json<Vec<Indisponibilite>>> {
    Ok(Json(
        s.ent
            .structure_indisponibilites(&acteur(&session), Cible::Salle(id), q.depuis.unwrap_or_else(Utc::now))
            .await
            .map_err(map_db)?,
    ))
}

async fn indispo_lieu(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<DepuisQuery>,
) -> Res<Json<Vec<Indisponibilite>>> {
    Ok(Json(
        s.ent
            .structure_indisponibilites(&acteur(&session), Cible::Lieu(id), q.depuis.unwrap_or_else(Utc::now))
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct IndispoBody {
    du: DateTime<Utc>,
    au: DateTime<Utc>,
    motif_code: String,
    motif: Option<String>,
}

async fn indispo_salle_declarer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<IndispoBody>,
) -> Res<(StatusCode, Json<RapportIndisponibilite>)> {
    let r = s
        .ent
        .structure_indisponibilite_declarer(&acteur(&session), Cible::Salle(id), b.du, b.au, &b.motif_code, b.motif.as_deref())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn indispo_lieu_declarer(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<IndispoBody>,
) -> Res<(StatusCode, Json<RapportIndisponibilite>)> {
    let r = s
        .ent
        .structure_indisponibilite_declarer(&acteur(&session), Cible::Lieu(id), b.du, b.au, &b.motif_code, b.motif.as_deref())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

#[derive(Debug, Deserialize)]
struct LotBody {
    salles: Vec<Uuid>,
    du: DateTime<Utc>,
    au: DateTime<Utc>,
    motif_code: String,
    motif: Option<String>,
    evenement_ref: Option<Uuid>,
}

async fn indispo_lot(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<LotBody>,
) -> Res<(StatusCode, Json<RapportIndisponibilite>)> {
    let r = s
        .ent
        .structure_indisponibilite_lot(
            &acteur(&session),
            b.salles,
            b.du,
            b.au,
            &b.motif_code,
            b.motif.as_deref(),
            b.evenement_ref,
        )
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

#[derive(Debug, Deserialize, Default)]
struct AnnulerBody {
    #[serde(default)]
    tout_le_lot: bool,
}

async fn indispo_annuler(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<AnnulerBody>>,
) -> Res<Json<Value>> {
    let tout = body.map(|b| b.0.tout_le_lot).unwrap_or(false);
    let n = s.ent.structure_indisponibilite_annuler(&acteur(&session), id, tout).await.map_err(map_db)?;
    Ok(Json(serde_json::json!({ "annulees": n })))
}
