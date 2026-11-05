//! Socle 1 — A7 : cookie de liaison du SSO au navigateur, bout en bout HTTP
//! (routeur réel, base locale de test). Sans `ENT_DATABASE_URL`, ignorés.

use super::*;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use ent_oidc::testing::{FakeOidcTransport, TestIdp};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

#[test]
fn listes_d_environnements_non_productifs_alignees() {
    assert_eq!(ent_auth::ENVS_NON_PROD, ENVS_NON_PROD);
    assert_eq!(ent_oidc::ssrf::ENVS_NON_PROD, ENVS_NON_PROD);
}

struct Banc {
    app: axum::Router,
    etab: Uuid,
    idp: TestIdp,
    transport: Arc<FakeOidcTransport>,
}

async fn banc() -> Option<Banc> {
    let url = std::env::var("ENT_DATABASE_URL").ok().filter(|u| !u.trim().is_empty())?;
    let mut ent = PgEnt::connect(&url).await.ok()?;
    ent.migrate().await.ok()?;
    let etab = Uuid::new_v4();
    let mut idp = TestIdp::new();
    idp.email = format!("sso-http-{}@idp.test", Uuid::new_v4());
    idp.subject = format!("sub-{}", Uuid::new_v4());
    let transport = Arc::new(FakeOidcTransport::new(idp.clone()));
    ent.set_oidc_transport(transport.clone());
    ent.register_user(&idp.email, "SsoHttp1!SsoHttp", Some("Compte SSO"), etab, "prof")
        .await
        .ok()?;
    ent.upsert_oidc_provider_for_actor(
        etab,
        "direction",
        ent_db::OidcProviderParams {
            slug: "idp-http".into(),
            libelle: "IdP".into(),
            issuer: idp.issuer.clone(),
            client_id: idp.client_id.clone(),
            client_secret: idp.client_secret.clone(),
            scopes: None,
            redirect_uri: None,
        },
    )
    .await
    .ok()?;
    Some(Banc { app: app(AppState::nouveau(ent)), etab, idp, transport })
}

async fn post(
    app: &axum::Router,
    uri: &str,
    cookie: Option<&str>,
    body: Value,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let (st, h) = (res.status(), res.headers().clone());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes).unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into()));
    (st, h, v)
}

fn param(url: &str, k: &str) -> String {
    // state / nonce / code_challenge sont en base64url ou hex : pas d'échappement.
    url.split_once('?')
        .map(|(_, q)| q)
        .unwrap_or("")
        .split('&')
        .find_map(|kv| kv.strip_prefix(&format!("{k}=")))
        .unwrap_or("")
        .to_string()
}

/// Démarre un parcours : renvoie (state, valeur brute du cookie de liaison, en-tête Set-Cookie).
async fn demarrer(b: &Banc) -> (String, String, String) {
    let (st, h, v) = post(
        &b.app,
        "/auth/oidc/idp-http/start",
        None,
        json!({"etablissement_id": b.etab}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let url = v["url"].as_str().unwrap().to_string();
    b.transport.arm_nonce(&param(&url, "nonce"));
    b.transport.arm_code_challenge(&param(&url, "code_challenge"));
    let sc = h.get(header::SET_COOKIE).expect("Set-Cookie au start").to_str().unwrap().to_string();
    let valeur = sc
        .strip_prefix("__Host-ent_oidc=")
        .and_then(|r| r.split(';').next())
        .expect("cookie __Host-ent_oidc")
        .to_string();
    (param(&url, "state"), valeur, sc)
}

#[tokio::test]
async fn sso_start_pose_un_cookie_host_httponly_secure_lax() {
    let Some(b) = banc().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (_state, valeur, sc) = demarrer(&b).await;
    assert!(valeur.len() >= 32, "valeur aléatoire suffisante");
    for attendu in ["__Host-ent_oidc=", "Secure", "HttpOnly", "SameSite=Lax", "Path=/", "Max-Age=600"] {
        assert!(sc.contains(attendu), "{attendu} absent de : {sc}");
    }
    assert!(!sc.contains("Domain="), "__Host- interdit Domain");
    let (_, h, _) = post(&b.app, "/auth/oidc/idp-http/start", None, json!({"etablissement_id": b.etab})).await;
    assert_eq!(h.get(header::CACHE_CONTROL).unwrap(), "no-store");
}

#[tokio::test]
async fn sso_complete_exige_le_cookie_du_navigateur_initial() {
    let Some(b) = banc().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    // Sans cookie : refus 401 (login CSRF / rejeu depuis un autre navigateur).
    let (state, _valeur, _sc) = demarrer(&b).await;
    let (st, _, _) = post(&b.app, "/auth/oidc/complete", None, json!({"code": "c", "state": state})).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // Mauvais cookie, ou cookie portant un autre nom : refus.
    let (state, valeur, _) = demarrer(&b).await;
    let (st, _, _) = post(
        &b.app, "/auth/oidc/complete", Some("__Host-ent_oidc=autre"), json!({"code": "c", "state": state}),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (state, valeur2, _) = demarrer(&b).await;
    let (st, _, _) = post(
        &b.app, "/auth/oidc/complete", Some(&format!("ent_oidc={valeur2}")), json!({"code": "c", "state": state}),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "le préfixe __Host- fait partie du nom");
    let _ = valeur;

    // Bon cookie (parmi d'autres cookies) : succès, cookie effacé, jamais de secret dans la réponse.
    let (state, valeur, _) = demarrer(&b).await;
    let cookie = format!("a=b; __Host-ent_oidc={valeur}; c=d");
    let (st, h, corps) = post(&b.app, "/auth/oidc/complete", Some(&cookie), json!({"code": "c", "state": state})).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let sc = h.get(header::SET_COOKIE).expect("effacement").to_str().unwrap();
    assert!(sc.contains("__Host-ent_oidc=;") && sc.contains("Max-Age=0"), "{sc}");
    assert!(!corps.to_string().contains(&b.idp.client_secret));

    // Rejeu avec le bon cookie : le state est consommé.
    let (st, _, _) = post(&b.app, "/auth/oidc/complete", Some(&cookie), json!({"code": "c", "state": state})).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sso_email_non_verifie_donne_401_http() {
    let Some(b) = banc().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    b.transport.set_email_verified(None);
    let (state, valeur, _) = demarrer(&b).await;
    let cookie = format!("__Host-ent_oidc={valeur}");
    let (st, _, corps) = post(&b.app, "/auth/oidc/complete", Some(&cookie), json!({"code": "c", "state": state})).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "{corps}");
}
