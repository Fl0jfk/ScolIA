//! Rendu serveur des écrans « Sorties et voyages » (socle 42, lot V0a). Aucune base ni backend :
//! seul `routeur_web` (rendu Leptos) est servi ; les données (liste, détail, droits) sont chargées
//! par le navigateur avec la session, le serveur rend la structure et les états de chargement.
//! Les droits (porteur, direction, famille, autre établissement) sont éprouvés contre le vrai
//! backend par `ent-api` (`tests_voyages.rs`).

use ent_web_server::{options_leptos, routeur_web};

async fn demarrer() -> String {
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = ecoute.local_addr().unwrap();
    let app = routeur_web(options_leptos());
    tokio::spawn(async move {
        axum::serve(ecoute, app).await.unwrap();
    });
    format!("http://{addr}")
}

async fn html(base: &str, chemin: &str) -> String {
    let rep = reqwest::get(format!("{base}{chemin}")).await.unwrap();
    assert_eq!(rep.status(), 200, "{chemin}");
    let csp = rep.headers()["content-security-policy"].to_str().unwrap();
    assert!(
        csp.contains("script-src 'self' 'nonce-"),
        "{chemin} : CSP à nonce"
    );
    assert!(
        !csp.contains("'unsafe-inline'"),
        "{chemin} : pas de script en ligne"
    );
    assert_eq!(rep.headers()["cache-control"], "no-store", "{chemin}");
    let page = rep.text().await.unwrap();
    assert!(page.contains(r#"<html lang="fr""#), "{chemin}");
    assert!(
        !page.contains(" style=\""),
        "{chemin} : style en ligne interdit"
    );
    page
}

#[tokio::test]
async fn pages_des_sorties_dans_la_coquille() {
    let base = demarrer().await;
    let id = "6f1c3c80-0000-4000-8000-000000000042";
    for (chemin, attendu) in [
        ("/web/voyages", "Sorties et voyages"),
        ("/web/voyages", "Chargement des sorties"),
        ("/web/voyages/nouveau", "Nouvelle sortie"),
        ("/web/voyages/nouveau", "Créer le brouillon"),
        ("/web/voyages/nouveau", "Classes concernées"),
        (&format!("/web/voyages/{id}"), "Chargement de la sortie"),
    ] {
        let page = html(&base, chemin).await;
        assert!(page.contains(attendu), "{chemin} : {attendu}");
        assert!(
            page.contains("Aller au contenu"),
            "{chemin} : lien d'évitement"
        );
        assert!(
            page.contains("Navigation principale"),
            "{chemin} : coquille"
        );
    }
    // Chaque champ du formulaire de création est étiqueté.
    let page = html(&base, "/web/voyages/nouveau").await;
    for etiquette in [
        "Titre",
        "Nature",
        "Forme",
        "Destination",
        "Date de départ",
        "Heure de retour",
    ] {
        assert!(page.contains(etiquette), "étiquette « {etiquette} »");
    }
    // Aucune donnée d'établissement dans le HTML du serveur : ni sortie, ni élève, ni circulaire.
    let page = html(&base, "/web/voyages").await;
    assert!(!page.contains("Circulaire aux familles"));
    assert!(!page.contains("élève(s) inscrit(s)"));
}
