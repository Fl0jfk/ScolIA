//! Socle 41 (internat), lots I0 et I1 : parcours HTTP et droits par rôle (base locale de test).
//!
//! Complète `ent-db/src/tests_internat.rs` (règles) : ici on vérifie que les routes appliquent les
//! droits côté serveur, que les refus de règle ont le bon statut, et qu'**aucune route d'internat
//! n'existe sans session** (ni lien public, ni jeton : NJ-22 et NJ-51 relèvent des lots I3 et I8).

use super::*;
use crate::tests_l3::{appel, etat, inscrire};
use axum::http::StatusCode;
use chrono::{Duration, NaiveDate};
use domain_eleve::CreateEleve;
use serde_json::json;
use uuid::Uuid;

async fn eleve_interne(ent: &PgEnt, etab: Uuid, sexe: &str) -> Uuid {
    let jour = domain_eleve::horloge::jour_paris();
    ent.create_eleve(CreateEleve {
        etablissement_id: etab,
        nom: format!("Api{}", Uuid::new_v4().simple()),
        prenom: "Léo".into(),
        ine: None,
        sexe: Some(sexe.into()),
        date_naissance: NaiveDate::from_ymd_opt(2010, 3, 14),
        numero_badge: None,
        redoublant: false,
        classe: None,
        regime: Some("interne".into()),
        mef: None,
        site: None,
        date_entree: Some(jour - Duration::days(400)),
        date_sortie: None,
        motif_sortie: None,
        actor_user_id: None,
    })
    .await
    .unwrap()
    .id
}

#[tokio::test]
async fn internat_api_aucune_route_sans_session() {
    let Some((state, _ent)) = etat().await else { return };
    let app = app(state);
    let id = Uuid::new_v4();
    let routes: Vec<(&str, String)> = vec![
        ("GET", "/vs/internat/chambres".into()),
        ("POST", "/vs/internat/chambres".into()),
        ("PATCH", format!("/vs/internat/chambres/{id}")),
        ("GET", "/vs/internat/affectations".into()),
        ("POST", "/vs/internat/affectations".into()),
        ("GET", "/vs/internat/presences".into()),
        ("POST", "/vs/internat/presences".into()),
        ("GET", "/vs/internat/presences/historique".into()),
        ("GET", format!("/eleves/{id}/internat")),
        ("POST", format!("/eleves/{id}/internat/suivis")),
        ("GET", "/internat/zones".into()),
        ("POST", "/internat/zones".into()),
        ("PATCH", format!("/internat/zones/{id}")),
        ("GET", "/internat/chambres".into()),
        ("POST", "/internat/chambres".into()),
        ("GET", format!("/internat/chambres/{id}/lits")),
        ("PUT", format!("/internat/lits/{id}/neutralisation")),
        ("DELETE", format!("/internat/lits/{id}/neutralisation")),
        ("GET", format!("/internat/eleves/{id}/sejours")),
        ("POST", format!("/internat/eleves/{id}/sejours/synchroniser")),
        ("POST", format!("/internat/sejours/{id}/rythmes")),
        ("POST", "/internat/affectations".into()),
        ("POST", format!("/internat/eleves/{id}/changement-chambre")),
        ("POST", "/internat/derogations".into()),
        ("POST", "/internat/interdictions-contact".into()),
        ("GET", "/internat/plan".into()),
        ("GET", "/internat/parametres".into()),
        ("PUT", "/internat/parametres".into()),
    ];
    for (methode, uri) in routes {
        let corps = if methode == "GET" || methode == "DELETE" { None } else { Some(json!({})) };
        let (st, _) = appel(&app, methode, &uri, None, corps).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{methode} {uri} doit exiger une session");
    }
}

#[tokio::test]
async fn internat_api_droits_par_role_et_refus_de_regle() {
    let Some((state, ent)) = etat().await else { return };
    let app = app(state);
    let etab = Uuid::new_v4();
    let cpe = inscrire(&app, etab, "cpe").await;
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let compta = inscrire(&app, etab, "compta").await;
    let garcon = eleve_interne(&ent, etab, "M").await;
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();

    // Hors équipe d'internat : 403 sur toutes les lectures et écritures (RI-03, NJ-21).
    for (c, nom) in [(&prof, "prof"), (&compta, "compta")] {
        for (m, uri, corps) in [
            ("GET", format!("/eleves/{garcon}/internat"), None),
            ("GET", "/internat/plan".to_string(), None),
            ("GET", "/vs/internat/presences".to_string(), None),
            ("POST", "/vs/internat/presences".to_string(), Some(json!({"eleve_id": garcon, "statut": "present"}))),
            ("POST", format!("/eleves/{garcon}/internat/suivis"), Some(json!({"texte": "x"}))),
        ] {
            let (st, _) = appel(&app, m, &uri, Some(&c.jeton), corps).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "{nom} : {m} {uri}");
        }
    }

    // Parcours du CPE : chambre de filles, affectation d'un garçon → 409 (public, R-IT11).
    let (st, filles) = appel(&app, "POST", "/internat/chambres", Some(&cpe.jeton), Some(json!({
        "batiment": "A", "etage": 1, "code": format!("F{}", &Uuid::new_v4().simple().to_string()[..5]),
        "capacite": 2, "public": "filles"
    }))).await;
    assert_eq!(st, StatusCode::CREATED, "{filles}");
    let (st, corps) = appel(&app, "POST", "/internat/affectations", Some(&cpe.jeton), Some(json!({
        "eleve_id": garcon, "chambre_id": filles["id"], "debut": nuit
    }))).await;
    assert_eq!(st, StatusCode::CONFLICT, "{corps}");
    let (st, g) = appel(&app, "POST", "/internat/chambres", Some(&cpe.jeton), Some(json!({
        "batiment": "A", "etage": 1, "code": format!("G{}", &Uuid::new_v4().simple().to_string()[..5]),
        "capacite": 2, "public": "garcons", "accessible_pmr": true
    }))).await;
    assert_eq!(st, StatusCode::CREATED);
    let (st, aff) = appel(&app, "POST", "/internat/affectations", Some(&cpe.jeton), Some(json!({
        "eleve_id": garcon, "chambre_id": g["id"], "debut": "pas-une-date", "motif_code": "rentree"
    }))).await;
    // `debut` doit être une date : la saisie invalide est refusée (4xx), jamais une panne.
    assert!(st.is_client_error(), "{st} {aff}");
    let debut = (nuit - Duration::days(3)).to_string();
    let (st, aff) = appel(&app, "POST", "/internat/affectations", Some(&cpe.jeton), Some(json!({
        "eleve_id": garcon, "chambre_id": g["id"], "debut": debut, "motif_code": "rentree"
    }))).await;
    assert_eq!(st, StatusCode::CREATED, "{aff}");
    assert!(aff["lit_id"].is_string() && aff["sejour_id"].is_string());

    // Présence : nuit courante par défaut ; deux pointages = deux faits (R-IT13) ; futur → 422 ;
    // passé sans motif → 422.
    let (st, p) = appel(&app, "POST", "/vs/internat/presences", Some(&cpe.jeton), Some(json!({"eleve_id": garcon, "statut": "present"}))).await;
    assert_eq!(st, StatusCode::OK, "{p}");
    let (st, _) = appel(&app, "POST", "/vs/internat/presences", Some(&cpe.jeton), Some(json!({"eleve_id": garcon, "statut": "absent"}))).await;
    assert_eq!(st, StatusCode::OK);
    let (st, h) = appel(&app, "GET", &format!("/vs/internat/presences/historique?eleve_id={garcon}&jour={nuit}"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(h.as_array().map(Vec::len), Some(2), "{h}");
    let futur = (nuit + Duration::days(1)).to_string();
    let (st, _) = appel(&app, "POST", "/vs/internat/presences", Some(&cpe.jeton), Some(json!({"eleve_id": garcon, "statut": "present", "jour": futur}))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let passe = (nuit - Duration::days(1)).to_string();
    let (st, _) = appel(&app, "POST", "/vs/internat/presences", Some(&cpe.jeton), Some(json!({"eleve_id": garcon, "statut": "present", "jour": passe}))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, _) = appel(&app, "POST", "/vs/internat/presences", Some(&cpe.jeton), Some(json!({
        "eleve_id": garcon, "statut": "present", "jour": passe, "motif_correction_code": "oubli"
    }))).await;
    assert_eq!(st, StatusCode::OK);

    // Suivis : jamais visibles de la famille à la saisie ; la santé est signalée ; 300 caractères au plus.
    let (st, s) = appel(&app, "POST", &format!("/eleves/{garcon}/internat/suivis"), Some(&cpe.jeton), Some(json!({
        "texte": "Allergie aux arachides à surveiller", "visible_famille": true
    }))).await;
    assert_eq!(st, StatusCode::CREATED);
    assert_eq!(s["visible_famille"], false);
    assert_eq!(s["avertissement"], "sante");
    let (st, _) = appel(&app, "POST", &format!("/eleves/{garcon}/internat/suivis"), Some(&cpe.jeton), Some(json!({"texte": "x".repeat(301)}))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, r) = appel(&app, "GET", &format!("/eleves/{garcon}/internat"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(r["suivis"].as_array().is_some_and(|l| !l.is_empty()), "le CPE lit les suivis privés");

    // Paramètres : lecture CPE, modification direction seulement.
    let (st, _) = appel(&app, "PUT", "/internat/parametres", Some(&cpe.jeton), Some(json!({"seuil_retard_minutes": 40}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, p) = appel(&app, "PUT", "/internat/parametres", Some(&dir.jeton), Some(json!({"seuil_retard_minutes": 40}))).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(p["seuil_retard_minutes"], 40);
    // Plan : sans nom d'élève.
    let (st, plan) = appel(&app, "GET", "/internat/plan", Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(!plan.to_string().contains("Api"), "le plan ne porte aucun nom : {plan}");
    // Chambre : réduire sous l'occupation → 409.
    let (st, _) = appel(&app, "PATCH", &format!("/vs/internat/chambres/{}", g["id"].as_str().unwrap()), Some(&cpe.jeton), Some(json!({"capacite": 1}))).await;
    assert_eq!(st, StatusCode::OK, "1 occupant pour 1 lit : accepté");
    let autre = eleve_interne(&ent, etab, "M").await;
    let (st, _) = appel(&app, "POST", "/internat/affectations", Some(&cpe.jeton), Some(json!({
        "eleve_id": autre, "chambre_id": g["id"], "debut": nuit
    }))).await;
    assert_eq!(st, StatusCode::CONFLICT, "chambre pleine");
}
