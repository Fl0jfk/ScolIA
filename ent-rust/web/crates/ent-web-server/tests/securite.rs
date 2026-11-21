//! Rendu serveur des écrans « Sécurité du compte » (socle 1). Aucune base ni backend :
//! seul `routeur_web` (rendu Leptos) est servi ; les données de la session sont chargées
//! par le navigateur, le serveur rend la structure et les états de chargement.

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
    assert!(
        rep.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("script-src 'self' 'nonce-"),
        "{chemin} : CSP à nonce"
    );
    rep.text().await.unwrap()
}

#[tokio::test]
async fn ecran_d_enrolement_rendu_sans_navigation_ni_lien_de_sortie() {
    let base = demarrer().await;
    let page = html(&base, "/web/securite/enrolement").await;
    assert!(page.contains(r#"<html lang="fr""#));
    assert!(page.contains("Enregistrez votre passkey"), "titre");
    assert!(
        page.contains("Vérification de votre compte"),
        "état de chargement"
    );
    assert!(page.contains("Se déconnecter"), "seule sortie");
    assert!(page.contains(r#"id="contenu""#));
    // Écran plein : ni barre de navigation de la coquille, ni lien vers d'autres pages métier.
    assert!(!page.contains("Navigation principale"), "pas de coquille");
    assert!(!page.contains("Aller au contenu"), "pas de coquille");
    for lien in [
        "/web/appel",
        "/web/messages",
        "/web/classes",
        "/web/securite\"",
    ] {
        assert!(
            !page.contains(&format!("href=\"{lien}")),
            "lien interdit : {lien}"
        );
    }
    assert!(page.contains("/pkg/ent-web.js"));
}

#[tokio::test]
async fn pages_du_compte_rendues_dans_la_coquille() {
    let base = demarrer().await;
    for (chemin, attendu) in [
        ("/web/securite", "Sécurité du compte"),
        ("/web/securite", "Mes passkeys"),
        ("/web/securite", "Mes appareils"),
        ("/web/securite", "Niveau de cette session"),
        ("/web/direction/securite", "Politique de sécurité"),
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
    // Lien « Compte » dans la navigation, pour tous les profils.
    let page = html(&base, "/web/securite").await;
    assert!(page.contains(r#"href="/web/securite""#), "lien Compte");
    // Formulaires accessibles : champs étiquetés, mot de passe en champ masqué.
    assert!(page.contains("Mot de passe actuel"));
    assert!(page.contains(r#"type="password""#));
    assert!(page.contains(r#"autocomplete="current-password""#));
    assert!(page.contains(r#"for="ajout-mdp""#));
    // Aucun style ni script en ligne (CSP stricte).
    assert!(!page.contains(" style=\""), "style en ligne interdit");
}
