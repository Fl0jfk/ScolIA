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
    // Tâche de maintenance : `ent-api reprise-pieces …` (pas de serveur HTTP).
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("reprise-pieces") {
        let code = rt.block_on(reprise_pieces(&args[1..]));
        std::process::exit(code);
    }
    // Opérateur de la plateforme : établissements et domaines (B-02, pas de route HTTP).
    if args.first().map(String::as_str) == Some("plateforme-domaine") {
        let code = rt.block_on(plateforme_domaine(&args[1..]));
        std::process::exit(code);
    }
    rt.block_on(run_api());
}

const AIDE_PLATEFORME: &str = "\
ent-api plateforme-domaine <commande>
  Outil de l'opérateur ScolIA (accès direct à la base, ENT_DATABASE_URL) : un administrateur
  d'établissement ne peut ni ajouter ni retirer un domaine (audit Fable 1, B-02).

  lister                                    tous les domaines servis et leur établissement
  declarer <etablissement-uuid> <slug> <nom>  déclare un établissement ou change son slug/nom
  ajouter  <etablissement-uuid> <domaine>   rattache un domaine (hôtes réservés refusés,
                                            domaine d'un autre établissement refusé)
  retirer  <etablissement-uuid> <domaine>   retire un domaine
Code de sortie : 0 succès ; 1 erreur.
";

async fn plateforme_domaine(args: &[String]) -> i32 {
    let commande = args.first().map(String::as_str).unwrap_or("--aide");
    if matches!(commande, "--aide" | "--help" | "-h") {
        print!("{AIDE_PLATEFORME}");
        return 0;
    }
    let etab = |i: usize| args.get(i).and_then(|v| uuid::Uuid::parse_str(v).ok());
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        return erreur_plateforme("ENT_DATABASE_URL requis");
    };
    let ent = match ent_db::PgEnt::connect(&url).await {
        Ok(e) => e,
        Err(e) => return erreur_plateforme(&format!("connexion : {e}")),
    };
    let resultat: Result<String, String> = match (commande, args.len()) {
        ("lister", 1) => ent
            .lister_domaines_plateforme()
            .await
            .map(|l| {
                l.into_iter()
                    .map(|(d, e)| format!("{d}\t{e}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .map_err(|e| e.to_string()),
        ("declarer", 4) => match etab(1) {
            Some(id) => ent
                .declarer_etablissement_plateforme(id, &args[2], &args[3])
                .await
                .map(|i| format!("établissement {} déclaré : {} ({})", i.id, i.slug, i.nom))
                .map_err(|e| e.to_string()),
            None => Err("UUID d'établissement invalide".into()),
        },
        ("ajouter", 3) => match etab(1) {
            Some(id) => ent
                .ajouter_domaine_plateforme(id, &args[2])
                .await
                .map(|d| format!("domaine {d} rattaché à {id}"))
                .map_err(|e| e.to_string()),
            None => Err("UUID d'établissement invalide".into()),
        },
        ("retirer", 3) => match etab(1) {
            Some(id) => ent
                .retirer_domaine_plateforme(id, &args[2])
                .await
                .map_err(|e| e.to_string())
                .and_then(|retire| {
                    if retire {
                        Ok(format!("domaine retiré de {id}"))
                    } else {
                        Err("domaine non servi par cet établissement".into())
                    }
                }),
            None => Err("UUID d'établissement invalide".into()),
        },
        _ => Err("commande ou nombre d'arguments invalide (voir --aide)".into()),
    };
    match resultat {
        Ok(msg) => {
            if !msg.is_empty() {
                println!("{msg}");
            }
            0
        }
        Err(e) => erreur_plateforme(&e),
    }
}

fn erreur_plateforme(msg: &str) -> i32 {
    eprintln!("plateforme-domaine : {msg}");
    1
}

const AIDE_REPRISE: &str = "\
ent-api reprise-pieces [options]
  Sort les pièces historiques (BYTEA) de la messagerie et du PAI vers le magasin d'objets.
  Idempotente et reprenable : relancer après un arrêt ne refait que le reste.

  --simulation              ne rien écrire (ni objet, ni registre, ni ligne) ; contrôle tout
  --sauvegarde-verifiee     vider `contenu` après vérification (à n'utiliser qu'après un point
                            de sauvegarde vérifié) ; sans cela : copie vérifiée, BYTEA conservé
  --etablissement <uuid>    un seul établissement (défaut : tous, un par un)
  --table <messagerie|pai|toutes>   (défaut : toutes)
  --lot <n>                 taille d'un lot (défaut 50)
  --limite <n>              s'arrêter proprement après n pièces
Code de sortie : 0 tout repris / simulation sans échec ; 1 erreur ; 2 au moins une pièce en échec.
Les journaux et le rapport ne contiennent ni nom de fichier ni clé d'objet.
";

async fn reprise_pieces(args: &[String]) -> i32 {
    use ent_db::reprise_bytea::{OptionsReprise, TablePiece};
    let mut options = OptionsReprise::default();
    let mut etab: Option<uuid::Uuid> = None;
    let mut tables: Vec<TablePiece> = TablePiece::TOUTES.to_vec();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut valeur = || it.next().cloned();
        match a.as_str() {
            "--simulation" => options.simulation = true,
            "--sauvegarde-verifiee" => options.vider = true,
            "--etablissement" => match valeur().and_then(|v| uuid::Uuid::parse_str(&v).ok()) {
                Some(u) => etab = Some(u),
                None => return erreur_reprise("--etablissement attend un UUID"),
            },
            "--table" => match valeur().as_deref() {
                Some("toutes") => {}
                Some(v) => match TablePiece::parse(v) {
                    Some(t) => tables = vec![t],
                    None => return erreur_reprise("--table : messagerie, pai ou toutes"),
                },
                None => return erreur_reprise("--table attend une valeur"),
            },
            "--lot" => match valeur().and_then(|v| v.parse::<i64>().ok()).filter(|n| *n >= 1) {
                Some(n) => options.taille_lot = n,
                None => return erreur_reprise("--lot attend un entier >= 1"),
            },
            "--limite" => match valeur().and_then(|v| v.parse::<u64>().ok()) {
                Some(n) => options.limite_pieces = Some(n),
                None => return erreur_reprise("--limite attend un entier"),
            },
            "--aide" | "--help" | "-h" => {
                print!("{AIDE_REPRISE}");
                return 0;
            }
            _ => return erreur_reprise("option inconnue (voir --aide)"),
        }
    }
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        return erreur_reprise("ENT_DATABASE_URL requis");
    };
    let ent = match ent_db::PgEnt::connect(&url).await {
        Ok(e) => e,
        Err(e) => return erreur_reprise(&format!("connexion : {e}")),
    };
    let rapports = match ent.reprendre_pieces(&tables, etab, &options).await {
        Ok(r) => r,
        Err(e) => {
            return erreur_reprise(&format!(
                "reprise impossible (schéma à jour, migration 601 ? stockage joignable ?) : {e}"
            ))
        }
    };
    let mut echecs = 0;
    for r in &rapports {
        println!("{}", r.resume());
        echecs += r.en_echec.len();
    }
    let restantes: u64 = rapports.iter().map(|r| r.restantes).sum();
    println!(
        "Total : {} pièce(s) en échec, {} avec octets encore en base{}",
        echecs,
        restantes,
        if options.simulation { " (simulation : rien n'a été écrit)" } else { "" }
    );
    i32::from(echecs > 0) * 2
}

fn erreur_reprise(msg: &str) -> i32 {
    eprintln!("reprise-pieces : {msg}");
    1
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
    // Socle 5 (S3, STR-94) : exécution des levées programmées de fermetures exceptionnelles. Une
    // levée = un événement ; la levée programmée est effacée dans la même transaction, donc la
    // tâche est rejouable sans doublon de message.
    {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            loop {
                match ent.structure_fermetures_levees_tache().await {
                    Ok(n) if n > 0 => tracing::info!(fermetures = n, "levées programmées exécutées"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("levées programmées de fermetures : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });
    }
    // Socle 12 (AS4) : arrêt du décompte du mois terminé, au jour métier de chaque établissement. Idempotent :
    // un mois déjà arrêté n'est jamais réécrit, la tâche peut tourner à chaque passage.
    {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            loop {
                match ent.assiduite_arret_mois_tache().await {
                    Ok(n) if n > 0 => tracing::info!(mois = n, "mois d'assiduité arrêtés"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("arrêt du mois d'assiduité : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
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
                    // RD-05 : rendez-vous dont le responsable a perdu son lien avec l'enfant.
                    if let Err(e) = ent.reviser_rdv_liens_rompus(chrono::Utc::now(), None).await {
                        tracing::warn!(error = %e, "revue des rendez-vous à lien rompu en échec");
                    }
                    // D-058 : un cours rétabli dans la salle fait tomber la réservation.
                    if let Err(e) = ent
                        .agenda_reviser_reservations_edt(chrono::Utc::now(), None)
                        .await
                    {
                        tracing::warn!(error = %e, "revue des réservations de lieux en échec");
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

    // Socle 29 (P2) : rapprochement de l'index des tâches (reprise initiale, appels non faits,
    // réparation des événements perdus). Idempotent, toutes les 10 minutes ; l'affichage ne dépend
    // jamais de lui (revérification à la lecture, R-PO3). Désactivable (`ENT_AFAIRE_RAPPROCHEMENT=0`).
    let rapprochement_afaire = (std::env::var("ENT_AFAIRE_RAPPROCHEMENT").as_deref() != Ok("0")).then(|| {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(90)).await;
            loop {
                match ent.rapprocher_afaire_tous(chrono::Utc::now()).await {
                    Ok(n) if n > 0 => tracing::info!(ecarts = n, "rapprochement de l'index des tâches"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("rapprochement de l'index des tâches : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(600)).await;
            }
        })
    });

    // Socle 15 (A2) : purge programmée des pièces et du fil des absences de personnel à leur date de
    // conservation (jamais sous gel), une fois par heure et par établissement (le jour dépend du fuseau de
    // chaque établissement). Idempotente. Désactivable (`ENT_TACHES_ABSENCES=0`).
    let taches_absences = (std::env::var("ENT_TACHES_ABSENCES").unwrap_or_default() != "0").then(|| {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(90)).await;
            loop {
                // Impact des absences à venir : rattrape une nouvelle version d'EDT (CAP-46).
                match ent.recalculer_impacts_a_venir().await {
                    Ok(b) if b.a_revoir > 0 => tracing::info!(?b, "impacts d'absences à revoir"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("recalcul des impacts d'absences : {e}"),
                }
                match ent.purger_echeances(None).await {
                    Ok(b) if b.pieces_purgees > 0 || b.messages_supprimes > 0 => {
                        tracing::info!(?b, "purge des pièces d'absences de personnel")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("purge des pièces d'absences de personnel : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        })
    });

    // Socle 15 (A4) : expiration des sollicitations de remplacement (20 minutes le jour même), envoi de celles
    // que la plage calme avait différées, alerte d'accusé manquant. Chaque minute, idempotente.
    let sollicitations_absences = (std::env::var("ENT_TACHES_ABSENCES").unwrap_or_default() != "0").then(|| {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(45)).await;
            loop {
                match ent.traiter_sollicitations().await {
                    Ok(b) if b.expirees > 0 || b.accuses_manquants > 0 => {
                        tracing::info!(?b, "sollicitations de remplacement")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("sollicitations de remplacement : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        })
    });

    // Socle 15 (A5) : état du remplacement des absences longues (« nommé » → « en cours » → « terminé »), une
    // seule notification par étape. Chaque minute, idempotente.
    let remplacements_absences = (std::env::var("ENT_TACHES_ABSENCES").unwrap_or_default() != "0").then(|| {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(50)).await;
            loop {
                match ent.avancer_etats_remplacement().await {
                    Ok(b) if b.etapes_changees > 0 => tracing::info!(?b, "état du remplacement"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("état du remplacement : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
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
    if let Some(t) = rapprochement_afaire {
        t.abort();
    }
    if let Some(t) = taches_absences {
        t.abort();
    }
    if let Some(t) = sollicitations_absences {
        t.abort();
    }
    if let Some(t) = remplacements_absences {
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
