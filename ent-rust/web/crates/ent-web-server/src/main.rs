//! `ent-web-server` : ent-api + frontend Leptos sur la même origine (prototype).

use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Même réglage que `ent-api` : certaines chaînes async métier sont profondes
    // (seed labo) et `ui.rs` embarque une très grosse chaîne.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(16 * 1024 * 1024)
        .build()
        .expect("runtime tokio");

    rt.block_on(async {
        let (app, addr) = match ent_web_server::routeur_complet().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("démarrage impossible : {e}");
                std::process::exit(1);
            }
        };
        if !addr.ip().is_loopback() {
            eprintln!("refus : le prototype n'écoute que sur 127.0.0.1 (reçu {addr})");
            std::process::exit(1);
        }
        let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
        println!("prototype ENT : http://{addr}/web/appel");
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .expect("serveur");
    });
}
