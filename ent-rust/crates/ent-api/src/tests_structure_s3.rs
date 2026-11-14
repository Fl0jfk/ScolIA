//! Socle 5, lot S3 (HTTP) : routes du calendrier, des semaines A/B et des fermetures — session
//! obligatoire, matrice de droits, aperçu par défaut, cloisonnement entre établissements, refus
//! 409 d'un doublon. Base locale de labo.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn appel(
    app: &axum::Router,
    m: &str,
    uri: &str,
    jeton: Option<&str>,
    corps: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(m).uri(uri);
    if let Some(j) = jeton {
        b = b.header("authorization", format!("Bearer {j}"));
    }
    let req = match corps {
        Some(c) => b
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&c).unwrap()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let st = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (st, v)
}

#[tokio::test]
async fn routes_calendrier_droits_apercu_cloisonnement_et_409() {
    let Some(url) = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
    else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = PgEnt::connect(&url).await.unwrap();
    ent.migrate().await.unwrap();
    let state = AppState::nouveau(ent);
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let autre = state.ent.seed_labo_cast().await.expect("cast B");
    let annee = state
        .ent
        .referentiel_annee_courante_assuree(cast.etablissement_id)
        .await
        .unwrap();
    let app = app(state.clone());
    let (dir, adm, prof, parent, dir_b) = (
        cast.direction.token.as_str(),
        cast.administratif.token.as_str(),
        cast.prof.token.as_str(),
        cast.parent_lea.token.as_str(),
        autre.direction.token.as_str(),
    );
    let demain = (domain_eleve::horloge::jour_paris() + chrono::Duration::days(2)).to_string();
    let apres = (domain_eleve::horloge::jour_paris() + chrono::Duration::days(3)).to_string();
    let a = annee.id;

    // Session obligatoire.
    for (m, u) in [
        (
            "GET",
            format!("/structure/calendrier/evenements?annee_id={a}"),
        ),
        ("POST", "/structure/calendrier/evenements".to_string()),
        ("POST", "/structure/calendrier/adoption".to_string()),
        ("GET", format!("/structure/calendrier/jour?date={demain}")),
        ("GET", format!("/structure/semaines?annee_id={a}")),
        ("POST", "/structure/fermetures".to_string()),
    ] {
        let (st, _) = appel(&app, m, &u, None, None).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {u}");
    }

    // Lecture : personnel oui, famille non.
    let jour = format!("/structure/calendrier/jour?date={demain}");
    let (st, j) = appel(&app, "GET", &jour, Some(prof), None).await;
    assert_eq!(st, StatusCode::OK, "{j}");
    assert!(j["type_jour"].is_string());
    let (st, _) = appel(&app, "GET", &jour, Some(parent), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Écriture du calendrier : administratif oui, professeur non.
    let evt = json!({"annee_id": a, "type": "pont", "libelle": "Pont", "du": demain, "au": demain});
    let (st, _) = appel(
        &app,
        "POST",
        "/structure/calendrier/evenements",
        Some(prof),
        Some(evt.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, c) = appel(
        &app,
        "POST",
        "/structure/calendrier/evenements",
        Some(adm),
        Some(evt),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    let evt_id = c["evenement"]["id"].as_str().unwrap().to_string();
    // Corps invalide : 422.
    let (st, _) = appel(
        &app,
        "POST",
        "/structure/calendrier/evenements",
        Some(adm),
        Some(json!({"annee_id": a, "type": "n_importe_quoi", "libelle": "x", "du": demain, "au": demain})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, l) = appel(
        &app,
        "GET",
        &format!("/structure/calendrier/evenements?annee_id={a}"),
        Some(prof),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(l.as_array().unwrap().len(), 1);
    // Le jour du pont n'a plus cours.
    let (_, j) = appel(&app, "GET", &jour, Some(dir), None).await;
    assert!(j["evenements"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["id"] == json!(evt_id)));

    // Adoption : aperçu par défaut, sans écriture.
    let (st, r) = appel(
        &app,
        "POST",
        "/structure/calendrier/adoption",
        Some(adm),
        Some(json!({"annee_id": a})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert_eq!(r["ecrit"], json!(false));
    let (st, m) = appel(
        &app,
        "GET",
        "/structure/calendrier/modele-saisie?zone=a",
        Some(prof),
        None,
    )
    .await;
    assert_eq!(
        (st, m["periodes"].as_array().map(Vec::len)),
        (StatusCode::OK, Some(5))
    );

    // Fermeture : direction seulement ; aperçu par défaut ; doublon 409.
    let ferme =
        json!({"du": demain, "au": apres, "motif": "Alerte météo", "internes_presents": true});
    let (st, _) = appel(
        &app,
        "POST",
        "/structure/fermetures",
        Some(adm),
        Some(ferme.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, r) = appel(
        &app,
        "POST",
        "/structure/fermetures",
        Some(dir),
        Some(ferme.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert_eq!(r["ecrit"], json!(false));
    assert!(r["apercu"]["services_maintenus"]
        .as_array()
        .unwrap()
        .contains(&json!("internat")));
    let mut ecrire = ferme.clone();
    ecrire["apercu"] = json!(false);
    let (st, r) = appel(
        &app,
        "POST",
        "/structure/fermetures",
        Some(dir),
        Some(ecrire.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    let f_id = r["evenement"]["id"].as_str().unwrap().to_string();
    let (st, _) = appel(
        &app,
        "POST",
        "/structure/fermetures",
        Some(dir),
        Some(ecrire),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    let (st, h) = appel(
        &app,
        "GET",
        &format!("/structure/fermetures/{f_id}/historique"),
        Some(prof),
        None,
    )
    .await;
    assert_eq!((st, h.as_array().map(Vec::len)), (StatusCode::OK, Some(1)));

    // Cloisonnement : l'établissement voisin ne voit ni ne modifie rien.
    let (st, l) = appel(
        &app,
        "GET",
        &format!("/structure/calendrier/evenements?annee_id={a}"),
        Some(dir_b),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(l.as_array().unwrap().is_empty());
    for (m, u, c) in [
        (
            "GET",
            format!("/structure/calendrier/evenements/{evt_id}"),
            None,
        ),
        (
            "POST",
            format!("/structure/calendrier/evenements/{evt_id}/annuler"),
            Some(json!({"motif": "x"})),
        ),
        (
            "POST",
            format!("/structure/fermetures/{f_id}/lever"),
            Some(json!({"motif": "x"})),
        ),
        (
            "PUT",
            format!("/structure/fermetures/{f_id}"),
            Some(json!({"motif": "x", "au": apres})),
        ),
    ] {
        let (st, _) = appel(&app, m, &u, Some(dir_b), c).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{m} {u}");
    }

    // Levée par la direction, puis l'événement n'est plus en vigueur ; annulation du pont.
    let (st, e) = appel(
        &app,
        "POST",
        &format!("/structure/fermetures/{f_id}/lever"),
        Some(dir),
        Some(json!({"motif": "fin d'alerte", "a_partir_du": demain})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{e}");
    assert!(e["annule_le"].is_string());
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/structure/calendrier/evenements/{evt_id}/annuler"),
        Some(adm),
        Some(json!({"motif": "pont renoncé"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (_, j) = appel(&app, "GET", &jour, Some(dir), None).await;
    assert!(j["evenements"].as_array().unwrap().is_empty());

    // Semaines : règle puis lecture (professeur).
    let lundi = {
        use chrono::Datelike;
        let t = domain_eleve::horloge::jour_paris();
        (t + chrono::Duration::days(7 - i64::from(t.weekday().num_days_from_monday()))).to_string()
    };
    let (st, s) = appel(&app, "PUT", "/structure/semaines", Some(adm), Some(json!({"annee_id": a, "etiquettes": ["A", "B"], "mode": "alternance_continue", "premiere_semaine": lundi}))).await;
    assert_eq!(st, StatusCode::OK, "{s}");
    let (st, s) = appel(
        &app,
        "GET",
        &format!("/structure/semaines?annee_id={a}"),
        Some(prof),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(s["semaines"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["etiquette"] == json!("A")));
    let (st, _) = appel(&app, "PUT", "/structure/semaines", Some(prof), Some(json!({"annee_id": a, "etiquettes": ["A", "B"], "mode": "alternance_continue", "premiere_semaine": lundi}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}
