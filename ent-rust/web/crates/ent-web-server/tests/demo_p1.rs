//! Démo P1 : le parcours joué à l'écran (admission → fiche 360°) contre le **vrai** backend et
//! une base PostgreSQL **locale** de test (données de labo fictives). Les écrans appellent ces
//! mêmes routes ; les fonctions de modèle d'écran sont utilisées telles quelles.
//!
//! Lancement :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test demo_p1 -- --test-threads=1
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_web_api::types_inscriptions::{
    CampagneBody, DecisionBody, NiveauCampagneDto, PieceCampagneDto, PieceDecisionBody,
};
use ent_web_api::{ApiClient, ApiError, ClientPreinscription};
use ent_web_app::modules::inscriptions::modele::{
    construire_validation, corps_preinscription, peut, ChoixSaisi, SaisieFormulaire,
};
use ent_web_server::labo::{self, Labo};
use serde_json::json;
use uuid::Uuid;

async fn demarrer() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent");
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

fn refuse<T: std::fmt::Debug>(r: Result<T, ApiError>) -> bool {
    matches!(r, Err(ApiError::Interdit(_) | ApiError::Introuvable(_)))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pages_de_la_demo_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let http = reqwest::Client::new();
    let harry = l.eleve("Harry").expect("Harry").id;
    for chemin in [
        format!("/web/preinscription?etab={}", l.etablissement_id),
        "/web/inscriptions".to_string(),
        format!("/web/eleves/{harry}"),
    ] {
        let r = http.get(format!("{base}{chemin}")).send().await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        let html = r.text().await.unwrap();
        assert!(
            !html.contains(" style=\""),
            "aucun style en ligne : {chemin}"
        );
    }
}

/// Admission jusqu'à la fiche : réserves déclarées par la famille, instruction par le
/// secrétariat, décision de la direction, **validation par le secrétariat** avec rapprochement
/// humain, puis scolarité à la rentrée, fiche 360° et cloisonnement (parent, professeur).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admission_jusqu_a_la_fiche_et_cloisonnement() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let direction = personnel(&base, &l, "direction").await;
    let secretariat = personnel(&base, &l, "administratif").await;
    let prof = personnel(&base, &l, "prof").await;
    let lily_email = l
        .personas
        .iter()
        .find(|p| p.role == "parent" && p.label.contains("Harry"))
        .unwrap()
        .email
        .clone();
    let lily = client_de(&base, &l, &lily_email).await;
    let narcissa_email = l
        .personas
        .iter()
        .find(|p| p.role == "parent" && p.label.contains("Draco"))
        .unwrap()
        .email
        .clone();
    let narcissa = client_de(&base, &l, &narcissa_email).await;

    // 1. Secrétariat : campagne d'admission ouverte.
    let camp = secretariat
        .creer_campagne(&CampagneBody {
            nature: "admission".into(),
            libelle: format!("Rentrée démo {}", &Uuid::new_v4().simple().to_string()[..6]),
            annee_cible: "2027-2028".into(),
            niveaux: vec![NiveauCampagneDto {
                code: "6EME".into(),
                capacite: Some(30),
            }],
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

    // 2. Famille : préinscription avec un second parent « à ne pas solliciter » et une adresse
    //    à protéger (formulaire de l'écran → corps envoyé).
    let public = ClientPreinscription::new(base.clone());
    let v = lily
        .post_json("/preinscription/session", json!({}))
        .await
        .expect("jeton de session");
    let famille = public.avec_jeton(v["jeton"].as_str().unwrap().to_string());
    let prenom = format!("Rose{}", &Uuid::new_v4().simple().to_string()[..4]);
    let saisie = SaisieFormulaire {
        eleve_nom: "Potter".into(),
        eleve_prenom: prenom.clone(),
        niveau: "6EME".into(),
        resp_nom: "Potter".into(),
        resp_prenom: "Lily".into(),
        resp_lien: "mere".into(),
        resp_adresse_protegee: true,
        utiliser_mes_infos: true,
        second: true,
        second_nom: "Potter".into(),
        second_prenom: "James".into(),
        second_lien: "pere".into(),
        second_email: format!("james-{}@exemple.test", Uuid::new_v4().simple()),
        second_meme_foyer: false,
        second_ne_pas_solliciter: true,
        ..Default::default()
    };
    let d = famille
        .deposer(&corps_preinscription(camp.id, &saisie))
        .await
        .expect("dépôt");
    famille
        .deposer_piece(d.id, "livret", "livret.pdf")
        .await
        .expect("pièce");

    // 3. Secrétariat : instruit ; les réserves déclarées sont visibles, jamais d'adresse.
    let vu = secretariat
        .dossier_inscription(d.id)
        .await
        .expect("dossier");
    let second = vu.responsables.iter().find(|r| r.rang == 2).unwrap();
    assert!(second
        .formulaire
        .reserve
        .as_ref()
        .is_some_and(|r| r.ne_pas_solliciter));
    assert!(vu
        .responsables
        .iter()
        .find(|r| r.rang == 1)
        .unwrap()
        .formulaire
        .reserve
        .as_ref()
        .is_some_and(|r| r.adresse_a_proteger));
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
    // Direction : admet (le secrétariat ne décide pas).
    let admis = DecisionBody {
        decision: "admis".into(),
        motif: None,
    };
    assert!(refuse(secretariat.decider_dossier(d.id, &admis).await));
    let apres = direction
        .decider_dossier(d.id, &admis)
        .await
        .expect("admis");
    assert!(peut(&apres, "valider"));
    assert!(refuse(prof.dossier_inscription(d.id).await));

    // 3 bis. Direction : l'année 2027-2028 existe dans le référentiel (« préparer l'année
    //        suivante », écran Référentiel › Années) ; sans elle la scolarité attend (RP2-10).
    let annees = direction.annees_statut().await.expect("années");
    if !annees.iter().any(|a| a.libelle == "2027-2028") {
        let courante = annees
            .iter()
            .find(|a| a.courante)
            .expect("année courante")
            .id;
        direction
            .preparer_annee_suivante(courante, false)
            .await
            .expect("préparation de l'année suivante");
    }

    // 4. Secrétariat : valide avec le rapprochement humain construit par le modèle de l'écran.
    let corr = secretariat
        .correspondances_dossier(d.id)
        .await
        .expect("correspondances (secrétariat)");
    let choix: Vec<ChoixSaisi> = corr
        .responsables
        .iter()
        .map(|r| ChoixSaisi {
            personne: r
                .candidats
                .first()
                .map(|c| c.personne_id.to_string())
                .unwrap_or_default(),
            foyer: r
                .candidats
                .first()
                .and_then(|c| c.foyers.first())
                .map(|f| f.foyer_id.to_string())
                .unwrap_or_default(),
        })
        .collect();
    let corps = construire_validation(&corr, &choix, "", false).expect("choix complets");
    let v = secretariat
        .valider_dossier(d.id, &corps)
        .await
        .expect("validation par le secrétariat");
    let eleve = v.eleve_id;
    assert!(v.inscription_annuelle_id.is_some());

    // 5. L'élève apparaît dans la scolarité, scolarisation à la rentrée (pas de classe encore).
    let sc = direction.scolarite_eleve(eleve).await.expect("scolarité");
    assert_eq!(sc.statut, "a_venir");
    assert_eq!(sc.scolarisations.len(), 1);
    assert!(sc.inscriptions.iter().any(|i| i.etat == "prevue"));

    // 6. Fiche 360° : le père « à ne pas solliciter » n'a reçu aucune invitation.
    let fiche = direction.fiche_dossier(eleve).await.expect("fiche");
    assert_eq!(fiche.nom, "Potter");
    // N° 4 : la mention « ne pas solliciter » du père, à confirmer, est une alerte de la fiche
    // (direction et secrétariat), sans référence ni adresse.
    for qui in [&direction, &secretariat] {
        let f = qui.fiche_dossier(eleve).await.expect("fiche du personnel");
        assert_eq!(f.protections.len(), 1, "une seule mention : {:?}", f.protections);
        assert!(!f.protections[0].confirmee);
    }
    assert!(prof.fiche_dossier(eleve).await.is_err(), "le professeur sans classe n'ouvre rien");
    assert!(!fiche.responsables.is_empty());
    let suivi = direction.dossier_inscription(d.id).await.unwrap();
    let inv2 = suivi
        .responsables
        .iter()
        .find(|r| r.rang == 2)
        .unwrap()
        .invitation
        .clone();
    assert!(
        inv2.is_none(),
        "aucune invitation au parent protégé : {inv2:?}"
    );

    // 7. Cloisonnement : une autre famille et un professeur sans classe n'ouvrent rien.
    assert!(refuse(narcissa.fiche_dossier(eleve).await));
    assert!(refuse(prof.fiche_dossier(eleve).await));
    assert!(refuse(prof.scolarite_eleve(eleve).await));
}

/// Le professeur d'un seul groupe ne voit que les élèves de ce groupe (D-022).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn professeur_d_un_groupe_ne_voit_que_son_groupe() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let direction = personnel(&base, &l, "direction").await;
    let prof3 = l
        .personnel
        .iter()
        .find(|p| p.role == "prof" && p.label == "Prof 3")
        .expect("Prof 3");
    let prof = client_de(&base, &l, &prof3.email).await;
    let harry = l.eleve("Harry").unwrap().id;
    let hermione = l.eleve("Hermione").unwrap().id;
    let ginny = l.eleve("Ginny").unwrap().id;

    let classes = direction
        .classes(chrono::Utc::now().date_naive())
        .await
        .expect("classes");
    let classe_4b = classes
        .iter()
        .find(|c| c.code == "4B" || c.libelle.contains("4B"))
        .expect("4B")
        .id;
    let code = format!("G{}", &Uuid::new_v4().simple().to_string()[..5]);
    let g = direction
        .creer_groupe(&code, Some("4B LV2 allemand"), Some(classe_4b), None)
        .await
        .expect("groupe");
    let groupe: Uuid = g["id"].as_str().unwrap().parse().unwrap();
    let matieres = direction.get_json("/matieres").await.expect("matières");
    let matiere = matieres.as_array().unwrap()[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    direction
        .post_json(
            "/referentiel/services",
            json!({"prof_user_id": prof3.user_id, "groupe_id": groupe, "matiere_id": matiere}),
        )
        .await
        .expect("service du groupe");
    let annees = direction.annees_statut().await.expect("années");
    let annee = annees
        .iter()
        .find(|a| a.courante)
        .expect("année courante")
        .id;
    for e in [harry, hermione] {
        direction
            .post_json(
                &format!("/eleves/{e}/scolarite/options"),
                json!({"annee_id": annee, "ajouter": [groupe], "date_effet": chrono::Utc::now().date_naive()}),
            )
            .await
            .expect("affectation au groupe");
    }

    assert!(prof.fiche_dossier(harry).await.is_ok());
    assert!(prof.fiche_dossier(hermione).await.is_ok());
    assert!(refuse(prof.fiche_dossier(ginny).await));
    assert!(refuse(prof.scolarite_eleve(ginny).await));
}

/// Démo P1 n° 3 : la famille lit la scolarité (minimisée) et l'INE de SON enfant ; une autre
/// famille, un responsable à lien suspendu, un responsable « ne pas solliciter » et l'élève
/// n'y accèdent pas. Droits vérifiés par le serveur.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn la_famille_lit_la_scolarite_de_son_enfant() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let direction = personnel(&base, &l, "direction").await;
    let harry = l.eleve("Harry").expect("Harry").id;
    let email_de = |fragment: &str| {
        l.personas
            .iter()
            .find(|p| p.role == "parent" && p.label.contains(fragment))
            .unwrap_or_else(|| panic!("parent {fragment}"))
            .email
            .clone()
    };
    let lily_email = email_de("Harry");
    let lily = client_de(&base, &l, &lily_email).await;
    let narcissa = client_de(&base, &l, &email_de("Draco")).await;

    // Contrôle positif : le personnel lit, la mère aussi (même statut), sans le réservé au personnel.
    let personnel_vu = direction.scolarite_eleve(harry).await.expect("direction");
    let famille_vue = lily.scolarite_eleve(harry).await.expect("la mère lit la scolarité");
    assert_eq!(famille_vue.statut, personnel_vu.statut);
    assert_eq!(famille_vue.scolarisations.len(), personnel_vu.scolarisations.len());
    for s in &famille_vue.scolarisations {
        assert!(s.motif_sortie.is_none() && s.motif_sortie_libre.is_none());
        assert!(s.exeat_document_id.is_none());
        assert!(s.destination.uai.is_none() && s.destination.libelle.is_none());
        assert!(s.origine.uai.is_none() && s.origine.libelle.is_none());
    }
    assert!(famille_vue.inscriptions.iter().all(|i| i.decision_passage.is_none()));

    // L'INE vient de la fiche (groupe G3, ouvert aux familles pour leur enfant).
    let ine_famille = lily.fiche_dossier(harry).await.expect("fiche famille").ine;
    let ine_personnel = direction.fiche_dossier(harry).await.expect("fiche").ine;
    assert_eq!(ine_famille, ine_personnel);

    // Autre famille, élève : refus.
    assert!(refuse(narcissa.scolarite_eleve(harry).await));
    if let Some(p) = l.personas.iter().find(|p| p.role == "eleve" && p.label.contains("Harry")) {
        let eleve = client_de(&base, &l, &p.email).await;
        assert!(refuse(eleve.scolarite_eleve(harry).await));
    }

    // La mère telle que la voit la direction : personne et lien.
    let fiche = direction.fiche_dossier(harry).await.expect("fiche");
    let mere = fiche
        .responsables
        .iter()
        .find(|r| r.personne.email.as_deref() == Some(lily_email.as_str()))
        .expect("la mère parmi les responsables");

    // « Ne pas solliciter » : plus d'accès tant que la mention est posée.
    direction
        .definir_ne_pas_solliciter(mere.personne.id, true, Some("réf. démo".into()))
        .await
        .expect("mention posée");
    assert!(refuse(lily.scolarite_eleve(harry).await));
    let vue_direction = direction.fiche_dossier(harry).await.expect("fiche");
    assert_eq!(vue_direction.protections.len(), 1);
    assert!(vue_direction.protections[0].confirmee);
    // Jamais visible de la famille, même pour sa propre mention.
    let vue_mere = lily.fiche_dossier(harry).await.expect("fiche de la mère");
    assert!(vue_mere.protections.is_empty());
    // Tableau de bord de la direction : les mentions à confirmer ; la famille n'y a pas accès.
    assert!(direction.alertes_protection().await.is_ok());
    assert!(refuse(lily.alertes_protection().await));
    direction
        .definir_ne_pas_solliciter(mere.personne.id, false, None)
        .await
        .ok();

    // Lien suspendu : refus ; rétabli : accès.
    direction
        .suspendre_lien(mere.lien_id, "Mesure de démonstration".into(), None)
        .await
        .expect("suspension");
    assert!(refuse(lily.scolarite_eleve(harry).await));
    direction.retablir_lien(mere.lien_id).await.expect("rétablissement");
}
