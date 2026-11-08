//! Socle 5, lot S2 (HTTP) : routes `/structure/*` — session obligatoire, matrice de droits,
//! cloisonnement entre établissements, aperçu par défaut de l'import. Base locale de labo.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn appel(app: &axum::Router, m: &str, uri: &str, jeton: Option<&str>, corps: Option<Value>) -> (StatusCode, Value) {
    let mut b = Request::builder().method(m).uri(uri);
    if let Some(j) = jeton {
        b = b.header("authorization", format!("Bearer {j}"));
    }
    let req = match corps {
        Some(c) => b.header("content-type", "application/json").body(Body::from(serde_json::to_vec(&c).unwrap())).unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let st = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (st, v)
}

#[tokio::test]
async fn routes_structure_droits_cloisonnement_et_apercu_par_defaut() {
    let Some(url) = std::env::var("ENT_DATABASE_URL").ok().filter(|u| !u.trim().is_empty()) else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = PgEnt::connect(&url).await.unwrap();
    ent.migrate().await.unwrap();
    let state = AppState::nouveau(ent);
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let autre = state.ent.seed_labo_cast().await.expect("cast B");
    let app = app(state.clone());
    let (dir, prof, parent) = (cast.direction.token.as_str(), cast.prof.token.as_str(), cast.parent_lea.token.as_str());
    let sfx = uuid::Uuid::new_v4().simple().to_string()[..6].to_uppercase();

    // Session obligatoire.
    for (m, u) in [("GET", "/structure/salles"), ("GET", "/structure/profil"), ("PUT", "/structure/fuseau")] {
        let (st, _) = appel(&app, m, u, None, None).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {u}");
    }
    // Lecture : personnel oui, famille non. Écriture : gestionnaires seulement.
    let (st, _) = appel(&app, "GET", "/structure/sites", Some(prof), None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(&app, "GET", "/structure/sites", Some(parent), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let corps_salle = |code: &str, site: &str| json!({"code": code, "libelle": "Salle", "site_id": site});
    let (st, site) = appel(&app, "POST", "/referentiel/sites", Some(dir), Some(json!({"code": format!("S{sfx}"), "libelle": "Site S2"}))).await;
    assert_eq!(st, StatusCode::CREATED, "{site}");
    let site_id = site["id"].as_str().unwrap().to_string();
    let (st, _) = appel(&app, "POST", "/structure/salles", Some(prof), Some(corps_salle(&format!("P{sfx}"), &site_id))).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "un professeur ne crée pas de salle");
    let (st, s) = appel(&app, "POST", "/structure/salles", Some(dir), Some(corps_salle(&format!("D{sfx}"), &site_id))).await;
    assert_eq!(st, StatusCode::CREATED, "{s}");
    let salle_id = s["id"].as_str().unwrap().to_string();
    assert_eq!(s["site_code"], format!("S{sfx}"));

    // Cloisonnement : l'autre établissement ne voit pas la salle, ne peut ni la modifier ni s'en servir.
    let adir = autre.direction.token.as_str();
    let (st, l) = appel(&app, "GET", "/structure/salles", Some(adir), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(l.as_array().unwrap().iter().all(|x| x["id"] != salle_id.as_str()));
    let (st, _) = appel(&app, "PUT", &format!("/structure/salles/{salle_id}"), Some(adir), Some(corps_salle(&format!("D{sfx}"), &site_id))).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = appel(&app, "POST", "/structure/salles", Some(adir), Some(corps_salle(&format!("X{sfx}"), &site_id))).await;
    assert_eq!(st, StatusCode::NOT_FOUND, "site d'un autre établissement");
    let (st, _) = appel(&app, "POST", &format!("/structure/salles/{salle_id}/indisponibilites"), Some(adir),
        Some(json!({"du": "2026-11-10T08:00:00Z", "au": "2026-11-10T09:00:00Z", "motif_code": "travaux"}))).await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // L'import est un aperçu tant qu'on ne dit pas « apercu: false ».
    let csv = format!("code;libelle;site\nI{sfx};Importée;S{sfx}\n");
    let (st, r) = appel(&app, "POST", "/structure/salles/import", Some(dir), Some(json!({"csv": csv}))).await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert_eq!((r["apercu"].clone(), r["creees"].clone()), (json!(true), json!(1)));
    let (_, l) = appel(&app, "GET", "/structure/salles", Some(dir), None).await;
    assert!(l.as_array().unwrap().iter().all(|x| x["code"] != format!("I{sfx}").as_str()), "aperçu = aucune écriture");
    let (st, r) = appel(&app, "POST", "/structure/salles/import", Some(dir), Some(json!({"csv": csv, "apercu": false}))).await;
    assert_eq!((st, r["creees"].clone()), (StatusCode::OK, json!(1)));

    // Profil : lecture vide, puis saisie ; paramètre inconnu refusé (400).
    let (st, p) = appel(&app, "GET", "/structure/profil", Some(prof), None).await;
    assert_eq!(st, StatusCode::OK);
    let _ = p;
    let (st, _) = appel(&app, "PUT", "/structure/parametres/structure.inconnue", Some(dir), Some(json!({"valeur": 1}))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = appel(&app, "PUT", "/structure/parametres/structure.trajet_defaut_min", Some(dir), Some(json!({"valeur": 10}))).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(&app, "PUT", "/structure/parametres/structure.trajet_defaut_min", Some(prof), Some(json!({"valeur": 10}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Journal lisible par le personnel.
    let (st, j) = appel(&app, "GET", "/structure/journal?objet=parametre&ref=structure.trajet_defaut_min", Some(prof), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(!j.as_array().unwrap().is_empty());
}
