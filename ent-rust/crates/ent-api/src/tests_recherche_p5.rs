//! Socle 29, lot P5 : la recherche globale côté HTTP (base locale). Les scénarios métier (droits par source,
//! adresses protégées, plafonds, blocage, aucun texte conservé) sont dans `ent-db` (`tests_recherche_p5`) ;
//! ici : authentification, cloisonnement par profil et codes de retour.

use super::tests_l3::{appel, etat, inscrire};
use super::*;
use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn recherche_routes_authentifiees_et_cloisonnees() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let parent = inscrire(&app, etab, "parent").await;
    let prof = inscrire(&app, etab, "prof").await;
    let admin = inscrire(&app, etab, "admin").await;
    let inconnue = Uuid::new_v4();

    // Sans jeton : 401 partout.
    for (m, u) in [
        ("GET", "/recherche?q=potter".to_string()),
        ("GET", "/recherche/blocages".to_string()),
        ("POST", format!("/recherche/{inconnue}/debloquer")),
        ("POST", "/recherche/debloquer-moi".to_string()),
    ] {
        let (st, _) = appel(&app, m, &u, None, Some(json!({"motif": "autre"}))).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {u}");
    }

    // Témoin : le personnel cherche (réponse structurée, au plus 40 résultats) ; trop court : vide.
    let (st, corps) = appel(&app, "GET", "/recherche?q=potter", Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    assert!(corps["resultats"].is_array() && corps["resultats"].as_array().unwrap().len() <= 40);
    assert_eq!(corps["ralenti"], false);
    let (st, corps) = appel(&app, "GET", "/recherche?q=a", Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(corps["resultats"].as_array().unwrap().is_empty());
    // Une famille n'a pas la recherche.
    let (st, _) = appel(
        &app,
        "GET",
        "/recherche?q=potter",
        Some(&parent.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Blocages et déblocage : réservés à `recherche.debloquer` (administrateur).
    for (nom, jeton) in [("prof", &prof.jeton), ("parent", &parent.jeton)] {
        let (st, _) = appel(&app, "GET", "/recherche/blocages", Some(jeton), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{nom}");
        let (st, _) = appel(
            &app,
            "POST",
            &format!("/recherche/{inconnue}/debloquer"),
            Some(jeton),
            None,
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{nom}");
    }
    let (st, liste) = appel(&app, "GET", "/recherche/blocages", Some(&admin.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(liste.is_array());
    // Lever un compte non bloqué : 404 (pas d'oracle).
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/recherche/{inconnue}/debloquer"),
        Some(&admin.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Déblocage personnel : motif hors liste refusé ; personne non bloquée : refus.
    let (st, _) = appel(
        &app,
        "POST",
        "/recherche/debloquer-moi",
        Some(&prof.jeton),
        Some(json!({"motif": "n.importe.quoi"})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = appel(
        &app,
        "POST",
        "/recherche/debloquer-moi",
        Some(&prof.jeton),
        Some(json!({"motif": "autre"})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Aucune écriture par GET.
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/recherche/{inconnue}/debloquer"),
        Some(&admin.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::METHOD_NOT_ALLOWED);
    let (st, _) = appel(
        &app,
        "GET",
        "/recherche/debloquer-moi",
        Some(&prof.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::METHOD_NOT_ALLOWED);
}
