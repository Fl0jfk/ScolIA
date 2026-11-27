//! Cahier de textes, travail et copies contre le **vrai** backend (`ent_api::app`) et une
//! base PostgreSQL **locale** de test (données de labo fictives) : pages rendues par le
//! serveur (CSP, langue, aucun style en ligne), parcours enseignant → élève → famille →
//! copie → correction, lien signé réellement servi, permissions des autres profils.
//!
//! Lancement :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test cahier -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use chrono::{Duration, NaiveDate};
use ent_web_api::cahier_types::CreerTravailBody;
use ent_web_api::{depot_fichier, ApiClient, ApiError};
use ent_web_server::labo::{self, Labo};
use uuid::Uuid;

const PDF: &[u8] =
    b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";

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

async fn client_de(base: &str, l: &Labo, email: &str) -> ApiClient {
    let s = labo::connecter(base, l, email)
        .await
        .unwrap_or_else(|e| panic!("connexion {email} : {e:?}"));
    ApiClient::new(base, Some(s.token))
}

async fn persona(base: &str, l: &Labo, role: &str, label: &str) -> ApiClient {
    let p = l
        .personas
        .iter()
        .find(|p| p.role == role && p.label.contains(label))
        .unwrap_or_else(|| panic!("persona {role} « {label} »"));
    client_de(base, l, &p.email).await
}

fn aujourd_hui() -> NaiveDate {
    chrono::Local::now().date_naive()
}

fn est_interdit<T>(r: Result<T, ApiError>) -> bool {
    matches!(r, Err(ApiError::Interdit(_)))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pages_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    let http = reqwest::Client::new();
    let inconnu = Uuid::from_u128(1);
    for (chemin, titre) in [
        ("/web/cahier".to_string(), "Cahier de textes"),
        (format!("/web/cahier/travaux/{inconnu}"), "Copies reçues"),
        ("/web/agenda".to_string(), "Travail à faire"),
    ] {
        let r = http.get(format!("{base}{chemin}")).send().await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(r.headers().contains_key("content-security-policy"), "CSP");
        let html = r.text().await.unwrap();
        assert!(html.contains(titre), "{chemin} : titre");
        assert!(html.contains(r#"<html lang="fr""#), "{chemin} : langue");
        assert!(
            !html.contains(" style=\""),
            "{chemin} : aucun style en ligne (CSP)"
        );
    }
}

/// Parcours : l'enseignant publie un travail à rendre pour 4B ; l'élève et la famille le
/// voient, une autre famille non ; l'élève rend une copie ; l'enseignant la lit par lien signé
/// puis la corrige ; l'élève voit le retour. Les autres profils sont refusés.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parcours_enseignant_eleve_famille_copie_et_correction() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let admin = persona(&base, &l, "admin", "").await;
    admin.amorcer_referentiel().await.expect("amorçage");
    let rogue = persona(&base, &l, "prof", "Rogue").await;
    let harry = persona(&base, &l, "eleve", "Harry").await;
    let lily = persona(&base, &l, "parent", "Harry").await;
    let narcissa = persona(&base, &l, "parent", "Draco").await;
    let harry_id = l.eleve("Harry").expect("Harry").id;
    let draco_id = l.eleve("Draco").expect("Draco").id;
    let jour = aujourd_hui();
    let classe_4b = admin
        .classes(jour)
        .await
        .expect("classes")
        .into_iter()
        .find(|c| c.code == "4B")
        .expect("classe 4B")
        .id;

    // L'enseignant publie ; les durées alimentent la charge de la classe.
    let pour_le = jour + Duration::days(3);
    let cree = rogue
        .cahier_donner_travail(&CreerTravailBody {
            seance_id: None,
            classe_id: Some(classe_4b),
            groupe_id: None,
            matiere_libelle: Some("Potions".into()),
            type_travail: Some("a_rendre".into()),
            titre: "Philtre de paix".into(),
            consigne: "Rédiger la recette complète.".into(),
            pour_le,
            duree_estimee_min: Some(150),
            date_limite: None,
            acceptation_retard: Some(true),
            formats_acceptes: vec!["pdf".into()],
        })
        .await
        .expect("publication");
    let travail = cree.travail;
    assert_eq!(
        (travail.cible.as_str(), travail.rendu_en_ligne),
        ("4B", true)
    );
    assert!(
        cree.charge.surcharge,
        "150 min dans la journée : alerte de surcharge"
    );
    let charge = rogue
        .cahier_charge(classe_4b, jour, jour + Duration::days(7))
        .await
        .expect("charge visible de l'équipe");
    assert!(charge
        .jours
        .iter()
        .any(|j| j.jour == pour_le && j.surcharge));

    // Élève et famille concernés voient le travail ; l'autre famille ne voit pas Harry.
    let semaine = (jour, jour + Duration::days(7));
    let agenda_h = harry
        .cahier_agenda(None, semaine.0, semaine.1)
        .await
        .expect("agenda élève");
    assert!(agenda_h
        .iter()
        .any(|i| i.travail.id == travail.id && !i.fait));
    let agenda_p = lily
        .cahier_agenda(Some(harry_id), semaine.0, semaine.1)
        .await
        .expect("agenda famille");
    assert!(agenda_p.iter().any(|i| i.travail.id == travail.id));
    assert!(est_interdit(
        narcissa
            .cahier_agenda(Some(harry_id), semaine.0, semaine.1)
            .await
    ));
    assert!(est_interdit(
        harry
            .cahier_agenda(Some(draco_id), semaine.0, semaine.1)
            .await
    ));
    // La famille ne marque ni ne rend ; l'élève marque « fait » son propre travail seulement.
    assert!(est_interdit(
        lily.cahier_rendre(
            travail.id,
            "",
            Some(depot_fichier("c.pdf", "application/pdf", PDF))
        )
        .await
    ));
    assert!(est_interdit(
        lily.cahier_marquer_fait(travail.id, true).await
    ));

    // Dépôt contrôlé : format hors liste refusé, fichier actif refusé, PDF accepté.
    let png = [
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0x0D, b'I', b'H', b'D', b'R',
    ];
    assert!(matches!(
        harry
            .cahier_rendre(
                travail.id,
                "",
                Some(depot_fichier("s.png", "image/png", &png))
            )
            .await,
        Err(ApiError::Invalide(_))
    ));
    assert!(matches!(
        harry
            .cahier_rendre(
                travail.id,
                "",
                Some(depot_fichier(
                    "a.pdf",
                    "application/pdf",
                    b"<svg onload=1/>"
                ))
            )
            .await,
        Err(ApiError::Invalide(_))
    ));
    let remise = harry
        .cahier_rendre(
            travail.id,
            "Voici ma recette",
            Some(depot_fichier("recette.pdf", "application/pdf", PDF)),
        )
        .await
        .expect("remise");
    assert!(!remise.en_retard && remise.pieces.len() == 1);

    // L'enseignant récupère la copie par un lien signé réellement servi (téléchargement forcé).
    let suivi = rogue.cahier_suivi(travail.id).await.expect("suivi");
    assert!(
        suivi.nb_rendus >= 1
            && suivi
                .lignes
                .iter()
                .any(|x| x.eleve_id == harry_id && x.remise.is_some())
    );
    let piece = remise.pieces[0].id;
    let lien = rogue.cahier_lien_piece(piece).await.expect("lien");
    let http = reqwest::Client::new();
    let r = http
        .get(format!("{base}{}", lien.url))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .starts_with("attachment"));
    assert_eq!(r.bytes().await.unwrap().as_ref(), PDF);
    // Copie privée : ni l'autre famille, ni la comptabilité, ni la vie scolaire.
    assert!(est_interdit(narcissa.cahier_lien_piece(piece).await));
    let compta = client_de(&base, &l, &l.persona("compta").unwrap().email).await;
    assert!(est_interdit(compta.cahier_lien_piece(piece).await));
    let cpe = client_de(&base, &l, &l.persona("cpe").unwrap().email).await;
    assert!(est_interdit(cpe.cahier_lien_piece(piece).await));
    assert!(est_interdit(
        compta.cahier_charge(classe_4b, semaine.0, semaine.1).await
    ));
    // La vie scolaire lit le cahier de la classe mais ne le modifie pas.
    assert!(cpe
        .cahier_travaux(Some(classe_4b), None, semaine.0, semaine.1)
        .await
        .is_ok());
    assert!(est_interdit(cpe.cahier_supprimer_travail(travail.id).await));

    // Correction → retour visible de l'élève, plus de nouveau dépôt.
    let corrigee = rogue
        .cahier_retour(remise.id, "Très bon travail.", None)
        .await
        .expect("retour");
    assert_eq!(corrigee.statut, "corrige");
    let agenda_h = harry
        .cahier_agenda(None, semaine.0, semaine.1)
        .await
        .unwrap();
    let item = agenda_h
        .iter()
        .find(|i| i.travail.id == travail.id)
        .unwrap();
    assert_eq!(
        item.remise
            .as_ref()
            .and_then(|r| r.retour_commentaire.as_deref()),
        Some("Très bon travail.")
    );
    assert!(matches!(
        harry
            .cahier_rendre(
                travail.id,
                "",
                Some(depot_fichier("autre.pdf", "application/pdf", PDF))
            )
            .await,
        Err(ApiError::Conflit(_))
    ));
    // Marquer « fait » : un travail à rendre en ligne se coche par le dépôt, pas à la main.
    harry
        .cahier_marquer_fait(travail.id, true)
        .await
        .expect("fait");
    // Suppression par l'auteur.
    rogue
        .cahier_supprimer_travail(travail.id)
        .await
        .expect("suppression");
    assert!(harry
        .cahier_agenda(None, semaine.0, semaine.1)
        .await
        .unwrap()
        .iter()
        .all(|i| i.travail.id != travail.id));
}

/// Sans session : toutes les routes répondent 401.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn routes_protegees_sans_session() {
    let Some(base) = demarrer().await else { return };
    let anonyme = ApiClient::new(&base, None);
    let j = aujourd_hui();
    assert!(matches!(
        anonyme.cahier_agenda(None, j, j).await,
        Err(ApiError::NonConnecte)
    ));
    assert!(matches!(
        anonyme.cahier_seances(j, j).await,
        Err(ApiError::NonConnecte)
    ));
    assert!(matches!(
        anonyme.cahier_lien_piece(Uuid::from_u128(2)).await,
        Err(ApiError::NonConnecte)
    ));
}
