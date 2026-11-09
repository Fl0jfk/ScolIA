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

    // Socle 2 (S2-8) : SMS en mode réel (OVH, D-004) fail-closed hors labo — une
    // configuration absente ou invalide refuse le démarrage au lieu de retomber
    // silencieusement en mode simulé.
    let non_prod = std::env::var("ENT_ENV")
        .ok()
        .map(|e| e.trim().to_ascii_lowercase())
        .is_some_and(|v| ent_auth::ENVS_NON_PROD.contains(&v.as_str()));
    match ent_sms::SmsService::from_env_strict(non_prod) {
        Err(e) => {
            eprintln!("ent-api : {e}");
            std::process::exit(1);
        }
        Ok(sms) => {
            if let Some(e) = sms.erreur_config() {
                tracing::warn!(erreur = %e, "configuration SMS OVH invalide : SMS simulés");
            }
            tracing::info!(
                fournisseur = sms.fournisseur(),
                reel = sms.is_live(),
                "SMS : {}",
                if sms.is_live() { "envoi réel" } else { "mode simulé" }
            );
        }
    }

    // Socle 2 (S2-0) : distribution de l'outbox (réactions entre domaines) en
    // tâche de fond. Désactivable (`ENT_OUTBOX_DISTRIBUTEUR=0`) pour un
    // déploiement où un processus dédié s'en charge.
    let distributeur = distributeur_actif().then(|| {
        let d = std::sync::Arc::new(
            state
                .ent
                .distributeur(ent_db::outbox::ConfigDistribution::default()),
        );
        (d.clone(), d.lancer())
    });

    // Inscriptions (I0) : expédition des messages mis en file par les réactions, hors de
    // toute transaction métier (reprise et échecs tracés en base).
    let expediteur_inscriptions = distributeur_actif().then(|| {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = ent.expedier_envois_inscriptions(50).await {
                    tracing::warn!("expédition des envois d'inscription : {e}");
                }
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        })
    });
    // S2-5 (ADM-04) : balayage des délégations échues (enregistre l'expiration et publie
    // `delegation.expiree` une fois). Les droits, eux, s'éteignent par la date seule.
    {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = ent.delegations_balayer(domain_eleve::today_utc()).await {
                    tracing::warn!("balayage des délégations : {e}");
                }
                tokio::time::sleep(std::time::Duration::from_secs(600)).await;
            }
        });
    }
    // Socle 2 (S2-8) : expédition des notifications (e-mail, SMS, push) et rappels
    // de rendez-vous, en tâches de fond. Désactivables (`ENT_NOTIFICATIONS_EXPEDITEUR=0`).
    let expediteur = notifications_actives().then(|| {
        let x = std::sync::Arc::new(
            state
                .ent
                .expediteur(ent_db::notifications::ConfigExpedition::default()),
        );
        let rappels = {
            let ent = state.ent.clone();
            tokio::spawn(async move {
                loop {
                    if let Err(e) = ent
                        .emettre_rappels_rdv(
                            chrono::Utc::now(),
                            chrono::Duration::hours(24),
                            None,
                        )
                        .await
                    {
                        tracing::warn!(error = %e, "rappels de rendez-vous en échec");
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                }
            })
        };
        (x.clone(), x.lancer(), rappels)
    });

    // Socle 2 (E4) : tâche quotidienne de l'élève (statuts dérivés, majorité, identités
    // annoncées devenues effectives). Idempotente, lancée toutes les heures car « le jour »
    // dépend du fuseau de chaque établissement. Désactivable (`ENT_TACHES_ELEVES=0`).
    let taches_eleves = taches_eleves_actives().then(|| {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            loop {
                match ent.tache_nocturne_eleves().await {
                    Ok(b) => tracing::info!(?b, "tâche quotidienne des élèves"),
                    Err(e) => tracing::warn!("tâche quotidienne des élèves : {e}"),
                }
                // Lot E6 (ELV-22) : détection des doublons, une fois par jour et par établissement
                // (le passage du jour est mémorisé en base ; les appels suivants ne font rien).
                match ent.tache_doublons_eleves().await {
                    Ok(n) if n > 0 => tracing::info!(etablissements = n, "détection des doublons d'élèves"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("détection des doublons d'élèves : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        })
    });

    // `ConnectInfo<SocketAddr>` : adresse de la socket, seule IP non falsifiable
    // (base des gardes anti-abus ; voir `client_ip`).
    axum::serve(
        listener,
        app(state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serve");
    if let Some(t) = expediteur_inscriptions {
        t.abort();
    }
    if let Some(t) = taches_eleves {
        t.abort();
    }
    if let Some((d, tache)) = distributeur {
        d.arreter();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(10), tache).await;
    }
    if let Some((x, tache, rappels)) = expediteur {
        x.arreter();
        rappels.abort();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(10), tache).await;
    }
    tracing::info!("ent-api arrêté proprement");
}

fn notifications_actives() -> bool {
    !matches!(
        std::env::var("ENT_NOTIFICATIONS_EXPEDITEUR")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "0" | "false" | "non" | "off"
    )
}

fn taches_eleves_actives() -> bool {
    !matches!(
        std::env::var("ENT_TACHES_ELEVES")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "0" | "false" | "non" | "off"
    )
}

fn distributeur_actif() -> bool {
    !matches!(
        std::env::var("ENT_OUTBOX_DISTRIBUTEUR")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "0" | "false" | "non" | "off"
    )
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
