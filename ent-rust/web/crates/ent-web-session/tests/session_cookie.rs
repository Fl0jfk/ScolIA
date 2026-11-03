//! Tests du **prototype isolé** de session par cookie (D-016), contre le vrai `ent-api`
//! (inchangé) et une base PostgreSQL **locale** de labo. Comptes fictifs créés à la volée.
//!
//!   . ent-rust/web/scripts/env.sh && bash ent-rust/web/scripts/pg.sh start
//!   cd ent-rust/web && cargo test -p ent-web-session -- --test-threads=1
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).

use std::net::SocketAddr;
use std::time::Duration;

use reqwest::{header, Client, StatusCode};
use serde_json::{json, Value};
use uuid::Uuid;
use webauthn_authenticator_rs::prelude::Url;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_authenticator_rs::WebauthnAuthenticator;

const MDP: &str = "DevLocalPass1!";
/// Origine publique simulée (celle du navigateur). C'est aussi l'origine WebAuthn par défaut
/// d'ent-auth (`ENT_RP_ORIGIN` absent).
const ORIGINE: &str = "http://localhost:3000";
const COOKIE: &str = "__Host-ent_session";

struct Banc {
    base: String,
    http: Client,
}

async fn demarrer(duree_max: Duration, inactivite: Duration) -> Option<Banc> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent (sourcer scripts/env.sh)");
        return None;
    };
    assert!(
        url.contains("@127.0.0.1:"),
        "base de test locale uniquement"
    );
    let (state, _) = ent_api::boot_from_env().await.expect("ent-api");
    let api = ent_api::app(state);
    let config = ent_web_session::Config {
        origines: vec![ORIGINE.into()],
        secure: true,
        duree_max,
        inactivite,
    };
    let app = ent_web_session::passerelle(api.clone(), config)
        .merge(ent_web_session::page_demo())
        .merge(api);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    Some(Banc {
        base: format!("http://{addr}"),
        http: Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap(),
    })
}

async fn banc() -> Option<Banc> {
    demarrer(
        Duration::from_secs(12 * 3600),
        Duration::from_secs(2 * 3600),
    )
    .await
}

/// Compte fictif (inscription ouverte en environnement de labo uniquement).
async fn compte(b: &Banc, role: &str) -> (Uuid, String) {
    let etab = Uuid::new_v4();
    let email = format!("proto-cookie-{}@ent.lab", Uuid::new_v4());
    let r = b
        .http
        .post(format!("{}/auth/register", b.base))
        .json(&json!({ "email": email, "password": MDP, "etablissement_id": etab, "role": role }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::CREATED, "inscription de labo");
    (etab, email)
}

/// Requête « navigateur » : origine publique + en-tête anti-CSRF.
fn nav(b: &Banc, methode: reqwest::Method, chemin: &str) -> reqwest::RequestBuilder {
    b.http
        .request(methode, format!("{}{chemin}", b.base))
        .header(header::ORIGIN, ORIGINE)
        .header("x-ent-csrf", "1")
}

fn set_cookie(r: &reqwest::Response) -> Option<String> {
    r.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|v| v.starts_with(COOKIE))
        .map(str::to_string)
}

fn valeur(set_cookie: &str) -> String {
    set_cookie
        .split(';')
        .next()
        .unwrap()
        .split_once('=')
        .unwrap()
        .1
        .to_string()
}

async fn connexion_cookie(b: &Banc, etab: Uuid, email: &str) -> String {
    let r = nav(b, reqwest::Method::POST, "/web/session/connexion")
        .json(&json!({ "email": email, "password": MDP, "etablissement_id": etab }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    valeur(&set_cookie(&r).expect("cookie posé"))
}

async fn moi_cookie(b: &Banc, jeton: &str) -> StatusCode {
    b.http
        .get(format!("{}/web/api/auth/me", b.base))
        .header(header::COOKIE, format!("{COOKIE}={jeton}"))
        .send()
        .await
        .unwrap()
        .status()
}

async fn moi_bearer(b: &Banc, jeton: &str) -> StatusCode {
    b.http
        .get(format!("{}/auth/me", b.base))
        .bearer_auth(jeton)
        .send()
        .await
        .unwrap()
        .status()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn connexion_mot_de_passe_pose_un_cookie_httponly_sans_jeton_dans_le_corps() {
    let Some(b) = banc().await else { return };
    let (etab, email) = compte(&b, "prof").await;
    let r = nav(&b, reqwest::Method::POST, "/web/session/connexion")
        .json(&json!({ "email": email, "password": MDP, "etablissement_id": etab }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let c = set_cookie(&r).expect("Set-Cookie");
    for attendu in ["HttpOnly", "Secure", "SameSite=Lax", "Path=/"] {
        assert!(c.contains(attendu), "attribut {attendu} absent : {c}");
    }
    assert!(
        !c.to_lowercase().contains("domain="),
        "__Host- : pas de Domain"
    );
    let max_age: u64 = c
        .split("Max-Age=")
        .nth(1)
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        max_age > 0 && max_age <= 12 * 3600,
        "Max-Age ≤ 12 h : {max_age}"
    );
    assert_eq!(r.headers()[header::CACHE_CONTROL], "no-store");
    let corps: Value = r.json().await.unwrap();
    assert!(
        corps.get("token").is_none(),
        "jeton absent du corps : {corps}"
    );
    assert_eq!(corps["email"], email.as_str());

    let jeton = valeur(&c);
    let r = b
        .http
        .get(format!("{}/web/api/auth/me", b.base))
        .header(header::COOKIE, format!("autre=1; {COOKIE}={jeton}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(r.headers()[header::VARY], "Cookie");
    assert_eq!(r.json::<Value>().await.unwrap()["email"], email.as_str());

    // Mauvais mot de passe : refus transmis, aucun cookie.
    let r = nav(&b, reqwest::Method::POST, "/web/session/connexion")
        .json(&json!({ "email": email, "password": "mauvais", "etablissement_id": etab }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert!(set_cookie(&r).is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn csrf_origine_et_en_tete_exiges_y_compris_a_la_connexion() {
    let Some(b) = banc().await else { return };
    let (etab, email) = compte(&b, "prof").await;
    let corps = json!({ "email": email, "password": MDP, "etablissement_id": etab });
    let url = format!("{}/web/session/connexion", b.base);

    // Connexion (« login CSRF ») : sans origine, origine étrangère, sans en-tête → 403.
    let sans_origine = b.http.post(&url).header("x-ent-csrf", "1").json(&corps);
    let etrangere = b
        .http
        .post(&url)
        .header(header::ORIGIN, "https://pirate.example")
        .header("x-ent-csrf", "1")
        .json(&corps);
    let sans_en_tete = b
        .http
        .post(&url)
        .header(header::ORIGIN, ORIGINE)
        .json(&corps);
    for (nom, req) in [
        ("sans origine", sans_origine),
        ("origine étrangère", etrangere),
        ("sans X-ENT-CSRF", sans_en_tete),
    ] {
        let r = req.send().await.unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN, "{nom}");
        assert!(set_cookie(&r).is_none(), "{nom} : aucun cookie");
    }
    // Sans `Origin` mais `Sec-Fetch-Site: same-origin` (navigateur, même site) → accepté.
    let r = b
        .http
        .post(&url)
        .header("sec-fetch-site", "same-origin")
        .header("x-ent-csrf", "1")
        .json(&corps)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);

    // Requête qui modifie via le relais : cookie présent mais origine étrangère → 403.
    let jeton = connexion_cookie(&b, etab, &email).await;
    let modifier = |origine: &str, csrf: bool| {
        let mut r = b
            .http
            .post(format!("{}/web/api/auth/passkey/register/start", b.base))
            .header(header::COOKIE, format!("{COOKIE}={jeton}"))
            .header(header::ORIGIN, origine)
            .json(&json!({}));
        if csrf {
            r = r.header("x-ent-csrf", "1");
        }
        r
    };
    let r = modifier("https://pirate.example", true)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    let r = modifier(ORIGINE, false).send().await.unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    let r = modifier(ORIGINE, true).send().await.unwrap();
    assert_eq!(
        r.status(),
        StatusCode::OK,
        "même origine + en-tête : accepté"
    );
    // Lecture (GET) : pas de contrôle CSRF (aucune donnée modifiée).
    assert_eq!(moi_cookie(&b, &jeton).await, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn le_relais_refuse_les_routes_qui_emettent_un_jeton() {
    let Some(b) = banc().await else { return };
    let (etab, email) = compte(&b, "prof").await;
    let jeton = connexion_cookie(&b, etab, &email).await;
    for chemin in [
        "/web/api/auth/login",
        "/web/api/auth/register",
        "/web/api/auth/passkey/login/start",
        "/web/api/auth/passkey/login/finish",
        "/web/api/auth/oidc/complete",
    ] {
        let r = nav(&b, reqwest::Method::POST, chemin)
            .header(header::COOKIE, format!("{COOKIE}={jeton}"))
            .json(&json!({ "email": email, "password": MDP, "etablissement_id": etab }))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN, "{chemin}");
    }
    // Sans cookie : 401, quelle que soit la route.
    let r = b
        .http
        .get(format!("{}/web/api/auth/me", b.base))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deconnexion_revoque_en_base_et_efface_le_cookie() {
    let Some(b) = banc().await else { return };
    let (etab, email) = compte(&b, "prof").await;
    let jeton = connexion_cookie(&b, etab, &email).await;
    assert_eq!(moi_cookie(&b, &jeton).await, StatusCode::OK);
    assert_eq!(moi_bearer(&b, &jeton).await, StatusCode::OK);

    // Déconnexion sans en-tête CSRF : refusée (pas de déconnexion forcée par un autre site).
    let r = b
        .http
        .post(format!("{}/web/session/deconnexion", b.base))
        .header(header::ORIGIN, "https://pirate.example")
        .header(header::COOKIE, format!("{COOKIE}={jeton}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert_eq!(moi_cookie(&b, &jeton).await, StatusCode::OK);

    let r = nav(&b, reqwest::Method::POST, "/web/session/deconnexion")
        .header(header::COOKIE, format!("{COOKIE}={jeton}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    let c = set_cookie(&r).expect("effacement");
    assert!(c.contains("Max-Age=0"), "{c}");
    // L'ancien cookie (rejoué, par exemple depuis une copie) est refusé partout.
    assert_eq!(moi_cookie(&b, &jeton).await, StatusCode::UNAUTHORIZED);
    assert_eq!(
        moi_bearer(&b, &jeton).await,
        StatusCode::UNAUTHORIZED,
        "session révoquée en base, pas seulement côté navigateur"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn changement_de_mot_de_passe_revoque_les_autres_sessions() {
    let Some(b) = banc().await else { return };
    let (etab, email) = compte(&b, "prof").await;
    let onglet_a = connexion_cookie(&b, etab, &email).await;
    let poste_b = connexion_cookie(&b, etab, &email).await;
    let r = nav(&b, reqwest::Method::POST, "/web/api/auth/password")
        .header(header::COOKIE, format!("{COOKIE}={onglet_a}"))
        .json(&json!({ "current_password": MDP, "new_password": "NouveauLabo2!x" }))
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success(), "changement : {}", r.status());
    assert_eq!(
        moi_cookie(&b, &onglet_a).await,
        StatusCode::OK,
        "session courante gardée"
    );
    assert_eq!(
        moi_cookie(&b, &poste_b).await,
        StatusCode::UNAUTHORIZED,
        "autre session révoquée"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expiration_par_inactivite_puis_par_duree_maximale() {
    // Durées raccourcies pour le test : inactivité 1 s, maximum 3 s.
    let Some(b) = demarrer(Duration::from_secs(3), Duration::from_secs(1)).await else {
        return;
    };
    let (etab, email) = compte(&b, "prof").await;

    let jeton = connexion_cookie(&b, etab, &email).await;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let r = b
        .http
        .get(format!("{}/web/api/auth/me", b.base))
        .header(header::COOKIE, format!("{COOKIE}={jeton}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED, "inactivité dépassée");
    assert!(set_cookie(&r).unwrap().contains("Max-Age=0"));
    assert_eq!(
        moi_bearer(&b, &jeton).await,
        StatusCode::UNAUTHORIZED,
        "session expirée aussi révoquée en base"
    );

    // Activité régulière (toutes les 0,6 s) : valide jusqu'à la durée maximale (3 s).
    let jeton = connexion_cookie(&b, etab, &email).await;
    let debut = std::time::Instant::now();
    let mut refus_apres = None;
    while debut.elapsed() < Duration::from_secs(5) {
        tokio::time::sleep(Duration::from_millis(600)).await;
        if moi_cookie(&b, &jeton).await == StatusCode::UNAUTHORIZED {
            refus_apres = Some(debut.elapsed());
            break;
        }
    }
    let t = refus_apres.expect("refus après la durée maximale");
    assert!(
        t >= Duration::from_secs(3),
        "pas de refus avant 3 s : {t:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bearer_mobile_et_ancienne_interface_inchanges() {
    let Some(b) = banc().await else { return };
    let (etab, email) = compte(&b, "prof").await;

    // Appli mobile / ancienne interface : jeton dans le corps, Bearer, sans contrôle CSRF.
    let r = b
        .http
        .post(format!("{}/auth/login", b.base))
        .header(header::ORIGIN, "https://autre.example")
        .json(&json!({ "email": email, "password": MDP, "etablissement_id": etab }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert!(
        set_cookie(&r).is_none(),
        "aucun cookie pour les clients Bearer"
    );
    let bearer = r.json::<Value>().await.unwrap()["token"]
        .as_str()
        .expect("jeton dans le corps (inchangé)")
        .to_string();
    assert_eq!(moi_bearer(&b, &bearer).await, StatusCode::OK);

    // Le cookie n'est PAS accepté par les routes existantes (seulement sous /web/api).
    let web = connexion_cookie(&b, etab, &email).await;
    let r = b
        .http
        .get(format!("{}/auth/me", b.base))
        .header(header::COOKIE, format!("{COOKIE}={web}"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);

    // Ancienne interface servie comme avant.
    let r = b.http.get(format!("{}/app", b.base)).send().await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert!(
        r.text().await.unwrap().contains("ent_token"),
        "UI /app inchangée"
    );

    // Sessions indépendantes : fermer la session web ne ferme pas celle de /app (et inversement).
    nav(&b, reqwest::Method::POST, "/web/session/deconnexion")
        .header(header::COOKIE, format!("{COOKIE}={web}"))
        .send()
        .await
        .unwrap();
    assert_eq!(moi_bearer(&b, &bearer).await, StatusCode::OK);
    let web2 = connexion_cookie(&b, etab, &email).await;
    let r = b
        .http
        .post(format!("{}/auth/logout", b.base))
        .bearer_auth(&bearer)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    assert_eq!(moi_cookie(&b, &web2).await, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn passkey_second_facteur_et_sans_mot_de_passe_posent_le_cookie() {
    let Some(b) = banc().await else { return };
    let (etab, email) = compte(&b, "parent").await;
    let origine = Url::parse(ORIGINE).unwrap();
    let mut appareil = WebauthnAuthenticator::new(SoftPasskey::new(true));

    // Enrôlement par le relais (cookie → Bearer), avec contrôle CSRF.
    let jeton = connexion_cookie(&b, etab, &email).await;
    let debut: Value = nav(
        &b,
        reqwest::Method::POST,
        "/web/api/auth/passkey/register/start",
    )
    .header(header::COOKIE, format!("{COOKIE}={jeton}"))
    .json(&json!({}))
    .send()
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
    let ccr: webauthn_rs_proto::CreationChallengeResponse =
        serde_json::from_value(debut["options"].clone()).unwrap();
    let enr = appareil
        .do_registration(origine.clone(), ccr)
        .expect("enrôlement logiciel");
    let r = nav(
        &b,
        reqwest::Method::POST,
        "/web/api/auth/passkey/register/finish",
    )
    .header(header::COOKIE, format!("{COOKIE}={jeton}"))
    .json(&json!({ "token": debut["token"], "credential": enr, "name": "Test" }))
    .send()
    .await
    .unwrap();
    assert_eq!(r.status(), StatusCode::CREATED);

    // 1) Mot de passe seul : second facteur demandé, AUCUN cookie.
    let r = nav(&b, reqwest::Method::POST, "/web/session/connexion")
        .json(&json!({ "email": email, "password": MDP, "etablissement_id": etab }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert!(set_cookie(&r).is_none(), "pas de session avant la passkey");
    let defi: Value = r.json().await.unwrap();
    assert_eq!(defi["mfa_required"], true);
    let rcr: webauthn_rs_proto::RequestChallengeResponse =
        serde_json::from_value(defi["passkey"]["options"].clone()).unwrap();
    let assertion = appareil.do_authentication(origine.clone(), rcr).unwrap();
    let r = nav(&b, reqwest::Method::POST, "/web/session/passkey/fin")
        .json(&json!({ "token": defi["passkey"]["token"], "credential": assertion }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let c = set_cookie(&r).expect("cookie après passkey");
    assert!(c.contains("HttpOnly"));
    assert!(r.json::<Value>().await.unwrap().get("token").is_none());
    assert_eq!(moi_cookie(&b, &valeur(&c)).await, StatusCode::OK);

    // 2) Sans mot de passe : début → assertion → cookie.
    let r = nav(&b, reqwest::Method::POST, "/web/session/passkey/debut")
        .json(&json!({ "email": email, "etablissement_id": etab }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let defi: Value = r.json().await.unwrap();
    let rcr: webauthn_rs_proto::RequestChallengeResponse =
        serde_json::from_value(defi["options"].clone()).unwrap();
    let assertion = appareil.do_authentication(origine.clone(), rcr).unwrap();
    // Le défi est à usage unique : sans en-tête CSRF, refus avant tout traitement.
    let r = b
        .http
        .post(format!("{}/web/session/passkey/fin", b.base))
        .header(header::ORIGIN, ORIGINE)
        .json(&json!({ "token": defi["token"], "credential": assertion }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    let r = nav(&b, reqwest::Method::POST, "/web/session/passkey/fin")
        .json(&json!({ "token": defi["token"], "credential": assertion }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let c = set_cookie(&r).expect("cookie sans mot de passe");
    assert_eq!(moi_cookie(&b, &valeur(&c)).await, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn page_de_demonstration_avec_csp_stricte() {
    let Some(b) = banc().await else { return };
    let r = b
        .http
        .get(format!("{}/web/_session", b.base))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let csp = r.headers()[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .unwrap();
    assert!(csp.contains("script-src 'self'") && csp.contains("frame-ancestors 'self'"));
    let html = r.text().await.unwrap();
    assert!(!html.contains("<script>"), "aucun script en ligne");
    let js = include_str!("../static/session.js");
    for acces in [
        "localStorage.",
        "localStorage[",
        "sessionStorage.",
        "document.cookie",
    ] {
        assert!(
            !js.contains(acces),
            "le script ne doit pas utiliser {acces}"
        );
    }
}
