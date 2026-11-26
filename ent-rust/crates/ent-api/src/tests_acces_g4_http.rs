//! Socle 27, lot G4-1 (HTTP) : `/acces/*` et l'ouverture d'une fiche hors périmètre (`GET /eleves/{id}`) répondent
//! comme le contrat le dit (session exigée, « motif requis » avec la liste S-542, motif inconnu refusé, jamais de nom
//! d'agent). Les règles métier sont dans `ent-db` (`tests_acces_g4.rs`). Base locale de labo ; l'heure du service est
//! celle de l'horloge du serveur (les routes n'ont pas d'horloge injectée).

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

async fn appel(app: &axum::Router, uri: &str, jeton: Option<&str>) -> (StatusCode, Value) {
    let mut b = Request::builder().method("GET").uri(uri);
    if let Some(j) = jeton {
        b = b.header("authorization", format!("Bearer {j}"));
    }
    let res = app
        .clone()
        .oneshot(b.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let st = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (st, v)
}

#[tokio::test]
async fn g4_http_consultations_motif_requis_et_refus() {
    let Some(url) = super::tests_l3::url_locale() else {
        eprintln!("skip G4 HTTP: ENT_DATABASE_URL");
        return;
    };
    let ent = PgEnt::connect(&url).await.unwrap();
    ent.migrate().await.unwrap();
    let state = AppState::nouveau(ent);
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let app = app(state.clone());
    let (prof, parent) = (cast.prof.token.as_str(), cast.parent_tom.token.as_str());

    // Session exigée.
    for uri in ["/acces/consultations", "/acces/motifs"] {
        let (st, _) = appel(&app, uri, None).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{uri}");
    }
    // Liste S-542.
    let (st, motifs) = appel(&app, "/acces/motifs", Some(prof)).await;
    assert_eq!(st, StatusCode::OK);
    assert!(motifs
        .as_array()
        .is_some_and(|m| m.len() >= 6 && m.iter().any(|x| x["code"] == "urgence")));

    // Dans le périmètre : aucun motif demandé.
    let (st, _) = appel(&app, &format!("/eleves/{}", cast.eleve_lea_id), Some(prof)).await;
    assert_eq!(st, StatusCode::OK, "fiche d'un élève de ses classes");
    // Hors périmètre : 409 « motif requis » avec la liste ; motif inconnu refusé ; motif valable servi.
    let hors = format!("/eleves/{}", cast.eleve_tom_id);
    let (st, corps) = appel(&app, &hors, Some(prof)).await;
    assert_eq!(st, StatusCode::CONFLICT);
    let corps: Value = match corps {
        Value::String(t) => serde_json::from_str(&t).expect("corps JSON motif_requis"),
        v => v,
    };
    assert_eq!(corps["code"], "motif_requis");
    assert!(corps["motifs"].as_array().is_some_and(|m| m.len() >= 6));
    let (st, _) = appel(&app, &format!("{hors}?motif=pour_voir"), Some(prof)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "motif hors liste");
    let (st, fiche) = appel(&app, &format!("{hors}?motif=urgence"), Some(prof)).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(fiche["id"], cast.eleve_tom_id.to_string());
    // Un élève inexistant répond « introuvable », jamais « motif requis » (aucun oracle d'existence).
    let (st, _) = appel(
        &app,
        &format!("/eleves/{}", uuid::Uuid::new_v4()),
        Some(prof),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // La famille voit la consultation motivée : fonction et service, jamais le nom de l'agent.
    let (st, page) = appel(&app, "/acces/consultations", Some(parent)).await;
    assert_eq!(st, StatusCode::OK);
    let texte = page.to_string();
    assert!(
        texte.contains("\"dossiers\"") && texte.contains("Urgence"),
        "{texte}"
    );
    assert!(
        !texte.contains("Rogue") && !texte.contains(&cast.prof.user_id.to_string()),
        "{texte}"
    );
}
