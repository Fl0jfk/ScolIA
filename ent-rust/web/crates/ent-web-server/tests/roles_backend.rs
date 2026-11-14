//! Écrans « Rôles et droits » et récupération de passkey contre le **vrai** backend
//! (`ent_api::app`) et une base PostgreSQL **locale** de test : catalogue, cycle de vie d'un
//! rôle (création depuis un modèle, modification, attribution datée, retrait, suppression),
//! refus (personnel sans `roles.gerer`, escalade, portée non admise, cible manquante) et
//! réponses publiques de la récupération de passkey.
//!
//! Lancement : comme `contre_backend.rs` (`ENT_DATABASE_URL` vers une base locale de test,
//! `--features labo`, `--test-threads=1`). Sans `ENT_DATABASE_URL`, les tests sont ignorés.
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_web_api::{ApiClient, ApiError, DroitApi};
use ent_web_app::modules::roles::modele::{
    message_erreur_role, modele_par_id, peut_gerer_roles, EditionRole,
};
use ent_web_server::labo::{self, Labo};
use uuid::Uuid;

async fn demarrer() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent");
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

async fn client_du_role(base: &str, l: &Labo, role: &str) -> (ApiClient, Uuid) {
    let p = l
        .personnel
        .iter()
        .find(|p| p.role == role)
        .unwrap_or_else(|| panic!("personnel de labo « {role} »"));
    let s = labo::connecter(base, l, &p.email).await.expect("connexion");
    (ApiClient::new(base, Some(s.token)), p.user_id)
}

fn droit(action: &str, portee: &str) -> DroitApi {
    DroitApi {
        action: action.into(),
        portee: portee.into(),
        cibles: vec![],
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cycle_de_vie_d_un_role_contre_vrai_backend() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let (direction, _) = client_du_role(&base, &l, "direction").await;
    let (prof, _) = client_du_role(&base, &l, "prof").await;
    let (compta, compta_id) = client_du_role(&base, &l, "compta").await;

    // L'écran n'est proposé qu'à qui a `roles.gerer` : Direction oui, enseignant non.
    let droits_direction = direction.mes_droits().await.expect("mes droits");
    assert!(peut_gerer_roles(&droits_direction));
    let droits_prof = prof.mes_droits().await.expect("mes droits prof");
    assert!(!peut_gerer_roles(&droits_prof));
    // … et le serveur refuse de toute façon (403) : lecture comme écriture.
    assert!(matches!(prof.roles().await, Err(ApiError::Interdit(_))));
    let tableau = EditionRole::vide();
    assert!(matches!(
        prof.creer_role(&tableau.saisie()).await,
        Err(ApiError::Interdit(_) | ApiError::Invalide(_))
    ));
    // Sans session : 401 (reconnexion).
    assert_eq!(
        ApiClient::new(&base, None)
            .catalogue_roles()
            .await
            .unwrap_err(),
        ApiError::NonConnecte
    );

    // Catalogue : les actions d'administration n'admettent que l'établissement.
    let cat = direction.catalogue_roles().await.expect("catalogue");
    assert!(cat.actions.len() >= 9);
    let gerer = cat
        .actions
        .iter()
        .find(|a| a.code == "roles.gerer")
        .unwrap();
    assert_eq!(gerer.portees, ["etablissement"]);
    assert_eq!(cat.portees.len(), 4);

    // Création depuis le modèle « Accueil », avec le catalogue réel.
    let suffixe = Uuid::new_v4().simple().to_string();
    let mut e = EditionRole::depuis_modele(modele_par_id("accueil").unwrap(), &cat);
    e.nom = format!("Accueil {}", &suffixe[..8]);
    assert!(e.erreurs().is_empty());
    let cree = direction.creer_role(&e.saisie()).await.expect("création");
    assert_eq!(cree.nom, e.nom);
    assert_eq!(
        cree.droits,
        [
            droit("absences.lire", "etablissement"),
            droit("eleves.annuaire", "etablissement"),
            droit("eleves.lire", "etablissement"),
        ]
    );
    assert!(direction
        .roles()
        .await
        .unwrap()
        .iter()
        .any(|r| r.id == cree.id));

    // Modification : ajout d'un droit « mes groupes ».
    let mut e2 = EditionRole::depuis_role(&cree);
    assert!(e2.definir("notes.lire", "mes_groupes", true, &cat));
    let modifie = direction
        .modifier_role(cree.id, &e2.saisie())
        .await
        .expect("modification");
    assert_eq!(modifie.droits.len(), 4);

    // Portée non admise (roles.gerer + mes_classes) : refusé, message compréhensible.
    let mauvais = ent_web_api::SaisieRole {
        nom: "Mauvais".into(),
        description: None,
        modele: None,
        droits: vec![droit("roles.gerer", "mes_classes")],
    };
    let refus = direction.creer_role(&mauvais).await.unwrap_err();
    assert!(matches!(refus, ApiError::Invalide(_)), "{refus:?}");
    assert!(message_erreur_role(&refus).contains("portée"));
    // « Désignés » sans classe ni groupe : refusé.
    let sans_cible = ent_web_api::SaisieRole {
        nom: "Sans cible".into(),
        description: None,
        modele: None,
        droits: vec![droit("notes.lire", "designes")],
    };
    assert!(matches!(
        direction.creer_role(&sans_cible).await,
        Err(ApiError::Invalide(_))
    ));
    // Cible d'un autre établissement ou inexistante : refusée (400).
    let cible_etrangere = ent_web_api::SaisieRole {
        nom: "Cible étrangère".into(),
        description: None,
        modele: None,
        droits: vec![DroitApi {
            action: "notes.lire".into(),
            portee: "designes".into(),
            cibles: vec![Uuid::new_v4()],
        }],
    };
    assert!(matches!(
        direction.creer_role(&cible_etrangere).await,
        Err(ApiError::Invalide(_))
    ));

    // Attribution datée à la comptabilité, puis effet sur `mes-droits`.
    let fin = chrono::Utc::now().date_naive() + chrono::Duration::days(30);
    direction
        .attribuer_role(cree.id, compta_id, Some(fin))
        .await
        .expect("attribution");
    let attribue = direction
        .roles()
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.id == cree.id)
        .unwrap();
    assert_eq!(attribue.membres, [compta_id]);
    let mes = compta.mes_droits().await.unwrap();
    assert!(
        mes.iter()
            .any(|d| d.action == "notes.lire" && d.portee == "mes_groupes"),
        "effet immédiat : {mes:?}"
    );

    // Un rôle ne s'attribue pas à une personne inconnue.
    assert!(direction
        .attribuer_role(cree.id, Uuid::new_v4(), None)
        .await
        .is_err());

    // Retrait puis suppression ; un second retrait est « introuvable ».
    direction
        .retirer_role(cree.id, compta_id)
        .await
        .expect("retrait");
    assert!(matches!(
        direction.retirer_role(cree.id, compta_id).await,
        Err(ApiError::Introuvable(_))
    ));
    direction
        .supprimer_role(cree.id)
        .await
        .expect("suppression");
    assert!(!direction
        .roles()
        .await
        .unwrap()
        .iter()
        .any(|r| r.id == cree.id));
    assert!(matches!(
        direction.supprimer_role(cree.id).await,
        Err(ApiError::Introuvable(_))
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anti_escalade_explique_contre_vrai_backend() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let (direction, _) = client_du_role(&base, &l, "direction").await;
    let (compta, compta_id) = client_du_role(&base, &l, "compta").await;
    let cat = direction.catalogue_roles().await.unwrap();

    // La direction donne à la comptabilité le droit de gérer les rôles, sans plus.
    let suffixe = Uuid::new_v4().simple().to_string();
    let gestionnaire = ent_web_api::SaisieRole {
        nom: format!("Gestionnaire {}", &suffixe[..8]),
        description: None,
        modele: None,
        droits: vec![droit("roles.gerer", "etablissement")],
    };
    let g = direction
        .creer_role(&gestionnaire)
        .await
        .expect("rôle gestionnaire");
    direction
        .attribuer_role(g.id, compta_id, None)
        .await
        .expect("attribution");
    assert!(peut_gerer_roles(&compta.mes_droits().await.unwrap()));

    // Elle ne peut pas accorder un droit qu'elle n'a pas (notes.saisir) : 403 expliqué.
    let mut e = EditionRole::vide();
    e.nom = format!("Escalade {}", &suffixe[..8]);
    assert!(e.definir("notes.saisir", "etablissement", true, &cat));
    let refus = compta.creer_role(&e.saisie()).await.unwrap_err();
    assert!(matches!(refus, ApiError::Interdit(_)), "{refus:?}");
    let message = message_erreur_role(&refus);
    assert!(
        message.contains("accorder un droit que vous n'avez pas"),
        "{message}"
    );
    // … ni attribuer un rôle plus puissant que les siens.
    assert!(compta.attribuer_role(g.id, compta_id, None).await.is_ok());
    let (prof, prof_id) = client_du_role(&base, &l, "prof").await;
    let puissant = direction
        .creer_role(&ent_web_api::SaisieRole {
            nom: format!("Notes {}", &suffixe[..8]),
            description: None,
            modele: None,
            droits: vec![droit("notes.saisir", "etablissement")],
        })
        .await
        .expect("rôle puissant");
    assert!(matches!(
        compta.attribuer_role(puissant.id, prof_id, None).await,
        Err(ApiError::Interdit(_))
    ));
    drop(prof);

    // Un droit qu'elle a (roles.gerer) reste accordable.
    let mut ok = EditionRole::vide();
    ok.nom = format!("Permis {}", &suffixe[..8]);
    ok.definir("roles.gerer", "etablissement", true, &cat);
    let permis = compta
        .creer_role(&ok.saisie())
        .await
        .expect("droit couvert");

    // Nettoyage.
    for id in [permis.id, puissant.id, g.id] {
        direction.supprimer_role(id).await.expect("nettoyage");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn personnel_groupes_et_recuperation_contre_vrai_backend() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let (direction, _) = client_du_role(&base, &l, "direction").await;

    // Liste du personnel pour l'attribution : la comptabilité de labo y figure.
    let personnel = direction.personnel("").await.expect("personnel");
    assert!(personnel.iter().any(|p| p.role == "compta"));
    let filtre = direction.personnel("zzzz-inconnu").await.expect("filtre");
    assert!(filtre.is_empty());

    // Responsables d'un groupe inexistant : introuvable.
    assert!(matches!(
        direction
            .definir_responsables_groupe(Uuid::new_v4(), &[])
            .await,
        Err(ApiError::Introuvable(_))
    ));

    // Récupération de passkey : demande publique, réponse neutre, qu'il y ait un compte ou non.
    let anonyme = ApiClient::new(&base, None);
    // Adresse inconnue (unique à chaque lancement : la limitation par adresse est de 2/heure).
    let inconnue = format!("inconnue-{}@ent.lab", Uuid::new_v4().simple());
    anonyme
        .demander_recuperation_passkey(&inconnue, l.etablissement_id)
        .await
        .expect("réponse neutre (compte inconnu)");
    // Compte connu : même réponse neutre (ou limitation si l'adresse a déjà servi dans l'heure
    // sur cette base de test : jamais une réponse qui distingue les comptes).
    let connu = anonyme
        .demander_recuperation_passkey(&personnel[0].email, l.etablissement_id)
        .await;
    assert!(
        matches!(connu, Ok(()) | Err(ApiError::TropDeRequetes)),
        "{connu:?}"
    );
    // Jetons invalides : refus identiques pour les trois étapes, aucune fuite.
    let faux = "00".repeat(32);
    let r = anonyme.confirmer_recuperation_passkey(&faux).await;
    // Sans messagerie configurée (base de test), la confirmation répond 503 avant tout examen
    // du lien (fail-closed, le jeton n'est pas consommé) ; sinon 400.
    assert!(
        matches!(
            r,
            Err(ApiError::Invalide(_)) | Err(ApiError::Serveur(503, _))
        ),
        "{r:?}"
    );
    assert!(matches!(
        anonyme.annuler_recuperation_passkey(&faux).await,
        Err(ApiError::Invalide(_))
    ));
    assert!(matches!(
        anonyme.finaliser_recuperation_passkey(&faux).await,
        Err(ApiError::Invalide(_))
    ));
}
