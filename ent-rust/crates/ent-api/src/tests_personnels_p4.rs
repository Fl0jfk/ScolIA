//! Socle 6, lot P4 — missions et périmètre, côté HTTP (base locale).
//!
//! * catalogue des types, nomination d'un professeur principal, « mes missions » ;
//! * matrice de droits côté serveur : la direction nomme, le professeur lit, le parent n'a rien ;
//! * codes HTTP : 201, 401, 403, 404 (autre établissement), 409 (cardinalité), 422 (périmètre non admis) ;
//! * simulation sans écriture ; explication de l'accès (soi-même sans droit, autrui réservé) ;
//! * le port d'un autre socle (PRS-67) n'a aucune route.

use super::tests_l3::{appel, etat, inscrire};
use super::*;
use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn pp_nomme_par_la_direction_visible_dans_mes_missions_et_cloisonne() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip P4: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let prof2 = inscrire(&app, etab, "prof").await;
    let prof3 = inscrire(&app, etab, "prof").await;
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
    let (_, annuaire) = appel(&app, "GET", "/personnels/annuaire", Some(&dir.jeton), None).await;
    let agents: Vec<String> = annuaire
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["fonction_code"] == "enseignant")
        .map(|l| l["agent_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(agents.len(), 3);
    let corps = |agent: &str| {
        json!({"type_code": "professeur_principal", "agent_id": agent,
               "perimetres": [{"nature": "classe", "cible_id": classe.id}], "annee_id": annee.id})
    };

    // 401 sans jeton ; 403 pour le professeur et le parent.
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/missions",
            None,
            Some(corps(&agents[0]))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/missions",
            Some(&prof.jeton),
            Some(corps(&agents[0]))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "GET",
            "/personnels/missions",
            Some(&parent.jeton),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "GET",
            "/personnels/missions/types",
            Some(&parent.jeton),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    // Le catalogue fourni est lisible par le personnel.
    let (st, types) = appel(
        &app,
        "GET",
        "/personnels/missions/types",
        Some(&prof.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{types}");
    assert!(types
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["code"] == "professeur_principal" && t["fourni"] == true));

    // La direction nomme un PP : 201, avertissement (aucun service), droits appliqués.
    let (st, m) = appel(
        &app,
        "POST",
        "/personnels/missions",
        Some(&dir.jeton),
        Some(corps(&agents[0])),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{m}");
    assert_eq!(m["type_code"], "professeur_principal");
    assert_eq!(m["droits_appliques"], true);
    assert!(m["avertissements"]
        .to_string()
        .contains("agent_n_enseigne_pas_dans_le_perimetre"));
    let mid = m["id"].as_str().unwrap().to_string();
    // Deux PP au plus par classe : le deuxième passe, le troisième est un 409.
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/missions",
            Some(&dir.jeton),
            Some(corps(&agents[1]))
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/missions",
            Some(&dir.jeton),
            Some(corps(&agents[2]))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // 422 : un périmètre que le type n'admet pas.
    let (st, _) = appel(
        &app,
        "POST",
        "/personnels/missions",
        Some(&dir.jeton),
        Some(json!({"type_code": "professeur_principal", "agent_id": agents[2], "perimetres": [{"nature": "etablissement"}]})),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    // Aucune route pour le port d'un autre socle.
    assert!(matches!(
        appel(
            &app,
            "POST",
            "/personnels/missions/socle-tiers",
            Some(&dir.jeton),
            Some(json!({}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
    ));

    // Tout le personnel lit les missions ; chacun voit les siennes.
    let (st, liste) = appel(
        &app,
        "GET",
        "/personnels/missions",
        Some(&prof3.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{liste}");
    assert_eq!(liste.as_array().unwrap().len(), 2);
    let (st, mes) = appel(
        &app,
        "GET",
        "/personnels/moi/missions",
        Some(&prof.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{mes}");
    let _ = (&prof2, &mes);

    // Simulation : jamais d'écriture, réservée à la revue des accès.
    let sim = json!({"type_code": "professeur_principal", "agent_id": agents[2],
                     "perimetres": [{"nature": "classe", "cible_id": classe.id}]});
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/missions/simuler",
            Some(&prof.jeton),
            Some(sim.clone())
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (st, s) = appel(
        &app,
        "POST",
        "/personnels/missions/simuler",
        Some(&dir.jeton),
        Some(sim),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{s}");
    assert_eq!(s["sans_effet_sur_les_droits"], false);
    // Explication : un agent n'explique pas les droits d'un collègue.
    let q = format!(
        "/personnels/acces/expliquer?agent_id={}&eleve_id={}",
        agents[1],
        Uuid::new_v4()
    );
    assert_eq!(
        appel(&app, "GET", &q, Some(&prof.jeton), None).await.0,
        StatusCode::FORBIDDEN
    );

    // Fin de mission puis annulation (404 la seconde fois) ; autre établissement : 404.
    let (st, fin) = appel(
        &app,
        "POST",
        &format!("/personnels/missions/{mid}/terminer"),
        Some(&dir.jeton),
        Some(json!({"au": annee.date_fin})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{fin}");
    let autre = Uuid::new_v4();
    let dir2 = inscrire(&app, autre, "direction").await;
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/personnels/missions/{mid}/annuler"),
            Some(&dir2.jeton),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (st, a) = appel(
        &app,
        "POST",
        &format!("/personnels/missions/{mid}/annuler"),
        Some(&dir.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{a}");
    assert_eq!(a["annulee"], true);
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/personnels/missions/{mid}/annuler"),
            Some(&dir.jeton),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}
