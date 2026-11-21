//! D-017 R2 — le middleware de droits (HTTP, base locale) : un rôle paramétré
//! ouvre exactement la route de gestion visée, le retrait la referme à la
//! requête suivante, et une session sans rôle garde les droits de son profil.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use ent_auth::{Action, Portee};
use ent_db::{Droit, RoleSaisie};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const MDP: &str = "DevLocalPass1!";

async fn appel(app: &axum::Router, methode: &str, uri: &str, jeton: &str) -> StatusCode {
    let req = Request::builder()
        .method(methode)
        .uri(uri)
        .header("authorization", format!("Bearer {jeton}"))
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(req).await.unwrap().status()
}

async fn inscrire(app: &axum::Router, etab: Uuid, role: &str) -> (String, Uuid) {
    let email = format!("r2-{role}-{}@ent.local", Uuid::new_v4().simple());
    let post = |uri: &str, corps: Value| {
        let req = Request::builder()
            .method("POST")
            .uri(uri.to_string())
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&corps).unwrap()))
            .unwrap();
        app.clone().oneshot(req)
    };
    let r = post(
        "/auth/register",
        json!({"email": email, "password": MDP, "etablissement_id": etab, "role": role}),
    )
    .await
    .unwrap();
    assert_eq!(r.status(), StatusCode::CREATED, "register {role}");
    let r = post(
        "/auth/login",
        json!({"email": email, "password": MDP, "etablissement_id": etab}),
    )
    .await
    .unwrap();
    assert_eq!(r.status(), StatusCode::OK, "login {role}");
    let bytes = http_body_util::BodyExt::collect(r.into_body())
        .await
        .unwrap()
        .to_bytes();
    let b: Value = serde_json::from_slice(&bytes).unwrap();
    (
        b["token"].as_str().unwrap().to_string(),
        Uuid::parse_str(b["user_id"].as_str().unwrap()).unwrap(),
    )
}

#[tokio::test]
async fn r2_middleware_ouvre_et_referme_la_route_de_gestion() {
    let Some(url) = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
    else {
        eprintln!("skip R2: ENT_DATABASE_URL");
        return;
    };
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let state = AppState::nouveau(ent);
    let ent = state.ent.clone();
    let app = app(state);
    let etab = Uuid::new_v4();
    let (_, admin) = inscrire(&app, etab, "admin").await;
    let (compta, compta_id) = inscrire(&app, etab, "compta").await;
    let (adm, _) = inscrire(&app, etab, "administratif").await;

    // Profil seul : la comptable ne lit pas le calendrier EDT ; l'administratif oui.
    assert_eq!(
        appel(&app, "GET", "/edt/calendrier-ab", &compta).await,
        StatusCode::FORBIDDEN
    );
    assert_ne!(
        appel(&app, "GET", "/edt/calendrier-ab", &adm).await,
        StatusCode::FORBIDDEN
    );

    let role = ent
        .creer_role_for_actor(
            etab,
            admin,
            "admin",
            RoleSaisie {
                nom: "Planification".into(),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::EdtGerer,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, admin, "admin", role.id, compta_id, None)
        .await
        .unwrap();
    // Même jeton, requête suivante : la route de gestion est ouverte.
    let st = appel(&app, "GET", "/edt/calendrier-ab", &compta).await;
    assert!(
        st != StatusCode::FORBIDDEN && st != StatusCode::UNAUTHORIZED,
        "{st}"
    );
    // … sans ouvrir une autre gestion (annonces) que le rôle ne donne pas.
    let req = Request::builder()
        .method("POST")
        .uri("/annonces")
        .header("authorization", format!("Bearer {compta}"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"titre": "x", "corps": "y", "audience": "parents"}).to_string(),
        ))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(req).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );

    ent.retirer_role_for_actor(etab, admin, "admin", role.id, compta_id)
        .await
        .unwrap();
    assert_eq!(
        appel(&app, "GET", "/edt/calendrier-ab", &compta).await,
        StatusCode::FORBIDDEN
    );
    ent.supprimer_role_for_actor(etab, admin, "admin", role.id)
        .await
        .unwrap();
}
