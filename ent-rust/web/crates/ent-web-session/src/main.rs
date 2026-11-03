//! Serveur du **prototype isolé** de session par cookie (D-016) : `ent-api` existant
//! (inchangé) + passerelle de session + page de démonstration `/web/_session`.
//!
//! Base PostgreSQL **locale** de labo uniquement (`ENT_DATABASE_URL`, refus d'ent-db sinon).
//! Ouvrir `http://localhost:8095/web/_session` (nom `localhost` : exigé par les passkeys,
//! dont le domaine par défaut est `localhost`).
//!
//! Variables (toutes facultatives) :
//! - `PROTO_SESSION_ADDR` (défaut `127.0.0.1:8095`) ;
//! - `PROTO_SESSION_ORIGINES` (défaut `http://localhost:8095`, liste séparée par des virgules) ;
//! - `PROTO_SESSION_MAX_S` / `PROTO_SESSION_INACTIVITE_S` (défauts 43 200 s = 12 h, 7 200 s = 2 h).

use std::net::SocketAddr;
use std::time::Duration;

fn secondes(nom: &str, defaut: u64) -> Duration {
    Duration::from_secs(
        std::env::var(nom)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaut),
    )
}

#[tokio::main]
async fn main() {
    let (state, _) = ent_api::boot_from_env().await.expect("démarrage ent-api");
    let api = ent_api::app(state);
    let addr: SocketAddr = std::env::var("PROTO_SESSION_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8095".into())
        .parse()
        .expect("PROTO_SESSION_ADDR");
    assert!(
        addr.ip().is_loopback(),
        "prototype : écoute locale uniquement"
    );
    let config = ent_web_session::Config {
        origines: std::env::var("PROTO_SESSION_ORIGINES")
            .unwrap_or_else(|_| format!("http://localhost:{}", addr.port()))
            .split(',')
            .map(|s| s.trim().to_string())
            .collect(),
        secure: true,
        duree_max: secondes("PROTO_SESSION_MAX_S", 12 * 3600),
        inactivite: secondes("PROTO_SESSION_INACTIVITE_S", 2 * 3600),
    };
    println!(
        "prototype session cookie : http://localhost:{}/web/_session",
        addr.port()
    );
    println!("origines autorisées : {:?}", config.origines);
    let app = ent_web_session::passerelle(api.clone(), config)
        .merge(ent_web_session::page_demo())
        .merge(api);
    let listener = tokio::net::TcpListener::bind(addr).await.expect("écoute");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("serveur");
}
