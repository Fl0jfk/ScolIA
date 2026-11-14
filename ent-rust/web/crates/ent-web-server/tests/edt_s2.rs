//! EDT de la semaine, absences et remplacements, changements vus par les familles, délégations
//! (socle 2, S2-5) contre le **vrai** backend (`ent_api::app`) et une base PostgreSQL **locale** de
//! test : rendu serveur de chaque page, décodage des types partagés, parcours principal
//! (absence → remplaçants proposés → remplacement → changements côté famille, annulation, annulation
//! en masse avec aperçu, rétablissement), délégations datées et rôles refusés.
//!
//! Lancement (voir `contre_backend.rs`) :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test edt_s2 -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use ent_types::edt_s2::CreneauSaisieDto;
use ent_web_api::{conflits_de, ApiClient, ApiError, CibleSemaine, DeclarationAbsenceRh};
use ent_web_app::modules::changements_edt::modele as changements;
use ent_web_app::modules::delegations::modele::{composer, SaisieDelegation};
use ent_web_app::modules::edt_semaine::modele::{regrouper_par_jour, StatutSeance};
use ent_web_app::modules::referentiel_admin::modele::annee_courante;
use ent_web_app::modules::remplacements::modele::{
    cmd_remplacement, composer_masse, peut_traiter, resume_traitement, trier_candidats,
    SaisieMasse, PORTEE_CLASSES,
};
use ent_web_app::noyau::droits::droits_du_role;
use ent_web_app::noyau::droits_moteur::DroitsMoteur;
use ent_web_app::noyau::navigation::{self, entrees_avec_moteur};
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

/// Jour de test neuf : un mardi de l'année scolaire courante, au moins une semaine plus tard
/// (les séances n'existent que dans une année scolaire). Les écritures de test sont levées en fin
/// de parcours et chaque exécution tire un jour différent.
fn jour_neuf(debut: NaiveDate, fin: NaiveDate) -> NaiveDate {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;
    let depart = (ent_db::jour_paris() + Duration::days(7)).max(debut);
    let etendue = (fin - depart).num_days().max(7);
    let candidat = depart + Duration::days(n.rem_euclid(etendue));
    let decalage = (2 - i64::from(candidat.weekday().num_days_from_monday())).rem_euclid(7);
    let mardi = candidat + Duration::days(decalage);
    if mardi > fin {
        mardi - Duration::days(7)
    } else {
        mardi
    }
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

fn interdit<T: std::fmt::Debug>(r: Result<T, ApiError>, quoi: &str) {
    assert!(
        matches!(r, Err(ApiError::Interdit(_))),
        "{quoi} : 403 attendu, obtenu {r:?}"
    );
}

fn h(heure: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(heure, 0, 0).unwrap()
}

/// Navigation d'une personne : profil de base + droits effectifs du moteur (comme la coquille).
async fn adresses_de(api: &ApiClient, role: &str) -> Vec<String> {
    let moteur = DroitsMoteur::new(api.mes_droits().await.expect("mes droits"));
    entrees_avec_moteur(droits_du_role(role), &moteur)
        .into_iter()
        .map(|e| e.href)
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pages_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    for (chemin, titre) in [
        ("/web/edt/semaine", "Emploi du temps de la semaine"),
        ("/web/edt/remplacements", "Absences et remplacements"),
        ("/web/changements-edt", "Changements d"),
        ("/web/delegations", "Délégations de droits"),
    ] {
        let r = reqwest::get(format!("{base}{chemin}")).await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(
            r.headers().contains_key("content-security-policy"),
            "CSP {chemin}"
        );
        let html = r.text().await.unwrap();
        assert!(html.contains(titre), "{chemin} : titre « {titre} » absent");
        assert!(html.contains("lang=\"fr\""), "{chemin}");
        assert!(
            html.contains("Chargement"),
            "{chemin} : état de chargement rendu côté serveur"
        );
        assert!(
            !html.contains(" style=\""),
            "{chemin} : style en ligne (CSP)"
        );
        assert!(
            html.contains("id=\"contenu\""),
            "{chemin} : zone principale"
        );
        // La date du jour n'est jamais dans le HTML du serveur (hydratation identique).
        // Ni le jour de Paris ni le jour UTC (le serveur de rendu est en UTC).
        for aujourdhui in [ent_db::jour_paris().to_string(), chrono::Utc::now().format("%Y-%m-%d").to_string()] {
            assert!(
                !html.contains(&aujourdhui),
                "{chemin} : date du jour rendue"
            );
        }
    }
}

/// Parcours principal : le titulaire est absent, l'absence est validée, la direction voit les
/// séances touchées, choisit un remplaçant, puis la famille voit le changement ; annulation,
/// annulation en masse (aperçu puis application idempotente) et rétablissement.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn absence_remplacement_annulations_et_famille() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let admin = personnel(&base, &l, "admin").await;
    let _ = admin.amorcer_referentiel().await; // classes du labo (idempotent)

    let harry = l.eleve("Harry").expect("Harry de labo");
    let classe = harry.classe.clone().expect("classe de Harry");
    let profs: Vec<_> = l.personnel.iter().filter(|p| p.role == "prof").collect();
    assert!(profs.len() >= 2, "deux enseignants de labo au moins");
    let (titulaire, suppleant) = (profs[0], profs[1]);
    let titulaire_api = client_de(&base, &l, &titulaire.email).await;
    let annees = admin.annees_statut().await.expect("années");
    let courante = annee_courante(&annees).expect("année courante").clone();
    let jour = jour_neuf(courante.date_debut, courante.date_fin);

    // Créneau de test à 06:00 (aucun cours du labo à cette heure), réutilisé s'il existe déjà.
    let saisie = CreneauSaisieDto {
        classe: classe.clone(),
        groupe_id: None,
        matiere_libelle: "Test EDT S2-5".into(),
        prof_user_id: titulaire.user_id,
        jour_semaine: jour.weekday().number_from_monday() as i16,
        heure_debut: h(6),
        heure_fin: h(7),
        salle: None,
        site: String::new(),
        semaine_ab: "toutes".into(),
    };
    let existants = admin
        .creneaux_admin(Some(&classe), Some(titulaire.user_id), None)
        .await
        .expect("créneaux");
    let creneau = match existants
        .into_iter()
        .find(|c| c.matiere_libelle == saisie.matiere_libelle && c.heure_debut == h(6))
    {
        Some(c) => c,
        None => admin
            .creer_creneau(&saisie, false)
            .await
            .expect("créneau créé")
            .creneau
            .expect("créneau renvoyé"),
    };
    let classe_id = creneau.classe_id.expect("classe rattachée au référentiel");

    // Conflit : même professeur, même heure → 409 avec la liste des conflits (item « administration »).
    let doublon = CreneauSaisieDto {
        matiere_libelle: "Doublon".into(),
        ..saisie.clone()
    };
    let refus = admin
        .creer_creneau(&doublon, true)
        .await
        .expect_err("conflit attendu");
    let conflits = conflits_de(&refus).expect("corps 409 décodé");
    assert!(
        conflits.conflits.iter().any(|c| c.nature == "professeur"),
        "{conflits:?}"
    );

    // Semaine de la classe : la séance est prévue, regroupée sous le bon jour.
    let sem = admin
        .edt_semaine(&CibleSemaine::Classe(classe_id), jour)
        .await
        .expect("semaine de la classe");
    let jours = regrouper_par_jour(&sem, true);
    let du_jour = jours.iter().find(|j| j.jour == jour).expect("le jour");
    let ligne = du_jour
        .seances
        .iter()
        .find(|s| s.matiere == "Test EDT S2-5")
        .expect("séance de test");
    assert_eq!(ligne.statut, StatutSeance::Prevue);
    assert_eq!(ligne.horaire, "06:00 – 07:00");
    // Personnel : le titulaire voit la sienne ; un salle inconnue renvoie une semaine vide.
    let mienne = titulaire_api
        .edt_semaine(&CibleSemaine::Moi, jour)
        .await
        .expect("ma semaine");
    assert!(mienne
        .seances
        .iter()
        .any(|s| s.matiere_libelle == "Test EDT S2-5"));
    let salle = admin
        .edt_semaine(&CibleSemaine::Salle("SALLE-INCONNUE".into()), jour)
        .await
        .expect("semaine d'une salle");
    assert!(salle.seances.is_empty());

    // Absence du titulaire : signalée par lui-même, pas de traitement avant validation.
    let absence = titulaire_api
        .declarer_absence_rh(&DeclarationAbsenceRh {
            user_id: None,
            debut: jour,
            fin: jour,
            motif: "Test S2-5".into(),
        })
        .await
        .expect("absence déclarée");
    interdit(
        titulaire_api.absences_edt(jour, jour).await,
        "un enseignant ne traite pas les absences",
    );
    let avant = admin.absences_edt(jour, jour).await.expect("absences");
    let signalee = avant
        .iter()
        .find(|a| a.absence.id == absence.id)
        .expect("absence signalée listée");
    assert!(signalee.seances.is_empty());
    assert!(resume_traitement(signalee).contains("validée"));
    admin
        .valider_absence_rh(absence.id)
        .await
        .expect("absence validée");
    let apres = admin.absences_edt(jour, jour).await.expect("absences");
    let validee = apres
        .iter()
        .find(|a| a.absence.id == absence.id)
        .expect("absence validée listée");
    assert!(validee.a_traiter >= 1, "{validee:?}");
    let touchee = validee
        .seances
        .iter()
        .find(|s| s.creneau_id == creneau.id)
        .expect("séance touchée")
        .clone();
    assert!(peut_traiter(&touchee));

    // Remplaçants proposés : le suppléant (autre matière) n'apparaît qu'avec « tous ».
    let tous = admin
        .remplacants(creneau.id, jour, true)
        .await
        .expect("remplaçants");
    let tous = trier_candidats(tous);
    let choisi = tous
        .iter()
        .find(|c| c.user_id == suppleant.user_id)
        .expect("le suppléant est disponible");
    assert!(
        !tous.iter().any(|c| c.user_id == titulaire.user_id),
        "l'absent n'est jamais proposé"
    );
    let cmd =
        cmd_remplacement(absence.id, &touchee, &choisi.user_id.to_string()).expect("commande");
    let r1 = admin
        .organiser_remplacement(&cmd)
        .await
        .expect("remplacement");
    assert!(!r1.deja_fait);
    let r2 = admin.organiser_remplacement(&cmd).await.expect("répété");
    assert!(r2.deja_fait, "idempotent");

    // La semaine de la classe le dit en toutes lettres ; la famille le voit sans motif libre.
    let sem = admin
        .edt_semaine(&CibleSemaine::Classe(classe_id), jour)
        .await
        .unwrap();
    let jours = regrouper_par_jour(&sem, true);
    let ligne = jours
        .iter()
        .flat_map(|j| j.seances.iter())
        .find(|s| s.matiere == "Test EDT S2-5")
        .expect("séance remplacée");
    assert_eq!(ligne.statut, StatutSeance::Remplacee);
    assert!(
        ligne
            .mention
            .as_deref()
            .is_some_and(|m| m.starts_with("Remplacé par")),
        "{ligne:?}"
    );
    let parent = parent_de(&base, &l, "Harry").await;
    let vus = parent
        .changements_edt(harry.id, jour, jour)
        .await
        .expect("changements de l'enfant");
    let groupes = changements::grouper(&vus);
    let carte = groupes
        .iter()
        .flat_map(|g| g.changements.iter())
        .find(|c| c.cours.starts_with("Test EDT S2-5"))
        .expect("changement visible par la famille");
    assert_eq!(carte.nature, changements::Nature::Remplacement);
    assert!(carte.motif.is_none(), "jamais de motif libre");
    assert!(
        parent
            .edt_semaine(&CibleSemaine::Eleve(harry.id), jour)
            .await
            .is_ok(),
        "la famille lit la semaine de son enfant"
    );

    // Rétablir, puis annuler le cours.
    admin
        .lever_exception(creneau.id, jour)
        .await
        .expect("rétabli");
    assert!(matches!(
        admin.lever_exception(creneau.id, jour).await,
        Err(ApiError::Introuvable(_))
    ));
    let annule = admin
        .annuler_seance(creneau.id, jour, Some(absence.id))
        .await
        .expect("annulation");
    assert!(!annule.deja_fait);
    let vus = parent.changements_edt(harry.id, jour, jour).await.unwrap();
    assert!(vus
        .iter()
        .any(|c| c.nature == "annulation" && c.matiere_libelle == "Test EDT S2-5"));
    admin.lever_exception(creneau.id, jour).await.unwrap();

    // Annulation en masse de la classe ce jour-là : aperçu obligatoire, puis application idempotente.
    let masse = SaisieMasse {
        du: jour.to_string(),
        au: jour.to_string(),
        portee: PORTEE_CLASSES.into(),
        classes: vec![classe_id],
        heure_debut: String::new(),
        heure_fin: String::new(),
        motif: "Test fermeture exceptionnelle".into(),
    };
    let apercu = admin
        .annulation_masse(&composer_masse(&masse, true).unwrap())
        .await
        .expect("aperçu");
    assert!(apercu.apercu && apercu.nb_seances >= 1, "{apercu:?}");
    assert!(apercu.classes.iter().any(|c| c.classe_id == classe_id));
    let cmd = composer_masse(&masse, false).unwrap();
    let appliquee = admin.annulation_masse(&cmd).await.expect("application");
    assert!(
        !appliquee.apercu && !appliquee.deja_applique,
        "{appliquee:?}"
    );
    let encore = admin.annulation_masse(&cmd).await.expect("même clé");
    assert!(
        encore.deja_applique,
        "la clé d'idempotence évite un second lot"
    );
    let vus = parent.changements_edt(harry.id, jour, jour).await.unwrap();
    let annulation = vus
        .iter()
        .find(|c| c.nature == "annulation" && c.matiere_libelle == "Test EDT S2-5")
        .expect("annulation du lot vue par la famille");
    assert_eq!(
        annulation.motif_public.as_deref(),
        Some("Test fermeture exceptionnelle")
    );
    admin.lever_exception(creneau.id, jour).await.unwrap();

    // Rôles refusés : la vie scolaire n'annule pas en masse, la famille ne gère rien.
    let cpe = personnel(&base, &l, "cpe").await;
    interdit(
        cpe.annulation_masse(&composer_masse(&masse, true).unwrap())
            .await,
        "annulation en masse par la vie scolaire",
    );
    interdit(parent.absences_edt(jour, jour).await, "absences (famille)");
    interdit(
        parent.remplacants(creneau.id, jour, true).await,
        "remplaçants (famille)",
    );
    interdit(
        parent
            .organiser_remplacement(
                &cmd_remplacement(absence.id, &touchee, &suppleant.user_id.to_string()).unwrap(),
            )
            .await,
        "remplacement (famille)",
    );
    interdit(
        parent
            .edt_semaine(&CibleSemaine::Classe(classe_id), jour)
            .await,
        "semaine d'une classe (famille)",
    );
    interdit(
        parent.creer_creneau(&saisie, true).await,
        "créneau (famille)",
    );
    // Un autre parent ne voit pas les changements de Harry.
    let autre = parent_de(&base, &l, "Draco").await;
    interdit(
        autre.changements_edt(harry.id, jour, jour).await,
        "changements d'un enfant non lié",
    );

    // Navigation selon les droits du moteur.
    let adm = adresses_de(&admin, "admin").await;
    for attendu in [
        navigation::REMPLACEMENTS,
        navigation::EDT_SEMAINE,
        navigation::DELEGATIONS,
    ] {
        assert!(
            adm.contains(&attendu.to_string()),
            "admin : {attendu} manquant"
        );
    }
    let prof = adresses_de(&titulaire_api, "prof").await;
    assert!(prof.contains(&navigation::EDT_SEMAINE.to_string()));
    assert!(!prof.contains(&navigation::REMPLACEMENTS.to_string()));
    let fam = adresses_de(&parent, "parent").await;
    assert!(fam.contains(&navigation::CHANGEMENTS_EDT.to_string()));
    assert!(!fam.contains(&navigation::REMPLACEMENTS.to_string()));
    assert!(!fam.contains(&navigation::DELEGATIONS.to_string()));
}

/// Délégations : octroi daté par l'administration, effet immédiat sur les droits du délégué,
/// révocation ; refus aux autres rôles ; date de fin passée refusée.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delegations_datees_et_roles_refuses() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let admin = personnel(&base, &l, "admin").await;
    let prof_p = l.personnel.iter().find(|p| p.role == "prof").expect("prof");
    let prof = client_de(&base, &l, &prof_p.email).await;
    let aujourdhui = ent_db::jour_paris();

    // Le personnel lit « ses » délégations ; la gestion lui est refusée.
    assert!(prof
        .mes_delegations()
        .await
        .expect("mes délégations")
        .is_empty());
    interdit(
        prof.delegations(false).await,
        "liste des délégations (enseignant)",
    );

    // Une délégation ancienne laissée par un essai précédent est révoquée d'abord.
    for d in admin.delegations(true).await.expect("liste") {
        if d.delegue_user_id == prof_p.user_id && d.etat == "active" {
            admin.revoquer_delegation(d.id).await.expect("révocation");
        }
    }
    interdit(
        prof.absences_edt(aujourdhui, aujourdhui).await,
        "sans délégation, pas d'accès aux absences",
    );

    let corps = composer(
        &SaisieDelegation {
            delegue: prof_p.user_id.to_string(),
            actions: vec!["edt.gerer".into()],
            debut: String::new(),
            fin: (aujourdhui + Duration::days(7)).to_string(),
            motif: "Test S2-5 : remplacement de la direction".into(),
        },
        aujourdhui,
    )
    .expect("saisie valide");
    let d = admin.accorder_delegation(&corps).await.expect("délégation");
    assert_eq!(d.etat, "active");
    assert_eq!(d.source, "manuelle");
    assert_eq!(d.droits.len(), 1);

    // Effet immédiat : le délégué est vu par « mes délégations » et gère l'emploi du temps.
    let mes = prof.mes_delegations().await.unwrap();
    assert!(mes.iter().any(|m| m.id == d.id && m.etat == "active"));
    assert!(prof.absences_edt(aujourdhui, aujourdhui).await.is_ok());
    assert!(admin
        .delegations(true)
        .await
        .unwrap()
        .iter()
        .any(|x| x.id == d.id));
    assert!(adresses_de(&prof, "prof")
        .await
        .contains(&navigation::REMPLACEMENTS.to_string()));

    // Révocation : le droit disparaît tout de suite.
    let r = admin.revoquer_delegation(d.id).await.expect("révoquée");
    assert_eq!(r.etat, "revoquee");
    interdit(
        prof.absences_edt(aujourdhui, aujourdhui).await,
        "après révocation",
    );
    assert!(!admin
        .delegations(true)
        .await
        .unwrap()
        .iter()
        .any(|x| x.id == d.id));

    // Refus : la date de fin dans le passé, un rôle sans `delegations.gerer`, un non-personnel.
    let mut passe = corps.clone();
    passe.fin = aujourdhui - Duration::days(1);
    assert!(matches!(
        admin.accorder_delegation(&passe).await,
        Err(ApiError::Invalide(_))
    ));
    let cpe = personnel(&base, &l, "cpe").await;
    interdit(
        cpe.accorder_delegation(&corps).await,
        "octroi (vie scolaire)",
    );
    interdit(
        cpe.revoquer_delegation(d.id).await,
        "révocation (vie scolaire)",
    );
    let parent = parent_de(&base, &l, "Harry").await;
    // « Mes délégations » est ouverte à tous les comptes : une famille n'en a simplement aucune.
    assert!(parent
        .mes_delegations()
        .await
        .expect("mes délégations")
        .is_empty());
    interdit(parent.delegations(false).await, "liste (famille)");
    let _: Uuid = d.id;
}
