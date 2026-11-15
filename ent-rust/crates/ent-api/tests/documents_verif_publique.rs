//! Socle 7, lot D3 : routes **publiques sans session** de vérification des documents (`/verifier/{code}`,
//! `POST /verifier`), contre le vrai routeur `ent-api` et une base PostgreSQL locale de test (`ENT_DATABASE_URL`,
//! sinon SKIP). Les cas métier (trois modes, remplacé, annulé) sont dans `ent-db/src/tests_documents_d3.rs` ; ici :
//!
//! * l'établissement vient de l'**adresse** : adresse inconnue → 404, jamais celui du client ;
//! * un code mal formé, inexistant ou trop long répond **la même chose** (200 `inconnu`), sans différence de forme ;
//! * toutes les réponses portent `x-robots-tag: noindex, nofollow` et `cache-control: no-store` ;
//! * le dépôt de fichier : octets quelconques → `different` ; plus de 10 Mio → 413 ; rien n'est conservé.
//!
//! Les variables `ENT_TENANT_PAR_HOTE` / `ENT_TRUST_PROXY` sont modifiées : les tests de ce binaire sont sérialisés.

// Le verrou global sérialise les tests qui modifient l'environnement du processus (comme `tenant_hote.rs`).
#![allow(clippy::await_holding_lock)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ent_api::{app, AppState};
use ent_db::PgEnt;
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::Mutex;
use tower::ServiceExt;
use uuid::Uuid;

static VERROU: Mutex<()> = Mutex::new(());

struct Reponse {
    statut: StatusCode,
    robots: Option<String>,
    cache: Option<String>,
    corps: Value,
}

struct Monde {
    app: axum::Router,
    hote: String,
}

async fn monde() -> Option<Monde> {
    let url = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty());
    let Some(url) = url else {
        eprintln!("SKIP documents_verif_publique : ENT_DATABASE_URL absent");
        return None;
    };
    let ent = PgEnt::connect(&url).await.expect("connexion");
    ent.migrate().await.expect("migrations");
    let state = AppState::nouveau(ent);
    let etab = Uuid::new_v4();
    let suffixe = Uuid::new_v4().simple().to_string();
    let hote = format!("doc-{}.test.scolia.fr", &suffixe[..12]);
    state
        .ent
        .declarer_etablissement_plateforme(
            etab,
            &format!("doc{}", &suffixe[..12]),
            "Établissement de vérification",
        )
        .await
        .expect("déclaration");
    state
        .ent
        .ajouter_domaine_plateforme(etab, &hote)
        .await
        .expect("domaine");
    Some(Monde {
        app: app(state),
        hote,
    })
}

impl Monde {
    async fn appel(&self, methode: &str, chemin: &str, hote: &str, corps: Vec<u8>) -> Reponse {
        let req = Request::builder()
            .method(methode)
            .uri(chemin)
            .header("host", hote)
            .body(Body::from(corps))
            .unwrap();
        let res = self.app.clone().oneshot(req).await.unwrap();
        let (statut, en_tetes) = (res.status(), res.headers().clone());
        let octets = res.into_body().collect().await.unwrap().to_bytes();
        let valeur = |k: &str| {
            en_tetes
                .get(k)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        Reponse {
            statut,
            robots: valeur("x-robots-tag"),
            cache: valeur("cache-control"),
            corps: serde_json::from_slice(&octets)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&octets).into_owned())),
        }
    }
}

fn verrou() -> std::sync::MutexGuard<'static, ()> {
    VERROU.lock().unwrap_or_else(|e| e.into_inner())
}

#[tokio::test]
async fn verification_par_code_sans_session_reponse_uniforme() {
    let _v = verrou();
    std::env::remove_var("ENT_TENANT_PAR_HOTE");
    std::env::remove_var("ENT_TRUST_PROXY");
    std::env::set_var("ENT_RATE_LIMIT", "off");
    let Some(m) = monde().await else { return };
    // Adresse inconnue : 404 (l'établissement ne se choisit pas).
    let r = m
        .appel(
            "GET",
            "/verifier/ABCDEFGHJKLM",
            "inconnu.example.org",
            vec![],
        )
        .await;
    assert_eq!(r.statut, StatusCode::NOT_FOUND);
    // Codes inconnus, mal formés, numéro séquentiel, trop long, caractères hors alphabet : UNE réponse, au caractère près.
    let mut vues = Vec::new();
    for code in [
        "ABCDEFGHJKLM",
        "abcd-efgh-jklm",
        "0000-0000-0000",
        "2026-SCO-000123",
        "x",
        "%C3%89%C3%89%C3%89%C3%89%C3%89%C3%89%C3%89%C3%89%C3%89%C3%89%C3%89%C3%89",
        "ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ",
    ] {
        let r = m
            .appel("GET", &format!("/verifier/{code}"), &m.hote, vec![])
            .await;
        assert_eq!(r.statut, StatusCode::OK, "{code}");
        assert_eq!(r.robots.as_deref(), Some("noindex, nofollow"), "{code}");
        assert!(
            r.cache.as_deref().is_some_and(|c| c.contains("no-store")),
            "{code} : {:?}",
            r.cache
        );
        vues.push((code, r.corps));
    }
    for (code, corps) in &vues {
        assert_eq!(
            corps,
            &serde_json::json!({ "resultat": "inconnu" }),
            "{code}"
        );
    }
}

#[tokio::test]
async fn verification_par_depot_sans_session() {
    let _v = verrou();
    std::env::remove_var("ENT_TENANT_PAR_HOTE");
    std::env::remove_var("ENT_TRUST_PROXY");
    std::env::set_var("ENT_RATE_LIMIT", "off");
    let Some(m) = monde().await else { return };
    let r = m
        .appel("POST", "/verifier", "inconnu.example.org", b"%PDF".to_vec())
        .await;
    assert_eq!(r.statut, StatusCode::NOT_FOUND);
    // Un fichier quelconque (ou vide) n'est jamais « identique » ; la réponse ne dit rien de plus.
    for octets in [b"%PDF-1.4 un fichier quelconque".to_vec(), Vec::new()] {
        let r = m.appel("POST", "/verifier", &m.hote, octets).await;
        assert_eq!(r.statut, StatusCode::OK);
        assert_eq!(r.corps, serde_json::json!({ "resultat": "different" }));
        assert_eq!(r.robots.as_deref(), Some("noindex, nofollow"));
        assert!(r.cache.as_deref().is_some_and(|c| c.contains("no-store")));
    }
    // Plafond de 10 Mio : au-delà, 413 avant tout calcul.
    let trop = vec![0u8; ent_db::documents_natures::DEPOT_MAX_OCTETS + 2];
    let r = m.appel("POST", "/verifier", &m.hote, trop).await;
    assert_eq!(r.statut, StatusCode::PAYLOAD_TOO_LARGE);
}
