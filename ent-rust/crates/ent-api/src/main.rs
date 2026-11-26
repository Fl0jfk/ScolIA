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
  creer    <slug> <nom>                     CRÉE un établissement (identifiant généré, jamais d'écrasement ;
                                            slug déjà pris refusé) — socle 28, MS0
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
        ("creer", 3) => ent
            .creer_etablissement_plateforme(&args[1], &args[2])
            .await
            .map(|i| format!("établissement {} créé : {} ({})", i.id, i.slug, i.nom))
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
  --analyser                après la reprise, analyser les pièces de messagerie en quarantaine (antivirus
                            `CLAMAV_ADDR`, désarmement et normalisation : lot M5) ; sans démon, elles y restent
Code de sortie : 0 tout repris / simulation sans échec ; 1 erreur ; 2 au moins une pièce en échec.
Les journaux et le rapport ne contiennent ni nom de fichier ni clé d'objet.
";

async fn reprise_pieces(args: &[String]) -> i32 {
    use ent_db::reprise_bytea::{OptionsReprise, TablePiece};
    let mut options = OptionsReprise::default();
    let mut etab: Option<uuid::Uuid> = None;
    let mut analyser = false;
    let mut tables: Vec<TablePiece> = TablePiece::TOUTES.to_vec();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut valeur = || it.next().cloned();
        match a.as_str() {
            "--simulation" => options.simulation = true,
            "--sauvegarde-verifiee" => options.vider = true,
            "--analyser" => analyser = true,
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
    // M5 : analyse a posteriori des pièces de messagerie (jamais en simulation, jamais sans antivirus).
    if analyser && !options.simulation && tables.contains(&TablePiece::Messagerie) {
        let Some(av) = ent_db::documents_pieces_taches::antivirus_depuis_env() else {
            return erreur_reprise("--analyser exige CLAMAV_ADDR (fermé par défaut : sans antivirus, rien ne sort de quarantaine)");
        };
        let bilan = match etab {
            Some(e) => ent.analyser_pieces_messagerie_etablissement(e, av, 500).await,
            None => ent.analyser_pieces_messagerie_tache(av, 500).await,
        };
        match bilan {
            Ok(b) => println!(
                "Analyse des pièces de messagerie : {} saine(s), {} infectée(s), {} refusée(s), {} en quarantaine{}",
                b.saines,
                b.infectees,
                b.rejetees,
                b.en_quarantaine,
                if b.antivirus_indisponible { " (antivirus indisponible : passe interrompue)" } else { "" }
            ),
            Err(e) => return erreur_reprise(&format!("analyse impossible : {e}")),
        }
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
                // Socle 3 (I5) : publie les décisions dont l'heure programmée par la direction est venue
                // (aucune décision n'est prise ici), puis expédie les messages qu'elles mettent en file.
                match ent.publier_decisions_echues_tache(chrono::Utc::now()).await {
                    Ok(n) if n > 0 => tracing::info!(n, "décisions d'inscription publiées à l'heure programmée"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("publication des décisions d'inscription : {e}"),
                }
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
    // Socle 12 (AS5) : relance graduée des absences sans motif, au jour métier de chaque établissement. Inactive
    // tant que `vie_scolaire.relance_justificatif` n'est pas basculée sur la brique ; idempotente (une étape ne
    // part qu'une fois par épisode).
    {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            loop {
                match ent.assiduite_relance_tache().await {
                    Ok(n) if n > 0 => tracing::info!(etapes = n, "relances d'assiduité"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("relance d'assiduité : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
    }
    // Socle 12 (AS6) : vérification de l'appel au jour métier de chaque établissement (chaque minute) : relance
    // automatique de l'appel non fait (une seule fois par séance, à `début + délai`) et alerte de l'élève exclu de
    // cours non arrivé. Idempotente : la base garantit l'unicité de la relance.
    {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            loop {
                match ent.assiduite_verification_tache().await {
                    Ok(n) if n > 0 => tracing::info!(faits = n, "vérification d'assiduité"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("vérification d'assiduité : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });
    }
    // Socle 41 (I2) : rappel d'un appel de nuit non validé à l'heure limite (une seule fois par niveau :
    // surveillant, puis CPE d'astreinte). Chaque minute ; idempotent, la base garantit l'unicité.
    {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            loop {
                match ent.internat_rappels_tache().await {
                    Ok(n) if n > 0 => tracing::info!(rappels = n, "rappels d'appel de nuit"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("rappels d'appel de nuit : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });
    }
    // Socle 41 (I3) : veille des sorties de l'internat : cas « retard de retour » et « non récupéré » au-delà du
    // seuil, sorties jamais utilisées qui expirent, sorties à venir revues (personne devenue interdite). Chaque
    // minute ; idempotent (un seul cas ouvert par élève et par nuit).
    {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            loop {
                match ent.internat_sorties_tache().await {
                    Ok(n) if n > 0 => tracing::info!(sorties = n, "veille des sorties d'internat"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("veille des sorties d'internat : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
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

    // Suites du 18 novembre : passages quotidiens des voyages (alertes d'échéance des acomptes fournisseurs, purge
    // à 30 jours des exports CSV de l'argent) et balayage borné des objets orphelins (photos d'élèves, exports),
    // sur le MÊME planificateur : idempotent, lancé toutes les heures car « le jour » dépend du fuseau de chaque
    // établissement ; y passe aussi l'arrêt automatique des notes des périodes (socle 21, M8). Désactivable (`ENT_TACHES_VOYAGES_OBJETS=0`).
    let taches_voyages_objets = taches_voyages_objets_actives().then(|| {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(120)).await;
            loop {
                match ent.passages_quotidiens(None).await {
                    Ok(b) if b.acomptes_alertes + b.exports_purges + b.photos_orphelines + b.exports_orphelins + b.periodes_arretees + b.publications_automatiques + b.erreurs > 0 => {
                        tracing::info!(?b, "passages quotidiens des voyages et balayage des objets")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("passages quotidiens des voyages et balayage des objets : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        })
    });

    // Socle 21 (V3b, F7) : publications automatiques des évaluations (programmées, délai maximal, rappels), boucle
    // DÉDIÉE toutes les 5 minutes : une publication programmée sort au plus 6 minutes après l'heure choisie
    // (`passages_quotidiens::RETARD_MAX_PUBLICATION_PROGRAMMEE_SECONDES`). Idempotent. Désactivable
    // (`ENT_PUBLICATIONS_EVALUATIONS=0`).
    let publications_evaluations = (!valeur_desactivante(&std::env::var("ENT_PUBLICATIONS_EVALUATIONS").unwrap_or_default())).then(|| {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(150)).await;
            loop {
                match ent.eva_v3_publications_automatiques_tous(chrono::Utc::now()).await {
                    Ok(b) if b.programmees + b.delai_max + b.rappels > 0 => {
                        tracing::info!(?b, "publications automatiques des évaluations")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("publications automatiques des évaluations : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(300)).await;
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

    // Socle 15 (A6) : remontée hebdomadaire du remplacement de courte durée. Chaque heure, idempotente : elle ne
    // prépare que la semaine passée, une seule fois (S-1516 : lundi, paramétrable plus tard).
    let remontees_rcd = (std::env::var("ENT_TACHES_ABSENCES").unwrap_or_default() != "0").then(|| {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(70)).await;
            loop {
                match ent.preparer_remontees_hebdomadaires().await {
                    Ok(b) if b.preparees > 0 => tracing::info!(?b, "remontées RCD préparées"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("remontées RCD : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        })
    });

    // MSE-D01 : purge périodique des sessions et jetons expirés (désactivable : `ENT_PURGE_PERIODE_SECONDES`
    // bornée entre 60 s et 24 h ; une heure par défaut).
    let purge_periodique =
        ent_api::purge::lancer(state.ent.clone(), ent_api::purge::periode_depuis_env());

    // Socle 15 (A7) : heures constatées depuis les appels faits (socle 12) et file DSN J+5. Toutes les cinq minutes,
    // idempotentes : une heure déjà constatée n'est jamais recréée.
    let heures_absences = (std::env::var("ENT_TACHES_ABSENCES").unwrap_or_default() != "0").then(|| {
        let ent = state.ent.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(90)).await;
            loop {
                match ent.constater_heures_assurees().await {
                    Ok(b) if b.constatees > 0 => tracing::info!(?b, "heures constatées"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("heures constatées : {e}"),
                }
                if let Err(e) = ent.preparer_dsn_planifie().await {
                    tracing::warn!("file DSN : {e}");
                }
                // A8 : rattrapages dus (création manquante, créneaux assurés par l'appel, relance J+10, échéance).
                // A9 : une fermeture de l'établissement neutralise les absences du jour (impacts « sans objet »).
                if let Err(e) = ent.neutraliser_absences_fermees().await {
                    tracing::warn!("fermeture : {e}");
                }
                match ent.suivre_rattrapages().await {
                    Ok(b) if b.crees + b.relances + b.alertes_echeance + b.soldes > 0 => tracing::info!(?b, "rattrapages suivis"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("rattrapages : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            }
        })
    });

    // Socle 7 (D5b) : analyse antivirus des pièces en quarantaine et purge des objets des pièces infectées.
    // Fermé par défaut : sans `CLAMAV_ADDR`, l'analyse n'est pas lancée et aucune pièce ne sort de quarantaine.
    // `ENT_TACHES_PIECES=0` désactive les deux tâches (processus dédié).
    let taches_pieces_actives = std::env::var("ENT_TACHES_PIECES").unwrap_or_default() != "0";
    let analyse_pieces = if !taches_pieces_actives {
        None
    } else if let Some(av) = ent_db::documents_pieces_taches::antivirus_depuis_env() {
        let ent = state.ent.clone();
        Some(tokio::spawn(async move {
            loop {
                match ent.analyser_pieces_tache(av.clone(), 20).await {
                    Ok(b) if b.antivirus_indisponible => {
                        tracing::warn!("antivirus indisponible : pièces maintenues en quarantaine")
                    }
                    Ok(b) if b.analyses != Default::default() => {
                        tracing::info!(analyses = ?b.analyses, "pièces analysées")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("analyse des pièces : {e}"),
                }
                // Messagerie (M5) : mêmes règles — pièces jointes en quarantaine, antivirus puis normalisation.
                match ent.analyser_pieces_messagerie_tache(av.clone(), 20).await {
                    Ok(b) if b.antivirus_indisponible => {
                        tracing::warn!("antivirus indisponible : pièces de messagerie maintenues en quarantaine")
                    }
                    Ok(b) if b != Default::default() => {
                        tracing::info!(analyses = ?b, "pièces de messagerie analysées")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("analyse des pièces de messagerie : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        }))
    } else {
        tracing::warn!(
            "CLAMAV_ADDR absente : aucune analyse antivirus, les pièces déposées restent en quarantaine (fermé par défaut)"
        );
        None
    };
    let purge_pieces = taches_pieces_actives.then(|| {
        let ent = state.ent.clone();
        let retention = ent_db::documents_pieces_taches::retention_infectees_depuis_env();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(120)).await;
            loop {
                // Socle 27 (G3a, E1) : destruction des pièces d'identité des demandes de droits closes, demandée
                // à la clôture et rejouée tant que le magasin ne l'a pas confirmée.
                match ent.droits_destruction_pieces_tache().await {
                    Ok(n) if n > 0 => tracing::info!(n, "pièces d'identité de demandes de droits détruites"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("destruction des pièces d'identité (droits) : {e}"),
                }
                // Socle 27 (G3b) : objets des extraits remis, supprimés à la fin de la mise à disposition (7 jours).
                match ent.droits_extraits_purger_tache().await {
                    Ok(n) if n > 0 => tracing::info!(n, "fichiers d'extraits RGPD purgés"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!("purge des extraits RGPD : {e}"),
                }
                // Socle 27 (G4) : partitions mensuelles des traces d'accès créées à l'avance (mois courant et deux suivants).
                if let Err(e) = ent.acces_partitions_assurer().await {
                    tracing::warn!("partitions des traces d'accès : {e}");
                }
                match ent.purger_objets_pieces_infectees_tache(retention).await {
                    Ok(b) if b.purges > 0 || b.echecs > 0 => {
                        tracing::info!(?b, "objets de pièces infectées purgés")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("purge des pièces infectées : {e}"),
                }
                // Messagerie (M5) : originaux des pièces refusées (infectées 7 jours, rejetées 30 jours par défaut).
                let (j_inf, j_rej) = ent_db::retention_pieces_msg_depuis_env();
                match ent.purger_pieces_messagerie_refusees_tache(j_inf, j_rej).await {
                    Ok(b) if b.purges > 0 || b.echecs > 0 => {
                        tracing::info!(?b, "originaux de pièces de messagerie refusées purgés")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("purge des pièces de messagerie refusées : {e}"),
                }
                // Socle 3 (I3) : fichiers des dossiers d'inscription non aboutis, après le délai de recours.
                match ent
                    .purger_pieces_non_aboutis_tache(ent_db::inscriptions_pieces::PURGE_NON_ABOUTIS_JOURS)
                    .await
                {
                    Ok(b) if b.dossiers > 0 || b.echecs > 0 => {
                        tracing::info!(?b, "pièces de dossiers d'inscription non aboutis purgées")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("purge des pièces d'inscription : {e}"),
                }
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        })
    });

    // Socle 32 (espaces de travail, ET0–ET1) : réservations de quota expirées, corbeille (30 jours), élagage des versions et
    // analyse antivirus des fichiers déposés. **Fermé par défaut** : sans `CLAMAV_ADDR`, rien n'est analysé et aucun fichier
    // déposé n'est visible des autres membres. `ENT_TACHES_ESPACES=0` désactive la tâche (processus dédié).
    let taches_espaces = (std::env::var("ENT_TACHES_ESPACES").unwrap_or_default() != "0").then(|| {
        let ent = state.ent.clone();
        let antivirus = ent_db::documents_pieces_taches::antivirus_depuis_env();
        if antivirus.is_none() {
            tracing::warn!("CLAMAV_ADDR absente : les fichiers déposés dans les espaces restent invisibles des autres membres (fermé par défaut)");
        }
        tokio::spawn(async move {
            loop {
                match ent.espaces_tache_tous_etablissements(antivirus.clone(), chrono::Utc::now()).await {
                    Ok((liberees, purge, elaguees, analyse)) => {
                        if liberees + purge.effaces + purge.gardes + elaguees + analyse.saines + analyse.infectees > 0 {
                            tracing::info!(liberees, ?purge, elaguees, ?analyse, "espaces : tâches de fond");
                        }
                        if analyse.indisponibles > 0 {
                            tracing::warn!(indisponibles = analyse.indisponibles, "espaces : analyse indisponible, fichiers maintenus invisibles");
                        }
                    }
                    Err(e) => tracing::warn!("espaces : tâches de fond : {e}"),
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
    purge_periodique.abort();
    if let Some(t) = expediteur_inscriptions {
        t.abort();
    }
    if let Some(t) = taches_eleves {
        t.abort();
    }
    if let Some(t) = taches_voyages_objets {
        t.abort();
    }
    if let Some(t) = publications_evaluations {
        t.abort();
    }
    if let Some(t) = rapprochement_afaire {
        t.abort();
    }
    if let Some(t) = taches_absences {
        t.abort();
    }
    if let Some(t) = analyse_pieces {
        t.abort();
    }
    if let Some(t) = purge_pieces {
        t.abort();
    }
    if let Some(t) = taches_espaces {
        t.abort();
    }
    if let Some(t) = sollicitations_absences {
        t.abort();
    }
    if let Some(t) = remplacements_absences {
        t.abort();
    }
    if let Some(t) = remontees_rcd {
        t.abort();
    }
    if let Some(t) = heures_absences {
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

/// Passages quotidiens des voyages et balayage des objets : actifs sauf `ENT_TACHES_VOYAGES_OBJETS=0|false|non|off`.
fn taches_voyages_objets_actives() -> bool {
    !valeur_desactivante(&std::env::var("ENT_TACHES_VOYAGES_OBJETS").unwrap_or_default())
}

fn valeur_desactivante(v: &str) -> bool {
    matches!(v.trim().to_ascii_lowercase().as_str(), "0" | "false" | "non" | "off")
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

#[cfg(test)]
mod tests_planificateur {
    use super::valeur_desactivante;

    /// Les passages quotidiens des voyages se désactivent par la variable d'environnement, comme les autres tâches.
    #[test]
    fn la_variable_desactive_les_passages_quotidiens() {
        for v in ["0", "false", "NON", " off "] {
            assert!(valeur_desactivante(v), "{v}");
        }
        for v in ["", "1", "true", "oui"] {
            assert!(!valeur_desactivante(v), "{v}");
        }
    }
}
