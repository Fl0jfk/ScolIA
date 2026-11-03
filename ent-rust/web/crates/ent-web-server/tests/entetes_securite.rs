//! En-têtes de sécurité des pages `/web` : CSP stricte avec nonce par requête.
//! Aucune base de données nécessaire : seul `routeur_web` (rendu Leptos) est servi.

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

/// Renvoie (nonce de l'en-tête CSP, en-tête CSP complet, corps HTML).
async fn page(base: &str) -> (String, String, String) {
    let rep = reqwest::get(format!("{base}/web/connexion")).await.unwrap();
    assert!(rep.status().is_success());
    let csp = rep
        .headers()
        .get("content-security-policy")
        .expect("CSP absente sur /web/connexion")
        .to_str()
        .unwrap()
        .to_string();
    let corps = rep.text().await.unwrap();
    let nonce = csp
        .split("'nonce-")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .expect("nonce absent de la CSP")
        .to_string();
    (nonce, csp, corps)
}

#[tokio::test]
async fn csp_stricte_et_nonce_unique_par_requete() {
    let base = demarrer().await;
    let (n1, csp, corps) = page(&base).await;
    let (n2, _, corps2) = page(&base).await;

    for directive in [
        "default-src 'self'",
        "script-src 'self' 'nonce-",
        "'wasm-unsafe-eval'",
        "style-src 'self';",
        "img-src 'self' data:",
        "connect-src 'self'",
        "object-src 'none'",
        "base-uri 'none'",
        "frame-ancestors 'self'",
        "form-action 'self'",
    ] {
        assert!(
            csp.contains(directive),
            "directive absente : {directive} dans {csp}"
        );
    }
    assert!(
        !csp.contains("unsafe-inline"),
        "unsafe-inline interdit : {csp}"
    );

    assert!(n1.len() >= 16 && n2.len() >= 16, "nonce trop court");
    assert_ne!(n1, n2, "le nonce doit changer à chaque requête");

    // Le nonce de l'en-tête est celui des scripts d'amorçage ; aucun script en ligne sans nonce.
    assert!(
        corps.contains(&format!("nonce=\"{n1}\"")),
        "nonce absent des balises script"
    );
    assert!(corps2.contains(&format!("nonce=\"{n2}\"")));
    assert!(
        !corps.contains(&format!("nonce=\"{n2}\"")),
        "nonce d'une autre requête dans la page"
    );
    for balise in corps.split("<script").skip(1) {
        let ouverture = balise.split('>').next().unwrap();
        if !ouverture.contains("src=") {
            assert!(
                ouverture.contains("nonce="),
                "script en ligne sans nonce : {ouverture}"
            );
        }
    }
}

#[tokio::test]
async fn autres_entetes_de_securite_presents() {
    let base = demarrer().await;
    let rep = reqwest::get(format!("{base}/web/connexion")).await.unwrap();
    let h = rep.headers();
    assert_eq!(h["x-content-type-options"], "nosniff");
    assert_eq!(h["x-frame-options"], "SAMEORIGIN");
    assert_eq!(h["referrer-policy"], "same-origin");
}
