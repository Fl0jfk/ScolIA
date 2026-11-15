//! Évaluations (socle 2, S2-7) contre le **vrai** backend (`ent_api::app`) et une base
//! PostgreSQL **locale** de test : rendu serveur des écrans, puis une période complète avec le
//! client typé du frontend (grille → publication → relevé famille → fermeture → bulletin
//! publié → PDF → export LSU) et les refus attendus.
//!
//! Lancement (voir `contre_backend.rs`) :
//!   cd ent-rust/web && cargo test -p ent-web-server --features labo --test evaluations -- --test-threads=1 --nocapture
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use chrono::Duration;
use ent_types::evaluations::{
    CreerDevoirGrilleBody, GenererBulletinsBody, PublierBulletinsBody, SaisieGrilleBody,
};
use ent_web_api::{ApiClient, ApiError};
use ent_web_app::modules::evaluations::modele::changements;
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
async fn pages_evaluations_rendues_par_le_serveur() {
    let Some(base) = demarrer().await else { return };
    let id = Uuid::nil();
    for chemin in [
        "/web/evaluations".to_string(),
        format!("/web/evaluations/grille?service={id}&periode={id}"),
        format!("/web/evaluations/classes/{id}?periode={id}"),
        format!("/web/evaluations/conseils/{id}"),
        format!("/web/evaluations/bulletins?classe={id}&periode={id}"),
        format!("/web/evaluations/eleves/{id}"),
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

/// Une période complète, avec le client du frontend, et les permissions.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn periode_complete_avec_le_client_du_frontend() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let etab = l.etablissement_id;
    let url = std::env::var("ENT_DATABASE_URL").unwrap();
    let ent = ent_db::PgEnt::connect(&url).await.expect("connexion base");
    ent.migrate().await.expect("migrations");

    // Préparation du référentiel (hors périmètre de l'écran) : classe, matière, service du
    // professeur (PP), un élève neuf et le responsable de Harry lié avec « voir les notes ».
    let prof = personnel(&l, "prof");
    let annee = ent.referentiel_annee_courante_assuree(etab).await.unwrap();
    let suffixe = &Uuid::new_v4().simple().to_string()[..6];
    let classe = ent
        .referentiel_creer_classe(
            "admin",
            etab,
            Some(annee.id),
            &format!("W27-{suffixe}"),
            None,
            None,
        )
        .await
        .unwrap();
    let matiere = ent
        .create_matiere(
            ent_db::CreateMatiere {
                etablissement_id: etab,
                code: format!("WM{suffixe}"),
                libelle: "Mathématiques".into(),
                source_nomenclature: None,
            },
            None,
        )
        .await
        .unwrap();
    let service = ent
        .referentiel_creer_service(
            "admin",
            etab,
            ent_db::CreerService {
                prof_user_id: prof.user_id,
                matiere_id: Some(matiere.id),
                classe_id: Some(classe.id),
                groupe_id: None,
                professeur_principal: true,
            },
        )
        .await
        .unwrap();
    let eleve = ent
        .create_eleve(ent_db::CreateEleve {
            etablissement_id: etab,
            nom: "Lovegood".into(),
            prenom: "Luna".into(),
            ine: Some("123456789ZZ".into()),
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
    ent.referentiel_affecter_eleve_classe("admin", etab, classe.id, eleve.id, None)
        .await
        .unwrap();
    let parent = l
        .personas
        .iter()
        .find(|p| p.role == "parent" && p.label.contains("Harry"))
        .expect("parent de Harry");
    let c_parent = client_de(&base, &l, &parent.email).await;
    let parent_id = c_parent.moi().await.expect("moi").user_id;
    ent.link_parent(ent_db::CreateLien {
        etablissement_id: etab,
        eleve_id: eleve.id,
        user_id: parent_id,
        foyer_id: None,
        lien: "responsable".into(),
        peut_voir_notes: true,
        peut_voir_vs: true,
        peut_justifier_absence: false,
        peut_autoriser_sorties: false,
        est_payeur: false,
        part_paiement: None,
    })
    .await
    .unwrap();
    let t = ent_db::jour_paris();
    let periode = ent
        .creer_note_periode(ent_db::CreerNotePeriode {
            etablissement_id: etab,
            code: format!("W{suffixe}"),
            libelle: "Trimestre W".into(),
            date_debut: (t - Duration::days(5)).max(annee.date_debut),
            date_fin: (t + Duration::days(20)).min(annee.date_fin),
            actor_user_id: None,
        })
        .await
        .unwrap();

    // 1. Le professeur voit son service, crée deux évaluations et saisit la grille.
    let c_prof = client_de(&base, &l, &prof.email).await;
    let services = c_prof.services_evaluables().await.expect("services");
    assert!(services.iter().any(|s| s.service_id == service.id));
    let periodes = c_prof.periodes_notes().await.expect("périodes");
    assert!(periodes
        .iter()
        .any(|p| p.id == periode.id && p.statut == "ouverte"));
    let mut devoirs = Vec::new();
    for (titre, coef, sur) in [("Contrôle", 1.0, 20.0), ("Exposé", 2.0, 10.0)] {
        devoirs.push(
            c_prof
                .creer_devoir_grille(&CreerDevoirGrilleBody {
                    service_id: service.id,
                    periode_id: periode.id,
                    titre: titre.into(),
                    date_devoir: t,
                    coef,
                    sur,
                })
                .await
                .expect("devoir"),
        );
    }
    // La grille de l'écran : seules les cellules modifiées partent (modèle du frontend).
    for (d, texte) in devoirs.iter().zip(["14,5", "abs"]) {
        let resultats =
            changements(&[(eleve.id, String::new(), texte.to_string())], d.sur).unwrap();
        let r = c_prof
            .saisir_resultats(d.id, &SaisieGrilleBody { resultats })
            .await
            .expect("saisie");
        assert_eq!(r.enregistres, 1);
    }
    let g = c_prof
        .grille_notes(service.id, periode.id)
        .await
        .expect("grille");
    let ligne = g.lignes.iter().find(|x| x.eleve_id == eleve.id).unwrap();
    assert_eq!(ligne.moyenne, Some(14.5), "l'absent ne compte pas");
    assert!(g.saisie_ouverte);

    // 2. Avant publication, la famille ne voit rien ; après, le relevé montre l'évaluation.
    let r = c_parent
        .releve_eleve(eleve.id, periode.id)
        .await
        .expect("relevé");
    assert!(r.matieres.iter().all(|m| m.evaluations.is_empty()));
    c_prof
        .publier_devoir(devoirs[0].id)
        .await
        .expect("publication");
    let r = c_parent
        .releve_eleve(eleve.id, periode.id)
        .await
        .expect("relevé");
    let m = r
        .matieres
        .iter()
        .find(|m| m.matiere_id == matiere.id)
        .unwrap();
    assert_eq!((m.evaluations.len(), m.moyenne), (1, Some(14.5)));
    // La famille ne saisit pas, ne lit pas la grille.
    assert!(matches!(
        c_parent.grille_notes(service.id, periode.id).await,
        Err(ApiError::Interdit(_))
    ));

    // 3. Direction : fermeture, génération, publication ; la famille télécharge le PDF.
    let c_dir = client_de(&base, &l, &personnel(&l, "direction").email).await;
    assert!(matches!(
        c_prof
            .changer_statut_periode(periode.id, "fermee", None)
            .await,
        Err(ApiError::Interdit(_))
    ));
    let p = c_dir
        .changer_statut_periode(periode.id, "fermee", None)
        .await
        .expect("fermeture");
    assert_eq!(p.statut, "fermee");
    let gen = c_dir
        .generer_bulletins(&GenererBulletinsBody {
            classe_id: classe.id,
            periode_id: periode.id,
            motif: None,
        })
        .await
        .expect("génération");
    assert_eq!(gen.bulletins.len(), 1);
    assert!(c_parent.bulletins_eleve(eleve.id).await.unwrap().is_empty());
    c_dir
        .publier_bulletins(&PublierBulletinsBody {
            classe_id: classe.id,
            periode_id: periode.id,
        })
        .await
        .expect("publication");
    let b = c_parent.bulletins_eleve(eleve.id).await.expect("bulletins");
    assert_eq!(b.len(), 1);
    let pdf = c_parent.bulletin_pdf(b[0].id).await.expect("PDF");
    assert!(pdf.starts_with(b"%PDF-1.4"));
    let detail = c_parent.bulletin(b[0].id).await.expect("détail");
    assert_eq!(detail.contenu.moyenne_generale, Some(14.5));

    // 4. Export LSU par le secrétariat, avec l'avertissement [NV].
    let c_adm = client_de(&base, &l, &personnel(&l, "administratif").email).await;
    let e = c_adm
        .exporter_livret("lsu", classe.id, periode.id)
        .await
        .expect("export");
    assert!(e.avertissement.contains("[NV]"));
    assert!(e.contenu.contains("id-be=\"123456789ZZ\""));
    assert!(matches!(
        c_prof.exporter_livret("lsu", classe.id, periode.id).await,
        Err(ApiError::Interdit(_))
    ));
}
