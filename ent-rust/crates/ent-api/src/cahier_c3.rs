//! Routes `/cahier/c3/*` du lot C3 du socle 20 : agenda, « ce que j'ai manqué », coche « fait », résumé hebdomadaire,
//! abonnement au calendrier. Lecteurs : l'élève lui-même ou un responsable habilité (`ent-db` relit les droits du lien à
//! chaque appel). L'établissement vient de la session, jamais de la requête.
//!
//! Une seule route est publique : `GET /cahier/c3/calendrier/{jeton}` (un logiciel de calendrier n'envoie ni cookie ni
//! jeton d'API). Le jeton (244 bits, révocable, stocké haché) porte l'établissement ; les droits du compte abonné sont
//! relus à chaque lecture ; le flux ne contient ni consigne, ni copie, ni note.

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, put};
use axum::{Json, Router};
use chrono::{NaiveDate, Utc};
use ent_types::cahier_c3::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::EntSession;
use crate::ical::{ical_escape, push_folded};
use crate::referentiel::acteur;
use crate::routes::map_db;
use crate::AppState;

type Res<T> = Result<T, (StatusCode, String)>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cahier/c3/agenda", get(agenda))
        .route("/cahier/c3/manque", get(manque))
        .route("/cahier/c3/resume", get(resume))
        .route("/cahier/c3/travaux/{id}/fait", put(cocher))
        .route("/cahier/c3/abonnements", get(abonnements).post(abonnement_creer))
        .route("/cahier/c3/abonnements/{id}", delete(abonnement_revoquer))
        .route("/cahier/c3/calendrier/{jeton}", get(calendrier))
}

#[derive(Debug, Deserialize)]
struct PeriodeQuery {
    eleve_id: Option<Uuid>,
    du: NaiveDate,
    au: NaiveDate,
}

#[derive(Debug, Deserialize)]
struct SemaineQuery {
    eleve_id: Option<Uuid>,
    semaine: Option<NaiveDate>,
}

async fn agenda(State(s): State<AppState>, session: EntSession, Query(q): Query<PeriodeQuery>) -> Res<Json<AgendaDto>> {
    Ok(Json(s.ent.cdt_agenda(&acteur(&session), q.eleve_id, q.du, q.au).await.map_err(map_db)?))
}

async fn manque(State(s): State<AppState>, session: EntSession, Query(q): Query<PeriodeQuery>) -> Res<Json<ManqueDto>> {
    Ok(Json(s.ent.cdt_manque(&acteur(&session), q.eleve_id, q.du, q.au).await.map_err(map_db)?))
}

async fn resume(
    State(s): State<AppState>,
    session: EntSession,
    Query(q): Query<SemaineQuery>,
) -> Res<Json<ResumeSemaineDto>> {
    Ok(Json(s.ent.cdt_resume_semaine(&acteur(&session), q.eleve_id, q.semaine).await.map_err(map_db)?))
}

async fn cocher(
    State(s): State<AppState>,
    session: EntSession,
    Path(id): Path<Uuid>,
    Json(b): Json<CocheBody>,
) -> Res<StatusCode> {
    s.ent.cdt_cocher(&acteur(&session), id, b.fait).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn abonnements(State(s): State<AppState>, session: EntSession) -> Res<Json<Vec<AbonnementDto>>> {
    Ok(Json(s.ent.cdt_abonnements(&acteur(&session)).await.map_err(map_db)?))
}

async fn abonnement_creer(
    State(s): State<AppState>,
    session: EntSession,
    Json(b): Json<AbonnementBody>,
) -> Res<(StatusCode, Json<AbonnementCreeDto>)> {
    Ok((
        StatusCode::CREATED,
        Json(s.ent.cdt_abonnement_creer(&acteur(&session), b.eleve_id).await.map_err(map_db)?),
    ))
}

async fn abonnement_revoquer(State(s): State<AppState>, session: EntSession, Path(id): Path<Uuid>) -> Res<StatusCode> {
    s.ent.cdt_abonnement_revoquer(&acteur(&session), id).await.map_err(map_db)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Flux iCalendar : jours entiers (`VALUE=DATE`), aucun fuseau à convertir, aucune consigne. Réponse **non mise en cache**
/// (un abonnement révoqué doit cesser aussitôt) et jamais indexable.
async fn calendrier(State(s): State<AppState>, Path(jeton): Path<String>) -> Response {
    match s.ent.cdt_calendrier_par_jeton(&jeton).await {
        Ok((nom, lignes)) => {
            let corps = rendre_calendrier(&nom, &lignes, Utc::now());
            let mut r = corps.into_response();
            let h = r.headers_mut();
            h.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/calendar; charset=utf-8"));
            h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            h.insert("x-robots-tag", HeaderValue::from_static("noindex"));
            r
        }
        // Jeton inconnu, révoqué, mal formé ou droits retirés : la même réponse, sans rien dire de plus.
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Calendrier VCALENDAR de l'agenda : un événement « journée entière » par ligne.
pub fn rendre_calendrier(nom: &str, lignes: &[AgendaLigneDto], maintenant: chrono::DateTime<Utc>) -> String {
    let mut buf = String::with_capacity(lignes.len() * 200 + 256);
    push_folded(&mut buf, "BEGIN:VCALENDAR");
    push_folded(&mut buf, "VERSION:2.0");
    push_folded(&mut buf, "PRODID:-//ScolIA//ENT Cahier de textes//FR");
    push_folded(&mut buf, "CALSCALE:GREGORIAN");
    push_folded(&mut buf, "METHOD:PUBLISH");
    push_folded(&mut buf, &format!("X-WR-CALNAME:{}", ical_escape(nom)));
    let stamp = maintenant.format("%Y%m%dT%H%M%SZ").to_string();
    for l in lignes {
        let resume = match l.genre.as_str() {
            GENRE_EVALUATION => format!("Évaluation — {} : {}", l.matiere, l.titre),
            GENRE_EPREUVE => format!("Épreuve — {} : {}", l.matiere, l.titre),
            GENRE_SORTIE => format!("Sortie — {}", l.titre),
            _ => format!("{} — {}", l.matiere, l.titre),
        };
        let fin = l.jour.succ_opt().unwrap_or(l.jour);
        push_folded(&mut buf, "BEGIN:VEVENT");
        push_folded(&mut buf, &format!("UID:{}-{}@scolia-cahier", l.genre, l.id));
        push_folded(&mut buf, &format!("DTSTAMP:{stamp}"));
        push_folded(&mut buf, &format!("DTSTART;VALUE=DATE:{}", l.jour.format("%Y%m%d")));
        push_folded(&mut buf, &format!("DTEND;VALUE=DATE:{}", fin.format("%Y%m%d")));
        push_folded(&mut buf, &format!("SUMMARY:{}", ical_escape(&resume)));
        if let Some(h) = &l.heure {
            push_folded(&mut buf, &format!("DESCRIPTION:{}", ical_escape(&format!("À rendre pour {h}"))));
        }
        push_folded(&mut buf, "END:VEVENT");
    }
    push_folded(&mut buf, "END:VCALENDAR");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Barre oblique inverse d'échappement iCalendar.
    const BS: char = '\\';

    fn ligne(genre: &str, titre: &str) -> AgendaLigneDto {
        AgendaLigneDto {
            genre: genre.into(),
            id: Uuid::nil(),
            cours_id: None,
            matiere: "Maths".into(),
            titre: titre.into(),
            nature: Some("Exercices".into()),
            consigne: Some("CONSIGNE SECRETE".into()),
            jour: NaiveDate::from_ymd_opt(2026, 11, 20).unwrap(),
            heure: Some("08:00".into()),
            duree_estimee_min: Some(30),
            rendu: None,
            auteur: Some("Mme Dupont".into()),
            statut: None,
            fait: Some(true),
            echeance_prolongee: false,
            modifie_le: None,
            version: 1,
        }
    }

    #[test]
    fn flux_sans_consigne_ni_coche_et_avec_echappement() {
        let t = chrono::DateTime::from_timestamp(0, 0).unwrap();
        let mut l = ligne(GENRE_TRAVAIL, "Fiche; n°3, partie 2");
        l.consigne = None; // le serveur retire la consigne avant le rendu
        let ics = rendre_calendrier("Agenda", &[l, ligne(GENRE_EVALUATION, "Contrôle")], t);
        assert!(ics.contains("DTSTART;VALUE=DATE:20261120\r\n"));
        assert!(ics.contains("DTEND;VALUE=DATE:20261121\r\n"));
        assert!(ics.contains(&format!("SUMMARY:Maths — Fiche{BS}; n°3{BS}, partie 2\r\n")));
        assert!(ics.contains("Évaluation — Maths : Contrôle"));
        assert!(!ics.contains("SECRETE")); // le rendu n'écrit jamais la consigne
        assert!(!ics.contains("Dupont"));
        assert_eq!(ics.matches("BEGIN:VEVENT").count(), 2);
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
    }
}
