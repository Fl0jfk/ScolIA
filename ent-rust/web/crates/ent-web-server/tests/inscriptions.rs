//! Admissions, préinscriptions et réinscriptions (S2-3) contre le **vrai** backend
//! (`ent_api::app`) et une base PostgreSQL **locale** de test (données de labo fictives) :
//! rendu serveur des pages, démarrage indiscernable, rien sans vérification, parcours
//! famille (jeton issu d'une session ouverte), instruction, décision, validation,
//! réinscription, permissions.
//!
//! Lancement :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test inscriptions -- --test-threads=1
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_web_api::types_inscriptions::{
    CampagneBody, ChoixFoyer, ChoixPersonne, ConfirmationReinscriptionBody, DecisionBody,
    PieceCampagneDto, PieceDecisionBody, ResolutionResponsable, ValidationBody,
};
use ent_web_api::{ApiClient, ApiError, ClientPreinscription};
use ent_web_app::modules::inscriptions::modele::{corps_preinscription, peut, SaisieFormulaire};
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
async fn pages_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    let http = reqwest::Client::new();
    for (chemin, attendu) in [
        ("/web/preinscription", "Votre adresse e-mail"),
        ("/web/invitation?jeton=abc", "Invitation"),
        ("/web/inscriptions", "Inscriptions"),
        ("/web/reinscription", "Réinscription"),
    ] {
        let r = http.get(format!("{base}{chemin}")).send().await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(
            r.headers().contains_key("content-security-policy"),
            "CSP {chemin}"
        );
        let html = r.text().await.unwrap();
        assert!(html.contains(attendu), "{chemin} : « {attendu} » attendu");
        assert!(
            !html.contains(" style=\""),
            "aucun style en ligne (CSP) {chemin}"
        );
    }
    // Rien d'une famille n'est rendu sans vérification.
    let html = http
        .get(format!("{base}/web/preinscription"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!html.contains("Mes demandes"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parcours_admission_et_reinscription_contre_le_backend() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let direction = personnel(&base, &l, "direction").await;
    let secretariat = personnel(&base, &l, "administratif").await;
    let prof = personnel(&base, &l, "prof").await;
    let lily = parent_de(&base, &l, "Harry").await;

    // Permissions : le professeur ne voit pas les inscriptions.
    assert!(matches!(
        prof.campagnes_inscription().await,
        Err(ApiError::Interdit(_))
    ));

    // Campagne d'admission ouverte par le secrétariat.
    let camp = secretariat
        .creer_campagne(&CampagneBody {
            nature: "admission".into(),
            libelle: "Rentrée test".into(),
            annee_cible: "2027-2028".into(),
            pieces: vec![PieceCampagneDto {
                code: "livret".into(),
                libelle: "Livret scolaire".into(),
                obligatoire: true,
            }],
            ..Default::default()
        })
        .await
        .expect("campagne");
    secretariat
        .ouvrir_campagne(camp.id)
        .await
        .expect("ouverture");

    // Démarrage public : même réponse pour une adresse connue ou inconnue.
    let public = ClientPreinscription::new(base.clone());
    let lily_email = l
        .personas
        .iter()
        .find(|p| p.role == "parent" && p.label.contains("Harry"))
        .unwrap()
        .email
        .clone();
    let a = public
        .demarrer("inconnu-web@exemple.test", Some(l.etablissement_id))
        .await
        .expect("démarrage");
    let b = public
        .demarrer(&lily_email, Some(l.etablissement_id))
        .await
        .expect("démarrage");
    assert_eq!(a.message, b.message);
    // Rien sans vérification.
    assert!(matches!(public.espace().await, Err(ApiError::NonConnecte)));
    assert!(public.verifier(a.demande_id, "12345").await.is_err());

    // Vérification par la session ouverte (variante passkey) : jeton en mémoire.
    let v = lily
        .post_json("/preinscription/session", serde_json::json!({}))
        .await
        .expect("jeton de session");
    let famille = public.avec_jeton(v["jeton"].as_str().unwrap().to_string());
    let espace = famille.espace().await.expect("espace");
    assert!(espace.connu);
    assert!(espace.campagnes.iter().any(|c| c.id == camp.id));

    // Dépôt depuis le modèle de l'écran (second parent séparé : e-mail obligatoire).
    let saisie = SaisieFormulaire {
        eleve_nom: "Potter".into(),
        eleve_prenom: format!("Albus{}", &Uuid::new_v4().simple().to_string()[..4]),
        resp_nom: "Potter".into(),
        resp_prenom: "Lily".into(),
        second: true,
        second_nom: "Potter".into(),
        second_email: format!("james-{}@exemple.test", Uuid::new_v4().simple()),
        second_meme_foyer: false,
        utiliser_mes_infos: true,
        ..Default::default()
    };
    let d = famille
        .deposer(&corps_preinscription(camp.id, &saisie))
        .await
        .expect("dépôt");
    assert_eq!(d.statut, "deposee");
    assert!(peut(&d, "deposer_piece"));
    famille
        .deposer_piece(d.id, "livret", "livret.pdf")
        .await
        .expect("pièce");

    // Instruction (secrétariat), décision et validation (direction).
    secretariat
        .decider_piece(
            d.id,
            "livret",
            &PieceDecisionBody {
                statut: "acceptee".into(),
                motif: None,
            },
        )
        .await
        .expect("pièce acceptée");
    let admis = DecisionBody {
        decision: "admis".into(),
        motif: None,
    };
    assert!(matches!(
        secretariat.decider_dossier(d.id, &admis).await,
        Err(ApiError::Interdit(_))
    ));
    let vu = direction
        .decider_dossier(d.id, &admis)
        .await
        .expect("admis");
    assert!(peut(&vu, "valider"));
    // Rapprochement humain (I0) : les correspondances sont présentées, la direction confirme
    // chacune ; sans ce choix, la validation est refusée.
    assert!(direction
        .valider_dossier(d.id, &ValidationBody::default())
        .await
        .is_err());
    let corr = direction
        .correspondances_dossier(d.id)
        .await
        .expect("correspondances");
    let responsables = corr
        .responsables
        .iter()
        .filter_map(|r| {
            r.candidats.first().map(|c| ResolutionResponsable {
                responsable_id: r.responsable_id,
                personne: ChoixPersonne::Existante {
                    personne_id: c.personne_id,
                },
                foyer: Some(match c.foyers.first() {
                    Some(f) => ChoixFoyer::Existant {
                        foyer_id: f.foyer_id,
                    },
                    None => ChoixFoyer::Nouveau,
                }),
            })
        })
        .collect();
    let v = direction
        .valider_dossier(
            d.id,
            &ValidationBody {
                responsables,
                ..Default::default()
            },
        )
        .await
        .expect("validation");
    assert_ne!(v.eleve_id, Uuid::nil());
    let suivi = famille.dossier(d.id).await.expect("suivi famille");
    assert_eq!(suivi.statut, "validee");
    assert!(peut(&suivi, "coordonnees_bancaires"));
    let cb = famille.coordonnees_bancaires(d.id).await.expect("banque");
    assert!(cb.mandat.is_none());

    // Réinscription : campagne ouverte, la famille confirme pour son enfant.
    let rei = secretariat
        .creer_campagne(&CampagneBody {
            nature: "reinscription".into(),
            libelle: "Réinscription test".into(),
            annee_cible: "2027-2028".into(),
            ..Default::default()
        })
        .await
        .expect("campagne de réinscription");
    let o = secretariat
        .ouvrir_campagne(rei.id)
        .await
        .expect("ouverture");
    assert!(o.dossiers_prepares > 0);
    let mes = lily.mes_reinscriptions().await.expect("réinscriptions");
    let harry = l.eleve("Harry").expect("Harry").id;
    let dh = mes
        .iter()
        .find(|x| x.eleve_id == Some(harry))
        .expect("dossier de Harry");
    let c = lily
        .confirmer_reinscription(
            dh.id,
            &ConfirmationReinscriptionBody {
                regime: Some("demi_pension".into()),
            },
        )
        .await
        .expect("confirmation");
    assert_eq!(c.statut, "deposee");
    assert!(matches!(
        prof.mes_reinscriptions().await,
        Err(ApiError::Interdit(_))
    ));
    direction
        .valider_dossier(dh.id, &ValidationBody::default())
        .await
        .expect("réinscription validée");
}
