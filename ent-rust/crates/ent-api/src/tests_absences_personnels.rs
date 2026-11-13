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

/// Lot A2 : pièces, fil, conservation, côté HTTP. Dépôt en base64 par l'agent ; 404 uniforme hors du cercle
/// RH ; 403 sans le droit de conservation ; aucun nom de fichier dans les réponses ; fil en ajout seul.
#[tokio::test]
async fn absences_des_personnels_pieces_fil_et_conservation() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip A15 A2 : ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let prof2 = inscrire(&app, etab, "prof").await;
    let parent = inscrire(&app, etab, "parent").await;
    let jour = (chrono::Utc::now() + Duration::days(21)).date_naive().to_string();
    let (st, b) = appel(
        &app,
        "POST",
        "/personnels/absences",
        Some(&prof.jeton),
        Some(json!({"nature_code": "maladie", "debut": jour, "fin": jour})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    let id = b["absences"][0]["id"].as_str().unwrap().to_string();
    let pdf = "JVBERi0xLjQKMSAwIG9iago8PCAvVHlwZSAvQ2F0YWxvZyA+PgplbmRvYmoKdHJhaWxlcgo8PCAvUm9vdCAxIDAgUiA+PgolJUVPRgo=";
    let depot = |nom: &str, volet: bool| {
        json!({"type_piece": "arret", "volet_1_exclu": volet,
               "fichier": {"nom": nom, "mime_type": "application/pdf", "contenu_base64": pdf}})
    };
    let url = format!("/personnels/absences/{id}/pieces");
    // 401 sans jeton ; un parent : 403 ; un collègue : 404 uniforme.
    let (st, _) = appel(&app, "POST", &url, None, Some(depot("a.pdf", true))).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = appel(&app, "POST", &url, Some(&parent.jeton), Some(depot("a.pdf", true))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "POST", &url, Some(&prof2.jeton), Some(depot("a.pdf", true))).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Un arrêt sans l'attestation « volets 2 et 3 » est refusé ; avec, il est déposé.
    let (st, _) = appel(&app, "POST", &url, Some(&prof.jeton), Some(depot("a.pdf", false))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, b) = appel(&app, "POST", &url, Some(&prof.jeton), Some(depot("arret-chimio-dr-martin.pdf", true))).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    assert!(!b.to_string().contains("chimio"), "nom d'origine renvoyé : {b}");
    let piece = b["id"].as_str().unwrap().to_string();
    // Liste : l'agent et la direction ; jamais le collègue.
    let (st, b) = appel(&app, "GET", &url, Some(&prof.jeton), None).await;
    assert_eq!((st, b.as_array().map(Vec::len)), (StatusCode::OK, Some(1)), "{b}");
    let (st, _) = appel(&app, "GET", &url, Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(&app, "GET", &url, Some(&prof2.jeton), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Lien : jamais ouvert au collègue (404) ; l'agent l'obtient (200) ou 503 si le signeur est désactivé.
    let lien = format!("{url}/{piece}/lien");
    let (st, _) = appel(&app, "POST", &lien, Some(&prof2.jeton), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, b) = appel(&app, "POST", &lien, Some(&prof.jeton), None).await;
    assert!(matches!(st, StatusCode::OK | StatusCode::SERVICE_UNAVAILABLE), "{st} {b}");
    // Qui a lu : l'agent seul.
    let (st, _) = appel(&app, "GET", &format!("{url}/lectures"), Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(&app, "GET", &format!("{url}/lectures"), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Fil : l'agent et la direction écrivent ; 4 001 caractères refusés ; le collègue ne lit ni n'écrit.
    let fil = format!("/personnels/absences/{id}/messages");
    let (st, b) = appel(&app, "POST", &fil, Some(&prof.jeton), Some(json!({"texte": "Voici mon arrêt."}))).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    assert_eq!(b["role_affiche"], "agent");
    let (st, b) = appel(&app, "POST", &fil, Some(&dir.jeton), Some(json!({"texte": "Bien reçu."}))).await;
    assert_eq!((st, &b["role_affiche"]), (StatusCode::CREATED, &json!("direction")), "{b}");
    let (st, _) = appel(&app, "POST", &fil, Some(&prof.jeton), Some(json!({"texte": "x".repeat(4001)}))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, _) = appel(&app, "GET", &fil, Some(&prof2.jeton), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, b) = appel(&app, "GET", &fil, Some(&prof.jeton), None).await;
    assert_eq!((st, b.as_array().map(Vec::len)), (StatusCode::OK, Some(2)), "{b}");
    // Conservation : la direction seule (gel, purges à venir, purge à la demande, confidentialité).
    let a_venir = "/personnels/absences/purges-a-venir";
    for c in [&prof, &prof2] {
        let (st, _) = appel(&app, "GET", a_venir, Some(&c.jeton), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    let (st, b) = appel(&app, "GET", a_venir, Some(&dir.jeton), None).await;
    assert_eq!((st, b.as_array().map(Vec::len)), (StatusCode::OK, Some(1)), "{b}");
    let gel = format!("/personnels/absences/{id}/gels");
    let (st, _) = appel(&app, "POST", &gel, Some(&prof.jeton), Some(json!({"motif": "contentieux", "du": jour}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, b) = appel(&app, "POST", &gel, Some(&dir.jeton), Some(json!({"motif": "Recours en cours", "du": jour}))).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    let (st, _) = appel(
        &app,
        "POST",
        &format!("{url}/{piece}/purger"),
        Some(&dir.jeton),
        Some(json!({"motif_code": "autre"})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "la purge est suspendue par le gel");
    let (st, _) = appel(&app, "PUT", &format!("/personnels/absences/{id}/confidentialite"), Some(&prof.jeton), Some(json!({"renforcee": true}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "PUT", &format!("/personnels/absences/{id}/confidentialite"), Some(&dir.jeton), Some(json!({"renforcee": true}))).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
}
