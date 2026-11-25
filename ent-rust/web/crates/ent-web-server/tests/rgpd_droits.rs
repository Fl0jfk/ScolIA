//! Rendu serveur des écrans du **guichet des demandes de droits** (socle 27, lot G3a). Aucune base ni
//! backend : seul `routeur_web` (rendu Leptos) est servi ; les données (file, fiche, suivi) sont chargées
//! par le navigateur avec la session, le serveur rend la structure et les états de chargement.
//!
//! Les droits (direction, secrétariat, professeur, famille) sont éprouvés contre le vrai backend par
//! `ent-api` (`tests_rgpd_g3a_http.rs`) et par `ent-db` (`tests_rgpd_g3a.rs`).

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
async fn pages_du_guichet_dans_la_coquille() {
    let base = demarrer().await;
    let id = "6f1c3c80-0000-4000-8000-000000000027";
    for (chemin, attendu) in [
        ("/web/rgpd/demandes", "Demandes de droits"),
        ("/web/rgpd/demandes", "Chargement des demandes"),
        ("/web/rgpd/demandes/saisir", "Saisir une demande reçue"),
        ("/web/rgpd/demandes/saisir", "Chargement du guichet"),
        (
            &format!("/web/rgpd/demandes/{id}"),
            "Chargement de la demande",
        ),
        ("/web/mes-donnees", "Mes données"),
        ("/web/mes-donnees", "Chargement de vos demandes"),
        ("/web/mes-donnees/demande", "Déposer une demande"),
        ("/web/mes-donnees/demande", "Ce que vous demandez"),
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

/// La saisie d'un courrier : chaque champ est étiqueté, et l'écran dit que la date de réception
/// commande le délai légal (RGP-143, RGP-303).
#[tokio::test]
async fn saisie_d_un_courrier_etiquetee_et_explicite() {
    let base = demarrer().await;
    let page = html(&base, "/web/rgpd/demandes/saisir").await;
    for etiquette in [
        "Comment la demande est arrivée",
        "Date de réception",
        "Nom du demandeur",
        "À quel titre",
        "Personne concernée",
        "Droits exercés",
        "Périmètre demandé",
    ] {
        assert!(page.contains(etiquette), "étiquette « {etiquette} »");
    }
    assert!(
        page.contains("c'est elle qui fait courir le délai légal"),
        "la date de réception est expliquée"
    );
    assert!(
        page.contains("pas de la saisie"),
        "l'écart réception / saisie est dit"
    );
}

/// Le dépôt par la personne annonce le masquage des tiers (R-RG15, art. 15.4) et ne promet rien
/// d'autre.
#[tokio::test]
async fn depot_par_la_personne_annonce_le_masquage_des_tiers() {
    let base = demarrer().await;
    let page = html(&base, "/web/mes-donnees/demande").await;
    assert!(page.contains("Pour moi-même"));
    assert!(page.contains("Pour mon enfant"));
    for droit in ["Accès", "Rectification", "Effacement", "Qui a consulté"] {
        assert!(page.contains(droit), "droit « {droit} »");
    }
    assert!(
        page.contains("porter atteinte aux droits d'autrui"),
        "le masquage des tiers est annoncé"
    );
    assert!(
        page.contains("délai d'un mois"),
        "le délai légal est annoncé"
    );
}

/// **R-RG32 et E14** : le HTML rendu par le serveur ne porte aucune donnée d'établissement. Les écrans du
/// guichet ne sont que structure et squelettes avant l'hydratation.
#[tokio::test]
async fn aucune_donnee_d_etablissement_dans_le_html_du_serveur() {
    let base = demarrer().await;
    for chemin in [
        "/web/rgpd/demandes",
        "/web/rgpd/demandes/saisir",
        "/web/mes-donnees",
        "/web/mes-donnees/demande",
    ] {
        let page = html(&base, chemin).await;
        // Aucune référence de demande, aucun nom, aucun motif : tout vient du navigateur.
        assert!(!page.contains("2027-0"), "{chemin} : référence de demande");
        assert!(!page.contains("Granger"), "{chemin} : nom de famille");
        assert!(
            !page.contains("rgpd.demande"),
            "{chemin} : code interne de conservation"
        );
    }
}
