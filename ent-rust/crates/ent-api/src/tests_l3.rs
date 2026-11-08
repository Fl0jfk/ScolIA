//! Lot L3 — sécurité des comptes et de la récupération (HTTP, base locale).
//!
//! Règles couvertes : changement d'e-mail (hiérarchie, mot de passe pour soi,
//! révocation, journal pseudonymisé, notification à l'ancienne adresse),
//! réinitialisation par le personnel (lien sans secret pour parent / élève),
//! réinitialisation des passkeys (hiérarchie, acteur réel, révocation),
//! passkeys de l'utilisateur (mot de passe, plafond, nom, révocation des
//! autres sessions), rattachement sans mot de passe.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

pub(crate) const MDP: &str = "DevLocalPass1!";

pub(crate) fn url_locale() -> Option<String> {
    std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
}

/// État de test avec messagerie en dry-run (reçus consultables).
pub(crate) async fn etat() -> Option<(AppState, Arc<PgEnt>)> {
    let mut ent = PgEnt::connect(&url_locale()?).await.ok()?;
    ent.migrate().await.ok()?;
    ent.install_mail_dry_run();
    let s = AppState::nouveau(ent);
    let ent = s.ent.clone();
    Some((s, ent))
}

pub(crate) async fn appel(
    app: &axum::Router,
    methode: &str,
    uri: &str,
    jeton: Option<&str>,
    corps: Option<Value>,
) -> (StatusCode, Value) {
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
    (st, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

pub(crate) struct Compte {
    pub(crate) jeton: String,
    pub(crate) id: Uuid,
    pub(crate) email: String,
}

pub(crate) async fn inscrire(app: &axum::Router, etab: Uuid, role: &str) -> Compte {
    let email = format!("l3-{role}-{}@ent.local", Uuid::new_v4().simple());
    let (st, _) = appel(
        app,
        "POST",
        "/auth/register",
        None,
        Some(json!({"email": email, "password": MDP, "etablissement_id": etab, "role": role})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "register {role}");
    let c = connecter(app, etab, &email).await;
    Compte {
        jeton: c.0,
        id: c.1,
        email,
    }
}

pub(crate) async fn connecter(app: &axum::Router, etab: Uuid, email: &str) -> (String, Uuid) {
    let (st, b) = appel(
        app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"email": email, "password": MDP, "etablissement_id": etab})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "login");
    (
        b["token"].as_str().unwrap().to_string(),
        Uuid::parse_str(b["user_id"].as_str().unwrap()).unwrap(),
    )
}

pub(crate) async fn vivante(app: &axum::Router, jeton: &str) -> bool {
    appel(app, "GET", "/auth/me", Some(jeton), None).await.0 == StatusCode::OK
}

/// Lignes de journal (action, acteur, payload en texte) pour un établissement.
async fn journal(etab: Uuid, action: &str) -> Vec<(Option<Uuid>, String)> {
    let pool = sqlx::PgPool::connect(&url_locale().unwrap()).await.unwrap();
    let rows: Vec<(Option<Uuid>, String)> = sqlx::query_as(
        "SELECT actor_user_id, payload::text FROM ent_journal_modif \
         WHERE etablissement_id = $1 AND action = $2",
    )
    .bind(etab)
    .bind(action)
    .fetch_all(&pool)
    .await
    .unwrap();
    rows
}

/// Dernier jeton d'un lien `…/<chemin>?token=…` reçu par `a`.
pub(crate) fn lien_email(ent: &PgEnt, a: &str, chemin: &str) -> Option<String> {
    ent.mailer().dry_run_corps().into_iter().rev().filter(|(to, _, _)| to == a).find_map(
        |(_, _, corps)| {
            let i = corps.find(&format!("{chemin}?token="))? + chemin.len() + "?token=".len();
            Some(corps[i..].chars().take_while(|c| c.is_ascii_hexdigit()).collect::<String>())
        },
    )
}

async fn prof_parent_neuf(app: &axum::Router, etab: Uuid) -> Compte {
    inscrire(app, etab, "parent").await
}

fn email_uri(id: Uuid) -> String {
    format!("/users/{id}/email")
}

#[tokio::test]
async fn l3_email_hierarchie_mot_de_passe_revocation_journal_notification() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip L3: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let admin = inscrire(&app, etab, "admin").await;
    let dir = inscrire(&app, etab, "direction").await;
    let adm = inscrire(&app, etab, "administratif").await;
    let prof = inscrire(&app, etab, "prof").await;
    let parent = inscrire(&app, etab, "parent").await;
    let nouveau = |p: &str| format!("l3-nouveau-{p}-{}@ent.local", Uuid::new_v4().simple());

    // Administratif : jamais Admin, Direction, ni un autre Administratif.
    for cible in [&admin, &dir] {
        let (st, _) = appel(&app, "PUT", &email_uri(cible.id), Some(&adm.jeton),
            Some(json!({"email": nouveau("pirate")}))).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "administratif → admin/direction");
    }
    // Direction : jamais un Admin.
    let (st, _) = appel(&app, "PUT", &email_uri(admin.id), Some(&dir.jeton),
        Some(json!({"email": nouveau("pirate")}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "direction → admin");
    // Un prof ne change pas l'e-mail d'un tiers.
    let (st, _) = appel(&app, "PUT", &email_uri(parent.id), Some(&prof.jeton),
        Some(json!({"email": nouveau("pirate")}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "prof → tiers");

    // Soi-même : mot de passe actuel obligatoire.
    let (st, _) = appel(&app, "PUT", &email_uri(prof.id), Some(&prof.jeton),
        Some(json!({"email": nouveau("moi")}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "soi-même sans mot de passe");
    let (st, _) = appel(&app, "PUT", &email_uri(prof.id), Some(&prof.jeton),
        Some(json!({"email": nouveau("moi"), "current_password": "Mauvais-Pass-1"}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "soi-même, mauvais mot de passe");
    assert!(vivante(&app, &prof.jeton).await, "refus : session intacte");

    // RF-01 : l'Administratif ne touche plus aux comptes du personnel.
    let (st, _) = appel(&app, "PUT", &email_uri(prof.id), Some(&adm.jeton),
        Some(json!({"email": nouveau("pirate")}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "administratif → prof");
    // Direction → prof : accepté mais pas effectif avant confirmation par la nouvelle
    // adresse ; ancienne adresse prévenue avec un lien d'annulation.
    let nouvelle = nouveau("prof");
    ent.mailer().clear_receipts();
    let (st, _) = appel(&app, "PUT", &email_uri(prof.id), Some(&dir.jeton),
        Some(json!({"email": nouvelle}))).await;
    assert_eq!(st, StatusCode::ACCEPTED);
    assert!(vivante(&app, &prof.jeton).await, "rien n'est effectif avant confirmation");
    let recus = ent.mailer().receipts();
    assert!(recus.iter().any(|r| r.to == prof.email), "l'ancienne adresse est prévenue");
    assert!(recus.iter().any(|r| r.to == nouvelle), "la nouvelle adresse reçoit le lien");
    let jeton = lien_email(&ent, &nouvelle, "/email/confirmer").expect("lien de confirmation");
    let (st, _) = appel(&app, "POST", "/auth/email/confirmer", None,
        Some(json!({"token": jeton}))).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    assert!(!vivante(&app, &prof.jeton).await, "sessions de la cible révoquées");
    let lignes = journal(etab, "email_modifie").await;
    let l = lignes
        .iter()
        .find(|(_, p)| p.contains(&prof.id.to_string()))
        .expect("ligne de journal email_modifie");
    assert_eq!(l.0, Some(dir.id), "acteur réel = la direction");
    assert!(!l.1.contains(&prof.email) && !l.1.contains(&nouvelle), "pas d'adresse en clair");
    assert!(l.1.contains("ancienne_empreinte") && l.1.contains("nouvelle_empreinte"));

    // Soi-même avec le bon mot de passe : demande acceptée, effective après confirmation.
    let propre = nouveau("parent");
    let (st, _) = appel(&app, "PUT", &email_uri(parent.id), Some(&parent.jeton),
        Some(json!({"email": propre, "current_password": MDP}))).await;
    assert_eq!(st, StatusCode::ACCEPTED);
    assert!(vivante(&app, &parent.jeton).await);
    let jeton = lien_email(&ent, &propre, "/email/confirmer").expect("lien de confirmation");
    let (st, _) = appel(&app, "POST", "/auth/email/confirmer", None,
        Some(json!({"token": jeton}))).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    assert!(!vivante(&app, &parent.jeton).await);

    // Administratif → parent : niveau strictement inférieur, accepté.
    let (st, _) = appel(&app, "PUT", &email_uri(prof_parent_neuf(&app, etab).await.id),
        Some(&adm.jeton), Some(json!({"email": nouveau("parent2")}))).await;
    assert_eq!(st, StatusCode::ACCEPTED, "administratif → parent");

    // Direction → administratif / Admin → direction : autorisés.
    let (st, _) = appel(&app, "PUT", &email_uri(adm.id), Some(&dir.jeton),
        Some(json!({"email": nouveau("adm")}))).await;
    assert_eq!(st, StatusCode::ACCEPTED);
    let (st, _) = appel(&app, "PUT", &email_uri(dir.id), Some(&admin.jeton),
        Some(json!({"email": nouveau("dir")}))).await;
    assert_eq!(st, StatusCode::ACCEPTED);
    // Jamais d'un pair : Direction → Direction.
    let dir2 = inscrire(&app, etab, "direction").await;
    let (st, _) = appel(&app, "PUT", &email_uri(dir2.id), Some(&dir.jeton),
        Some(json!({"email": nouveau("pair")}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "direction → direction (pair)");
}

#[tokio::test]
async fn l3_reinitialisation_par_lien_sans_secret_pour_tous_les_profils() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip L3: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let admin = inscrire(&app, etab, "admin").await;
    let dir = inscrire(&app, etab, "direction").await;
    let adm = inscrire(&app, etab, "administratif").await;
    let prof = inscrire(&app, etab, "prof").await;
    let parent = inscrire(&app, etab, "parent").await;
    let uri = |id: Uuid| format!("/etablissement/personnel/{id}/reset-mot-de-passe");

    // Parent : lien envoyé, 202, aucun secret ; le mot de passe n'est pas modifié.
    ent.mailer().clear_receipts();
    let (st, corps) = appel(&app, "POST", &uri(parent.id), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::ACCEPTED);
    assert_eq!(corps["lien_envoye"], true);
    assert!(corps.get("mot_de_passe_temporaire").is_none(), "aucun secret renvoyé : {corps}");
    assert!(ent.mailer().receipts().iter().any(|r| r.to == parent.email
        && r.subject.contains("Réinitialisation")));
    assert!(vivante(&app, &parent.jeton).await, "l'envoi du lien ne coupe pas la session");
    connecter(&app, etab, &parent.email).await; // ancien mot de passe toujours valable

    // Administratif / prof : pas de droit ; soi-même interdit.
    for t in [&adm.jeton, &prof.jeton] {
        let (st, _) = appel(&app, "POST", &uri(parent.id), Some(t), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }
    let (st, _) = appel(&app, "POST", &uri(dir.id), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Personnel (socle 6 P0) : plus de mot de passe temporaire — lien envoyé (202), aucun secret
    // dans la réponse, sessions révoquées, ancien mot de passe valable tant que le lien n'est pas
    // utilisé.
    ent.mailer().clear_receipts();
    let (st, corps) = appel(&app, "POST", &uri(prof.id), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::ACCEPTED);
    assert_eq!(corps["lien_envoye"], true);
    assert!(corps.get("mot_de_passe_temporaire").is_none(), "aucun secret renvoyé : {corps}");
    assert!(!corps.to_string().to_lowercase().contains("password"), "{corps}");
    assert!(ent.mailer().receipts().iter().any(|r| r.to == prof.email
        && r.subject.contains("Réinitialisation")));
    assert!(!vivante(&app, &prof.jeton).await);
    connecter(&app, etab, &prof.email).await;
    // Direction ne touche pas un Admin.
    let (st, _) = appel(&app, "POST", &uri(admin.id), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn l3_reinitialisation_parent_sans_messagerie_prete_reponse_explicite() {
    let Some(url) = url_locale() else {
        eprintln!("skip L3: ENT_DATABASE_URL");
        return;
    };
    let ent = PgEnt::connect(&url).await.unwrap();
    ent.migrate().await.unwrap();
    if ent.mailer().is_ready() {
        eprintln!("skip L3: messagerie configurée dans l'environnement");
        return;
    }
    let app = app(AppState::nouveau(ent));
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let parent = inscrire(&app, etab, "parent").await;
    let (st, corps) = appel(
        &app,
        "POST",
        &format!("/etablissement/personnel/{}/reset-mot-de-passe", parent.id),
        Some(&dir.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE);
    assert!(corps.get("mot_de_passe_temporaire").is_none());
}

#[tokio::test]
async fn l3_reset_passkeys_hierarchie_acteur_reel_et_revocation() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip L3: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let admin = inscrire(&app, etab, "admin").await;
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let uri = |id: Uuid| format!("/auth/users/{id}/passkeys");

    let (st, _) = appel(&app, "DELETE", &uri(admin.id), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "direction → admin");
    let (st, _) = appel(&app, "DELETE", &uri(dir.id), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "pas soi-même");
    let (st, _) = appel(&app, "DELETE", &uri(prof.id), Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "prof");

    let (st, _) = appel(&app, "DELETE", &uri(prof.id), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(!vivante(&app, &prof.jeton).await, "sessions de la cible révoquées");
    let l = journal(etab, "passkey_reset_admin").await;
    let ligne = l
        .iter()
        .find(|(_, p)| p.contains(&prof.id.to_string()))
        .expect("journal passkey_reset_admin");
    assert_eq!(ligne.0, Some(dir.id), "acteur réel = la direction");

    let (st, _) = appel(&app, "DELETE", &uri(dir.id), Some(&admin.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "admin → direction");
}

pub(crate) async fn enroler(
    app: &axum::Router,
    jeton: &str,
    mot_de_passe: Option<&str>,
    nom: &str,
) -> (StatusCode, Value) {
    use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator};
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;
    let (st, start) = appel(app, "POST", "/auth/passkey/register/start", Some(jeton), Some(json!({}))).await;
    if st != StatusCode::OK {
        return (st, start);
    }
    let ccr: ent_auth::CreationChallengeResponse =
        serde_json::from_value(start["options"].clone()).unwrap();
    let mut dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let reg = dev
        .do_registration(Url::parse("http://localhost:3000").unwrap(), ccr)
        .expect("enregistrement logiciel");
    let mut corps = json!({"token": start["token"], "credential": reg, "name": nom});
    if let Some(p) = mot_de_passe {
        corps["current_password"] = json!(p);
    }
    appel(app, "POST", "/auth/passkey/register/finish", Some(jeton), Some(corps)).await
}

#[tokio::test]
async fn l3_passkeys_utilisateur_mot_de_passe_nom_plafond_et_revocation_des_autres_sessions() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip L3: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let c = inscrire(&app, etab, "prof").await;
    let autre = connecter(&app, etab, &c.email).await.0; // 2ᵉ session du même compte

    // Ajout : mot de passe obligatoire et exact, nom borné.
    let (st, _) = enroler(&app, &c.jeton, None, "Sans mot de passe").await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = enroler(&app, &c.jeton, Some("Mauvais-Pass-1"), "Mauvais").await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = enroler(&app, &c.jeton, Some(MDP), &"x".repeat(65)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "nom trop long");
    let (st, _) = enroler(&app, &c.jeton, Some(MDP), "ctl\u{0007}char").await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "caractère de contrôle");
    for (nom, cas) in [("bidi", "ma\u{202E}clé"), ("largeur nulle", "ma\u{200B}clé"), ("BOM", "\u{FEFF}clé x")] {
        let (st, _) = enroler(&app, &c.jeton, Some(MDP), cas).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "caractère Unicode Cf ({nom})");
    }
    assert!(vivante(&app, &autre).await, "refus : l'autre session survit");

    let (st, cree) = enroler(&app, &c.jeton, Some(MDP), "  Ma clé  ").await;
    assert_eq!(st, StatusCode::CREATED);
    assert_eq!(cree["name"], "Ma clé", "nom nettoyé (trim)");
    assert!(vivante(&app, &c.jeton).await, "la session courante est conservée");
    assert!(!vivante(&app, &autre).await, "les autres sessions sont révoquées");
    let id = cree["id"].as_str().unwrap().to_string();

    // Suppression : mot de passe obligatoire ; révocation des autres sessions.
    let (st, _) = appel(&app, "DELETE", &format!("/auth/passkeys/{id}"), Some(&c.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "suppression sans mot de passe");
    let (st, _) = appel(&app, "DELETE", &format!("/auth/passkeys/{id}"), Some(&c.jeton),
        Some(json!({"current_password": "Mauvais-Pass-1"}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "suppression, mauvais mot de passe");
    let (st, _) = appel(&app, "DELETE", &format!("/auth/passkeys/{id}"), Some(&c.jeton),
        Some(json!({"current_password": MDP}))).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (_, liste) = appel(&app, "GET", "/auth/passkeys", Some(&c.jeton), None).await;
    assert_eq!(liste.as_array().unwrap().len(), 0);

    // Plafond : 10 passkeys, la 11ᵉ est refusée (409).
    for i in 0..10 {
        let (st, b) = enroler(&app, &c.jeton, Some(MDP), &format!("clé {i}")).await;
        assert_eq!(st, StatusCode::CREATED, "enrôlement {i} : {b}");
    }
    let (st, _) = enroler(&app, &c.jeton, Some(MDP), "de trop").await;
    assert_eq!(st, StatusCode::CONFLICT, "plafond de 10 passkeys");
}


async fn pool() -> sqlx::PgPool {
    sqlx::PgPool::connect(&url_locale().unwrap()).await.unwrap()
}

/// Jetons de réinitialisation encore actifs d'un compte.
async fn jetons_actifs(user_id: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_password_reset WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(&pool().await)
    .await
    .unwrap()
}

/// Donne au même compte une appartenance dans un second établissement.
async fn ajouter_appartenance(user_id: Uuid, etab: Uuid, role: &str) {
    sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(etab)
        .bind(role)
        .execute(&pool().await)
        .await
        .unwrap();
}

#[tokio::test]
async fn l3_administratif_ne_change_pas_l_email_d_un_autre_administratif() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip L3: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let a1 = inscrire(&app, etab, "administratif").await;
    let a2 = inscrire(&app, etab, "administratif").await;
    let (st, _) = appel(&app, "PUT", &email_uri(a2.id), Some(&a1.jeton),
        Some(json!({"email": format!("pirate-{}@ent.local", Uuid::new_v4().simple())}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "administratif → administratif");
    assert!(vivante(&app, &a2.jeton).await, "refus : session de la cible intacte");
}

#[tokio::test]
async fn l3_cible_membre_d_un_autre_etablissement_refusee_pour_tout_tiers_403_uniforme() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip L3: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let (etab, autre_etab) = (Uuid::new_v4(), Uuid::new_v4());
    let admin = inscrire(&app, etab, "admin").await;
    let dir = inscrire(&app, etab, "direction").await;
    let parent = inscrire(&app, etab, "parent").await;
    let prof = inscrire(&app, etab, "prof").await;
    // Même identité, second établissement (parent ET prof : quel que soit le rôle).
    ajouter_appartenance(parent.id, autre_etab, "parent").await;
    ajouter_appartenance(prof.id, autre_etab, "eleve").await;
    let nouveau = || format!("l3-x-{}@ent.local", Uuid::new_v4().simple());

    for cible in [&parent, &prof] {
        for acteur in [&dir, &admin] {
            let (st, _) = appel(&app, "PUT", &email_uri(cible.id), Some(&acteur.jeton),
                Some(json!({"email": nouveau()}))).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "e-mail multi-établissements");
            let (st, _) = appel(&app, "POST",
                &format!("/etablissement/personnel/{}/reset-mot-de-passe", cible.id),
                Some(&acteur.jeton), None).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "mot de passe multi-établissements");
            let (st, _) = appel(&app, "DELETE", &format!("/auth/users/{}/passkeys", cible.id),
                Some(&acteur.jeton), None).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "passkeys multi-établissements");
        }
        assert!(vivante(&app, &cible.jeton).await, "aucune session coupée");
    }
    // 403 uniforme : un compte inconnu de l'établissement n'est pas un 404.
    let (st, _) = appel(&app, "PUT", &email_uri(Uuid::new_v4()), Some(&dir.jeton),
        Some(json!({"email": nouveau()}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "non-membre : 403, pas 404");

    // Le propriétaire garde ses actions (avec son mot de passe actuel).
    let (st, _) = appel(&app, "PUT", &email_uri(parent.id), Some(&parent.jeton),
        Some(json!({"email": nouveau(), "current_password": MDP}))).await;
    assert_eq!(st, StatusCode::ACCEPTED, "le propriétaire demande son changement d'e-mail");
}

#[tokio::test]
async fn l3_jetons_en_attente_invalides_par_email_reset_mot_de_passe_et_reset_passkeys() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip L3: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let adm = inscrire(&app, etab, "administratif").await;
    let prof = inscrire(&app, etab, "prof").await;
    let parent = inscrire(&app, etab, "parent").await;
    let eleve = inscrire(&app, etab, "eleve").await;

    // Changement d'e-mail : le lien envoyé à l'ancienne adresse meurt.
    ent.demander_reinitialisation_mot_de_passe(&parent.email, "http://localhost:8088").await.unwrap();
    assert_eq!(jetons_actifs(parent.id).await, 1, "jeton en attente");
    let (st, _) = appel(&app, "PUT", &email_uri(parent.id), Some(&adm.jeton),
        Some(json!({"email": format!("l3-n-{}@ent.local", Uuid::new_v4().simple())}))).await;
    assert_eq!(st, StatusCode::ACCEPTED);
    assert_eq!(jetons_actifs(parent.id).await, 0, "jeton invalidé après changement d'e-mail");

    // Réinitialisation du mot de passe d'un membre du personnel.
    ent.demander_reinitialisation_mot_de_passe(&prof.email, "http://localhost:8088").await.unwrap();
    assert_eq!(jetons_actifs(prof.id).await, 1);
    let (st, _) = appel(&app, "POST",
        &format!("/etablissement/personnel/{}/reset-mot-de-passe", prof.id),
        Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::ACCEPTED);
    // Socle 6 P0 : la réinitialisation envoie un nouveau lien ; l'ancien est remplacé, il n'en reste qu'un.
    assert_eq!(jetons_actifs(prof.id).await, 1, "ancien lien remplacé par le nouveau");

    // Reset des passkeys.
    ent.demander_reinitialisation_mot_de_passe(&eleve.email, "http://localhost:8088").await.unwrap();
    assert_eq!(jetons_actifs(eleve.id).await, 1);
    let (st, _) = appel(&app, "DELETE", &format!("/auth/users/{}/passkeys", eleve.id),
        Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(jetons_actifs(eleve.id).await, 0, "jeton invalidé après reset passkeys");
}

#[tokio::test]
async fn l3_mauvais_mot_de_passe_actuel_ne_touche_pas_au_compteur_de_connexion() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip L3: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let c = inscrire(&app, etab, "prof").await;
    let nouveau = || format!("l3-r-{}@ent.local", Uuid::new_v4().simple());

    for i in 0..5 {
        let (st, _) = appel(&app, "PUT", &email_uri(c.id), Some(&c.jeton),
            Some(json!({"email": nouveau(), "current_password": "Mauvais-Pass-1"}))).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "mauvais mot de passe n°{i}");
    }
    // Compteur dédié épuisé : 429 sur ces seules routes, même avec le bon mot de passe.
    let (st, _) = appel(&app, "PUT", &email_uri(c.id), Some(&c.jeton),
        Some(json!({"email": nouveau(), "current_password": MDP}))).await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS, "ré-authentification verrouillée");

    // Le compteur de LOGIN n'a pas bougé : aucune ligne pour l'e-mail, login possible.
    let lignes: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_login_guard WHERE email = $1")
        .bind(c.email.to_lowercase())
        .fetch_one(&pool().await)
        .await
        .unwrap();
    assert_eq!(lignes, 0, "compteur de connexion intact");
    connecter(&app, etab, &c.email).await;
}

#[tokio::test]
async fn l3_pas_de_faux_202_messagerie_sans_envoi_confirme_503_et_jeton_invalide() {
    let Some(url) = url_locale() else {
        eprintln!("skip L3: ENT_DATABASE_URL");
        return;
    };
    for (live, nom) in [(false, "dry-run non accepté"), (true, "échec SMTP")] {
        let mut ent = PgEnt::connect(&url).await.unwrap();
        ent.migrate().await.unwrap();
        ent.installer_mailer_de_test(live, false);
        let state = AppState::nouveau(ent);
        let taches = state.clone();
        let app = app(state);
        let etab = Uuid::new_v4();
        let dir = inscrire(&app, etab, "direction").await;
        let parent = inscrire(&app, etab, "parent").await;
        let uri = format!("/etablissement/personnel/{}/reset-mot-de-passe", parent.id);

        let (st, corps) = appel(&app, "POST", &uri, Some(&dir.jeton), None).await;
        assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE, "{nom} : pas de faux 202");
        assert!(corps.get("lien_envoye").is_none());
        assert_eq!(jetons_actifs(parent.id).await, 0, "{nom} : jeton invalidé");

        // « Mot de passe oublié » public : réponse neutre 202, mais aucun jeton actif.
        let (st, _) = appel(&app, "POST", "/auth/forgot-password", None,
            Some(json!({"email": parent.email}))).await;
        assert_eq!(st, StatusCode::ACCEPTED, "{nom} : réponse publique neutre");
        // L'envoi est différé hors requête : attendre la fin avant de vérifier.
        taches.attendre_taches().await;
        assert_eq!(jetons_actifs(parent.id).await, 0, "{nom} : aucun jeton public actif");
    }

    // URL publique absente (hors labo) : le service refuse d'émettre un jeton.
    let mut ent = PgEnt::connect(&url).await.unwrap();
    ent.migrate().await.unwrap();
    ent.install_mail_dry_run();
    let state = AppState::nouveau(ent);
    let ent = state.ent.clone();
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let parent = inscrire(&app, etab, "parent").await;
    let r = ent
        .reinitialiser_mot_de_passe_for_actor(etab, dir.id, "direction", parent.id, "")
        .await;
    assert!(matches!(r, Err(ent_db::EntDbError::EnvoiImpossible)), "URL absente → 503");
    assert_eq!(jetons_actifs(parent.id).await, 0, "aucun jeton créé sans URL publique");
}

#[tokio::test]
async fn l3_reset_passkeys_admin_supprime_reellement_les_passkeys_creees() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip L3: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    for i in 0..2 {
        let (st, b) = enroler(&app, &prof.jeton, Some(MDP), &format!("clé {i}")).await;
        assert_eq!(st, StatusCode::CREATED, "enrôlement {i} : {b}");
    }
    assert_eq!(ent.list_passkeys_for_actor(etab, prof.id).await.unwrap().len(), 2);

    let (st, _) = appel(&app, "DELETE", &format!("/auth/users/{}/passkeys", prof.id),
        Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(ent.list_passkeys_for_actor(etab, prof.id).await.unwrap().len(), 0,
        "toutes les passkeys ont été supprimées");
    assert!(!vivante(&app, &prof.jeton).await, "sessions révoquées");
    connecter(&app, etab, &prof.email).await; // retombé en simple mot de passe
}

/// Socle 1 — la passkey appartient à la **personne** (D-013) : une passkey
/// enrôlée depuis l'établissement A est exigée aussi à la connexion par B.
/// Avant correction, un membre de deux établissements contournait son 2ᵉ
/// facteur en se connectant par l'établissement où il n'avait rien enrôlé.
#[tokio::test]
async fn socle1_passkey_exigee_dans_tous_les_etablissements_de_la_personne() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip socle1: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab_a = Uuid::new_v4();
    let etab_b = Uuid::new_v4();
    let prof = inscrire(&app, etab_a, "prof").await;
    ajouter_appartenance(prof.id, etab_b, "prof").await;
    let (st, b) = enroler(&app, &prof.jeton, Some(MDP), "Clé A").await;
    assert_eq!(st, StatusCode::CREATED, "enrôlement depuis A : {b}");

    for etab in [etab_a, etab_b] {
        let (st, corps) = appel(
            &app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": prof.email, "password": MDP, "etablissement_id": etab})),
        )
        .await;
        assert_eq!(st, StatusCode::OK);
        assert!(
            corps.get("token").map(|t| t.is_null()).unwrap_or(true),
            "mot de passe seul : aucune session, établissement {etab}"
        );
        assert_eq!(corps["mfa_required"], json!(true),
            "2ᵉ facteur exigé dans l'établissement {etab} : {corps}");
    }
    // La liste des passkeys est la même vue de A et de B.
    assert_eq!(ent.list_passkeys_for_actor(etab_b, prof.id).await.unwrap().len(), 1);
}

/// Socle 1 (A2) — passkey obligatoire : session « enrôlement seulement », aucune
/// donnée accessible avant l'enrôlement, mot de passe non redemandé pendant
/// l'enrôlement, session élevée ensuite ; la connexion suivante exige la passkey.
#[tokio::test]
async fn socle1_passkey_obligatoire_session_d_enrolement_puis_complete() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip socle1: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let parent = inscrire(&app, etab, "parent").await;

    // Seule la direction fixe la politique ; rôle inconnu refusé.
    let (st, _) = appel(&app, "PUT", "/etablissement/securite", Some(&prof.jeton),
        Some(json!({"passkey_obligatoire_roles": ["prof"]}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "PUT", "/etablissement/securite", Some(&dir.jeton),
        Some(json!({"passkey_obligatoire_roles": ["inconnu"]}))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, p) = appel(&app, "PUT", "/etablissement/securite", Some(&dir.jeton),
        Some(json!({"passkey_obligatoire_roles": ["prof", "cpe"]}))).await;
    assert_eq!(st, StatusCode::OK, "{p}");
    assert_eq!(p["passkey_obligatoire_roles"], json!(["cpe", "prof"]));

    // Le professeur, sans passkey : session d'enrôlement.
    let (jeton, _) = connecter(&app, etab, &prof.email).await;
    let (st, me) = appel(&app, "GET", "/auth/me", Some(&jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(me["portee"], "enrolement");
    assert_eq!(me["passkey_obligatoire"], true);
    for uri in ["/eleves/search?q=a", "/messages/fils", "/auth/sessions", "/vs/absences"] {
        let (st, _) = appel(&app, "GET", uri, Some(&jeton), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{uri} fermé avant l'enrôlement");
    }
    // Variantes de chemin : jamais de succès (slash final, sous-ressource).
    for (m, uri) in [("GET", "/auth/passkeys/"), ("DELETE", &*format!("/auth/passkeys/{}", Uuid::new_v4())),
                     ("GET", "/auth/me/")] {
        let (st, _) = appel(&app, m, uri, Some(&jeton), None).await;
        assert!(!st.is_success(), "{m} {uri} : {st}");
    }
    // Session ouverte par mot de passe : le mot de passe est redemandé à
    // l'enrôlement (un jeton d'enrôlement volé ne suffit pas, revue E2).
    let (st, _) = enroler(&app, &jeton, None, "Sans mot de passe").await;
    assert_ne!(st, StatusCode::CREATED, "enrôlement sans mot de passe refusé");
    let (st, b) = enroler(&app, &jeton, Some(MDP), "Clé obligatoire").await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    let (_, me) = appel(&app, "GET", "/auth/me", Some(&jeton), None).await;
    assert_eq!(me["portee"], "complete");
    let (st, _) = appel(&app, "GET", "/auth/sessions", Some(&jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    // Prochaine connexion par mot de passe : 2ᵉ facteur exigé.
    let (_, corps) = appel(&app, "POST", "/auth/login", None,
        Some(json!({"email": prof.email, "password": MDP, "etablissement_id": etab}))).await;
    assert_eq!(corps["mfa_required"], json!(true));

    // Rôle non soumis : session complète.
    let (jeton_p, _) = connecter(&app, etab, &parent.email).await;
    let (_, me) = appel(&app, "GET", "/auth/me", Some(&jeton_p), None).await;
    assert_eq!(me["portee"], "complete");
    assert_eq!(me["passkey_obligatoire"], false);
}

/// Socle 1 (A3) — web : 2 h d'inactivité ; mobile : pas de délai d'inactivité,
/// prolongation à l'usage. Appareils connectés : liste et déconnexion ciblée.
#[tokio::test]
async fn socle1_sessions_web_inactivite_mobile_longue_et_appareils() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip socle1: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let c = inscrire(&app, etab, "prof").await;
    let autre = inscrire(&app, etab, "prof").await;
    let (st, b) = appel(&app, "POST", "/auth/login", None, Some(json!({
        "email": c.email, "password": MDP, "etablissement_id": etab, "client": "mobile"}))).await;
    assert_eq!(st, StatusCode::OK);
    let mobile = b["token"].as_str().unwrap().to_string();
    let (_, me) = appel(&app, "GET", "/auth/me", Some(&mobile), None).await;
    assert_eq!((me["client"].as_str(), me["assurance"].as_str()), (Some("mobile"), Some("mot_de_passe")));

    // Liste : 2 sessions (web d'inscription + mobile), la courante marquée.
    let (st, liste) = appel(&app, "GET", "/auth/sessions", Some(&c.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    let liste = liste.as_array().unwrap().clone();
    assert_eq!(liste.len(), 2, "{liste:?}");
    assert_eq!(liste.iter().filter(|s| s["courante"] == true).count(), 1);

    // 3 h sans activité : la session web expire, pas la mobile.
    // (L'échéance mobile est ramenée à J+1 : la prolongation à l'usage doit la repousser.)
    sqlx::query("UPDATE ent_session SET expires_at = now() + interval '1 day' WHERE user_id = $1 AND client = 'mobile'")
        .bind(c.id)
        .execute(&pool().await)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_session SET last_seen_at = now() - interval '3 hours' WHERE user_id = $1")
        .bind(c.id)
        .execute(&pool().await)
        .await
        .unwrap();
    assert!(!vivante(&app, &c.jeton).await, "web : inactivité > 2 h");
    assert!(vivante(&app, &mobile).await, "mobile : pas de délai d'inactivité");
    let (exp,): (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
        "SELECT expires_at FROM ent_session WHERE user_id = $1 AND client = 'mobile'")
        .bind(c.id)
        .fetch_one(&pool().await)
        .await
        .unwrap();
    assert!(exp > chrono::Utc::now() + chrono::Duration::days(80), "mobile prolongée à l'usage");

    // Déconnexion ciblée : on ne touche pas aux sessions d'autrui.
    let (_, liste) = appel(&app, "GET", "/auth/sessions", Some(&mobile), None).await;
    let id_mobile = liste[0]["id"].as_str().unwrap().to_string();
    let (st, _) = appel(&app, "DELETE", &format!("/auth/sessions/{id_mobile}"), Some(&autre.jeton), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND, "session d'un autre compte");
    assert!(vivante(&app, &mobile).await);
    let (st, _) = appel(&app, "DELETE", &format!("/auth/sessions/{id_mobile}"), Some(&mobile), None).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    assert!(!vivante(&app, &mobile).await);
}
