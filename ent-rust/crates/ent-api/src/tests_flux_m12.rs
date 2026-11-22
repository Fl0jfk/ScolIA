//! Messagerie M12, côté HTTP (base locale) : `GET /messages/flux`. Les scénarios métier (droits, curseur, troncature,
//! retrait) sont dans `ent-db` (`tests_messagerie_m12`, `tests_messagerie_revue`) ; ici : authentification (401),
//! cloisonnement entre deux établissements, absence de contenu, limite de débit et de concurrence (429).

use super::tests_l3::{appel, etat};
use super::*;
use axum::http::StatusCode;
use domain_eleve::OuvrirFil;
use ent_db::OptionsEnvoiMessage;
use uuid::Uuid;

#[tokio::test]
async fn flux_http_authentifie_cloisonne_sans_contenu_et_borne() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip M12 HTTP: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    // Deux établissements : le flux de l'un ne dit rien de l'autre.
    let a = ent.seed_labo_cast().await.expect("établissement A");
    let b = ent.seed_labo_cast().await.expect("établissement B");
    assert_ne!(a.etablissement_id, b.etablissement_id);
    let dir = |c: &ent_db::LaboCast| (c.direction.user_id, c.direction.role.as_str().to_string());

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

    // La direction de A écrit à la famille de Léa ; la direction de B écrit aussi dans son propre établissement.
    let mut ouverts = Vec::new();
    for (c, sujet) in [(&a, "Flux HTTP A"), (&b, "Flux HTTP B")] {
        let (d, r) = dir(c);
        let v = ent
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
        ouverts.push(v[0].fil.id);
    }
    let fil_a: Uuid = ouverts[0];

    // Limite par session : le limiteur du processus est partagé, on part d'un état vide (les cas de débit et de
    // pagination sont éprouvés avec une horloge injectée dans `flux_limite::tests`, sans attente réelle).
    crate::flux_limite::limite_flux().vider();
    // Deux appareils (deux sessions) du même compte reçoivent **tous deux** l'événement (M-D).
    let (st, connexion) = appel(
        &app,
        "POST",
        "/auth/login",
        None,
        Some(serde_json::json!({
            "email": a.parent_lea.email,
            "password": ent_db::LABO_PASSWORD,
            "etablissement_id": a.etablissement_id,
        })),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "seconde session du même compte : {connexion}"
    );
    let jeton2 = connexion["token"].as_str().expect("jeton").to_string();
    assert_ne!(jeton2, a.parent_lea.token, "deux sessions distinctes");
    let url = format!("/messages/flux?apres={}", curseur.replace('+', "%2B"));
    let mut flux_a = serde_json::Value::Null;
    for jeton in [&a.parent_lea.token, &jeton2] {
        let (st, flux) = appel(&app, "GET", &url, Some(jeton), None).await;
        assert_eq!(
            st,
            StatusCode::OK,
            "chaque appareil reçoit le temps réel : {flux}"
        );
        // Le compte « famille de Léa » du labo est partagé avec les autres tests qui tournent en parallèle sur la même
        // base : d'autres fils peuvent apparaître. On exige le fil de ce test (genre « nouveau ») et l'absence du fil de B,
        // pas un nombre exact d'événements.
        let fils = flux["fils"].as_array().unwrap();
        let du_test: Vec<_> = fils
            .iter()
            .filter(|f| f["fil_id"] == fil_a.to_string())
            .collect();
        assert_eq!(du_test.len(), 1, "{flux}");
        assert_eq!(du_test[0]["genre"], "nouveau");
        assert!(
            fils.iter().all(|f| f["fil_id"] != ouverts[1].to_string()),
            "le fil de B ne fuit pas vers A : {flux}"
        );
        flux_a = flux;
    }
    let texte = flux_a.to_string();
    assert!(
        !texte.contains("confidentiel") && !texte.contains("Flux HTTP"),
        "aucun contenu : {texte}"
    );

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

    // Concurrence (F-1) : un appel **réellement en attente** (curseur dans le futur : aucun événement, il patiente)
    // occupe la place de sa session ; le même jeton reçoit 429 `Concurrent` (et non `TropVite`, puisque `apres` est
    // fourni), pendant qu'une **autre session** du même compte, elle, est servie (M-D).
    let futur = (chrono::Utc::now() + chrono::Duration::hours(1))
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    let url_long = format!("/messages/flux?apres={futur}&attente=3");
    let (jeton_cpe, cpe) = (a.cpe.token.clone(), a.cpe.user_id);
    let long = {
        let (app, url_long, jeton) = (app.clone(), url_long.clone(), jeton_cpe.clone());
        tokio::spawn(async move { appel(&app, "GET", &url_long, Some(&jeton), None).await })
    };
    // Barrière (pas d'attente à durée fixe) : la place du compte est prise.
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while crate::flux_limite::limite_flux().appels_en_cours(cpe) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("l'appel long a pris sa place");
    let (st, _) = appel(&app, "GET", &url_long, Some(&jeton_cpe), None).await;
    assert_eq!(
        st,
        StatusCode::TOO_MANY_REQUESTS,
        "un seul appel à la fois par session"
    );
    let (st, connexion_cpe) = appel(
        &app,
        "POST",
        "/auth/login",
        None,
        Some(serde_json::json!({
            "email": a.cpe.email,
            "password": ent_db::LABO_PASSWORD,
            "etablissement_id": a.etablissement_id,
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{connexion_cpe}");
    let autre = connexion_cpe["token"].as_str().expect("jeton").to_string();
    let (st, corps) = appel(&app, "GET", "/messages/flux", Some(&autre), None).await;
    assert_eq!(
        st,
        StatusCode::OK,
        "une autre session du même compte est servie : {corps}"
    );
    let (st, corps) = long.await.unwrap();
    assert_eq!(st, StatusCode::OK, "le premier appel aboutit : {corps}");
}
