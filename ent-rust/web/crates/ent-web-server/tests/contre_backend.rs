//! Tests du prototype contre le **vrai** backend (`ent_api::app`) et une base
//! PostgreSQL **locale** de test, avec les données de labo fictives.
//!
//! Lancement (voir docs/LEPTOS_CALL_PROTOTYPE.md) :
//!   . ent-rust/web/scripts/env.sh && bash ent-rust/web/scripts/pg.sh start
//!   cd ent-rust/web && cargo test -p ent-web-server --test contre_backend -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).

use std::net::SocketAddr;
use std::time::Instant;

use chrono::{Datelike, Duration, NaiveDate};
use ent_web_api::{ApiClient, ApiError, AppelApi, CAPACITES_API_ACTUELLE};
use ent_web_app::appel::modele::{changements, visibles, Feuille, Filtre, Saisie, Statut};
use ent_web_server::labo::{self, Labo};

async fn demarrer() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent (sourcer scripts/env.sh)");
        return None;
    };
    assert!(url.contains("@127.0.0.1:"), "base de test locale uniquement");
    let (app, _) = ent_web_server::routeur_complet().await.expect("démarrage ent-api + web");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap();
    });
    Some(format!("http://{addr}"))
}

/// Un jour futur, même jour de semaine qu'aujourd'hui, différent à chaque exécution :
/// chaque test ouvre des appels neufs (un appel clos ne peut plus être modifié).
fn jour_neuf() -> NaiveDate {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;
    chrono::Local::now().date_naive() + Duration::weeks(1 + n.rem_euclid(2000))
}

async fn prof(base: &str, l: &Labo, index: usize) -> ApiClient {
    let p = l.personnel.iter().filter(|p| p.role == "prof").nth(index).expect("prof de labo");
    let s = labo::connecter(base, l, &p.email).await.expect("connexion prof");
    ApiClient::new(base, Some(s.token))
}

async fn creneau_4b(api: &ApiClient, jour: NaiveDate) -> ent_web_api::Creneau {
    api.mes_creneaux(jour)
        .await
        .expect("mes créneaux")
        .into_iter()
        .find(|c| c.classe == "4B")
        .expect("séance 4B du professeur de labo")
}

/// Refus métier d'ent-api : 400, ou 404 quand son message contient « introuvable ».
fn refus_metier<T>(r: &Result<T, ApiError>) -> bool {
    matches!(r, Err(ApiError::Invalide(_)) | Err(ApiError::Introuvable(_)))
}

fn saisies_initiales(f: &Feuille) -> Vec<Saisie> {
    f.eleves
        .iter()
        .map(|e| Saisie { eleve_id: e.eleve_id, statut: e.statut_enregistre, commentaire: String::new() })
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pages_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    let http = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap();

    let r = http.get(format!("{base}/web/appel")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers()["x-frame-options"], "SAMEORIGIN");
    assert_eq!(r.headers()["x-content-type-options"], "nosniff");
    let html = r.text().await.unwrap();
    assert!(html.contains(r#"<html lang="fr""#), "langue de la page");
    assert!(html.contains("Faire l&#x27;appel") || html.contains("Faire l'appel"), "titre");
    assert!(html.contains("Chargement des séances"), "état de chargement rendu côté serveur");
    assert!(html.contains("/pkg/ent-web.js"), "script d'hydratation");
    assert!(html.contains("/pkg/ent-web.css"), "feuille de style");
    assert!(html.contains("Aller au contenu"), "lien d'évitement");
    assert!(html.contains(r#"for="jour""#), "champ date étiqueté");

    let html = http.get(format!("{base}/web/connexion")).send().await.unwrap().text().await.unwrap();
    for attendu in ["Adresse e-mail", "Mot de passe", "Établissement", r#"autocomplete="current-password""#] {
        assert!(html.contains(attendu), "page de connexion : {attendu}");
    }
    assert!(html.contains("Environnement de test"), "fonction serveur exécutée au rendu");

    let r = http.get(format!("{base}/web")).send().await.unwrap();
    assert!(r.status().is_redirection() || r.status() == 200, "redirection /web → /web/appel ({})", r.status());

    let r = http.get(format!("{base}/web/inconnue")).send().await.unwrap();
    assert_eq!(r.status(), 404);

    // L'API existante répond toujours sur la même origine.
    let r = http.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    // L'ancienne interface est toujours servie, inchangée.
    let r = http.get(format!("{base}/app")).send().await.unwrap();
    assert_eq!(r.status(), 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn appel_complet_contre_vrai_backend() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");

    // Erreurs d'authentification.
    let anonyme = ApiClient::new(&base, None);
    assert_eq!(anonyme.moi().await.unwrap_err(), ApiError::NonConnecte);
    let p0 = l.personnel.iter().find(|p| p.role == "prof").unwrap();
    let r = anonyme.connexion(&p0.email, "mauvais-mot-de-passe", l.etablissement_id).await;
    assert_eq!(r.unwrap_err(), ApiError::NonConnecte);

    let api = prof(&base, &l, 0).await;
    let moi = api.moi().await.unwrap();
    assert_eq!(moi.role, "prof");
    assert_eq!(moi.etablissement_id, l.etablissement_id);

    let jour = jour_neuf();
    let c = creneau_4b(&api, jour).await;
    let appel: AppelApi = api.ouvrir_appel(c.id, jour).await.expect("ouverture");
    assert!(!appel.est_clos());
    assert!(!appel.lignes.is_empty(), "élèves de 4B");
    assert!(appel.lignes.iter().all(|l| l.etat == "present"), "présent par défaut");
    assert!(appel.lignes.iter().all(|l| l.eleve_label.is_some()), "noms fournis");

    // Ré-ouvrir renvoie le même appel (idempotent).
    assert_eq!(api.ouvrir_appel(c.id, jour).await.unwrap().id, appel.id);

    // Mauvais jour de semaine : refus métier. ent-api renvoie 404 « appel déjà clos
    // ou introuvable » (map_db classe le message par mot-clé) : constat documenté.
    let r = api.ouvrir_appel(c.id, jour + Duration::days(1)).await;
    assert!(refus_metier(&r), "{r:?}");
    println!("ouverture un mauvais jour : {:?}", r.unwrap_err());

    // Saisie dans le modèle d'écran, puis envoi des seuls changements.
    let f = Feuille::depuis_api(&appel);
    let mut s = saisies_initiales(&f);
    s[0].statut = Statut::Absent;
    if s.len() > 1 {
        s[1].statut = Statut::Retard;
    }
    let liste = changements(&f.eleves, &s, CAPACITES_API_ACTUELLE).unwrap();
    assert_eq!(liste.len(), s.len().min(2));
    for ch in &liste {
        api.marquer(appel.id, ch.eleve_id, ch.etat).await.expect("marquer");
    }

    // « dispensé » : refusé par l'API actuelle (écart documenté).
    let r = api
        .post_json(
            &format!("/appels/{}/lignes", appel.id),
            serde_json::json!({ "eleve_id": f.eleves[0].eleve_id, "etat": "dispense" }),
        )
        .await;
    assert!(matches!(r, Err(ApiError::Invalide(_))), "{r:?}");

    let relu = api.appel_par_id(appel.id).await.unwrap();
    let relu = Feuille::depuis_api(&relu);
    assert_eq!(relu.eleves[0].statut_enregistre, Statut::Absent);

    let clos = api.clore(appel.id).await.expect("clôture");
    assert!(clos.est_clos());
    // Après clôture : plus de modification possible.
    let r = api.marquer(appel.id, f.eleves[0].eleve_id, ent_web_api::EtatApi::Present).await;
    assert!(refus_metier(&r), "{r:?}");
    println!("marquer après clôture : {:?}", r.unwrap_err());
    let r = api.clore(appel.id).await;
    assert!(refus_metier(&r), "{r:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn permissions_respectees() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let jour = jour_neuf();
    let rogue = prof(&base, &l, 0).await;
    let c = creneau_4b(&rogue, jour).await;

    // Famille : ni EDT d'appel ni ouverture.
    let parent = l.persona("parent").expect("persona parent");
    let sp = labo::connecter(&base, &l, &parent.email).await.unwrap();
    let api_parent = ApiClient::new(&base, Some(sp.token));
    assert!(matches!(api_parent.mes_creneaux(jour).await, Err(ApiError::Interdit(_))));
    assert!(matches!(api_parent.ouvrir_appel(c.id, jour).await, Err(ApiError::Interdit(_))));

    // Autre professeur : pas l'appel d'une séance dont il n'est ni titulaire ni remplaçant.
    let autre = prof(&base, &l, 1).await;
    assert!(matches!(autre.ouvrir_appel(c.id, jour).await, Err(ApiError::Interdit(_))));

    // CPE : peut ouvrir l'appel d'une séance (remplacement / vie scolaire).
    if let Some(cpe) = l.premier("cpe") {
        let s = labo::connecter(&base, &l, &cpe.email).await.unwrap();
        let api_cpe = ApiClient::new(&base, Some(s.token));
        let r = api_cpe.ouvrir_appel(c.id, jour).await;
        println!("CPE ouvre l'appel de la 4B : {}", if r.is_ok() { "autorisé" } else { "refusé" });
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn classes_nombreuses_38_et_400() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let admin = labo::connecter(&base, &l, &l.persona("admin").unwrap().email).await.unwrap();
    let p = l.personnel.iter().find(|p| p.role == "prof").unwrap();
    let jour = jour_neuf();
    let t = Instant::now();
    let rapport = labo::preparer_classes(&base, &admin, p.user_id, chrono::Local::now().date_naive())
        .await
        .expect("classes de test");
    println!("préparation des classes : {:?} {rapport:?}", t.elapsed());
    assert_eq!(jour.weekday(), chrono::Local::now().date_naive().weekday());

    let api = prof(&base, &l, 0).await;
    let creneaux = api.mes_creneaux(jour).await.unwrap();
    for (classe, attendu) in [("PROTO38", 38usize), ("PROTO400", 400)] {
        let c = creneaux.iter().find(|c| c.classe == classe).expect(classe);
        let t = Instant::now();
        let appel = api.ouvrir_appel(c.id, jour).await.unwrap();
        let t_ouvrir = t.elapsed();
        assert_eq!(appel.lignes.len(), attendu, "{classe}");

        let t = Instant::now();
        let f = Feuille::depuis_api(&appel);
        let statuts: Vec<_> = f.eleves.iter().map(|e| e.statut_enregistre).collect();
        let v = visibles(&f.eleves, &statuts, &Filtre { recherche: "mart".into(), statut: None });
        let t_modele = t.elapsed();
        assert!(!v.is_empty());

        let mut s = saisies_initiales(&f);
        for x in s.iter_mut().step_by(4) {
            x.statut = Statut::Absent;
        }
        let liste = changements(&f.eleves, &s, CAPACITES_API_ACTUELLE).unwrap();
        let t = Instant::now();
        for ch in &liste {
            api.marquer(appel.id, ch.eleve_id, ch.etat).await.unwrap();
        }
        let t_marquer = t.elapsed();
        let t = Instant::now();
        let clos = api.clore(appel.id).await.unwrap();
        let t_clore = t.elapsed();
        assert!(clos.est_clos());
        println!(
            "{classe} ({attendu} élèves) : ouverture {:?}, modèle {:?}, {} lignes envoyées en {:?} ({:?}/ligne), clôture {:?}",
            t_ouvrir,
            t_modele,
            liste.len(),
            t_marquer,
            t_marquer / liste.len().max(1) as u32,
            t_clore
        );
    }
}
