//! D-018 — récupération de passkey en libre-service par e-mail (HTTP, base locale).
//!
//! Neutralité de la réponse, délai de sécurité, annulation (lien / connexion
//! par passkey), finalisation (passkeys supprimées, sessions révoquées), jeton
//! rejoué, personne de deux établissements, jetons hachés, une seule demande
//! active, limitation de débit. Les dates sont manipulées en base (pas d'attente).

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const MDP: &str = "DevLocalPass1!";
const RECUP: &str = "/auth/passkey/recuperation";

fn url_locale() -> Option<String> {
    std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
}

async fn etat() -> Option<(AppState, Arc<PgEnt>)> {
    let mut ent = PgEnt::connect(&url_locale()?).await.ok()?;
    ent.migrate().await.ok()?;
    ent.install_mail_dry_run();
    let s = AppState::nouveau(ent);
    let ent = s.ent.clone();
    Some((s, ent))
}

async fn pool() -> sqlx::PgPool {
    sqlx::PgPool::connect(&url_locale().unwrap()).await.unwrap()
}

async fn appel(
    app: &axum::Router,
    methode: &str,
    uri: &str,
    jeton: Option<&str>,
    corps: Option<Value>,
) -> (StatusCode, String) {
    let mut b = Request::builder().method(methode).uri(uri);
    if let Some(t) = jeton {
        b = b.header("authorization", format!("Bearer {t}"));
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
    (st, String::from_utf8_lossy(&bytes).into_owned())
}

fn json_de(t: &str) -> Value {
    serde_json::from_str(t).unwrap_or(json!({}))
}

struct Compte {
    jeton: String,
    id: Uuid,
    email: String,
}

async fn inscrire(app: &axum::Router, etab: Uuid, role: &str) -> Compte {
    let email = format!("recup-{role}-{}@ent.local", Uuid::new_v4().simple());
    let (st, _) = appel(
        app,
        "POST",
        "/auth/register",
        None,
        Some(json!({"email": email, "password": MDP, "etablissement_id": etab, "role": role})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "register {role}");
    let (jeton, id) = connecter(app, etab, &email).await;
    Compte { jeton, id, email }
}

async fn connecter(app: &axum::Router, etab: Uuid, email: &str) -> (String, Uuid) {
    let (st, b) = appel(
        app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"email": email, "password": MDP, "etablissement_id": etab})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "login : {b}");
    let b = json_de(&b);
    (
        b["token"].as_str().expect("session").to_string(),
        Uuid::parse_str(b["user_id"].as_str().unwrap()).unwrap(),
    )
}

async fn vivante(app: &axum::Router, jeton: &str) -> bool {
    appel(app, "GET", "/auth/me", Some(jeton), None).await.0 == StatusCode::OK
}

async fn ajouter_appartenance(user_id: Uuid, etab: Uuid, role: &str) {
    sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(etab)
        .bind(role)
        .execute(&pool().await)
        .await
        .unwrap();
}

type Appareil = webauthn_authenticator_rs::WebauthnAuthenticator<
    webauthn_authenticator_rs::softpasskey::SoftPasskey,
>;

fn origine() -> webauthn_authenticator_rs::prelude::Url {
    webauthn_authenticator_rs::prelude::Url::parse("http://localhost:3000").unwrap()
}

/// Enrôle une passkey logicielle et **conserve** l'appareil (connexion ultérieure).
async fn enroler(app: &axum::Router, c: &Compte) -> Appareil {
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;
    use webauthn_authenticator_rs::WebauthnAuthenticator;
    let (st, start) = appel(
        app,
        "POST",
        "/auth/passkey/register/start",
        Some(&c.jeton),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "start : {start}");
    let start = json_de(&start);
    let ccr: ent_auth::CreationChallengeResponse =
        serde_json::from_value(start["options"].clone()).unwrap();
    let mut dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let reg = dev
        .do_registration(origine(), ccr)
        .expect("enregistrement logiciel");
    let (st, b) = appel(
        app,
        "POST",
        "/auth/passkey/register/finish",
        Some(&c.jeton),
        Some(json!({"token": start["token"], "credential": reg, "name": "Clé", "current_password": MDP})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "finish : {b}");
    dev
}

/// Connexion complète mot de passe + passkey ; renvoie le statut de `finish`.
async fn connexion_passkey(
    app: &axum::Router,
    dev: &mut Appareil,
    etab: Uuid,
    email: &str,
) -> StatusCode {
    let (st, b) = appel(
        app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"email": email, "password": MDP, "etablissement_id": etab})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let b = json_de(&b);
    assert_eq!(b["mfa_required"], true, "2ᵉ facteur attendu : {b}");
    let rcr: ent_auth::RequestChallengeResponse =
        serde_json::from_value(b["passkey"]["options"].clone()).unwrap();
    let assertion = dev
        .do_authentication(origine(), rcr)
        .expect("assertion logicielle");
    appel(
        app,
        "POST",
        "/auth/passkey/login/finish",
        None,
        Some(json!({"token": b["passkey"]["token"], "credential": assertion})),
    )
    .await
    .0
}

/// Dernier jeton d'un lien `…/<chemin>?token=…` envoyé à `a`.
fn lien(ent: &PgEnt, a: &str, chemin: &str) -> Option<String> {
    ent.mailer()
        .dry_run_corps()
        .into_iter()
        .rev()
        .filter(|(to, _, _)| to == a)
        .find_map(|(_, _, corps)| {
            let i = corps.find(&format!("{chemin}?token="))? + chemin.len() + "?token=".len();
            Some(
                corps[i..]
                    .chars()
                    .take_while(|c| c.is_ascii_hexdigit())
                    .collect::<String>(),
            )
        })
}

/// Messages de récupération reçus (hors alertes d'enrôlement de passkey).
fn nb_mails(ent: &PgEnt, a: &str) -> usize {
    ent.mailer()
        .dry_run_corps()
        .iter()
        .filter(|(to, sujet, _)| to == a && sujet.contains("cupération"))
        .count()
}

/// Demande + attente de la tâche différée.
async fn demander(
    state: &AppState,
    app: &axum::Router,
    email: &str,
    etab: Uuid,
) -> (StatusCode, String) {
    let r = appel(
        app,
        "POST",
        RECUP,
        None,
        Some(json!({"email": email, "etablissement_id": etab})),
    )
    .await;
    state.attendre_taches().await;
    r
}

async fn jeton(app: &axum::Router, action: &str, token: &str) -> (StatusCode, String) {
    appel(
        app,
        "POST",
        &format!("{RECUP}/{action}"),
        None,
        Some(json!({"token": token})),
    )
    .await
}

async fn statuts(user: Uuid) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT statut FROM ent_recuperation_passkey WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(user)
    .fetch_all(&pool().await)
    .await
    .unwrap()
}

/// Fin du délai de sécurité simulée (au lieu d'attendre).
async fn delai_ecoule(user: Uuid) {
    sqlx::query(
        "UPDATE ent_recuperation_passkey SET effective_at = now() - interval '1 minute'
         WHERE user_id = $1 AND statut = 'en_attente'",
    )
    .bind(user)
    .execute(&pool().await)
    .await
    .unwrap();
}

async fn nb_passkeys(ent: &PgEnt, etab: Uuid, user: Uuid) -> usize {
    ent.list_passkeys_for_actor(etab, user).await.unwrap().len()
}

async fn journal(etab: Uuid, action: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT payload::text FROM ent_journal_modif WHERE etablissement_id = $1 AND action = $2",
    )
    .bind(etab)
    .bind(action)
    .fetch_all(&pool().await)
    .await
    .unwrap()
}

/// Une personne avec une passkey, prête à récupérer.
async fn personne_avec_passkey(app: &axum::Router, etab: Uuid) -> (Compte, Appareil) {
    let c = inscrire(app, etab, "prof").await;
    let dev = enroler(app, &c).await;
    (c, dev)
}

#[tokio::test]
async fn recup_reponse_neutre_et_envoi_seulement_si_eligible() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip recup: ENT_DATABASE_URL");
        return;
    };
    let app = app(state.clone());
    let etab = Uuid::new_v4();
    let (valide, _dev) = personne_avec_passkey(&app, etab).await;
    let sans_passkey = inscrire(&app, etab, "prof").await;
    let inconnu = format!("inconnu-{}@ent.local", Uuid::new_v4().simple());
    let autre_etab = Uuid::new_v4();

    let cas = [
        (valide.email.clone(), etab, true),
        (sans_passkey.email.clone(), etab, false),
        (inconnu.clone(), etab, false),
        (valide.email.clone(), autre_etab, false), // pas membre de cet établissement
    ];
    let mut reponses = vec![];
    for (email, e, _) in &cas {
        reponses.push(demander(&state, &app, email, *e).await);
    }
    for r in &reponses {
        assert_eq!(
            r,
            &(StatusCode::ACCEPTED, String::new()),
            "réponse neutre identique"
        );
    }
    assert_eq!(
        nb_mails(&ent, &valide.email),
        1,
        "un seul e-mail, au compte valide"
    );
    assert_eq!(nb_mails(&ent, &sans_passkey.email), 0);
    assert_eq!(nb_mails(&ent, &inconnu), 0);
    assert_eq!(
        statuts(sans_passkey.id).await.len(),
        0,
        "aucune demande sans passkey"
    );
    // Validations d'entrée (indépendantes de l'existence du compte).
    let (st, _) = appel(
        &app,
        "POST",
        RECUP,
        None,
        Some(json!({"email": " ", "etablissement_id": etab})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let long = format!("{}@ent.local", "a".repeat(300));
    let (st, _) = appel(
        &app,
        "POST",
        RECUP,
        None,
        Some(json!({"email": long, "etablissement_id": etab})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn recup_delai_respecte_puis_finalisation_supprime_passkeys_et_revoque_sessions() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip recup: ENT_DATABASE_URL");
        return;
    };
    let app = app(state.clone());
    let etab = Uuid::new_v4();
    let (c, _dev) = personne_avec_passkey(&app, etab).await;
    assert!(vivante(&app, &c.jeton).await);
    assert_eq!(nb_passkeys(&ent, etab, c.id).await, 1);

    // Avant confirmation : le lien de confirmation n'est pas un jeton de finalisation.
    demander(&state, &app, &c.email, etab).await;
    let confirm =
        lien(&ent, &c.email, "/recuperation-passkey/confirmer").expect("lien de confirmation");
    assert_eq!(confirm.len(), 64, "jeton de 256 bits (hex)");
    assert_eq!(
        jeton(&app, "finaliser", &confirm).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        nb_passkeys(&ent, etab, c.id).await,
        1,
        "rien n'est supprimé sans délai"
    );

    // Confirmation : délai de sécurité (24 h par défaut), alerte avec 2 liens.
    let (st, corps) = jeton(&app, "confirmer", &confirm).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let fin: chrono::DateTime<chrono::Utc> = json_de(&corps)["effective_at"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let h = (fin - chrono::Utc::now()).num_minutes();
    assert!(
        (23 * 60..=24 * 60).contains(&h),
        "délai ≈ 24 h, obtenu {h} min"
    );
    assert_eq!(statuts(c.id).await, vec!["en_attente"]);
    let annul = lien(&ent, &c.email, "/recuperation-passkey/annuler").expect("lien d'annulation");
    let finalisation =
        lien(&ent, &c.email, "/recuperation-passkey/finaliser").expect("lien de finalisation");
    assert_ne!(annul, finalisation);
    assert_ne!(annul, confirm);

    // Délai non écoulé : refus 409, jeton NON consommé, rien de supprimé.
    let (st, _) = jeton(&app, "finaliser", &finalisation).await;
    assert_eq!(st, StatusCode::CONFLICT, "délai de sécurité");
    assert_eq!(nb_passkeys(&ent, etab, c.id).await, 1);
    assert!(vivante(&app, &c.jeton).await);
    // Le jeton de finalisation ne sert pas à annuler (jetons distincts).
    assert_eq!(
        jeton(&app, "annuler", &finalisation).await.0,
        StatusCode::BAD_REQUEST
    );
    // Le lien de confirmation ne peut plus être rejoué.
    assert_eq!(
        jeton(&app, "confirmer", &confirm).await.0,
        StatusCode::BAD_REQUEST
    );

    // Délai écoulé : finalisation.
    delai_ecoule(c.id).await;
    assert_eq!(
        jeton(&app, "finaliser", &finalisation).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        nb_passkeys(&ent, etab, c.id).await,
        0,
        "passkeys supprimées"
    );
    assert!(!vivante(&app, &c.jeton).await, "sessions révoquées");
    assert_eq!(statuts(c.id).await, vec!["terminee"]);
    // Rejeu refusé.
    assert_eq!(
        jeton(&app, "finaliser", &finalisation).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        jeton(&app, "annuler", &annul).await.0,
        StatusCode::BAD_REQUEST
    );
    // Reconnexion par mot de passe seul.
    let (nouvelle, _) = connecter(&app, etab, &c.email).await;
    assert!(vivante(&app, &nouvelle).await);

    // Journal : sans e-mail en clair, avec empreinte.
    for action in [
        "passkey_recuperation_en_attente",
        "passkey_recuperation_finalisee",
    ] {
        let l = journal(etab, action).await;
        assert_eq!(l.len(), 1, "{action}");
        assert!(
            !l[0].contains(&c.email) && l[0].contains("empreinte"),
            "{}",
            l[0]
        );
    }
}

#[tokio::test]
async fn recup_annulation_par_lien() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip recup: ENT_DATABASE_URL");
        return;
    };
    let app = app(state.clone());
    let etab = Uuid::new_v4();
    let (c, _dev) = personne_avec_passkey(&app, etab).await;
    demander(&state, &app, &c.email, etab).await;
    let confirm = lien(&ent, &c.email, "/recuperation-passkey/confirmer").unwrap();
    assert_eq!(jeton(&app, "confirmer", &confirm).await.0, StatusCode::OK);
    let annul = lien(&ent, &c.email, "/recuperation-passkey/annuler").unwrap();
    let finalisation = lien(&ent, &c.email, "/recuperation-passkey/finaliser").unwrap();

    assert_eq!(
        jeton(&app, "annuler", &annul).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(statuts(c.id).await, vec!["annulee"]);
    assert_eq!(
        jeton(&app, "annuler", &annul).await.0,
        StatusCode::BAD_REQUEST,
        "usage unique"
    );
    // Même délai écoulé, la finalisation est refusée et rien n'est supprimé.
    delai_ecoule(c.id).await;
    assert_eq!(
        jeton(&app, "finaliser", &finalisation).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(nb_passkeys(&ent, etab, c.id).await, 1);
    assert!(vivante(&app, &c.jeton).await);
    assert_eq!(journal(etab, "passkey_recuperation_annulee").await.len(), 1);
}

#[tokio::test]
async fn recup_annulation_par_connexion_passkey_pendant_le_delai() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip recup: ENT_DATABASE_URL");
        return;
    };
    let app = app(state.clone());
    let etab = Uuid::new_v4();
    let (c, mut dev) = personne_avec_passkey(&app, etab).await;
    demander(&state, &app, &c.email, etab).await;
    let confirm = lien(&ent, &c.email, "/recuperation-passkey/confirmer").unwrap();
    assert_eq!(jeton(&app, "confirmer", &confirm).await.0, StatusCode::OK);
    let finalisation = lien(&ent, &c.email, "/recuperation-passkey/finaliser").unwrap();

    // La personne retrouve sa passkey et se connecte : la récupération est annulée.
    assert_eq!(
        connexion_passkey(&app, &mut dev, etab, &c.email).await,
        StatusCode::OK
    );
    assert_eq!(statuts(c.id).await, vec!["annulee"]);
    delai_ecoule(c.id).await;
    assert_eq!(
        jeton(&app, "finaliser", &finalisation).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(nb_passkeys(&ent, etab, c.id).await, 1, "passkey conservée");
    assert_eq!(journal(etab, "passkey_recuperation_annulee").await.len(), 1);
}

#[tokio::test]
async fn recup_personne_de_deux_etablissements_alertes_cloisonnees() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip recup: ENT_DATABASE_URL");
        return;
    };
    let app = app(state.clone());
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let direction_a = inscrire(&app, a, "direction").await;
    let (c, _dev) = personne_avec_passkey(&app, a).await;
    ajouter_appartenance(c.id, b, "prof").await;

    // Bloquée côté direction (identité multi-établissements) : d'où D-018.
    let (st, _) = appel(
        &app,
        "DELETE",
        &format!("/auth/users/{}/passkeys", c.id),
        Some(&direction_a.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Récupération en libre-service, demandée via B.
    demander(&state, &app, &c.email, b).await;
    let confirm = lien(&ent, &c.email, "/recuperation-passkey/confirmer").expect("lien");
    assert_eq!(jeton(&app, "confirmer", &confirm).await.0, StatusCode::OK);
    let finalisation = lien(&ent, &c.email, "/recuperation-passkey/finaliser").unwrap();

    // Alerte journalisée dans chaque établissement, sans révéler l'autre.
    for (ici, ailleurs) in [(a, b), (b, a)] {
        let l = journal(ici, "passkey_recuperation_en_attente").await;
        assert_eq!(l.len(), 1, "alerte dans l'établissement {ici}");
        assert!(
            !l[0].contains(&ailleurs.to_string()),
            "pas de fuite de l'autre établissement"
        );
        assert!(!l[0].contains(&c.email));
    }
    delai_ecoule(c.id).await;
    assert_eq!(
        jeton(&app, "finaliser", &finalisation).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(nb_passkeys(&ent, a, c.id).await, 0);
    for etab in [a, b] {
        assert_eq!(
            journal(etab, "passkey_recuperation_finalisee").await.len(),
            1
        );
        let (jt, _) = connecter(&app, etab, &c.email).await;
        assert!(
            vivante(&app, &jt).await,
            "connexion par mot de passe dans {etab}"
        );
    }
}

#[tokio::test]
async fn recup_jetons_haches_une_seule_demande_active_et_expirations() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip recup: ENT_DATABASE_URL");
        return;
    };
    let app = app(state.clone());
    let etab = Uuid::new_v4();
    let (c, _dev) = personne_avec_passkey(&app, etab).await;

    // Deux demandes : la première est remplacée (un seul lien actif).
    demander(&state, &app, &c.email, etab).await;
    let premier = lien(&ent, &c.email, "/recuperation-passkey/confirmer").unwrap();
    demander(&state, &app, &c.email, etab).await;
    let second = lien(&ent, &c.email, "/recuperation-passkey/confirmer").unwrap();
    assert_ne!(premier, second);
    assert_eq!(statuts(c.id).await, vec!["annulee", "demandee"]);
    assert_eq!(
        jeton(&app, "confirmer", &premier).await.0,
        StatusCode::BAD_REQUEST
    );

    // Seuls les hachés sont en base.
    let brut: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_recuperation_passkey WHERE confirm_hash = $1 OR confirm_hash = $2",
    )
    .bind(&premier)
    .bind(&second)
    .fetch_one(&pool().await)
    .await
    .unwrap();
    assert_eq!(brut, 0, "aucun jeton en clair en base");

    // Lien de confirmation expiré.
    sqlx::query("UPDATE ent_recuperation_passkey SET confirm_expires_at = now() - interval '1 second' WHERE user_id = $1 AND statut = 'demandee'")
        .bind(c.id).execute(&pool().await).await.unwrap();
    assert_eq!(
        jeton(&app, "confirmer", &second).await.0,
        StatusCode::BAD_REQUEST
    );

    // Nouvelle demande, confirmée ; une demande en attente n'est pas écrasée.
    ent.mailer().clear_receipts();
    let sql_nettoyage = sqlx::query("DELETE FROM ent_rate_limit")
        .execute(&pool().await)
        .await;
    let _ = sql_nettoyage;
    demander(&state, &app, &c.email, etab).await;
    let l3 = lien(&ent, &c.email, "/recuperation-passkey/confirmer").unwrap();
    assert_eq!(jeton(&app, "confirmer", &l3).await.0, StatusCode::OK);
    let finalisation = lien(&ent, &c.email, "/recuperation-passkey/finaliser").unwrap();
    let avant = nb_mails(&ent, &c.email);
    demander(&state, &app, &c.email, etab).await;
    assert_eq!(
        nb_mails(&ent, &c.email),
        avant,
        "pas de nouvelle demande pendant le délai"
    );
    assert_eq!(statuts(c.id).await.last().unwrap(), "en_attente");

    // Fenêtre de finalisation dépassée → expirée.
    delai_ecoule(c.id).await;
    sqlx::query("UPDATE ent_recuperation_passkey SET final_expires_at = now() - interval '1 second' WHERE user_id = $1 AND statut = 'en_attente'")
        .bind(c.id).execute(&pool().await).await.unwrap();
    assert_eq!(
        jeton(&app, "finaliser", &finalisation).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(statuts(c.id).await.last().unwrap(), "expiree");
    assert_eq!(nb_passkeys(&ent, etab, c.id).await, 1);

    // Bornes de taille / jeton aléatoire.
    for action in ["confirmer", "annuler", "finaliser"] {
        assert_eq!(
            jeton(&app, action, &"a".repeat(300)).await.0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(jeton(&app, action, "  ").await.0, StatusCode::BAD_REQUEST);
        assert_eq!(
            jeton(&app, action, &"ab".repeat(32)).await.0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn recup_limitation_de_debit_par_adresse() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip recup: ENT_DATABASE_URL");
        return;
    };
    let app = app(state.clone());
    let email = format!("spam-{}@ent.local", Uuid::new_v4().simple());
    let corps = json!({"email": email, "etablissement_id": Uuid::new_v4()});
    for i in 0..2 {
        let (st, _) = appel(&app, "POST", RECUP, None, Some(corps.clone())).await;
        assert_eq!(st, StatusCode::ACCEPTED, "demande {i}");
    }
    let (st, _) = appel(&app, "POST", RECUP, None, Some(corps)).await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS);
}
