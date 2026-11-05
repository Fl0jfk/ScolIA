//! Messagerie contre le **vrai** backend (`ent_api::app`) et une base PostgreSQL **locale**
//! de test (données de labo fictives) : un professeur écrit à une famille, la famille répond,
//! une autre famille est refusée, l'élève lit les siens, pièces jointes, modèles, classement,
//! archivage, rôles sans messagerie, session absente.
//!
//! Lancement :
//!   . ent-rust/web/scripts/env.sh && bash ent-rust/web/scripts/pg.sh start
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test messagerie -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_web_api::{ApiClient, ApiError, EleveTrouve};
use ent_web_server::labo::{self, Labo};
use uuid::Uuid;

async fn demarrer() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent (sourcer scripts/env.sh)");
        return None;
    };
    let parsed = ent_db::assert_local_postgres_url(&url)
        .unwrap_or_else(|e| panic!("base de test locale uniquement : {e}"));
    let nom_base = parsed.path().trim_start_matches('/').to_ascii_lowercase();
    assert!(
        ["test", "proto", "lab"]
            .iter()
            .any(|m| nom_base.contains(m)),
        "le nom de la base doit contenir test, proto ou lab (refusé : {nom_base})"
    );
    let (app, _) = ent_web_server::routeur_complet()
        .await
        .expect("démarrage ent-api + web");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    Some(format!("http://{addr}"))
}

async fn client(base: &str, l: &Labo, email: &str) -> ApiClient {
    let s = labo::connecter(base, l, email)
        .await
        .expect("connexion de labo");
    ApiClient::new(base, Some(s.token))
}

/// Parent de labo n° `index` (0 = Lily Potter, 1 = Narcissa Malefoy, 2 = Molly Weasley).
async fn parent(base: &str, l: &Labo, index: usize) -> ApiClient {
    let p = l
        .personas
        .iter()
        .filter(|p| p.role == "parent")
        .nth(index)
        .expect("parent de labo");
    client(base, l, &p.email).await
}

async fn staff(base: &str, l: &Labo, role: &str) -> ApiClient {
    let p = l.persona(role).expect("persona de labo");
    client(base, l, &p.email).await
}

fn unique(prefixe: &str) -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefixe} {n}")
}

async fn eleve(api: &ApiClient, nom: &str, prenom: &str) -> EleveTrouve {
    api.messagerie_chercher_eleves(nom)
        .await
        .expect("recherche d'élèves")
        .into_iter()
        .find(|e| e.prenom.eq_ignore_ascii_case(prenom) && e.nom.eq_ignore_ascii_case(nom))
        .unwrap_or_else(|| panic!("élève de labo {prenom} {nom}"))
}

fn refus<T: std::fmt::Debug>(r: &Result<T, ApiError>) -> bool {
    matches!(r, Err(ApiError::Interdit(_)))
}

const PDF: &[u8] = b"%PDF-1.4\n% piece jointe de test\n";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn professeur_famille_professeur() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let prof = staff(&base, &l, "prof").await;
    let lily = parent(&base, &l, 0).await;
    let harry = eleve(&prof, "POTTER", "Harry").await;
    let sujet = unique("Sortie scolaire");

    // 1. Le professeur écrit à propos d'Harry, avec une pièce jointe.
    let ouvert = prof
        .ouvrir_fil_messages(
            harry.id,
            &sujet,
            "Bonjour,\nmerci de signer l'autorisation.",
        )
        .await
        .expect("ouverture du fil");
    assert_eq!(ouvert.fil.sujet, sujet);
    assert_eq!(ouvert.messages.len(), 1);
    let premier = ouvert.messages[0].id;
    let piece = prof
        .joindre_piece(premier, "autorisation.pdf", "application/pdf", PDF)
        .await
        .expect("pièce jointe du professeur");
    assert_eq!(piece.taille_octets as usize, PDF.len());

    // 2. La famille voit le fil non lu dans sa boîte, l'ouvre (accusé), télécharge la pièce.
    let boite = lily.boite_messages(false).await.expect("boîte famille");
    let fil = boite
        .iter()
        .find(|f| f.id == ouvert.fil.id)
        .expect("le fil figure dans la boîte de la famille");
    assert!(fil.nb_non_lus >= 1, "non lu avant ouverture : {fil:?}");
    assert!(fil
        .eleve_label
        .as_deref()
        .unwrap_or("")
        .to_lowercase()
        .contains("potter"));
    let lu = lily.detail_fil(fil.id).await.expect("ouverture famille");
    assert_eq!(lu.messages.len(), 1);
    assert_eq!(lu.messages[0].pieces.len(), 1);
    assert!(lu.messages[0].auteur_label.is_some(), "auteur nommé");
    let octets = lily
        .telecharger_piece(lu.messages[0].pieces[0].id)
        .await
        .expect("téléchargement famille");
    assert_eq!(octets, PDF, "contenu identique à l'envoi");
    let boite = lily.boite_messages(false).await.unwrap();
    assert_eq!(
        boite.iter().find(|f| f.id == fil.id).unwrap().nb_non_lus,
        0,
        "lu après ouverture"
    );

    // 3. La famille répond (avec une pièce) ; une réponse vide est refusée.
    assert!(matches!(
        lily.repondre_fil(fil.id, "   ").await,
        Err(ApiError::Invalide(_))
    ));
    let reponse = lily
        .repondre_fil(fil.id, "Signé, merci.")
        .await
        .expect("réponse famille");
    lily.joindre_piece(
        reponse.id,
        "scan.png",
        "image/png",
        &[0x89, b'P', b'N', b'G'],
    )
    .await
    .expect("pièce jointe famille");

    // 4. Le professeur voit la réponse non lue, puis les deux messages dans l'ordre, accusé inclus.
    let boite = prof.boite_messages(false).await.expect("boîte professeur");
    assert!(
        boite
            .iter()
            .find(|f| f.id == fil.id)
            .expect("fil chez le professeur")
            .nb_non_lus
            >= 1
    );
    let relu = prof.detail_fil(fil.id).await.unwrap();
    assert_eq!(relu.messages.len(), 2);
    assert_eq!(
        relu.messages[0].corps.trim(),
        "Bonjour,\nmerci de signer l'autorisation."
    );
    assert_eq!(relu.messages[1].corps, "Signé, merci.");
    assert_eq!(relu.messages[1].pieces[0].nom, "scan.png");
    assert!(
        relu.messages[0].nb_accuses >= 1,
        "la famille a accusé réception : {:?}",
        relu.messages[0]
    );
    let boite = prof.boite_messages(false).await.unwrap();
    assert_eq!(boite.iter().find(|f| f.id == fil.id).unwrap().nb_non_lus, 0);

    // 5. Classer puis archiver (personnel) : la boîte courante n'affiche plus le fil.
    let classe = prof
        .classer_fil(fil.id, "haute", Some(" VS "))
        .await
        .expect("classement");
    assert!(classe.est_prioritaire());
    assert_eq!(classe.dossier.as_deref(), Some("vs"));
    assert!(matches!(
        prof.classer_fil(fil.id, "haute", Some("vie scolaire"))
            .await,
        Err(ApiError::Invalide(_))
    ));
    assert!(matches!(
        prof.classer_fil(fil.id, "urgente", None).await,
        Err(ApiError::Invalide(_))
    ));
    let archive = prof.archiver_fil(fil.id).await.expect("archivage");
    assert!(archive.archive);
    let courante = prof.boite_messages(false).await.unwrap();
    assert!(courante.iter().all(|f| f.id != fil.id), "archivé masqué");
    let avec = prof.boite_messages(true).await.unwrap();
    let f = avec
        .iter()
        .find(|f| f.id == fil.id)
        .expect("archivé listé sur demande");
    assert!(f.archive && f.est_prioritaire() && f.dossier.as_deref() == Some("vs"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloisonnement_et_roles() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let prof = staff(&base, &l, "prof").await;
    let lily = parent(&base, &l, 0).await;
    let narcissa = parent(&base, &l, 1).await;
    let harry = eleve(&prof, "POTTER", "Harry").await;

    let ouvert = prof
        .ouvrir_fil_messages(harry.id, &unique("Réunion"), "Texte confidentiel")
        .await
        .unwrap();
    let fil = ouvert.fil.id;
    let message = ouvert.messages[0].id;
    let piece = prof
        .joindre_piece(message, "note.txt", "text/plain", b"note")
        .await
        .unwrap();

    // Une autre famille ne voit ni le fil, ni son contenu, ni sa pièce, et ne peut pas y écrire.
    assert!(narcissa
        .boite_messages(false)
        .await
        .unwrap()
        .iter()
        .all(|f| f.id != fil));
    assert!(refus(&narcissa.detail_fil(fil).await));
    assert!(refus(&narcissa.telecharger_piece(piece.id).await));
    assert!(refus(&narcissa.repondre_fil(fil, "intrusion").await));
    assert!(refus(
        &narcissa
            .joindre_piece(message, "x.txt", "text/plain", b"x")
            .await
    ));
    assert!(refus(
        &narcissa
            .ouvrir_fil_messages(harry.id, "Pas mon enfant", "Bonjour")
            .await
    ));
    // Elle ne voit que ses enfants.
    let enfants = narcissa.enfants_famille().await.unwrap();
    assert!(enfants.iter().all(|e| e.id != harry.id) && !enfants.is_empty());
    let enfants_lily = lily.enfants_famille().await.unwrap();
    assert!(enfants_lily.iter().any(|e| e.id == harry.id));

    // La famille n'a ni les outils du personnel, ni la recherche d'élèves.
    assert!(refus(&lily.archiver_fil(fil).await));
    assert!(refus(&lily.classer_fil(fil, "haute", None).await));
    assert!(refus(&lily.modeles_messages().await));
    assert!(refus(&lily.messagerie_chercher_eleves("POTTER").await));
    assert!(refus(
        &lily
            .ouvrir_fil_depuis_modele(Uuid::new_v4(), harry.id)
            .await
    ));
    // Un message sans objet ou sans texte est refusé par le serveur.
    assert!(matches!(
        lily.ouvrir_fil_messages(harry.id, " ", "Bonjour").await,
        Err(ApiError::Invalide(_))
    ));
    assert!(matches!(
        lily.ouvrir_fil_messages(harry.id, "Objet", " ").await,
        Err(ApiError::Invalide(_))
    ));

    // Pièces jointes : seul l'auteur du message en ajoute ; types et tailles contrôlés.
    assert!(lily
        .joindre_piece(message, "x.txt", "text/plain", b"x")
        .await
        .is_err());
    let r = prof
        .joindre_piece(message, "virus.exe", "application/x-msdownload", b"MZ")
        .await;
    assert!(matches!(r, Err(ApiError::Invalide(_))), "{r:?}");
    let gros = vec![0u8; 256 * 1024 + 1];
    let r = prof
        .joindre_piece(message, "gros.pdf", "application/pdf", &gros)
        .await;
    assert!(matches!(r, Err(ApiError::Invalide(_))), "{r:?}");
    let r = prof
        .joindre_piece(message, "vide.pdf", "application/pdf", b"")
        .await;
    assert!(matches!(r, Err(ApiError::Invalide(_))), "{r:?}");

    // L'élève lit les fils qui le concernent (s'il a un compte de labo).
    if let Some(p) = l.persona("eleve") {
        let eleve_api = client(&base, &l, &p.email).await;
        let boite = eleve_api.boite_messages(false).await.expect("boîte élève");
        assert!(boite.iter().any(|f| f.id == fil), "l'élève voit son fil");
        assert!(
            boite.iter().all(|f| f.eleve_id == harry.id),
            "et seulement les siens"
        );
        let detail = eleve_api.detail_fil(fil).await.expect("lecture élève");
        assert_eq!(detail.messages.len(), 1);
        assert!(refus(&eleve_api.archiver_fil(fil).await));
        assert!(refus(&eleve_api.modeles_messages().await));
        assert!(refus(&eleve_api.messagerie_chercher_eleves("POTTER").await));
        let ron = eleve(&prof, "WEASLEY", "Ron").await;
        assert!(refus(
            &eleve_api
                .ouvrir_fil_messages(ron.id, "Autre élève", "Bonjour")
                .await
        ));
    }

    // Rôles sans messagerie : comptabilité, infirmerie, psychologue → 403.
    for role in ["compta", "infirmerie", "psy"] {
        if l.persona(role).is_some() {
            let api = staff(&base, &l, role).await;
            assert!(refus(&api.boite_messages(false).await), "{role} : boîte");
            assert!(refus(&api.detail_fil(fil).await), "{role} : fil");
            assert!(refus(&api.modeles_messages().await), "{role} : modèles");
        }
    }

    // Sans session : 401 partout.
    let anonyme = ApiClient::new(&base, None);
    assert_eq!(
        anonyme.boite_messages(false).await,
        Err(ApiError::NonConnecte)
    );
    assert_eq!(anonyme.detail_fil(fil).await, Err(ApiError::NonConnecte));
    assert_eq!(
        anonyme.telecharger_piece(piece.id).await,
        Err(ApiError::NonConnecte)
    );
    // Identifiants inconnus : 404 pour une personne autorisée.
    assert!(matches!(
        prof.detail_fil(Uuid::new_v4()).await,
        Err(ApiError::Introuvable(_))
    ));
    assert!(prof.telecharger_piece(Uuid::new_v4()).await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn modeles_du_personnel() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let prof = staff(&base, &l, "prof").await;
    let lily = parent(&base, &l, 0).await;
    let harry = eleve(&prof, "POTTER", "Harry").await;
    let titre = unique("Modèle absence");

    let cree = prof
        .post_json(
            "/messages/modeles",
            serde_json::json!({
                "titre": titre,
                "sujet": "Absence de {prenom}",
                "corps": "Bonjour, {prenom} {nom} ({classe}) était absent."
            }),
        )
        .await
        .expect("création d'un modèle");
    let modele_id: Uuid = serde_json::from_value(cree["id"].clone()).unwrap();
    let modeles = prof.modeles_messages().await.expect("liste des modèles");
    assert!(modeles.iter().any(|m| m.id == modele_id && m.actif));

    let d = prof
        .ouvrir_fil_depuis_modele(modele_id, harry.id)
        .await
        .expect("fil depuis modèle");
    assert!(
        d.fil.sujet.starts_with("Absence de Harry"),
        "{:?}",
        d.fil.sujet
    );
    let texte = &d.messages[0].corps;
    assert!(
        texte.contains("Harry") && texte.to_lowercase().contains("potter"),
        "{texte}"
    );
    assert!(!texte.contains('{'), "marqueurs remplacés : {texte}");

    // La famille de l'élève reçoit le message issu du modèle.
    let boite = lily.boite_messages(false).await.unwrap();
    assert!(boite.iter().any(|f| f.id == d.fil.id));
    // Modèle inconnu : refusé.
    assert!(prof
        .ouvrir_fil_depuis_modele(Uuid::new_v4(), harry.id)
        .await
        .is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pages_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    for (chemin, attendu) in [
        ("/web/messages", "Chargement des conversations"),
        ("/web/messages/nouveau", "Chargement du formulaire"),
        (
            "/web/messages/6f1c3c80-0000-4000-8000-000000000001",
            "Chargement de la conversation",
        ),
    ] {
        let r = http.get(format!("{base}{chemin}")).send().await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        let csp = r.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .to_string();
        assert!(
            csp.contains("style-src 'self';"),
            "pas de style en ligne : {csp}"
        );
        let html = r.text().await.unwrap();
        assert!(html.contains(r#"<html lang="fr""#), "{chemin}");
        assert!(
            html.contains(attendu),
            "{chemin} : état de chargement rendu côté serveur"
        );
        assert!(
            html.contains("Aller au contenu"),
            "{chemin} : lien d'évitement"
        );
        assert!(
            !html.contains(" style=\""),
            "{chemin} : aucun style en ligne"
        );
    }
}
