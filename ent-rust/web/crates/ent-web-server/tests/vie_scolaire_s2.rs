//! Vie scolaire complète (socle 2, S2-4) contre le **vrai** backend (`ent_api::app`) et une base
//! PostgreSQL **locale** de test : la famille déclare une absence, le professeur fait l'appel,
//! l'absence est rapprochée (« prévenue »), la vie scolaire la voit dans la journée ; permissions.
//!
//! Lancement (voir `contre_backend.rs`) :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test vie_scolaire_s2 -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use chrono::{Datelike, Duration, NaiveDate};
use ent_types::vie_scolaire::DeclarerAbsenceBody;
use ent_web_api::{ApiClient, ApiError, EtatApi};
use ent_web_app::composants_metier::declaration::{
    composer_declaration, Origine, SaisieDeclaration,
};
use ent_web_app::modules::journee_vs::modele::compteurs;
use ent_web_server::labo::{self, Labo};
use uuid::Uuid;

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

/// Prochain lundi strictement futur au jour de Paris : un jour de classe ouvré explicite (jamais le jour
/// UTC ni le jour de semaine réel, pour que les tests passent aussi le week-end et entre 23 h et minuit UTC).
fn lundi_a_venir() -> NaiveDate {
    let j = ent_db::jour_paris();
    j + Duration::days(7 - i64::from(j.weekday().num_days_from_monday()))
}

/// Un jour futur à moins de 200 jours (fenêtre de déclaration), toujours un lundi (jour ouvré explicite).
fn jour_neuf() -> NaiveDate {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;
    lundi_a_venir() + Duration::weeks(n.rem_euclid(25))
}

async fn client_de(base: &str, l: &Labo, email: &str) -> ApiClient {
    let s = labo::connecter(base, l, email)
        .await
        .unwrap_or_else(|e| panic!("connexion {email} : {e:?}"));
    ApiClient::new(base, Some(s.token))
}

async fn personnel(base: &str, l: &Labo, role: &str) -> ApiClient {
    let p = l
        .personnel
        .iter()
        .find(|p| p.role == role)
        .unwrap_or_else(|| panic!("personnel {role} de labo"));
    client_de(base, l, &p.email).await
}

async fn parent_de(base: &str, l: &Labo, prenom: &str) -> ApiClient {
    let p = l
        .personas
        .iter()
        .find(|p| p.role == "parent" && p.label.contains(prenom))
        .unwrap_or_else(|| panic!("parent de {prenom}"));
    client_de(base, l, &p.email).await
}

fn corps(eleve: Uuid, jour: NaiveDate, motif: &str) -> DeclarerAbsenceBody {
    composer_declaration(
        &SaisieDeclaration {
            eleve: Some(eleve),
            debut_jour: jour.to_string(),
            motif: motif.into(),
            declarant_nom: "Mme Potter".into(),
            declarant_qualite: "mère".into(),
            commentaire: "Rendez-vous médical".into(),
            ..Default::default()
        },
        Origine::Accueil,
    )
    .expect("saisie valide")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn page_journee_rendue_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    let r = reqwest::get(format!("{base}/web/journee")).await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.headers().contains_key("content-security-policy"), "CSP");
    let html = r.text().await.unwrap();
    assert!(html.contains("Journée"), "titre");
    assert!(
        html.contains("Chargement"),
        "état de chargement rendu côté serveur"
    );
    assert!(!html.contains(" style=\""), "aucun style en ligne (CSP)");
}

/// La famille de Harry déclare l'absence ; le professeur fait l'appel ; la vie scolaire voit
/// l'absence « prévenue » ; les autres rôles sont refusés.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn declaration_famille_appel_rapprochement_journee() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let harry = l.eleve("Harry").expect("Harry").id;
    let jour = jour_neuf();

    // 1. La famille (peut justifier) déclare l'absence de la journée.
    let lily = parent_de(&base, &l, "Harry").await;
    let enfants = lily.enfants_famille().await.expect("enfants");
    assert!(
        enfants
            .iter()
            .find(|e| e.id == harry)
            .unwrap()
            .peut_justifier_absence
    );
    let mut b = corps(harry, jour, "maladie");
    b.declarant_nom = None;
    b.declarant_qualite = None;
    let d = lily
        .declarer_absence_famille(&b)
        .await
        .expect("déclaration");
    assert_eq!(
        (d.source.as_str(), d.statut.as_str()),
        ("famille", "active")
    );
    assert!(
        d.declarant_nom.is_none(),
        "identité du compte, pas de la saisie"
    );

    // 2. Le professeur fait l'appel de 4B ce jour-là : Harry absent. L'appel est pré-rempli (A1).
    let prof = personnel(&base, &l, "prof").await;
    let c = prof
        .mes_creneaux(jour)
        .await
        .expect("créneaux")
        .into_iter()
        .find(|c| c.classe == "4B")
        .expect("séance 4B");
    let creneau_4b = c.id;
    let appel = prof.ouvrir_appel(c.id, jour).await.expect("ouvrir");
    let attendus: Vec<Uuid> = serde_json::from_value(
        prof.get_json(&format!("/appels/{}/attendus-absents", appel.id))
            .await
            .expect("attendus"),
    )
    .unwrap();
    assert_eq!(
        attendus,
        vec![harry],
        "seul l'élève dont l'absence est annoncée"
    );
    prof.marquer(appel.id, harry, EtatApi::Absent)
        .await
        .unwrap();
    prof.clore(appel.id).await.expect("clôture");

    // 3. La vie scolaire voit une absence prévenue, la déclaration, et aucun appel à vérifier.
    let cpe = personnel(&base, &l, "cpe").await;
    let t = cpe.tableau_du_jour(jour).await.expect("tableau du jour");
    let a = t
        .absents
        .iter()
        .find(|a| a.eleve_id == harry)
        .expect("Harry absent");
    assert!(a.prevenue, "absence annoncée par la famille");
    assert_eq!(a.statut, "a_traiter");
    assert!(t
        .declarations
        .iter()
        .any(|d| d.declaration.eleve_id == harry));
    let c = compteurs(&t);
    assert!(c.absents >= 1 && c.declarations >= 1);
    // La séance 4B est faite (appel clos) : elle n'est pas dans les appels à vérifier.
    assert!(t
        .verification
        .seances_sans_appel
        .iter()
        .all(|s| s.creneau_id != creneau_4b));

    // 4. Permissions : un professeur, une famille et un élève ne voient pas la journée.
    for api in [&prof, &lily] {
        match api.tableau_du_jour(jour).await {
            Err(ApiError::Interdit(_)) => {}
            autre => panic!("accès attendu refusé, obtenu {autre:?}"),
        }
    }
    // Une famille ne déclare pas à l'accueil ; l'accueil ne passe pas par la porte « famille ».
    assert!(matches!(
        lily.declarer_absence_accueil(&corps(harry, jour, "maladie"))
            .await,
        Err(ApiError::Interdit(_))
    ));
    let cpe_decl = cpe
        .declarer_absence_famille(&corps(harry, jour, "maladie"))
        .await;
    assert!(
        matches!(cpe_decl, Err(ApiError::Interdit(_))),
        "{cpe_decl:?}"
    );
}

/// L'accueil enregistre une absence annoncée par téléphone (identité obligatoire) ; la vie
/// scolaire la voit le jour même ; une autre famille n'y a pas accès.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn absence_annoncee_a_l_accueil() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let hermione = l.eleve("Hermione").expect("Hermione").id;
    let jour = jour_neuf();
    let accueil = personnel(&base, &l, "administratif").await;

    // Identité de l'appelant obligatoire (le formulaire la valide aussi avant l'envoi).
    let mut sans_nom = corps(hermione, jour, "maladie");
    sans_nom.declarant_nom = None;
    assert!(matches!(
        accueil.declarer_absence_accueil(&sans_nom).await,
        Err(ApiError::Invalide(_))
    ));
    // Motif inconnu : refusé par le serveur.
    assert!(accueil
        .declarer_absence_accueil(&corps(hermione, jour, "motif_inconnu"))
        .await
        .is_err());
    let d = accueil
        .declarer_absence_accueil(&corps(hermione, jour, "maladie"))
        .await
        .expect("déclaration à l'accueil");
    assert_eq!(d.source, "accueil");
    assert_eq!(d.declarant_nom.as_deref(), Some("Mme Potter"));

    let cpe = personnel(&base, &l, "cpe").await;
    let t = cpe.tableau_du_jour(jour).await.unwrap();
    assert!(t
        .declarations
        .iter()
        .any(|x| x.declaration.id == d.id && x.declaration.eleve_id == hermione));
    // Annulation par le personnel autorisé, puis disparition de la journée.
    cpe.annuler_declaration(d.id).await.expect("annulation");
    let t = cpe.tableau_du_jour(jour).await.unwrap();
    assert!(t.declarations.iter().all(|x| x.declaration.id != d.id));
    // Les déclarations d'Hermione restent invisibles pour la famille de Harry.
    let lily = parent_de(&base, &l, "Harry").await;
    assert!(matches!(
        lily.declarations_eleve(hermione).await,
        Err(ApiError::Interdit(_))
    ));
}
