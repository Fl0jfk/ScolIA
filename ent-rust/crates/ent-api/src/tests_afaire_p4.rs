//! Socle 29, lot P4 : les files des personnels côté HTTP (base locale). Les scénarios métier (deux CPE,
//! prises simultanées, expiration, orphelines) sont dans `ent-db` (`tests_afaire_p4`) ; ici :
//! authentification, cloisonnement par profil et codes de retour des nouvelles routes.

use super::tests_l3::{appel, etat, inscrire};
use super::*;
use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn files_routes_authentifiees_cloisonnees_et_sans_ecriture_par_get() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip P4: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let parent = inscrire(&app, etab, "parent").await;
    let prof = inscrire(&app, etab, "prof").await;
    let cpe = inscrire(&app, etab, "cpe").await;
    let admin = inscrire(&app, etab, "admin").await;
    let inconnue = Uuid::new_v4();

    // Sans jeton : 401 partout.
    let routes: Vec<(&str, String)> = vec![
        ("GET", "/afaire/files".to_string()),
        ("GET", "/afaire/files/vs.justificatifs".to_string()),
        ("GET", "/afaire/supervision".to_string()),
        ("GET", "/afaire/orphelines".to_string()),
        ("POST", format!("/afaire/{inconnue}/prendre")),
        ("POST", format!("/afaire/{inconnue}/relacher")),
        ("POST", format!("/afaire/{inconnue}/preparee")),
        ("POST", format!("/afaire/{inconnue}/reattribuer")),
        ("POST", format!("/afaire/orphelines/{inconnue}/sans-objet")),
    ];
    for (m, u) in &routes {
        let (st, _) = appel(&app, m, u, None, Some(json!({"vers_compte_id": inconnue}))).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {u}");
    }

    // Témoin : un CPE (membre de la file de la vie scolaire) lit « Mes files » et sa file.
    let (st, corps) = appel(&app, "GET", "/afaire/files", Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let tuiles = corps["tuiles"].as_array().expect("tuiles");
    assert!(
        tuiles
            .iter()
            .any(|t| t["code"] == "vs.justificatifs" && t["origine"] == "afaire"),
        "{corps}"
    );
    let (st, file) = appel(
        &app,
        "GET",
        "/afaire/files/vs.justificatifs",
        Some(&cpe.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{file}");
    assert!(file["taches"].is_array() && file["deux_niveaux"] == false);

    // Un professeur n'est membre d'aucune file : « Mes files » vide, la file elle-même introuvable (404,
    // identique à une file qui n'existe pas : aucune énumération). Une famille n'a pas cet écran (403).
    let (st, corps) = appel(&app, "GET", "/afaire/files", Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(corps["tuiles"].as_array().unwrap().is_empty(), "{corps}");
    for u in [
        "/afaire/files/vs.justificatifs",
        "/afaire/files/n.importe.quoi",
    ] {
        let (st, _) = appel(&app, "GET", u, Some(&prof.jeton), None).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{u}");
    }
    let (st, _) = appel(&app, "GET", "/afaire/files", Some(&parent.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Prendre une tâche qui n'existe pas : 404, jamais « déjà prise ».
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/afaire/{inconnue}/prendre"),
        Some(&cpe.jeton),
        Some(json!({"niveau": "decider"})),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Corps absent : accepté (niveau « décider » par défaut), même réponse.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/afaire/{inconnue}/prendre"),
        Some(&cpe.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Aucune écriture par GET : prendre, relâcher, préparer, réattribuer ne répondent qu'en POST.
    for fin in ["prendre", "relacher", "preparee", "reattribuer"] {
        let (st, _) = appel(
            &app,
            "GET",
            &format!("/afaire/{inconnue}/{fin}"),
            Some(&cpe.jeton),
            None,
        )
        .await;
        assert_eq!(
            st,
            StatusCode::METHOD_NOT_ALLOWED,
            "GET /afaire/{{id}}/{fin} ne doit rien faire"
        );
    }
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/afaire/orphelines/{inconnue}/sans-objet"),
        Some(&admin.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::METHOD_NOT_ALLOWED);

    // Réattribuer : réservé aux responsables de service (le CPE a 403, pas 404 : le droit manque).
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/afaire/{inconnue}/reattribuer"),
        Some(&cpe.jeton),
        Some(json!({"vers_compte_id": inconnue})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Supervision et orphelines : nombres pour l'administrateur ; refus pour les autres profils.
    for u in ["/afaire/supervision", "/afaire/orphelines"] {
        for (nom, jeton) in [
            ("cpe", &cpe.jeton),
            ("prof", &prof.jeton),
            ("parent", &parent.jeton),
        ] {
            let (st, _) = appel(&app, "GET", u, Some(jeton), None).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "{nom} {u}");
        }
    }
    let (st, sup) = appel(&app, "GET", "/afaire/supervision", Some(&admin.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{sup}");
    assert!(sup["lignes"].is_array() && sup["orphelines"].is_number());
    let (st, orph) = appel(&app, "GET", "/afaire/orphelines", Some(&admin.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(orph.is_array());
    // Clore une orpheline qui n'en est pas une : 404 (pas d'énumération).
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/afaire/orphelines/{inconnue}/sans-objet"),
        Some(&admin.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}
