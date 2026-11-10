//! Socle 6, lot P3 — services d'enseignement et interventions, côté HTTP (base locale).
//!
//! * création, répartition, suppléance (une saisie par jour), correction, remplacement d'agent ;
//! * matrice de droits côté serveur : la direction et le secrétariat gèrent, le professeur lit ;
//! * codes HTTP : 201, 401, 403, 404 (autre établissement), 409 (conflit), 422 (invalide) ;
//! * aucune donnée personnelle superflue dans les réponses.

use super::tests_l3::{appel, etat, inscrire};
use super::*;
use axum::http::StatusCode;
use ent_db::CreateMatiere;
use serde_json::{json, Value};
use uuid::Uuid;

async fn agent_du(app: &axum::Router, jeton: &str, user_email: &str) -> String {
    let (st, liste) = appel(app, "GET", "/personnels/annuaire", Some(jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{liste}");
    liste
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l.to_string().contains(user_email))
        .or_else(|| liste.as_array().unwrap().first())
        .unwrap()["agent_id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn cycle_de_vie_d_un_service_et_de_ses_interventions() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip P3: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let sec = inscrire(&app, etab, "administratif").await;
    let prof = inscrire(&app, etab, "prof").await;
    let _prof2 = inscrire(&app, etab, "prof").await;
    let parent = inscrire(&app, etab, "parent").await;
    let annee = ent.referentiel_annee_courante_assuree(etab).await.unwrap();
    let classe = ent
        .referentiel_creer_classe(
            "administratif",
            etab,
            Some(annee.id),
            &format!("A{}", &Uuid::new_v4().simple().to_string()[..5]),
            Some("4e"),
            Some("4EME"),
        )
        .await
        .unwrap();
    let matiere = ent
        .create_matiere(
            CreateMatiere {
                etablissement_id: etab,
                code: format!("H{}", &Uuid::new_v4().simple().to_string()[..5]),
                libelle: "Histoire".into(),
                source_nomenclature: None,
            },
            None,
        )
        .await
        .unwrap();
    // Les agents des deux professeurs (via l'annuaire).
    let (_, annuaire) = appel(&app, "GET", "/personnels/annuaire", Some(&dir.jeton), None).await;
    let agents: Vec<String> = annuaire
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["fonction_code"] == "enseignant")
        .map(|l| l["agent_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(agents.len(), 2);
    let _ = agent_du;
    let (a1, a2) = (agents[0].clone(), agents[1].clone());

    // 401 sans jeton ; 403 pour le professeur et le parent.
    let corps = json!({"annee_id": annee.id, "classe_id": classe.id, "matiere_id": matiere.id, "heures_hebdo": 3.0});
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/services",
            None,
            Some(corps.clone())
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/services",
            Some(&prof.jeton),
            Some(corps.clone())
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "GET",
            "/personnels/services",
            Some(&parent.jeton),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // 422 : modalité inconnue.
    let (st, _) = appel(&app, "POST", "/personnels/services", Some(&dir.jeton), Some(json!({"annee_id": annee.id, "classe_id": classe.id, "matiere_id": matiere.id, "modalite": "zzz"}))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);

    // Le secrétariat crée un service vacant avec titulaire absent : 201, vacant.
    let (st, s) = appel(
        &app,
        "POST",
        "/personnels/services",
        Some(&sec.jeton),
        Some(corps.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{s}");
    let sid = s["id"].as_str().unwrap().to_string();
    assert_eq!(s["vacant"], true);
    assert_eq!(s["etat"], "ouvert");
    // Un titulaire le pourvoit.
    let (st, s) = appel(
        &app,
        "POST",
        &format!("/personnels/services/{sid}/intervenants"),
        Some(&dir.jeton),
        Some(json!({"agent_id": a1, "nature": "titulaire", "du": annee.date_debut})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{s}");
    assert_eq!(s["vacant"], false);
    let titulaire = s["intervenants"][0]["id"].as_str().unwrap().to_string();
    // 409 : un second titulaire en même temps.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/personnels/services/{sid}/intervenants"),
        Some(&dir.jeton),
        Some(json!({"agent_id": a2, "nature": "titulaire", "du": annee.date_debut})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    // Suppléance : 201 ; une deuxième sur les mêmes jours : 409 ; la saisie effective passe au suppléant.
    let du = annee.date_debut + chrono::Duration::days(20);
    let au = annee.date_debut + chrono::Duration::days(30);
    let (st, s) = appel(
        &app,
        "POST",
        &format!("/personnels/services/{sid}/suppleance"),
        Some(&dir.jeton),
        Some(json!({"agent_id": a2, "du": du, "au": au})),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{s}");
    let supp = s["intervenants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["nature"] == "suppleant")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/personnels/services/{sid}/suppleance"),
        Some(&dir.jeton),
        Some(json!({"agent_id": a2, "du": du, "au": au})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    // Lecture : le professeur voit la répartition filtrée par agent, les vacants à zéro.
    let (st, rep) = appel(
        &app,
        "GET",
        &format!("/personnels/services?agent_id={a1}&le={du}"),
        Some(&prof.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{rep}");
    let ligne = rep["services"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == sid.as_str())
        .unwrap();
    let eff: Vec<(String, bool)> = ligne["intervenants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["nature"].as_str().unwrap().to_string(),
                i["saisie_effective"].as_bool().unwrap(),
            )
        })
        .collect();
    assert!(
        eff.contains(&("titulaire".to_string(), false))
            && eff.contains(&("suppleant".to_string(), true)),
        "{eff:?}"
    );
    assert_eq!(rep["vacants"], 0);
    // « Mes services » : le titulaire retrouve son service ; le parent n'a pas ce droit.
    let (st, moi) = appel(
        &app,
        "GET",
        "/personnels/moi/services",
        Some(&prof.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{moi}");
    assert_eq!(
        appel(
            &app,
            "GET",
            "/personnels/moi/services",
            Some(&parent.jeton),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Fin anticipée de la suppléance, puis correction de l'intervention du titulaire.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/personnels/interventions/{supp}/terminer"),
        Some(&dir.jeton),
        Some(json!({"au": du})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, s) = appel(&app, "POST", &format!("/personnels/interventions/{titulaire}/corriger"), Some(&sec.jeton), Some(json!({"motif": "erreur_saisie", "remplacement": {"agent_id": a2, "nature": "titulaire", "du": annee.date_debut + chrono::Duration::days(31)}}))).await;
    assert_eq!(st, StatusCode::OK, "{s}");
    assert!(s["intervenants"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["id"] == titulaire.as_str() && i["annulee"] == true));
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/personnels/interventions/{titulaire}/corriger"),
        Some(&sec.jeton),
        Some(json!({"motif": "doublon"})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    // Remplacer un agent : l'aperçu ne change rien, l'application oui.
    let corps_rempl = |apercu: bool| json!({"agent_sortant_id": a2, "agent_entrant_id": a1, "a_partir_du": annee.date_debut + chrono::Duration::days(60), "apercu": apercu});
    let (st, ap) = appel(
        &app,
        "POST",
        "/personnels/services/remplacer-agent",
        Some(&dir.jeton),
        Some(corps_rempl(true)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{ap}");
    assert_eq!(ap["apercu"], true);
    assert!(!ap["lignes"].as_array().unwrap().is_empty());
    let (st, r) = appel(
        &app,
        "POST",
        "/personnels/services/remplacer-agent",
        Some(&dir.jeton),
        Some(corps_rempl(false)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert!(r["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["resultat"] == "fait"));
    // Clore puis annuler une fois clos : 409.
    let (st, c) = appel(
        &app,
        "POST",
        &format!("/personnels/services/{sid}/clore"),
        Some(&dir.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{c}");
    assert_eq!(c["etat"], "clos");
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/personnels/services/{sid}/annuler"),
        Some(&dir.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    // Créneaux sans service : lisible par le personnel.
    assert_eq!(
        appel(
            &app,
            "GET",
            "/personnels/services/creneaux-sans-service",
            Some(&prof.jeton),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    // Aucune donnée superflue : ni mot de passe, ni champ e-mail, téléphone ou adresse dans une réponse de service.
    let (_, fin) = appel(
        &app,
        "GET",
        &format!("/personnels/services/{sid}"),
        Some(&dir.jeton),
        None,
    )
    .await;
    for interdit in ["password", "\"email\"", "telephone", "adresse"] {
        assert!(
            !fin.to_string().contains(interdit),
            "« {interdit} » dans {fin}"
        );
    }

    // Un autre établissement : 404 uniforme sur ce service et sur ses interventions.
    let autre = Uuid::new_v4();
    let dir_b = inscrire(&app, autre, "direction").await;
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/personnels/services/{sid}"),
            Some(&dir_b.jeton),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/personnels/interventions/{supp}/terminer"),
            Some(&dir_b.jeton),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "PUT",
            &format!("/personnels/services/{sid}"),
            Some(&dir_b.jeton),
            Some(json!({"heures_hebdo": 1.0}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let _: Value = json!(null);
}
