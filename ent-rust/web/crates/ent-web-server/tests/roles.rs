//! Rendu serveur des écrans « Rôles et droits » (D-017) et de la récupération de passkey
//! (D-018). Aucune base ni backend : seul `routeur_web` (rendu Leptos) est servi ; les données
//! sont chargées par le navigateur, le serveur rend la structure et les états de chargement.

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
async fn pages_d_administration_des_roles_dans_la_coquille() {
    let base = demarrer().await;
    let id = "6f1c3c80-0000-4000-8000-000000000001";
    for (chemin, attendu) in [
        ("/web/direction/roles", "Rôles et droits"),
        ("/web/direction/roles", "Nouveau rôle"),
        ("/web/direction/roles", "Responsables des groupes"),
        ("/web/direction/roles/nouveau", "Nouveau rôle"),
        ("/web/direction/roles/groupes", "Vérification de vos droits"),
        (
            &format!("/web/direction/roles/{id}"),
            "Vérification de vos droits",
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
        assert!(
            page.contains(r#"id="contenu""#) || page.contains("contenu"),
            "{chemin}"
        );
    }
    // Aucune donnée d'établissement dans le HTML rendu par le serveur : la garde attend
    // `/roles/mes-droits`, rien n'est affiché avant.
    let page = html(&base, "/web/direction/roles").await;
    assert!(!page.contains("Modifier le rôle"));
}

#[tokio::test]
async fn pages_de_recuperation_de_passkey() {
    let base = demarrer().await;

    // Demande publique : hors coquille, formulaire étiqueté, explication du délai.
    let page = html(&base, "/web/recuperation-passkey").await;
    assert!(page.contains("perdu ma passkey"));
    assert!(page.contains("Adresse e-mail"));
    assert!(page.contains(r#"for="recup-email""#));
    assert!(page.contains(r#"type="email""#));
    assert!(
        page.contains("délai de sécurité"),
        "délai expliqué avant d'agir"
    );
    assert!(page.contains("Envoyer le lien"));
    assert!(page.contains(r#"name="robots" content="noindex""#));
    assert!(!page.contains("Navigation principale"), "page publique");

    // Liens des e-mails : adresses exactes envoyées par ent-api (sans /web).
    let secret = "JETON-SECRET-0123456789";
    for (etape, titre, bouton) in [
        (
            "confirmer",
            "Confirmer la récupération",
            "Confirmer ma demande",
        ),
        (
            "annuler",
            "Annuler la récupération",
            "Annuler la récupération",
        ),
        (
            "finaliser",
            "Finaliser la récupération",
            "Supprimer mes passkeys",
        ),
    ] {
        let page = html(
            &base,
            &format!("/recuperation-passkey/{etape}?token={secret}"),
        )
        .await;
        assert!(page.contains(titre), "{etape} : titre");
        assert!(
            page.contains(bouton),
            "{etape} : action explicite par bouton"
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
    }
    let page = html(&base, "/recuperation-passkey/confirmer?token=x").await;
    assert!(
        page.contains("délai de sécurité"),
        "le délai est expliqué avant la confirmation"
    );

    // Lien sans jeton : message d'erreur, aucune action possible.
    let page = html(&base, "/recuperation-passkey/finaliser").await;
    assert!(page.contains("incomplet ou invalide"));
    assert!(!page.contains("Supprimer mes passkeys"));
}

#[tokio::test]
async fn la_connexion_propose_la_recuperation() {
    let base = demarrer().await;
    let page = html(&base, "/web/connexion").await;
    assert!(
        page.contains("perdu ma passkey"),
        "lien sur l'écran de connexion"
    );
    assert!(page.contains(r#"href="/web/recuperation-passkey""#));
}
