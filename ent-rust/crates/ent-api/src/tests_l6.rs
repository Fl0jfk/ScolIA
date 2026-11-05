//! Lot L6 — en-têtes no-store, erreurs internes génériques, span de requête,
//! traçabilité des exports. Les tests base sont ignorés sans `ENT_DATABASE_URL`.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
#[cfg(feature = "labo")]
use serde_json::Value;
#[cfg(feature = "labo")]
use sqlx::Row;
use tower::ServiceExt;
use uuid::Uuid;

async fn texte(res: axum::response::Response) -> String {
    let b = res.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&b).to_string()
}

#[cfg(feature = "labo")]
async fn state_local() -> Option<AppState> {
    let url = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())?;
    let ent = PgEnt::connect(&url).await.ok()?;
    ent.migrate().await.ok()?;
    Some(AppState::nouveau(ent))
}

#[cfg(feature = "labo")]
async fn inscrire_et_connecter(app: &axum::Router, etab: Uuid, role: &str) -> String {
    let email = format!("l6-{}@ent.local", Uuid::new_v4());
    let reg = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"email":"{email}","password":"DevLocalPass1!","etablissement_id":"{etab}","role":"{role}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(reg.status() == StatusCode::CREATED || reg.status() == StatusCode::CONFLICT);
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
    let v: Value = serde_json::from_str(&texte(login).await).unwrap();
    v["token"].as_str().unwrap().to_string()
}

#[cfg(feature = "labo")]
#[tokio::test]
async fn no_store_sur_auth_me_et_en_tetes_complementaires() {
    let Some(state) = state_local().await else {
        eprintln!("skip L6: ENT_DATABASE_URL");
        return;
    };
    let app = app(state);
    let etab = Uuid::new_v4();
    let token = inscrire_et_connecter(&app, etab, "admin").await;
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/auth/me")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let h = res.headers();
    assert_eq!(h["cache-control"], "no-store");
    assert_eq!(h["pragma"], "no-cache");
    assert_eq!(h["referrer-policy"], "same-origin");
    assert!(h["permissions-policy"].to_str().unwrap().contains("camera=()"));
    assert!(h.get("x-request-id").is_some());

    // /health aussi ; /static garde son cache (pas de no-store).
    let health = app
        .clone()
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(health.headers()["cache-control"], "no-store");
    let stat = app
        .oneshot(
            Request::builder()
                .uri("/static/manifest.webmanifest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stat.status(), StatusCode::OK);
    assert!(stat.headers().get("cache-control").is_none());
}

#[tokio::test]
async fn reponse_500_remplacee_par_un_message_generique_avec_reference() {
    use axum::routing::get;
    let routeur: axum::Router = axum::Router::new()
        .route(
            "/boom",
            get(|| async {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "error returned from database: relation \"ent_secret\" does not exist",
                )
            }),
        )
        .layer(axum::middleware::from_fn(http_garde::en_tetes_et_erreurs));
    let res = routeur
        .oneshot(
            Request::builder()
                .uri("/boom")
                .header("x-request-id", "ref-abc-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(res.headers()["cache-control"], "no-store");
    let corps = texte(res).await;
    assert_eq!(corps, "Erreur interne. Référence : ref-abc-123");
}

#[tokio::test]
async fn panne_base_donne_500_generique_sans_detail_sql() {
    // Pool « paresseux » vers un port fermé : toute requête échoue côté
    // infrastructure, sans toucher à aucune vraie base.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy("postgres://u:p@127.0.0.1:1/aucune")
        .unwrap();
    let app = app(AppState::nouveau(PgEnt::from_pool(pool)));
    let res = app
        .oneshot(
            Request::builder()
                .uri("/auth/me")
                .header("authorization", "Bearer jeton-quelconque")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "une panne base ne doit être ni 401 ni 404"
    );
    let id = res.headers()["x-request-id"].to_str().unwrap().to_string();
    let corps = texte(res).await;
    assert_eq!(corps, format!("Erreur interne. Référence : {id}"));
    for interdit in ["sqlx", "Postgres", "127.0.0.1", "pool", "connect", "database"] {
        assert!(!corps.contains(interdit), "fuite « {interdit} » : {corps}");
    }
}

#[test]
fn map_db_classe_les_pannes_en_500_avant_les_sous_chaines() {
    use ent_db::EntDbError;
    // Le texte contient « note », « psy », « document » : mots qui, avant
    // correction, transformaient la panne en 400.
    let panne = EntDbError::Sqlx(sqlx::Error::Protocol(
        "table note psy document inaccessible".into(),
    ));
    let (code, msg) = routes::map_db(panne);
    assert_eq!(code, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!msg.contains("note"));

    let (code, _) = routes::map_db(EntDbError::Domain(domain_eleve::DomainError::Infrastructure));
    assert_eq!(code, StatusCode::INTERNAL_SERVER_ERROR);

    // Les 4xx métier restent inchangés.
    let (code, _) = routes::map_db(EntDbError::Domain(domain_eleve::DomainError::NotFound));
    assert_eq!(code, StatusCode::NOT_FOUND);
    let (code, _) = routes::map_db(EntDbError::Forbidden);
    assert_eq!(code, StatusCode::FORBIDDEN);
}

#[cfg(feature = "labo")]
#[tokio::test]
async fn export_ogec_laisse_une_trace_dans_le_journal() {
    let Some(state) = state_local().await else {
        eprintln!("skip L6: ENT_DATABASE_URL");
        return;
    };
    let url = std::env::var("ENT_DATABASE_URL").unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let app = app(state);
    let etab = Uuid::new_v4();
    let token = inscrire_et_connecter(&app, etab, "admin").await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/facturation/export/ogec?debut=2026-09-01&fin=2026-09-30")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "export OGEC");
    let n: i64 = sqlx::query(
        "SELECT count(*) AS n FROM ent_journal_modif \
         WHERE etablissement_id = $1 AND domaine = 'export' AND action = 'ogec'",
    )
    .bind(etab)
    .fetch_one(&pool)
    .await
    .unwrap()
    .get("n");
    assert_eq!(n, 1);
}

#[test]
fn debug_des_corps_de_connexion_masque_mot_de_passe_et_jeton() {
    let b: routes::LoginBody = serde_json::from_str(&format!(
        r#"{{"email":"x@ent.local","password":"MotDePasseSecret1!","etablissement_id":"{}"}}"#,
        Uuid::new_v4()
    ))
    .unwrap();
    let d = format!("{b:?}");
    assert!(!d.contains("MotDePasseSecret1!") && !d.contains("x@ent.local"));
    assert!(d.contains("[masqué]"));
}

#[cfg(feature = "labo")]
#[test]
fn debug_du_corps_d_inscription_masque_le_mot_de_passe() {
    let r: routes::RegisterBody = serde_json::from_str(&format!(
        r#"{{"email":"x@ent.local","password":"MotDePasseSecret1!","etablissement_id":"{}"}}"#,
        Uuid::new_v4()
    ))
    .unwrap();
    assert!(!format!("{r:?}").contains("MotDePasseSecret1!"));
}


fn app_base_morte() -> axum::Router {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy("postgres://u:p@127.0.0.1:1/aucune")
        .unwrap();
    app(AppState::nouveau(PgEnt::from_pool(pool)))
}

#[tokio::test]
async fn x_request_id_douteux_est_remplace_par_un_uuid() {
    for douteux in ["a b<script>", "x".repeat(65).as_str(), "id\u{e9}"] {
        let res = app_base_morte()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header(
                        "x-request-id",
                        axum::http::HeaderValue::from_bytes(douteux.as_bytes()).unwrap(),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let id = res.headers()["x-request-id"].to_str().unwrap().to_string();
        assert!(Uuid::parse_str(&id).is_ok(), "identifiant non régénéré : {id}");
    }
    // Un identifiant sûr est conservé.
    let res = app_base_morte()
        .oneshot(
            Request::builder()
                .uri("/health")
                .header("x-request-id", "ref-ABC-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.headers()["x-request-id"], "ref-ABC-123");
    assert!(http_garde::request_id_sur("ref-ABC-123"));
    assert!(!http_garde::request_id_sur(""));
}

#[tokio::test]
async fn ready_sans_base_renvoie_un_message_fixe() {
    let res = app_base_morte()
        .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(texte(res).await, "base indisponible");
}

#[test]
fn map_oidc_et_map_pra_ne_renvoient_pas_de_detail_technique() {
    use ent_db::{EntDbError, OidcError};
    let (code, msg) = routes::map_oidc(EntDbError::Oidc(OidcError::Network(
        "http://10.0.0.7:8443/.well-known tcp connect error".into(),
    )));
    assert_eq!(code, StatusCode::BAD_GATEWAY);
    assert!(!msg.contains("10.0.0.7") && !msg.contains("tcp"), "{msg}");
    let (code, msg) = routes::map_oidc(EntDbError::Oidc(OidcError::Discovery(
        "champ secret interne".into(),
    )));
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert!(!msg.contains("secret"), "{msg}");
    let (code, msg) = routes::map_oidc(EntDbError::Sqlx(sqlx::Error::Protocol(
        "relation ent_x inaccessible".into(),
    )));
    assert_eq!(code, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!msg.contains("ent_x"), "{msg}");

    let (code, msg) = routes::map_pra(EntDbError::Domain(
        domain_eleve::DomainError::Infrastructure,
    ));
    assert_eq!(code, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(msg, "Erreur interne.");
    let (code, _) = routes::map_pra(EntDbError::Domain(domain_eleve::DomainError::Reessayer));
    assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE);
    let (code, msg) = routes::map_db(EntDbError::Event(ent_db::EventBusError::Infrastructure));
    assert_eq!(code, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(msg, "Erreur interne.");
}

#[test]
fn erreurs_de_validation_metier_restent_des_4xx() {
    use domain_eleve::DomainError as D;
    use ent_db::EntDbError as E;
    // Pièce PAI, photo, pièce jointe : types / tailles refusés = 400 (pas 500).
    for d in [D::PaiPieceInvalide, D::TrombiInvalide, D::ValeurInvalide] {
        let (code, msg) = routes::map_db(E::Domain(d));
        assert_eq!(code, StatusCode::BAD_REQUEST, "{msg}");
        assert!(!msg.is_empty() && msg != "Erreur interne.");
    }
    let (code, _) = routes::map_db(E::Object(ent_db::ObjectStoreError::InvalidKey));
    assert_eq!(code, StatusCode::BAD_REQUEST);
    let (code, msg) = routes::map_db(E::FichierAbsent);
    assert_eq!(code, StatusCode::NOT_FOUND);
    assert_eq!(msg, "fichier absent pour ce document");
    // Liens courts sans secret de signature valide : fail-closed, 503 sans détail.
    let (code, msg) = routes::map_db(E::LiensIndisponibles);
    assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE);
    assert!(!msg.contains("SIGNING_KEY"), "{msg}");
    // Les vraies pannes restent des 500 génériques.
    let (code, msg) = routes::map_db(E::Object(ent_db::ObjectStoreError::Io("/var/x: disk".into())));
    assert_eq!(code, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(msg, "Erreur interne.");
}
