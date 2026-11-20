//! Socle 41 (internat), lots I0, I1 et I2 : parcours HTTP et droits par rôle (base locale de test).
//!
//! Complète `ent-db/src/tests_internat.rs` (règles) : ici on vérifie que les routes appliquent les
//! droits côté serveur, que les refus de règle ont le bon statut, et qu'**aucune route d'internat
//! n'existe sans session** (ni lien public, ni jeton : NJ-22 et NJ-51 relèvent des lots I3 et I8).

use super::*;
use crate::tests_l3::{appel, etat, inscrire};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use chrono::{Duration, NaiveDate};
use domain_eleve::CreateEleve;
use serde_json::json;
use uuid::Uuid;

/// Requête à corps brut : le corps et l'en-tête `Content-Type` sont choisis par le test (un corps VIDE accompagné de
/// `Content-Type: application/json`, ce que `Option<Json<_>>` refuse).
async fn appel_brut(
    app: &axum::Router,
    methode: &str,
    uri: &str,
    jeton: &str,
    content_type: Option<&str>,
    corps: &str,
) -> (StatusCode, serde_json::Value) {
    let mut b = Request::builder().method(methode).uri(uri).header("authorization", format!("Bearer {jeton}"));
    if let Some(ct) = content_type {
        b = b.header("content-type", ct);
    }
    let res = app.clone().oneshot(b.body(Body::from(corps.to_string())).unwrap()).await.unwrap();
    let st = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (st, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

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
        // Lot I2 : la nuit.
        ("GET", "/internat/nuits/2026-03-28/attendu".into()),
        ("GET", "/internat/nuits/2026-03-28/effectif".into()),
        ("POST", "/internat/nuits/2026-03-28/transmission".into()),
        ("POST", "/internat/nuits-exceptionnelles".into()),
        ("POST", format!("/internat/nuits-exceptionnelles/{id}/annulation")),
        ("POST", "/internat/retraits".into()),
        ("POST", format!("/internat/retraits/{id}/annulation")),
        ("GET", "/internat/listes".into()),
        ("POST", "/internat/listes".into()),
        ("PUT", format!("/internat/listes/{id}")),
        ("GET", "/internat/appels?nuit=2026-03-28".into()),
        ("POST", "/internat/appels".into()),
        ("GET", format!("/internat/appels/{id}")),
        ("GET", format!("/internat/appels/{id}/historique")),
        ("GET", format!("/internat/appels/{id}/pdf")),
        ("POST", format!("/internat/appels/{id}/pointages")),
        ("POST", format!("/internat/appels/{id}/tous-presents")),
        ("POST", format!("/internat/appels/{id}/cloture")),
        ("POST", format!("/internat/appels/{id}/validation")),
        ("GET", "/internat/cas?nuit=2026-03-28".into()),
        ("GET", format!("/internat/cas/{id}")),
        ("POST", format!("/internat/cas/{id}/etapes")),
        ("POST", format!("/internat/cas/{id}/issue")),
        ("GET", "/internat/procedure".into()),
        ("PUT", "/internat/procedure".into()),
        ("GET", "/internat/lectures-sensibles".into()),
        // Lot I3 : absences et sorties (R-IT39 : aucune action par lien sans compte).
        ("GET", "/internat/sortie-types".into()),
        ("PUT", "/internat/sortie-types/mercredi".into()),
        ("POST", "/internat/absences".into()),
        ("GET", "/internat/absences/a-traiter".into()),
        ("POST", format!("/internat/absences/{id}/decision")),
        ("POST", format!("/internat/absences/{id}/annulation")),
        ("POST", format!("/internat/absences/{id}/opposition")),
        ("POST", format!("/internat/absences/{id}/arbitrage")),
        ("GET", format!("/internat/eleves/{id}/absences")),
        ("POST", "/internat/sorties".into()),
        ("GET", "/internat/sorties/a-traiter".into()),
        ("POST", format!("/internat/sorties/{id}/reponse")),
        ("POST", format!("/internat/sorties/{id}/decision")),
        ("POST", format!("/internat/sorties/{id}/annulation")),
        ("POST", format!("/internat/sorties/{id}/opposition")),
        ("POST", format!("/internat/sorties/{id}/arbitrage")),
        ("POST", format!("/internat/sorties/{id}/personne")),
        ("POST", format!("/internat/sorties/{id}/retard")),
        ("POST", format!("/internat/absences/{id}/depart")),
        ("POST", format!("/internat/sorties/{id}/depart")),
        ("POST", format!("/internat/sorties/{id}/retour")),
        ("GET", format!("/internat/eleves/{id}/sorties")),
        ("POST", "/internat/departs/2026-04-17/preparation".into()),
        ("GET", "/internat/departs/2026-04-17/remise".into()),
        ("GET", "/internat/nuits/2026-03-28/sorties-du-soir".into()),
        ("POST", "/internat/correspondants".into()),
        ("POST", format!("/internat/correspondants/{id}/suspension")),
        ("POST", format!("/internat/correspondants/{id}/reprise")),
        ("POST", format!("/internat/correspondants/{id}/fin")),
        ("POST", format!("/internat/contacts/{id}/requalification")),
        ("GET", format!("/internat/eleves/{id}/correspondants")),
        ("POST", "/internat/tiers".into()),
        ("GET", format!("/internat/eleves/{id}/tiers")),
        ("POST", format!("/internat/tiers/{id}/revocation")),
        ("POST", format!("/internat/tiers/{id}/opposition")),
        ("POST", format!("/internat/tiers/{id}/arbitrage")),
        ("POST", format!("/internat/tiers/{id}/reprise")),
        ("GET", format!("/internat/absences/{id}/historique")),
        ("GET", format!("/internat/sorties/{id}/historique")),
        ("GET", format!("/internat/correspondants/{id}/historique")),
        ("GET", format!("/internat/eleves/{id}/autorisations")),
        ("POST", "/internat/autorisations/demande".into()),
        ("POST", "/internat/autorisations/revocation".into()),
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

/// Parcours HTTP de la nuit (lot I2). Valable à toute heure : l'élève est hébergé tous les soirs de la
/// semaine par une version de rythme posée par la route, et chaque appel prend la nuit courante du serveur.
#[tokio::test]
async fn internat_api_la_nuit_parcours_droits_et_pdf() {
    let Some((state, ent)) = etat().await else { return };
    let app = app(state);
    let etab = Uuid::new_v4();
    let cpe = inscrire(&app, etab, "cpe").await;
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let compta = inscrire(&app, etab, "compta").await;
    let garcon = eleve_interne(&ent, etab, "M").await;
    // Nuit passée FIXE (règle 33 : aucune dépendance à l'heure réelle) ; la transmission du matin (M3) exige une nuit
    // terminée, et les appels d'une nuit passée sont des saisies a posteriori (le CPE a le droit de valider).
    let nuit = chrono::NaiveDate::from_ymd_opt(2026, 3, 28).unwrap();

    let (st, g) = appel(&app, "POST", "/internat/chambres", Some(&cpe.jeton), Some(json!({
        "batiment": "A", "etage": 1, "code": format!("N{}", &Uuid::new_v4().simple().to_string()[..5]),
        "capacite": 2, "public": "garcons"
    }))).await;
    assert_eq!(st, StatusCode::CREATED, "{g}");
    let debut = (nuit - Duration::days(3)).to_string();
    let (st, aff) = appel(&app, "POST", "/internat/affectations", Some(&cpe.jeton), Some(json!({
        "eleve_id": garcon, "chambre_id": g["id"], "debut": debut, "motif_code": "rentree"
    }))).await;
    assert_eq!(st, StatusCode::CREATED, "{aff}");
    let sejour = aff["sejour_id"].as_str().unwrap().to_string();
    let (st, r) = appel(&app, "POST", &format!("/internat/sejours/{sejour}/rythmes"), Some(&cpe.jeton), Some(json!({
        "du": (nuit - Duration::days(2)).to_string(), "dimanche": true, "lundi": true, "mardi": true,
        "mercredi": true, "jeudi": true, "vendredi": true, "samedi": true
    }))).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");

    // Hors équipe d'internat : 403 partout (professeur, comptabilité).
    for (c, nom) in [(&prof, "prof"), (&compta, "compta")] {
        for (m, uri, corps) in [
            ("GET", format!("/internat/nuits/{nuit}/attendu"), None),
            ("GET", format!("/internat/appels?nuit={nuit}"), None),
            ("POST", "/internat/appels".to_string(), Some(json!({"type_appel": "soir", "perimetre": "internat", "nuit": nuit.to_string()}))),
            ("GET", format!("/internat/cas?nuit={nuit}"), None),
            ("GET", "/internat/procedure".to_string(), None),
            ("GET", format!("/internat/nuits/{nuit}/effectif"), None),
        ] {
            let (st, _) = appel(&app, m, &uri, Some(&c.jeton), corps).await;
            assert_eq!(st, StatusCode::FORBIDDEN, "{nom} : {m} {uri}");
        }
    }

    // L'attendu, puis l'appel du soir de tout l'internat.
    let (st, att) = appel(&app, "GET", &format!("/internat/nuits/{nuit}/attendu"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{att}");
    assert_eq!(att["lignes"].as_array().map(Vec::len), Some(1));
    let (st, vue) = appel(&app, "POST", "/internat/appels", Some(&cpe.jeton), Some(json!({"type_appel": "soir", "perimetre": "internat", "nuit": nuit.to_string()}))).await;
    assert_eq!(st, StatusCode::CREATED, "{vue}");
    let id = vue["appel"]["id"].as_str().unwrap().to_string();
    // Deuxième ouverture du même appel : 409 (R-IT31). Périmètre incohérent : 4xx.
    let (st, _) = appel(&app, "POST", "/internat/appels", Some(&cpe.jeton), Some(json!({"type_appel": "soir", "perimetre": "internat", "nuit": nuit.to_string()}))).await;
    assert_eq!(st, StatusCode::CONFLICT);
    let (st, _) = appel(&app, "POST", "/internat/appels", Some(&cpe.jeton), Some(json!({"type_appel": "soir", "perimetre": "zone", "nuit": nuit.to_string()}))).await;
    assert!(st.is_client_error());
    // Clôture avant pointage : 409 ; pointage ; clôture ; validation U1 ; correction sans motif : 422.
    let (st, _) = appel(&app, "POST", &format!("/internat/appels/{id}/cloture"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::CONFLICT);
    let (st, p) = appel(&app, "POST", &format!("/internat/appels/{id}/pointages"), Some(&cpe.jeton), Some(json!({"eleve_id": garcon, "marque": "present"}))).await;
    assert_eq!(st, StatusCode::CREATED, "{p}");
    let (st, p) = appel(&app, "POST", &format!("/internat/appels/{id}/pointages"), Some(&cpe.jeton), Some(json!({"eleve_id": Uuid::new_v4(), "marque": "present"}))).await;
    assert!(st.is_client_error(), "élève hors périmètre : {st} {p}");
    let (st, p) = appel(&app, "POST", &format!("/internat/appels/{id}/pointages"), Some(&cpe.jeton), Some(json!({"eleve_id": garcon, "marque": "asthme"}))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{p}");
    let (st, c) = appel(&app, "POST", &format!("/internat/appels/{id}/cloture"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{c}");
    let (st, v) = appel(&app, "POST", &format!("/internat/appels/{id}/validation"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    assert_eq!(v["urgence"], "U1");
    let (st, _) = appel(&app, "POST", &format!("/internat/appels/{id}/pointages"), Some(&cpe.jeton), Some(json!({"eleve_id": garcon, "marque": "absent"}))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "correction sans motif");
    let (st, _) = appel(&app, "POST", &format!("/internat/appels/{id}/pointages"), Some(&cpe.jeton), Some(json!({
        "eleve_id": garcon, "marque": "absent", "motif_correction_code": "erreur_saisie"
    }))).await;
    assert_eq!(st, StatusCode::CREATED);
    // Historique, PDF, liste des appels, cas, procédure par défaut, effectif, transmission idempotente.
    let (st, h) = appel(&app, "GET", &format!("/internat/appels/{id}/historique"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(h.as_array().map(Vec::len), Some(2), "{h}");
    let (st, _) = appel(&app, "GET", &format!("/internat/appels/{id}/pdf"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, l) = appel(&app, "GET", &format!("/internat/appels?nuit={nuit}"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(l.as_array().map(Vec::len), Some(1));
    let (st, pr) = appel(&app, "GET", "/internat/procedure", Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(pr["etapes"].as_array().map(Vec::len), Some(5));
    // Seules la direction et l'administration publient la procédure.
    let (st, _) = appel(&app, "PUT", "/internat/procedure", Some(&cpe.jeton), Some(json!({"etapes": []}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, e) = appel(&app, "GET", &format!("/internat/nuits/{nuit}/effectif"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(e["diner"], 1);
    let (st, t) = appel(&app, "POST", &format!("/internat/nuits/{nuit}/transmission"), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{t}");
    assert_eq!(t["nouvelle"], true);
    // Corps facultatif : vide, même avec `Content-Type: application/json`, c'est « sans motif » (et non une erreur 400).
    let (st, t) = appel_brut(&app, "POST", &format!("/internat/nuits/{nuit}/transmission"), &dir.jeton, Some("application/json"), "").await;
    assert_eq!(st, StatusCode::OK, "corps vide + Content-Type JSON : {t}");
    assert_eq!(t["nouvelle"], false);
    let (st, t) = appel_brut(&app, "POST", &format!("/internat/nuits/{nuit}/transmission"), &dir.jeton, Some("application/json"), "{}").await;
    assert_eq!(st, StatusCode::OK, "objet vide : {t}");
    let (st, t) = appel_brut(&app, "POST", &format!("/internat/nuits/{nuit}/transmission"), &dir.jeton, Some("application/json"), "{pas du json").await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "corps invalide : 400, pas 500 : {t}");
    let (st, t) = appel(&app, "POST", &format!("/internat/nuits/{nuit}/transmission"), Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(t["nouvelle"], false);
    // Journal des lectures : lisible par la direction, pas par le CPE.
    let (st, j) = appel(&app, "GET", "/internat/lectures-sensibles", Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(j.as_array().is_some_and(|l| !l.is_empty()), "{j}");
    let (st, _) = appel(&app, "GET", "/internat/lectures-sensibles", Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}

/// Une date de nuit hors bornes raisonnables (an 9999, an 1) est refusée proprement (400) par toutes les routes qui la
/// reçoivent, au lieu de faire déborder les calculs de dates de la base (500).
#[tokio::test]
async fn internat_api_dates_de_nuit_hors_bornes_refusees_en_400() {
    let Some((state, _ent)) = etat().await else { return };
    let app = app(state);
    let etab = Uuid::new_v4();
    let dir = inscrire(&app, etab, "direction").await;
    for nuit in ["9999-12-31", "+262142-12-31", "0001-01-01", "1999-12-31", "2101-01-01"] {
        for (m, uri) in [
            ("GET", format!("/internat/nuits/{nuit}/attendu")),
            ("GET", format!("/internat/nuits/{nuit}/effectif")),
            ("POST", format!("/internat/nuits/{nuit}/transmission")),
            ("GET", format!("/internat/appels?nuit={nuit}")),
            ("GET", format!("/internat/cas?nuit={nuit}")),
        ] {
            let (st, r) = appel(&app, m, &uri, Some(&dir.jeton), None).await;
            assert_eq!(st, StatusCode::BAD_REQUEST, "{m} {uri} : {r}");
        }
        let (st, r) = appel(&app, "POST", "/internat/appels", Some(&dir.jeton), Some(json!({"type_appel": "soir", "perimetre": "internat", "nuit": nuit}))).await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "ouverture d'appel {nuit} : {r}");
    }
    // Dans les bornes : la route répond normalement (jamais 400 pour la date).
    let (st, r) = appel(&app, "GET", "/internat/nuits/2026-03-28/attendu", Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{r}");
    // Corps vide avec Content-Type JSON, sans rien à transmettre : un refus MÉTIER (409), pas une erreur de corps.
    let (st, r) = appel_brut(&app, "POST", "/internat/nuits/2026-03-28/transmission", &dir.jeton, Some("application/json"), "").await;
    assert_eq!(st, StatusCode::CONFLICT, "aucun appel : refus métier, pas d'erreur de corps : {r}");
}

/// Lot I3 : parcours HTTP du CPE (régularisation d'une nuit passée, file à traiter, sortie exceptionnelle, décision,
/// départ refusé sans personne) et droits par rôle ; dates hors bornes refusées en 400. Les parcours de la famille
/// (titulaire, élève majeur, désaccords) sont éprouvés contre la base dans `ent-db/src/tests_internat_i3.rs`.
#[tokio::test]
async fn internat_api_absences_sorties_parcours_cpe_droits_et_bornes() {
    let Some((state, ent)) = etat().await else { return };
    let app = app(state);
    let etab = Uuid::new_v4();
    let cpe = inscrire(&app, etab, "cpe").await;
    let dir = inscrire(&app, etab, "direction").await;
    let prof = inscrire(&app, etab, "prof").await;
    let garcon = eleve_interne(&ent, etab, "M").await;
    let nuit = ent.nuit_courante_internat(etab).await.unwrap();
    let hier = (nuit - Duration::days(1)).to_string();

    // Les types de sortie se lisent de toute session ; seul `internat.gerer` les règle.
    let (st, t) = appel(&app, "GET", "/internat/sortie-types", Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{t}");
    assert_eq!(t.as_array().map(Vec::len), Some(6));
    let (st, _) = appel(&app, "PUT", "/internat/sortie-types/soiree", Some(&prof.jeton), Some(json!({"actif": false}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "PUT", "/internat/sortie-types/soiree", Some(&cpe.jeton), Some(json!({"autorisation": "au_hasard"}))).await;
    assert!(st.is_client_error() && st != StatusCode::FORBIDDEN, "{st}");

    // Régularisation d'une nuit passée par le CPE (CIT-37) : 201, puis visible dans l'historique de l'élève.
    let (st, a) = appel(&app, "POST", "/internat/absences", Some(&cpe.jeton), Some(json!({
        "eleve_id": garcon, "du_nuit": hier, "au_nuit": hier, "regularisation": true, "motif_texte": "appel de la mère"
    }))).await;
    assert_eq!(st, StatusCode::CREATED, "{a}");
    assert_eq!((a["statut"].as_str(), a["annonce_qualite"].as_str()), (Some("regularisee"), Some("personnel")));
    // Sans motif : refus de règle (4xx), jamais une panne ; un professeur n'annonce pas ; dates absurdes : 400.
    let (st, _) = appel(&app, "POST", "/internat/absences", Some(&cpe.jeton), Some(json!({
        "eleve_id": garcon, "du_nuit": (nuit - Duration::days(2)).to_string(), "au_nuit": (nuit - Duration::days(2)).to_string(), "regularisation": true
    }))).await;
    assert!(st.is_client_error(), "{st}");
    let (st, _) = appel(&app, "POST", "/internat/absences", Some(&prof.jeton), Some(json!({"eleve_id": garcon, "du_nuit": nuit, "au_nuit": nuit}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "POST", "/internat/absences", Some(&cpe.jeton), Some(json!({"eleve_id": garcon, "du_nuit": "9999-01-01", "au_nuit": "9999-01-02"}))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = appel(&app, "GET", "/internat/departs/9999-01-01/remise", Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let id_absence = a["id"].as_str().unwrap().to_string();
    let (st, hist) = appel(&app, "GET", &format!("/internat/absences/{id_absence}/historique"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{hist}");
    assert_eq!(hist[0]["acte"], "regularisation");
    let (st, _) = appel(&app, "GET", &format!("/internat/absences/{id_absence}/historique"), Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "GET", &format!("/internat/absences/{}/historique", Uuid::new_v4()), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, h) = appel(&app, "GET", &format!("/internat/eleves/{garcon}/absences"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{h}");
    assert_eq!(h.as_array().map(Vec::len), Some(1));
    // File à traiter : réservée à `internat.sorties.decider`.
    let (st, _) = appel(&app, "GET", "/internat/absences/a-traiter", Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, f) = appel(&app, "GET", "/internat/absences/a-traiter", Some(&dir.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{f}");
    // Trancher une absence qui n'est pas bloquée : 409 ; une absence inconnue : 404.
    let id = a["id"].as_str().unwrap().to_string();
    let (st, _) = appel(&app, "POST", &format!("/internat/absences/{id}/arbitrage"), Some(&dir.jeton), Some(json!({"decision": "lever"}))).await;
    assert_eq!(st, StatusCode::CONFLICT);
    let (st, _) = appel(&app, "POST", &format!("/internat/absences/{}/arbitrage", Uuid::new_v4()), Some(&dir.jeton), Some(json!({"decision": "lever"}))).await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Sortie exceptionnelle demandée par le CPE : en attente des titulaires (accord ponctuel), départ refusé tant que non autorisée.
    // Jour de l'établissement + 3 jours, à midi UTC (aucune lecture directe de l'horloge dans le test).
    let debut = (nuit + Duration::days(3)).and_hms_opt(10, 0, 0).unwrap().and_utc();
    let (st, s) = appel(&app, "POST", "/internat/sorties", Some(&cpe.jeton), Some(json!({
        "eleve_id": garcon, "type_code": "exceptionnelle", "debut": debut, "fin_prevue": debut + Duration::hours(3), "seul": false
    }))).await;
    assert_eq!(st, StatusCode::CREATED, "{s}");
    assert_eq!(s["statut"], "demandee");
    let sid = s["id"].as_str().unwrap().to_string();
    let (st, _) = appel(&app, "POST", &format!("/internat/sorties/{sid}/depart"), Some(&cpe.jeton), Some(json!({}))).await;
    assert_eq!(st, StatusCode::CONFLICT, "pas encore autorisée");
    let (st, _) = appel(&app, "POST", &format!("/internat/sorties/{sid}/annulation"), Some(&cpe.jeton), Some(json!({}))).await;
    assert_eq!(st, StatusCode::OK);
    // Départs du vendredi : réservés à `internat.gerer` ; liste de remise : `internat.appel`.
    let (st, _) = appel(&app, "POST", "/internat/departs/2030-04-19/preparation", Some(&prof.jeton), Some(json!({}))).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, r) = appel(&app, "GET", &format!("/internat/departs/{nuit}/remise"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{r}");
    let (st, _) = appel(&app, "GET", &format!("/internat/departs/{nuit}/remise"), Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Tiers autorisé à récupérer (suites de l'I3, M3) : poser et lire sont réservés au personnel de l'internat (403 pour
    // un professeur) ; la pose du CPE d'une personne inconnue est un refus de règle (4xx), jamais une panne.
    let pose = json!({
        "eleve_id": garcon, "personne_id": Uuid::new_v4(), "demande_par_personne_id": Uuid::new_v4(),
        "demande_ecrite_le": (nuit - Duration::days(1)).to_string(), "du": nuit.to_string(),
        "au": (nuit + Duration::days(10)).to_string(), "motif_texte": "Demande écrite d'un titulaire"
    });
    let (st, _) = appel(&app, "POST", "/internat/tiers", Some(&prof.jeton), Some(pose.clone())).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "GET", &format!("/internat/eleves/{garcon}/tiers"), Some(&prof.jeton), None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = appel(&app, "POST", "/internat/tiers", Some(&cpe.jeton), Some(pose)).await;
    assert!(st.is_client_error() && st != StatusCode::FORBIDDEN, "{st}");
    let (st, l) = appel(&app, "GET", &format!("/internat/eleves/{garcon}/tiers"), Some(&cpe.jeton), None).await;
    assert_eq!(st, StatusCode::OK, "{l}");
    assert_eq!(l.as_array().map(Vec::len), Some(0));
    // Un tiers inconnu : 404 pour l'opposition, l'arbitrage, la révocation et la reprise.
    let inconnu = Uuid::new_v4();
    for (route, corps) in [
        ("revocation", json!({})),
        ("opposition", json!({})),
        ("arbitrage", json!({"decision": "lever", "motif_texte": "x"})),
        ("reprise", json!({"demande_par_personne_id": Uuid::new_v4(), "motif_texte": "x"})),
    ] {
        let (st, _) = appel(&app, "POST", &format!("/internat/tiers/{inconnu}/{route}"), Some(&dir.jeton), Some(corps)).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{route}");
    }
}
