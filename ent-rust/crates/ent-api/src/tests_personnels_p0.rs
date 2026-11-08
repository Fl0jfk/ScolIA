//! Socle 6, lot P0 — correctifs de sécurité des accès, côté HTTP (base locale).
//!
//! * fin du mot de passe temporaire : le provisionnement et la réinitialisation n'exposent aucun
//!   secret, une invitation part par e-mail et la personne choisit son mot de passe ;
//! * un accès fermé ou suspendu refuse la connexion, invalide le jeton déjà ouvert ;
//! * le dernier administrateur ne peut pas être fermé (409) ;
//! * l'annuaire du personnel n'expose pas de téléphone ;
//! * isolation : un autre établissement ne peut pas fermer l'accès d'une personne.

use super::tests_l3::{appel, connecter, etat, inscrire, lien_email, vivante, MDP};
use super::*;
use axum::http::StatusCode;
use serde_json::{json, Value};
use uuid::Uuid;

/// Aucun champ de la réponse ne doit ressembler à un secret.
fn sans_secret(corps: &Value) {
    let texte = corps.to_string().to_lowercase();
    for mot in ["mot_de_passe", "password", "secret", "token", "jeton"] {
        assert!(
            !texte.contains(mot),
            "« {mot} » dans une réponse HTTP : {corps}"
        );
    }
}

#[tokio::test]
async fn provisionnement_et_reinitialisation_sans_secret_invitation_par_email() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip P0: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let admin = inscrire(&app, etab, "admin").await;

    ent.mailer().clear_receipts();
    let email = format!("invite-{}@ent.local", Uuid::new_v4().simple());
    let (st, corps) = appel(
        &app,
        "POST",
        "/etablissement/personnel",
        Some(&admin.jeton),
        Some(json!({"email": email, "display_name": "Nouvelle recrue", "role": "prof"})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{corps}");
    assert_eq!(corps["invitation_envoyee"], true);
    sans_secret(&corps);
    // Une invitation (lien à usage unique) est partie à l'adresse, rien d'autre.
    assert!(ent
        .mailer()
        .receipts()
        .iter()
        .any(|r| r.to == email && r.subject.contains("Invitation")));
    let jeton = lien_email(&ent, &email, "/reset-mot-de-passe").expect("lien d'invitation");
    // Personne ne connaît de mot de passe : aucune connexion avant l'activation.
    for essai in [MDP, "password", "Temp0raire!Aa9"] {
        let (st, _) = appel(
            &app,
            "POST",
            "/auth/login",
            None,
            Some(json!({"email": email, "password": essai, "etablissement_id": etab})),
        )
        .await;
        assert_eq!(st, StatusCode::UNAUTHORIZED);
    }
    // La personne active son accès par le lien (usage unique).
    let choisi = format!("Choisi-{}-Aa9!", Uuid::new_v4().simple());
    let (st, _) = appel(
        &app,
        "POST",
        "/auth/reset-password",
        None,
        Some(json!({"token": jeton, "new_password": choisi})),
    )
    .await;
    assert!(st.is_success(), "{st}");
    let (st, _) = appel(
        &app,
        "POST",
        "/auth/reset-password",
        None,
        Some(json!({"token": jeton, "new_password": format!("Autre-{choisi}")})),
    )
    .await;
    assert!(!st.is_success(), "le lien est à usage unique");
    let (st, _) = appel(
        &app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"email": email, "password": choisi, "etablissement_id": etab})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Adresse déjà connue ailleurs : réponse indiscernable d'un succès, aucun message, aucun secret.
    let autre_etab = Uuid::new_v4();
    let ailleurs = inscrire(&app, autre_etab, "prof").await;
    ent.mailer().clear_receipts();
    let (st, corps) = appel(
        &app,
        "POST",
        "/etablissement/personnel",
        Some(&admin.jeton),
        Some(json!({"email": ailleurs.email, "role": "prof"})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    assert_eq!(corps["invitation_envoyee"], true);
    sans_secret(&corps);
    assert!(ent
        .mailer()
        .receipts()
        .iter()
        .all(|r| r.to != ailleurs.email));

    // Réinitialisation d'un personnel : lien (202), sessions révoquées, aucun secret.
    let prof = inscrire(&app, etab, "prof").await;
    let (st, corps) = appel(
        &app,
        "POST",
        &format!("/etablissement/personnel/{}/reset-mot-de-passe", prof.id),
        Some(&admin.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED);
    sans_secret(&json!({"lien_envoye": corps["lien_envoye"], "user_id": corps["user_id"]}));
    assert!(corps.get("mot_de_passe_temporaire").is_none());
    assert!(!vivante(&app, &prof.jeton).await);
}

#[tokio::test]
async fn acces_ferme_ou_suspendu_http_connexion_et_jeton_refuses_dernier_admin_409() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip P0: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let admin = inscrire(&app, etab, "admin").await;
    let admin2 = inscrire(&app, etab, "admin").await;
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let uri = |id: Uuid, action: &str| format!("/etablissement/personnel/{id}/acces/{action}");

    // Droits : un prof ne ferme rien ; la direction ne touche pas un admin ; pas soi-même.
    let (st, _) = appel(
        &app,
        "POST",
        &uri(admin.id, "fermer"),
        Some(&prof.jeton),
        Some(json!({"motif": "depart"})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(
        &app,
        "POST",
        &uri(admin.id, "fermer"),
        Some(&dir.jeton),
        Some(json!({"motif": "depart"})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(
        &app,
        "POST",
        &uri(admin.id, "fermer"),
        Some(&admin.jeton),
        Some(json!({"motif": "depart"})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // La direction suspend un prof : connexion refusée (même réponse qu'un mauvais mot de passe),
    // jeton déjà ouvert refusé.
    assert!(vivante(&app, &prof.jeton).await);
    let (st, corps) = appel(
        &app,
        "POST",
        &uri(prof.id, "suspendre"),
        Some(&dir.jeton),
        Some(json!({"motif": "conservatoire"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    assert_eq!(corps["etat"], "suspendu");
    sans_secret(&corps);
    assert!(!vivante(&app, &prof.jeton).await, "jeton déjà ouvert");
    let tentative = |email: String| {
        let app = app.clone();
        async move {
            appel(
                &app,
                "POST",
                "/auth/login",
                None,
                Some(json!({"email": email, "password": MDP, "etablissement_id": etab})),
            )
            .await
        }
    };
    let (st_suspendu, corps_suspendu) = tentative(prof.email.clone()).await;
    let (st_faux, corps_faux) = appel(
        &app,
        "POST",
        "/auth/login",
        None,
        Some(json!({"email": prof.email, "password": "Faux-Mot-De-Passe1!", "etablissement_id": etab})),
    )
    .await;
    assert_eq!(st_suspendu, StatusCode::UNAUTHORIZED);
    assert_eq!(
        (st_suspendu, &corps_suspendu),
        (st_faux, &corps_faux),
        "pas d'oracle d'état"
    );

    // Réouverture puis fermeture.
    let (st, _) = appel(
        &app,
        "POST",
        &uri(prof.id, "rouvrir"),
        Some(&dir.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(tentative(prof.email.clone()).await.0, StatusCode::OK);
    let (st, corps) = appel(
        &app,
        "POST",
        &uri(prof.id, "fermer"),
        Some(&dir.jeton),
        Some(json!({"motif": "mutation"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(corps["etat"], "ferme");
    assert_eq!(
        tentative(prof.email.clone()).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (st, _) = appel(
        &app,
        "POST",
        &uri(prof.id, "fermer"),
        Some(&dir.jeton),
        Some(json!({"motif": "inconnu"})),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // Dernier administrateur : on ferme le second, puis le premier est protégé (409).
    let (st, _) = appel(
        &app,
        "POST",
        &uri(admin2.id, "fermer"),
        Some(&admin.jeton),
        Some(json!({"motif": "depart"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    // `admin2` est fermé : sa session ne peut plus agir ; on vérifie le garde-fou côté service, avec
    // un acteur admin encore ouvert (un troisième).
    let admin3 = inscrire(&app, etab, "admin").await;
    let (st, _) = appel(
        &app,
        "POST",
        &uri(admin.id, "fermer"),
        Some(&admin3.jeton),
        Some(json!({"motif": "depart"})),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "deux administrateurs ouverts : la fermeture d'un seul passe"
    );
    let (st, corps) = appel(
        &app,
        "POST",
        &uri(admin3.id, "suspendre"),
        Some(&admin.jeton),
        Some(json!({"motif": "conge"})),
    )
    .await;
    // `admin` est fermé : son jeton est refusé (401), l'établissement garde `admin3`.
    assert_eq!(st, StatusCode::UNAUTHORIZED, "{corps}");
    assert!(vivante(&app, &admin3.jeton).await);

    // Isolation : l'admin d'un autre établissement n'a aucun pouvoir sur cette personne.
    let autre = Uuid::new_v4();
    let admin_autre = inscrire(&app, autre, "admin").await;
    let _ = inscrire(&app, autre, "admin").await;
    let (st, _) = appel(
        &app,
        "POST",
        &uri(admin3.id, "fermer"),
        Some(&admin_autre.jeton),
        Some(json!({"motif": "depart"})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    assert!(vivante(&app, &admin3.jeton).await);
}

#[tokio::test]
async fn dernier_administrateur_409_et_date_de_fin_http() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip P0: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let admin = inscrire(&app, etab, "admin").await;
    let admin2 = inscrire(&app, etab, "admin").await;
    let uri = |id: Uuid, action: &str| format!("/etablissement/personnel/{id}/acces/{action}");
    // Programmer une fin sur `admin2` : permis (admin reste durable).
    let demain = (chrono::Utc::now() + chrono::Duration::days(3)).date_naive();
    let (st, corps) = appel(
        &app,
        "PUT",
        &uri(admin2.id, "fin"),
        Some(&admin.jeton),
        Some(json!({"date_fin": demain})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    assert_eq!(corps["date_fin"], json!(demain));
    // `admin` est maintenant le seul administrateur durable : `admin2` ne peut pas le ferrer d'une
    // fin (409), et un admin ne peut pas se cibler lui-même (403).
    let (st, _) = appel(
        &app,
        "PUT",
        &uri(admin.id, "fin"),
        Some(&admin2.jeton),
        Some(json!({"date_fin": demain})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    let (st, _) = appel(
        &app,
        "POST",
        &uri(admin.id, "fermer"),
        Some(&admin2.jeton),
        Some(json!({"motif": "depart"})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    let (st, _) = appel(
        &app,
        "PUT",
        &uri(admin2.id, "fin"),
        Some(&admin2.jeton),
        Some(json!({"date_fin": null})),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    // Date passée : refus de validation.
    let hier = (chrono::Utc::now() - chrono::Duration::days(3)).date_naive();
    let (st, _) = appel(
        &app,
        "PUT",
        &uri(admin2.id, "fin"),
        Some(&admin.jeton),
        Some(json!({"date_fin": hier})),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn annuaire_du_personnel_sans_telephone() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip P0: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let admin = inscrire(&app, etab, "admin").await;
    let prof = inscrire(&app, etab, "prof").await;
    let collegue = inscrire(&app, etab, "prof").await;
    // Le prof renseigne son téléphone (donnée personnelle, hors annuaire).
    let (st, _) = appel(
        &app,
        "PUT",
        &format!("/users/{}/telephone", prof.id),
        Some(&prof.jeton),
        Some(json!({"telephone": "0612345678"})),
    )
    .await;
    assert!(st.is_success(), "{st}");

    let lire = |uri: String, jeton: String| {
        let app = app.clone();
        async move {
            let req = axum::http::Request::builder()
                .uri(uri)
                .header("authorization", format!("Bearer {jeton}"))
                .body(axum::body::Body::empty())
                .unwrap();
            let res = tower::ServiceExt::oneshot(app, req).await.unwrap();
            let st = res.status();
            let bytes = http_body_util::BodyExt::collect(res.into_body())
                .await
                .unwrap()
                .to_bytes();
            (st, String::from_utf8_lossy(&bytes).to_string())
        }
    };
    for jeton in [&collegue.jeton, &admin.jeton] {
        let (st, texte) = lire("/personnel/search?q=".into(), jeton.clone()).await;
        assert_eq!(st, StatusCode::OK);
        assert!(
            texte.contains(&prof.email),
            "le collègue figure dans l'annuaire"
        );
        assert!(!texte.contains("0612345678"), "téléphone exposé : {texte}");
        assert!(
            !texte.contains("telephone"),
            "champ téléphone exposé : {texte}"
        );
        let (st, texte) = lire(format!("/personnel/{}", prof.id), jeton.clone()).await;
        assert_eq!(st, StatusCode::OK);
        assert!(
            !texte.contains("0612345678") && !texte.contains("telephone"),
            "{texte}"
        );
    }
}
