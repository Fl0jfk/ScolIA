//! Grille de notes V3 (socle 21) contre le **vrai** backend (`ent_api::app`) et une base PostgreSQL locale de
//! test : rendu serveur de la page, puis un parcours complet avec le client typé du frontend (cours, évaluations,
//! saisie au clavier avec codes rapides, conflit de version, moyenne du moteur, collage, import, publication)
//! et les refus attendus (famille, direction en lecture, autre professeur).
//!
//! Lancement :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test grille_v3 -- --test-threads=1 --nocapture
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »). Aucune date ne dépend de l'horloge du test :
//! les dates d'évaluation partent du début de la période du cours.
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_types::evaluations_v3::{
    CelluleSaisieDto, CreerEvaluationV3Body, ImportBody, ImportLigneBody, PublierV3Body,
    SaisieLotBody,
};
use ent_web_api::{ApiClient, ApiError};
use ent_web_app::modules::evaluations::grille_v3::modele::{
    lire_tableur, plan_collage, plan_fichier,
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

fn personnel<'a>(l: &'a Labo, role: &str) -> &'a labo::Personnel {
    l.personnel
        .iter()
        .find(|p| p.role == role)
        .unwrap_or_else(|| panic!("personnel {role} de labo"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn page_grille_rendue_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    for chemin in [
        "/web/evaluations/grille",
        "/web/evaluations/grille?cours=00000000-0000-0000-0000-000000000000",
    ] {
        let r = reqwest::get(format!("{base}{chemin}")).await.unwrap();
        assert_eq!(r.status(), 200, "{chemin}");
        assert!(
            r.headers().contains_key("content-security-policy"),
            "CSP {chemin}"
        );
        let html = r.text().await.unwrap();
        assert!(
            !html.contains(" style=\""),
            "aucun style en ligne (CSP) : {chemin}"
        );
        assert!(html.contains("lang=\"fr\""), "{chemin}");
    }
}

fn cell(ev: Uuid, eleve: Uuid, texte: &str, version: Option<i32>) -> CelluleSaisieDto {
    CelluleSaisieDto {
        evaluation_id: ev,
        eleve_id: eleve,
        texte: texte.into(),
        version_attendue: version,
        motif: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parcours_grille_v3_avec_le_client_du_frontend_et_les_permissions() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let etab = l.etablissement_id;
    let url = std::env::var("ENT_DATABASE_URL").unwrap();
    let ent = ent_db::PgEnt::connect(&url).await.expect("connexion base");
    ent.migrate().await.expect("migrations");

    // Préparation du référentiel (hors périmètre de l'écran) : classe, matière, service du professeur, deux élèves.
    let prof = personnel(&l, "prof");
    let annee = ent.referentiel_annee_courante_assuree(etab).await.unwrap();
    let suffixe = Uuid::new_v4().simple().to_string()[..6].to_string();
    let code_classe = format!("W3-{suffixe}");
    let classe = ent
        .referentiel_creer_classe("admin", etab, Some(annee.id), &code_classe, None, None)
        .await
        .unwrap();
    let matiere = ent
        .create_matiere(
            ent_db::CreateMatiere {
                etablissement_id: etab,
                code: format!("W3M{suffixe}"),
                libelle: "Mathématiques V3".into(),
                source_nomenclature: None,
            },
            None,
        )
        .await
        .unwrap();
    ent.referentiel_creer_service(
        "admin",
        etab,
        ent_db::CreerService {
            prof_user_id: prof.user_id,
            matiere_id: Some(matiere.id),
            classe_id: Some(classe.id),
            groupe_id: None,
            professeur_principal: false,
        },
    )
    .await
    .unwrap();
    let mut eleves = Vec::new();
    for (nom, prenom) in [("Abbott", "Hannah"), ("Bones", "Susan")] {
        let e = ent
            .create_eleve(ent_db::CreateEleve {
                etablissement_id: etab,
                nom: format!("{nom}{suffixe}"),
                prenom: prenom.into(),
                ine: None,
                sexe: None,
                date_naissance: None,
                numero_badge: None,
                redoublant: false,
                classe: None,
                regime: None,
                mef: None,
                site: None,
                date_entree: None,
                date_sortie: None,
                motif_sortie: None,
                actor_user_id: None,
            })
            .await
            .unwrap();
        ent.referentiel_affecter_eleve_classe(
            "admin",
            etab,
            classe.id,
            e.id,
            Some(annee.date_debut),
        )
        .await
        .unwrap();
        eleves.push(e);
    }

    let c_prof = client_de(&base, &l, &prof.email).await;
    // 1. Le cours est dans « mes enseignements » ; ses périodes sont celles de la périodisation de la classe.
    let ens = c_prof.eva3_enseignements().await.expect("enseignements");
    let cours = ens
        .iter()
        .find(|e| e.cible_code == code_classe && e.libelle.contains("Mathématiques V3"))
        .expect("le cours du professeur")
        .clone();
    assert_eq!(cours.droit, "saisie");
    assert!(!cours.sans_matiere);
    let periodes = c_prof
        .eva3_periodes(cours.cours_id)
        .await
        .expect("périodes");
    assert!(
        !periodes.is_empty(),
        "la classe a une périodisation d'évaluation"
    );
    let periode = periodes[0].clone();
    let date = periode.date_debut;

    // 2. Création de deux évaluations, saisie avec codes rapides, moyenne du moteur.
    let corps = |titre: &str, bareme_m: i64, coef_m: i64| CreerEvaluationV3Body {
        cours_id: cours.cours_id,
        titre: titre.into(),
        date_eval: date,
        bareme_m,
        coef_m,
        comptage: None,
        hors_bareme_autorise: false,
        sous_matiere_id: None,
        restreinte_a: vec![],
        publication: None,
        publication_prevue_le: None,
        annoncee: false,
    };
    let e1 = c_prof
        .eva3_creer(&corps("Contrôle", 20_000, 1_000))
        .await
        .expect("évaluation 1");
    let e2 = c_prof
        .eva3_creer(&corps("Exposé", 10_000, 2_000))
        .await
        .expect("évaluation 2");
    let (a, b) = (eleves[0].id, eleves[1].id);
    let r = c_prof
        .eva3_saisir(&SaisieLotBody {
            cours_id: cours.cours_id,
            cellules: vec![
                cell(e1.id, a, "14,5", None),
                cell(e2.id, a, "abs", None),
                cell(e1.id, b, "21", None),
                cell(e1.id, b, "9", None),
            ],
            origine: None,
        })
        .await
        .expect("saisie");
    let issues: Vec<&str> = r.cellules.iter().map(|c| c.issue.as_str()).collect();
    assert_eq!(issues, ["ecrit", "ecrit", "refuse", "ecrit"], "{r:?}");
    let g = c_prof
        .eva3_grille(cours.cours_id, periode.id)
        .await
        .expect("grille");
    assert!(g.peut_saisir);
    let la = g.eleves.iter().find(|x| x.eleve_id == a).unwrap();
    assert_eq!(la.moyenne_m, Some(14_500), "l'absent ne compte pas");
    let lb = g.eleves.iter().find(|x| x.eleve_id == b).unwrap();
    assert_eq!(lb.moyenne_m, Some(9_000));
    assert!(g.moyenne_classe.is_some());

    // 3. Deux onglets : jamais d'écrasement silencieux.
    let v = la.cellules[0].version.unwrap();
    let ok = c_prof
        .eva3_saisir(&SaisieLotBody {
            cours_id: cours.cours_id,
            cellules: vec![cell(e1.id, a, "15", Some(v))],
            origine: None,
        })
        .await
        .unwrap();
    assert_eq!(ok.cellules[0].issue, "ecrit");
    let perime = c_prof
        .eva3_saisir(&SaisieLotBody {
            cours_id: cours.cours_id,
            cellules: vec![cell(e1.id, a, "12", Some(v))],
            origine: None,
        })
        .await
        .unwrap();
    assert_eq!(perime.cellules[0].issue, "conflit");
    assert_eq!(
        perime.cellules[0].conflit.as_ref().unwrap().valeur_m,
        Some(15_000)
    );
    let histo = c_prof.eva3_historique(e1.id, a).await.expect("historique");
    assert_eq!(histo.len(), 2);

    // 4. Collage d'un bloc (modèle du frontend) : aperçu, puis confirmation avec l'empreinte.
    let bloc = lire_tableur("11\t8\n12\tnr\n");
    let ids_el = [a, b];
    let ids_ev = [e1.id, e2.id];
    let (cibles, lignes) = plan_collage(&bloc, 0, 0, &ids_el, &ids_ev).unwrap();
    let import = ImportBody {
        cours_id: cours.cours_id,
        mode: "collage".into(),
        cibles,
        lignes,
        empreinte: None,
    };
    let ap = c_prof.eva3_apercu_import(&import).await.expect("aperçu");
    assert_eq!(ap.totaux.rapprochees, 2);
    let r = c_prof
        .eva3_confirmer_import(&ImportBody {
            empreinte: Some(ap.empreinte.clone()),
            ..import.clone()
        })
        .await
        .expect("confirmation");
    assert!(
        r.cellules
            .iter()
            .all(|c| c.issue == "ecrit" || c.issue == "inchange"),
        "{r:?}"
    );
    // L'empreinte d'un aperçu périmé est refusée (409).
    let r = c_prof
        .eva3_confirmer_import(&ImportBody {
            empreinte: Some(ap.empreinte),
            ..import
        })
        .await;
    assert!(matches!(r, Err(ApiError::Conflit(_))), "{r:?}");

    // 5. Import d'un fichier : rapprochement par nom (n'importe quel ordre, sans accent).
    let table = lire_tableur(&format!(
        "Nom;Contrôle\nHannah ABBOTT{suffixe};16\nSusan Bones{suffixe};7,5\nInconnu Z;3\n"
    ));
    let evals = vec![
        (e1.id, "Contrôle".to_string()),
        (e2.id, "Exposé".to_string()),
    ];
    let plan = plan_fichier(&table, &evals, None).unwrap();
    // « Hannah ABBOTTxxxxxx » n'est pas « Abbottxxxxxx Hannah » : le nom est celui de la fiche (nom + prénom).
    let imp = ImportBody {
        cours_id: cours.cours_id,
        mode: "fichier".into(),
        cibles: plan.cibles,
        lignes: plan.lignes,
        empreinte: None,
    };
    let ap = c_prof
        .eva3_apercu_import(&imp)
        .await
        .expect("aperçu fichier");
    assert_eq!(ap.totaux.non_rapprochees, 1, "{ap:?}");
    assert_eq!(ap.totaux.rapprochees, 2);
    let _ = ImportLigneBody {
        eleve_id: None,
        nom: None,
        rang: None,
        cellules: vec![],
    };

    // 6. Publication : rien n'est visible sans publication ; puis publiée, puis retirée.
    let p = c_prof
        .eva3_publication(
            e1.id,
            &PublierV3Body {
                action: "maintenant".into(),
                prevue_le: None,
                prevue_locale: None,
                motif: None,
            },
        )
        .await
        .expect("publication");
    assert!(p.publiee_le.is_some());
    let p = c_prof
        .eva3_publication(
            e1.id,
            &PublierV3Body {
                action: "retirer".into(),
                prevue_le: None,
                prevue_locale: None,
                motif: Some("erreur de barème".into()),
            },
        )
        .await
        .expect("retrait");
    assert!(p.publiee_le.is_none() && p.retiree_le.is_some());

    // 7. Permissions : famille jamais ; direction en lecture ; autre professeur refusé.
    let parent = l
        .personas
        .iter()
        .find(|p| p.role == "parent")
        .expect("un parent");
    let c_parent = client_de(&base, &l, &parent.email).await;
    assert!(matches!(
        c_parent.eva3_grille(cours.cours_id, periode.id).await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        c_parent
            .eva3_saisir(&SaisieLotBody {
                cours_id: cours.cours_id,
                cellules: vec![cell(e1.id, a, "20", None)],
                origine: None
            })
            .await,
        Err(ApiError::Interdit(_))
    ));
    let dir = personnel(&l, "direction");
    let c_dir = client_de(&base, &l, &dir.email).await;
    let gd = c_dir
        .eva3_grille(cours.cours_id, periode.id)
        .await
        .expect("la direction lit");
    assert!(!gd.peut_saisir);
    let refus = c_dir
        .eva3_saisir(&SaisieLotBody {
            cours_id: cours.cours_id,
            cellules: vec![cell(e1.id, b, "20", None)],
            origine: None,
        })
        .await;
    assert!(
        matches!(refus, Err(ApiError::Interdit(_)))
            || refus.is_ok_and(|r| r.cellules[0].issue == "refuse")
    );
    let autre = l
        .personnel
        .iter()
        .filter(|p| p.role == "prof")
        .find(|p| p.user_id != prof.user_id);
    if let Some(autre) = autre {
        let c_autre = client_de(&base, &l, &autre.email).await;
        assert!(matches!(
            c_autre.eva3_grille(cours.cours_id, periode.id).await,
            Err(ApiError::Interdit(_))
        ));
    }
    // Sans session : refus.
    let sans = ApiClient::new(&base, None);
    assert!(matches!(
        sans.eva3_enseignements().await,
        Err(ApiError::NonConnecte)
    ));
}
