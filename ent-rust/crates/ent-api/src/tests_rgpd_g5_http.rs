//! Socle 27, palier G5 (HTTP) : les routes `/rgpd/violations/*` du registre des violations répondent comme le contrat
//! le dit. Les règles métier sont dans `ent-db` (`tests_rgpd_g5.rs`) ; ici on vérifie **la surface** : session exigée,
//! déclaration ouverte à tout personnel et fermée aux familles, minimisation de ce que voit le déclarant, droits de
//! gestion et de décision, export CSV, refus neutres.
//!
//! Base locale de labo (cast Harry Potter).

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

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

/// Le parcours de bout en bout par HTTP : un professeur déclare, la direction instruit et clôt, et les refus attendus.
#[tokio::test]
async fn g5_http_registre_parcours_et_refus() {
    let Some(url) = super::tests_l3::url_locale() else {
        eprintln!("skip G5 HTTP: ENT_DATABASE_URL");
        return;
    };
    let ent = PgEnt::connect(&url).await.unwrap();
    ent.migrate().await.unwrap();
    let state = AppState::nouveau(ent);
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let app = app(state.clone());
    let (dir, prof, parent) = (
        cast.direction.token.as_str(),
        cast.prof.token.as_str(),
        cast.parent_lea.token.as_str(),
    );

    // Sans session : 401 (la route existe).
    for (m, uri) in [
        ("GET", "/rgpd/violations"),
        ("POST", "/rgpd/violations"),
        ("GET", "/rgpd/violations/declarations"),
        ("POST", "/rgpd/violations/declarations"),
        ("GET", "/rgpd/violations/export.csv"),
    ] {
        let (st, _) = appel(&app, m, uri, None, Some(json!({}))).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {uri}");
    }

    // Une famille ne déclare pas (elle passe par le guichet des droits) ; un professeur déclare.
    let (st, _) = appel(
        &app,
        "POST",
        "/rgpd/violations/declarations",
        Some(parent),
        Some(json!({"intitule": "x"})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, v) = appel(
        &app,
        "POST",
        "/rgpd/violations/declarations",
        Some(prof),
        Some(json!({"intitule": "Clé USB avec les notes de 4e B perdue dans le bus"})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{v}");
    let id: Uuid = v["id"].as_str().unwrap().parse().unwrap();
    assert!(v["reference"].as_str().unwrap().starts_with("V-"));
    assert_eq!(v["avancement"], "recue");
    // Minimisation : le déclarant ne reçoit ni le risque, ni l'échéance, ni la chronologie.
    for champ in ["risque", "echeance_cnil_le", "chronologie", "categories"] {
        assert!(v.get(champ).is_none(), "{champ} exposé au déclarant");
    }
    let (st, v) = appel(
        &app,
        "GET",
        "/rgpd/violations/declarations",
        Some(prof),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(v
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["id"] == id.to_string()));

    // Le registre et la fiche sont fermés au professeur.
    for uri in [
        "/rgpd/violations".to_string(),
        format!("/rgpd/violations/{id}"),
        "/rgpd/violations/export.csv".into(),
    ] {
        let (st, _) = appel(&app, "GET", &uri, Some(prof), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{uri}");
    }

    // La direction voit la fiche, avec l'horloge des 72 h.
    let (st, v) = appel(
        &app,
        "GET",
        &format!("/rgpd/violations/{id}"),
        Some(dir),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["etat"], "ouverte");
    assert!(v["heures_restantes"].as_i64().unwrap() >= 71);
    // Violation inconnue : 404 neutre.
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/rgpd/violations/{}", Uuid::new_v4()),
        Some(dir),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Qualification, avis du DPO, décision, non-notification, clôture.
    let (st, v) = appel(
        &app,
        "POST",
        &format!("/rgpd/violations/{id}/qualification"),
        Some(dir),
        Some(json!({
            "nature_incident": "violation", "confidentialite": true, "categories": ["eva.note"],
            "personnes_nombre": 28, "personnes_estime": false, "personnes_mineurs": true,
            "consequences": "clé retrouvée scellée le soir même"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert!(v["risque_propose"].is_string());
    // Valeur hors liste : 400.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/violations/{id}/risque"),
        Some(dir),
        Some(json!({"risque": "moyen", "motif": "x"})),
    )
    .await;
    assert!(
        st == StatusCode::BAD_REQUEST
            || st == StatusCode::CONFLICT
            || st == StatusCode::UNPROCESSABLE_ENTITY,
        "{st}"
    );
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/violations/{id}/avis-dpo"),
        Some(dir),
        Some(json!({"texte": "DPO : clé retrouvée intacte, risque faible"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, v) = appel(
        &app,
        "POST",
        &format!("/rgpd/violations/{id}/risque"),
        Some(dir),
        Some(json!({"risque": "faible", "motif": "support retrouvé scellé, aucune lecture possible"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/violations/{id}/non-notification"),
        Some(dir),
        Some(json!({"texte": "risque faible décidé après avis du DPO"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, v) = appel(
        &app,
        "POST",
        &format!("/rgpd/violations/{id}/cloture"),
        Some(dir),
        Some(json!({"texte": "clés USB chiffrées pour tous les personnels"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["etat"], "close");
    // Une violation close ne se modifie plus : 409.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/violations/{id}/mesures"),
        Some(dir),
        Some(json!({"nature": "prise", "texte": "après coup"})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);

    // Le déclarant voit l'avancement, rien d'autre.
    let (_, v) = appel(
        &app,
        "GET",
        "/rgpd/violations/declarations",
        Some(prof),
        None,
    )
    .await;
    let d = v
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == id.to_string())
        .unwrap()
        .clone();
    assert_eq!(d["avancement"], "close");

    // Registre filtré et export CSV.
    let (st, v) = appel(&app, "GET", "/rgpd/violations?etat=close", Some(dir), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(v.as_array().unwrap().iter().all(|l| l["etat"] == "close"));
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/rgpd/violations/export.csv")
                .header("authorization", format!("Bearer {dir}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let ct = res
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(ct.starts_with("text/csv"), "{ct}");
    let corps =
        String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap();
    assert!(corps.contains("Clé USB avec les notes"));
}
