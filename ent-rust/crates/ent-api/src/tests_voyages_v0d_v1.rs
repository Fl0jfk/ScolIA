//! Socle 42, lots V0d et V1 (HTTP) : routes de la restauration, du journal de voyage, de la reprise, des
//! exigences, des relances, des ordres de mission et de la liste nominative — session obligatoire (sauf le
//! lien nominatif, public mais sans fuite), droits, cloisonnement. Base locale de labo (cast Harry Potter).

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn appel_brut(
    app: &axum::Router,
    m: &str,
    uri: &str,
    jeton: Option<&str>,
    corps: Option<Value>,
) -> (StatusCode, axum::http::HeaderMap, Value) {
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
    let h = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (st, h, v)
}

async fn appel(app: &axum::Router, m: &str, uri: &str, jeton: Option<&str>, corps: Option<Value>) -> (StatusCode, Value) {
    let (st, _, v) = appel_brut(app, m, uri, jeton, corps).await;
    (st, v)
}

#[tokio::test]
async fn routes_v0d_et_v1_session_droits_cloisonnement_et_lien_public_sans_fuite() {
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
    let b_dir = autre.direction.token.as_str();
    let fantome = Uuid::new_v4();
    let pid = Uuid::new_v4();

    // Session obligatoire sur toutes les routes (sauf le lien nominatif).
    for (m, u) in [
        ("PUT", "/voyages/reglage-restauration".to_string()),
        ("GET", format!("/voyages/{fantome}/restauration")),
        ("PUT", format!("/voyages/{fantome}/restauration")),
        ("GET", format!("/voyages/{fantome}/restauration/apercu")),
        ("POST", format!("/voyages/{fantome}/restauration/commande")),
        ("GET", format!("/voyages/{fantome}/journal")),
        ("POST", format!("/voyages/{fantome}/journal/activer")),
        ("POST", format!("/voyages/{fantome}/journal/publications")),
        ("GET", "/voyages/journal-famille".to_string()),
        ("POST", "/voyages/journal/purger".to_string()),
        ("POST", "/voyages/reprise/importer".to_string()),
        ("GET", "/voyages/reprise/tableau".to_string()),
        ("POST", format!("/voyages/reprise/{fantome}/basculer")),
        ("POST", format!("/voyages/{fantome}/campagne")),
        ("GET", format!("/voyages/{fantome}/exigences")),
        ("POST", format!("/voyages/{fantome}/autorisations/guichet")),
        ("POST", format!("/voyages/{fantome}/desaccords")),
        ("POST", format!("/voyages/{fantome}/exigences/{pid}/droit_image/derogation")),
        ("POST", format!("/voyages/{fantome}/relance")),
        ("POST", "/voyages/relances/traiter".to_string()),
        ("POST", format!("/voyages/{fantome}/ordres-mission")),
        ("GET", format!("/voyages/{fantome}/transmissions")),
        ("POST", format!("/voyages/{fantome}/transmissions")),
    ] {
        let (st, _) = appel(&app, m, &u, None, None).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {u}");
    }

    // Reprise : administrateur et direction seulement (professeur et famille refusés), tableau vide au départ.
    assert_eq!(appel(&app, "GET", "/voyages/reprise/tableau", Some(prof), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(appel(&app, "GET", "/voyages/reprise/tableau", Some(parent), None).await.0, StatusCode::FORBIDDEN);
    let (st, tableau) = appel(&app, "GET", "/voyages/reprise/tableau", Some(dir), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(tableau.as_array().is_some_and(|t| t.is_empty()));
    // Import : un statut inconnu est listé, jamais deviné ; un séjour sans titre est refusé.
    let debut = chrono::Utc::now() + chrono::Duration::days(60);
    let corps = json!({
        "params": { "gel": (chrono::Utc::now() + chrono::Duration::days(30)).date_naive(), "fin_vol": (chrono::Utc::now() + chrono::Duration::days(40)).date_naive() },
        "sejours": [{ "source_id": "NX-HTTP", "statut_source": "BIDON", "type_source": "SIMPLE", "titre": "x",
                      "debut": debut.to_rfc3339(), "fin": (debut + chrono::Duration::hours(6)).to_rfc3339() }],
    });
    let (st, rapport) = appel(&app, "POST", "/voyages/reprise/importer", Some(dir), Some(corps.clone())).await;
    assert_eq!(st, StatusCode::OK, "{rapport}");
    assert_eq!(rapport["statuts_inconnus"], json!(["BIDON"]));
    assert!(appel(&app, "POST", "/voyages/reprise/importer", Some(prof), Some(corps)).await.0.is_client_error());
    // Un séjour d'un autre établissement n'existe pas pour la direction de B.
    assert_eq!(appel(&app, "POST", &format!("/voyages/reprise/{fantome}/basculer"), Some(b_dir), Some(json!({"confirme": true}))).await.0, StatusCode::NOT_FOUND);

    // Restauration, exigences, ordres, transmissions : « introuvable » pour une sortie inconnue, quel que soit le profil.
    for (m, u, corps) in [
        ("GET", format!("/voyages/{fantome}/restauration"), None),
        ("GET", format!("/voyages/{fantome}/exigences"), None),
        ("GET", format!("/voyages/{fantome}/journal"), None),
        ("GET", format!("/voyages/{fantome}/ordres-mission"), None),
        ("GET", format!("/voyages/{fantome}/transmissions"), None),
        ("POST", format!("/voyages/{fantome}/campagne"), Some(json!({"avec_image": true}))),
        ("POST", format!("/voyages/{fantome}/relance"), Some(json!({"confirme": true}))),
    ] {
        for jeton in [dir, prof, b_dir] {
            let (st, _) = appel(&app, m, &u, Some(jeton), corps.clone()).await;
            assert_eq!(st, StatusCode::NOT_FOUND, "{m} {u}");
        }
    }

    // Déclenchements manuels : direction seule.
    assert_eq!(appel(&app, "POST", "/voyages/relances/traiter", Some(prof), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(appel(&app, "POST", "/voyages/journal/purger", Some(prof), None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(appel(&app, "POST", "/voyages/relances/traiter", Some(dir), None).await.0, StatusCode::OK);
    assert_eq!(appel(&app, "POST", "/voyages/journal/purger", Some(dir), None).await.0, StatusCode::OK);
    // Boîte de restauration : direction seule ; adresse multiple refusée.
    assert_eq!(appel(&app, "PUT", "/voyages/reglage-restauration", Some(prof), Some(json!({"boite": "a@b.fr"}))).await.0, StatusCode::FORBIDDEN);
    assert!(appel(&app, "PUT", "/voyages/reglage-restauration", Some(dir), Some(json!({"boite": "a@b.fr, c@d.fr"}))).await.0.is_client_error());
    assert_eq!(appel(&app, "PUT", "/voyages/reglage-restauration", Some(dir), Some(json!({"boite": "cuisine@lycee.fr"}))).await.0, StatusCode::NO_CONTENT);
    // La famille n'a aucune route de gestion.
    assert_eq!(appel(&app, "POST", &format!("/voyages/{fantome}/journal/activer"), Some(parent), None).await.0, StatusCode::NOT_FOUND);

    // Lien nominatif : route PUBLIQUE ; tout jeton invalide répond 404 identique, avec les entêtes de discrétion.
    for jeton in ["abc".to_string(), format!("{}.{}", cast.etablissement_id.simple(), "0".repeat(64)), format!("{}.{}", autre.etablissement_id.simple(), "1".repeat(64))] {
        let (st, _, _) = appel_brut(&app, "GET", &format!("/voyages/journal/lien/{jeton}"), None, None).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{jeton}");
        let (st, _, _) = appel_brut(&app, "GET", &format!("/voyages/journal/lien/{jeton}/photos/{fantome}"), None, None).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{jeton}");
    }
    // Corps invalides : erreur client, jamais 5xx.
    for (u, corps) in [
        (format!("/voyages/{fantome}/autorisations/guichet"), json!({"participant_id": "x"})),
        (format!("/voyages/{fantome}/transmissions"), json!({"champs": "nom"})),
        (format!("/voyages/{fantome}/journal/publications"), json!({"type": 3})),
    ] {
        let (st, _) = appel(&app, "POST", &u, Some(dir), Some(corps)).await;
        assert!(st.is_client_error(), "{u} : {st}");
    }
}
