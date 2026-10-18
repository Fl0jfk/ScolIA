use ent_api::{app, boot_from_env};
use tracing_subscriber::EnvFilter;

/// Format des journaux : `text` (défaut, lisible en console) ou `json`
/// (`ENT_LOG_FORMAT=json`) pour un collecteur (Loki / Scaleway Cockpit).
fn format_journal_json() -> bool {
    matches!(
        std::env::var("ENT_LOG_FORMAT")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "json" | "structured" | "structure"
    )
}

fn main() {
    let filtre = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    if format_journal_json() {
        tracing_subscriber::fmt()
            .json()
            .flatten_event(true)
            .with_current_span(true)
            .with_span_list(false)
            .with_env_filter(filtre)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filtre).init();
    }

    // Certaines chaînes async métier sont très profondes (seed labo, scénarios
    // denses). Les workers tokio ont 2 Mio de pile par défaut : insuffisant en
    // build debug. On élargit la pile (surchargeable par `ENT_WORKER_STACK_MB`).
    let stack_mb = std::env::var("ENT_WORKER_STACK_MB")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|m| *m >= 2)
        .unwrap_or(16);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(stack_mb * 1024 * 1024)
        .build()
        .expect("tokio runtime");
    rt.block_on(run_api());
}

async fn run_api() {
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

    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serve");
    tracing::info!("ent-api arrêté proprement");
}

/// Attend Ctrl-C ou SIGTERM (arrêt gracieux demandé par l'orchestrateur).
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("signal d'arrêt reçu — fermeture gracieuse…");
}
