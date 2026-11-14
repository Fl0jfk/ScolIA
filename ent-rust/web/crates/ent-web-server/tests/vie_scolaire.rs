//! Vie scolaire contre le **vrai** backend (`ent_api::app`) et une base PostgreSQL
//! **locale** de test (données de labo fictives) : absences issues d'un appel clos,
//! justification par la famille, décision du personnel, permissions, signaux.
//!
//! Lancement (voir `contre_backend.rs`) :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test vie_scolaire -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use chrono::{Datelike, Duration, NaiveDate};
use ent_web_api::{Absence, ApiClient, ApiError, DecisionApi, EtatApi};
use ent_web_app::composants_metier::absences::StatutAbsence;
use ent_web_app::modules::absences::modele::{
    acces, composer_justification, options_motifs, peut_justifier, Acces,
};
use ent_web_app::modules::vie_scolaire::modele::{decisions_possibles, fusionner, Decision};
use ent_web_app::noyau::droits::droits_du_role;
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

/// Un jour futur, toujours un lundi (jour ouvré explicite), différent à chaque exécution :
/// chaque test ouvre des appels neufs (un appel clos ne peut plus être modifié).
fn jour_neuf() -> NaiveDate {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;
    lundi_a_venir() + Duration::weeks(n.rem_euclid(2000))
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

/// Parent de labo dont le libellé de persona cite cet enfant.
async fn parent_de(base: &str, l: &Labo, prenom: &str) -> ApiClient {
    let p = l
        .personas
        .iter()
        .find(|p| p.role == "parent" && p.label.contains(prenom))
        .unwrap_or_else(|| panic!("parent de {prenom}"));
    client_de(base, l, &p.email).await
}

fn id_eleve(l: &Labo, prenom: &str) -> Uuid {
    l.eleve(prenom)
        .unwrap_or_else(|| panic!("élève {prenom} dans la distribution de labo"))
        .id
}

async fn trouver(api: &ApiClient, jour: NaiveDate, eleve: Uuid) -> Option<Absence> {
    api.absences_a_traiter(jour)
        .await
        .expect("liste VS")
        .into_iter()
        .find(|a| a.eleve_id == eleve)
}

/// Ouvre l'appel 4B du professeur de labo ce jour-là, marque Harry et Hermione absents et
/// Ginny en retard, puis clôt. Renvoie le jour.
async fn appel_clos_4b(base: &str, l: &Labo) -> NaiveDate {
    let jour = jour_neuf();
    let p = l
        .personnel
        .iter()
        .find(|p| p.role == "prof")
        .expect("prof de labo");
    let prof = client_de(base, l, &p.email).await;
    let c = prof
        .mes_creneaux(jour)
        .await
        .expect("créneaux")
        .into_iter()
        .find(|c| c.classe == "4B")
        .expect("séance 4B");
    let appel = prof.ouvrir_appel(c.id, jour).await.expect("ouverture");
    for (prenom, etat) in [
        ("Harry", EtatApi::Absent),
        ("Hermione", EtatApi::Absent),
        ("Ginny", EtatApi::Retard),
    ] {
        prof.marquer(appel.id, id_eleve(l, prenom), etat)
            .await
            .unwrap_or_else(|e| panic!("marquer {prenom} : {e:?}"));
    }
    assert!(prof.clore(appel.id).await.expect("clôture").est_clos());
    jour
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pages_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    let http = reqwest::Client::new();
    for (chemin, titre) in [
        ("/web/vie-scolaire", "Vie scolaire"),
        ("/web/absences", "Absences"),
    ] {
        let r = http.get(format!("{base}{chemin}")).send().await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(r.headers().contains_key("content-security-policy"), "CSP");
        let html = r.text().await.unwrap();
        assert!(html.contains(titre), "{chemin} : titre");
        assert!(html.contains(r#"<html lang="fr""#), "{chemin} : langue");
        assert!(
            html.contains("Chargement"),
            "{chemin} : état de chargement rendu côté serveur"
        );
        assert!(
            !html.contains(" style=\""),
            "{chemin} : aucun style en ligne (CSP)"
        );
    }
}

/// Parcours complet : appel clos → absences à traiter → la famille justifie → le CPE
/// accepte ; l'autre absence est refusée puis annulée ; le retard est visible.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn appel_clos_justification_famille_decision_cpe() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let jour = appel_clos_4b(&base, &l).await;
    let (harry, hermione, ginny) = (
        id_eleve(&l, "Harry"),
        id_eleve(&l, "Hermione"),
        id_eleve(&l, "Ginny"),
    );
    let cpe = personnel(&base, &l, "cpe").await;

    // 1. Les absents de l'appel clos arrivent dans « Absences à traiter », nom fourni.
    let a_harry = trouver(&cpe, jour, harry).await.expect("absence de Harry");
    assert_eq!(a_harry.eleve_label.as_deref(), Some("Harry Potter"));
    assert_eq!((a_harry.debut, a_harry.fin), (jour, jour));
    assert_eq!(StatutAbsence::depuis(&a_harry), StatutAbsence::NonJustifiee);
    let a_hermione = trouver(&cpe, jour, hermione).await.expect("Hermione");
    assert!(
        trouver(&cpe, jour, ginny).await.is_none(),
        "un retard n'est pas une absence"
    );

    // 2. Le retard de Ginny est un fait distinct, lisible par la vie scolaire.
    let retards = cpe.retards_eleve(ginny).await.expect("retards de Ginny");
    assert!(retards.iter().any(|r| r.jour == jour), "{retards:?}");

    // 3. La famille de Harry (peut justifier) voit l'absence et la justifie.
    let lily = parent_de(&base, &l, "Harry").await;
    let enfants = lily.enfants_famille().await.expect("enfants");
    let enfant = enfants.iter().find(|e| e.id == harry).expect("Harry");
    assert_eq!(acces(enfant), Acces::Complet);
    assert!(enfant.peut_justifier_absence);
    let vues = lily.absences_eleve(harry).await.expect("absences de Harry");
    let vue = vues
        .iter()
        .find(|a| a.id == a_harry.id)
        .expect("absence vue");
    assert!(peut_justifier(enfant, vue));
    let motifs = lily.motifs_absence().await.expect("motifs");
    let options = options_motifs(&motifs);
    assert_eq!(options.last().map(|(c, _)| c.as_str()), Some("autre"));
    let texte = composer_justification(&motifs, &options[0].0, "Fièvre depuis la veille")
        .or_else(|_| composer_justification(&motifs, "autre", "Fièvre depuis la veille"))
        .expect("motif composé");
    let apres = lily
        .justifier_absence(a_harry.id, texte.clone())
        .await
        .expect("justification");
    assert_eq!(apres.motif_justif.as_deref(), Some(texte.as_str()));
    assert_eq!(
        StatutAbsence::depuis(&apres),
        StatutAbsence::JustificatifEnAttente
    );

    // 4. Le CPE voit le justificatif en attente, avec le texte déposé.
    let en_attente = trouver(&cpe, jour, harry)
        .await
        .expect("toujours à traiter");
    assert_eq!(
        StatutAbsence::depuis(&en_attente),
        StatutAbsence::JustificatifEnAttente
    );
    assert_eq!(en_attente.motif_justif.as_deref(), Some(texte.as_str()));
    let droits_cpe = droits_du_role("cpe");
    assert_eq!(
        decisions_possibles(StatutAbsence::JustificatifEnAttente, droits_cpe),
        [
            Decision::Accepter,
            Decision::Refuser,
            Decision::AnnulerPresent
        ]
    );

    // 5. Le parent peut retirer puis redéposer sa justification tant qu'elle est en attente.
    let retiree = lily
        .retirer_justificatif(a_harry.id)
        .await
        .expect("retrait");
    assert_eq!(StatutAbsence::depuis(&retiree), StatutAbsence::NonJustifiee);
    lily.justifier_absence(a_harry.id, texte.clone())
        .await
        .expect("nouvelle justification");

    // 6. Le CPE accepte : l'absence est justifiée et sort de la liste « à traiter »,
    //    mais l'écran la garde avec son nouveau statut (fusion de session).
    let decidee = cpe
        .decider_absence(
            a_harry.id,
            DecisionApi::Accepter,
            Some("Certificat reçu".into()),
        )
        .await
        .expect("acceptation");
    assert_eq!(StatutAbsence::depuis(&decidee), StatutAbsence::Justifiee);
    assert_eq!(decidee.decision_note.as_deref(), Some("Certificat reçu"));
    assert!(decidee.eleve_label.is_none(), "la réponse n'a pas le nom");
    assert!(
        trouver(&cpe, jour, harry).await.is_none(),
        "sortie de la liste serveur"
    );
    let affichee = fusionner(
        cpe.absences_a_traiter(jour).await.unwrap(),
        &[ent_web_app::modules::vie_scolaire::modele::appliquer_decision(&a_harry, decidee)],
        jour,
    );
    let ligne = affichee
        .iter()
        .find(|a| a.id == a_harry.id)
        .expect("gardée");
    assert_eq!(ligne.eleve_label.as_deref(), Some("Harry Potter"));
    assert_eq!(StatutAbsence::depuis(ligne), StatutAbsence::Justifiee);
    // La famille voit la décision, sans pouvoir rejustifier.
    let vue = lily
        .absences_eleve(harry)
        .await
        .unwrap()
        .into_iter()
        .find(|a| a.id == a_harry.id)
        .unwrap();
    assert_eq!(StatutAbsence::depuis(&vue), StatutAbsence::Justifiee);
    assert!(!peut_justifier(enfant, &vue));
    assert!(
        lily.justifier_absence(a_harry.id, "autre".into())
            .await
            .is_err(),
        "une absence déjà justifiée ne se rejustifie pas"
    );

    // 7. Le CPE refuse l'absence d'Hermione (motif), puis la remet « présent » (erreur d'appel).
    let refusee = cpe
        .decider_absence(
            a_hermione.id,
            DecisionApi::Refuser,
            Some("Pas de pièce".into()),
        )
        .await
        .expect("refus");
    assert_eq!(StatutAbsence::depuis(&refusee), StatutAbsence::Refusee);
    assert_eq!(refusee.decision_note.as_deref(), Some("Pas de pièce"));
    let annulee = cpe
        .decider_absence(a_hermione.id, DecisionApi::AnnulerPresent, None)
        .await
        .expect("annulation");
    assert_eq!(
        StatutAbsence::depuis(&annulee),
        StatutAbsence::AnnuleePresent
    );
    assert!(trouver(&cpe, jour, hermione).await.is_none());

    // 8. Contrat JSON : la réponse réelle se relit à l'identique dans le type partagé.
    let brut = cpe
        .get_json(&format!("/vs/absences?jour={jour}"))
        .await
        .unwrap();
    let typee: Vec<Absence> = serde_json::from_value(brut.clone()).unwrap();
    assert_eq!(serde_json::to_value(&typee).unwrap(), brut);
}

/// Permissions : parent sans drapeau, parent d'un autre foyer, enseignant, élève.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn permissions_famille_eleve_et_personnel() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let (harry, draco) = (id_eleve(&l, "Harry"), id_eleve(&l, "Draco"));
    let cpe = personnel(&base, &l, "cpe").await;

    // Absences déclarées par la vie scolaire (jours distincts, loin dans le futur).
    let jour = jour_neuf();
    let motif = cpe.motifs_absence().await.expect("motifs")[0].code.clone();
    let declarer = |eleve: Uuid, j: NaiveDate| {
        let (cpe, motif) = (cpe.clone(), motif.clone());
        async move {
            let v = cpe
                .post_json(
                    &format!("/eleves/{eleve}/absences"),
                    serde_json::json!({ "debut": j, "fin": j, "motif": motif }),
                )
                .await
                .expect("déclaration");
            serde_json::from_value::<Absence>(v).expect("absence")
        }
    };
    let a_draco = declarer(draco, jour).await;
    let a_harry = declarer(harry, jour + Duration::days(1)).await;

    let narcissa = parent_de(&base, &l, "Draco").await;
    let lily = parent_de(&base, &l, "Harry").await;

    // Parent SANS drapeau (Draco : ni justifier ni voir la vie scolaire).
    let enfants = narcissa.enfants_famille().await.expect("enfants");
    let e = enfants.iter().find(|e| e.id == draco).expect("Draco");
    assert!(!e.peut_justifier_absence && !e.peut_voir_vs);
    assert_eq!(acces(e), Acces::Aucun, "l'écran explique, ne propose rien");
    assert!(!peut_justifier(e, &a_draco));
    assert!(matches!(
        narcissa.absences_eleve(draco).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        narcissa
            .justifier_absence(a_draco.id, "Maladie".into())
            .await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        narcissa.retirer_justificatif(a_draco.id).await,
        Err(ApiError::Interdit(_))
    ));
    let resume = narcissa.resume_enfant(draco).await.expect("résumé");
    assert!(
        resume.absences_a_justifier.is_empty(),
        "rien n'est exposé sans drapeau"
    );

    // Parent d'un AUTRE foyer : ni lecture ni justification ni retrait.
    assert!(matches!(
        lily.absences_eleve(draco).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        lily.justifier_absence(a_draco.id, "Maladie".into()).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        narcissa
            .justifier_absence(a_harry.id, "Maladie".into())
            .await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        lily.resume_enfant(draco).await,
        Err(ApiError::Interdit(_)) | Err(ApiError::Introuvable(_))
    ));
    // Une famille ne décide jamais, ne lit pas la liste de l'établissement ni les signaux.
    for decision in [
        DecisionApi::Accepter,
        DecisionApi::Refuser,
        DecisionApi::AnnulerPresent,
    ] {
        assert!(
            matches!(
                lily.decider_absence(a_harry.id, decision, None).await,
                Err(ApiError::Interdit(_))
            ),
            "{decision:?}"
        );
    }
    assert!(matches!(
        lily.absences_a_traiter(jour).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        lily.signaux_absence().await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        lily.fiche_eleve(draco).await,
        Err(ApiError::Interdit(_)) | Err(ApiError::Introuvable(_))
    ));

    // Enseignant : lit, corrige une erreur d'appel, mais ne tranche pas les justificatifs.
    let prof = personnel(&base, &l, "prof").await;
    assert!(prof.absences_a_traiter(jour).await.is_ok());
    for decision in [DecisionApi::Accepter, DecisionApi::Refuser] {
        assert!(
            matches!(
                prof.decider_absence(a_harry.id, decision, None).await,
                Err(ApiError::Interdit(_))
            ),
            "{decision:?}"
        );
    }
    assert!(matches!(
        prof.prendre_signal(Uuid::new_v4()).await,
        Err(ApiError::Interdit(_))
    ));
    let droits_prof = droits_du_role("prof");
    assert_eq!(
        decisions_possibles(StatutAbsence::NonJustifiee, droits_prof),
        [Decision::AnnulerPresent],
        "l'écran ne propose que ce que le serveur accepte"
    );

    // Compte élève : ses propres absences en lecture, pas celles d'un autre, pas d'écriture.
    let eleve_persona = l
        .personas
        .iter()
        .find(|p| p.role == "eleve")
        .expect("persona élève");
    let eleve = client_de(&base, &l, &eleve_persona.email).await;
    let moi = eleve.moi().await.expect("moi");
    assert_eq!(moi.eleve_id, Some(harry), "compte lié à la fiche de Harry");
    assert!(droits_du_role(&moi.role).mes_absences);
    let siennes = eleve.absences_eleve(harry).await.expect("ses absences");
    assert!(siennes.iter().any(|a| a.id == a_harry.id));
    assert!(eleve.retards_eleve(harry).await.is_ok());
    assert!(matches!(
        eleve.absences_eleve(draco).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        eleve.retards_eleve(draco).await,
        Err(ApiError::Interdit(_)) | Err(ApiError::Introuvable(_))
    ));
    // Pas la justification d'une absence d'un autre élève.
    assert!(matches!(
        eleve.justifier_absence(a_draco.id, "Maladie".into()).await,
        Err(ApiError::Interdit(_))
    ));
    // ÉCART DOCUMENTÉ : `actor_can_declare_absence` accepte le compte élève pour SA propre
    // fiche (déclarer / justifier lui-même). L'écran « Absences » reste en lecture seule
    // pour l'élève (cahier des charges) ; à trancher côté produit/backend.
    let r = eleve
        .justifier_absence(a_harry.id, "Rendez-vous médical".into())
        .await;
    println!(
        "justification par l'élève de sa propre absence : {:?}",
        r.as_ref().map(|a| &a.statut)
    );
    assert!(matches!(
        eleve.absences_a_traiter(jour).await,
        Err(ApiError::Interdit(_))
    ));

    // Sans session : tout est refusé.
    let anonyme = ApiClient::new(&base, None);
    assert_eq!(
        anonyme.absences_a_traiter(jour).await.unwrap_err(),
        ApiError::NonConnecte
    );
    assert_eq!(
        anonyme.enfants_famille().await.unwrap_err(),
        ApiError::NonConnecte
    );

    // Nettoyage fonctionnel : le CPE annule les deux absences de test.
    for a in [&a_draco, &a_harry] {
        cpe.decider_absence(a.id, DecisionApi::AnnulerPresent, None)
            .await
            .expect("annulation");
    }
}

/// Signaux d'absence répétée : le seuil (10) déclenche un signal ; le CPE le prend en
/// charge puis le clôt avec une note obligatoire.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signal_d_absence_repetee_prendre_et_clore() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let seamus = id_eleve(&l, "Seamus");
    let cpe = personnel(&base, &l, "cpe").await;

    // Dix jours consécutifs d'une même année scolaire (octobre d'une année future).
    let debut = NaiveDate::from_ymd_opt(jour_neuf().year(), 10, 1).unwrap();
    let motif = cpe.motifs_absence().await.expect("motifs")[0].code.clone();
    let mut creees = Vec::new();
    for i in 0..10 {
        let j = debut + Duration::days(i);
        let v = cpe
            .post_json(
                &format!("/eleves/{seamus}/absences"),
                serde_json::json!({ "debut": j, "fin": j, "motif": motif }),
            )
            .await
            .expect("déclaration");
        creees.push(serde_json::from_value::<Absence>(v).unwrap().id);
    }

    let signaux = cpe.signaux_absence().await.expect("signaux");
    let s = signaux
        .iter()
        .find(|s| s.eleve_id == seamus && s.statut != "clos")
        .unwrap_or_else(|| panic!("signal pour Seamus parmi {signaux:?}"));
    assert!(s.compteur >= s.seuil);
    assert_eq!(s.eleve_label.as_deref(), Some("Seamus Finnigan"));
    let id = s.id;

    // Les décisions sur les signaux sont réservées à ceux qui tranchent.
    let prof = personnel(&base, &l, "prof").await;
    assert!(matches!(
        prof.prendre_signal(id).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        prof.clore_signal(id, "note".into()).await,
        Err(ApiError::Interdit(_))
    ));

    let pris = if s.statut == "ouvert" {
        cpe.prendre_signal(id).await.expect("prise en charge")
    } else {
        s.clone()
    };
    assert_eq!(pris.statut, "en_cours");
    // Clôture : la note est obligatoire (le serveur refuse une note vide).
    assert!(cpe.clore_signal(id, "   ".into()).await.is_err());
    let clos = cpe
        .clore_signal(id, "Famille convoquée".into())
        .await
        .expect("clôture");
    assert_eq!(clos.statut, "clos");
    assert_eq!(clos.note_vs.as_deref(), Some("Famille convoquée"));
    // Un signal clos ne se reclôt pas.
    assert!(cpe.clore_signal(id, "encore".into()).await.is_err());
    let apres = cpe.signaux_absence().await.unwrap();
    assert!(
        apres.iter().all(|x| x.id != id),
        "un signal clos quitte la liste"
    );

    // Nettoyage fonctionnel.
    for a in creees {
        let _ = cpe
            .decider_absence(a, DecisionApi::AnnulerPresent, None)
            .await;
    }
}
