//! Messagerie M12, côté HTTP (base locale) : `GET /messages/flux` et `POST /messages/{id}/annuler`. Les scénarios
//! métier (droits, curseur, troncature, retrait, transaction longue) sont dans `ent-db` (`tests_messagerie_m12`,
//! `tests_messagerie_revue`) ; ici : authentification (401), cloisonnement entre deux établissements, absence de
//! contenu, limite de débit et de concurrence (429 avec `Retry-After`), rappel d'une pagination tronquée, plafond du
//! compte, annulation d'un différé.
//!
//! Aucune attente réelle : le temps du flux est injecté (`AppState::flux_reglages`), l'intervalle minimal du limiteur
//! est fixé par test (`LimiteFlux::avec_intervalle`), et chaque test a **son** limiteur (aucun état global partagé).

use super::tests_l3::{appel, etat};
use super::*;
use crate::flux_limite::LimiteFlux;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use domain_eleve::OuvrirFil;
use ent_db::{OptionsEnvoiMessage, ReglagesFlux};
use http_body_util::BodyExt;
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

/// Appel complet : statut, en-têtes et corps en texte (les erreurs métier sont du texte, pas du JSON).
async fn appel_complet(
    app: &axum::Router,
    methode: &str,
    uri: &str,
    jeton: &str,
) -> (StatusCode, HeaderMap, String) {
    let req = Request::builder()
        .method(methode)
        .uri(uri)
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let (st, entetes) = (res.status(), res.headers().clone());
    let octets = res.into_body().collect().await.unwrap().to_bytes();
    (st, entetes, String::from_utf8_lossy(&octets).into_owned())
}

/// Nouvelle session d'un compte du labo (un appareil de plus) : jeton distinct.
async fn nouvelle_session(app: &axum::Router, email: &str, etab: Uuid) -> String {
    let (st, connexion) = appel(
        app,
        "POST",
        "/auth/login",
        None,
        Some(serde_json::json!({ "email": email, "password": ent_db::LABO_PASSWORD, "etablissement_id": etab })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "connexion : {connexion}");
    connexion["token"].as_str().expect("jeton").to_string()
}

/// Connexion directe à la base locale (vérifications et mises en scène hors service).
async fn pool() -> sqlx::PgPool {
    sqlx::PgPool::connect(&super::tests_l3::url_locale().expect("ENT_DATABASE_URL"))
        .await
        .expect("base locale")
}

async fn ouvrir_fil(ent: &PgEnt, c: &ent_db::LaboCast, sujet: &str) -> ent_db::FilOuvert {
    let (d, r) = (c.direction.user_id, c.direction.role.as_str().to_string());
    let mut v = ent
        .ouvrir_fils_for_actor(
            &r,
            OuvrirFil {
                etablissement_id: c.etablissement_id,
                eleve_id: c.eleve_lea_id,
                sujet: sujet.into(),
                premier_message: "Corps-confidentiel-du-flux".into(),
                actor_user_id: d,
            },
            OptionsEnvoiMessage::default(),
        )
        .await
        .expect("ouverture");
    assert_eq!(v.len(), 1);
    v.remove(0)
}

#[tokio::test]
async fn flux_http_authentifie_cloisonne_sans_contenu_et_borne() {
    let Some((mut state, ent)) = etat().await else {
        eprintln!("skip M12 HTTP: ENT_DATABASE_URL");
        return;
    };
    // Cet état a son limiteur (sans intervalle minimal : plusieurs appels de suite, aucune horloge) et son temps.
    state.flux_limite = LimiteFlux::avec_intervalle(Duration::ZERO);
    let app = app(state);
    // Deux établissements : le flux de l'un ne dit rien de l'autre.
    let a = ent.seed_labo_cast().await.expect("établissement A");
    let b = ent.seed_labo_cast().await.expect("établissement B");
    assert_ne!(a.etablissement_id, b.etablissement_id);

    // Sans jeton : 401.
    let (st, _) = appel(&app, "GET", "/messages/flux", None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // Départ : un curseur, aucun événement (famille de A).
    let (st, depart) = appel(
        &app,
        "GET",
        "/messages/flux",
        Some(&a.parent_lea.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{depart}");
    assert!(depart["fils"].as_array().unwrap().is_empty());
    let curseur = depart["curseur"].as_str().expect("curseur").to_string();
    assert!(
        curseur.starts_with("v3."),
        "curseur opaque, format v2 : {curseur}"
    );

    // Les directions de A et de B écrivent à la famille de Léa de leur établissement.
    let fil_a = ouvrir_fil(&ent, &a, "Flux HTTP A").await.fil.id;
    let fil_b = ouvrir_fil(&ent, &b, "Flux HTTP B").await.fil.id;

    // Deux appareils (deux sessions) du même compte reçoivent **tous deux** l'événement (M-D).
    let jeton2 = nouvelle_session(&app, &a.parent_lea.email, a.etablissement_id).await;
    assert_ne!(jeton2, a.parent_lea.token, "deux sessions distinctes");
    let url = format!("/messages/flux?apres={curseur}");
    let mut flux_a = serde_json::Value::Null;
    for jeton in [&a.parent_lea.token, &jeton2] {
        let (st, flux) = appel(&app, "GET", &url, Some(jeton), None).await;
        assert_eq!(
            st,
            StatusCode::OK,
            "chaque appareil reçoit le temps réel : {flux}"
        );
        // Le compte « famille de Léa » du labo est partagé avec les autres tests : d'autres fils peuvent apparaître. On
        // exige le fil de ce test (genre « nouveau ») et l'absence du fil de B, pas un nombre exact d'événements.
        let fils = flux["fils"].as_array().unwrap();
        let du_test: Vec<_> = fils
            .iter()
            .filter(|f| f["fil_id"] == fil_a.to_string())
            .collect();
        assert_eq!(du_test.len(), 1, "{flux}");
        assert_eq!(du_test[0]["genre"], "nouveau");
        assert!(
            fils.iter().all(|f| f["fil_id"] != fil_b.to_string()),
            "le fil de B ne fuit pas vers A : {flux}"
        );
        flux_a = flux;
    }
    let texte = flux_a.to_string();
    assert!(
        !texte.contains("confidentiel") && !texte.contains("Flux HTTP"),
        "aucun contenu : {texte}"
    );

    // Rappel avec le curseur rendu : plus rien (aucun événement servi deux fois), même route, même session.
    let suivant = flux_a["curseur"].as_str().unwrap().to_string();
    let (st, rappel) = appel(
        &app,
        "GET",
        &format!("/messages/flux?apres={suivant}"),
        Some(&a.parent_lea.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{rappel}");
    assert!(
        rappel["fils"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["fil_id"] != fil_a.to_string()),
        "pas de répétition : {rappel}"
    );
    // Un curseur de l'ancien format (instant) : resynchronisation, rien d'autre.
    let (st, ancien) = appel(
        &app,
        "GET",
        "/messages/flux?apres=2026-11-22T10:00:00Z",
        Some(&a.parent_lea.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{ancien}");
    assert_eq!(ancien["resynchro"], true);
    assert!(ancien["fils"].as_array().unwrap().is_empty());

    // Cloisonnement : la famille de B, avec le même curseur, ne reçoit que le fil de **son** établissement.
    let (st, flux_b) = appel(&app, "GET", &url, Some(&b.parent_lea.token), None).await;
    assert_eq!(st, StatusCode::OK, "{flux_b}");
    let ids_b: Vec<String> = flux_b["fils"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["fil_id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        !ids_b.contains(&fil_a.to_string()),
        "le fil de A ne fuit pas vers B : {flux_b}"
    );
}

#[tokio::test]
async fn flux_http_retry_after_concurrence_pagination_et_plafond_du_compte() {
    let Some((mut state, ent)) = etat().await else {
        eprintln!("skip M12 HTTP: ENT_DATABASE_URL");
        return;
    };
    // Intervalle minimal d'une heure : « trop vite » est un fait du test, pas de l'horloge. Une « seconde » du flux
    // dure une heure et la troncature est à un fil : un appel patient ne rend jamais la main de lui-même (on l'abandonne),
    // un appel sans attente rend aussitôt.
    state.flux_limite = LimiteFlux::avec_intervalle(Duration::from_secs(3600));
    state.flux_reglages = ReglagesFlux {
        seconde: Duration::from_secs(3600),
        fils_max: 1,
    };
    let limite = state.flux_limite.clone();
    let app = app(state);
    let a = ent.seed_labo_cast().await.expect("établissement A");
    let (etab, lily, email) = (
        a.etablissement_id,
        a.parent_lea.user_id,
        a.parent_lea.email.clone(),
    );

    // « Trop vite » : second appel immédiat d'une même session → 429 et `Retry-After` = l'intervalle.
    let s0 = nouvelle_session(&app, &email, etab).await;
    let (st, _, corps) = appel_complet(&app, "GET", "/messages/flux", &s0).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let (st, entetes, _) = appel_complet(&app, "GET", "/messages/flux", &s0).await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        entetes.get("retry-after").and_then(|v| v.to_str().ok()),
        Some("3600"),
        "Retry-After (trop vite)"
    );

    // Deux événements pour la famille, puis une session qui lit avec une limite d'un fil : réponse tronquée.
    let curseur0 = {
        let (_, _, corps) = appel_complet(
            &app,
            "GET",
            "/messages/flux",
            &nouvelle_session(&app, &email, etab).await,
        )
        .await;
        serde_json::from_str::<serde_json::Value>(&corps).unwrap()["curseur"]
            .as_str()
            .unwrap()
            .to_string()
    };
    ouvrir_fil(&ent, &a, "Pagination 1").await;
    ouvrir_fil(&ent, &a, "Pagination 2").await;
    let pagine = nouvelle_session(&app, &email, etab).await;
    let (st, _, corps) = appel_complet(
        &app,
        "GET",
        &format!("/messages/flux?apres={curseur0}"),
        &pagine,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let premiere: serde_json::Value = serde_json::from_str(&corps).unwrap();
    assert_eq!(
        premiere["tronque"], true,
        "précondition : réponse tronquée à un fil : {corps}"
    );
    let suite = premiere["curseur"].as_str().unwrap().to_string();

    // Curseur courant (rien de nouveau après lui) pour les appels qui doivent patienter.
    let attente_url = {
        let (_, _, corps) = appel_complet(
            &app,
            "GET",
            "/messages/flux",
            &nouvelle_session(&app, &email, etab).await,
        )
        .await;
        let c = serde_json::from_str::<serde_json::Value>(&corps).unwrap()["curseur"]
            .as_str()
            .unwrap()
            .to_string();
        format!("/messages/flux?apres={c}&attente=25")
    };
    // Huit appels longs en cours (huit appareils) : le plafond du compte est atteint.
    let mut longs = Vec::new();
    for _ in 0..crate::flux_limite::FLUX_APPELS_MAX_PAR_COMPTE {
        let jeton = nouvelle_session(&app, &email, etab).await;
        let (app, url) = (app.clone(), attente_url.clone());
        longs.push(tokio::spawn(async move {
            appel_complet(&app, "GET", &url, &jeton).await
        }));
    }
    tokio::time::timeout(Duration::from_secs(30), async {
        while limite.appels_en_cours(lily) < crate::flux_limite::FLUX_APPELS_MAX_PAR_COMPTE {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("les huit appels longs ont pris leur place");

    // Une session sans pagination en cours est refusée au plafond, avec `Retry-After`.
    let neuve = nouvelle_session(&app, &email, etab).await;
    let (st, entetes, _) = appel_complet(&app, "GET", "/messages/flux", &neuve).await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS, "plafond du compte");
    assert_eq!(
        entetes.get("retry-after").and_then(|v| v.to_str().ok()),
        Some("2"),
        "Retry-After (concurrence)"
    );
    // M-2 : un curseur fabriqué ne bénéficie PAS du rappel libre, même sur la session dont la dernière réponse était
    // tronquée : ni l'intervalle ni le plafond du compte ne sont contournés (v3.0.-.-.0.-.<emis>.<cluster>).
    let forge = "v3.0.-.-.0.-.4102444800000000.0";
    let (st, _, _) =
        appel_complet(&app, "GET", &format!("/messages/flux?apres={forge}"), &pagine).await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS, "curseur fabriqué : pas de rappel libre");
    // Mais le rappel d'une pagination tronquée n'est **jamais** refusé : ni l'intervalle (une heure), ni le plafond.
    let (st, _, corps) = appel_complet(
        &app,
        "GET",
        &format!("/messages/flux?apres={suite}"),
        &pagine,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "rappel après `tronque` : jamais refusé, plafond compris : {corps}"
    );

    // Les appels abandonnés libèrent leur place (annulation de la requête comprise).
    for l in &longs {
        l.abort();
    }
    tokio::time::timeout(Duration::from_secs(30), async {
        while limite.appels_en_cours(lily) > 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("places libérées à l'annulation");
}

#[tokio::test]
async fn flux_http_appel_patient_aboutit_avec_le_temps_injecte() {
    let Some((mut state, ent)) = etat().await else {
        eprintln!("skip M12 HTTP: ENT_DATABASE_URL");
        return;
    };
    // `attente=25` dure 25 ms : aucune attente de 25 s ; l'appel aboutit avec un flux vide et un curseur.
    state.flux_limite = LimiteFlux::avec_intervalle(Duration::ZERO);
    state.flux_reglages = ReglagesFlux {
        seconde: Duration::from_millis(1),
        fils_max: 200,
    };
    let app = app(state);
    let a = ent.seed_labo_cast().await.expect("établissement A");
    let (_, _, corps) = appel_complet(&app, "GET", "/messages/flux", &a.cpe.token).await;
    let curseur = serde_json::from_str::<serde_json::Value>(&corps).unwrap()["curseur"]
        .as_str()
        .unwrap()
        .to_string();
    let debut = std::time::Instant::now();
    let (st, _, corps) = appel_complet(
        &app,
        "GET",
        &format!("/messages/flux?apres={curseur}&attente=25"),
        &a.cpe.token,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let flux: serde_json::Value = serde_json::from_str(&corps).unwrap();
    assert!(flux["fils"].as_array().unwrap().is_empty());
    assert!(flux["curseur"].as_str().unwrap().starts_with("v3."));
    assert!(
        debut.elapsed() < Duration::from_secs(20),
        "le temps est injecté : pas d'attente réelle de 25 s"
    );
}

#[tokio::test]
async fn annulation_http_corriger_puis_annuler_refus_apres_remise_et_inconnu() {
    let Some((mut state, ent)) = etat().await else {
        eprintln!("skip M12 HTTP: ENT_DATABASE_URL");
        return;
    };
    state.flux_limite = LimiteFlux::avec_intervalle(Duration::ZERO);
    let app = app(state);
    let a = ent.seed_labo_cast().await.expect("établissement A");
    let (etab, dir) = (a.etablissement_id, a.direction.user_id);
    let role_dir = a.direction.role.as_str().to_string();
    let pool = pool().await;
    let differe = |id: Uuid, expr: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query(&format!(
                "UPDATE ent_msg SET remise_prevue_le = {expr} WHERE id = $1"
            ))
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        }
    };

    // 1. Corriger puis annuler : 204, et plus rien (message, version).
    let ouvert = ouvrir_fil(&ent, &a, "Hésitation").await;
    let m = ouvert.message.id;
    differe(m, "now() + interval '2 hours'").await;
    ent.corriger_message_fil_for_actor(etab, dir, &role_dir, m, "Corrigé")
        .await
        .expect("correction du différé");
    let (st, _, corps) = appel_complet(
        &app,
        "POST",
        &format!("/messages/{m}/annuler"),
        &a.direction.token,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::NO_CONTENT,
        "corriger puis annuler : {corps}"
    );
    let (messages, versions): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ent_msg WHERE id = $1), (SELECT count(*) FROM ent_msg_version WHERE message_id = $1)",
    )
    .bind(m)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((messages, versions), (0, 0), "annulation totale");

    // 2. Après la remise : 409 explicite « déjà remis : retire-le », le message reste.
    let remis = ouvrir_fil(&ent, &a, "Parti").await.message.id;
    differe(remis, "now() - interval '1 second'").await;
    let (st, _, corps) = appel_complet(
        &app,
        "POST",
        &format!("/messages/{remis}/annuler"),
        &a.direction.token,
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "{corps}");
    assert!(
        corps.contains("déjà remis : retire-le"),
        "refus explicite : {corps}"
    );
    let reste: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_msg WHERE id = $1")
        .bind(remis)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(reste, 1);

    // 3. Identifiant inconnu : 404 (jamais une erreur technique opaque).
    let (st, _, corps) = appel_complet(
        &app,
        "POST",
        &format!("/messages/{}/annuler", Uuid::new_v4()),
        &a.direction.token,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "{corps}");
}
