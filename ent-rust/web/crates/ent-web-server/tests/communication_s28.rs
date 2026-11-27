//! Communication complète (socle 2, S2-8) contre le **vrai** backend (`ent_api::app`) et une base
//! PostgreSQL **locale** de test : pages rendues par le serveur, préférences de notification,
//! campagne avec aperçu puis confirmation, formulaire et consentement, permissions.
//!
//! Lancement (voir `contre_backend.rs`) :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test communication_s28 -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »). Aucun envoi réel :
//! e-mail, SMS et push sont simulés.
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_types::communication::{
    ConfirmationLancementBody, NouveauFormulaireBody, NouvelleCampagneBody, ReponseFormulaireBody,
};
use ent_web_api::{ApiClient, ApiError};
use ent_web_app::modules::campagnes::modele::{
    composer, corps_lancement, peut_confirmer, SaisieCampagne,
};
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
async fn pages_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    for (chemin, titre) in [
        ("/web/notifications", "Mes notifications"),
        ("/web/campagnes", "Campagnes d&#x27;information"),
        ("/web/rencontres", "Rencontres parents-professeurs"),
        ("/web/rendez-vous", "Rendez-vous avec les enseignants"),
        ("/web/formulaires", "Formulaires et consentements"),
    ] {
        let r = reqwest::get(format!("{base}{chemin}")).await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(
            r.headers().contains_key("content-security-policy"),
            "CSP {chemin}"
        );
        let html = r.text().await.unwrap();
        let sans_apostrophe = titre.replace("&#x27;", "'");
        assert!(
            html.contains(titre) || html.contains(&sans_apostrophe),
            "{chemin} : titre « {titre} » absent"
        );
        assert!(html.contains("Chargement"), "{chemin} : état de chargement");
        assert!(
            !html.contains(" style=\""),
            "{chemin} : aucun style en ligne (CSP)"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn preferences_de_notification_aller_retour() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let lily = parent_de(&base, &l, "Harry").await;
    let mut p = lily.preferences_notification().await.expect("lecture");
    assert!(p.email && p.push, "e-mail et mobile actifs par défaut");
    // Le SMS demande un consentement explicite ; les valeurs sont relues telles qu'écrites.
    p.sms = true;
    p.resume = "quotidien".into();
    p.calme_debut = Some(21 * 60);
    p.calme_fin = Some(7 * 60);
    let ecrit = lily
        .definir_preferences_notification(&p)
        .await
        .expect("écriture");
    assert_eq!(ecrit, p);
    assert_eq!(lily.preferences_notification().await.unwrap(), p);
    // Valeurs invalides refusées par le serveur.
    let mut mauvais = p.clone();
    mauvais.resume = "hebdo".into();
    assert!(matches!(
        lily.definir_preferences_notification(&mauvais).await,
        Err(ApiError::Invalide(_))
    ));
    // Remise aux défauts pour les autres scénarios.
    p.sms = false;
    p.resume = "immediat".into();
    p.calme_debut = None;
    p.calme_fin = None;
    lily.definir_preferences_notification(&p).await.unwrap();
    // Traçabilité : liste lisible, sans plantage de décodage.
    lily.mes_envois_notification().await.expect("envois");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn campagne_apercu_puis_confirmation_et_droits() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let direction = personnel(&base, &l, "direction").await;
    let jour = chrono::Local::now().date_naive();
    let classe = direction
        .classes(jour)
        .await
        .expect("classes")
        .into_iter()
        .find(|c| c.code == "4B")
        .expect("classe 4B");

    // Les modèles d'écran composent le corps envoyé au serveur.
    let corps = composer(&SaisieCampagne {
        titre: "Réunion de rentrée".into(),
        corps: "Jeudi à 18 h".into(),
        email: true,
        sms: false,
        push: false,
        audience: "classe".into(),
        classe_id: classe.id.to_string(),
    })
    .expect("saisie valide");
    let c = direction
        .creer_campagne_information(&corps)
        .await
        .expect("création");
    assert_eq!(c.statut, "brouillon");

    // Aperçu : destinataires nommés, sans adresse ni numéro ; rien n'est parti.
    let a = direction.apercu_campagne(c.id).await.expect("aperçu");
    assert!(a.nb_destinataires >= 1 && a.nb_eleves >= 1);
    assert!(
        peut_confirmer(&a, false),
        "sans SMS, la confirmation suffit"
    );
    assert!(!serde_json::to_string(&a).unwrap().contains('@'));

    // Mauvaise empreinte : refus (409), la campagne reste lançable.
    let faux = ConfirmationLancementBody {
        empreinte: "0".repeat(64),
        nb_destinataires: a.nb_destinataires,
        confirme_sms: false,
    };
    assert!(matches!(
        direction.lancer_campagne(c.id, &faux).await,
        Err(ApiError::Conflit(_))
    ));
    // Confirmation avec l'empreinte de l'aperçu, puis second lancement refusé.
    let lancee = direction
        .lancer_campagne(c.id, &corps_lancement(&a, false))
        .await
        .expect("lancement");
    assert_eq!(lancee.statut, "lancee");
    assert!(matches!(
        direction
            .lancer_campagne(c.id, &corps_lancement(&a, false))
            .await,
        Err(ApiError::Conflit(_))
    ));
    let bilan = direction.bilan_campagne(c.id).await.expect("bilan");
    assert!(bilan.iter().all(|b| b.nombre > 0));

    // Droits : famille et enseignant n'ont pas accès aux campagnes.
    let lily = parent_de(&base, &l, "Harry").await;
    assert!(matches!(lily.campagnes().await, Err(ApiError::Interdit(_))));
    let prof = personnel(&base, &l, "prof").await;
    assert!(matches!(prof.campagnes().await, Err(ApiError::Interdit(_))));
    assert!(matches!(
        prof.creer_campagne_information(&NouvelleCampagneBody {
            titre: "x".into(),
            corps: "y".into(),
            canaux: vec!["email".into()],
            audience: "etablissement".into(),
            cible_ids: vec![],
        })
        .await,
        Err(ApiError::Interdit(_))
    ));
    // Plafonds : lecture réservée à `notifications.gerer`.
    direction.plafonds_notification().await.expect("plafonds");
    assert!(matches!(
        lily.plafonds_notification().await,
        Err(ApiError::Interdit(_))
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn formulaire_consentement_de_bout_en_bout() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let harry = l.eleve("Harry").expect("Harry").id;
    let direction = personnel(&base, &l, "direction").await;
    let lily = parent_de(&base, &l, "Harry").await;
    let jour = chrono::Local::now().date_naive();
    let classe = direction
        .classes(jour)
        .await
        .unwrap()
        .into_iter()
        .find(|c| c.code == "4B")
        .unwrap();
    // Un parent ne crée pas de formulaire.
    let brouillon = NouveauFormulaireBody {
        type_formulaire: "consentement".into(),
        objet: Some("droit_image".into()),
        titre: "Droit à l'image".into(),
        texte: "Autorisez-vous la publication de la photo de votre enfant ?".into(),
        audience: "classe".into(),
        cible_ids: vec![classe.id],
    };
    assert!(matches!(
        lily.creer_formulaire(&brouillon).await,
        Err(ApiError::Interdit(_))
    ));
    let f = direction
        .creer_formulaire(&brouillon)
        .await
        .expect("création");
    assert_eq!(f.statut, "brouillon");
    let f = direction
        .publier_formulaire(f.id)
        .await
        .expect("publication");
    assert_eq!(f.statut, "publie");
    // La famille le voit pour Harry, à traiter ; elle refuse.
    let mes = lily.formulaires_famille().await.expect("formulaires");
    let m = mes
        .iter()
        .find(|m| m.formulaire_id == f.id && m.eleve_id == harry)
        .expect("formulaire pour Harry");
    assert!(m.a_repondre);
    let etat = lily
        .repondre_formulaire(
            f.id,
            &ReponseFormulaireBody {
                eleve_id: harry,
                decision: "refuse".into(),
                commentaire: None,
            },
        )
        .await
        .expect("réponse");
    assert_eq!(etat.decision, "refuse");
    // La direction lit « qui consent » pour cet objet et cette classe : un refus.
    let liste = direction
        .consentements_classe("droit_image", classe.id)
        .await
        .expect("consentements");
    assert_eq!(
        liste.iter().find(|c| c.eleve_id == harry).unwrap().decision,
        "refuse"
    );
    // Un autre parent ne peut pas répondre pour Harry (introuvable).
    let autre = parent_de(&base, &l, "Draco").await;
    assert!(matches!(
        autre
            .repondre_formulaire(
                f.id,
                &ReponseFormulaireBody {
                    eleve_id: harry,
                    decision: "accepte".into(),
                    commentaire: None,
                },
            )
            .await,
        Err(ApiError::Introuvable(_))
    ));
    // Le professeur n'a pas accès à la lecture des consentements.
    let prof = personnel(&base, &l, "prof").await;
    assert!(matches!(
        prof.consentements_classe("droit_image", classe.id).await,
        Err(ApiError::Interdit(_))
    ));
    // Retour à un état neutre pour les autres scénarios : acceptation puis archivage.
    lily.repondre_formulaire(
        f.id,
        &ReponseFormulaireBody {
            eleve_id: harry,
            decision: "accepte".into(),
            commentaire: None,
        },
    )
    .await
    .unwrap();
    direction.archiver_formulaire(f.id).await.unwrap();
}
