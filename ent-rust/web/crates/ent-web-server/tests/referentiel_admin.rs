//! Administration du référentiel (S2-2) et discipline / signalements (S2-4) contre le **vrai**
//! backend (`ent_api::app`) et une base PostgreSQL **locale** de test : rendu serveur des écrans,
//! décodage des DTO partagés, parcours d'écriture (idempotents, sans toucher à l'année courante),
//! navigation selon les droits du moteur et permissions (au moins un rôle refusé par écran).
//!
//! Lancement (voir `contre_backend.rs`) :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test referentiel_admin -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use chrono::{Datelike, Duration, NaiveDate};
use ent_web_api::{ApiClient, ApiError};
use ent_web_app::modules::discipline_vs::modele::{
    actions_procedure, composer_decision, composer_procedure, composer_punition, SaisieProcedure,
    SaisiePunition,
};
use ent_web_app::modules::referentiel_admin::modele::{
    annee_courante, composer_grille, composer_periodes, composer_plage, composer_site,
    lignes_depuis_grille, lignes_depuis_periodes, periodes_modele, regrouper_calendrier,
    SaisiePlage, KINDS_JOUR,
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

fn suffixe() -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros();
    format!("{}", n % 100_000_000)
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
        ("/web/referentiel", "Référentiel scolaire"),
        ("/web/referentiel/annees", "Années et bascule"),
        ("/web/referentiel/structure", "Sites, niveaux et MEF"),
        ("/web/referentiel/classes", "Classes et groupes"),
        ("/web/referentiel/calendrier", "Périodes et calendrier"),
        ("/web/referentiel/grille", "Grille horaire et séances"),
        ("/web/discipline", "Punitions et sanctions"),
        ("/web/signalements", "Signalements d"),
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
            !html.contains(" style=\""),
            "{chemin} : style en ligne (CSP)"
        );
        assert!(
            html.contains("id=\"contenu\""),
            "{chemin} : zone principale"
        );
    }
    // Les sections du référentiel se repèrent au clavier et aux lecteurs d'écran.
    let html = reqwest::get(format!("{base}/web/referentiel/calendrier"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(html.contains("aria-label=\"Sections du référentiel\""));
    assert!(html.contains("aria-current=\"page\""));
}

/// Le personnel de gestion administre le référentiel ; les types partagés décodent les réponses du
/// vrai serveur ; la saisie composée par le modèle de vue est acceptée.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn administration_du_referentiel() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let admin = personnel(&base, &l, "admin").await;
    let _ = admin.amorcer_referentiel().await; // année courante et classes du labo (idempotent)

    // Années : une année courante, statuts décodés.
    let annees = admin.annees_statut().await.expect("années");
    let courante = annee_courante(&annees).expect("une année courante").clone();
    assert!(courante.courante);

    // Sites : création validée par le modèle, doublon refusé, désactivation puis réactivation.
    let code = format!("S{}", suffixe());
    let (c, lib) = composer_site(&code, "Site de test").expect("saisie valide");
    let site = admin
        .creer_site(&c, &lib, Some("1 rue de test"), None)
        .await
        .expect("site");
    assert!(admin
        .sites()
        .await
        .unwrap()
        .iter()
        .any(|s| s.id == site.id && s.actif));
    assert!(
        admin.creer_site(&c, &lib, None, None).await.is_err(),
        "code déjà pris"
    );
    let s = admin
        .maj_site(site.id, None, None, Some(false))
        .await
        .unwrap();
    assert!(!s.actif);
    admin
        .maj_site(site.id, None, None, Some(true))
        .await
        .unwrap();

    // Niveaux : nomenclature nationale (idempotente), MEF importés et rejets signalés.
    admin.amorcer_niveaux().await.expect("niveaux");
    let second = admin.amorcer_niveaux().await.expect("niveaux (2e fois)");
    assert_eq!(second.niveaux_crees, 0, "idempotent");
    let niveaux = admin.niveaux().await.unwrap();
    assert!(niveaux.len() >= 7 && niveaux.iter().any(|n| n.code == "6EME"));
    let mef = format!(
        "9{:010}",
        suffixe().parse::<u64>().unwrap() % 10_000_000_000
    );
    let (lignes, illisibles) = ent_web_app::modules::referentiel_admin::modele::lire_import_mef(
        &format!("{mef};MEF de test;6EME;\n;sans code\n"),
    );
    assert_eq!((lignes.len(), illisibles.len()), (1, 1));
    let r = admin
        .importer_mefs("import_siecle", &lignes)
        .await
        .expect("import");
    assert!(r.crees + r.mis_a_jour >= 1, "{r:?}");
    assert!(admin
        .mefs()
        .await
        .unwrap()
        .iter()
        .any(|m| m.code_mef == mef));

    // Périodes : sur une année de test lointaine (l'année courante n'est jamais modifiée).
    let libelle = "2098-2099";
    let test = match annees.iter().find(|a| a.libelle == libelle) {
        Some(a) => a.clone(),
        None => {
            admin
                .creer_annee(
                    libelle,
                    NaiveDate::from_ymd_opt(2098, 9, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2099, 7, 4).unwrap(),
                )
                .await
                .expect("année de test");
            let a = admin.annees_statut().await.unwrap();
            a.into_iter()
                .find(|a| a.libelle == libelle)
                .expect("année créée")
        }
    };
    let lignes = periodes_modele(&test, "trimestre", 3);
    let corps = composer_periodes(&lignes, &test).expect("couverture valide");
    let rendues = admin
        .definir_periodes(test.id, &corps)
        .await
        .expect("périodes enregistrées");
    assert_eq!(rendues.len(), 3);
    assert_eq!(
        lignes_depuis_periodes(&admin.periodes_annee(test.id).await.unwrap()),
        lignes
    );
    // Une couverture incomplète est refusée par le modèle de vue comme par le serveur.
    let mut trou = lignes.clone();
    trou.remove(1);
    assert!(composer_periodes(&trou, &test).is_err());
    let corps_trou: Vec<_> = corps
        .iter()
        .cloned()
        .enumerate()
        .filter(|(i, _)| *i != 1)
        .map(|(_, p)| p)
        .collect();
    assert!(matches!(
        admin.definir_periodes(test.id, &corps_trou).await,
        Err(ApiError::Invalide(_))
    ));
    // Les périodes existantes de l'année courante se lisent (décodage) sans être modifiées.
    let _ = admin
        .periodes_annee(courante.id)
        .await
        .expect("périodes de l'année courante");

    // Calendrier : un jour fermé hors année, lu, regroupé, retiré.
    let jour = NaiveDate::from_ymd_opt(2099, 1, 1).unwrap();
    let plage = composer_plage(
        &SaisiePlage {
            du: jour.to_string(),
            au: (jour + Duration::days(2)).to_string(),
            kind: "ferme".into(),
            libelle: format!("Fermeture de test {}", suffixe()),
        },
        &KINDS_JOUR,
    )
    .expect("plage valide");
    let n = admin
        .ajouter_jours_fermes(plage.du, plage.au, &plage.kind, &plage.libelle, None)
        .await
        .expect("jours fermés");
    assert_eq!(n, 3);
    let jours = admin
        .calendrier(plage.du, plage.au)
        .await
        .expect("calendrier");
    let plages = regrouper_calendrier(&jours);
    assert!(
        plages
            .iter()
            .any(|p| p.libelle == plage.libelle && p.nb_jours() == 3),
        "{plages:?}"
    );
    for i in 0..3 {
        admin
            .retirer_jour_ferme(jour + Duration::days(i), None)
            .await
            .expect("retrait");
    }
    assert!(admin
        .calendrier(plage.du, plage.au)
        .await
        .unwrap()
        .is_empty());

    // Grille : chargement de la grille type, relue et renvoyée à l'identique.
    let grille = admin.amorcer_grille().await.expect("grille type");
    let lignes = lignes_depuis_grille(&grille);
    assert!(!lignes.is_empty());
    let corps = composer_grille(&lignes).expect("grille valide");
    let rendue = admin
        .definir_grille(None, &corps)
        .await
        .expect("grille enregistrée");
    assert_eq!(rendue.len(), corps.len());

    // Séances : génération sur une semaine lointaine, rapport décodé, puis liste.
    let du = courante.date_debut + Duration::days(7);
    let rapport = admin
        .generer_seances(courante.id, Some(du), Some(du + Duration::days(6)))
        .await
        .expect("génération");
    assert_eq!(rapport.du, Some(du));
    let _ = admin
        .seances_datees(du, du + Duration::days(6), None)
        .await
        .expect("séances");

    // Classes de l'année, groupes et ateliers avec responsables.
    let classes = admin.classes_annee(courante.id, du).await.expect("classes");
    assert!(!classes.is_empty());
    let atelier = format!("ATL{}", suffixe());
    admin
        .creer_groupe(&atelier, Some("Atelier de test"), None, None)
        .await
        .expect("atelier");
    let groupes = admin.groupes_admin(None).await.expect("groupes");
    let g = groupes
        .iter()
        .find(|g| g.code == atelier)
        .expect("atelier listé");
    assert!(g.classe_id.is_none(), "atelier transverse");
    let prof = l.premier("prof").expect("prof");
    let trouves = admin
        .chercher_personnel(&prof.email)
        .await
        .expect("personnel");
    let p = trouves
        .iter()
        .find(|p| p.user_id == prof.user_id)
        .expect("le professeur est trouvé");
    let ids = admin
        .definir_responsables_groupe(g.id, &[p.user_id])
        .await
        .expect("responsables");
    assert_eq!(ids, vec![prof.user_id]);
    let groupes = admin.groupes_admin(None).await.unwrap();
    let g = groupes.iter().find(|g| g.code == atelier).unwrap();
    assert_eq!(g.responsables.len(), 1);
    assert_eq!(g.responsables[0].user_id, prof.user_id);

    // Bascule : l'aperçu de montée vers l'année de test ne modifie rien (aperçu, jamais appliqué ici).
    let c4b = classes.iter().find(|c| c.code == "4B").map(|c| c.id);
    if let Some(src) = c4b {
        let r = admin
            .monter_eleves(
                test.id,
                courante.id,
                &[ent_types::referentiel_admin::CorrespondanceDto {
                    source: src,
                    cible: src,
                }],
                true,
            )
            .await;
        // Classe cible absente de l'année de test : le serveur refuse ; sinon l'aperçu est décodé.
        match r {
            Ok(apercu) => assert!(apercu.apercu),
            Err(e) => assert!(
                matches!(
                    e,
                    ApiError::Invalide(_) | ApiError::Introuvable(_) | ApiError::Conflit(_)
                ),
                "{e:?}"
            ),
        }
    }
}

/// Les droits décident des écrans et des écritures ; un refus du serveur reste traité comme tel.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn droits_du_moteur_navigation_et_refus() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");

    let admin = personnel(&base, &l, "admin").await;
    let v = adresses_de(&admin, "admin").await;
    for attendu in [
        navigation::REFERENTIEL,
        navigation::JOURNEE,
        navigation::DISCIPLINE,
        navigation::SIGNALEMENTS,
    ] {
        assert!(
            v.contains(&attendu.to_string()),
            "admin : {attendu} manquant dans {v:?}"
        );
    }
    // La vie scolaire (CPE) n'administre pas le référentiel mais signale et punit.
    let cpe = personnel(&base, &l, "cpe").await;
    let v = adresses_de(&cpe, "cpe").await;
    assert!(
        !v.contains(&navigation::REFERENTIEL.to_string()),
        "CPE : lecture seule du référentiel, pas d'écran d'administration : {v:?}"
    );
    assert!(
        v.contains(&navigation::SIGNALEMENTS.to_string())
            && v.contains(&navigation::DISCIPLINE.to_string())
    );
    // Un enseignant : discipline (ses élèves) oui, administration non.
    let prof = personnel(&base, &l, "prof").await;
    let v = adresses_de(&prof, "prof").await;
    assert!(v.contains(&navigation::DISCIPLINE.to_string()));
    for absent in [
        navigation::REFERENTIEL,
        navigation::SIGNALEMENTS,
        navigation::JOURNEE,
    ] {
        assert!(
            !v.contains(&absent.to_string()),
            "prof : {absent} ne doit pas apparaître"
        );
    }
    // Une famille ne voit aucun écran d'administration.
    let parent = parent_de(&base, &l, "Harry").await;
    let v = adresses_de(&parent, "parent").await;
    assert_eq!(
        v,
        [
            navigation::ACCUEIL,
            navigation::CHANGEMENTS_EDT,
            navigation::ABSENCES,
            navigation::MESSAGES
        ]
    );

    // Refus côté serveur (l'écran masque les boutons, mais le serveur est l'autorité).
    assert!(
        prof.sites().await.is_ok(),
        "lecture du référentiel pour tout le personnel"
    );
    assert!(matches!(
        prof.creer_site("X", "Y", None, None).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        prof.amorcer_niveaux().await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        prof.amorcer_grille().await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        parent.annees_statut().await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(parent.sites().await, Err(ApiError::Interdit(_))));
    assert!(matches!(
        prof.definir_responsables_groupe(Uuid::new_v4_nul(), &[])
            .await,
        Err(ApiError::Interdit(_))
    ));
}

trait UuidNul {
    fn new_v4_nul() -> Uuid;
}
impl UuidNul for Uuid {
    fn new_v4_nul() -> Uuid {
        Uuid::from_u128(0xdead_beef)
    }
}

/// Punitions (≠ sanctions), procédure disciplinaire et signalements d'assiduité.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discipline_et_signalements() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let harry = l.eleve("Harry").expect("Harry").id;
    let admin = personnel(&base, &l, "admin").await;
    let cpe = personnel(&base, &l, "cpe").await;
    let aujourdhui = chrono::Local::now().date_naive();

    // Types usuels, puis une punition donnée par la vie scolaire, clôturée.
    let types = admin.amorcer_types_punition().await.expect("types");
    let types = if types.is_empty() {
        cpe.types_punition().await.unwrap()
    } else {
        types
    };
    let type_id = types.iter().find(|t| t.actif).expect("un type actif").id;
    let corps = composer_punition(&SaisiePunition {
        type_id: type_id.to_string(),
        jour: aujourdhui.to_string(),
        motif: format!("Bavardages {}", suffixe()),
        echeance: (aujourdhui + Duration::days(7)).to_string(),
        duree: "60".into(),
    })
    .expect("saisie valide");
    let p = cpe.donner_punition(harry, &corps).await.expect("punition");
    assert_eq!(p.statut, "donnee");
    assert!(cpe
        .punitions_eleve(harry)
        .await
        .unwrap()
        .iter()
        .any(|x| x.id == p.id));
    let p = cpe
        .clore_punition(p.id, "effectuee")
        .await
        .expect("clôture");
    assert_eq!(p.statut, "effectuee");
    // Une punition n'est pas une sanction : aucune sanction n'apparaît pour autant.
    let sanctions = cpe.sanctions_eleve(harry).await.expect("sanctions");
    assert!(sanctions.iter().all(|s| s.type_id != type_id));

    // Procédure : chef d'établissement, observations puis décision.
    let b = composer_procedure(
        harry,
        &SaisieProcedure {
            instance: "chef_etablissement".into(),
            faits: "Faits de test".into(),
        },
    )
    .expect("saisie valide");
    let proc = admin.engager_procedure(&b).await.expect("procédure");
    assert_eq!(proc.statut, "engagee");
    let a = actions_procedure(&proc);
    assert!(a.observations && a.classer && !a.decider);
    let proc = admin
        .recueillir_observations(proc.id, aujourdhui)
        .await
        .expect("observations");
    assert!(actions_procedure(&proc).decider);
    let (texte, jour) = composer_decision("Avertissement", &aujourdhui.to_string()).unwrap();
    let proc = admin
        .decider_procedure(proc.id, &texte, jour)
        .await
        .expect("décision");
    assert_eq!(proc.statut, "decidee");
    assert_eq!(actions_procedure(&proc), Default::default());
    assert!(admin
        .procedures_eleve(harry)
        .await
        .unwrap()
        .iter()
        .any(|x| x.id == proc.id));
    // Une procédure peut être classée sans suite.
    let autre = admin.engager_procedure(&b).await.unwrap();
    assert_eq!(
        admin.classer_procedure(autre.id).await.unwrap().statut,
        "close"
    );

    // Signalements : décompte du mois, proposition, liste.
    let mois = NaiveDate::from_ymd_opt(aujourdhui.year(), aujourdhui.month(), 1).unwrap();
    let d = cpe.decompte_mensuel(mois).await.expect("décompte");
    assert_eq!(d.mois, mois);
    let propositions = cpe.proposer_signalements(mois).await.expect("propositions");
    let liste = cpe.signalements(mois).await.expect("signalements");
    assert!(propositions
        .iter()
        .all(|p| liste.iter().any(|s| s.id == p.id)));
    let regle = cpe.regle_assiduite().await.expect("règle");
    assert!(regle.seuil_signalement >= 1);
    let _ = cpe
        .demi_journees_eleve(harry, mois, aujourdhui)
        .await
        .expect("demi-journées");

    // Permissions : la famille ne punit pas ; l'enseignant ne gère ni la discipline ni les signalements.
    let parent = parent_de(&base, &l, "Harry").await;
    assert!(matches!(
        parent.donner_punition(harry, &corps).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        parent.decompte_mensuel(mois).await,
        Err(ApiError::Interdit(_))
    ));
    let prof = personnel(&base, &l, "prof").await;
    assert!(matches!(
        prof.engager_procedure(&b).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        prof.proposer_signalements(mois).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        prof.effacer_sanctions(aujourdhui).await,
        Err(ApiError::Interdit(_))
    ));
}
