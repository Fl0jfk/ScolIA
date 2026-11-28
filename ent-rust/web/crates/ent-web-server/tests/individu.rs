//! Fiches reliées (transverse « individu ») contre le **vrai** backend (`ent_api::app`) et une base
//! PostgreSQL **locale** de test : pages rendues par le serveur, parcours de la direction et du
//! secrétariat (chercher, relier, lever), « Mes autres comptes » (déclaration neutre, demande reçue,
//! refus), permissions. Un établissement neuf par test : aucune écriture dans les données de labo.
//!
//! Lancement :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test individu -- --test-threads=1
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »). Aucun envoi réel.
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_types::dossier::PersonneBody;
use ent_types::individu::RelierFichesBody;
use ent_web_api::{ApiClient, ApiError, Connexion};
use ent_web_app::modules::individu::modele::{
    fiches_choisissables, libelle_origine, lien_a_envoyer, MESSAGE_DECLARATION,
};
use uuid::Uuid;

const MDP: &str = "DevLocalPass1!";

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

/// Un compte neuf de l'établissement `etab`, et son adresse.
async fn inscrire(base: &str, etab: Uuid, role: &str) -> (ApiClient, String) {
    let email = format!("ind-{role}-{}@ent.local", Uuid::new_v4().simple());
    ApiClient::new(base, None)
        .post_json(
            "/auth/register",
            serde_json::json!({"email": email, "password": MDP, "etablissement_id": etab, "role": role}),
        )
        .await
        .unwrap_or_else(|e| panic!("inscription {role} : {e:?}"));
    match ApiClient::new(base, None)
        .connexion(&email, MDP, etab)
        .await
        .unwrap_or_else(|e| panic!("connexion {role} : {e:?}"))
    {
        Connexion::Ouverte(s) => (ApiClient::new(base, Some(s.token)), email),
        Connexion::PasskeyRequise(_) => panic!("passkey requise pour {role}"),
    }
}

fn fiche(nom: &str) -> PersonneBody {
    PersonneBody {
        nom: nom.into(),
        prenom: Some("Individu".into()),
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pages_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    for (chemin, titre) in [
        ("/web/securite/mes-comptes", "Mes autres comptes"),
        ("/web/direction/fiches-reliees", "Fiches reliées"),
    ] {
        let r = reqwest::get(format!("{base}{chemin}")).await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(
            r.headers().contains_key("content-security-policy"),
            "CSP {chemin}"
        );
        let html = r.text().await.unwrap();
        assert!(html.contains(titre), "{chemin} : titre « {titre} » absent");
        assert!(
            !html.contains(" style=\""),
            "{chemin} : aucun style en ligne (CSP)"
        );
    }
}

/// Le secrétariat cherche et relie deux fiches, la direction lève ; un professeur n'a rien de tout cela.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relier_et_lever_selon_les_droits() {
    let Some(base) = demarrer().await else { return };
    let etab = Uuid::new_v4();
    let (dir, _) = inscrire(&base, etab, "direction").await;
    let (sec, _) = inscrire(&base, etab, "administratif").await;
    let (prof, _) = inscrire(&base, etab, "prof").await;
    let nom = format!("Relie{}", &Uuid::new_v4().simple().to_string()[..6]);
    let a = sec.creer_personne(&fiche(&nom)).await.expect("fiche a");
    let b = sec.creer_personne(&fiche(&nom)).await.expect("fiche b");

    // Chercher : les deux homonymes sont trouvés, l'écran ne propose pas deux fois la même.
    let trouvees = sec.personnes_rechercher(&nom).await.expect("recherche");
    assert_eq!(trouvees.len(), 2, "{trouvees:?}");
    let choix_a = fiches_choisissables(&trouvees, None)
        .into_iter()
        .find(|f| f.id == a.id)
        .unwrap();
    let choix_b = fiches_choisissables(&trouvees, Some(a.id));
    assert_eq!(choix_b.len(), 1);
    assert_eq!(choix_b[0].id, b.id);
    let (pa, pb, motif) =
        lien_a_envoyer(Some(&choix_a), Some(&choix_b[0]), "fiche_sans_compte").unwrap();

    // Un professeur ne lit ni ne relie.
    assert!(matches!(
        prof.individu_liens().await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        prof.individu_propositions().await,
        Err(ApiError::Interdit(_))
    ));
    let corps = RelierFichesBody {
        personne_a: pa,
        personne_b: pb,
        motif_code: motif,
        depuis_proposition: false,
    };
    assert!(matches!(
        prof.individu_relier(&corps).await,
        Err(ApiError::Interdit(_))
    ));

    // Le secrétariat relie ; relier deux fois est refusé et expliqué.
    let lien = sec.individu_relier(&corps).await.expect("relier");
    let e = sec.individu_relier(&corps).await.unwrap_err();
    assert!(
        e.message_utilisateur().contains("déjà reliées"),
        "{}",
        e.message_utilisateur()
    );
    let liens = dir.individu_liens().await.expect("liens");
    assert_eq!(liens.len(), 1);
    assert_eq!(liens[0].id, lien);
    assert_ne!(libelle_origine(&liens[0].origine), "Lien enregistré");
    assert!(sec
        .individu_propositions()
        .await
        .expect("propositions")
        .is_empty());

    // Le secrétariat ne lève pas ; la direction lève (motif de la liste fermée).
    assert!(matches!(
        sec.individu_lever(lien, "erreur").await,
        Err(ApiError::Interdit(_))
    ));
    dir.individu_lever(lien, "erreur").await.expect("lever");
    assert!(dir.individu_liens().await.unwrap().is_empty());
    assert!(matches!(
        dir.individu_lever(lien, "erreur").await,
        Err(ApiError::Conflit(_)) | Err(ApiError::Invalide(_))
    ));
}

/// « Mes autres comptes » : réponse neutre, demande vue du seul compte cible, refus sans lien.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mes_autres_comptes_declaration_et_refus() {
    let Some(base) = demarrer().await else { return };
    let etab = Uuid::new_v4();
    let (prof, _) = inscrire(&base, etab, "prof").await;
    let (parent, email_parent) = inscrire(&base, etab, "parent").await;

    let moi = prof.individu_moi().await.expect("moi");
    assert_eq!(moi.autres_comptes, 0);
    assert!(moi.liens.is_empty() && moi.declarations_recues.is_empty());

    // Adresse inconnue et adresse connue : même réponse (l'écran affiche la même phrase).
    prof.individu_declarer("inconnu@nulle-part.example")
        .await
        .expect("adresse inconnue : réponse neutre");
    prof.individu_declarer(&email_parent)
        .await
        .expect("adresse connue");
    assert!(!MESSAGE_DECLARATION.is_empty());

    let recue = parent.individu_moi().await.expect("moi parent");
    assert_eq!(recue.declarations_recues.len(), 1);
    assert!(prof
        .individu_moi()
        .await
        .unwrap()
        .declarations_recues
        .is_empty());
    let d = recue.declarations_recues[0].id;
    // Le déclarant ne répond pas à sa propre demande.
    assert!(prof.individu_repondre(d, true).await.is_err());
    parent.individu_repondre(d, false).await.expect("refus");
    let apres = parent.individu_moi().await.unwrap();
    assert!(apres.declarations_recues.is_empty());
    assert_eq!(apres.autres_comptes, 0, "un refus ne relie rien");
}
