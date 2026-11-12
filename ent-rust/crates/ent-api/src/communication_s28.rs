//! Routes du lot S2-8 : notifications unifiées, campagnes, rendez-vous et
//! rencontres parents-professeurs, formulaires et consentements.
//!
//! L'autorité est `ent-db` (moteur de droits D-017 : `campagnes.lancer`,
//! `rencontres.gerer`, `formulaires.gerer`, `consentements.lire`,
//! `notifications.gerer`) ; côté famille, l'accès passe par le **lien actif**.
//! Ces routes ne font que traduire HTTP ↔ service. Une référence d'un autre
//! établissement ou d'une autre famille répond 404.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use ent_db::campagnes::{
    ApercuCampagne, Campagne, ConfirmationLancement, NouvelleCampagne,
};
use ent_db::formulaires::{
    ConsentementEleve, EtatConsentement, Formulaire, FormulaireFamille, NouveauFormulaire,
    ReponseFormulaire, ResultatsFormulaire,
};
use ent_db::notifications::{EnvoiVu, Plafonds, Preferences};
use ent_db::rencontres::{
    EnseignantCreneaux, LignePlanning, NouveauxCreneaux, NouvelleRencontre, Rencontre,
    RencontreFamille,
};
use ent_db::PgEnt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Notifications (D0)
        .route(
            "/notifications/preferences",
            get(lire_preferences).put(definir_preferences),
        )
        .route("/notifications/envois", get(mes_envois))
        .route("/notifications/plafonds", get(lire_plafonds).put(definir_plafonds))
        // Campagnes (COM-05)
        .route("/campagnes", get(lister_campagnes).post(creer_campagne))
        .route("/campagnes/{id}", get(lire_campagne))
        .route("/campagnes/{id}/apercu", post(apercu_campagne))
        .route("/campagnes/{id}/lancer", post(lancer_campagne))
        .route("/campagnes/{id}/annuler", post(annuler_campagne))
        .route("/campagnes/{id}/bilan", get(bilan_campagne))
        // Rencontres parents-professeurs (AGD-02, AGD-03)
        .route("/rencontres", get(lister_rencontres).post(creer_rencontre))
        .route("/rencontres/mes-creneaux", get(mes_creneaux))
        .route("/rencontres/classes/{classe_id}/enseignants", get(enseignants_classe))
        .route("/rencontres/{id}/planning", get(planning))
        .route("/rencontres/{id}/creneaux", post(ajouter_creneaux))
        .route("/rencontres/{id}/ouvrir", post(ouvrir_rencontre))
        .route("/rencontres/{id}/cloturer", post(cloturer_rencontre))
        .route("/rencontres/creneaux/{id}/reserver", post(reserver))
        .route("/rencontres/creneaux/{id}/annuler", post(annuler_reservation))
        .route("/rencontres/creneaux/{id}/bloquer", post(bloquer_creneau))
        .route("/rencontres/creneaux/{id}/presence", post(presence_rdv))
        .route("/rencontres/creneaux/{id}/fil", get(fil_creneau))
        .route("/famille/rencontres", get(rencontres_famille))
        .route(
            "/famille/rencontres/{id}/enfants/{eleve_id}/creneaux",
            get(creneaux_famille),
        )
        // Formulaires et consentements (FRM-01→05)
        .route("/formulaires", get(lister_formulaires).post(creer_formulaire))
        .route("/formulaires/mes", get(formulaires_famille))
        .route("/formulaires/{id}/publier", post(publier_formulaire))
        .route("/formulaires/{id}/archiver", post(archiver_formulaire))
        .route("/formulaires/{id}/resultats", get(resultats_formulaire))
        .route("/formulaires/{id}/repondre", post(repondre_formulaire))
        .route(
            "/consentements/{objet}/classes/{classe_id}",
            get(consentements_classe),
        )
}

fn ent(state: &AppState) -> &PgEnt {
    &state.ent
}

// ----- Notifications ---------------------------------------------------------

async fn lire_preferences(State(s): State<AppState>, se: EntSession) -> Res<Json<Preferences>> {
    ent(&s)
        .mes_preferences_notification(se.etablissement_id, se.user_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn definir_preferences(
    State(s): State<AppState>,
    se: EntSession,
    Json(p): Json<Preferences>,
) -> Res<Json<Preferences>> {
    ent(&s)
        .definir_preferences_notification(se.etablissement_id, se.user_id, p)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn mes_envois(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<EnvoiVu>>> {
    ent(&s)
        .mes_envois_notification(se.etablissement_id, se.user_id, 100)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn lire_plafonds(State(s): State<AppState>, se: EntSession) -> Res<Json<Plafonds>> {
    ent(&s)
        .plafonds_notification_for_actor(se.etablissement_id, se.user_id, &se.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn definir_plafonds(
    State(s): State<AppState>,
    se: EntSession,
    Json(p): Json<Plafonds>,
) -> Res<Json<Plafonds>> {
    ent(&s)
        .definir_plafonds_notification(se.etablissement_id, se.user_id, &se.role, p)
        .await
        .map(Json)
        .map_err(map_db)
}

// ----- Campagnes -------------------------------------------------------------

async fn lister_campagnes(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<Campagne>>> {
    ent(&s)
        .campagnes_for_actor(se.etablissement_id, se.user_id, &se.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creer_campagne(
    State(s): State<AppState>,
    se: EntSession,
    Json(n): Json<NouvelleCampagne>,
) -> Res<(StatusCode, Json<Campagne>)> {
    ent(&s)
        .creer_campagne(se.etablissement_id, se.user_id, &se.role, n)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(map_db)
}

async fn lire_campagne(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Campagne>> {
    ent(&s)
        .campagne_for_actor(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn apercu_campagne(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ApercuCampagne>> {
    ent(&s)
        .apercu_campagne(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn lancer_campagne(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(c): Json<ConfirmationLancement>,
) -> Res<Json<Campagne>> {
    ent(&s)
        .lancer_campagne(se.etablissement_id, se.user_id, &se.role, id, c)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn annuler_campagne(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Campagne>> {
    ent(&s)
        .annuler_campagne(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Serialize)]
struct LigneBilan {
    canal: String,
    statut: String,
    nombre: i64,
}

async fn bilan_campagne(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<LigneBilan>>> {
    // Visibilité : auteur ou `notifications.gerer` (même 404 sinon).
    ent(&s)
        .campagne_for_actor(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map_err(map_db)?;
    ent(&s)
        .bilan_envois_source(se.etablissement_id, "campagne", id)
        .await
        .map(|v| {
            Json(
                v.into_iter()
                    .map(|(canal, statut, nombre)| LigneBilan { canal, statut, nombre })
                    .collect(),
            )
        })
        .map_err(map_db)
}

// ----- Rencontres ------------------------------------------------------------

async fn lister_rencontres(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<Rencontre>>> {
    ent(&s)
        .rencontres_gestion(se.etablissement_id, se.user_id, &se.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creer_rencontre(
    State(s): State<AppState>,
    se: EntSession,
    Json(n): Json<NouvelleRencontre>,
) -> Res<(StatusCode, Json<Rencontre>)> {
    ent(&s)
        .creer_rencontre(se.etablissement_id, se.user_id, &se.role, n)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(map_db)
}

#[derive(Serialize)]
struct EnseignantClasse {
    user_id: Uuid,
    nom: String,
    matieres: String,
}

async fn enseignants_classe(
    State(s): State<AppState>,
    se: EntSession,
    Path(classe_id): Path<Uuid>,
) -> Res<Json<Vec<EnseignantClasse>>> {
    ent(&s)
        .enseignants_de_classe(se.etablissement_id, se.user_id, &se.role, classe_id)
        .await
        .map(|v| {
            Json(
                v.into_iter()
                    .map(|(user_id, nom, matieres)| EnseignantClasse { user_id, nom, matieres })
                    .collect(),
            )
        })
        .map_err(map_db)
}

async fn planning(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<LignePlanning>>> {
    ent(&s)
        .planning_rencontre(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Serialize)]
struct Nombre {
    crees: usize,
}

async fn ajouter_creneaux(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(n): Json<NouveauxCreneaux>,
) -> Res<(StatusCode, Json<Nombre>)> {
    ent(&s)
        .ajouter_creneaux(se.etablissement_id, se.user_id, &se.role, id, n)
        .await
        .map(|crees| (StatusCode::CREATED, Json(Nombre { crees })))
        .map_err(map_db)
}

async fn ouvrir_rencontre(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Rencontre>> {
    ent(&s)
        .ouvrir_rencontre(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn cloturer_rencontre(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Rencontre>> {
    ent(&s)
        .cloturer_rencontre(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn mes_creneaux(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<LignePlanning>>> {
    ent(&s)
        .mes_creneaux_enseignant(se.etablissement_id, se.user_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn rencontres_famille(
    State(s): State<AppState>,
    se: EntSession,
) -> Res<Json<Vec<RencontreFamille>>> {
    ent(&s)
        .rencontres_famille(se.etablissement_id, se.user_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creneaux_famille(
    State(s): State<AppState>,
    se: EntSession,
    Path((id, eleve_id)): Path<(Uuid, Uuid)>,
) -> Res<Json<Vec<EnseignantCreneaux>>> {
    ent(&s)
        .creneaux_famille(se.etablissement_id, se.user_id, id, eleve_id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct ReserverBody {
    eleve_id: Uuid,
}

async fn reserver(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<ReserverBody>,
) -> Res<Json<LignePlanning>> {
    ent(&s)
        .reserver_creneau(se.etablissement_id, se.user_id, id, b.eleve_id)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize, Default)]
struct AnnulerBody {
    /// Motif public : obligatoire pour l'organisateur (RD-09), ignoré pour une famille.
    #[serde(default)]
    motif: Option<String>,
}

async fn annuler_reservation(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    body: Option<Json<AnnulerBody>>,
) -> Res<Json<LignePlanning>> {
    let motif = body.and_then(|Json(b)| b.motif);
    ent(&s)
        .annuler_reservation(se.etablissement_id, se.user_id, &se.role, id, motif.as_deref())
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct BloquerBody {
    bloquer: bool,
}

/// RD-10 : l'enseignant du créneau bloque ou débloque son créneau.
async fn bloquer_creneau(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<BloquerBody>,
) -> Res<Json<LignePlanning>> {
    ent(&s)
        .bloquer_creneau(se.etablissement_id, se.user_id, &se.role, id, b.bloquer)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct PresenceBody {
    presence: String,
}

/// RD-10 : présence (honoré / non venu) saisie par l'enseignant du créneau.
async fn presence_rdv(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<PresenceBody>,
) -> Res<Json<LignePlanning>> {
    ent(&s)
        .marquer_presence_rdv(se.etablissement_id, se.user_id, id, &b.presence)
        .await
        .map(Json)
        .map_err(map_db)
}

/// RD-01 : fil des faits d'un créneau (organisateur ou enseignant du créneau).
async fn fil_creneau(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<ent_db::rencontres::FaitCreneau>>> {
    ent(&s)
        .fil_creneau(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

// ----- Formulaires et consentements -------------------------------------------

async fn lister_formulaires(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<Formulaire>>> {
    ent(&s)
        .formulaires_gestion(se.etablissement_id, se.user_id, &se.role)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn creer_formulaire(
    State(s): State<AppState>,
    se: EntSession,
    Json(n): Json<NouveauFormulaire>,
) -> Res<(StatusCode, Json<Formulaire>)> {
    ent(&s)
        .creer_formulaire(se.etablissement_id, se.user_id, &se.role, n)
        .await
        .map(|f| (StatusCode::CREATED, Json(f)))
        .map_err(map_db)
}

async fn publier_formulaire(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Formulaire>> {
    ent(&s)
        .publier_formulaire(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn archiver_formulaire(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Formulaire>> {
    ent(&s)
        .archiver_formulaire(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn resultats_formulaire(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<ResultatsFormulaire>> {
    ent(&s)
        .resultats_formulaire(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn formulaires_famille(
    State(s): State<AppState>,
    se: EntSession,
) -> Res<Json<Vec<FormulaireFamille>>> {
    ent(&s)
        .formulaires_famille(se.etablissement_id, se.user_id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn repondre_formulaire(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(r): Json<ReponseFormulaire>,
) -> Res<Json<EtatConsentement>> {
    ent(&s)
        .repondre_formulaire(se.etablissement_id, se.user_id, id, r)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn consentements_classe(
    State(s): State<AppState>,
    se: EntSession,
    Path((objet, classe_id)): Path<(String, Uuid)>,
) -> Res<Json<Vec<ConsentementEleve>>> {
    ent(&s)
        .consentements_classe_for_actor(se.etablissement_id, se.user_id, &se.role, &objet, classe_id)
        .await
        .map(Json)
        .map_err(map_db)
}
