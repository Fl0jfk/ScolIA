//! Liens de changement d'adresse e-mail (RF-01) contre le **vrai** backend (`ent_api::app`) et
//! une base PostgreSQL **locale** de test. Les routes sont publiques : un jeton inconnu ou
//! malformé est refusé de la même façon, sans rien révéler. (Le parcours complet avec un vrai
//! jeton est couvert côté `ent-db` / `ent-api`, qui lisent la messagerie à blanc.)
//!
//! Lancement : comme `roles_backend.rs`. Sans `ENT_DATABASE_URL`, le test est ignoré.
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_web_api::{ApiClient, ApiError};

async fn demarrer() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent");
        return None;
    };
    let parsed = ent_db::assert_local_postgres_url(&url)
        .unwrap_or_else(|e| panic!("base de test locale uniquement : {e}"));
    let nom_base = parsed.path().trim_start_matches('/').to_ascii_lowercase();
    assert!(
        ["test", "proto", "lab"].iter().any(|m| nom_base.contains(m)),
        "le nom de la base doit contenir test, proto ou lab (refusé : {nom_base})"
    );
    let (app, _) = ent_web_server::routeur_complet()
        .await
        .expect("démarrage ent-api + web");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap();
    });
    Some(format!("http://{addr}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn liens_de_changement_d_email_contre_vrai_backend() {
    let Some(base) = demarrer().await else { return };
    let anonyme = ApiClient::new(&base, None);

    // Jeton bien formé mais inconnu : refus identique pour les deux étapes.
    let faux = "00".repeat(32);
    assert!(matches!(
        anonyme.confirmer_changement_email(&faux).await,
        Err(ApiError::Invalide(_))
    ));
    assert!(matches!(
        anonyme.annuler_changement_email(&faux).await,
        Err(ApiError::Invalide(_))
    ));
    // Jeton trop long ou vide : refusé avant tout examen.
    let trop_long = "ab".repeat(200);
    assert!(matches!(
        anonyme.confirmer_changement_email(&trop_long).await,
        Err(ApiError::Invalide(_))
    ));
    assert!(matches!(
        anonyme.annuler_changement_email("  ").await,
        Err(ApiError::Invalide(_))
    ));
}
