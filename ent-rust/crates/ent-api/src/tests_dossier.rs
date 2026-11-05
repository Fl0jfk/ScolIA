//! Socle 2, lot S2-1 : routes du dossier élève et des personnes (HTTP, base
//! locale de labo). Codes de retour, minimisation famille, effet immédiat de la
//! suspension, isolation entre établissements.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use ent_types::dossier::{FicheEleveDto, PersonneDto, ResponsableDto};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn etat() -> Option<AppState> {
    let url = std::env::var("ENT_DATABASE_URL").ok().filter(|u| !u.trim().is_empty())?;
    let mut ent = PgEnt::connect(&url).await.ok()?;
    ent.migrate().await.ok()?;
    Some(AppState::nouveau(ent))
}

async fn appel(
    app: &axum::Router,
    methode: &str,
    uri: &str,
    jeton: &str,
    corps: Option<Value>,
) -> (StatusCode, String) {
    let b = Request::builder()
        .method(methode)
        .uri(uri)
        .header("authorization", format!("Bearer {jeton}"));
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
    (st, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn dossier_routes_droits_minimisation_suspension_isolation() {
    let Some(state) = etat().await else {
        eprintln!("skip S2-1 HTTP: ENT_DATABASE_URL");
        return;
    };
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let autre = state.ent.seed_labo_cast().await.expect("cast B");
    let app = app(state);
    let harry = cast.eleve_lea_id;
    let dir = cast.direction.token.as_str();
    let parent = cast.parent_lea.token.as_str();

    // Création d'une personne sans e-mail : 201 (direction), 403 (prof, parent).
    let corps = json!({ "nom": "Potter", "prenom": "James", "telephone": "06 11 22 33 44" });
    for jeton in [cast.prof.token.as_str(), parent] {
        let (st, _) = appel(&app, "POST", "/dossier/personnes", jeton, Some(corps.clone())).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    let (st, txt) = appel(&app, "POST", "/dossier/personnes", dir, Some(corps)).await;
    assert_eq!(st, StatusCode::CREATED, "{txt}");
    let pere: PersonneDto = serde_json::from_str(&txt).unwrap();
    assert!(!pere.a_compte);
    // Saisie invalide → 400 ; personne d'un autre établissement → 404.
    let (st, _) = appel(&app, "POST", "/dossier/personnes", dir, Some(json!({ "nom": " " }))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = appel(&app, "GET", &format!("/dossier/personnes/{}", pere.id), &autre.direction.token, None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Lien au père (foyer distinct), puis doublon → 409.
    let lien = json!({ "personne_id": pere.id, "lien": "père", "autorite_parentale": true,
                       "peut_voir_notes": true, "peut_voir_vs": true });
    let uri = format!("/dossier/eleves/{harry}/responsables");
    let (st, txt) = appel(&app, "POST", &uri, dir, Some(lien.clone())).await;
    assert_eq!(st, StatusCode::CREATED, "{txt}");
    let (st, _) = appel(&app, "POST", &uri, dir, Some(lien)).await;
    assert_eq!(st, StatusCode::CONFLICT);

    // Fiche : direction voit coordonnées et actions ; parent voit le père sans coordonnées.
    let (st, txt) = appel(&app, "GET", &format!("/dossier/eleves/{harry}/fiche"), dir, None).await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    let f: FicheEleveDto = serde_json::from_str(&txt).unwrap();
    assert!(f.responsables.iter().any(|r| r.personne.id == pere.id && r.personne.telephone.is_some()));
    assert!(f.actions.iter().any(|a| a.code == "personnes.gerer"));
    let (st, txt) = appel(&app, "GET", &format!("/dossier/eleves/{harry}/fiche"), parent, None).await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    let f: FicheEleveDto = serde_json::from_str(&txt).unwrap();
    let vu = f.responsables.iter().find(|r| r.personne.id == pere.id).unwrap();
    assert_eq!(vu.personne.telephone, None);
    assert!(f.actions.is_empty() && f.alertes.is_empty());
    let (st, _) = appel(&app, "GET", &format!("/dossier/eleves/{harry}/fiche"), &cast.parent_tom.token, None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = appel(&app, "GET", &format!("/dossier/eleves/{harry}/chronologie"), parent, None).await;
    assert_eq!(st, StatusCode::OK);

    // Suspension du lien de la mère : 403 administratif, 200 direction, puis la
    // mère perd l'accès aux routes existantes ET nouvelles ; rétablissement.
    let (_, txt) = appel(&app, "GET", &uri, dir, None).await;
    let resp: Vec<ResponsableDto> = serde_json::from_str(&txt).unwrap();
    let mere = resp.iter().find(|r| r.personne.a_compte && r.lien != "père").unwrap();
    let susp = format!("/dossier/liens/{}/suspendre", mere.lien_id);
    let motif = json!({ "motif": "Ordonnance du juge", "reference": "JAF-1" });
    let (st, _) = appel(&app, "POST", &susp, &cast.administratif.token, Some(motif.clone())).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, txt) = appel(&app, "POST", &susp, dir, Some(motif.clone())).await;
    assert_eq!(st, StatusCode::OK, "{txt}");
    let (st, _) = appel(&app, "POST", &susp, dir, Some(motif)).await;
    assert_eq!(st, StatusCode::CONFLICT);
    for route in [
        format!("/eleves/{harry}"),
        format!("/eleves/{harry}/dossier"),
        format!("/dossier/eleves/{harry}/fiche"),
        format!("/dossier/eleves/{harry}/responsables"),
    ] {
        let (st, _) = appel(&app, "GET", &route, parent, None).await;
        assert!(
            st == StatusCode::NOT_FOUND || st == StatusCode::FORBIDDEN,
            "{route} doit être refusé après suspension : {st}"
        );
    }
    let (st, txt) = appel(&app, "GET", "/famille/enfants", parent, None).await;
    if st == StatusCode::OK {
        assert!(!txt.contains(&harry.to_string()), "enfant masqué après suspension");
    }
    let (st, _) = appel(&app, "POST", &format!("/dossier/liens/{}/retablir", mere.lien_id), dir, None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(&app, "GET", &format!("/dossier/eleves/{harry}/fiche"), parent, None).await;
    assert_eq!(st, StatusCode::OK);

    // Payeurs : parts ≠ 100 → 400 ; compta autorisée ; prof refusé.
    let pere_lien = resp.iter().find(|r| r.personne.id == pere.id).unwrap().lien_id;
    let uri_p = format!("/dossier/eleves/{harry}/payeurs");
    let (st, _) = appel(&app, "PUT", &uri_p, &cast.prof.token, Some(json!({ "payeurs": [] }))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "PUT", &uri_p, &cast.compta.token,
        Some(json!({ "payeurs": [{ "lien_id": mere.lien_id, "part": 70 }] }))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    // Le père n'a pas de foyer : refusé comme payeur.
    let (st, _) = appel(&app, "PUT", &uri_p, &cast.compta.token,
        Some(json!({ "payeurs": [{ "lien_id": mere.lien_id, "part": 50 }, { "lien_id": pere_lien, "part": 50 }] }))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = appel(&app, "PUT", &uri_p, &cast.compta.token,
        Some(json!({ "payeurs": [{ "lien_id": mere.lien_id, "part": 100 }] }))).await;
    assert_eq!(st, StatusCode::OK);

    // Fusion : 403 administratif ; aller-retour par la direction.
    let (_, txt) = appel(&app, "POST", "/dossier/personnes", dir, Some(json!({ "nom": "Potter", "prenom": "Jamie" }))).await;
    let dup: PersonneDto = serde_json::from_str(&txt).unwrap();
    let corps = json!({ "source_id": dup.id, "cible_id": pere.id, "motif": "Doublon" });
    let (st, _) = appel(&app, "POST", "/dossier/fusions", &cast.administratif.token, Some(corps.clone())).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, txt) = appel(&app, "POST", "/dossier/fusions", dir, Some(corps)).await;
    assert_eq!(st, StatusCode::CREATED, "{txt}");
    let id = serde_json::from_str::<Value>(&txt).unwrap()["id"].as_str().unwrap().to_string();
    let (st, _) = appel(&app, "POST", &format!("/dossier/fusions/{id}/annuler"), dir, None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(&app, "POST", &format!("/dossier/fusions/{id}/annuler"), &autre.direction.token, None).await;
    assert_eq!(st, StatusCode::NOT_FOUND, "fusion d'un autre établissement");
}
