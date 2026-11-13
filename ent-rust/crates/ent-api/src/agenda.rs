//! Routes `/agenda/*` (socle 19, lot AG1) : événements, séries, publication à une audience, réponses,
//! coupon, historique, vue agenda calculée et iCal ponctuel.
//!
//! L'autorité est `ent-db` (`agenda.*` du moteur de droits D-017, propriété de l'événement, place du
//! lecteur R-AG5). Ces routes ne font que traduire HTTP ↔ service ; un événement qu'on n'a pas le droit
//! de voir répond 404, comme un inconnu ou celui d'un autre établissement.

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use ent_types::agenda::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::EntSession;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/agenda", get(vue))
        .route("/agenda/classes/{classe_id}", get(vue_classe))
        .route("/agenda/types", get(types).post(creer_type))
        .route("/agenda/evenements", post(creer_evenement))
        .route("/agenda/series", post(creer_serie))
        .route("/agenda/semaine", post(ajouter_semaine))
        .route(
            "/agenda/evenements/{id}",
            get(lire_evenement).put(modifier_evenement),
        )
        .route(
            "/agenda/evenements/{id}/apercu-publication",
            post(apercu_publication),
        )
        .route("/agenda/evenements/{id}/publier", post(publier))
        .route("/agenda/evenements/{id}/annuler", post(annuler))
        .route("/agenda/evenements/{id}/dupliquer", post(dupliquer))
        .route("/agenda/evenements/{id}/reponse", post(repondre))
        .route("/agenda/evenements/{id}/coupon", post(repondre_coupon))
        .route("/agenda/evenements/{id}/historique", get(historique))
        .route("/agenda/evenements/{id}/ics", get(ics))
        // Lots AG2/AG3 : lieux, réservations, disponibilités.
        .route("/agenda/lieux/recherche", post(lieux_recherche))
        .route("/agenda/lieux/{salle_id}/planning", get(lieux_planning))
        .route(
            "/agenda/lieux/{salle_id}/regle",
            get(lieux_regle).put(lieux_parametrer),
        )
        .route(
            "/agenda/reservations",
            get(reservations_miennes).post(reserver),
        )
        .route(
            "/agenda/reservations/{id}/annuler",
            post(reservation_annuler),
        )
        .route(
            "/agenda/reservations/{id}/decision",
            post(reservation_decider),
        )
        .route("/agenda/disponibilites", post(disponibilites))
        .route("/agenda/creneau-commun", post(creneau_commun))
}

#[derive(Deserialize)]
struct VueQuery {
    du: NaiveDate,
    au: NaiveDate,
    /// Sources séparées par des virgules (`evenements,edt`) ; absent = toutes.
    #[serde(default)]
    sources: Option<String>,
    #[serde(default)]
    eleve_id: Option<Uuid>,
}

async fn vue(
    State(s): State<AppState>,
    se: EntSession,
    Query(q): Query<VueQuery>,
) -> Res<Json<VueAgenda>> {
    let sources: Vec<String> = q
        .sources
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(String::from)
        .collect();
    s.ent
        .agenda_vue(
            se.etablissement_id,
            se.user_id,
            &se.role,
            q.du,
            q.au,
            &sources,
            q.eleve_id,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct PlageQuery {
    du: NaiveDate,
    au: NaiveDate,
}

async fn vue_classe(
    State(s): State<AppState>,
    se: EntSession,
    Path(classe_id): Path<Uuid>,
    Query(q): Query<PlageQuery>,
) -> Res<Json<VueAgenda>> {
    s.ent
        .agenda_classe(
            se.etablissement_id,
            se.user_id,
            &se.role,
            classe_id,
            q.du,
            q.au,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Serialize)]
struct TypeAgenda {
    code: String,
    libelle: String,
    famille: String,
}

async fn types(State(s): State<AppState>, se: EntSession) -> Res<Json<Vec<TypeAgenda>>> {
    s.ent
        .agenda_types(se.etablissement_id)
        .await
        .map(|v| {
            Json(
                v.into_iter()
                    .map(|(code, libelle, famille)| TypeAgenda {
                        code,
                        libelle,
                        famille,
                    })
                    .collect(),
            )
        })
        .map_err(map_db)
}

#[derive(Deserialize)]
struct NouveauType {
    code: String,
    libelle: String,
    famille: String,
}

async fn creer_type(
    State(s): State<AppState>,
    se: EntSession,
    Json(n): Json<NouveauType>,
) -> Res<StatusCode> {
    s.ent
        .agenda_creer_type_local(
            se.etablissement_id,
            se.user_id,
            &se.role,
            &n.code,
            &n.libelle,
            &n.famille,
        )
        .await
        .map(|()| StatusCode::CREATED)
        .map_err(map_db)
}

async fn creer_evenement(
    State(s): State<AppState>,
    se: EntSession,
    Json(n): Json<NouvelEvenement>,
) -> Res<(StatusCode, Json<EvenementCree>)> {
    s.ent
        .agenda_creer_evenement(se.etablissement_id, se.user_id, &se.role, n)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(map_db)
}

/// Feuille de semaine de l'établissement (`agenda.semaine.gerer`).
async fn ajouter_semaine(
    State(s): State<AppState>,
    se: EntSession,
    Json(c): Json<CreneauSemaine>,
) -> Res<(StatusCode, Json<EvenementCree>)> {
    s.ent
        .agenda_semaine_ajouter(se.etablissement_id, se.user_id, &se.role, c)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(map_db)
}

async fn creer_serie(
    State(s): State<AppState>,
    se: EntSession,
    Json(n): Json<NouvelleSerie>,
) -> Res<(StatusCode, Json<EvenementCree>)> {
    s.ent
        .agenda_creer_serie(se.etablissement_id, se.user_id, &se.role, n)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(map_db)
}

async fn lire_evenement(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<EvenementDetail>> {
    s.ent
        .agenda_evenement(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn modifier_evenement(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(m): Json<ModifEvenement>,
) -> Res<Json<EvenementDetail>> {
    s.ent
        .agenda_modifier_evenement(se.etablissement_id, se.user_id, &se.role, id, m)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn apercu_publication(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(mut d): Json<DemandePublication>,
) -> Res<Json<ApercuPublication>> {
    d.apercu_seulement = true;
    match s
        .ent
        .agenda_publier_evenement(se.etablissement_id, se.user_id, &se.role, id, d)
        .await
        .map_err(map_db)?
    {
        Ok(a) => Ok(Json(a)),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne.".into())),
    }
}

async fn publier(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(mut d): Json<DemandePublication>,
) -> Res<Json<EvenementDetail>> {
    d.apercu_seulement = false;
    match s
        .ent
        .agenda_publier_evenement(se.etablissement_id, se.user_id, &se.role, id, d)
        .await
        .map_err(map_db)?
    {
        Err(detail) => Ok(Json(detail)),
        Ok(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne.".into())),
    }
}

async fn annuler(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(a): Json<AnnulationEvenement>,
) -> Res<Json<EvenementDetail>> {
    s.ent
        .agenda_annuler_evenement(se.etablissement_id, se.user_id, &se.role, id, a)
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct DupliquerBody {
    jour: NaiveDate,
}

async fn dupliquer(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<DupliquerBody>,
) -> Res<(StatusCode, Json<EvenementCree>)> {
    s.ent
        .agenda_dupliquer_evenement(se.etablissement_id, se.user_id, &se.role, id, b.jour)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(map_db)
}

async fn repondre(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(r): Json<ReponseInvitation>,
) -> Res<Json<EvenementDetail>> {
    s.ent
        .agenda_repondre(se.etablissement_id, se.user_id, id, r)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn repondre_coupon(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(r): Json<ReponseCoupon>,
) -> Res<Json<EvenementDetail>> {
    s.ent
        .agenda_repondre_coupon(se.etablissement_id, se.user_id, id, r)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn historique(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
) -> Res<Json<Vec<FaitJournal>>> {
    s.ent
        .agenda_historique(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn ics(State(s): State<AppState>, se: EntSession, Path(id): Path<Uuid>) -> Res<Response> {
    let texte = s
        .ent
        .agenda_ics(se.etablissement_id, se.user_id, &se.role, id)
        .await
        .map_err(map_db)?;
    let mut res = texte.into_response();
    let h = res.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/calendar; charset=utf-8"),
    );
    h.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"evenement.ics\""),
    );
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    Ok(res)
}

// ───────────────────────── Lots AG2 / AG3 ─────────────────────────

fn acteur(se: &EntSession) -> ent_db::Acteur<'_> {
    ent_db::Acteur {
        etablissement_id: se.etablissement_id,
        user_id: se.user_id,
        role: &se.role,
    }
}

async fn lieux_recherche(
    State(s): State<AppState>,
    se: EntSession,
    Json(q): Json<RechercheSalle>,
) -> Res<Json<Vec<SalleDispo>>> {
    s.ent
        .agenda_rechercher_salles(&acteur(&se), q)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn lieux_planning(
    State(s): State<AppState>,
    se: EntSession,
    Path(salle): Path<Uuid>,
    Query(q): Query<PlageQuery>,
) -> Res<Json<PlanningSalle>> {
    s.ent
        .agenda_planning_salle(&acteur(&se), salle, q.du, q.au)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn lieux_regle(
    State(s): State<AppState>,
    se: EntSession,
    Path(salle): Path<Uuid>,
) -> Res<Json<RegleLieu>> {
    s.ent
        .agenda_regle_lieu(&acteur(&se), salle)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn lieux_parametrer(
    State(s): State<AppState>,
    se: EntSession,
    Path(salle): Path<Uuid>,
    Json(r): Json<RegleLieu>,
) -> Res<Json<RegleLieu>> {
    s.ent
        .agenda_parametrer_lieu(&acteur(&se), salle, r)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn reservations_miennes(
    State(s): State<AppState>,
    se: EntSession,
    Query(q): Query<PlageQuery>,
) -> Res<Json<Vec<ReservationVue>>> {
    s.ent
        .agenda_mes_reservations(&acteur(&se), q.du, q.au)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn reserver(
    State(s): State<AppState>,
    se: EntSession,
    Json(n): Json<NouvelleReservation>,
) -> Res<Json<RapportReservation>> {
    s.ent
        .agenda_reserver_lieu(&acteur(&se), n)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn reservation_annuler(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(a): Json<AnnulationReservation>,
) -> Res<Json<usize>> {
    s.ent
        .agenda_annuler_reservation(&acteur(&se), id, a)
        .await
        .map(Json)
        .map_err(map_db)
}

async fn reservation_decider(
    State(s): State<AppState>,
    se: EntSession,
    Path(id): Path<Uuid>,
    Json(d): Json<DecisionReservation>,
) -> Res<StatusCode> {
    s.ent
        .agenda_decider_reservation(&acteur(&se), id, d)
        .await
        .map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct DispoBody {
    personnes: Vec<Uuid>,
    jour: NaiveDate,
    heure_debut: chrono::NaiveTime,
    heure_fin: chrono::NaiveTime,
}

async fn disponibilites(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<DispoBody>,
) -> Res<Json<Vec<DisponibilitePersonne>>> {
    s.ent
        .agenda_libre_occupe(
            &acteur(&se),
            &b.personnes,
            b.jour,
            b.heure_debut,
            b.heure_fin,
        )
        .await
        .map(Json)
        .map_err(map_db)
}

#[derive(Deserialize)]
struct CommunBody {
    personnes: Vec<Uuid>,
    du: NaiveDate,
    au: NaiveDate,
    duree_min: i64,
}

async fn creneau_commun(
    State(s): State<AppState>,
    se: EntSession,
    Json(b): Json<CommunBody>,
) -> Res<Json<Vec<PlageCommune>>> {
    s.ent
        .agenda_creneau_commun(&acteur(&se), &b.personnes, b.du, b.au, b.duree_min)
        .await
        .map(Json)
        .map_err(map_db)
}
