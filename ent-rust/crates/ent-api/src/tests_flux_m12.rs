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

    // Limite de débit : un rappel immédiat du même compte (attente=0) est refusé (429) ; un autre compte n'est pas gêné.
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/messages/flux?apres={}", curseur.replace('+', "%2B")),
        Some(&a.parent_lea.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS, "martèlement attente=0");
    // Après l'intervalle minimal : la famille de A reçoit l'événement de son fil, sans contenu.
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    let (st, flux_a) = appel(
        &app,
        "GET",
        &format!("/messages/flux?apres={}", curseur.replace('+', "%2B")),
        Some(&a.parent_lea.token),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{flux_a}");
    let fils = flux_a["fils"].as_array().unwrap();
    assert_eq!(fils.len(), 1, "{flux_a}");
    assert_eq!(fils[0]["fil_id"], fil_a.to_string());
    assert_eq!(fils[0]["genre"], "nouveau");
    let texte = flux_a.to_string();
    assert!(
        !texte.contains("confidentiel") && !texte.contains("Flux HTTP"),
        "aucun contenu : {texte}"
    );

    // Cloisonnement : la famille de B, avec le même curseur, ne reçoit que le fil de **son** établissement.
    let (st, flux_b) = appel(
        &app,
        "GET",
        &format!("/messages/flux?apres={}", curseur.replace('+', "%2B")),
        Some(&b.parent_lea.token),
        None,
    )
    .await;
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

    // Concurrence : un appel en attente (long) occupe la place du compte ; un second appel est refusé (429).
    let long = {
        let app = app.clone();
        let jeton = a.cpe.token.clone();
        tokio::spawn(async move {
            appel(&app, "GET", "/messages/flux?attente=3", Some(&jeton), None).await
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let (st, _) = appel(
        &app,
        "GET",
        "/messages/flux?attente=3",
        Some(&a.cpe.token),
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::TOO_MANY_REQUESTS,
        "un seul appel à la fois par compte"
    );
    let (st, corps) = long.await.unwrap();
    assert_eq!(st, StatusCode::OK, "le premier appel aboutit : {corps}");
}
