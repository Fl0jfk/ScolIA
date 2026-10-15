use ent_api::{app, boot_from_env};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let (state, bind) = boot_from_env().await.unwrap_or_else(|e| {
        eprintln!("ent-api boot failed: {e}");
        std::process::exit(1);
    });

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .unwrap_or_else(|e| {
            eprintln!("bind {bind}: {e}");
            std::process::exit(1);
        });
    tracing::info!("ent-api listening on http://{bind} (local only)");
    axum::serve(listener, app(state)).await.expect("serve");
}
