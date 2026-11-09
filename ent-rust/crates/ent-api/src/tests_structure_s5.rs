//! Socle 5, lot S5 (HTTP) : routes d'offre et d'enseignements suivis — session obligatoire,
//! matrice de droits, cloisonnement entre établissements, refus 409 par règle de choix, périmètre
//! D-022 du professeur. Base locale de labo.

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
async fn routes_offre_et_enseignements_droits_cloisonnement_et_409() {
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
    let eleve = cast.eleve_lea_id;
    let app = app(state.clone());
    let (dir, adm, prof, parent) = (
        cast.direction.token.as_str(),
        cast.administratif.token.as_str(),
        cast.prof.token.as_str(),
        cast.parent_lea.token.as_str(),
    );
    let sfx = uuid::Uuid::new_v4().simple().to_string()[..6].to_uppercase();

    // Session obligatoire.
    for (m, u) in [
        ("GET", "/structure/matieres"),
        ("GET", "/structure/offres"),
        ("POST", "/structure/offres"),
        ("GET", "/structure/enseignements-incoherences"),
    ] {
        let (st, _) = appel(&app, m, u, None, None).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {u}");
    }
    // Lecture du catalogue : personnel oui, famille non. Écriture : gestionnaires seulement.
    let (st, _) = appel(&app, "GET", "/structure/offres", Some(prof), None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(&app, "GET", "/structure/offres", Some(parent), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let corps_offre = |code: &str, modalite: &str| json!({"annee_id": annee.id, "code": format!("{code}{sfx}"), "libelle": code, "modalite": modalite});
    let (st, _) = appel(
        &app,
        "POST",
        "/structure/offres",
        Some(prof),
        Some(corps_offre("X", "autre")),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(
        &app,
        "POST",
        "/structure/matieres",
        Some(prof),
        Some(json!({"code": "ZZ", "libelle": "Zz"})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Direction : matière (201), deux LV2 et une règle « exactement 1 ».
    let (st, m) = appel(
        &app,
        "POST",
        "/structure/matieres",
        Some(dir),
        Some(json!({"code": format!("ESP{sfx}"), "libelle": "Espagnol", "couleur": "#aa00cc"})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{m}");
    let (st, esp) = appel(
        &app,
        "POST",
        "/structure/offres",
        Some(adm),
        Some(corps_offre("ESP", "langue_2")),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{esp}");
    let (st, all) = appel(
        &app,
        "POST",
        "/structure/offres",
        Some(adm),
        Some(corps_offre("ALL", "langue_2")),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    let (esp_id, all_id) = (
        esp["id"].as_str().unwrap().to_string(),
        all["id"].as_str().unwrap().to_string(),
    );
    let (st, _) = appel(&app, "POST", "/structure/regles-choix", Some(adm), Some(json!({
        "annee_id": annee.id, "libelle": "LV2 : exactement 1", "offre_ids": [esp_id, all_id], "min": 1, "max": 1}))).await;
    assert_eq!(st, StatusCode::CREATED);
    // Valeur invalide : 400 ; code déjà pris : 409.
    let (st, _) = appel(
        &app,
        "POST",
        "/structure/offres",
        Some(adm),
        Some(corps_offre("Y", "inconnue")),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = appel(
        &app,
        "POST",
        "/structure/offres",
        Some(adm),
        Some(corps_offre("ESP", "langue_2")),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);

    // Enseignements suivis : une LV2 → 200 ; une seconde → 409 avec les écarts ; aperçu → 200 sans écriture.
    let uri = format!("/structure/eleves/{eleve}/enseignements");
    let (st, r) = appel(
        &app,
        "POST",
        &uri,
        Some(adm),
        Some(json!({"annee_id": annee.id, "ajouter": [{"offre_id": esp_id}]})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert_eq!(r["ecrit"], json!(true));
    let (st, r) = appel(
        &app,
        "POST",
        &uri,
        Some(adm),
        Some(json!({"annee_id": annee.id, "ajouter": [{"offre_id": all_id}]})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "{r}");
    assert!(
        r["violations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["code"] == json!("regle_max") && v["bloquant"] == json!(true)),
        "{r}"
    );
    let (st, r) = appel(
        &app,
        "POST",
        &uri,
        Some(adm),
        Some(json!({
        "annee_id": annee.id, "apercu": true,
        "quitter": [{"offre_id": esp_id, "fin_nature": "changement_en_cours_annee", "motif": "x"}],
        "ajouter": [{"offre_id": all_id}]})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert_eq!(r["ecrit"], json!(false));
    let (st, l) = appel(
        &app,
        "GET",
        &format!("{uri}?annee_id={}&actifs=true", annee.id),
        Some(adm),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    // (le labo peut déjà porter d'autres enseignements repris des anciennes options)
    let sur_esp = |l: &Value| {
        l.as_array()
            .unwrap()
            .iter()
            .filter(|e| e["offre_id"] == json!(esp_id))
            .count()
    };
    assert_eq!(
        sur_esp(&l),
        1,
        "l'aperçu n'a rien écrit, le choix réel est unique"
    );
    // Le professeur ne modifie rien ; la famille ne lit rien ici.
    let (st, _) = appel(
        &app,
        "POST",
        &uri,
        Some(prof),
        Some(json!({"annee_id": annee.id, "ajouter": [{"offre_id": all_id}]})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "GET", &uri, Some(parent), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Écarts « choix sans groupe » et reprise : gestionnaires seulement.
    let (st, _) = appel(
        &app,
        "GET",
        &format!(
            "/structure/enseignements-incoherences?annee_id={}",
            annee.id
        ),
        Some(adm),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(
        &app,
        "GET",
        &format!(
            "/structure/enseignements-incoherences?annee_id={}",
            annee.id
        ),
        Some(prof),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, c) = appel(
        &app,
        "GET",
        "/structure/enseignements-reprise",
        Some(adm),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        c["heritees"].as_i64().unwrap(),
        c["reprises"].as_i64().unwrap() + c["a_traiter"].as_i64().unwrap()
    );

    // Fermeture : aperçu (200) ; l'administratif ne ferme pas (403) ; la direction oui.
    let uri_f = format!("/structure/offres/{esp_id}/fermer");
    let (st, ap) = appel(
        &app,
        "POST",
        &uri_f,
        Some(dir),
        Some(json!({"motif": "pas d'effectif"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(ap["ferme"], json!(false));
    let (st, _) = appel(
        &app,
        "POST",
        &uri_f,
        Some(adm),
        Some(json!({"motif": "x", "confirmer": true})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, ap) = appel(
        &app,
        "POST",
        &uri_f,
        Some(dir),
        Some(json!({"motif": "pas d'effectif", "confirmer": true})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(ap["ferme"], json!(true));

    // Cloisonnement : un autre établissement ne voit ni l'offre (404), ni l'élève (404), ne modifie rien.
    let adir = autre.direction.token.as_str();
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/structure/offres/{esp_id}"),
        Some(adir),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = appel(
        &app,
        "PUT",
        &format!("/structure/offres/{esp_id}"),
        Some(adir),
        Some(json!({"libelle": "Piraté"})),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = appel(&app, "GET", &uri, Some(adir), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, r) = appel(
        &app,
        "POST",
        &uri,
        Some(adir),
        Some(json!({"annee_id": annee.id, "ajouter": [{"offre_id": all_id}]})),
    )
    .await;
    assert!(
        st == StatusCode::NOT_FOUND || st == StatusCode::FORBIDDEN,
        "{st} {r}"
    );
    let (st, l) = appel(&app, "GET", "/structure/offres", Some(adir), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(l
        .as_array()
        .unwrap()
        .iter()
        .all(|o| o["id"] != json!(esp_id)));
}
