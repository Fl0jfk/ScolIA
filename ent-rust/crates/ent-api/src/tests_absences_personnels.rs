//! Socle 15, lots A0–A1 — absences des personnels, côté HTTP (base locale).
//!
//! * 401 sans jeton ; 403 selon la matrice de droits (professeur, accueil, parent) ;
//! * 404 uniforme pour une absence hors du cercle RH (pas d'oracle) ;
//! * la vue « organisation » et le port « non remplacées » ne portent ni nature ni motif ;
//! * circuit externe : la maladie d'un agent de l'État est « prise en compte ».

use super::tests_l3::{appel, etat, inscrire};
use super::*;
use axum::http::StatusCode;
use chrono::Duration;
use serde_json::{json, Value};
use uuid::Uuid;

#[tokio::test]
async fn absences_des_personnels_droits_et_minimisation() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip A15 : ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let prof2 = inscrire(&app, etab, "prof").await;
    let accueil = inscrire(&app, etab, "accueil").await;
    let parent = inscrire(&app, etab, "parent").await;
    let jour = (chrono::Utc::now() + Duration::days(20)).date_naive().to_string();
    let corps = json!({"nature_code": "maladie", "debut": jour, "fin": jour});

    // 401 sans jeton.
    let (st, _) = appel(&app, "POST", "/personnels/absences", None, Some(corps.clone())).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    // Un parent ne déclare rien.
    let (st, _) = appel(&app, "POST", "/personnels/absences", Some(&parent.jeton), Some(corps.clone())).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Le professeur déclare la sienne : 201, il lit sa nature.
    let (st, b) = appel(&app, "POST", "/personnels/absences", Some(&prof.jeton), Some(corps)).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    let id = b["absences"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(b["absences"][0]["nature_code"], "maladie");
    assert_eq!(b["absences"][0]["decision"], "externe");
    // Hors du cercle RH : 404 uniforme (collègue), 403 (parent).
    let (st, _) = appel(&app, "GET", &format!("/personnels/absences/{id}"), Some(&prof2.jeton), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = appel(&app, "GET", &format!("/personnels/absences/{id}"), Some(&parent.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Vue organisation : refusée au professeur et à l'accueil, sans nature pour la direction.
    let org = format!("/personnels/absences/organisation?du={jour}&au={jour}");
    for c in [&prof2, &accueil] {
        let (st, _) = appel(&app, "GET", &org, Some(&c.jeton), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    let (st, b) = appel(&app, "GET", &org, Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    assert_eq!(b.as_array().unwrap().len(), 1);
    let texte = b.to_string();
    for interdit in ["maladie", "nature", "traitement", "decision"] {
        assert!(!texte.contains(interdit), "« {interdit} » dans la vue organisation : {texte}");
    }
    // Valider : pas le professeur ; la direction prend en compte (l'employeur décide).
    let (st, _) = appel(&app, "POST", &format!("/personnels/absences/{id}/valider"), Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, b) = appel(&app, "POST", &format!("/personnels/absences/{id}/refuser"), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::CONFLICT, "refus impossible pour une maladie : {b}");
    let (st, b) = appel(&app, "POST", &format!("/personnels/absences/{id}/valider"), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    assert_eq!(b["etat"], "prise_en_compte");
    // Port « non remplacées » : lisible de la direction, jamais de l'accueil ; ni motif ni nature.
    let port = format!("/personnels/absences/non-remplacees?du={jour}&au={jour}");
    let (st, b) = appel(&app, "GET", &port, Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    assert!(b.is_array());
    let (st, _) = appel(&app, "GET", &port, Some(&accueil.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Les routes historiques ne renvoient jamais la nature : la liste porte « Absence ».
    let (st, b): (StatusCode, Value) = appel(&app, "GET", &format!("/rh/absences?debut={jour}&fin={jour}"), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(!b.to_string().contains("maladie"), "{b}");
    // Un filtre de statut inconnu est refusé (AR-22 : le paramètre est traité).
    let (st, _) = appel(&app, "GET", "/rh/absences?statut=n_importe_quoi", Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}
