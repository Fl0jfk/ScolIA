//! Socle 6, lot P5 — équipes et responsables de groupes, côté HTTP
//! (base locale). Matrice de droits côté serveur, codes HTTP (201, 401, 403, 404, 409, 422), séparation des établissements.

use super::tests_l3::{appel, etat, inscrire};
use super::*;
use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn equipes_et_responsables_de_groupe_matrice_de_droits_et_cloisonnement() {
    let Some((state, ent)) = etat().await else {
        eprintln!("skip P5: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let parent = inscrire(&app, etab, "parent").await;
    // 401 sans jeton ; le parent n'a ni types ni liste ; le professeur lit les types.
    assert_eq!(
        appel(&app, "GET", "/personnels/equipes/types", None, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        appel(
            &app,
            "GET",
            "/personnels/equipes/types",
            Some(&parent.jeton),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (st, types) = appel(
        &app,
        "GET",
        "/personnels/equipes/types",
        Some(&prof.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(types.as_array().unwrap().len(), 8);
    // Création : la direction (201) ; le professeur sans lien (403).
    let corps = |agent: &str| {
        json!({"type_code": "cellule_veille", "libelle": "Veille", "confidentielle": true,
                                      "membres": [{"agent_id": agent}]})
    };
    let (_, annuaire) = appel(&app, "GET", "/personnels/annuaire", Some(&dir.jeton), None).await;
    let membre_agent = annuaire
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["fonction_code"] == "enseignant")
        .map(|x| x["agent_id"].as_str().unwrap().to_string())
        .expect("un agent enseignant");
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/equipes",
            Some(&prof.jeton),
            Some(corps(&membre_agent))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (st, e) = appel(
        &app,
        "POST",
        "/personnels/equipes",
        Some(&dir.jeton),
        Some(corps(&membre_agent)),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{e}");
    assert_eq!(e["ouvre_des_droits"], false);
    let id = e["id"].as_str().unwrap().to_string();
    // Un autre établissement : 404 ; type inconnu : 404 ; objet sans élève : 422.
    let autre = Uuid::new_v4();
    let dir2 = inscrire(&app, autre, "direction").await;
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/personnels/equipes/{id}"),
            Some(&dir2.jeton),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (st, _) = appel(
        &app,
        "POST",
        "/personnels/equipes",
        Some(&dir.jeton),
        Some(json!({"type_code": "equipe_educative_eleve", "libelle": "Suivi"})),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    // Clôture : 200 puis 409.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/personnels/equipes/{id}/clore"),
        Some(&dir.jeton),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/personnels/equipes/{id}/clore"),
        Some(&dir.jeton),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    // Équipe calculée : la direction lit une classe ; le parent n'a pas ce droit (403) ; nature inconnue : 422.
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
    let uri = format!(
        "/personnels/equipes/calculee?nature=classe&cible_id={}",
        classe.id
    );
    let (st, c) = appel(&app, "GET", &uri, Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{c}");
    assert!(c["membres"].as_array().unwrap().is_empty());
    assert_eq!(
        appel(&app, "GET", &uri, Some(&parent.jeton), None).await.0,
        StatusCode::FORBIDDEN
    );
    let (st, _) = appel(
        &app,
        "GET",
        &format!(
            "/personnels/equipes/calculee?nature=planete&cible_id={}",
            classe.id
        ),
        Some(&dir.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    // Responsables de groupe : 201 pour la direction, 403 pour le professeur, 404 pour un autre établissement.
    let groupe = ent
        .referentiel_creer_groupe(
            "administratif",
            etab,
            Some(annee.id),
            &format!("G{}", &Uuid::new_v4().simple().to_string()[..5]),
            Some("Atelier"),
            None,
        )
        .await
        .unwrap();
    let uri = format!("/personnels/groupes/{}/responsables", groupe.id);
    let corps = json!({"agent_id": membre_agent, "nature": "animateur"});
    assert_eq!(
        appel(&app, "POST", &uri, Some(&prof.jeton), Some(corps.clone()))
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(&app, "POST", &uri, Some(&dir2.jeton), Some(corps.clone()))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let (st, r) = appel(&app, "POST", &uri, Some(&dir.jeton), Some(corps.clone())).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    assert_eq!(
        appel(&app, "POST", &uri, Some(&dir.jeton), Some(corps))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let rid = r["id"].as_str().unwrap();
    let (st, t) = appel(
        &app,
        "POST",
        &format!("/personnels/groupes/responsables/{rid}/terminer"),
        Some(&dir.jeton),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{t}");
}
