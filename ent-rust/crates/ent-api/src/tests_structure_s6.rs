//! Socle 5, lot S6 (HTTP) : routes `/structure/groupes*`, `/structure/eleves/classe`, `/structure/affectations*`,
//! `/structure/a-affecter` — session obligatoire, matrice de droits, aperçu par défaut des commandes ★,
//! cloisonnement entre établissements, conflits et refus. Base locale de labo.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn appel(
    app: &axum::Router,
    m: &str,
    uri: &str,
    jeton: Option<&str>,
    corps: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(m).uri(uri);
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

#[tokio::test]
async fn routes_groupes_droits_apercu_par_defaut_cloisonnement_et_conflits() {
    let Some(url) = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
    else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = PgEnt::connect(&url).await.unwrap();
    ent.migrate().await.unwrap();
    let state = AppState::nouveau(ent);
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let autre = state.ent.seed_labo_cast().await.expect("cast B");
    let etab = cast.etablissement_id;
    let annee = state
        .ent
        .referentiel_annee_courante_assuree(etab)
        .await
        .unwrap();
    let sfx = uuid::Uuid::new_v4().simple().to_string()[..6].to_uppercase();
    let classe = state
        .ent
        .referentiel_creer_classe(
            "administratif",
            etab,
            Some(annee.id),
            &format!("G4{sfx}"),
            None,
            Some("4EME"),
        )
        .await
        .unwrap();
    let classe2 = state
        .ent
        .referentiel_creer_classe(
            "administratif",
            etab,
            Some(annee.id),
            &format!("G5{sfx}"),
            None,
            Some("5EME"),
        )
        .await
        .unwrap();
    let eleve = state
        .ent
        .create_eleve(domain_eleve::CreateEleve {
            etablissement_id: etab,
            nom: format!("Http{sfx}"),
            prenom: "S6".into(),
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
    let app = app(state.clone());
    let (dir, adm, prof, parent) = (
        cast.direction.token.as_str(),
        cast.administratif.token.as_str(),
        cast.prof.token.as_str(),
        cast.parent_lea.token.as_str(),
    );
    let b_dir = autre.direction.token.as_str();

    // Session obligatoire sur toutes les routes du lot.
    for (m, u) in [
        ("GET", "/structure/groupes"),
        ("POST", "/structure/groupes"),
        ("POST", "/structure/groupes/generer"),
        ("POST", "/structure/groupes/demi-groupes"),
        ("POST", "/structure/groupes/activites"),
        (
            "GET",
            "/structure/groupes/00000000-0000-4000-8000-000000000001",
        ),
        (
            "PUT",
            "/structure/groupes/00000000-0000-4000-8000-000000000001",
        ),
        (
            "POST",
            "/structure/groupes/00000000-0000-4000-8000-000000000001/fermer",
        ),
        (
            "POST",
            "/structure/groupes/00000000-0000-4000-8000-000000000001/composition",
        ),
        (
            "GET",
            "/structure/groupes/00000000-0000-4000-8000-000000000001/propositions",
        ),
        (
            "POST",
            "/structure/groupes/00000000-0000-4000-8000-000000000001/regle",
        ),
        (
            "POST",
            "/structure/groupes/00000000-0000-4000-8000-000000000001/dedoubler",
        ),
        ("POST", "/structure/eleves/classe"),
        ("POST", "/structure/affectations/lot"),
        ("POST", "/structure/affectations/correction"),
        ("GET", "/structure/a-affecter"),
    ] {
        let (st, _) = appel(&app, m, u, None, None).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED, "{m} {u}");
    }
    // Lecture : personnel oui, famille non. Écriture : gestionnaires seulement.
    assert_eq!(
        appel(&app, "GET", "/structure/groupes", Some(prof), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        appel(&app, "GET", "/structure/groupes", Some(parent), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    let corps =
        json!({"code": format!("G{sfx}"), "nature": "enseignement", "classes": [classe.id]});
    assert_eq!(
        appel(
            &app,
            "POST",
            "/structure/groupes",
            Some(prof),
            Some(corps.clone())
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Valeurs invalides : 400 / 422.
    for mauvais in [
        json!({"code": "", "nature": "enseignement"}),
        json!({"code": "X", "nature": "inconnue"}),
        json!({"nature": "enseignement"}),
    ] {
        let (st, _) = appel(&app, "POST", "/structure/groupes", Some(dir), Some(mauvais)).await;
        assert!(
            st == StatusCode::BAD_REQUEST || st == StatusCode::UNPROCESSABLE_ENTITY,
            "{st}"
        );
    }
    // Création (201), même code : 409.
    let (st, g) = appel(
        &app,
        "POST",
        "/structure/groupes",
        Some(dir),
        Some(corps.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{g}");
    assert_eq!(g["nature"], json!("enseignement"));
    let id = g["id"].as_str().unwrap().to_string();
    assert_eq!(
        appel(&app, "POST", "/structure/groupes", Some(adm), Some(corps))
            .await
            .0,
        StatusCode::CONFLICT
    );
    // Composition : le professeur ne compose pas ; l'administratif oui ; l'aperçu n'écrit rien.
    let compo = json!({"entrants": [eleve.id], "sortants": []});
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/structure/groupes/{id}/composition"),
            Some(prof),
            Some(compo.clone())
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (st, r) = appel(
        &app,
        "POST",
        &format!("/structure/groupes/{id}/composition"),
        Some(adm),
        Some(json!({"entrants": [eleve.id], "sortants": [], "apercu": true})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert_eq!(r["applique"], json!(false));
    let (_, d) = appel(
        &app,
        "GET",
        &format!("/structure/groupes/{id}"),
        Some(adm),
        None,
    )
    .await;
    assert_eq!(
        d["membres"].as_array().unwrap().len(),
        0,
        "l'aperçu n'a rien écrit"
    );
    let (st, r) = appel(
        &app,
        "POST",
        &format!("/structure/groupes/{id}/composition"),
        Some(adm),
        Some(compo),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert_eq!(r["applique"], json!(true));
    let (_, d) = appel(
        &app,
        "GET",
        &format!("/structure/groupes/{id}"),
        Some(adm),
        None,
    )
    .await;
    assert_eq!(d["membres"].as_array().unwrap().len(), 1);
    // La composition (noms d'élèves) n'est pas lisible par un professeur simple.
    assert_eq!(
        appel(
            &app,
            "GET",
            &format!("/structure/groupes/{id}"),
            Some(prof),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    // Changement de classe : aperçu par défaut (rien n'est écrit) ; `apercu:false` écrit.
    let (st, ap) = appel(
        &app,
        "POST",
        "/structure/eleves/classe",
        Some(adm),
        Some(json!({"eleve_id": eleve.id, "classe_id": classe2.id})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{ap}");
    assert_eq!(ap["applique"], json!(false), "aperçu par défaut");
    let (st, ap) = appel(
        &app,
        "POST",
        "/structure/eleves/classe",
        Some(adm),
        Some(json!({"eleve_id": eleve.id, "classe_id": classe2.id, "apercu": false})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{ap}");
    assert_eq!(ap["applique"], json!(true));
    // Lot et file de travail.
    let (st, l) = appel(
        &app,
        "POST",
        "/structure/affectations/lot",
        Some(adm),
        Some(json!({"classe_id": classe.id, "eleves": [eleve.id]})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{l}");
    assert_eq!(l["applique"], json!(false));
    assert!(l["refuses"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["cause"] == json!("dans_une_autre_classe")));
    assert_eq!(
        appel(&app, "GET", "/structure/a-affecter", Some(prof), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        appel(&app, "GET", "/structure/a-affecter", Some(parent), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    // Cloisonnement : l'autre établissement ne voit, ne modifie, ne ferme, ne compose rien de ce groupe.
    for (m, u, c) in [
        ("GET", format!("/structure/groupes/{id}"), None),
        (
            "PUT",
            format!("/structure/groupes/{id}"),
            Some(json!({"libelle": "Piraté"})),
        ),
        (
            "POST",
            format!("/structure/groupes/{id}/fermer"),
            Some(json!({})),
        ),
        (
            "POST",
            format!("/structure/groupes/{id}/composition"),
            Some(json!({"entrants": [eleve.id], "sortants": []})),
        ),
        ("GET", format!("/structure/groupes/{id}/propositions"), None),
        (
            "POST",
            format!("/structure/groupes/{id}/dedoubler"),
            Some(json!({"repartition": "alterne", "apercu": false})),
        ),
        (
            "POST",
            "/structure/eleves/classe".to_string(),
            Some(json!({"eleve_id": eleve.id, "classe_id": classe2.id, "apercu": false})),
        ),
    ] {
        let (st, _) = appel(&app, m, &u, Some(b_dir), c).await;
        assert!(
            st == StatusCode::NOT_FOUND || st == StatusCode::FORBIDDEN,
            "{m} {u} → {st}"
        );
    }
    // Fermeture (dernier jour inclus) puis refermeture : 409.
    let (st, f) = appel(
        &app,
        "POST",
        &format!("/structure/groupes/{id}/fermer"),
        Some(dir),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{f}");
    assert_eq!(
        appel(
            &app,
            "POST",
            &format!("/structure/groupes/{id}/fermer"),
            Some(dir),
            Some(json!({}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
}
