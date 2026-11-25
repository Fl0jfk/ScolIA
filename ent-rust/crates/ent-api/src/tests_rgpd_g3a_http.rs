//! Socle 27, lot G3a (HTTP) : les routes `/rgpd/*` du guichet des demandes de droits répondent 200, 201,
//! 400, 401, 403, 404 ou 409 comme le contrat le dit. Les règles métier sont dans `ent-db`
//! (`tests_rgpd_g3a.rs`) ; ici on vérifie **la surface** : session exigée, cloisonnement par l'adresse,
//! neutralité des refus, minimisation de la file.
//!
//! Base locale de labo (cast Harry Potter).

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn appel(
    app: &axum::Router,
    m: &str,
    uri: &str,
    entetes: &[(&str, &str)],
    jeton: Option<&str>,
    corps: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(m).uri(uri);
    for (k, v) in entetes {
        b = b.header(*k, *v);
    }
    if let Some(j) = jeton {
        b = b.header("authorization", format!("Bearer {j}"));
    }
    let req = match corps {
        Some(c) => b
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&c).unwrap()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let st = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (st, v)
}

/// Le parcours du guichet de bout en bout, par HTTP, et les refus attendus.
#[tokio::test]
async fn g3a_http_guichet_parcours_et_refus() {
    let Some(url) = super::tests_l3::url_locale() else {
        eprintln!("skip G3a HTTP: ENT_DATABASE_URL");
        return;
    };
    let ent = PgEnt::connect(&url).await.unwrap();
    ent.migrate().await.unwrap();
    let state = AppState::nouveau(ent);
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let etab = cast.etablissement_id;
    let app = app(state.clone());
    let (dir, adf, prof, parent) = (
        cast.direction.token.as_str(),
        cast.administratif.token.as_str(),
        cast.prof.token.as_str(),
        cast.parent_lea.token.as_str(),
    );
    let jour = state.ent.aujourdhui_etablissement(etab).await.unwrap();

    // Sans session : 401 sur chaque route du guichet (jamais 404 : la route existe).
    for (m, uri) in [
        ("GET", "/rgpd/demandes"),
        ("POST", "/rgpd/demandes"),
        ("GET", "/rgpd/guichet/referentiel"),
        ("GET", "/rgpd/mes-demandes"),
        ("POST", "/rgpd/mes-demandes"),
    ] {
        let (st, _) = appel(&app, m, uri, &[], None, Some(json!({}))).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {uri}");
    }

    // Un professeur et une famille n'approchent pas le guichet : 403.
    for jeton in [prof, parent] {
        let (st, _) = appel(&app, "GET", "/rgpd/demandes", &[], Some(jeton), None).await;
        assert_eq!(st, StatusCode::FORBIDDEN);
        let (st, _) = appel(
            &app,
            "GET",
            "/rgpd/guichet/referentiel",
            &[],
            Some(jeton),
            None,
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN);
    }

    // Référentiel du guichet : grille d'identité et modèles de réponse.
    let (st, r) = appel(
        &app,
        "GET",
        "/rgpd/guichet/referentiel",
        &[],
        Some(adf),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert!(r["grille_identite"]
        .as_array()
        .is_some_and(|a| !a.is_empty()));
    assert!(r["modeles_reponse"]
        .as_array()
        .is_some_and(|a| !a.is_empty()));
    assert_eq!(
        r["identite_suspend_delai"],
        json!(false),
        "défaut prudent : le délai court (§9.2)"
    );

    // Saisie d'un courrier reçu : 201.
    let (st, d) = appel(
        &app,
        "POST",
        "/rgpd/demandes",
        &[],
        Some(adf),
        Some(json!({
            "canal": "courrier",
            "recue_le": jour,
            "demandeur_nom": "Famille Granger",
            "demandeur_contact": "courrier postal",
            "qualite": "titulaire_autorite",
            "sujet_type": "eleve",
            "sujet_id": cast.eleve_lea_id,
            "droits": ["acces", "effacement"],
            "perimetre_texte": "tout le dossier"
        })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{d}");
    let id: Uuid = serde_json::from_value(d["id"].clone()).unwrap();
    assert_eq!(d["etat"], json!("identite_a_verifier"));
    assert_eq!(d["droits"].as_array().unwrap().len(), 2);

    // Canal inconnu, droit inconnu, date future : 400 (saisie invalide).
    for corps in [
        json!({"canal": "pigeon", "recue_le": jour, "demandeur_nom": "X Y", "qualite": "soi_meme",
               "sujet_type": "inconnu", "personne_concernee": "Z A", "droits": ["acces"]}),
        json!({"canal": "courrier", "recue_le": jour, "demandeur_nom": "X Y", "qualite": "soi_meme",
               "sujet_type": "inconnu", "personne_concernee": "Z A", "droits": ["inventé"]}),
        json!({"canal": "courrier", "recue_le": jour + chrono::Duration::days(2),
               "demandeur_nom": "X Y", "qualite": "soi_meme", "sujet_type": "inconnu",
               "personne_concernee": "Z A", "droits": ["acces"]}),
    ] {
        let (st, _) = appel(&app, "POST", "/rgpd/demandes", &[], Some(adf), Some(corps)).await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
    }

    // Une demande inexistante : 404 (jamais 403 : rien ne dit qu'elle existe ailleurs).
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/rgpd/demandes/{}", Uuid::new_v4()),
        &[],
        Some(dir),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Qualification motivée.
    let (st, d) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/qualification"),
        &[],
        Some(adf),
        Some(json!({ "qualification": "rgpd", "motif": "accès aux données de l'élève" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{d}");
    assert_eq!(d["qualification"], json!("rgpd"));

    // Vérification d'identité, puis instruction.
    let (st, d) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/identite"),
        &[],
        Some(adf),
        Some(json!({ "mode": "code_courriel", "reussie": true })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{d}");
    assert_eq!(d["etat"], json!("en_cours"));

    // Prolongation : le secrétariat ne prolonge pas (403), la direction oui (200).
    let max = jour.checked_add_months(chrono::Months::new(3)).unwrap();
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/prolongation"),
        &[],
        Some(adf),
        Some(json!({ "jusqu_au": max, "motif": "dossier volumineux" })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, d) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/prolongation"),
        &[],
        Some(dir),
        Some(json!({ "jusqu_au": max, "motif": "dossier volumineux, plusieurs services" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{d}");
    // Une seconde prolongation : 409.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/prolongation"),
        &[],
        Some(dir),
        Some(json!({ "jusqu_au": max, "motif": "encore" })),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);

    // Décisions : accorder (secrétariat), refuser (direction seulement).
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/decision"),
        &[],
        Some(adf),
        Some(json!({ "droit": "acces", "decision": "accorde", "motif_code": "art15" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let refus = json!({ "droit": "effacement", "decision": "refuse", "motif_code": "art17_3_b",
                        "motif_texte": "conservation imposée par un texte" });
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/decision"),
        &[],
        Some(adf),
        Some(refus.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN, "refuser est un arbitrage");
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/decision"),
        &[],
        Some(dir),
        Some(refus),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Clôture, puis toute écriture ultérieure : 409.
    let (st, d) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/cloture"),
        &[],
        Some(adf),
        Some(json!({ "reponse_resume": "extrait remis au guichet contre émargement" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{d}");
    assert_eq!(d["etat"], json!("close"));
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/qualification"),
        &[],
        Some(adf),
        Some(json!({ "qualification": "cada", "motif": "trop tard" })),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);

    // Réouverture : arbitrage.
    let (st, _) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/reouverture"),
        &[],
        Some(adf),
        Some(json!({ "motif": "réclamation" })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, d) = appel(
        &app,
        "POST",
        &format!("/rgpd/demandes/{id}/reouverture"),
        &[],
        Some(dir),
        Some(json!({ "motif": "réclamation du demandeur : extrait incomplet" })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{d}");
    assert_eq!(d["reouvertures"], json!(1));

    // La file : métadonnées seulement (R-RG32).
    let (st, t) = appel(&app, "GET", "/rgpd/demandes", &[], Some(dir), None).await;
    assert_eq!(st, StatusCode::OK, "{t}");
    assert!(t["lignes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["id"] == d["id"]));
    let brut = t.to_string();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let nom_eleve: String = sqlx::query_scalar("SELECT nom FROM ent_eleve WHERE id = $1")
        .bind(cast.eleve_lea_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!brut.contains(&nom_eleve), "aucun nom d'élève dans la file");
    assert!(
        !brut.contains("tout le dossier"),
        "aucun texte de périmètre"
    );
    // Le filtre `closes` est accepté.
    let (st, _) = appel(
        &app,
        "GET",
        "/rgpd/demandes?closes=true",
        &[],
        Some(dir),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
}

/// Dépôt et suivi côté demandeur connecté, par HTTP.
#[tokio::test]
async fn g3a_http_depot_et_suivi_du_demandeur() {
    let Some(url) = super::tests_l3::url_locale() else {
        eprintln!("skip G3a HTTP: ENT_DATABASE_URL");
        return;
    };
    let ent = PgEnt::connect(&url).await.unwrap();
    ent.migrate().await.unwrap();
    let state = AppState::nouveau(ent);
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let app = app(state.clone());
    let parent = cast.parent_lea.token.as_str();

    let (st, d) = appel(
        &app,
        "POST",
        "/rgpd/mes-demandes",
        &[],
        Some(parent),
        Some(json!({
            "qualite": "titulaire_autorite",
            "sujet_type": "eleve",
            "sujet_id": cast.eleve_lea_id,
            "droits": ["acces"]
        })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{d}");
    let id = d["id"].clone();

    // Suivi : la demande y est, avec une phrase d'avancement et sans détail interne.
    let (st, l) = appel(&app, "GET", "/rgpd/mes-demandes", &[], Some(parent), None).await;
    assert_eq!(st, StatusCode::OK, "{l}");
    let mienne = l
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == id)
        .expect("ma demande");
    assert!(mienne["avancement"].as_str().unwrap().contains("Reçue le"));
    assert!(mienne.get("journal").is_none(), "pas de journal interne");
    assert!(mienne.get("gestionnaire_personne_id").is_none());

    // Le parent ne lit pas la fiche du guichet de sa propre demande (403 : ce n'est pas son écran).
    let (st, _) = appel(
        &app,
        "GET",
        &format!("/rgpd/demandes/{}", id.as_str().unwrap()),
        &[],
        Some(parent),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);

    // Une qualité réservée au guichet : 400 (saisie invalide).
    let (st, _) = appel(
        &app,
        "POST",
        "/rgpd/mes-demandes",
        &[],
        Some(parent),
        Some(json!({
            "qualite": "tiers",
            "sujet_type": "eleve",
            "sujet_id": cast.eleve_lea_id,
            "droits": ["acces"]
        })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Le suivi d'un autre parent ne contient pas cette demande.
    let (st, l) = appel(
        &app,
        "GET",
        "/rgpd/mes-demandes",
        &[],
        Some(cast.parent_tom.token.as_str()),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(!l.as_array().unwrap().iter().any(|s| s["id"] == id));
}

/// RGP-142 et R-RG42 : la route publique rend **toujours** la même phrase, quelle que soit la personne
/// visée, et l'établissement vient de l'adresse (jamais du corps).
#[tokio::test]
async fn g3a_http_depot_sans_compte_reponse_invariable() {
    let Some(url) = super::tests_l3::url_locale() else {
        eprintln!("skip G3a HTTP: ENT_DATABASE_URL");
        return;
    };
    let ent = PgEnt::connect(&url).await.unwrap();
    ent.migrate().await.unwrap();
    let state = AppState::nouveau(ent);
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let etab = cast.etablissement_id;
    let hote = format!("rgpd-{}.ent.test", Uuid::new_v4().simple());
    state
        .ent
        .declarer_etablissement_plateforme(
            etab,
            &format!("g3a-{}", &Uuid::new_v4().simple().to_string()[..12]),
            "Établissement du guichet",
        )
        .await
        .expect("déclaration");
    state
        .ent
        .ajouter_domaine_plateforme(etab, &hote)
        .await
        .expect("domaine");
    let app = app(state.clone());
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let nom_eleve: String = sqlx::query_scalar("SELECT nom FROM ent_eleve WHERE id = $1")
        .bind(cast.eleve_lea_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    let h = [("host", hote.as_str())];
    let mut messages = Vec::new();
    for (nom, concernee) in [
        ("Parent sans compte", nom_eleve.as_str()),
        ("Quelqu'un d'autre", "Personne Introuvable"),
    ] {
        let (st, r) = appel(
            &app,
            "POST",
            "/rgpd/demandes/sans-compte",
            &h,
            None,
            Some(json!({
                "nom": nom,
                "contact": format!("{}@exemple.test", Uuid::new_v4().simple()),
                "qualite": "titulaire_autorite",
                "personne_concernee": concernee,
                "droits": ["acces"]
            })),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{r}");
        let m = r["message"].as_str().unwrap().to_string();
        assert!(!m.contains(&nom_eleve), "aucune confirmation : {m}");
        messages.push(m);
    }
    assert_eq!(
        messages[0], messages[1],
        "la réponse ne dit jamais si la personne est connue (R-RG42)"
    );

    // Sans adresse connue : 404 neutre (aucun établissement deviné depuis le corps).
    let (st, _) = appel(
        &app,
        "POST",
        "/rgpd/demandes/sans-compte",
        &[],
        None,
        Some(json!({
            "nom": "Sans adresse",
            "contact": "sans@exemple.test",
            "qualite": "ancien_eleve",
            "personne_concernee": "Sans adresse",
            "droits": ["acces"],
            "etablissement_id": etab
        })),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}
