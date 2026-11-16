//! Routes `/cahier/c5/*` du lot C5 du socle 20 : rendu et retour (`20_CAHIER_DE_TEXTES.md` §10).
//!
//! Élève et famille :
//! * `GET /cahier/c5/travaux/{id}/remise` : la situation (dépôts, accusés, brouillon, retour, délai) ; `?eleve_id=` pour un
//!   responsable habilité (il ne lit jamais les dépôts, R-C37).
//! * `PUT .../brouillon`, `DELETE .../brouillon/pieces/{id}`, `POST .../depot` : brouillon, dépôt immuable avec accusé ;
//!   l'heure retenue est celle de **début** de réception, prise à l'entrée de la requête (R-C10).
//! * `POST .../signalement` (« je n'arrive pas à déposer »), `POST .../delai` (demande de délai).
//! * `GET .../groupes`, `POST .../groupes/rejoindre` : inscription libre dans les groupes.
//!
//! Professeur (intervenant en saisie à la date) :
//! * `GET .../suivi`, `POST .../constats`, `POST .../prolongations`, `PUT .../cloture`, `POST .../depot-au-nom`,
//!   `PUT .../groupes`, `GET .../copies`, `POST .../relance`, `POST .../retours/lot`, `GET .../delais`.
//! * `PUT /cahier/c5/remises/{id}/retour`, `POST .../rouvrir`, `POST .../signalement-copie`.
//! * `POST /cahier/c5/delais/{id}/decision`, `POST /cahier/c5/pieces/{id}/lien` (lien court, droits relus).
//! * `POST /cahier/c5/pannes` : déclaration d'une panne (direction, `cahier.parametrer`).
//!
//! L'établissement vient de la session, jamais de la requête ; les fichiers sont décodés ici, contrôlés par le socle 7.

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use base64::Engine as _;
use chrono::Utc;
use ent_db::cahier::FichierDepot;
use ent_db::cahier_c5::DepotEntree;
use ent_types::cahier_c5::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

/// Fichiers au plus portés par un même corps de requête (le plafond de chaque fichier reste celui du socle 7).
const FICHIERS_PAR_CORPS: usize = 4;

pub fn routes() -> Router<AppState> {
    let up = ent_db::upload::PolitiqueFichiers::depuis_env();
    let gros = DefaultBodyLimit::max(up.limite_corps_base64(ent_db::upload::UsageFichier::Copie).saturating_mul(FICHIERS_PAR_CORPS));
    Router::new()
        .route("/cahier/c5/travaux/{id}/remise", get(remise))
        .route("/cahier/c5/travaux/{id}/brouillon", put(brouillon).layer(gros))
        .route("/cahier/c5/travaux/{id}/brouillon/pieces/{piece_id}", delete(brouillon_piece))
        .route("/cahier/c5/travaux/{id}/depot", post(deposer).layer(gros))
        .route("/cahier/c5/travaux/{id}/depot-au-nom", post(depot_au_nom).layer(gros))
        .route("/cahier/c5/travaux/{id}/signalement", post(signaler))
        .route("/cahier/c5/travaux/{id}/delai", post(demander_delai))
        .route("/cahier/c5/travaux/{id}/delais", get(delais_attente))
        .route("/cahier/c5/travaux/{id}/groupes", get(groupes).put(composer))
        .route("/cahier/c5/travaux/{id}/groupes/rejoindre", post(rejoindre))
        .route("/cahier/c5/travaux/{id}/suivi", get(suivi))
        .route("/cahier/c5/travaux/{id}/constats", post(constater))
        .route("/cahier/c5/travaux/{id}/prolongations", post(prolonger))
        .route("/cahier/c5/travaux/{id}/cloture", put(cloture))
        .route("/cahier/c5/travaux/{id}/copies", get(copies))
        .route("/cahier/c5/travaux/{id}/relance", post(relancer))
        .route("/cahier/c5/travaux/{id}/retours/lot", post(rendre_en_lot))
        .route("/cahier/c5/remises/{id}/retour", get(retour_lire).put(retour_enregistrer).layer(gros))
        .route("/cahier/c5/remises/{id}/rouvrir", post(rouvrir))
        .route("/cahier/c5/remises/{id}/signalement-copie", post(signaler_copie))
        .route("/cahier/c5/delais/{id}/decision", post(decider))
        .route("/cahier/c5/pieces/{id}/lien", post(lien_piece))
        .route("/cahier/c5/pannes", post(panne))
}

fn decoder(fichiers: Vec<FichierDepotBody>) -> Res<Vec<FichierDepot>> {
    fichiers
        .into_iter()
        .map(|f| {
            let contenu = base64::engine::general_purpose::STANDARD
                .decode(f.contenu_base64.trim())
                .map_err(|_| (StatusCode::BAD_REQUEST, "contenu_base64 invalide".to_string()))?;
            Ok(FichierDepot { nom: f.nom, mime_type: f.mime_type, contenu })
        })
        .collect()
}

fn entree(b: DepotBody) -> Res<DepotEntree> {
    Ok(DepotEntree { texte: b.texte, commentaire: b.commentaire, fichiers: decoder(b.fichiers)? })
}

#[derive(Debug, Deserialize)]
struct EleveQuery {
    eleve_id: Option<Uuid>,
}

async fn remise(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Query(q): Query<EleveQuery>) -> Res<Json<RemiseLueDto>> {
    Ok(Json(s.ent.cdt_remise_lire(&acteur(&session), id, q.eleve_id).await.map_err(map_db)?))
}

async fn brouillon(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<DepotBody>) -> Res<Json<RemiseLueDto>> {
    Ok(Json(s.ent.cdt_brouillon_enregistrer(&acteur(&session), id, entree(b)?).await.map_err(map_db)?))
}

async fn brouillon_piece(State(s): State<AppState>, session: EntSession, Path((id, piece_id)): Path<(Uuid, Uuid)>) -> Res<Json<RemiseLueDto>> {
    Ok(Json(s.ent.cdt_brouillon_retirer_piece(&acteur(&session), id, piece_id).await.map_err(map_db)?))
}

async fn deposer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<DepotBody>) -> Res<(StatusCode, Json<AccuseDto>)> {
    // Heure de début de réception : prise ici, avant tout traitement du corps (R-C10).
    let recu = Utc::now();
    let accuse = s.ent.cdt_deposer(&acteur(&session), id, entree(b)?, recu).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(accuse)))
}

async fn depot_au_nom(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<DepotAuNomBody>) -> Res<(StatusCode, Json<AccuseDto>)> {
    let recu = Utc::now();
    let e = DepotEntree { texte: b.texte, commentaire: None, fichiers: decoder(b.fichiers)? };
    let accuse = s.ent.cdt_deposer_au_nom(&acteur(&session), id, b.eleve_id, e, recu).await.map_err(map_db)?;
    Ok((StatusCode::CREATED, Json(accuse)))
}

async fn signaler(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<SignalementBody>) -> Res<Json<chrono::DateTime<Utc>>> {
    Ok(Json(s.ent.cdt_depot_signaler(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn demander_delai(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Query(q): Query<EleveQuery>, Json(b): Json<DemandeDelaiBody>) -> Res<Json<DelaiLuDto>> {
    Ok(Json(s.ent.cdt_delai_demander(&acteur(&session), id, q.eleve_id, b).await.map_err(map_db)?))
}

async fn delais_attente(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<DelaiAttenteDto>>> {
    Ok(Json(s.ent.cdt_delais_attente(&acteur(&session), id).await.map_err(map_db)?))
}

async fn groupes(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<GroupeDto>>> {
    Ok(Json(s.ent.cdt_groupes_actifs(&acteur(&session), id).await.map_err(map_db)?))
}

async fn composer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<GroupesBody>) -> Res<Json<Vec<GroupeDto>>> {
    Ok(Json(s.ent.cdt_groupes_composer(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn rejoindre(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<GroupeLibreBody>) -> Res<Json<GroupeDto>> {
    Ok(Json(s.ent.cdt_groupe_libre(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn suivi(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<SuiviC5Dto>> {
    Ok(Json(s.ent.cdt_suivi(&acteur(&session), id).await.map_err(map_db)?))
}

async fn constater(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<ConstatsBody>) -> Res<Json<usize>> {
    Ok(Json(s.ent.cdt_constater(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn prolonger(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<ProlongationBody>) -> Res<Json<usize>> {
    Ok(Json(s.ent.cdt_prolonger(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn cloture(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<ClotureBody>) -> Res<Json<chrono::DateTime<Utc>>> {
    Ok(Json(s.ent.cdt_cloture_fixer(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn copies(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Vec<CopieATelechargerDto>>> {
    Ok(Json(s.ent.cdt_copies_a_telecharger(&acteur(&session), id).await.map_err(map_db)?))
}

async fn relancer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<RelanceBody>) -> Res<Json<RelanceDto>> {
    Ok(Json(s.ent.cdt_relancer(&acteur(&session), id, b).await.map_err(map_db)?))
}

async fn rendre_en_lot(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<LotRenduDto>> {
    let n = s.ent.cdt_retours_rendre_en_lot(&acteur(&session), id).await.map_err(map_db)?;
    Ok(Json(LotRenduDto { rendus: n as i32 }))
}

async fn retour_lire(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<Json<Option<RetourProfDto>>> {
    Ok(Json(s.ent.cdt_retour_lire_prof(&acteur(&session), id).await.map_err(map_db)?))
}

async fn retour_enregistrer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(mut b): Json<RetourC5Body>) -> Res<Json<RetourProfDto>> {
    let fichiers = decoder(std::mem::take(&mut b.fichiers))?;
    Ok(Json(s.ent.cdt_retour_enregistrer(&acteur(&session), id, b, fichiers).await.map_err(map_db)?))
}

async fn rouvrir(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<RouvrirBody>) -> Res<StatusCode> {
    s.ent.cdt_remise_rouvrir(&acteur(&session), id, b).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn signaler_copie(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<SignalementCopieBody>) -> Res<StatusCode> {
    s.ent.cdt_copie_signaler(&acteur(&session), id, b).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn decider(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, Json(b): Json<DelaiDecisionBody>) -> Res<StatusCode> {
    s.ent.cdt_delai_decider(&acteur(&session), id, b).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Default, Deserialize)]
struct MotifBody {
    #[serde(default)]
    motif: Option<String>,
}

async fn lien_piece(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>, body: Option<Json<MotifBody>>) -> Res<Json<LienPieceDto>> {
    let motif = body.and_then(|Json(b)| b.motif);
    let l = s.ent.cdt_piece_lien(&acteur(&session), id, motif.as_deref()).await.map_err(map_db)?;
    Ok(Json(LienPieceDto { url: l.url, expires_at: l.expires_at }))
}

async fn panne(State(s): State<AppState>, session: EntSession, Json(b): Json<PanneBody>) -> Res<Json<PanneDto>> {
    Ok(Json(s.ent.cdt_panne_declarer(&acteur(&session), b).await.map_err(map_db)?))
}
