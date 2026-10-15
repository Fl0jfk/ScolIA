//! API HTTP du noyau ENT — faces qui **piochent** les tables centrales.
//! Auth 100 % Rust (sessions Postgres) + tenant de session sur chaque route métier.

mod auth;
mod dto;
mod routes;
mod ui;

pub use auth::EntSession;

use axum::Router;
use ent_db::PgEnt;
use std::sync::Arc;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct AppState {
    pub ent: Arc<PgEnt>,
}

pub fn app(state: AppState) -> Router {
    let fonts = ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/static"));
    Router::new()
        .nest_service("/static", fonts)
        .merge(routes::router())
        .with_state(state)
}

pub async fn boot_from_env() -> Result<(AppState, String), String> {
    let url = std::env::var("ENT_DATABASE_URL")
        .map_err(|_| "ENT_DATABASE_URL requis (postgres://…@127.0.0.1/…)".to_string())?;
    let mut ent = PgEnt::connect(&url)
        .await
        .map_err(|e| format!("connect: {e}"))?;
    ent.migrate().await.map_err(|e| format!("migrate: {e}"))?;
    ent.install_brain_p0();
    let bind = std::env::var("ENT_API_BIND").unwrap_or_else(|_| "127.0.0.1:8088".into());
    Ok((
        AppState {
            ent: Arc::new(ent),
        },
        bind,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::Utc;
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::ServiceExt;
    use uuid::Uuid;

    fn local_url() -> Option<String> {
        std::env::var("ENT_DATABASE_URL")
            .ok()
            .filter(|u| !u.trim().is_empty())
    }

    async fn state_local() -> Option<AppState> {
        let url = local_url()?;
        let mut ent = PgEnt::connect(&url).await.ok()?;
        ent.migrate().await.ok()?;
        ent.install_brain_p0();
        Some(AppState {
            ent: Arc::new(ent),
        })
    }

    async fn json_body(res: axum::response::Response) -> Value {
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap_or(json!({}))
    }

    async fn register_and_login(
        app: &axum::Router,
        etab: Uuid,
        email: &str,
    ) -> String {
        let reg = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"email":"{email}","password":"DevLocalPass1!","etablissement_id":"{etab}","role":"admin"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            reg.status() == StatusCode::CREATED || reg.status() == StatusCode::CONFLICT,
            "register {}",
            reg.status()
        );

        let login = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"email":"{email}","password":"DevLocalPass1!","etablissement_id":"{etab}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(login.status(), StatusCode::OK);
        let body = json_body(login).await;
        body["token"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn health_public() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let app = app(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn eleves_requires_auth() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let app = app(state);
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/eleves")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"nom":"A","prenom":"B"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn session_pg_tenant_isolation() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let etab = Uuid::new_v4();
        let other = Uuid::new_v4();
        let email = format!("staff-{}@ent.local", Uuid::new_v4());
        let app = app(state);

        let token = register_and_login(&app, etab, &email).await;
        let bearer = format!("Bearer {token}");

        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/eleves")
                    .header("authorization", &bearer)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"nom":"Api","prenom":"Test","classe":"4B"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::CREATED);
        let created = json_body(create).await;
        let id = created["id"].as_str().unwrap();

        let get_ok = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/eleves/{id}"))
                    .header("authorization", &bearer)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_ok.status(), StatusCode::OK);

        // Autre établissement : inscription + login → ne voit pas l'élève
        let email2 = format!("intrus-{}@ent.local", Uuid::new_v4());
        let token2 = register_and_login(&app, other, &email2).await;
        let get_other = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/eleves/{id}"))
                    .header("authorization", format!("Bearer {token2}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_other.status(), StatusCode::NOT_FOUND);

        let jour = Utc::now().date_naive();
        let abs = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/eleves/{id}/absences"))
                    .header("authorization", &bearer)
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"debut":"{jour}","fin":"{jour}","motif":"sejour"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(abs.status(), StatusCode::CREATED);

        let me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/auth/me")
                    .header("authorization", &bearer)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(me.status(), StatusCode::OK);
        let me_body = json_body(me).await;
        assert_eq!(me_body["etablissement_id"], etab.to_string());

        // Dates d'effet : cutover + effectif daté
        let cut = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/eleves/{id}/classe"))
                    .header("authorization", &bearer)
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"nouvelle_classe":"5B","date_effet":"{jour}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cut.status(), StatusCode::OK);

        let effectif = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/classes/5B/effectif?jour={jour}"))
                    .header("authorization", &bearer)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(effectif.status(), StatusCode::OK);
        let eff = json_body(effectif).await;
        assert!(eff["eleves"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["id"] == id));

        let periodes = app
            .oneshot(
                Request::builder()
                    .uri(format!("/eleves/{id}/periodes"))
                    .header("authorization", &bearer)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(periodes.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn api_multi_personas_matiere_et_notes() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let cast = state.ent.seed_labo_cast().await.expect("cast");
        let app = app(state);
        let jour = Utc::now().date_naive();

        // Compta ne crée pas de matière
        let forbid = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/matieres")
                    .header("authorization", format!("Bearer {}", cast.compta.token))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"code":"MATH","libelle":"Mathématiques"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(forbid.status(), StatusCode::FORBIDDEN);

        // Administratif crée Maths
        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/matieres")
                    .header(
                        "authorization",
                        format!("Bearer {}", cast.administratif.token),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"code":"MATH","libelle":"Mathématiques","source_nomenclature":"SIECLE:MATH"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);
        let matiere = json_body(created).await;
        let mid = matiere["id"].as_str().unwrap();

        // Prof note Léa
        let note = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/eleves/{}/notes", cast.eleve_lea_id))
                    .header("authorization", format!("Bearer {}", cast.prof.token))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"matiere_id":"{mid}","valeur":14.0,"date_note":"{jour}","libelle":"DS1"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(note.status(), StatusCode::CREATED);

        // Parent Léa lit · Parent Tom (autre enfant) interdit
        let ok_lea = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/eleves/{}/notes", cast.eleve_lea_id))
                    .header(
                        "authorization",
                        format!("Bearer {}", cast.parent_lea.token),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok_lea.status(), StatusCode::OK);

        let no_tom = app
            .oneshot(
                Request::builder()
                    .uri(format!("/eleves/{}/notes", cast.eleve_lea_id))
                    .header(
                        "authorization",
                        format!("Bearer {}", cast.parent_tom.token),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(no_tom.status(), StatusCode::FORBIDDEN);
    }
}
