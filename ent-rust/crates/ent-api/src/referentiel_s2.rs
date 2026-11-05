//! Routes `/referentiel/*` du lot S2-2 : cycle de vie de l'année et bascule, sites,
//! niveaux / MEF, périodes, calendrier, grille horaire, séances datées, `classe_id`.
//!
//! Droits : moteur D-017 (`referentiel.lire`, `referentiel.gerer`), vérifié par `ent-db`
//! à chaque appel (l'API ne refait pas la règle). Cloisonnement : l'établissement vient de
//! la session, jamais du corps de la requête ; une référence d'un autre établissement
//! répond 404. Les réponses sont les types de `ent-db` (sérialisables) ; ils passeront en
//! DTO partagés (`ent-types`) quand les écrans Leptos s'y brancheront.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_db::{
    AnneeDetail, ComparaisonClasse, CreneauGrille, CreneauGrilleSaisie, JourCalendrier, Mef,
    MefImport, Niveau, PeriodeRef as Periode, PeriodeSaisie, RapportImportMef, RapportLot, RapportOuverture,
    RapportReconduction, RapportSeances, SeanceRef as Seance, Site,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/referentiel/annees-statut", get(annees))
        .route("/referentiel/annees/{id}/ouvrir", post(ouvrir_annee))
        .route("/referentiel/annees/{id}/cloturer", post(cloturer_annee))
        .route("/referentiel/annees/{id}/preparer-suivante", post(preparer_suivante))
        .route("/referentiel/annees/{id}/reconduire", post(reconduire))
        .route("/referentiel/annees/{id}/affectations-lot", post(affectations_lot))
        .route("/referentiel/annees/{id}/monter", post(monter))
        .route("/referentiel/annees/{id}/periodes", get(periodes).put(definir_periodes))
        .route("/referentiel/annees/{id}/seances", post(generer_seances))
        .route("/referentiel/sites", get(sites).post(creer_site))
        .route("/referentiel/sites/{id}", put(maj_site))
        .route("/referentiel/niveaux", get(niveaux).post(creer_niveau))
        .route("/referentiel/niveaux/amorcer", post(amorcer_niveaux))
        .route("/referentiel/mef", get(mefs))
        .route("/referentiel/mef/import", post(importer_mefs))
        .route("/referentiel/classes/{id}/modifier", put(modifier_classe))
        .route("/referentiel/classes/{id}/professeur-principal", put(designer_pp))
        .route("/referentiel/calendrier", get(calendrier))
        .route("/referentiel/calendrier/jours", post(ajouter_jours).delete(retirer_jour))
        .route("/referentiel/calendrier/vacances", post(creer_vacances))
        .route("/referentiel/grille", get(grille).put(definir_grille))
        .route("/referentiel/grille/amorcer", post(amorcer_grille))
        .route("/referentiel/seances", get(seances))
        .route("/referentiel/creneaux/{id}/service", put(lier_service))
        .route("/referentiel/classe-id/comparaison", get(comparaison))
        .route("/referentiel/classe-id/rattacher", post(rattacher))
}

#[derive(Debug, Serialize)]
struct Id {
    id: Uuid,
}

// ───── Année ─────

async fn annees(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<AnneeDetail>>> {
    Ok(Json(s.ent.ref_annees(&acteur(&session)).await.map_err(map_db)?))
}

async fn ouvrir_annee(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<RapportOuverture>> {
    Ok(Json(s.ent.ref_ouvrir_annee(&acteur(&session), id).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize, Default)]
struct CloturerBody {
    #[serde(default)]
    forcer: bool,
}

async fn cloturer_annee(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<CloturerBody>>,
) -> Res<Json<AnneeDetail>> {
    let forcer = body.map(|b| b.0.forcer).unwrap_or(false);
    Ok(Json(
        s.ent
            .ref_cloturer_annee(&acteur(&session), id, forcer)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize, Default)]
struct ReconduireBody {
    cible_id: Option<Uuid>,
    #[serde(default)]
    avec_services: bool,
}

#[derive(Debug, Serialize)]
struct PreparationDto {
    annee: AnneeDetail,
    rapport: RapportReconduction,
}

async fn preparer_suivante(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<ReconduireBody>>,
) -> Res<Json<PreparationDto>> {
    let avec = body.map(|b| b.0.avec_services).unwrap_or(false);
    let (annee, rapport) = s
        .ent
        .ref_preparer_annee_suivante(&acteur(&session), id, avec)
        .await
        .map_err(map_db)?;
    Ok(Json(PreparationDto { annee, rapport }))
}

async fn reconduire(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<ReconduireBody>,
) -> Res<Json<RapportReconduction>> {
    let cible = body
        .cible_id
        .ok_or((StatusCode::BAD_REQUEST, "cible_id requis".to_string()))?;
    Ok(Json(
        s.ent
            .ref_reconduire_structures(&acteur(&session), id, cible, body.avec_services)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct ItemLot {
    eleve_id: Uuid,
    classe_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct LotBody {
    items: Vec<ItemLot>,
    #[serde(default = "vrai")]
    apercu: bool,
}

fn vrai() -> bool {
    true
}

async fn affectations_lot(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(body): Json<LotBody>,
) -> Res<Json<RapportLot>> {
    let items: Vec<(Uuid, Uuid)> = body.items.iter().map(|i| (i.eleve_id, i.classe_id)).collect();
    Ok(Json(
        s.ent
            .ref_affecter_lot(&acteur(&session), id, &items, body.apercu, "lot")
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct Correspondance {
    source: Uuid,
    cible: Uuid,
}

#[derive(Debug, Deserialize)]
struct MonterBody {
    source_id: Uuid,
    correspondances: Vec<Correspondance>,
    #[serde(default = "vrai")]
    apercu: bool,
}

async fn monter(
    State(s): State<AppState>,
    session: EntSession,
    Path(cible_id): Path<Uuid>,
    Json(body): Json<MonterBody>,
) -> Res<Json<RapportLot>> {
    let c: Vec<(Uuid, Uuid)> = body.correspondances.iter().map(|c| (c.source, c.cible)).collect();
    Ok(Json(
        s.ent
            .ref_monter(&acteur(&session), body.source_id, cible_id, &c, body.apercu)
            .await
            .map_err(map_db)?,
    ))
}

// ───── Périodes ─────

#[derive(Debug, Deserialize)]
struct NiveauQuery {
    niveau_id: Option<Uuid>,
}

async fn periodes(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<NiveauQuery>,
) -> Res<Json<Vec<Periode>>> {
    Ok(Json(
        s.ent
            .ref_periodes(&acteur(&session), id, q.niveau_id)
            .await
            .map_err(map_db)?,
    ))
}

async fn definir_periodes(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Query(q): Query<NiveauQuery>,
    Json(body): Json<Vec<PeriodeSaisie>>,
) -> Res<Json<Vec<Periode>>> {
    Ok(Json(
        s.ent
            .ref_definir_periodes(&acteur(&session), id, q.niveau_id, &body)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize, Default)]
struct GenererBody {
    du: Option<NaiveDate>,
    au: Option<NaiveDate>,
}

async fn generer_seances(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<GenererBody>>,
) -> Res<Json<RapportSeances>> {
    let b = body.map(|b| b.0).unwrap_or_default();
    Ok(Json(
        s.ent
            .ref_generer_seances(&acteur(&session), id, b.du, b.au)
            .await
            .map_err(map_db)?,
    ))
}

// ───── Sites, niveaux, MEF ─────

async fn sites(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<Site>>> {
    Ok(Json(s.ent.ref_sites(&acteur(&session)).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize)]
struct CreerSiteBody {
    code: String,
    libelle: String,
    adresse: Option<String>,
    uai: Option<String>,
}

async fn creer_site(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerSiteBody>,
) -> Res<(StatusCode, Json<Site>)> {
    let site = s
        .ent
        .ref_creer_site(&acteur(&session), &b.code, &b.libelle, b.adresse.as_deref(), b.uai.as_deref())
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(site)))
}

#[derive(Debug, Deserialize)]
struct MajSiteBody {
    libelle: Option<String>,
    adresse: Option<String>,
    actif: Option<bool>,
}

async fn maj_site(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<MajSiteBody>,
) -> Res<Json<Site>> {
    Ok(Json(
        s.ent
            .ref_maj_site(&acteur(&session), id, b.libelle.as_deref(), b.adresse.as_deref(), b.actif)
            .await
            .map_err(map_db)?,
    ))
}

async fn niveaux(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<Niveau>>> {
    Ok(Json(s.ent.ref_niveaux(&acteur(&session)).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize)]
struct CreerNiveauBody {
    code: String,
    libelle: String,
    #[serde(default)]
    ordre: i16,
    cycle: Option<i16>,
}

async fn creer_niveau(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<CreerNiveauBody>,
) -> Res<(StatusCode, Json<Niveau>)> {
    let n = s
        .ent
        .ref_creer_niveau(&acteur(&session), &b.code, &b.libelle, b.ordre, b.cycle)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(n)))
}

#[derive(Debug, Serialize)]
struct AmorcageNiveauxDto {
    niveaux_crees: i64,
    classes_rattachees: i64,
}

async fn amorcer_niveaux(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<AmorcageNiveauxDto>> {
    let (niveaux_crees, classes_rattachees) = s
        .ent
        .ref_amorcer_niveaux(&acteur(&session))
        .await
        .map_err(map_db)?;
    Ok(Json(AmorcageNiveauxDto {
        niveaux_crees,
        classes_rattachees,
    }))
}

async fn mefs(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<Mef>>> {
    Ok(Json(s.ent.ref_mefs(&acteur(&session)).await.map_err(map_db)?))
}

#[derive(Debug, Deserialize)]
struct ImportMefBody {
    #[serde(default = "source_siecle")]
    source: String,
    lignes: Vec<MefImport>,
}

fn source_siecle() -> String {
    "import_siecle".into()
}

async fn importer_mefs(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ImportMefBody>,
) -> Res<Json<RapportImportMef>> {
    Ok(Json(
        s.ent
            .ref_importer_mefs(&acteur(&session), &b.lignes, &b.source)
            .await
            .map_err(map_db)?,
    ))
}

// ───── Classes ─────

#[derive(Debug, Deserialize)]
struct ModifierClasseBody {
    libelle: Option<String>,
    site_id: Option<Uuid>,
    niveau_id: Option<Uuid>,
    mef_id: Option<Uuid>,
    archiver: Option<bool>,
}

async fn modifier_classe(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ModifierClasseBody>,
) -> Res<StatusCode> {
    s.ent
        .ref_modifier_classe(
            &acteur(&session),
            id,
            b.libelle.as_deref(),
            b.site_id,
            b.niveau_id,
            b.mef_id,
            b.archiver,
        )
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct PpBody {
    prof_user_id: Uuid,
}

async fn designer_pp(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PpBody>,
) -> Res<StatusCode> {
    s.ent
        .ref_designer_pp(&acteur(&session), id, b.prof_user_id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

// ───── Calendrier ─────

#[derive(Debug, Deserialize)]
struct PlageQuery {
    du: NaiveDate,
    au: NaiveDate,
    classe_id: Option<Uuid>,
    prof_id: Option<Uuid>,
}

async fn calendrier(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<PlageQuery>,
) -> Res<Json<Vec<JourCalendrier>>> {
    Ok(Json(
        s.ent
            .ref_calendrier(&acteur(&session), q.du, q.au)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct JoursBody {
    du: NaiveDate,
    au: NaiveDate,
    kind: String,
    libelle: String,
    site_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct AjoutesDto {
    ajoutes: i64,
}

async fn ajouter_jours(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<JoursBody>,
) -> Res<Json<AjoutesDto>> {
    let ajoutes = s
        .ent
        .ref_ajouter_jours(&acteur(&session), b.du, b.au, &b.kind, &b.libelle, b.site_id)
        .await
        .map_err(map_db)?;
    Ok(Json(AjoutesDto { ajoutes }))
}

#[derive(Debug, Deserialize)]
struct RetirerQuery {
    jour: NaiveDate,
    site_id: Option<Uuid>,
}

async fn retirer_jour(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<RetirerQuery>,
) -> Res<StatusCode> {
    s.ent
        .ref_retirer_jour(&acteur(&session), q.jour, q.site_id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct VacancesBody {
    libelle: String,
    kind: String,
    debut: NaiveDate,
    fin: NaiveDate,
}

async fn creer_vacances(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<VacancesBody>,
) -> Res<(StatusCode, Json<Id>)> {
    let id = s
        .ent
        .ref_creer_vacances(&acteur(&session), &b.libelle, &b.kind, b.debut, b.fin)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(Id { id })))
}

// ───── Grille, séances, EDT-03 ─────

#[derive(Debug, Deserialize)]
struct SiteQuery {
    site_id: Option<Uuid>,
}

async fn grille(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<SiteQuery>,
) -> Res<Json<Vec<CreneauGrille>>> {
    Ok(Json(
        s.ent.ref_grille(&acteur(&session), q.site_id).await.map_err(map_db)?,
    ))
}

async fn definir_grille(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<SiteQuery>,
    Json(b): Json<Vec<CreneauGrilleSaisie>>,
) -> Res<Json<Vec<CreneauGrille>>> {
    Ok(Json(
        s.ent
            .ref_definir_grille(&acteur(&session), q.site_id, &b)
            .await
            .map_err(map_db)?,
    ))
}

async fn amorcer_grille(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<CreneauGrille>>> {
    Ok(Json(s.ent.ref_amorcer_grille(&acteur(&session)).await.map_err(map_db)?))
}

async fn seances(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<PlageQuery>,
) -> Res<Json<Vec<Seance>>> {
    Ok(Json(
        s.ent
            .ref_seances(&acteur(&session), q.du, q.au, q.classe_id, q.prof_id)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct LierServiceBody {
    service_id: Option<Uuid>,
}

async fn lier_service(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<LierServiceBody>,
) -> Res<StatusCode> {
    s.ent
        .ref_lier_creneau_service(&acteur(&session), id, b.service_id)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

// ───── classe_id ─────

async fn comparaison(
    State(s): State<AppState>,
    session: EntSession,
) -> Res<Json<Vec<ComparaisonClasse>>> {
    Ok(Json(
        s.ent.ref_comparaison_classe_id(&acteur(&session)).await.map_err(map_db)?,
    ))
}

#[derive(Debug, Serialize)]
struct RattacheDto {
    table: String,
    rattachees: i64,
}

async fn rattacher(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<RattacheDto>>> {
    let r = s.ent.ref_rattacher_classes(&acteur(&session)).await.map_err(map_db)?;
    Ok(Json(
        r.into_iter()
            .map(|(table, rattachees)| RattacheDto { table, rattachees })
            .collect(),
    ))
}
