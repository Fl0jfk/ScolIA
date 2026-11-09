//! Routes `/structure/calendrier/*`, `/structure/semaines/*` et `/structure/fermetures/*` du
//! socle 5, lot S3 : calendrier de l'année, semaines A/B, jours cibles, fermetures exceptionnelles.
//!
//! Aucune règle ici : droits (`referentiel.lire` / `referentiel.gerer`, fermetures réservées à la
//! direction et à l'administrateur), cloisonnement (établissement de la session, jamais du corps),
//! validations, journal et événements sont dans `ent-db`. La déclaration d'une fermeture et
//! l'adoption du calendrier officiel sont des **aperçus par défaut** : il faut `apercu: false`
//! pour écrire.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_db::structure_calendrier::{
    modele_saisie, Cible, Comparaison, Evenement, EvenementModif, EvenementSaisie,
    FermetureModif, FermetureSaisie, HistoriqueFermeture, Jour, JourService, ModeleSaisie,
    RapportAdoption, RegleSemaineSaisie, ResultatEvenement, ResultatFermeture, SemainesVue,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/structure/calendrier/evenements",
            get(evenements).post(evenement_creer),
        )
        .route(
            "/structure/calendrier/evenements/{id}",
            get(evenement).put(evenement_modifier),
        )
        .route(
            "/structure/calendrier/evenements/{id}/annuler",
            post(evenement_annuler),
        )
        .route(
            "/structure/calendrier/evenements/{id}/retablir",
            post(evenement_retablir),
        )
        .route("/structure/calendrier/adoption", post(adopter))
        .route("/structure/calendrier/modele-saisie", get(modele))
        .route("/structure/calendrier/jour", get(jour))
        .route("/structure/calendrier/jours", get(jours))
        .route(
            "/structure/calendrier/services/{service}/jour",
            get(jour_service),
        )
        .route("/structure/calendrier/comparaison", get(comparaison))
        .route("/structure/semaines", get(semaines).put(semaines_definir))
        .route("/structure/semaines/forcer", post(semaine_forcer))
        .route("/structure/semaines/liberer", post(semaine_liberer))
        .route("/structure/fermetures", post(fermeture_declarer))
        .route("/structure/fermetures/{id}", put(fermeture_modifier))
        .route("/structure/fermetures/{id}/lever", post(fermeture_lever))
        .route(
            "/structure/fermetures/{id}/historique",
            get(fermeture_historique),
        )
}

// ───── Événements ─────

#[derive(Debug, Deserialize)]
struct EvenementsQuery {
    annee_id: Uuid,
    du: Option<NaiveDate>,
    au: Option<NaiveDate>,
    #[serde(default)]
    annules: bool,
}

async fn evenements(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<EvenementsQuery>,
) -> Res<Json<Vec<Evenement>>> {
    let plage = match (q.du, q.au) {
        (Some(d), Some(a)) => Some((d, a)),
        (None, None) => None,
        _ => return Err((StatusCode::BAD_REQUEST, "du et au vont ensemble".into())),
    };
    Ok(Json(
        s.ent
            .structure_calendrier_evenements(&acteur(&session), q.annee_id, plage, q.annules)
            .await
            .map_err(map_db)?,
    ))
}

async fn evenement(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Evenement>> {
    Ok(Json(
        s.ent
            .structure_calendrier_evenement(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

async fn evenement_creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<EvenementSaisie>,
) -> Res<(StatusCode, Json<ResultatEvenement>)> {
    let r = s
        .ent
        .structure_calendrier_evenement_creer(&acteur(&session), b)
        .await
        .map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(r)))
}

async fn evenement_modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<EvenementModif>,
) -> Res<Json<ResultatEvenement>> {
    Ok(Json(
        s.ent
            .structure_calendrier_evenement_modifier(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct MotifBody {
    motif: String,
}

async fn evenement_annuler(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<MotifBody>,
) -> Res<Json<Evenement>> {
    Ok(Json(
        s.ent
            .structure_calendrier_evenement_annuler(&acteur(&session), id, &b.motif)
            .await
            .map_err(map_db)?,
    ))
}

async fn evenement_retablir(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Evenement>> {
    Ok(Json(
        s.ent
            .structure_calendrier_evenement_retablir(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}

// ───── Adoption du calendrier officiel ─────

#[derive(Debug, Deserialize)]
struct AdoptionBody {
    annee_id: Uuid,
    /// Défaut : aperçu (rien n'est écrit).
    #[serde(default = "vrai")]
    apercu: bool,
    #[serde(default)]
    accepter_differences: bool,
}

fn vrai() -> bool {
    true
}

async fn adopter(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<AdoptionBody>,
) -> Res<Json<RapportAdoption>> {
    Ok(Json(
        s.ent
            .structure_calendrier_adopter(
                &acteur(&session),
                b.annee_id,
                b.apercu,
                b.accepter_differences,
            )
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct ModeleQuery {
    zone: Option<String>,
}

/// Modèle de saisie manuelle des vacances (préréglage documenté) : ne lit rien en base.
async fn modele(_session: EntSession, Query(q): Query<ModeleQuery>) -> Json<ModeleSaisie> {
    Json(modele_saisie(q.zone.as_deref()))
}

// ───── Jours ─────

#[derive(Debug, Deserialize)]
struct JourQuery {
    date: NaiveDate,
    site_id: Option<Uuid>,
    niveau_id: Option<Uuid>,
    classe_id: Option<Uuid>,
    groupe_id: Option<Uuid>,
}

impl JourQuery {
    fn cible(&self) -> Cible {
        Cible {
            site_id: self.site_id,
            niveau_id: self.niveau_id,
            classe_id: self.classe_id,
            groupe_id: self.groupe_id,
        }
    }
}

async fn jour(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<JourQuery>,
) -> Res<Json<Jour>> {
    Ok(Json(
        s.ent
            .structure_calendrier_jour(&acteur(&session), q.date, &q.cible())
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct JoursQuery {
    du: NaiveDate,
    au: NaiveDate,
    site_id: Option<Uuid>,
    niveau_id: Option<Uuid>,
    classe_id: Option<Uuid>,
    groupe_id: Option<Uuid>,
}

async fn jours(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<JoursQuery>,
) -> Res<Json<Vec<Jour>>> {
    let cible = Cible {
        site_id: q.site_id,
        niveau_id: q.niveau_id,
        classe_id: q.classe_id,
        groupe_id: q.groupe_id,
    };
    Ok(Json(
        s.ent
            .structure_calendrier_jours(&acteur(&session), q.du, q.au, &cible)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct DateQuery {
    date: NaiveDate,
}

async fn jour_service(
    State(s): State<AppState>,
    session: EntSession,
    Path(service): Path<String>,
    Query(q): Query<DateQuery>,
) -> Res<Json<JourService>> {
    Ok(Json(
        s.ent
            .structure_calendrier_jour_service(&acteur(&session), q.date, &service)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct ComparaisonQuery {
    annee_id: Uuid,
    reference_id: Uuid,
}

async fn comparaison(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<ComparaisonQuery>,
) -> Res<Json<Comparaison>> {
    Ok(Json(
        s.ent
            .structure_calendrier_comparer(&acteur(&session), q.annee_id, q.reference_id)
            .await
            .map_err(map_db)?,
    ))
}

// ───── Semaines A/B ─────

#[derive(Debug, Deserialize)]
struct AnneeQuery {
    annee_id: Uuid,
}

async fn semaines(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<AnneeQuery>,
) -> Res<Json<SemainesVue>> {
    Ok(Json(
        s.ent
            .structure_semaines(&acteur(&session), q.annee_id)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct RegleBody {
    annee_id: Uuid,
    #[serde(flatten)]
    regle: RegleSemaineSaisie,
}

async fn semaines_definir(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<RegleBody>,
) -> Res<Json<SemainesVue>> {
    Ok(Json(
        s.ent
            .structure_semaines_definir(&acteur(&session), b.annee_id, b.regle)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct ForcerBody {
    annee_id: Uuid,
    lundi: NaiveDate,
    /// Absent : semaine sans étiquette.
    etiquette: Option<String>,
    motif: String,
}

async fn semaine_forcer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<ForcerBody>,
) -> Res<Json<SemainesVue>> {
    Ok(Json(
        s.ent
            .structure_semaine_forcer(&acteur(&session), b.annee_id, b.lundi, b.etiquette, &b.motif)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct LibererBody {
    annee_id: Uuid,
    lundi: NaiveDate,
}

async fn semaine_liberer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<LibererBody>,
) -> Res<Json<SemainesVue>> {
    Ok(Json(
        s.ent
            .structure_semaine_liberer(&acteur(&session), b.annee_id, b.lundi)
            .await
            .map_err(map_db)?,
    ))
}

// ───── Fermetures exceptionnelles ─────

#[derive(Debug, Deserialize)]
struct FermetureBody {
    /// Défaut : aperçu (rien n'est écrit, aucun message n'est émis).
    #[serde(default = "vrai")]
    apercu: bool,
    #[serde(flatten)]
    saisie: FermetureSaisie,
}

async fn fermeture_declarer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<FermetureBody>,
) -> Res<(StatusCode, Json<ResultatFermeture>)> {
    let r = s
        .ent
        .structure_fermeture_declarer(&acteur(&session), b.saisie, b.apercu)
        .await
        .map_err(map_db)?;
    let code = if r.ecrit {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((code, Json(r)))
}

async fn fermeture_modifier(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<FermetureModif>,
) -> Res<Json<ResultatFermeture>> {
    Ok(Json(
        s.ent
            .structure_fermeture_modifier(&acteur(&session), id, b)
            .await
            .map_err(map_db)?,
    ))
}

#[derive(Debug, Deserialize)]
struct LeverBody {
    motif: String,
    a_partir_du: Option<NaiveDate>,
}

async fn fermeture_lever(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<LeverBody>,
) -> Res<Json<Evenement>> {
    Ok(Json(
        s.ent
            .structure_fermeture_lever(&acteur(&session), id, &b.motif, b.a_partir_du)
            .await
            .map_err(map_db)?,
    ))
}

async fn fermeture_historique(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<HistoriqueFermeture>>> {
    Ok(Json(
        s.ent
            .structure_fermeture_historique(&acteur(&session), id)
            .await
            .map_err(map_db)?,
    ))
}
