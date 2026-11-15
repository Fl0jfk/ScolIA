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

/// Lot A3 : impact, tableaux, files, présence, interrupteur — côté HTTP. 401 sans jeton ; 403 selon la matrice
/// de droits ; 404 uniforme hors du cercle ; aucune nature dans les réponses d'organisation.
#[tokio::test]
async fn absences_des_personnels_impact_tableaux_et_files() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip A15 A3 : ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let cpe = inscrire(&app, etab, "cpe").await;
    let prof = inscrire(&app, etab, "prof").await;
    let prof2 = inscrire(&app, etab, "prof").await;
    let accueil = inscrire(&app, etab, "accueil").await;
    let parent = inscrire(&app, etab, "parent").await;
    let jour = (chrono::Utc::now() + Duration::days(3)).date_naive().to_string();
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
    let agent = b["absences"][0]["agent_id"].as_str().unwrap().to_string();
    let base = "/personnels/absences";
    // 401 sans jeton.
    for chemin in [format!("{base}/tableau"), format!("{base}/compteurs"), format!("{base}/{id}/impact")] {
        let (st, _) = appel(&app, "GET", &chemin, None, None).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{chemin}");
    }
    // Impact : l'agent, l'organisation ; 404 pour un collègue ; 403 pour un parent. Aucune nature.
    let impact = format!("{base}/{id}/impact");
    for c in [&prof, &cpe, &dir] {
        let (st, b) = appel(&app, "GET", &impact, Some(&c.jeton), None).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert!(b.is_array());
        assert!(!b.to_string().contains("maladie"));
    }
    let (st, _) = appel(&app, "GET", &impact, Some(&prof2.jeton), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = appel(&app, "GET", &impact, Some(&parent.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Tableau et compteurs : organisation seulement ; fenêtre bornée (422).
    let tableau = format!("{base}/tableau?jour={jour}");
    for c in [&cpe, &dir] {
        let (st, b) = appel(&app, "GET", &tableau, Some(&c.jeton), None).await;
        assert_eq!(st, StatusCode::OK, "{b}");
        assert!(b["lignes"].is_array() && b["compteurs"].is_object());
        assert!(!b.to_string().contains("maladie"), "{b}");
    }
    for c in [&prof, &accueil, &parent] {
        let (st, _) = appel(&app, "GET", &tableau, Some(&c.jeton), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    let loin = (chrono::Utc::now() + Duration::days(90)).date_naive();
    let (st, _) = appel(&app, "GET", &format!("{base}/tableau?jour={loin}"), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, b) = appel(&app, "GET", &format!("{base}/compteurs"), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    assert!(b["a_valider"].as_u64().unwrap() >= 1);
    let (st, _) = appel(&app, "GET", &format!("{base}/compteurs"), Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Files : « à traiter » réservée à l'instructeur ; « prochaine étape » du cercle RH.
    let (st, b) = appel(&app, "GET", &format!("{base}/a-traiter"), Some(&dir.jeton), None).await;
    assert_eq!((st, b.is_array()), (StatusCode::OK, true), "{b}");
    let (st, _) = appel(&app, "GET", &format!("{base}/a-traiter"), Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, b) = appel(&app, "GET", &format!("{base}/{id}/prochaine-etape"), Some(&prof.jeton), None).await;
    assert_eq!((st, &b["acteur"], &b["action"]), (StatusCode::OK, &json!("valideur"), &json!("valider")), "{b}");
    let (st, _) = appel(&app, "GET", &format!("{base}/{id}/prochaine-etape"), Some(&prof2.jeton), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Retour : l'agent seul.
    let (st, _) = appel(&app, "GET", &format!("{base}/{id}/retour"), Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(&app, "GET", &format!("{base}/{id}/retour"), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Présence : accueil et organisation ; jamais un parent ni un collègue ; « inconnu » / « oui » seulement.
    let pres = format!("{base}/presence/{agent}");
    let (st, b) = appel(&app, "GET", &pres, Some(&accueil.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    assert!(["oui", "non", "inconnu"].contains(&b["statut"].as_str().unwrap()));
    assert!(!b.to_string().contains("maladie"));
    for c in [&prof2, &parent] {
        let (st, _) = appel(&app, "GET", &pres, Some(&c.jeton), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    // Interrupteur « EDT de référence » : lisible de l'organisation, posé par la direction après contrôle.
    let sw = format!("{base}/edt-reference");
    let (st, b) = appel(&app, "GET", &sw, Some(&cpe.jeton), None).await;
    assert_eq!((st, &b["actif"]), (StatusCode::OK, &json!(false)), "{b}");
    let (st, _) = appel(&app, "PUT", &sw, Some(&cpe.jeton), Some(json!({"actif": true}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "PUT", &sw, Some(&dir.jeton), Some(json!({"actif": true}))).await;
    assert_eq!(st, StatusCode::CONFLICT, "aucune séance dans les 14 jours : EDT non à jour");
    // Sans objet : jamais sans le droit d'organiser l'EDT ; impact inconnu : 404.
    let faux = Uuid::new_v4();
    let (st, _) = appel(&app, "POST", &format!("{base}/impacts/{faux}/sans-objet"), Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "POST", &format!("{base}/impacts/{faux}/sans-objet"), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

/// Lot A4 : réponse et sollicitation, côté HTTP. 401 sans jeton ; 403 sans le droit d'organiser ; 404 pour la
/// demande d'un autre ; sollicitation → acceptation → l'impact devient « remplacé » ; levée ; refus propre d'une
/// acceptation répétée ; aucune nature dans ce que voit le remplaçant.
#[tokio::test]
async fn absences_des_personnels_reponse_et_sollicitation() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip A15 A4 : ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let prof2 = inscrire(&app, etab, "prof").await;
    let prof3 = inscrire(&app, etab, "prof").await;
    let accueil = inscrire(&app, etab, "accueil").await;
    let base = "/personnels/absences";
    // 401 / 403 : sans jeton, et sans le droit d'organiser.
    let faux = Uuid::new_v4();
    let (st, _) = appel(&app, "GET", &format!("{base}/mes-sollicitations"), None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    for c in [&prof, &accueil] {
        let (st, _) = appel(&app, "GET", &format!("{base}/impacts/{faux}/propositions"), Some(&c.jeton), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
        let (st, _) = appel(&app, "POST", &format!("{base}/impacts/{faux}/lever"), Some(&c.jeton), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
        let (st, _) = appel(&app, "POST", &format!("{base}/impacts/{faux}/annuler-seance"), Some(&c.jeton), Some(json!({}))).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    let (st, _) = appel(&app, "GET", &format!("{base}/impacts/{faux}/propositions"), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND, "impact inconnu");
    // Une demande inconnue ou celle d'un autre : 404 uniforme pour le remplaçant.
    for action in ["accepter", "accuser"] {
        let (st, _) = appel(&app, "POST", &format!("{base}/sollicitations/{faux}/{action}"), Some(&prof2.jeton), None).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{action}");
    }
    let (st, _) = appel(&app, "POST", &format!("{base}/sollicitations/{faux}/refuser"), Some(&prof2.jeton), Some(json!({}))).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Sortie anticipée : refusée (R-AP24) avant même de chercher l'impact.
    let (st, b) = appel(&app, "POST", &format!("{base}/impacts/{faux}/annuler-seance"), Some(&dir.jeton), Some(json!({"sortie_anticipee": true}))).await;
    assert_eq!(st, StatusCode::CONFLICT, "{b}");
    // Mes sollicitations : vide, jamais d'erreur.
    let (st, b) = appel(&app, "GET", &format!("{base}/mes-sollicitations"), Some(&prof2.jeton), None).await;
    assert_eq!((st, b.as_array().map(Vec::len)), (StatusCode::OK, Some(0)), "{b}");
    let (st, _) = appel(&app, "POST", &format!("{base}/impacts/{faux}/appel-volontaires"), Some(&dir.jeton), Some(json!({"agents": [faux]}))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "au moins deux volontaires");
    let _ = prof3;
}

/// Lot A5 : absence longue et suppléance, côté HTTP. 401 sans jeton ; 403 sans l'action `suppleer` ; 404 pour une
/// absence ou une demande inconnue ; demande (201) → lecture → réponse → état du remplacement ; aucune nature
/// dans ce qui est renvoyé ; nomination réservée à la direction.
#[tokio::test]
async fn absences_des_personnels_suppleance_et_etat_du_remplacement() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip A15 A5 : ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let sec = inscrire(&app, etab, "administratif").await;
    let prof = inscrire(&app, etab, "prof").await;
    let accueil = inscrire(&app, etab, "accueil").await;
    let base = "/personnels/absences";
    let faux = Uuid::new_v4();
    // 401 sans jeton ; 403 sans l'action (professeur, accueil).
    let (st, _) = appel(&app, "GET", &format!("{base}/{faux}/remplacement"), None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    for c in [&prof, &accueil] {
        for (m, u, corps) in [
            ("POST", format!("{base}/{faux}/suppleance"), Some(json!({"destinataire": "rectorat"}))),
            ("GET", format!("{base}/{faux}/suppleance"), None),
            ("GET", format!("{base}/{faux}/remplacement"), None),
            ("POST", format!("{base}/demandes-suppleance/{faux}/reponse"), Some(json!({"issue": "refusee"}))),
            ("POST", format!("{base}/{faux}/suppleant"), Some(json!({"service_id": faux, "agent_id": faux, "du": "2027-01-04"}))),
        ] {
            let (st, _) = appel(&app, m, &u, Some(&c.jeton), corps).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "{m} {u}");
        }
    }
    // Inconnu : 404 uniforme.
    let (st, _) = appel(&app, "POST", &format!("{base}/{faux}/suppleance"), Some(&dir.jeton), Some(json!({"destinataire": "rectorat"}))).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = appel(&app, "POST", &format!("{base}/demandes-suppleance/{faux}/reponse"), Some(&dir.jeton), Some(json!({"issue": "refusee"}))).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Une absence réelle (validée par la direction) : demande par la secrétaire, lecture, réponse, état.
    let jour = (chrono::Utc::now() + Duration::days(20)).date_naive();
    let fin = jour + Duration::days(30);
    let (st, b) = appel(
        &app,
        "POST",
        base,
        Some(&prof.jeton),
        Some(json!({"nature_code": "convenance", "debut": jour.to_string(), "fin": fin.to_string()})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    let id = b["absences"][0]["id"].as_str().unwrap().to_string();
    let (st, b) = appel(&app, "POST", &format!("{base}/{id}/valider"), Some(&dir.jeton), Some(json!({}))).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    let (st, b) = appel(&app, "POST", &format!("{base}/{id}/suppleance"), Some(&sec.jeton), Some(json!({"destinataire": "tout le monde"}))).await;
    assert!(st.is_client_error(), "destinataire hors codes : {st} {b}");
    let (st, b) = appel(&app, "POST", &format!("{base}/{id}/suppleance"), Some(&sec.jeton), Some(json!({"destinataire": "rectorat", "reference": "DOS-0001"}))).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    assert_eq!(b["etat"], "demandee");
    let demande = b["id"].as_str().unwrap().to_string();
    let (st, b) = appel(&app, "POST", &format!("{base}/{id}/suppleance"), Some(&dir.jeton), Some(json!({"destinataire": "rectorat"}))).await;
    assert_eq!(st, StatusCode::CONFLICT, "une seule demande en cours : {b}");
    let (st, b) = appel(&app, "GET", &format!("{base}/{id}/remplacement"), Some(&dir.jeton), None).await;
    assert_eq!((st, b["etape"].as_str()), (StatusCode::OK, Some("en_recherche")), "{b}");
    let (st, b) = appel(&app, "POST", &format!("{base}/demandes-suppleance/{demande}/reponse"), Some(&sec.jeton), Some(json!({"issue": "refusee"}))).await;
    assert_eq!((st, b["etat"].as_str()), (StatusCode::OK, Some("refusee")), "{b}");
    let (st, b) = appel(&app, "GET", &format!("{base}/{id}/suppleance"), Some(&dir.jeton), None).await;
    assert_eq!((st, b.as_array().map(Vec::len)), (StatusCode::OK, Some(1)), "{b}");
    // Nomination : le secrétariat n'a pas le droit de nommer (403) ; la direction, sur un service inconnu : 400/404/422.
    let nommer = json!({"service_id": faux, "agent_id": faux, "du": jour.to_string()});
    let (st, _) = appel(&app, "POST", &format!("{base}/{id}/suppleant"), Some(&sec.jeton), Some(nommer.clone())).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, b) = appel(&app, "POST", &format!("{base}/{id}/suppleant"), Some(&dir.jeton), Some(nommer)).await;
    assert!(st.is_client_error() && st != StatusCode::FORBIDDEN, "service inconnu : {st} {b}");
    // Ni la nature ni le motif dans ce qui sort.
    let (_, b) = appel(&app, "GET", &format!("{base}/{id}/remplacement"), Some(&dir.jeton), None).await;
    let texte = b.to_string();
    for interdit in ["convenance", "nature", "motif"] {
        assert!(!texte.contains(interdit), "« {interdit} » dans l'état du remplacement : {texte}");
    }
}
