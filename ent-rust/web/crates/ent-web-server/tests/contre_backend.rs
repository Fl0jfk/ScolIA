//! Tests du prototype contre le **vrai** backend (`ent_api::app`) et une base
//! PostgreSQL **locale** de test, avec les données de labo fictives.
//!
//! Lancement (voir docs/LEPTOS_CALL_PROTOTYPE.md) :
//!   . ent-rust/web/scripts/env.sh && bash ent-rust/web/scripts/pg.sh start
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test contre_backend -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
//! Sans la feature `labo` (L4), ce fichier est vide : le labo n'existe pas hors feature.
#![cfg(feature = "labo")]

use std::net::SocketAddr;
use std::time::Instant;

use chrono::{Datelike, Duration, NaiveDate};
use ent_web_api::{ApiClient, ApiError, AppelApi, EtatApi, CAPACITES_API_ACTUELLE};
use ent_web_app::appel::modele::{changements, visibles, Feuille, Filtre, Saisie, Statut};
use ent_web_server::labo::{self, Labo};

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
    let p = l
        .personnel
        .iter()
        .filter(|p| p.role == "prof")
        .nth(index)
        .expect("prof de labo");
    let s = labo::connecter(base, l, &p.email)
        .await
        .expect("connexion prof");
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
    matches!(
        r,
        Err(ApiError::Invalide(_)) | Err(ApiError::Introuvable(_))
    )
}

fn saisies_initiales(f: &Feuille) -> Vec<Saisie> {
    f.eleves
        .iter()
        .map(|e| Saisie {
            eleve_id: e.eleve_id,
            statut: e.statut_enregistre,
            commentaire: String::new(),
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pages_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let r = http.get(format!("{base}/web/appel")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers()["x-frame-options"], "SAMEORIGIN");
    assert_eq!(r.headers()["x-content-type-options"], "nosniff");
    let html = r.text().await.unwrap();
    assert!(html.contains(r#"<html lang="fr""#), "langue de la page");
    assert!(
        html.contains("Faire l&#x27;appel") || html.contains("Faire l'appel"),
        "titre"
    );
    assert!(
        html.contains("Chargement des séances"),
        "état de chargement rendu côté serveur"
    );
    assert!(html.contains("/pkg/ent-web.js"), "script d'hydratation");
    assert!(html.contains("/pkg/ent-web.css"), "feuille de style");
    assert!(html.contains("Aller au contenu"), "lien d'évitement");
    assert!(
        html.contains("Date de l'appel") && html.contains(r#"id="jour""#),
        "champ date étiqueté (label englobant, nom accessible)"
    );

    // Écrans du référentiel scolaire : rendus côté serveur avec leur état de chargement.
    for (chemin, attendu) in [
        ("/web/classes", "Chargement des classes"),
        ("/web/mes-classes", "Chargement de vos classes"),
        (
            "/web/classes/6f1c3c80-0000-4000-8000-000000000001",
            "Chargement de la classe",
        ),
    ] {
        let r = http.get(format!("{base}{chemin}")).send().await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        let html = r.text().await.unwrap();
        assert!(html.contains(attendu), "{chemin} : {attendu}");
        assert!(
            html.contains("Aller au contenu"),
            "{chemin} : lien d'évitement"
        );
    }

    let html = http
        .get(format!("{base}/web/connexion"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    for attendu in [
        "Adresse e-mail",
        "Mot de passe",
        "Établissement",
        r#"autocomplete="current-password""#,
    ] {
        assert!(html.contains(attendu), "page de connexion : {attendu}");
    }
    assert!(
        html.contains("Environnement de test"),
        "fonction serveur exécutée au rendu"
    );

    let r = http.get(format!("{base}/web")).send().await.unwrap();
    assert!(
        r.status().is_redirection() || r.status() == 200,
        "redirection /web → /web/appel ({})",
        r.status()
    );

    let r = http
        .get(format!("{base}/web/inconnue"))
        .send()
        .await
        .unwrap();
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
    let r = anonyme
        .connexion(&p0.email, "mauvais-mot-de-passe", l.etablissement_id)
        .await;
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
    assert!(
        appel.lignes.iter().all(|l| l.etat == "present"),
        "présent par défaut"
    );
    assert!(
        appel.lignes.iter().all(|l| l.eleve_label.is_some()),
        "noms fournis"
    );

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
        api.marquer(appel.id, ch.eleve_id, ch.etat)
            .await
            .expect("marquer");
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
    let r = api
        .marquer(
            appel.id,
            f.eleves[0].eleve_id,
            ent_web_api::EtatApi::Present,
        )
        .await;
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
    assert!(matches!(
        api_parent.mes_creneaux(jour).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        api_parent.ouvrir_appel(c.id, jour).await,
        Err(ApiError::Interdit(_))
    ));

    // Autre professeur : pas l'appel d'une séance dont il n'est ni titulaire ni remplaçant.
    let autre = prof(&base, &l, 1).await;
    assert!(matches!(
        autre.ouvrir_appel(c.id, jour).await,
        Err(ApiError::Interdit(_))
    ));

    // CPE : peut ouvrir l'appel d'une séance (remplacement / vie scolaire).
    {
        let cpe = l.premier("cpe").expect("CPE de labo");
        let s = labo::connecter(&base, &l, &cpe.email).await.unwrap();
        let api_cpe = ApiClient::new(&base, Some(s.token));
        let r = api_cpe.ouvrir_appel(c.id, jour).await;
        // Règle d'ent-api : `role_peut_ouvrir_appel_remplacant` (admin, direction,
        // administratif, CPE) lève l'exigence « titulaire ou remplaçant ».
        let appel = r.expect("le CPE peut ouvrir l'appel d'une séance qui n'est pas la sienne");
        assert!(!appel.lignes.is_empty());
    }
}

/// Contrôle d'accès horizontal : un professeur qui connaît l'identifiant de l'appel
/// d'un collègue ne peut ni le lire, ni le modifier, ni le clore (403) ; la
/// supervision (CPE) reste autorisée.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn appel_d_un_collegue_inaccessible() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let jour = jour_neuf();
    let rogue = prof(&base, &l, 0).await;
    let autre = prof(&base, &l, 1).await;
    let c = creneau_4b(&rogue, jour).await;
    let appel = rogue.ouvrir_appel(c.id, jour).await.expect("ouverture");
    let eleve = appel.lignes.first().expect("une ligne").eleve_id;

    assert!(
        matches!(
            autre.appel_par_id(appel.id).await,
            Err(ApiError::Interdit(_))
        ),
        "lecture par un autre prof"
    );
    assert!(
        matches!(
            autre.marquer(appel.id, eleve, EtatApi::Absent).await,
            Err(ApiError::Interdit(_))
        ),
        "pointage par un autre prof"
    );
    assert!(
        matches!(autre.clore(appel.id).await, Err(ApiError::Interdit(_))),
        "clôture par un autre prof"
    );

    // L'appel n'a pas été altéré et reste ouvert pour son titulaire.
    let relu = rogue
        .appel_par_id(appel.id)
        .await
        .expect("lecture titulaire");
    assert!(!relu.est_clos());

    // Supervision : le CPE lit, pointe et clôt.
    let cpe = l.premier("cpe").expect("CPE de labo");
    let s = labo::connecter(&base, &l, &cpe.email).await.unwrap();
    let api_cpe = ApiClient::new(&base, Some(s.token));
    api_cpe.appel_par_id(appel.id).await.expect("CPE lit");
    api_cpe
        .marquer(appel.id, eleve, EtatApi::Absent)
        .await
        .expect("CPE pointe");
    assert!(api_cpe.clore(appel.id).await.expect("CPE clôt").est_clos());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn classes_nombreuses_38_et_400() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let admin = labo::connecter(&base, &l, &l.persona("admin").unwrap().email)
        .await
        .unwrap();
    let p = l.personnel.iter().find(|p| p.role == "prof").unwrap();
    let jour = jour_neuf();
    let t = Instant::now();
    let rapport =
        labo::preparer_classes(&base, &admin, p.user_id, chrono::Local::now().date_naive())
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
        let v = visibles(
            &f.eleves,
            &statuts,
            &Filtre {
                recherche: "mart".into(),
                statut: None,
            },
        );
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

/// Contrat JSON du lot 1 d'`ent-types` : chaque réponse réelle d'`ent-api` est relue
/// dans le type partagé puis re-sérialisée ; le JSON doit être **identique**
/// (mêmes champs, mêmes absences de champs facultatifs, mêmes formats).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn contrat_json_ent_types_lot1() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let http = reqwest::Client::new();
    fn identique<T: serde::Serialize + serde::de::DeserializeOwned>(
        nom: &str,
        brut: &serde_json::Value,
    ) {
        let t: T = serde_json::from_value(brut.clone())
            .unwrap_or_else(|e| panic!("{nom} illisible : {e}"));
        let relu = serde_json::to_value(&t).unwrap();
        assert_eq!(
            &relu, brut,
            "{nom} : le JSON d'ent-types diffère de celui d'ent-api"
        );
    }

    let p = l.personnel.iter().find(|p| p.role == "prof").unwrap();
    let login: serde_json::Value = http
        .post(format!("{base}/auth/login"))
        .json(&ent_types::auth::LoginBody {
            email: p.email.clone(),
            password: l.password.clone(),
            etablissement_id: l.etablissement_id,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    identique::<ent_types::auth::LoginResponseDto>("POST /auth/login", &login);
    let jeton = login["token"].as_str().expect("jeton").to_string();
    let get = |chemin: String| {
        let http = http.clone();
        let jeton = jeton.clone();
        async move {
            http.get(chemin)
                .bearer_auth(jeton)
                .send()
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap()
        }
    };

    let moi = get(format!("{base}/auth/me")).await;
    identique::<ent_types::auth::MeDto>("GET /auth/me", &moi);

    let jour = jour_neuf();
    let creneaux = get(format!("{base}/edt/mes-creneaux?jour={jour}")).await;
    for c in creneaux.as_array().expect("liste") {
        identique::<ent_types::edt::EdtCreneauDto>("GET /edt/mes-creneaux", c);
    }
    let c4b = creneaux
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["classe"] == "4B")
        .expect("4B");

    let ouvert: serde_json::Value = http
        .post(format!("{base}/appels"))
        .bearer_auth(&jeton)
        .json(&ent_types::appel::OuvrirAppelBody {
            creneau_id: c4b["id"].as_str().unwrap().parse().unwrap(),
            jour,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    identique::<ent_types::appel::AppelDto>("POST /appels", &ouvert);
    let id = ouvert["id"].as_str().unwrap();
    let clos: serde_json::Value = http
        .post(format!("{base}/appels/{id}/clore"))
        .bearer_auth(&jeton)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    identique::<ent_types::appel::AppelDto>("POST /appels/{id}/clore", &clos);
    assert_eq!(clos["statut"], "clos");
    println!("contrat ent-types lot 1 : 5 routes vérifiées, JSON identique");
}

/// Remplace les identifiants (UUID) et les jours variables par des repères fixes,
/// pour comparer le JSON **brut** (ordre des champs compris) d'une exécution à l'autre.
fn normaliser(brut: &str, jours: &[String]) -> String {
    let mut s = brut.to_string();
    for (i, j) in jours.iter().enumerate() {
        s = s.replace(j.as_str(), &format!("<jour{i}>"));
    }
    let c: Vec<char> = s.chars().collect();
    let forme = |w: &[char]| {
        w.len() == 36
            && w.iter().enumerate().all(|(i, ch)| match i {
                8 | 13 | 18 | 23 => *ch == '-',
                _ => ch.is_ascii_hexdigit(),
            })
    };
    let mut sortie = String::new();
    let mut i = 0;
    while i < c.len() {
        if i + 36 <= c.len() && forme(&c[i..i + 36]) {
            sortie.push_str("<uuid>");
            i += 36;
        } else {
            sortie.push(c[i]);
            i += 1;
        }
    }
    // L'ordre des lignes d'un appel n'est pas garanti par ent-api (pas d'ORDER BY après
    // modification) : on trie les objets de `lignes`, chacun gardé tel quel (texte brut).
    if let Some(debut) = sortie.find(r#""lignes":["#) {
        let d = debut + r#""lignes":["#.len();
        if let Some(fin) = sortie[d..].find("}]").map(|f| d + f + 1) {
            let mut lignes: Vec<&str> = sortie[d..fin].split("},{").collect();
            if lignes.len() > 1 {
                let n = lignes.len();
                let mut objets: Vec<String> = lignes
                    .drain(..)
                    .enumerate()
                    .map(|(k, o)| {
                        let o = if k == 0 {
                            o.to_string()
                        } else {
                            format!("{{{o}")
                        };
                        if k == n - 1 {
                            o
                        } else {
                            format!("{o}}}")
                        }
                    })
                    .collect();
                objets.sort();
                sortie = format!("{}{}{}", &sortie[..d], objets.join(","), &sortie[fin..]);
            }
        }
    }
    sortie
}

async fn envoyer(r: reqwest::RequestBuilder) -> (u16, String) {
    let rep = r.send().await.unwrap();
    (rep.status().as_u16(), rep.text().await.unwrap())
}

/// Instantané du contrat JSON des routes appel / EDT (lot 1 d'`ent-types` côté `ent-api`) :
/// corps bruts des réponses (ordre des champs compris) et **rejets** des corps invalides
/// (code HTTP + message). Écrit dans `ENT_CONTRAT_SORTIE` s'il est défini, pour comparer
/// avant / après une migration (`diff`). Vérifie aussi les invariants du contrat.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn contrat_json_appel_edt_instantane() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let http = reqwest::Client::new();
    let p = l.personnel.iter().find(|p| p.role == "prof").unwrap();
    let prof = labo::connecter(&base, &l, &p.email)
        .await
        .expect("prof")
        .token;
    let g = l
        .personnel
        .iter()
        .find(|p| ["direction", "admin", "administratif", "cpe"].contains(&p.role.as_str()))
        .expect("personnel de gestion de l'EDT");
    let gestion = labo::connecter(&base, &l, &g.email)
        .await
        .expect("gestion")
        .token;

    let jour = jour_neuf();
    let jour2 = jour + Duration::weeks(1);
    let jours = vec![jour.to_string(), jour2.to_string()];
    let mut instantane: Vec<String> = Vec::new();
    let mut noter = |nom: &str, statut: u16, corps: &str| {
        instantane.push(format!(
            "## {nom}\n{statut}\n{}\n",
            normaliser(corps, &jours)
        ));
    };
    let json = "application/json";

    // EDT : les 4 routes qui utilisent la conversion EdtCreneau -> EdtCreneauDto.
    let (s, mes) = envoyer(
        http.get(format!("{base}/edt/mes-creneaux?jour={jour}"))
            .bearer_auth(&prof),
    )
    .await;
    noter("GET /edt/mes-creneaux (prof)", s, &mes);
    assert_eq!(s, 200);
    let liste: serde_json::Value = serde_json::from_str(&mes).unwrap();
    let c4b = liste
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["classe"] == "4B")
        .expect("4B")
        .clone();
    let creneau_id = c4b["id"].as_str().unwrap().to_string();

    let (s, b) = envoyer(
        http.get(format!("{base}/edt/creneaux?jour={jour}"))
            .bearer_auth(&gestion),
    )
    .await;
    noter("GET /edt/creneaux (gestion)", s, &b);
    assert_eq!(s, 200);

    let (s, b) = envoyer(
        http.post(format!("{base}/edt/creneaux/{creneau_id}/affectation"))
            .bearer_auth(&gestion)
            .json(
                &serde_json::json!({ "prof_user_id": c4b["prof_user_id"], "salle": c4b["salle"] }),
            ),
    )
    .await;
    noter(
        "POST /edt/creneaux/{id}/affectation (même prof, même salle)",
        s,
        &b,
    );
    assert_eq!(s, 200, "{b}");

    // Appel : rejets, ouverture, marquage, lecture, clôture.
    let url_appels = format!("{base}/appels");
    for (nom, corps) in [
        ("POST /appels corps vide {}", "{}".to_string()),
        (
            "POST /appels sans jour",
            format!(r#"{{"creneau_id":"{creneau_id}"}}"#),
        ),
        (
            "POST /appels creneau_id invalide",
            format!(r#"{{"creneau_id":"abc","jour":"{jour}"}}"#),
        ),
        (
            "POST /appels jour invalide",
            format!(r#"{{"creneau_id":"{creneau_id}","jour":"31/12/2026"}}"#),
        ),
        (
            "POST /appels type incorrect",
            format!(r#"{{"creneau_id":42,"jour":"{jour}"}}"#),
        ),
        (
            "POST /appels JSON mal formé",
            r#"{"creneau_id":"#.to_string(),
        ),
    ] {
        let (s, b) = envoyer(
            http.post(&url_appels)
                .bearer_auth(&prof)
                .header("content-type", json)
                .body(corps),
        )
        .await;
        noter(nom, s, &b);
        assert!((400..500).contains(&s), "{nom} : {s}");
    }
    let (s, b) = envoyer(http.post(&url_appels).bearer_auth(&prof).body(format!(
        r#"{{"creneau_id":"{creneau_id}","jour":"{jour}"}}"#
    )))
    .await;
    noter("POST /appels sans content-type", s, &b);
    assert!((400..500).contains(&s), "sans content-type : {s}");

    let (s, ouvert) = envoyer(
        http.post(&url_appels)
            .bearer_auth(&prof)
            .header("content-type", json)
            .body(format!(
                r#"{{"creneau_id":"{creneau_id}","jour":"{jour}","champ_inconnu":true}}"#
            )),
    )
    .await;
    noter("POST /appels (champ inconnu ignoré)", s, &ouvert);
    assert_eq!(s, 201, "{ouvert}");
    let appel: serde_json::Value = serde_json::from_str(&ouvert).unwrap();
    let id = appel["id"].as_str().unwrap().to_string();
    let eleve = appel["lignes"][0]["eleve_id"]
        .as_str()
        .expect("élève")
        .to_string();

    let (s, b) = envoyer(
        http.get(format!("{base}/eleves/{eleve}/edt?jour={jour}"))
            .bearer_auth(&gestion),
    )
    .await;
    noter("GET /eleves/{id}/edt (gestion)", s, &b);

    let url_lignes = format!("{base}/appels/{id}/lignes");
    for (nom, corps) in [
        (
            "POST /appels/{id}/lignes sans etat",
            format!(r#"{{"eleve_id":"{eleve}"}}"#),
        ),
        (
            "POST /appels/{id}/lignes etat numérique",
            format!(r#"{{"eleve_id":"{eleve}","etat":1}}"#),
        ),
        (
            "POST /appels/{id}/lignes eleve_id invalide",
            r#"{"eleve_id":"x","etat":"absent"}"#.to_string(),
        ),
    ] {
        let (s, b) = envoyer(
            http.post(&url_lignes)
                .bearer_auth(&prof)
                .header("content-type", json)
                .body(corps),
        )
        .await;
        noter(nom, s, &b);
        assert!((400..500).contains(&s), "{nom} : {s}");
    }
    let marquer = |etat: &str| {
        http.post(&url_lignes)
            .bearer_auth(&prof)
            .header("content-type", json)
            .body(format!(r#"{{"eleve_id":"{eleve}","etat":"{etat}"}}"#))
    };
    let (s, b) = envoyer(marquer("absent")).await;
    noter("POST /appels/{id}/lignes (absent)", s, &b);
    assert_eq!(s, 204, "{b}");

    let (s, b) = envoyer(http.get(format!("{base}/appels/{id}")).bearer_auth(&prof)).await;
    noter("GET /appels/{id}", s, &b);
    assert_eq!(s, 200);
    let (s, b) = envoyer(
        http.post(format!("{base}/appels/{id}/clore"))
            .bearer_auth(&prof),
    )
    .await;
    noter("POST /appels/{id}/clore", s, &b);
    assert_eq!(s, 200);
    let (s, b) = envoyer(marquer("present")).await;
    noter(
        "POST /appels/{id}/lignes après clôture (refus métier)",
        s,
        &b,
    );
    assert!((400..500).contains(&s));
    let (s, b) = envoyer(
        http.post(&url_appels)
            .bearer_auth(&prof)
            .header("content-type", json)
            .body(format!(
                r#"{{"creneau_id":"{creneau_id}","jour":"{jour2}"}}"#
            )),
    )
    .await;
    noter("POST /appels (2e semaine)", s, &b);

    let texte = instantane.join("\n");
    if let Ok(chemin) = std::env::var("ENT_CONTRAT_SORTIE") {
        std::fs::write(&chemin, &texte).expect("écriture de l'instantané");
        println!("instantané écrit : {chemin}");
    }
    println!("contrat appel/EDT : {} cas enregistrés", instantane.len());
}

/// Tranche 1 : déconnexion, annonces, fils de messages, défi passkey.
/// Le parcours WebAuthn complet exige un authentificateur (clé/navigateur) : il n'est pas
/// testé ici (couvert côté navigateur ; `ent-web-session` embarque un authentificateur
/// logiciel pour le prototype cookie). On vérifie seulement ce qui ne le requiert pas.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tranche1_deconnexion_annonces_messages_passkey() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let api = prof(&base, &l, 0).await;

    // Annonces : lecture autorisée pour un professeur.
    let annonces = api.annonces().await.expect("annonces (prof)");
    for a in &annonces {
        assert!(!a.titre.trim().is_empty(), "titre d'annonce non vide");
    }
    println!("annonces visibles du prof : {}", annonces.len());

    // Fils de messages : lecture OK, forme cohérente.
    let fils = api.fils_messages().await.expect("fils (prof)");
    for f in &fils {
        assert!(!f.sujet.is_empty() || f.id != uuid::Uuid::nil());
        assert!(f.nb_non_lus >= 0);
    }
    println!("fils visibles du prof : {}", fils.len());

    // Rôle non autorisé : un parent n'a pas les annonces d'établissement (403).
    let parent = l.persona("parent").expect("persona parent");
    let sp = labo::connecter(&base, &l, &parent.email).await.unwrap();
    let api_parent = ApiClient::new(&base, Some(sp.token));
    assert!(matches!(
        api_parent.annonces().await,
        Err(ApiError::Interdit(_))
    ));
    // Sans session : 401.
    let anonyme = ApiClient::new(&base, None);
    assert_eq!(anonyme.annonces().await.unwrap_err(), ApiError::NonConnecte);
    assert_eq!(
        anonyme.fils_messages().await.unwrap_err(),
        ApiError::NonConnecte
    );

    // Passkey (socle 1, anti-énumération) : compte sans passkey et compte inconnu
    // reçoivent tous deux un défi de même forme (factice pour le second) ;
    // rien ne révèle l'existence du compte ni la présence d'une passkey.
    let p = l.personnel.iter().find(|p| p.role == "prof").unwrap();
    let connu = anonyme
        .connexion_passkey_debut(&p.email, l.etablissement_id)
        .await
        .expect("défi (réel ou factice) pour un compte connu");
    let inconnu = anonyme
        .connexion_passkey_debut("inconnu@example.invalid", l.etablissement_id)
        .await
        .expect("défi factice pour un compte inconnu");
    let cles = |o: &serde_json::Value| {
        o["publicKey"]
            .as_object()
            .map(|m| m.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    };
    assert_eq!(cles(&connu.options), cles(&inconnu.options), "même forme de défi");

    // Déconnexion : la session est révoquée, `moi()` répond 401.
    let s = labo::connecter(&base, &l, &p.email).await.unwrap();
    let api2 = ApiClient::new(&base, Some(s.token));
    assert_eq!(api2.moi().await.unwrap().role, "prof");
    api2.deconnexion().await.expect("déconnexion");
    assert_eq!(api2.moi().await.unwrap_err(), ApiError::NonConnecte);
    // La première session (autre jeton) reste valable.
    assert!(api.moi().await.is_ok());
}

/// Référentiel scolaire contre le vrai backend : « Mes classes » du professeur, détail,
/// liste complète et amorçage pour la gestion, refus pour la famille.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn referentiel_classes_contre_vrai_backend() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let jour = chrono::Local::now().date_naive();

    // Professeur : ses classes, avec effectif ; détail avec élèves et enseignants.
    let rogue = prof(&base, &l, 0).await;
    let mes = rogue.mes_classes(jour).await.expect("mes classes");
    let c4b = mes
        .iter()
        .find(|c| c.code == "4B")
        .expect("4B du professeur");
    assert!(c4b.effectif >= 1, "effectif issu du référentiel");
    let detail = rogue.classe(c4b.id, jour).await.expect("détail");
    assert_eq!(detail.classe.id, c4b.id);
    assert_eq!(detail.eleves.len() as i64, c4b.effectif);
    assert!(detail
        .eleves
        .iter()
        .all(|e| e.ine.is_none() && e.date_naissance.is_none()));
    assert!(detail
        .services
        .iter()
        .any(|s| s.matiere_libelle == "Potions"));
    // Rapprochement avec la séance de l'appel (composant partagé).
    let creneau = creneau_4b(&rogue, jour_neuf()).await;
    let trouvee = ent_web_app::composants_metier::classes::classe_du_creneau(&mes, &creneau.classe);
    assert_eq!(trouvee.map(|c| c.id), Some(c4b.id));

    // Gestion : liste complète, amorçage idempotent.
    let adm = l.premier("administratif").expect("administratif de labo");
    let s = labo::connecter(&base, &l, &adm.email).await.unwrap();
    let api_adm = ApiClient::new(&base, Some(s.token));
    let toutes = api_adm.classes(jour).await.expect("classes");
    assert!(toutes.len() >= mes.len());
    let r = api_adm.amorcer_referentiel().await.expect("amorçage");
    assert_eq!(
        (r.classes_creees, r.affectations_creees, r.services_crees),
        (0, 0, 0)
    );

    // Famille : refus (403) sur chaque route.
    let parent = l.persona("parent").expect("persona parent");
    let sp = labo::connecter(&base, &l, &parent.email).await.unwrap();
    let api_parent = ApiClient::new(&base, Some(sp.token));
    assert!(matches!(
        api_parent.classes(jour).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        api_parent.mes_classes(jour).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        api_parent.amorcer_referentiel().await,
        Err(ApiError::Interdit(_))
    ));
    // Le professeur ne peut pas amorcer (gestion seulement).
    assert!(matches!(
        rogue.amorcer_referentiel().await,
        Err(ApiError::Interdit(_))
    ));
    // Identifiant inconnu : introuvable.
    assert!(matches!(
        api_adm.classe(uuid::Uuid::new_v4(), jour).await,
        Err(ApiError::Introuvable(_))
    ));
}
