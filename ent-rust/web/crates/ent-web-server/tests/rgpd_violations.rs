//! Rendu serveur des écrans du **registre des violations** (socle 27, palier G5). Aucune base ni backend : seul
//! `routeur_web` est servi ; les données (registre, fiche, mes signalements) sont chargées par le navigateur avec
//! la session, le serveur rend la structure et les états de chargement.
//!
//! Les droits (direction, professeur, famille) sont éprouvés contre le vrai backend par `ent-api`
//! (`tests_rgpd_g5_http.rs`) et par `ent-db` (`tests_rgpd_g5.rs`).

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
async fn pages_du_registre_dans_la_coquille() {
    let base = demarrer().await;
    let id = "6f1c3c80-0000-4000-8000-000000000547";
    for (chemin, attendu) in [
        ("/web/incident", "Signaler un incident de données"),
        ("/web/incident", "Chargement de vos signalements"),
        ("/web/rgpd/violations", "Registre des violations"),
        ("/web/rgpd/violations", "Chargement du registre"),
        (
            "/web/rgpd/violations/saisir",
            "Saisir une violation signalée",
        ),
        (
            &format!("/web/rgpd/violations/{id}"),
            "Chargement de la violation",
        ),
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
}

/// RGP-240 : la déclaration tient en un écran, ses champs sont étiquetés, et elle dit qu'on ne reproche rien à
/// celui qui signale (CL27-45).
#[tokio::test]
async fn declaration_en_un_ecran_sans_reproche() {
    let base = demarrer().await;
    let page = html(&base, "/web/incident").await;
    for etiquette in [
        "En une phrase",
        "Précisions (facultatif)",
        "Depuis quand",
        "Aucune donnée personnelle n'est touchée",
    ] {
        assert!(page.contains(etiquette), "étiquette « {etiquette} »");
    }
    assert!(page.contains("Personne ne vous le reprochera"));
    assert!(
        page.contains("N&#x27;y recopiez pas les données")
            || page.contains("N'y recopiez pas les données")
    );
}

/// R-RG32 et E14 : le HTML rendu par le serveur ne porte aucune donnée d'établissement.
#[tokio::test]
async fn aucune_donnee_d_etablissement_dans_le_html_du_serveur() {
    let base = demarrer().await;
    for chemin in [
        "/web/incident",
        "/web/rgpd/violations",
        "/web/rgpd/violations/saisir",
    ] {
        let page = html(&base, chemin).await;
        assert!(
            !page.contains("V-2027-"),
            "{chemin} : référence de violation"
        );
        assert!(!page.contains("rgpd.violation"), "{chemin} : code interne");
    }
}
