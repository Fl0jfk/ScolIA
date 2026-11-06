//! Rendu serveur des pages des liens de changement d'adresse e-mail (RF-01) : aucune base ni
//! backend, seul `routeur_web` (rendu Leptos) est servi.

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
async fn pages_des_liens_de_changement_d_email() {
    let base = demarrer().await;
    let secret = "JETON-SECRET-0123456789";
    for (etape, titre, bouton, explication) in [
        (
            "confirmer",
            "Confirmer ma nouvelle adresse e-mail",
            "Confirmer cette adresse",
            "Ne confirmez pas",
        ),
        (
            "annuler",
            "Annuler le changement d&#x27;adresse e-mail",
            "Annuler le changement",
            "72 heures",
        ),
    ] {
        let page = html(&base, &format!("/email/{etape}?token={secret}")).await;
        // Le titre peut être échappé par le rendu (apostrophe) : on accepte les deux formes.
        let titre_brut = titre.replace("&#x27;", "'");
        assert!(
            page.contains(titre) || page.contains(&titre_brut),
            "{etape} : titre"
        );
        assert!(
            page.contains(bouton),
            "{etape} : action explicite par bouton"
        );
        assert!(
            page.contains(explication),
            "{etape} : effets expliqués avant d'agir"
        );
        assert!(
            page.contains(r#"name="robots" content="noindex""#),
            "{etape}"
        );
        assert!(
            !page.contains(secret),
            "{etape} : le jeton n'est jamais réécrit dans la page"
        );
        assert!(
            !page.contains("Navigation principale"),
            "{etape} : page publique"
        );
        assert!(
            page.contains(r#"id="contenu""#),
            "{etape} : cible du lien d'évitement"
        );
    }

    // Lien sans jeton, jeton vide ou contenant des espaces : message clair, aucune action.
    for chemin in [
        "/email/confirmer",
        "/email/annuler?token=",
        "/email/confirmer?token=a%20b",
    ] {
        let page = html(&base, chemin).await;
        assert!(page.contains("incomplet ou invalide"), "{chemin}");
        assert!(!page.contains("Confirmer cette adresse"), "{chemin}");
        assert!(!page.contains("Annuler le changement</"), "{chemin}");
    }
}
