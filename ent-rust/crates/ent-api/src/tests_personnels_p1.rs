//! Socle 6, lot P1 — agents et engagements, côté HTTP (base locale).
//!
//! * un compte du personnel crée un agent, visible dans l'annuaire et la fiche ;
//! * création par la direction avec correspondances à confirmer (409), puis création confirmée ;
//! * matrice de droits côté serveur (le client ne décide de rien) ;
//! * disponibilité : suspension d'engagement sans clôture, accès coupé puis rétabli ;
//! * cloisonnement : une référence d'un autre établissement répond 404 ;
//! * confidentialité : contact d'urgence et motif d'identité restreinte.

use super::tests_l3::{appel, connecter, etat, inscrire, vivante};
use super::*;
use axum::http::StatusCode;
use serde_json::{json, Value};
use uuid::Uuid;

fn contient(v: &Value, ident: &str) -> bool {
    v.to_string().contains(ident)
}

#[tokio::test]
async fn annuaire_fiche_creation_et_matrice_de_droits() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip P1: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let cpe = inscrire(&app, etab, "cpe").await;
    let parent = inscrire(&app, etab, "parent").await;

    // L'annuaire (tout le personnel) liste les comptes, sans téléphone ni secret ; les familles n'y entrent pas.
    let (st, liste) = appel(&app, "GET", "/personnels/annuaire", Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{liste}");
    let lignes = liste.as_array().unwrap();
    assert_eq!(lignes.len(), 3);
    for l in lignes {
        assert!(l.get("telephone").is_none() && l.get("password").is_none());
        assert_eq!(l["etat"], "actif");
    }
    assert!(lignes.iter().any(|l| l["fonction_code"] == "enseignant"));
    assert_eq!(
        appel(
            &app,
            "GET",
            "/personnels/annuaire",
            Some(&parent.jeton),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(&app, "GET", "/personnels/annuaire", None, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    // Les états autres que « actif » : fiche d'organisation exigée.
    assert_eq!(
        appel(
            &app,
            "GET",
            "/personnels/annuaire?etat=parti",
            Some(&prof.jeton),
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
            "/personnels/annuaire?etat=parti",
            Some(&dir.jeton),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        appel(
            &app,
            "GET",
            "/personnels/annuaire?etat=nimporte",
            Some(&dir.jeton),
            None
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // Fiche : la direction la lit, un professeur n'a pas à lire celle d'un collègue, il lit la sienne.
    let agent_prof = lignes
        .iter()
        .find(|l| l["fonction_code"] == "enseignant")
        .unwrap()["agent_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (st, fiche) = appel(
        &app,
        "GET",
        &format!("/personnels/agents/{agent_prof}"),
        Some(&dir.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{fiche}");
    assert_eq!(fiche["etat"], "actif");
    assert_eq!(fiche["fonctions"][0]["fonction_code"], "enseignant");
    assert_eq!(fiche["engagements"][0]["nature"], "a_preciser");
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/personnels/agents/{agent_prof}"),
            Some(&prof.jeton),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/personnels/agents/{agent_prof}"),
            Some(&cpe.jeton),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    // Création : refusée à un professeur (le serveur décide), acceptée à la direction.
    let corps =
        json!({"personne": {"nom": "Lupin", "prenom": "Remus"}, "fonction_code": "enseignant"});
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/agents",
            Some(&prof.jeton),
            Some(corps.clone())
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (st, cree) = appel(
        &app,
        "POST",
        "/personnels/agents",
        Some(&dir.jeton),
        Some(corps.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{cree}");
    let agent = cree["agent_id"].as_str().unwrap().to_string();
    // Même nom : 409 avec les correspondances à trancher, e-mails masqués, aucune écriture.
    let (st, conflit) = appel(
        &app,
        "POST",
        "/personnels/agents",
        Some(&dir.jeton),
        Some(corps.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(conflit["code"], "correspondances_a_confirmer");
    assert_eq!(conflit["correspondances"][0]["est_agent"], true);
    let mut confirme = corps.clone();
    confirme["confirmer_nouvelle"] = json!(true);
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/agents",
            Some(&dir.jeton),
            Some(confirme)
        )
        .await
        .0,
        StatusCode::CREATED
    );
    // Corps invalide : 422, pas 500.
    assert_eq!(
        appel(
            &app,
            "POST",
            "/personnels/agents",
            Some(&dir.jeton),
            Some(json!({}))
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // Engagement daté, puis maître du privé + salarié OGEC : deux engagements simultanés.
    let eng = |nature: &str, employeur: &str, org: Option<&str>| {
        let mut e = json!({"du": "2025-09-01", "nature": nature, "employeur_type": employeur, "quotite_pct": 50.0});
        if let Some(o) = org {
            e["employeur_organisme_id"] = json!(o);
        }
        e
    };
    let (st, e1) = appel(
        &app,
        "POST",
        &format!("/personnels/agents/{agent}/engagements"),
        Some(&dir.jeton),
        Some(eng("maitre_contractuel", "etat", None)),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{e1}");
    let ogec = Uuid::new_v4().to_string();
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/personnels/agents/{agent}/engagements"),
        Some(&dir.jeton),
        Some(eng("salarie_organisme", "organisme_gestion", Some(&ogec))),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    let (st, doublon) = appel(
        &app,
        "POST",
        &format!("/personnels/agents/{agent}/engagements"),
        Some(&dir.jeton),
        Some(eng("maitre_contractuel", "etat", None)),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "{doublon}");
    // Un autre rôle ne peut pas écrire un engagement.
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/personnels/agents/{agent}/engagements"),
            Some(&cpe.jeton),
            Some(eng("vacataire", "etat", None))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (_, fiche) = appel(
        &app,
        "GET",
        &format!("/personnels/agents/{agent}"),
        Some(&dir.jeton),
        None,
    )
    .await;
    assert_eq!(fiche["engagements"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn disponibilite_coupe_puis_retablit_l_acces_sans_cloturer_l_engagement() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip P1: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let (_, liste) = appel(
        &app,
        "GET",
        "/personnels/annuaire?q=prof",
        Some(&dir.jeton),
        None,
    )
    .await;
    let agent = liste
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["fonction_code"] == "enseignant")
        .unwrap()["agent_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (_, fiche) = appel(
        &app,
        "GET",
        &format!("/personnels/agents/{agent}"),
        Some(&dir.jeton),
        None,
    )
    .await;
    let eng = fiche["engagements"][0]["id"].as_str().unwrap().to_string();
    let aujourdhui = chrono::Utc::now().date_naive().to_string();
    assert!(vivante(&app, &prof.jeton).await);

    let (st, e) = appel(
        &app,
        "POST",
        &format!("/personnels/engagements/{eng}/suspendre"),
        Some(&dir.jeton),
        Some(json!({"du": aujourdhui, "motif_code": "disponibilite"})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{e}");
    assert!(e["au"].is_null(), "engagement non clos");
    assert_eq!(e["suspendu"], true);
    // Le jeton déjà ouvert ne passe plus, la reconnexion non plus ; l'engagement reste ouvert.
    assert!(!vivante(&app, &prof.jeton).await);
    let (st, _) = appel(&app, "POST", "/auth/login", None, Some(json!({"email": prof.email, "password": super::tests_l3::MDP, "etablissement_id": etab}))).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (_, fiche) = appel(
        &app,
        "GET",
        &format!("/personnels/agents/{agent}"),
        Some(&dir.jeton),
        None,
    )
    .await;
    assert_eq!(fiche["etat"], "suspendu");
    // Motif codé inconnu : 422.
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/personnels/engagements/{eng}/suspendre"),
            Some(&dir.jeton),
            Some(json!({"du": aujourdhui, "motif_code": "vacances"}))
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    // Reprise : l'accès revient, même compte, sans invitation.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/personnels/engagements/{eng}/reprendre"),
        Some(&dir.jeton),
        Some(json!({"date_reprise": aujourdhui})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (jeton, _) = connecter(&app, etab, &prof.email).await;
    assert!(vivante(&app, &jeton).await);
}

#[tokio::test]
async fn cloisonnement_et_confidentialite_http() {
    let Some((state, _ent)) = etat().await else {
        eprintln!("skip P1: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let dir_a = inscrire(&app, a, "direction").await;
    let dir_b = inscrire(&app, b, "direction").await;
    let prof_a = inscrire(&app, a, "prof").await;
    let secretaire = inscrire(&app, a, "administratif").await;
    let (_, liste) = appel(
        &app,
        "GET",
        "/personnels/annuaire",
        Some(&dir_a.jeton),
        None,
    )
    .await;
    let agent_prof = liste
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["fonction_code"] == "enseignant")
        .unwrap()["agent_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Établissement B : 404 uniforme, pas de fuite dans l'annuaire.
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/personnels/agents/{agent_prof}"),
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
            &format!("/personnels/agents/{agent_prof}/engagements"),
            Some(&dir_b.jeton),
            Some(json!({"nature": "titulaire", "employeur_type": "etat"}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        appel(
            &app,
            "PUT",
            &format!("/personnels/agents/{agent_prof}/identite-restreinte"),
            Some(&dir_b.jeton),
            Some(json!({"restreinte": true, "motif": "menace"}))
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (_, annuaire_b) = appel(
        &app,
        "GET",
        "/personnels/annuaire?limite=200",
        Some(&dir_b.jeton),
        None,
    )
    .await;
    assert!(!contient(&annuaire_b, &agent_prof));

    // Contact d'urgence : déclaré par l'agent, lu par lui et la direction, jamais par le secrétariat.
    let (st, _) = appel(&app, "PUT", "/personnels/moi/contact-urgence", Some(&prof_a.jeton), Some(json!({"ordre": 1, "nom": "Proche-Secret", "telephone": "06 12 34 56 78", "lien": "conjoint"}))).await;
    assert_eq!(st, StatusCode::OK);
    let (_, vue_dir) = appel(
        &app,
        "GET",
        &format!("/personnels/agents/{agent_prof}"),
        Some(&dir_a.jeton),
        None,
    )
    .await;
    assert_eq!(vue_dir["contacts_urgence"][0]["nom"], "Proche-Secret");
    let (st, vue_sec) = appel(
        &app,
        "GET",
        &format!("/personnels/agents/{agent_prof}"),
        Some(&secretaire.jeton),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(vue_sec["contacts_urgence"].is_null());
    assert!(!contient(&vue_sec, "Proche-Secret") && !contient(&vue_sec, "06 12 34 56 78"));
    // Un parent n'est pas agent : aucune action « sur soi-même » de l'agent.
    let parent = inscrire(&app, a, "parent").await;
    assert_eq!(
        appel(
            &app,
            "PUT",
            "/personnels/moi/pause-notifications",
            Some(&parent.jeton),
            Some(json!({"jusqu_au": null}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Identité restreinte : direction seulement, motif codé, motif jamais rendu au secrétariat.
    assert_eq!(
        appel(
            &app,
            "PUT",
            &format!("/personnels/agents/{agent_prof}/identite-restreinte"),
            Some(&secretaire.jeton),
            Some(json!({"restreinte": true, "motif": "menace"}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        appel(
            &app,
            "PUT",
            &format!("/personnels/agents/{agent_prof}/identite-restreinte"),
            Some(&dir_a.jeton),
            Some(json!({"restreinte": true, "motif": "menace"}))
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    let (_, vue_sec) = appel(
        &app,
        "GET",
        &format!("/personnels/agents/{agent_prof}"),
        Some(&secretaire.jeton),
        None,
    )
    .await;
    assert_eq!(vue_sec["identite_restreinte"], true);
    assert!(vue_sec["identite_restreinte_motif"].is_null());
}
