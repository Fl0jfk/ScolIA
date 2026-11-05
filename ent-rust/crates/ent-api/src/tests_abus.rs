//! Anti-abus de l'authentification (limitation de débit, IP fiable, verrou
//! (compte, IP), indiscernabilité des réponses, bornes de taille).
//!
//! Chaque test utilise des IP et des e-mails **uniques** : les seaux et
//! compteurs vivent en base et persistent d'une exécution à l'autre. Les
//! requêtes portent une `ConnectInfo` (adresse de socket) comme le fait le vrai
//! serveur (`into_make_service_with_connect_info`).

use super::*;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower::ServiceExt;
use uuid::Uuid;

const MDP: &str = "DevLocalPass1!";

async fn etat() -> Option<AppState> {
    let url = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())?;
    let mut ent = PgEnt::connect(&url).await.ok()?;
    ent.migrate().await.ok()?;
    ent.install_brain_p0();
    Some(AppState::nouveau(ent))
}

/// Restaure une variable d'environnement à la sortie du test (même en panique).
struct VarEnv(&'static str);
impl VarEnv {
    fn pose(nom: &'static str, valeur: &str) -> Self {
        std::env::set_var(nom, valeur);
        VarEnv(nom)
    }
}
impl Drop for VarEnv {
    fn drop(&mut self) {
        std::env::remove_var(self.0);
    }
}

/// IP v4 unique par appel (octets d'un UUID aléatoire).
fn ip_unique() -> String {
    let b = Uuid::new_v4().into_bytes();
    format!("{}.{}.{}.{}", 11 + b[0] % 200, b[1], b[2], b[3])
}

struct Rep {
    statut: StatusCode,
    en_tetes: HeaderMap,
    texte: String,
}

impl Rep {
    fn json(&self) -> Value {
        serde_json::from_str(&self.texte).unwrap_or(Value::Null)
    }
}

async fn post(
    app: &axum::Router,
    uri: &str,
    corps: Value,
    socket: Option<&str>,
    xff: Option<&str>,
    bearer: Option<&str>,
) -> Rep {
    let mut b = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(x) = xff {
        b = b.header("x-forwarded-for", x);
    }
    if let Some(t) = bearer {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    if let Some(s) = socket {
        let ip: std::net::IpAddr = s.parse().unwrap();
        b = b.extension(ConnectInfo(SocketAddr::new(ip, 40000)));
    }
    let res = app
        .clone()
        .oneshot(
            b.body(Body::from(serde_json::to_vec(&corps).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let statut = res.status();
    let en_tetes = res.headers().clone();
    let octets = res.into_body().collect().await.unwrap().to_bytes();
    Rep {
        statut,
        en_tetes,
        texte: String::from_utf8_lossy(&octets).into_owned(),
    }
}

async fn inscrire(app: &axum::Router, etab: Uuid, email: &str, role: &str) {
    let r = post(
        app,
        "/auth/register",
        json!({"email": email, "password": MDP, "etablissement_id": etab, "role": role}),
        None,
        None,
        None,
    )
    .await;
    assert!(
        r.statut == StatusCode::CREATED || r.statut == StatusCode::CONFLICT,
        "register {}",
        r.statut
    );
}

fn corps_login(email: &str, mdp: &str, etab: Uuid) -> Value {
    json!({"email": email, "password": mdp, "etablissement_id": etab})
}

/// « Forme » d'un JSON : clés et types, longueurs de chaînes arrondies — pour
/// comparer une réponse réelle et une réponse factice sans leurs valeurs.
fn forme(v: &Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut o = serde_json::Map::new();
            for (k, x) in m {
                o.insert(k.clone(), forme(x));
            }
            Value::Object(o)
        }
        Value::Array(a) => Value::Array(a.iter().take(1).map(forme).collect()),
        Value::String(s) => json!(format!("string:{}", s.len() / 8)),
        Value::Number(_) => json!("number"),
        Value::Bool(_) => json!("bool"),
        Value::Null => json!("null"),
    }
}

// --- Limitation de débit ------------------------------------------------------

#[tokio::test]
async fn abus_429_retry_after_au_dela_du_quota_par_identifiant() {
    let Some(state) = etat().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let ip = ip_unique();
    let email = format!("Spam-{}@ent.test", Uuid::new_v4().simple());

    // 3 demandes par heure et par e-mail (défaut) : la 4ᵉ est refusée, quelle que
    // soit la casse de l'adresse (identifiant normalisé), et même sans compte.
    for i in 0..3 {
        let e = if i % 2 == 0 {
            email.clone()
        } else {
            email.to_uppercase()
        };
        let r = post(
            &app,
            "/auth/forgot-password",
            json!({"email": e}),
            Some(&ip),
            None,
            None,
        )
        .await;
        assert_eq!(r.statut, StatusCode::ACCEPTED, "demande {i}");
    }
    let r = post(
        &app,
        "/auth/forgot-password",
        json!({"email": email}),
        Some(&ip),
        None,
        None,
    )
    .await;
    assert_eq!(r.statut, StatusCode::TOO_MANY_REQUESTS);
    let attente: u64 = r.en_tetes["retry-after"].to_str().unwrap().parse().unwrap();
    assert!((1..=3600).contains(&attente), "Retry-After = {attente}");
    assert_eq!(r.texte, anti_abus::MESSAGE_LIMITE, "message neutre");

    // Un autre e-mail n'est pas pénalisé.
    let autre = format!("autre-{}@ent.test", Uuid::new_v4().simple());
    let r = post(
        &app,
        "/auth/forgot-password",
        json!({"email": autre}),
        Some(&ip),
        None,
        None,
    )
    .await;
    assert_eq!(r.statut, StatusCode::ACCEPTED);
}

#[tokio::test]
async fn abus_429_par_ip_sur_forgot_et_reset() {
    let Some(state) = etat().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let _q1 = VarEnv::pose("ENT_RATE_FORGOT_PASSWORD_IP", "2/3600");
    let _q2 = VarEnv::pose("ENT_RATE_RESET_PASSWORD_IP", "2/3600");
    let ip = ip_unique();

    for i in 0..2 {
        let e = format!("ip-{i}-{}@ent.test", Uuid::new_v4().simple());
        let r = post(
            &app,
            "/auth/forgot-password",
            json!({"email": e}),
            Some(&ip),
            None,
            None,
        )
        .await;
        assert_eq!(r.statut, StatusCode::ACCEPTED);
    }
    let e = format!("ip-x-{}@ent.test", Uuid::new_v4().simple());
    let r = post(
        &app,
        "/auth/forgot-password",
        json!({"email": e}),
        Some(&ip),
        None,
        None,
    )
    .await;
    assert_eq!(r.statut, StatusCode::TOO_MANY_REQUESTS);
    assert!(r.en_tetes.contains_key("retry-after"));

    for i in 0..2 {
        let r = post(
            &app,
            "/auth/reset-password",
            json!({"token": format!("bidon-{i}-{}", Uuid::new_v4().simple()), "new_password": "NouveauPass1!2026"}),
            Some(&ip),
            None,
            None,
        )
        .await;
        assert_eq!(r.statut, StatusCode::BAD_REQUEST, "jeton bidon {i}");
    }
    let r = post(
        &app,
        "/auth/reset-password",
        json!({"token": format!("bidon-z-{}", Uuid::new_v4().simple()), "new_password": "NouveauPass1!2026"}),
        Some(&ip),
        None,
        None,
    )
    .await;
    assert_eq!(r.statut, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn abus_xff_usurpe_n_elude_pas_la_limite() {
    let Some(state) = etat().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let _q = VarEnv::pose("ENT_RATE_PASSKEY_LOGIN_START_IP", "3/3600");
    let corps = || json!({"email": "x@ent.test", "etablissement_id": Uuid::new_v4()});

    // --- Sans proxy de confiance : X-Forwarded-For est ignoré, seule la socket compte.
    let socket = ip_unique();
    for i in 0..3 {
        let forge = format!("9.9.9.{i}");
        let r = post(
            &app,
            "/auth/passkey/login/start",
            corps(),
            Some(&socket),
            Some(&forge),
            None,
        )
        .await;
        assert_eq!(r.statut, StatusCode::OK, "appel {i}");
    }
    let r = post(
        &app,
        "/auth/passkey/login/start",
        corps(),
        Some(&socket),
        Some("9.9.9.99"),
        None,
    )
    .await;
    assert_eq!(
        r.statut,
        StatusCode::TOO_MANY_REQUESTS,
        "XFF forgé ignoré sans proxy de confiance"
    );

    // --- Derrière un proxy de confiance : l'entrée de DROITE, pas la première.
    let _p = VarEnv::pose("ENT_TRUST_PROXY", "1");
    let proxy = "10.0.0.1";
    let reel = ip_unique();
    for i in 0..3 {
        let chaine = format!("66.66.66.{i}, {reel}");
        let r = post(
            &app,
            "/auth/passkey/login/start",
            corps(),
            Some(proxy),
            Some(&chaine),
            None,
        )
        .await;
        assert_eq!(r.statut, StatusCode::OK, "appel {i}");
    }
    let r = post(
        &app,
        "/auth/passkey/login/start",
        corps(),
        Some(proxy),
        Some(&format!("77.77.77.77, {reel}")),
        None,
    )
    .await;
    assert_eq!(
        r.statut,
        StatusCode::TOO_MANY_REQUESTS,
        "tête de chaîne forgée sans effet"
    );
    assert!(r.en_tetes.contains_key("retry-after"));
    // Un autre client réel (même proxy) n'est pas pénalisé.
    let autre = ip_unique();
    let r = post(
        &app,
        "/auth/passkey/login/start",
        corps(),
        Some(proxy),
        Some(&format!("66.66.66.0, {autre}")),
        None,
    )
    .await;
    assert_eq!(r.statut, StatusCode::OK);
}

#[tokio::test]
async fn abus_limiteur_atomique_sous_concurrence() {
    let Some(state) = etat().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let limiteur = state.ent.limiteur_debit();
    let cle = Uuid::new_v4().to_string();
    let quota = ent_db::Quota::new(5, 3600);
    let mut taches = Vec::new();
    for _ in 0..30 {
        let l = limiteur.clone();
        let c = cle.clone();
        taches.push(tokio::spawn(async move {
            l.verifier("test_concurrence", "id", &c, quota).await
        }));
    }
    let mut autorises = 0;
    for t in taches {
        if !t.await.unwrap().est_limite() {
            autorises += 1;
        }
    }
    assert_eq!(
        autorises, 5,
        "exactement le quota, jamais plus (requête atomique)"
    );
}

// --- Verrou de connexion (compte, IP) -----------------------------------------

#[tokio::test]
async fn abus_verrou_atomique_sous_concurrence() {
    let Some(state) = etat().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let email = format!("conc-{}@ent.test", Uuid::new_v4().simple());
    inscrire(&app, etab, &email, "parent").await;
    let ip = ip_unique();

    let mut taches = Vec::new();
    for _ in 0..20 {
        let app = app.clone();
        let (e, ip) = (email.clone(), ip.clone());
        taches.push(tokio::spawn(async move {
            post(
                &app,
                "/auth/login",
                corps_login(&e, "MauvaisPass1!", etab),
                Some(&ip),
                None,
                None,
            )
            .await
            .statut
        }));
    }
    let (mut refus_mdp, mut verrouilles) = (0, 0);
    for t in taches {
        match t.await.unwrap() {
            StatusCode::UNAUTHORIZED => refus_mdp += 1,
            StatusCode::TOO_MANY_REQUESTS => verrouilles += 1,
            autre => panic!("statut inattendu {autre}"),
        }
    }
    assert_eq!(
        refus_mdp, 5,
        "au plus 5 mots de passe vérifiés (incrément atomique avant vérification)"
    );
    assert_eq!(verrouilles, 15);
}

#[tokio::test]
async fn abus_attaquant_ne_verrouille_pas_la_victime_et_passkey_non_bloquee() {
    use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator};
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;

    let Some(state) = etat().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let email = format!("victime-{}@ent.test", Uuid::new_v4().simple());
    inscrire(&app, etab, &email, "parent").await;
    let (ip_victime, ip_attaquant) = (ip_unique(), ip_unique());
    let origine = Url::parse("http://localhost:3000").unwrap();

    // La victime se connecte, enrôle une passkey (2ᵉ facteur).
    let r = post(
        &app,
        "/auth/login",
        corps_login(&email, MDP, etab),
        Some(&ip_victime),
        None,
        None,
    )
    .await;
    assert_eq!(r.statut, StatusCode::OK);
    let jeton = r.json()["token"].as_str().unwrap().to_string();
    let debut = post(
        &app,
        "/auth/passkey/register/start",
        json!({}),
        Some(&ip_victime),
        None,
        Some(&jeton),
    )
    .await;
    assert_eq!(debut.statut, StatusCode::OK);
    let ccr: ent_auth::CreationChallengeResponse =
        serde_json::from_value(debut.json()["options"].clone()).unwrap();
    let mut appareil = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let enrolement = appareil
        .do_registration(origine.clone(), ccr)
        .expect("enrôlement");
    let fin = post(
        &app,
        "/auth/passkey/register/finish",
        json!({"token": debut.json()["token"], "credential": enrolement, "name": "Tel", "current_password": MDP}),
        Some(&ip_victime),
        None,
        Some(&jeton),
    )
    .await;
    assert_eq!(fin.statut, StatusCode::CREATED);

    // L'attaquant martèle le compte depuis SON IP : verrou (compte, IP) atteint.
    for i in 0..5 {
        let r = post(
            &app,
            "/auth/login",
            corps_login(&email, "MauvaisPass1!", etab),
            Some(&ip_attaquant),
            None,
            None,
        )
        .await;
        assert_eq!(r.statut, StatusCode::UNAUTHORIZED, "essai {i}");
    }
    let r = post(
        &app,
        "/auth/login",
        corps_login(&email, "MauvaisPass1!", etab),
        Some(&ip_attaquant),
        None,
        None,
    )
    .await;
    assert_eq!(
        r.statut,
        StatusCode::TOO_MANY_REQUESTS,
        "l'attaquant est verrouillé"
    );
    let r = post(
        &app,
        "/auth/login",
        corps_login(&email, MDP, etab),
        Some(&ip_attaquant),
        None,
        None,
    )
    .await;
    assert_eq!(
        r.statut,
        StatusCode::TOO_MANY_REQUESTS,
        "… même avec le bon mot de passe (depuis son IP)"
    );

    // La victime, depuis une autre IP, n'est PAS bloquée (au pire ralentie).
    let r = post(
        &app,
        "/auth/login",
        corps_login(&email, MDP, etab),
        Some(&ip_victime),
        None,
        None,
    )
    .await;
    assert_eq!(r.statut, StatusCode::OK, "la victime peut se connecter");
    assert_eq!(r.json()["mfa_required"], true);

    // La passkey n'est jamais soumise aux compteurs du mot de passe : même depuis
    // l'IP verrouillée, la connexion par passkey aboutit.
    let start = post(
        &app,
        "/auth/passkey/login/start",
        json!({"email": email, "etablissement_id": etab}),
        Some(&ip_attaquant),
        None,
        None,
    )
    .await;
    assert_eq!(start.statut, StatusCode::OK);
    let rcr: ent_auth::RequestChallengeResponse =
        serde_json::from_value(start.json()["options"].clone()).unwrap();
    let assertion = appareil.do_authentication(origine, rcr).expect("assertion");
    let session = post(
        &app,
        "/auth/passkey/login/finish",
        json!({"token": start.json()["token"], "credential": assertion}),
        Some(&ip_attaquant),
        None,
        None,
    )
    .await;
    assert_eq!(
        session.statut,
        StatusCode::OK,
        "passkey non bloquée par le verrou du mot de passe"
    );

    // Indiscernabilité : challenge réel (compte avec passkey) vs factice (inconnu).
    let faux = post(
        &app,
        "/auth/passkey/login/start",
        json!({"email": format!("inconnu-{}@ent.test", Uuid::new_v4().simple()), "etablissement_id": etab}),
        Some(&ip_victime),
        None,
        None,
    )
    .await;
    assert_eq!(faux.statut, start.statut);
    assert_eq!(
        forme(&faux.json()),
        forme(&start.json()),
        "même forme réel / factice"
    );
}

// --- Indiscernabilité ---------------------------------------------------------

#[tokio::test]
async fn abus_reponses_indiscernables_passkey_start_forgot_et_notmember() {
    let Some(state) = etat().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let autre_etab = Uuid::new_v4();
    let email = format!("exist-{}@ent.test", Uuid::new_v4().simple());
    inscrire(&app, etab, &email, "parent").await;
    let ip = ip_unique();

    // passkey/login/start : inconnu, non membre, membre sans passkey → même réponse.
    let inconnu = format!("zz-{}@ent.test", Uuid::new_v4().simple());
    let cas = [
        json!({"email": inconnu, "etablissement_id": etab}),
        json!({"email": email, "etablissement_id": autre_etab}),
        json!({"email": email, "etablissement_id": etab}),
    ];
    let mut reps = Vec::new();
    for c in cas {
        reps.push(post(&app, "/auth/passkey/login/start", c, Some(&ip), None, None).await);
    }
    for r in &reps {
        assert_eq!(r.statut, StatusCode::OK);
        assert!(r.json()["token"].is_string());
        assert!(r.json()["options"]["publicKey"]["challenge"].is_string());
        assert_eq!(forme(&r.json()), forme(&reps[0].json()));
    }
    // Deux jetons factices ne sont jamais égaux (pas de valeur constante repérable).
    assert_ne!(reps[0].json()["token"], reps[1].json()["token"]);

    // forgot-password : compte existant ou non → 202 au corps identique.
    let a = post(
        &app,
        "/auth/forgot-password",
        json!({"email": email}),
        Some(&ip),
        None,
        None,
    )
    .await;
    let b = post(
        &app,
        "/auth/forgot-password",
        json!({"email": format!("nul-{}@ent.test", Uuid::new_v4().simple())}),
        Some(&ip),
        None,
        None,
    )
    .await;
    assert_eq!((a.statut, a.texte.as_str()), (StatusCode::ACCEPTED, ""));
    assert_eq!((b.statut, b.texte.as_str()), (a.statut, a.texte.as_str()));

    // Connexion : mot de passe correct mais hors établissement (NotMember) =
    // exactement la réponse d'un mauvais mot de passe ou d'un compte inconnu.
    let mauvais = post(
        &app,
        "/auth/login",
        corps_login(&email, "MauvaisPass1!", etab),
        Some(&ip),
        None,
        None,
    )
    .await;
    let non_membre = post(
        &app,
        "/auth/login",
        corps_login(&email, MDP, autre_etab),
        Some(&ip),
        None,
        None,
    )
    .await;
    let inexistant = post(
        &app,
        "/auth/login",
        corps_login(&inconnu, MDP, etab),
        Some(&ip),
        None,
        None,
    )
    .await;
    assert_eq!(mauvais.statut, StatusCode::UNAUTHORIZED);
    for r in [&non_membre, &inexistant] {
        assert_eq!(r.statut, mauvais.statut);
        assert_eq!(r.texte, mauvais.texte);
    }
    assert!(!non_membre.texte.contains("membre"));
}

// --- Bornes de taille ---------------------------------------------------------

#[tokio::test]
async fn abus_bornes_de_taille_avant_hachage() {
    let Some(state) = etat().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let ip = ip_unique();
    let long_email = format!("{}@ent.test", "a".repeat(250));
    let long_mdp = "x".repeat(257);

    let r = post(
        &app,
        "/auth/login",
        corps_login(&long_email, MDP, etab),
        Some(&ip),
        None,
        None,
    )
    .await;
    assert_eq!(r.statut, StatusCode::BAD_REQUEST, "e-mail > 254");
    let debut = std::time::Instant::now();
    let r = post(
        &app,
        "/auth/login",
        corps_login("a@ent.test", &long_mdp, etab),
        Some(&ip),
        None,
        None,
    )
    .await;
    assert_eq!(r.statut, StatusCode::BAD_REQUEST, "mot de passe > 256");
    assert!(
        debut.elapsed().as_millis() < 500,
        "refus avant tout hachage Argon2"
    );
    let r = post(
        &app,
        "/auth/forgot-password",
        json!({"email": long_email}),
        Some(&ip),
        None,
        None,
    )
    .await;
    assert_eq!(r.statut, StatusCode::BAD_REQUEST);
    let r = post(
        &app,
        "/auth/reset-password",
        json!({"token": "t", "new_password": long_mdp}),
        Some(&ip),
        None,
        None,
    )
    .await;
    assert!(r.statut == StatusCode::BAD_REQUEST, "reset: {}", r.statut);
    let r = post(
        &app,
        "/auth/register",
        json!({"email": "ok@ent.test", "password": long_mdp, "etablissement_id": etab, "role": "parent"}),
        Some(&ip),
        None,
        None,
    )
    .await;
    assert_eq!(
        r.statut,
        StatusCode::BAD_REQUEST,
        "inscription : mot de passe > 256"
    );
    // Les valeurs limites passent la borne (mot de passe de 256 caractères).
    inscrire(
        &app,
        etab,
        &format!("lim-{}@ent.test", Uuid::new_v4().simple()),
        "parent",
    )
    .await;
}
