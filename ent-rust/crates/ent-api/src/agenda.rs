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
