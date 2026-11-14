//! Fiche élève 360° (S2-1) contre le **vrai** backend (`ent_api::app`) et une base
//! PostgreSQL **locale** de test (données de labo fictives) : rendu serveur, parent sans
//! e-mail, minimisation famille, suspension (accès coupé puis rendu), payeurs,
//! contacts proposés par la famille, chronologie, permissions.
//!
//! Lancement :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test dossier -- --test-threads=1
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_web_api::{ApiClient, ApiError, ResultatLien};
use ent_web_app::modules::dossier::modele::{
    contact_depuis, etat_lien, parts_payeurs, peut, EtatLien, SaisieResponsable, ACTION_PERSONNES,
    ACTION_SUSPENDRE,
};
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn page_fiche_rendue_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    let http = reqwest::Client::new();
    let r = http
        .get(format!("{base}/web/eleves/{}", Uuid::nil()))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.headers().contains_key("content-security-policy"), "CSP");
    let html = r.text().await.unwrap();
    assert!(html.contains("Dossier de l"), "titre");
    assert!(html.contains("Chargement du dossier"), "état de chargement");
    assert!(!html.contains(" style=\""), "aucun style en ligne (CSP)");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fiche_parent_sans_email_suspension_payeurs_contacts() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let harry = l.eleve("Harry").expect("Harry").id;
    let secretariat = personnel(&base, &l, "administratif").await;
    let direction = personnel(&base, &l, "direction").await;
    let compta = personnel(&base, &l, "compta").await;
    let lily = parent_de(&base, &l, "Harry").await;

    // Le secrétariat ajoute le père, sans e-mail ni compte (écran « Ajouter un responsable »).
    let saisie = SaisieResponsable {
        nom: "Potter".into(),
        prenom: "James".into(),
        telephone: "06 11 22 33 44".into(),
        lien: "père".into(),
        garde: "alternee".into(),
        autorite: true,
        voir_notes: true,
        voir_vs: true,
        ..Default::default()
    };
    // Commande unique : la personne et son lien dans une seule requête.
    let ResultatLien::Actif(r) = secretariat
        .ajouter_responsable(harry, &saisie.ajout().unwrap())
        .await
        .expect("ajout")
    else {
        panic!("père sans compte : lien actif attendu");
    };
    let p = r.personne.clone();
    assert!(!p.a_compte && p.email.is_none());
    assert_eq!(
        etat_lien(&r, ent_db::jour_paris()),
        EtatLien::Actif
    );

    // Fiche du secrétariat : le père avec son téléphone, actions de gestion, pas de suspension.
    let f = secretariat.fiche_dossier(harry).await.expect("fiche");
    assert!(f
        .responsables
        .iter()
        .any(|x| x.personne.id == p.id && x.personne.telephone.is_some()));
    assert!(peut(&f.actions, ACTION_PERSONNES));
    assert!(
        !peut(&f.actions, ACTION_SUSPENDRE),
        "suspension réservée à la direction"
    );
    // Le professeur ne crée pas de personne.
    let prof = personnel(&base, &l, "prof").await;
    assert!(matches!(
        prof.creer_personne(&saisie.personne().unwrap()).await,
        Err(ApiError::Interdit(_))
    ));

    // Vu par la mère : le père sans coordonnées, aucune action ni alerte.
    let f = lily.fiche_dossier(harry).await.expect("fiche famille");
    let pere = f
        .responsables
        .iter()
        .find(|x| x.personne.id == p.id)
        .unwrap();
    assert!(pere.personne.telephone.is_none());
    assert!(f.actions.is_empty() && f.alertes.is_empty());
    // La mère propose un contact d'urgence ; le secrétariat le voit « proposé ».
    let c = lily
        .ajouter_contact(
            harry,
            &contact_depuis("Dursley", "Pétunia", "01 02 03 04 05", "tante", true).unwrap(),
        )
        .await
        .expect("contact proposé");
    assert_eq!(c.statut, "propose");
    secretariat.valider_contact(c.id).await.expect("validation");

    // Payeurs : 100 % pour la mère (seule rattachée à un foyer), par la comptabilité.
    let f = secretariat.fiche_dossier(harry).await.unwrap();
    let mere = f
        .responsables
        .iter()
        .find(|x| x.personne.a_compte && x.lien != "père")
        .expect("mère")
        .clone();
    let parts = parts_payeurs(&[(mere.lien_id, "100".into()), (r.lien_id, String::new())]).unwrap();
    let v = compta.definir_payeurs(harry, parts).await.expect("payeurs");
    assert!(v.iter().any(|x| x.lien_id == mere.lien_id && x.est_payeur));

    // La direction suspend le lien de la mère : la fiche lui devient inaccessible.
    let d = direction.fiche_dossier(harry).await.unwrap();
    assert!(peut(&d.actions, ACTION_SUSPENDRE));
    assert!(matches!(
        secretariat
            .suspendre_lien(mere.lien_id, "Ordonnance".into(), None)
            .await,
        Err(ApiError::Interdit(_))
    ));
    let s = direction
        .suspendre_lien(
            mere.lien_id,
            "Ordonnance du juge".into(),
            Some("JAF-1".into()),
        )
        .await
        .expect("suspension");
    assert_eq!(
        etat_lien(&s, ent_db::jour_paris()),
        EtatLien::Suspendu
    );
    assert!(matches!(
        lily.fiche_dossier(harry).await,
        Err(ApiError::Introuvable(_))
    ));
    assert!(lily.chronologie_eleve(harry, 10).await.is_err());
    direction
        .retablir_lien(mere.lien_id)
        .await
        .expect("rétablir");
    assert!(lily.fiche_dossier(harry).await.is_ok());

    // Chronologie (direction) : le lien du père et la suspension y figurent.
    let c = direction
        .chronologie_eleve(harry, 200)
        .await
        .expect("chronologie");
    assert!(c.iter().any(|e| e.titre == "Responsable lié ou modifié"));
    assert!(c
        .iter()
        .any(|e| e.titre == "Lien d'un responsable suspendu"));
    assert!(c.windows(2).all(|w| w[0].quand >= w[1].quand));
}
