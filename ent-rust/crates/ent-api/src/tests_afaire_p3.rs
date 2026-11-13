//! Socle 29, lots P2 et P3 : l'à-faire côté HTTP (base locale). Les scénarios métier (neutralité,
//! plage calme, cartes d'action) sont dans `ent-db` (`tests_afaire_p2`) ; ici : authentification,
//! cloisonnement des routes et codes de retour.

use super::tests_l3::{appel, etat, inscrire};
use super::*;
use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn afaire_routes_authentifiees_cloisonnees_et_sans_ecriture_par_get() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip P3: ENT_DATABASE_URL");
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
        ("GET", "/afaire".to_string()),
        ("POST", format!("/afaire/{inconnue}/apercu")),
        ("POST", format!("/afaire/{inconnue}/executer")),
        ("POST", "/afaire/rapprochement".to_string()),
    ] {
        let (st, _) = appel(&app, m, &u, None, Some(json!({"confirme": true}))).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {u}");
    }

    // Témoin : un parent connecté lit son à-faire (vide, mais structuré).
    let (st, corps) = appel(&app, "GET", "/afaire", Some(&parent.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(corps["compteur"], 0);
    assert!(corps["taches"].as_array().unwrap().is_empty());
    // Le personnel a lui aussi un à-faire (ses files), jamais celui d'autrui.
    let (st, _) = appel(&app, "GET", "/afaire", Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::OK);

    // Une tâche qui n'est pas à la personne est introuvable (404), pour l'aperçu comme pour l'action.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/afaire/{inconnue}/apercu"),
        Some(&parent.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/afaire/{inconnue}/executer"),
        Some(&parent.jeton),
        Some(json!({"confirme": true, "motif": "x"})),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Aucune action à l'ouverture d'une adresse : ni `GET /afaire/{id}/executer`, ni `GET .../apercu`.
    for fin in ["executer", "apercu"] {
        let (st, _) = appel(
            &app,
            "GET",
            &format!("/afaire/{inconnue}/{fin}"),
            Some(&parent.jeton),
            None,
        )
        .await;
        assert_eq!(
            st,
            StatusCode::METHOD_NOT_ALLOWED,
            "GET /afaire/{{id}}/{fin} ne doit rien faire"
        );
    }

    // Rapprochement : réservé à l'administrateur.
    let (st, _) = appel(
        &app,
        "POST",
        "/afaire/rapprochement",
        Some(&parent.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(
        &app,
        "POST",
        "/afaire/rapprochement",
        Some(&prof.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, rapport) = appel(
        &app,
        "POST",
        "/afaire/rapprochement",
        Some(&admin.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(rapport["natures"].is_array());
}
