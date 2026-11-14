//! Finances (S2-9) contre le **vrai** backend (`ent_api::app`) et une base PostgreSQL
//! **locale** de test (données de labo fictives) : rendu serveur des pages, paramétrage,
//! aperçu de génération, permissions, espace famille (IBAN masqué, paiement en ligne
//! désactivé). Aucune écriture sur l'année courante : année de test lointaine.
//!
//! Lancement :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test finances -- --test-threads=1
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_web_api::types_finances::{
    ArticleBody, ExportPrelevementsBody, GenerationBody, PreparationBody, TarifBody,
};
use ent_web_api::{ApiClient, ApiError};
use ent_web_app::modules::finances::modele::{centimes_depuis_saisie, euros};
use ent_web_server::labo::{self, Labo};

const ANNEE: &str = "2039-2040";

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

async fn personnel(base: &str, l: &Labo, role: &str) -> ApiClient {
    let p = l
        .personnel
        .iter()
        .find(|p| p.role == role)
        .unwrap_or_else(|| panic!("personnel {role} de labo"));
    client_de(base, l, &p.email).await
}

async fn parent_de(base: &str, l: &Labo, prenom: &str) -> ApiClient {
    let p = l
        .personas
        .iter()
        .find(|p| p.role == "parent" && p.label.contains(prenom))
        .unwrap_or_else(|| panic!("parent de {prenom}"));
    client_de(base, l, &p.email).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pages_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    let http = reqwest::Client::new();
    for (chemin, attendu) in [
        ("/web/comptabilite", "Comptabilité"),
        ("/web/mes-factures", "Factures et paiements"),
    ] {
        let r = http.get(format!("{base}{chemin}")).send().await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(
            r.headers().contains_key("content-security-policy"),
            "CSP {chemin}"
        );
        let html = r.text().await.unwrap();
        assert!(html.contains(attendu), "{chemin} : « {attendu} » attendu");
        assert!(
            !html.contains(" style=\""),
            "aucun style en ligne (CSP) {chemin}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parcours_comptabilite_et_famille_contre_le_backend() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let compta = personnel(&base, &l, "compta").await;
    let prof = personnel(&base, &l, "prof").await;
    let lily = parent_de(&base, &l, "Harry").await;

    // Paramètres : lisibles par la comptabilité, refusés au professeur.
    let p = compta.parametres_finances().await.expect("paramètres");
    assert!(p.jours_prenotification >= 1);
    assert!(matches!(
        prof.parametres_finances().await,
        Err(ApiError::Interdit(_))
    ));

    // Article et tarif sur une année lointaine (code unique à chaque exécution).
    let suffixe = chrono::Utc::now().timestamp_micros() % 1_000_000;
    let article = compta
        .creer_article(&ArticleBody {
            code: format!("T{suffixe}"),
            libelle: "Scolarité (test)".into(),
            categorie: "scolarite".into(),
            compte_produit: None,
            actif: None,
        })
        .await
        .expect("article");
    let montant = centimes_depuis_saisie("1 234,56").unwrap();
    let tarif = compta
        .creer_tarif(&TarifBody {
            annee_scolaire: ANNEE.into(),
            article_id: article.id,
            niveau: None,
            regime: None,
            qf_min_centimes: None,
            qf_max_centimes: None,
            montant_annuel_centimes: montant,
        })
        .await
        .expect("tarif");
    assert_eq!(
        euros(tarif.montant_annuel_centimes),
        "1\u{202F}234,56\u{a0}€"
    );
    assert!(compta
        .tarifs(ANNEE)
        .await
        .unwrap()
        .iter()
        .any(|t| t.id == tarif.id));

    // Aperçu de génération : rien d'écrit.
    let r = compta
        .generer_annee(&GenerationBody {
            annee_scolaire: ANNEE.into(),
            apercu: true,
        })
        .await
        .expect("aperçu");
    assert!(r.apercu);

    // Listes : comptabilité oui, professeur et famille non.
    compta.factures_finances(None).await.expect("factures");
    compta.prelevements(None).await.expect("prélèvements");
    compta.relances(None).await.expect("relances");
    compta.exports_comptables().await.expect("exports");
    assert!(matches!(
        prof.factures_finances(None).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        lily.factures_finances(None).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        lily.emettre_factures(vec![article.id]).await,
        Err(ApiError::Interdit(_))
    ));

    // Préparation trop proche : refusée (délai de pré-notification) ; fichier CSV
    // sans prélèvement pré-notifié : conflit.
    let demain = ent_db::jour_paris() + chrono::Duration::days(1);
    assert!(matches!(
        compta
            .preparer_prelevements(&PreparationBody {
                date_execution: demain
            })
            .await,
        Err(ApiError::Invalide(_))
    ));
    let lointaine = chrono::NaiveDate::from_ymd_opt(2039, 9, 30).unwrap();
    assert!(matches!(
        compta
            .exporter_prelevements(&ExportPrelevementsBody {
                date_execution: lointaine,
                format: Some("csv".into()),
            })
            .await,
        Err(ApiError::Conflit(_))
    ));

    // Espace famille : pièces émises seulement, IBAN masqués, paiement en ligne désactivé.
    let fam = lily.finances_famille().await.expect("espace famille");
    for c in &fam.comptes {
        assert!(c.factures.iter().all(|f| f.statut != "brouillon"));
        assert!(c.mandats.iter().all(|m| m.iban_masque.contains("••••")));
    }
    let en_ligne = fam.paiement_en_ligne.expect("état du paiement en ligne");
    assert!(!en_ligne.actif);
}
