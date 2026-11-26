//! Socle 27, lot G3b (HTTP) : les routes `/rgpd/demandes/{id}/extraits/*` et `/rgpd/reglages` répondent comme le
//! contrat le dit (session exigée, droits, neutralité des refus). Les règles métier sont dans `ent-db`
//! (`tests_rgpd_g3b*.rs`). Base locale de labo.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use ent_db::droits_demandes::SaisieDemande;
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

#[tokio::test]
async fn g3b_http_extrait_parcours_et_refus() {
    let Some(url) = super::tests_l3::url_locale() else {
        eprintln!("skip G3b HTTP: ENT_DATABASE_URL");
        return;
    };
    let ent = PgEnt::connect(&url).await.unwrap();
    ent.migrate().await.unwrap();
    let state = AppState::nouveau(ent);
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let (etab, dir_id) = (cast.etablissement_id, cast.direction.user_id);
    let app = app(state.clone());
    let (dir, adf, prof, parent) = (
        cast.direction.token.as_str(),
        cast.administratif.token.as_str(),
        cast.prof.token.as_str(),
        cast.parent_lea.token.as_str(),
    );
    let jour = state.ent.aujourdhui_etablissement(etab).await.unwrap();
    let v = state
        .ent
        .droits_saisir_au_guichet(
            etab,
            dir_id,
            "direction",
            SaisieDemande {
                canal: "courrier",
                recue_le: jour,
                demandeur_compte_id: None,
                demandeur_personne_id: None,
                demandeur_nom_saisi: Some("Famille fictive"),
                demandeur_contact: Some("courrier postal"),
                qualite: "soi_meme",
                sujet_type: "eleve",
                sujet_id: Some(cast.eleve_lea_id),
                sujet_nom_saisi: None,
                droits: &["acces"],
                perimetre_texte: None,
                perimetre_du: None,
                perimetre_au: None,
            },
        )
        .await
        .unwrap();
    let id = v.id;
    state
        .ent
        .droits_verifier_identite(
            etab,
            dir_id,
            "direction",
            id,
            "code_courriel",
            true,
            None,
            None,
        )
        .await
        .unwrap();

    // Sans session : 401 (la route existe).
    for (m, uri) in [
        ("GET", format!("/rgpd/demandes/{id}/extraits")),
        ("POST", format!("/rgpd/demandes/{id}/extraits")),
        ("GET", "/rgpd/reglages".to_string()),
    ] {
        let (st, _) = appel(&app, m, &uri, None, Some(json!({}))).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {uri}");
    }
    // Professeur et famille : 403.
    for jeton in [prof, parent] {
        let (st, _) = appel(
            &app,
            "POST",
            &format!("/rgpd/demandes/{id}/extraits"),
            Some(jeton),
            Some(json!({})),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN);
        let (st, _) = appel(
            &app,
            "GET",
            &format!("/rgpd/demandes/{id}/extraits"),
            Some(jeton),
            None,
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    // Un seul des deux champs de sujet : 400.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/extraits"),
        Some(adf),
        Some(json!({ "sujet_type": "eleve" })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    // Demande inconnue : 404.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{}/extraits", Uuid::new_v4()),
        Some(dir),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Assemblage, structure, contenu, marquages.
    let (st, e) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/extraits"),
        Some(dir),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{e}");
    let extrait = e[0]["id"].as_str().unwrap().to_string();
    assert_eq!(e[0]["empreinte"].as_str().unwrap().len(), 64);
    let (st, c) = appel(
        &app,
        "GET",
        &format!("/rgpd/demandes/{id}/extraits/{extrait}/contenu"),
        Some(dir),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(c["empreinte"], e[0]["empreinte"]);
    let (st, m) = appel(
        &app,
        "GET",
        &format!("/rgpd/demandes/{id}/extraits/{extrait}/masquages"),
        Some(dir),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(m.is_array());
    // Extrait d'une autre demande : 404.
    let (st, _) = appel(
        &app,
        "GET",
        &format!(
            "/rgpd/demandes/{}/extraits/{extrait}/contenu",
            Uuid::new_v4()
        ),
        Some(dir),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Lot G3b-3 : remise, « Mes données », renvois, clôture consignée, pièces à choisir.
    let defi_uri = format!("/rgpd/demandes/{id}/extraits/{extrait}/remise/defi");
    for (m, uri) in [
        ("GET", "/rgpd/mes-donnees".to_string()),
        ("POST", defi_uri.clone()),
        ("GET", format!("/rgpd/demandes/{id}/renvois")),
        ("GET", format!("/rgpd/demandes/{id}/cloture")),
        ("GET", format!("/rgpd/demandes/{id}/pieces-identite")),
    ] {
        let (st, _) = appel(&app, m, &uri, None, Some(json!({}))).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {uri}");
    }
    // Le secrétariat n'a pas `droits.remettre` : 403.
    let (st, _) = appel(&app, "POST", &defi_uri, Some(adf), Some(json!({}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // La direction : extrait non remissible (marquage ouvert) ou sans passkey enregistrée → 409.
    let (st, r) = appel(&app, "POST", &defi_uri, Some(dir), Some(json!({}))).await;
    assert_eq!(st, StatusCode::CONFLICT, "{r}");
    // Un extrait inconnu : 404.
    let (st, _) = appel(
        &app,
        "POST",
        &format!(
            "/rgpd/demandes/{id}/extraits/{}/remise/defi",
            Uuid::new_v4()
        ),
        Some(dir),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Remise sans assertion valide : 400 (rien n'est tenté).
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/extraits/{extrait}/remise"),
        Some(dir),
        Some(json!({ "mode": "en_ligne", "defi_id": Uuid::new_v4(), "credential": {} })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    // « Mes données » : une famille sans extrait voit une liste vide ; un téléchargement sans assertion : 400 ;
    // un défi sur un extrait qui n'est pas le sien : 404.
    let (st, mes) = appel(&app, "GET", "/rgpd/mes-donnees", Some(parent), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(mes, json!([]));
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/mes-donnees/extraits/{extrait}/defi"),
        Some(parent),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/mes-donnees/fichiers/{}/telecharger", Uuid::new_v4()),
        Some(parent),
        Some(json!({ "defi_id": Uuid::new_v4(), "credential": {} })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    // Renvois : réservés à `droits.traiter` ; nature inconnue 400 ; trace puis liste.
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/rgpd/demandes/{id}/renvois"),
        Some(parent),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/renvois"),
        Some(dir),
        Some(json!({ "nature": "inconnue", "cible": "x", "texte": "texte" })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, r) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/renvois"),
        Some(dir),
        Some(json!({ "nature": "rectification", "cible": "SIECLE", "texte": "date de naissance à corriger" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert!(r["texte"]
        .as_str()
        .unwrap()
        .contains("à corriger dans la source"));
    let (st, l) = appel(
        &app,
        "GET",
        &format!("/rgpd/demandes/{id}/renvois"),
        Some(dir),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(l.as_array().unwrap().len(), 1);
    // Aucune clôture consignée tant que la demande est ouverte : 404.
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/rgpd/demandes/{id}/cloture"),
        Some(dir),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    // Pièces d'identité à choisir (Pf7) : liste pour le guichet, 403 pour la famille.
    let (st, p) = appel(
        &app,
        "GET",
        &format!("/rgpd/demandes/{id}/pieces-identite"),
        Some(dir),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{p}");
    assert!(p.is_array());
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/rgpd/demandes/{id}/pieces-identite"),
        Some(parent),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Réglage : lecture par la direction, valeur hors bornes refusée.
    let (st, r) = appel(&app, "GET", "/rgpd/reglages", Some(dir), None).await;
    assert_eq!(st, StatusCode::OK, "{r}");
    let (st, _) = appel(
        &app,
        "POST",
        "/rgpd/reglages",
        Some(dir),
        Some(json!({ "reglage": "plafond_depots_publics_jour", "valeur": 0 })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}
