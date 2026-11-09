//! Socle 5, lot S4 (HTTP) : routes `/structure/cours*` — session obligatoire, matrice de droits,
//! cloisonnement entre établissements, poste vacant, conflit et refus. Base locale de labo.

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
async fn routes_cours_droits_cloisonnement_poste_vacant_et_conflits() {
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
    let etab = cast.etablissement_id;
    let annee = state
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap();
    let sfx = uuid::Uuid::new_v4().simple().to_string()[..6].to_uppercase();
    let classe = state
        .ent
        .referentiel_creer_classe(
            "administratif",
            etab,
            Some(annee.id),
            &format!("C4{sfx}"),
            None,
            Some("4EME"),
        )
        .await
        .unwrap();
    let app = app(state.clone());
    let (dir, adm, prof, parent) = (
        cast.direction.token.as_str(),
        cast.administratif.token.as_str(),
        cast.prof.token.as_str(),
        cast.parent_lea.token.as_str(),
    );
    let b_dir = autre.direction.token.as_str();

    // Session obligatoire.
    for (m, u) in [
        ("GET", "/structure/cours"),
        ("POST", "/structure/cours"),
        ("GET", "/structure/cours/vacants"),
        ("GET", "/structure/cours/reprise"),
    ] {
        let (st, _) = appel(&app, m, u, None, None).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {u}");
    }
    // Lecture : personnel oui, famille non. Écriture : gestionnaires seulement.
    assert_eq!(
        appel(&app, "GET", "/structure/cours", Some(prof), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        appel(&app, "GET", "/structure/cours", Some(parent), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let corps = json!({"classe_id": classe.id, "libelle": "Soutien de maths", "heures_hebdo": 2, "semaines": "toutes"});
    assert_eq!(
        appel(
            &app,
            "POST",
            "/structure/cours",
            Some(prof),
            Some(corps.clone())
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Valeurs invalides : 400.
    for mauvais in [
        json!({"libelle": "Sans public"}),
        json!({"classe_id": classe.id}),
        json!({"classe_id": classe.id, "libelle": "x", "semaines": "Z"}),
    ] {
        let (st, _) = appel(&app, "POST", "/structure/cours", Some(dir), Some(mauvais)).await;
        assert!(
            st == StatusCode::BAD_REQUEST || st == StatusCode::UNPROCESSABLE_ENTITY,
            "{st}"
        );
    }
    // Direction : création (201) d'un poste vacant ; second identique : 409.
    let (st, c) = appel(
        &app,
        "POST",
        "/structure/cours",
        Some(dir),
        Some(corps.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    assert_eq!(c["vacant"], json!(true));
    assert_eq!(c["nb_services"], json!(0));
    let id = c["id"].as_str().unwrap().to_string();
    assert_eq!(
        appel(&app, "POST", "/structure/cours", Some(adm), Some(corps))
            .await
            .0,
        StatusCode::CONFLICT
    );
    // Liste et postes vacants.
    let (_, v) = appel(&app, "GET", "/structure/cours/vacants", Some(prof), None).await;
    assert!(v.as_array().unwrap().iter().any(|x| x["id"] == json!(id)));
    // Détail avec les professeurs : gestion seulement.
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/structure/cours/{id}"),
            Some(prof),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (st, d) = appel(
        &app,
        "GET",
        &format!("/structure/cours/{id}"),
        Some(adm),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(d["services"].as_array().unwrap().len(), 0);
    // Cloisonnement : l'autre établissement ne voit ni ne modifie ni ne ferme ni ne pourvoit ce cours.
    for (m, u, c) in [
        ("GET", format!("/structure/cours/{id}"), None),
        (
            "PUT",
            format!("/structure/cours/{id}"),
            Some(json!({"libelle": "Piraté"})),
        ),
        (
            "POST",
            format!("/structure/cours/{id}/fermer"),
            Some(json!({"a_la_date": "2026-01-05"})),
        ),
        (
            "POST",
            format!("/structure/cours/{id}/services"),
            Some(json!({"prof_user_id": autre.prof.user_id})),
        ),
        ("GET", format!("/referentiel/cours/{id}/membres"), None),
    ] {
        let (st, _) = appel(&app, m, &u, Some(b_dir), c).await;
        assert!(
            st == StatusCode::NOT_FOUND || st == StatusCode::FORBIDDEN,
            "{m} {u} → {st}"
        );
    }
    // Pourvoir : professeur du personnel (201), doublon (409), inconnu (404).
    let pose = json!({"prof_user_id": cast.prof.user_id});
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/structure/cours/{id}/services"),
        Some(dir),
        Some(pose.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/structure/cours/{id}/services"),
            Some(dir),
            Some(pose)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/structure/cours/{id}/services"),
            Some(dir),
            Some(json!({"prof_user_id": uuid::Uuid::new_v4()}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (_, d) = appel(
        &app,
        "GET",
        &format!("/structure/cours/{id}"),
        Some(dir),
        None,
    )
    .await;
    assert_eq!(
        (d["nb_services"].clone(), d["vacant"].clone()),
        (json!(1), json!(false))
    );
    // Le professeur lit son cours : public borné (liste vide ou non, mais pas d'erreur).
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/referentiel/cours/{id}/public"),
        Some(prof),
        None,
    )
    .await;
    assert!(st == StatusCode::OK, "{st}");
    // Modification, fermeture, refus de la seconde fermeture.
    let (st, m) = appel(
        &app,
        "PUT",
        &format!("/structure/cours/{id}"),
        Some(adm),
        Some(json!({"libelle": "Soutien maths 4e"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{m}");
    assert_eq!(m["libelle"], json!("Soutien maths 4e"));
    let jour = chrono::Utc::now().date_naive();
    let (st, f) = appel(
        &app,
        "POST",
        &format!("/structure/cours/{id}/fermer"),
        Some(adm),
        Some(json!({"a_la_date": jour})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{f}");
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/structure/cours/{id}/fermer"),
            Some(adm),
            Some(json!({"a_la_date": jour}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // État de la reprise (M7) : aucun service sans cours.
    let (st, r) = appel(&app, "GET", "/structure/cours/reprise", Some(dir), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(r["services"], r["services_avec_cours"], "{r}");
    assert_eq!(
        appel(&app, "GET", "/structure/cours/reprise", Some(prof), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
}
