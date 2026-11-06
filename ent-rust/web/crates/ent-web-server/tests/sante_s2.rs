//! Santé et accompagnement (socle 2, S2-11) contre le **vrai** backend (`ent_api::app`) et une
//! base PostgreSQL **locale** de test : pages rendues par le serveur, parcours infirmière →
//! professeur → famille, minimisation (le professeur ne reçoit que des aménagements et des
//! contraintes du catalogue), permissions refusées.
//!
//! Lancement (voir `contre_backend.rs`) :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test sante_s2 -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_types::sante::{PublierContrainteBody, SignerPaiBody};
use ent_web_api::{ApiClient, ApiError};
use ent_web_app::modules::accompagnement::modele::{
    composer_amenagement, composer_dispositif, SaisieAmenagement, SaisieDispositif,
};
use ent_web_app::modules::infirmerie::modele::{
    composer_pai, composer_passage, SaisiePai, SaisiePassage,
};
use ent_web_server::labo::{self, Labo};
use uuid::Uuid;

const MOTIF_SECRET: &str = "SENT-motif-confidentiel-web";
const NOTE_SECRET: &str = "SENT-note-clinique-web";

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

async fn par_role(base: &str, l: &Labo, role: &str) -> ApiClient {
    let email = l
        .personnel
        .iter()
        .find(|p| p.role == role)
        .map(|p| p.email.clone())
        .or_else(|| {
            l.personas
                .iter()
                .find(|p| p.role == role)
                .map(|p| p.email.clone())
        })
        .unwrap_or_else(|| panic!("compte {role} de labo"));
    client_de(base, l, &email).await
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
async fn pages_sante_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    for (chemin, titre) in [
        ("/web/infirmerie", "Infirmerie"),
        ("/web/accompagnement", "Accompagnement"),
        ("/web/amenagements", "Aménagements"),
        ("/web/sante-enfant", "Santé de mon enfant"),
        (
            "/web/infirmerie/eleves/00000000-0000-0000-0000-000000000001/urgence",
            "Fiche d",
        ),
    ] {
        let r = reqwest::get(format!("{base}{chemin}")).await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(
            r.headers().contains_key("content-security-policy"),
            "CSP {chemin}"
        );
        let html = r.text().await.unwrap();
        assert!(html.contains(titre), "{chemin} : titre « {titre} » absent");
        assert!(
            !html.contains(" style=\""),
            "{chemin} : style en ligne (CSP)"
        );
        // Aucune donnée de santé dans la coquille rendue côté serveur.
        assert!(!html.contains("SENT-"), "{chemin}");
    }
}

/// Infirmière → professeur → famille : le PAI est saisi et signé, la contrainte publiée sans
/// diagnostic, l'aménagement visible du professeur ; chaque rôle ne voit que sa part.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parcours_pai_contraintes_amenagements_et_permissions() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let harry = l.eleve("Harry").expect("Harry").id;
    let infirmiere = par_role(&base, &l, "infirmerie").await;
    let direction = par_role(&base, &l, "direction").await;
    let prof = par_role(&base, &l, "prof").await;
    let compta = par_role(&base, &l, "compta").await;
    let lily = parent_de(&base, &l, "Harry").await;

    // 1. L'infirmière enregistre le PAI (formulaire validé par le modèle d'écran).
    let corps_pai = composer_pai(&SaisiePai {
        motif: MOTIF_SECRET.into(),
        mesures: "Trousse à portée de main".into(),
        protocole: "Appeler le 15".into(),
        note_interne: NOTE_SECRET.into(),
        debut: "2026-09-01".into(),
        actif: true,
        trousse: true,
        trousse_lieu: "Infirmerie, armoire rouge".into(),
        ..Default::default()
    })
    .expect("saisie valide");
    let pai = infirmiere
        .enregistrer_pai(harry, &corps_pai)
        .await
        .expect("PAI")
        .expect("PAI renvoyé");
    assert!(pai.trousse_urgence && pai.pai.actif);
    let v = pai.version;

    // 2. Contrainte publiée (catalogue) : pour les enseignants et les sorties, jamais le motif.
    infirmiere
        .publier_contrainte(
            harry,
            &PublierContrainteBody {
                code: "ne_pas_courir".into(),
                audiences: vec!["enseignants".into(), "sorties".into()],
                date_fin: None,
            },
        )
        .await
        .expect("contrainte");
    assert!(matches!(
        infirmiere
            .publier_contrainte(
                harry,
                &PublierContrainteBody {
                    code: "asthme_severe".into(),
                    audiences: vec!["enseignants".into()],
                    date_fin: None,
                },
            )
            .await,
        Err(ApiError::Invalide(_)) | Err(ApiError::Interdit(_))
    ));
    let vue_prof = prof
        .contraintes_liste("enseignants")
        .await
        .expect("contraintes prof");
    let harry_prof = vue_prof
        .iter()
        .find(|e| e.eleve_id == harry)
        .expect("Harry chez le prof");
    assert!(harry_prof
        .contraintes
        .iter()
        .any(|c| c.code == "ne_pas_courir"));
    let json = serde_json::to_string(&vue_prof).unwrap();
    assert!(
        !json.contains(MOTIF_SECRET)
            && !json.contains(NOTE_SECRET)
            && !json.contains("Appeler le 15")
    );
    assert!(
        harry_prof.contraintes.iter().all(|c| c.source.is_none()),
        "la source reste interne à la santé"
    );

    // 3. Le professeur ne lit ni le PAI, ni les passages, ni le registre ; il a la fiche minimale.
    for refus in [
        prof.pai_complet(harry).await.map(|_| ()),
        prof.passages_du_jour(chrono::Local::now().date_naive())
            .await
            .map(|_| ()),
        prof.protocoles_sante().await.map(|_| ()),
        prof.dossier_accompagnement(harry).await.map(|_| ()),
    ] {
        assert!(matches!(refus, Err(ApiError::Interdit(_))), "{refus:?}");
    }
    let min = prof
        .fiche_urgence(harry)
        .await
        .expect("fiche d'urgence minimale");
    assert_eq!(min.niveau, "minimale");
    assert!(min.protocole_urgence.is_none() && min.medecin_nom.is_none());
    assert!(!serde_json::to_string(&min).unwrap().contains(MOTIF_SECRET));
    let complete = infirmiere
        .fiche_urgence(harry)
        .await
        .expect("fiche complète");
    assert_eq!(complete.niveau, "complete");
    assert_eq!(complete.protocole_urgence.as_deref(), Some("Appeler le 15"));
    // La comptabilité (restauration) n'a aucune fiche d'urgence ni aménagement.
    assert!(matches!(
        compta.fiche_urgence(harry).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        compta.mes_amenagements().await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        compta.contraintes_liste("enseignants").await,
        Err(ApiError::Interdit(_))
    ));
    let cantine = compta
        .contraintes_liste("restauration")
        .await
        .expect("liste restauration");
    assert!(cantine
        .iter()
        .all(|e| e.contraintes.iter().all(|c| c.code != "ne_pas_courir")));

    // 4. Signatures : la famille signe depuis son compte ; un autre responsable est refusé.
    let signe = lily
        .signer_pai(
            harry,
            &SignerPaiBody {
                partie: "famille".into(),
                signataire_nom: None,
                version: None,
            },
        )
        .await
        .expect("signature famille");
    assert!(signe.signatures.iter().any(|s| s.partie == "famille"));
    assert!(
        signe.pai.note_interne.is_empty(),
        "la note interne ne sort jamais vers la famille"
    );
    assert!(matches!(
        lily.signer_pai(
            harry,
            &SignerPaiBody {
                partie: "famille".into(),
                signataire_nom: None,
                version: None
            }
        )
        .await,
        Err(ApiError::Conflit(_)) | Err(ApiError::Invalide(_))
    ));
    assert!(matches!(
        lily.signer_pai(
            harry,
            &SignerPaiBody {
                partie: "medecin".into(),
                signataire_nom: Some("Dr X".into()),
                version: None
            }
        )
        .await,
        Err(ApiError::Interdit(_))
    ));
    let autre = parent_de(&base, &l, "Draco").await;
    assert!(matches!(
        autre
            .signer_pai(
                harry,
                &SignerPaiBody {
                    partie: "famille".into(),
                    signataire_nom: None,
                    version: None
                }
            )
            .await,
        Err(ApiError::Interdit(_))
    ));
    infirmiere
        .signer_pai(
            harry,
            &SignerPaiBody {
                partie: "medecin".into(),
                signataire_nom: Some("Dr Chourave".into()),
                version: None,
            },
        )
        .await
        .expect("médecin");
    // Modifier le PAI : nouvelle version, signatures à refaire.
    let apres = infirmiere
        .enregistrer_pai(harry, &corps_pai)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(apres.version, v + 1);
    assert!(apres.signatures.is_empty() && !apres.complet);
    assert!(matches!(
        autre.pai_complet(harry).await,
        Err(ApiError::Interdit(_))
    ));

    // 5. Accompagnement : la direction ouvre un PAP et définit un aménagement ; le professeur
    // de l'élève voit l'aménagement, jamais le dispositif.
    let dispositif = direction
        .ouvrir_dispositif(
            harry,
            &composer_dispositif(&SaisieDispositif {
                kind: "pap".into(),
                libelle: "Suivi renforcé".into(),
                ..Default::default()
            })
            .expect("saisie"),
        )
        .await
        .expect("PAP");
    direction
        .definir_amenagement(
            harry,
            &composer_amenagement(&SaisieAmenagement {
                code: "tiers_temps".into(),
                portee: "examen".into(),
                dispositif: dispositif.id.to_string(),
                ..Default::default()
            })
            .expect("saisie"),
        )
        .await
        .expect("aménagement");
    let mes = prof.mes_amenagements().await.expect("mes aménagements");
    let e = mes
        .iter()
        .find(|e| e.eleve_id == harry)
        .expect("Harry aménagé");
    assert!(e
        .amenagements
        .iter()
        .any(|a| a.code == "tiers_temps" && a.portee == "examen"));
    let json = serde_json::to_string(&mes).unwrap().to_lowercase();
    assert!(
        !json.contains("suivi renforcé") && !json.contains("\"kind\"") && !json.contains("pap")
    );
    // La famille voit les aménagements de son enfant ; l'autre famille non.
    assert!(!lily.amenagements_eleve(harry).await.unwrap().is_empty());
    assert!(matches!(
        autre.amenagements_eleve(harry).await,
        Err(ApiError::Interdit(_))
    ));
    // Un dispositif inconnu du catalogue d'aménagements est refusé.
    let hors = ent_types::sante::DefinirAmenagementBody {
        code: "dyslexie_severe".into(),
        portee: "cours".into(),
        dispositif_id: None,
        date_fin: None,
    };
    assert!(direction.definir_amenagement(harry, &hors).await.is_err());
    let _ = Uuid::nil();

    // 6. Passage : la famille informée par une notification sans détail ; la vue famille masque la note.
    let passage = infirmiere
        .creer_passage_sante(
            &composer_passage(&SaisiePassage {
                eleve: Some(harry),
                jour: chrono::Local::now().date_naive().to_string(),
                motif: "accident".into(),
                note_soins: NOTE_SECRET.into(),
                visible_famille: true,
                issue: "retour_classe".into(),
                ..Default::default()
            })
            .expect("saisie"),
        )
        .await
        .expect("passage");
    assert_eq!(passage.famille_information, "informee");
    let famille = lily.passages_eleve(harry).await.expect("passages famille");
    assert!(famille.iter().any(|p| p.id == passage.id));
    assert!(famille
        .iter()
        .all(|p| p.note_soins.is_empty() && p.soins.is_empty()));
    assert!(matches!(
        lily.passages_du_jour(chrono::Local::now().date_naive())
            .await,
        Err(ApiError::Interdit(_))
    ));
}
