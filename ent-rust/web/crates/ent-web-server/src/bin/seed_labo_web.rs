//! Prépare les données de labo du prototype sur un serveur local déjà démarré.
//!   seed-labo-web [http://127.0.0.1:8090]
//! Écrit l'identifiant de l'établissement de labo dans `$SCOLIA_OUTILS/labo-etab.txt`.

use ent_web_server::labo;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let base = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8090".into());
    if !(base.starts_with("http://127.0.0.1") || base.starts_with("http://localhost")) {
        eprintln!("refus : données de labo uniquement sur un serveur local");
        std::process::exit(2);
    }
    let l = labo::bootstrap(&base).await.expect("labo/bootstrap");
    let admin_email = l
        .persona("admin")
        .map(|p| p.email.clone())
        .expect("persona admin");
    let admin = labo::connecter(&base, &l, &admin_email)
        .await
        .expect("connexion admin");
    let prof = l.premier("prof").expect("au moins un professeur").clone();
    let jour = chrono::Local::now().date_naive();
    let rapport = labo::preparer_classes(&base, &admin, prof.user_id, jour)
        .await
        .expect("classes de test");

    if let Ok(dir) = std::env::var("SCOLIA_OUTILS") {
        let _ = std::fs::write(
            format!("{dir}/labo-etab.txt"),
            l.etablissement_id.to_string(),
        );
    }
    println!("établissement de labo : {}", l.etablissement_id);
    println!("professeur de labo    : {} ({})", prof.label, prof.email);
    for r in rapport {
        println!("  - {r}");
    }
    println!("mot de passe de labo  : {}", l.password);
    println!("page : {base}/web/connexion?etab={}", l.etablissement_id);
}
