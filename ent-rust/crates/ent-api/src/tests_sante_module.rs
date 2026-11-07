//! Module santé désactivé par défaut (règle produit : pas de donnée de santé sans hébergement
//! HDS) : refus côté serveur tant que l'établissement ne l'a pas activé, activation réservée
//! aux administrateurs, isolation entre établissements (HTTP, base locale de labo).

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn etat() -> Option<AppState> {
    let url = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())?;
    let ent = PgEnt::connect(&url).await.ok()?;
    ent.migrate().await.ok()?;
    Some(AppState::nouveau(ent))
}

async fn appel(
    app: &axum::Router,
    methode: &str,
    uri: &str,
    jeton: Option<&str>,
    corps: Option<Value>,
) -> (StatusCode, String) {
    let mut b = Request::builder().method(methode).uri(uri);
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
    (st, String::from_utf8_lossy(&bytes).into_owned())
}

fn refuse_module(st: StatusCode, corps: &str) -> bool {
    st == StatusCode::FORBIDDEN && corps.contains("module santé est désactivé")
}

#[test]
fn chemins_de_sante_reconnus() {
    use crate::sante::chemin_de_sante;
    let id = "6f1c3c80-0000-4000-8000-000000000001";
    for p in [
        "/sante/passages".to_string(),
        "/sante/catalogue".into(),
        "/sante/protocoles/".into(),
        "/accompagnement/mes-amenagements".into(),
        "/infirmerie/passages".into(),
        "/pai/pieces/abc".into(),
        format!("/eleves/{id}/fiche-sante"),
        format!("/eleves/{id}/fiche-sante.pdf"),
        format!("/eleves/{id}/pai"),
        format!("/eleves/{id}/pai/signer"),
        format!("/eleves/{id}/pai-complet"),
        format!("/eleves/{id}/fiche-urgence"),
        format!("/eleves/{id}/accompagnement"),
        format!("/eleves/{id}/dispositifs"),
        format!("/eleves/{id}/aesh"),
        format!("/eleves/{id}/reunions"),
        format!("/eleves/{id}/amenagements"),
        format!("/eleves/{id}/contraintes"),
    ] {
        assert!(chemin_de_sante(&p), "{p} doit être gardé");
    }
    for p in [
        "/sante/module",
        "/sante/module/",
        "/health",
        "/eleves",
        "/appel/ouvrir",
        &format!("/eleves/{id}"),
        &format!("/eleves/{id}/notes"),
        &format!("/eleves/{id}/dossier"),
    ] {
        assert!(!chemin_de_sante(p), "{p} ne doit pas être gardé");
    }
}

#[tokio::test]
async fn module_sante_ferme_par_defaut_active_par_un_administrateur_et_isole() {
    let Some(state) = etat().await else {
        eprintln!("skip module santé HTTP: ENT_DATABASE_URL");
        return;
    };
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let autre = state.ent.seed_labo_cast().await.expect("cast B");
    let app = app(state.clone());
    let dir = cast.direction.token.as_str();
    let inf = cast.infirmieres[0].token.as_str();
    let prof = cast.prof.token.as_str();
    let eleve = cast.eleve_lea_id;

    // Sans session : jamais de donnée (401), le module ne change rien.
    let (st, _) = appel(&app, "GET", "/sante/passages", None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // Fermé par défaut : toutes les routes de santé, anciennes et nouvelles, refusent (403).
    assert_eq!(state.ent.module_sante_actif(cast.etablissement_id).await.unwrap(), false);
    let gardees = [
        ("GET", "/sante/passages".to_string()),
        ("GET", "/sante/catalogue".to_string()),
        ("GET", "/infirmerie/passages".to_string()),
        ("GET", format!("/eleves/{eleve}/fiche-sante")),
        ("GET", format!("/eleves/{eleve}/pai")),
        ("GET", format!("/eleves/{eleve}/pai-complet")),
        ("GET", format!("/eleves/{eleve}/fiche-urgence")),
        ("GET", format!("/eleves/{eleve}/accompagnement")),
        ("GET", "/accompagnement/mes-amenagements".to_string()),
    ];
    for jeton in [dir, inf, prof, cast.parent_lea.token.as_str()] {
        for (methode, chemin) in &gardees {
            let (st, corps) = appel(&app, methode, chemin, Some(jeton), None).await;
            assert!(
                refuse_module(st, &corps),
                "{methode} {chemin} : attendu 403 module désactivé, reçu {st} {corps}"
            );
        }
    }

    // L'état se lit sans être activé ; personne ne peut l'activer sans le droit d'administrer.
    let (st, corps) = appel(&app, "GET", "/sante/module", Some(prof), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(serde_json::from_str::<Value>(&corps).unwrap(), json!({"actif": false}));
    for jeton in [prof, inf, cast.parent_lea.token.as_str()] {
        let (st, _) = appel(&app, "PUT", "/sante/module", Some(jeton), Some(json!({"actif": true}))).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "activation réservée aux administrateurs");
    }
    assert_eq!(state.ent.module_sante_actif(cast.etablissement_id).await.unwrap(), false);

    // Activation par la direction : les routes répondent normalement (plus le refus du module).
    let (st, corps) = appel(&app, "PUT", "/sante/module", Some(dir), Some(json!({"actif": true}))).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let (st, corps) = appel(&app, "GET", "/sante/passages", Some(inf), None).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let (st, corps) = appel(&app, "GET", "/sante/catalogue", Some(prof), None).await;
    assert_eq!(st, StatusCode::OK, "{corps}");
    let (st, corps) = appel(&app, "GET", "/infirmerie/passages", Some(inf), None).await;
    assert!(!refuse_module(st, &corps), "{st} {corps}");

    // Isolation : l'autre établissement reste fermé.
    let (st, corps) = appel(&app, "GET", "/sante/passages", Some(autre.direction.token.as_str()), None).await;
    assert!(refuse_module(st, &corps), "{st} {corps}");
    assert_eq!(state.ent.module_sante_actif(autre.etablissement_id).await.unwrap(), false);

    // Désactivation : retour immédiat au refus.
    let (st, _) = appel(&app, "PUT", "/sante/module", Some(dir), Some(json!({"actif": false}))).await;
    assert_eq!(st, StatusCode::OK);
    let (st, corps) = appel(&app, "GET", "/sante/passages", Some(inf), None).await;
    assert!(refuse_module(st, &corps), "{st} {corps}");
    let (st, corps) = appel(&app, "GET", &format!("/eleves/{eleve}/pai"), Some(dir), None).await;
    assert!(refuse_module(st, &corps), "{st} {corps}");
}
