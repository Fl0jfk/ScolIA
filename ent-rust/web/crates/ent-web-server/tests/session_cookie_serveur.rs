//! Le relais de session par cookie `HttpOnly` (A6) **monté dans le serveur de production**
//! (`assembler`) : connexion, relais, déconnexion, et pages Leptos servies par le même routeur.
//! Les scénarios détaillés (CSRF, expiration par la base, élévation, passkeys) sont dans les
//! tests de `ent-web-session`.
//!
//! Base PostgreSQL **locale** de test uniquement ; sans `ENT_DATABASE_URL`, ignoré (« SKIP »).
//! Sans la feature `labo` (L4), ce fichier est vide.
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use reqwest::{header, StatusCode};
use serde_json::{json, Value};
use uuid::Uuid;

const MDP: &str = "DevLocalPass1!";
const COOKIE: &str = "__Host-ent_session";

async fn demarrer() -> Option<(String, String)> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent (sourcer scripts/env.sh)");
        return None;
    };
    let parsed = ent_db::assert_local_postgres_url(&url)
        .unwrap_or_else(|e| panic!("base de test locale uniquement : {e}"));
    let nom_base = parsed.path().trim_start_matches('/').to_ascii_lowercase();
    assert!(
        ["test", "proto", "lab"]
            .iter()
            .any(|m| nom_base.contains(m)),
        "le nom de la base doit contenir test, proto ou lab (refusé : {nom_base})"
    );
    let (state, _) = ent_api::boot_from_env().await.expect("ent-api");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let origine = format!("http://localhost:{}", addr.port());
    let session = ent_web_server::config_session(addr, |n| {
        (n == "ENT_WEB_ORIGINES").then(|| origine.clone())
    })
    .expect("configuration de session");
    let app = ent_web_server::assembler(
        ent_web_server::options_leptos(),
        ent_api::app(state),
        session,
    );
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    Some((format!("http://{addr}"), origine))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_par_cookie_sur_le_serveur_complet() {
    let Some((base, origine)) = demarrer().await else {
        return;
    };
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let etab = Uuid::new_v4();
    let email = format!("serveur-cookie-{}@ent.lab", Uuid::new_v4());
    let r = http
        .post(format!("{base}/auth/register"))
        .json(&json!({ "email": email, "password": MDP, "etablissement_id": etab, "role": "prof" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::CREATED);

    // Les pages Leptos sont servies par le même routeur (CSP à nonce) et ne portent aucun jeton.
    let page = http
        .get(format!("{base}/web/connexion"))
        .send()
        .await
        .unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    assert!(page.headers().contains_key("content-security-policy"));
    assert!(!page.headers().contains_key(header::SET_COOKIE));

    // Connexion par le relais : cookie HttpOnly, aucun jeton dans le corps.
    let r = http
        .post(format!("{base}/web/session/connexion"))
        .header(header::ORIGIN, &origine)
        .header("x-ent-csrf", "1")
        .json(&json!({ "email": email, "password": MDP, "etablissement_id": etab }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let c = r
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with(COOKIE))
        .expect("cookie de session")
        .to_string();
    for attendu in ["HttpOnly", "Secure", "SameSite=Lax", "Path=/"] {
        assert!(c.contains(attendu), "{attendu} : {c}");
    }
    let corps: Value = r.json().await.unwrap();
    assert!(corps.get("token").is_none(), "pas de jeton dans le corps");
    let jeton = c
        .split(';')
        .next()
        .unwrap()
        .split_once('=')
        .unwrap()
        .1
        .to_string();

    // Relais : le cookie suffit sous /web/api ; ailleurs (API mobile) il est ignoré.
    let moi = http
        .get(format!("{base}/web/api/auth/me"))
        .header(header::COOKIE, format!("{COOKIE}={jeton}"))
        .send()
        .await
        .unwrap();
    assert_eq!(moi.status(), StatusCode::OK);
    let moi: Value = moi.json().await.unwrap();
    assert_eq!(moi["portee"], "complete");
    assert_eq!(moi["client"], "web");
    let direct = http
        .get(format!("{base}/auth/me"))
        .header(header::COOKIE, format!("{COOKIE}={jeton}"))
        .send()
        .await
        .unwrap();
    assert_eq!(
        direct.status(),
        StatusCode::UNAUTHORIZED,
        "cookie ignoré hors /web/api"
    );

    // Temps restant (alerte avant expiration) : lu en base.
    let etat: Value = http
        .get(format!("{base}/web/session/etat"))
        .header(header::COOKIE, format!("{COOKIE}={jeton}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(etat["expire_dans"].as_u64().unwrap() > 11 * 3600);
    assert_eq!(etat["inactivite_max"], 7200);

    // Déconnexion : révocation en base + cookie effacé.
    let r = http
        .post(format!("{base}/web/session/deconnexion"))
        .header(header::ORIGIN, &origine)
        .header("x-ent-csrf", "1")
        .header(header::COOKIE, format!("{COOKIE}={jeton}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    assert!(r
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .any(|v| v.to_str().unwrap().contains("Max-Age=0")));
    let apres = http
        .get(format!("{base}/auth/me"))
        .bearer_auth(&jeton)
        .send()
        .await
        .unwrap();
    assert_eq!(apres.status(), StatusCode::UNAUTHORIZED, "révoquée en base");
}
