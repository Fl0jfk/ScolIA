//! API HTTP du noyau ENT — faces qui **piochent** les tables centrales.
//! Auth 100 % Rust (sessions Postgres) + tenant de session sur chaque route métier.

#[macro_use]
mod masque;
pub mod anti_abus;
mod auth;
pub mod client_ip;
mod dossier;
mod personnels;
mod personnels_services;
mod dto;
pub mod http_garde;
mod ical;
mod finances;
mod inscriptions;
pub mod obs;
mod referentiel;
mod cahier;
mod referentiel_s2;
mod structure;
mod structure_calendrier;
mod structure_cours;
mod structure_offre;
mod structure_periodes;
mod voyages;
mod edt_s2;
mod roles;
mod routes;
mod vie_scolaire;
mod evaluations;
mod communication_s28;
mod eleve_propositions;
mod eleve_fusion;
mod sante;
pub mod tenant;
mod ui;

pub use auth::EntSession;
pub use tenant::TenantHote;
/// Environnements non productifs (porte du labo et de l'inscription libre) ;
/// exposé pour le test de parité avec le bandeau labo du frontend.
pub use routes::ENVS_NON_PROD;

use axum::Router;
use ent_db::PgEnt;
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tracing::Level;

/// SHA git injecté au build (`GIT_SHA`) — `"dev"` en local.
pub const GIT_SHA: &str = match option_env!("GIT_SHA") {
    Some(s) => s,
    None => "dev",
};

/// Version du crate (Cargo).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct AppState {
    pub ent: Arc<PgEnt>,
    /// Registre de métriques du processus (`/metrics`).
    pub obs: Arc<ent_obs::Metrics>,
    /// Politique d'accès à `/metrics` (jeton, ouvert, désactivé).
    pub metrics_acces: obs::MetricsAcces,
    /// Tâches de fond lancées par des handlers (compteur de tâches en cours).
    taches: Arc<std::sync::atomic::AtomicUsize>,
}

impl AppState {
    /// Lance un traitement différé hors requête (ex. envoi d'un e-mail dont le
    /// temps ne doit pas être observable par le client). Le compteur permet aux
    /// tests d'attendre la fin (`attendre_taches`).
    pub(crate) fn lancer_tache<F>(&self, travail: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        use std::sync::atomic::Ordering;
        let compteur = self.taches.clone();
        compteur.fetch_add(1, Ordering::SeqCst);
        tokio::spawn(async move {
            travail.await;
            compteur.fetch_sub(1, Ordering::SeqCst);
        });
    }

    /// Attend la fin des tâches différées (tests ; borné à 10 s).
    pub async fn attendre_taches(&self) {
        use std::sync::atomic::Ordering;
        for _ in 0..1000 {
            if self.taches.load(Ordering::SeqCst) == 0 {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    /// État de production/test : familles de métriques déclarées, politique
    /// d'accès lue dans l'environnement.
    pub fn nouveau(ent: PgEnt) -> Self {
        let obs = Arc::new(ent_obs::Metrics::new());
        obs::installer_familles(&obs);
        Self {
            ent: Arc::new(ent),
            obs,
            metrics_acces: obs::MetricsAcces::depuis_env(),
            taches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
}

/// Taille maximale du corps de requête (pièces, photos, PDF). Configurable.
fn max_body_bytes() -> usize {
    std::env::var("ENT_MAX_BODY_BYTES")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(25 * 1024 * 1024)
}

/// Délai maximal de traitement d'une requête (secondes). `0` = désactivé.
fn request_timeout_secs() -> u64 {
    std::env::var("ENT_REQUEST_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(120)
}

/// CORS optionnel via `ENT_CORS_ORIGINS` (liste séparée par des virgules,
/// ou `*`). Sans variable → pas de CORS (same-origin uniquement).
fn cors_layer_from_env() -> Option<CorsLayer> {
    let raw = std::env::var("ENT_CORS_ORIGINS").ok()?;
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if raw == "*" {
        return Some(CorsLayer::permissive());
    }
    let origins: Vec<axum::http::HeaderValue> =
        raw.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    if origins.is_empty() {
        return None;
    }
    Some(
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(Any)
            .allow_headers(Any),
    )
}

pub fn app(state: AppState) -> Router {
    let fonts = ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/static"));
    let mut router = Router::new()
        .nest_service("/static", fonts)
        .merge(routes::router())
        // Établissement déduit de l'adresse (C2, D-012) : résolu une fois par requête.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            tenant::resoudre_tenant,
        ))
        // Module santé désactivé par défaut : refus côté serveur des routes de santé tant que
        // l'établissement ne l'a pas activé (règle produit : pas de donnée de santé sans HDS).
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            sante::exiger_module,
        ))
        // R2 (D-017) : droits effectifs de la personne, une fois par requête.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::contexte_droits,
        ))
        .with_state(state.clone())
        // Limitation de débit des routes d'authentification publiques (quotas en
        // base, `429` + `Retry-After`). Interne au plafond de taille du corps.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            anti_abus::limiter_debit,
        ))
        // Garde-fou corps de requête (anti-DoS).
        .layer(RequestBodyLimitLayer::new(max_body_bytes()));

    // Délai maximal de traitement (0 = désactivé).
    let timeout = request_timeout_secs();
    if timeout > 0 {
        router = router.layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(timeout),
        ));
    }

    router = router
        // Un panic dans un handler ne doit jamais tuer le serveur.
        .layer(CatchPanicLayer::new())
        // En-têtes no-store et 500 génériques (couche externe au catch-panic).
        .layer(axum::middleware::from_fn(http_garde::en_tetes_et_erreurs))
        // En-têtes de sécurité de base.
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_CONTENT_TYPE_OPTIONS,
            axum::http::HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::X_FRAME_OPTIONS,
            axum::http::HeaderValue::from_static("SAMEORIGIN"),
        ))
        // Journalisation des requêtes (méthode, chemin, statut, latence) au niveau
        // INFO pour qu'elles apparaissent avec le `RUST_LOG=info` par défaut.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(http_garde::span_requete)
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
        )
        // Identifiant de requête : repris dans la réponse pour corréler les logs.
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        // Un `x-request-id` fourni par le client n'est gardé que s'il a un format sûr
        // (sinon retiré : SetRequestIdLayer en génère un UUID). Couche externe.
        .layer(axum::middleware::from_fn(http_garde::assainir_request_id))
        // Métriques : couche **externe** (elle voit le statut final, y compris
        // un 408 du délai ou un 500 du catch-panic) et lit `MatchedPath`
        // inséré par le routeur, donc un gabarit et non le chemin brut.
        .layer(axum::middleware::from_fn_with_state(
            state,
            obs::suivre_requetes,
        ));

    match cors_layer_from_env() {
        Some(cors) => router.layer(cors),
        None => router,
    }
}

pub async fn boot_from_env() -> Result<(AppState, String), String> {
    let url = std::env::var("ENT_DATABASE_URL")
        .map_err(|_| "ENT_DATABASE_URL requis (postgres://…@127.0.0.1/…)".to_string())?;
    let mut ent = PgEnt::connect(&url)
        .await
        .map_err(|e| format!("connect: {e}"))?;
    ent.migrate().await.map_err(|e| format!("migrate: {e}"))?;
    ent.install_brain_p0();
    if let Ok(purged) = ent.purge_expired_sessions().await {
        if purged > 0 {
            tracing::info!("sessions expirées purgées : {purged}");
        }
    }
    let _ = ent.purge_expired_password_resets().await;
    // A7 : rechiffre les secrets clients OIDC hérités en clair (idempotent).
    match ent.rechiffrer_secrets_oidc().await {
        Ok(n) if n > 0 => tracing::info!("secrets clients OIDC rechiffrés : {n}"),
        Ok(_) => {}
        Err(e) => tracing::warn!("rechiffrement des secrets OIDC impossible : {e}"),
    }
    // D3 : rechiffre les champs sensibles hérités en clair (IBAN, santé, psy) — idempotent.
    match ent.rechiffrer_champs_sensibles().await {
        Ok(r) if r.champs_rechiffres > 0 => {
            tracing::info!("champs sensibles rechiffrés : {}", r.champs_rechiffres)
        }
        Ok(_) => {}
        Err(e) => tracing::warn!("rechiffrement des champs sensibles impossible : {e}"),
    }
    let _ = ent.purger_recuperations_passkey().await;
    let _ = ent.purger_changements_email().await;
    let bind = std::env::var("ENT_API_BIND").unwrap_or_else(|_| "127.0.0.1:8088".into());
    Ok((AppState::nouveau(ent), bind))
}

// Tests L6 : ceux qui exigent `/auth/register` ou le cast labo sont gatés un à un.
#[cfg(test)]
mod tests_l6;
#[cfg(all(test, feature = "labo"))]
mod tests_l3;
#[cfg(all(test, feature = "labo"))]
mod tests_personnels_p0;
#[cfg(all(test, feature = "labo"))]
mod tests_personnels_p1;
#[cfg(all(test, feature = "labo"))]
mod tests_personnels_p2;
#[cfg(all(test, feature = "labo"))]
mod tests_personnels_p3;
#[cfg(all(test, feature = "labo"))]
mod tests_r2;
#[cfg(all(test, feature = "labo"))]
mod tests_eleve_propositions;
#[cfg(all(test, feature = "labo"))]
mod tests_abus;
#[cfg(all(test, feature = "labo"))]
mod tests_sso;
#[cfg(all(test, feature = "labo"))]
mod tests_recuperation;
#[cfg(all(test, feature = "labo"))]
mod tests_dossier;
#[cfg(all(test, feature = "labo"))]
mod tests_inscriptions;
#[cfg(all(test, feature = "labo"))]
mod tests_evaluations_v0;
#[cfg(all(test, feature = "labo"))]
mod tests_edt_s2;
#[cfg(all(test, feature = "labo"))]
mod tests_sante_module;
#[cfg(all(test, feature = "labo"))]
mod tests_finances;
#[cfg(all(test, feature = "labo"))]
mod tests_structure;
#[cfg(all(test, feature = "labo"))]
mod tests_structure_s3;
#[cfg(all(test, feature = "labo"))]
mod tests_structure_s4;
#[cfg(all(test, feature = "labo"))]
mod tests_voyages;
#[cfg(all(test, feature = "labo"))]
mod tests_structure_s5;
#[cfg(all(test, feature = "labo"))]
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
        // Messagerie en dry-run : les invitations / liens envoyés sont consultables.
        ent.install_mail_dry_run();
        Some(AppState::nouveau(ent))
    }

    /// Active un compte invité : lit le lien reçu par `email`, choisit un mot de passe par
    /// `POST /auth/reset-password` et le renvoie (socle 6 P0 : plus de mot de passe temporaire).
    async fn activer_par_lien(app: &axum::Router, ent: &PgEnt, email: &str) -> String {
        let chemin = "/reset-mot-de-passe";
        let token = ent
            .mailer()
            .dry_run_corps()
            .into_iter()
            .rev()
            .filter(|(to, _, _)| to == email)
            .find_map(|(_, _, corps)| {
                let i = corps.find(&format!("{chemin}?token="))? + chemin.len() + "?token=".len();
                Some(
                    corps[i..]
                        .chars()
                        .take_while(|c| c.is_ascii_hexdigit())
                        .collect::<String>(),
                )
            })
            .expect("lien reçu par e-mail");
        let mdp = format!("Invite-{}-Aa9!", Uuid::new_v4().simple());
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/reset-password")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"token": token, "new_password": mdp}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(res.status().is_success(), "activation par lien : {}", res.status());
        mdp
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
        register_and_login_role(app, etab, email, "admin").await
    }

    /// Socle 1 (B1) : donne au professeur (par e-mail) un créneau d'EDT dans la
    /// classe, pour qu'il « suive » ses élèves (sinon notes et dossier lui sont fermés).
    async fn rattacher_prof_classe(etab: Uuid, email: &str, classe: &str) {
        let url = std::env::var("ENT_DATABASE_URL").expect("ENT_DATABASE_URL");
        let pool = sqlx::PgPool::connect(&url).await.expect("pool");
        sqlx::query(
            "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle,
                                          prof_user_id, jour_semaine, heure_debut, heure_fin)
             SELECT $1, $2, $3, 'Test', u.id, 1, '08:00', '09:00'
               FROM ent_user u WHERE lower(u.email) = lower($4)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(classe)
        .bind(email)
        .execute(&pool)
        .await
        .expect("créneau du professeur");
    }

    async fn register_and_login_role(
        app: &axum::Router,
        etab: Uuid,
        email: &str,
        role: &str,
    ) -> String {
        register_login_full(app, etab, email, role).await.0
    }

    async fn register_login_full(
        app: &axum::Router,
        etab: Uuid,
        email: &str,
        role: &str,
    ) -> (String, Uuid) {
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
        let token = body["token"].as_str().unwrap().to_string();
        let user_id = Uuid::parse_str(body["user_id"].as_str().unwrap()).unwrap();
        (token, user_id)
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
    async fn ready_checks_db_and_security_headers() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let app = app(state);
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // Readiness : base joignable + en-têtes de sécurité posés par les couches globales.
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert_eq!(res.headers().get("x-frame-options").unwrap(), "SAMEORIGIN");
        // Corrélation logs ↔ réponse.
        assert!(res.headers().get("x-request-id").is_some());
        let body = json_body(res).await;
        assert_eq!(body["db"], "up");
        assert!(body["git_sha"].is_string());
        assert!(body["version"].is_string());
    }

    #[tokio::test]
    async fn login_locked_after_repeated_failures() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let etab = Uuid::new_v4();
        let email = format!("brute-{}@ent.local", Uuid::new_v4());
        let app = app(state);
        // Première connexion valide : compteur à zéro.
        let _ = register_and_login(&app, etab, &email).await;

        let mut saw_429 = false;
        for _ in 0..12 {
            let res = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/auth/login")
                        .header("content-type", "application/json")
                        .body(Body::from(format!(
                            r#"{{"email":"{email}","password":"MauvaisPass1!","etablissement_id":"{etab}"}}"#
                        )))
                        .unwrap(),
                )
                .await
                .unwrap();
            if res.status() == StatusCode::TOO_MANY_REQUESTS {
                saw_429 = true;
                break;
            }
            assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        }
        assert!(saw_429, "la connexion doit finir verrouillée (429)");

        // Même avec le bon mot de passe, le verrou tient.
        let res = app
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
        assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    /// Garde par IP via HTTP : `ENT_TRUST_PROXY=1` active la lecture de
    /// `x-forwarded-for`. Au-delà du seuil, l'IP est verrouillée (429) même pour
    /// d'autres comptes, et une autre IP n'est pas pénalisée.
    #[tokio::test]
    async fn api_garde_login_par_ip() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        std::env::set_var("ENT_TRUST_PROXY", "1");
        std::env::set_var("ENT_LOGIN_MAX_FAILED_IP", "3");
        let etab = Uuid::new_v4();
        // IP **et** e-mails uniques par exécution : les compteurs (compte + IP)
        // vivent en base et persistent entre exécutions de la suite.
        let uid = Uuid::new_v4();
        let b = uid.into_bytes();
        let ip_brulee = format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]);
        let ip_neuve = format!("{}.{}.{}.{}", b[4], b[5], b[6], b[7]);
        let app = app(state);

        let call = |email: String, ip: &str| {
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                // Le proxy de confiance (socket 10.0.0.1) ajoute l'IP vue en dernier.
                .header("x-forwarded-for", format!("6.6.6.6, {ip}"))
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [10, 0, 0, 1],
                    4000,
                ))))
                .body(Body::from(format!(
                    r#"{{"email":"{email}","password":"MauvaisPass1!","etablissement_id":"{etab}"}}"#
                )))
                .unwrap()
        };

        // 3 échecs depuis la même IP (comptes distincts) → seuil atteint.
        for i in 0..3 {
            let res = app
                .clone()
                .oneshot(call(format!("ip-brute-{uid}-{i}@ent.local"), &ip_brulee))
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        }
        // Même IP au-delà du seuil → 429.
        let res = app
            .clone()
            .oneshot(call(format!("ip-brute-{uid}-x@ent.local"), &ip_brulee))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
        // Autre IP → non pénalisée (401).
        let res = app
            .clone()
            .oneshot(call(format!("ip-brute-{uid}-y@ent.local"), &ip_neuve))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        std::env::remove_var("ENT_TRUST_PROXY");
        std::env::remove_var("ENT_LOGIN_MAX_FAILED_IP");
    }

    /// Régression : un **compte neuf** (jamais connecté, donc sans ligne
    /// `ent_login_guard` préexistante) doit aussi accumuler les échecs jusqu'au
    /// verrou. Avant correctif, la fenêtre d'une ligne neuve démarrait au
    /// plancher → compteur bloqué à 1 → verrou jamais atteint.
    #[tokio::test]
    async fn api_garde_login_compte_neuf_accumule() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let etab = Uuid::new_v4();
        let email = format!("neuf-{}@ent.local", Uuid::new_v4());
        let app = app(state);

        let mut verrou = false;
        for _ in 0..8 {
            let res = app
                .clone()
                .oneshot(login_request(etab, &email, "MauvaisPass1!"))
                .await
                .unwrap();
            match res.status() {
                StatusCode::TOO_MANY_REQUESTS => {
                    verrou = true;
                    break;
                }
                StatusCode::UNAUTHORIZED => {}
                autre => panic!("statut inattendu: {autre}"),
            }
        }
        assert!(
            verrou,
            "un compte neuf (jamais connecté) doit finir verrouillé"
        );
    }

    fn login_request(
        etab: Uuid,
        email: &str,
        password: &str,
    ) -> axum::http::Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"email":"{email}","password":"{password}","etablissement_id":"{etab}"}}"#
            )))
            .unwrap()
    }

    fn provision_request(bearer: &str, email: &str, role: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/etablissement/personnel")
            .header("authorization", bearer)
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"email":"{email}","role":"{role}"}}"#
            )))
            .unwrap()
    }

    #[tokio::test]
    async fn api_provisionne_personnel_garde_fous() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let etab = Uuid::new_v4();
        let admin_email = format!("admin-{}@ent.local", Uuid::new_v4());
        let dir_email = format!("dir-{}@ent.local", Uuid::new_v4());
        let parent_email = format!("par-{}@ent.local", Uuid::new_v4());
        let ent = state.ent.clone();
        let app = app(state);

        let admin = register_and_login_role(&app, etab, &admin_email, "admin").await;
        let dir = register_and_login_role(&app, etab, &dir_email, "direction").await;
        let parent = register_and_login_role(&app, etab, &parent_email, "parent").await;

        // Admin provisionne un prof → 201, invitation par e-mail (aucun mot de passe renvoyé).
        let prof_email = format!("prof-{}@ent.local", Uuid::new_v4());
        let res = app
            .clone()
            .oneshot(provision_request(
                &format!("Bearer {admin}"),
                &prof_email,
                "prof",
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = json_body(res).await;
        assert_eq!(body["role"], "prof");
        // Aucun secret dans la réponse : une invitation part par e-mail.
        assert!(body.get("mot_de_passe_temporaire").is_none(), "secret renvoyé : {body}");
        assert_eq!(body["invitation_envoyee"], true);
        let temp = activer_par_lien(&app, &ent, &prof_email).await;

        let login = app
            .clone()
            .oneshot(login_request(etab, &prof_email, &temp))
            .await
            .unwrap();
        assert_eq!(login.status(), StatusCode::OK);
        assert_eq!(json_body(login).await["role"], "prof");

        // Doublon d'e-mail → 409.
        let res = app
            .clone()
            .oneshot(provision_request(
                &format!("Bearer {admin}"),
                &prof_email,
                "prof",
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CONFLICT);

        // Direction ne peut pas créer un Admin → 403.
        let res = app
            .clone()
            .oneshot(provision_request(
                &format!("Bearer {dir}"),
                &format!("x-{}@ent.local", Uuid::new_v4()),
                "admin",
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        // Direction peut créer un prof → 201.
        let res = app
            .clone()
            .oneshot(provision_request(
                &format!("Bearer {dir}"),
                &format!("p2-{}@ent.local", Uuid::new_v4()),
                "prof",
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        // Un parent ne provisionne pas → 403.
        let res = app
            .oneshot(provision_request(
                &format!("Bearer {parent}"),
                &format!("p3-{}@ent.local", Uuid::new_v4()),
                "prof",
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn api_reinitialise_mot_de_passe_personnel() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let etab = Uuid::new_v4();
        let ent = state.ent.clone();
        let app = app(state);
        let (admin_token, admin_id) = register_login_full(
            &app,
            etab,
            &format!("adm-{}@ent.local", Uuid::new_v4()),
            "admin",
        )
        .await;
        let (dir_token, _) = register_login_full(
            &app,
            etab,
            &format!("dir-{}@ent.local", Uuid::new_v4()),
            "direction",
        )
        .await;

        // Admin provisionne un prof.
        let prof_email = format!("prof-{}@ent.local", Uuid::new_v4());
        let res = app
            .clone()
            .oneshot(provision_request(
                &format!("Bearer {admin_token}"),
                &prof_email,
                "prof",
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = json_body(res).await;
        let prof_id = Uuid::parse_str(body["user_id"].as_str().unwrap()).unwrap();
        assert!(body.get("mot_de_passe_temporaire").is_none(), "secret renvoyé : {body}");
        let temp1 = activer_par_lien(&app, &ent, &prof_email).await;

        // Le prof se connecte avec le mot de passe qu'il a choisi.
        let login = app
            .clone()
            .oneshot(login_request(etab, &prof_email, &temp1))
            .await
            .unwrap();
        assert_eq!(login.status(), StatusCode::OK);
        let prof_token = json_body(login).await["token"].as_str().unwrap().to_string();

        // Admin réinitialise → nouveau mot de passe + sessions du prof révoquées.
        let reset_uri = format!("/etablissement/personnel/{prof_id}/reset-mot-de-passe");
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&reset_uri)
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // Lien envoyé par e-mail (202) : aucun secret dans la réponse.
        assert_eq!(res.status(), StatusCode::ACCEPTED);
        let corps = json_body(res).await;
        assert!(corps.get("mot_de_passe_temporaire").is_none(), "secret renvoyé : {corps}");
        assert_eq!(corps["lien_envoye"], true);
        let temp2 = activer_par_lien(&app, &ent, &prof_email).await;
        assert_ne!(temp1, temp2);

        let me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/auth/me")
                    .header("authorization", format!("Bearer {prof_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(me.status(), StatusCode::UNAUTHORIZED);

        let old = app
            .clone()
            .oneshot(login_request(etab, &prof_email, &temp1))
            .await
            .unwrap();
        assert_eq!(old.status(), StatusCode::UNAUTHORIZED);
        let new = app
            .clone()
            .oneshot(login_request(etab, &prof_email, &temp2))
            .await
            .unwrap();
        assert_eq!(new.status(), StatusCode::OK);

        // Direction ne peut pas réinitialiser un Admin → 403.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/etablissement/personnel/{admin_id}/reset-mot-de-passe"))
                    .header("authorization", format!("Bearer {dir_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        // Auto-réinitialisation interdite → 403.
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/etablissement/personnel/{admin_id}/reset-mot-de-passe"))
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn api_change_password_et_audit_connexion() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let etab = Uuid::new_v4();
        let email = format!("pwd-{}@ent.local", Uuid::new_v4());
        let app = app(state);
        let token = register_and_login(&app, etab, &email).await;
        let bearer = format!("Bearer {token}");

        // Audit d'accès : la connexion est tracée dans le journal (domaine auth).
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/audit/journal?domaine=auth")
                    .header("authorization", &bearer)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = json_body(res).await;
        assert!(
            body.as_array()
                .map(|a| a.iter().any(|e| e["action"] == "connexion"))
                .unwrap_or(false),
            "journal auth: {body}"
        );

        // Mauvais mot de passe actuel refusé.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/password")
                    .header("authorization", &bearer)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"current_password":"FauxPass1!","new_password":"NouveauPass1!"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        // Changement valide.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/password")
                    .header("authorization", &bearer)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"current_password":"DevLocalPass1!","new_password":"NouveauPass1!"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        // L'ancien mot de passe ne fonctionne plus, le nouveau oui.
        let old = app
            .clone()
            .oneshot(login_request(etab, &email, "DevLocalPass1!"))
            .await
            .unwrap();
        assert_eq!(old.status(), StatusCode::UNAUTHORIZED);
        let new = app
            .oneshot(login_request(etab, &email, "NouveauPass1!"))
            .await
            .unwrap();
        assert_eq!(new.status(), StatusCode::OK);
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
                        r#"{"code":"ANG","libelle":"Anglais"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(forbid.status(), StatusCode::FORBIDDEN);

        // Administratif crée Anglais (code non fourni par le seed labo)
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
                        r#"{"code":"ANG","libelle":"Anglais","source_nomenclature":"SIECLE:ANG"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);
        let matiere = json_body(created).await;
        let mid = matiere["id"].as_str().unwrap();

        // Doublon de code → 409 Conflict (pas 404 ni 400).
        let doublon = app
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
                        r#"{"code":"ANG","libelle":"Anglais bis"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(doublon.status(), StatusCode::CONFLICT);

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
        // V0 (D1) : la saisie historique est fermée (410) ; les notes se saisissent par la grille.
        assert_eq!(note.status(), StatusCode::GONE);

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

    /// Sortie élève via HTTP : le compte face perd immédiatement son accès
    /// (session révoquée → 401). Prouve la propagation `sortir` → auth.
    #[tokio::test]
    async fn api_sortie_eleve_revoque_acces_face() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let cast = state.ent.seed_labo_cast().await.expect("cast");
        let app = app(state);
        let jour = Utc::now().date_naive().to_string();

        let eleve = cast
            .eleves
            .iter()
            .find(|e| e.compte.is_some())
            .expect("un élève avec compte face");
        let compte = eleve.compte.as_ref().expect("compte face");
        let eleve_id = eleve.id;

        // Avant sortie : l'élève lit son EDT (200).
        let avant = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/edt/mes-creneaux?jour={jour}"))
                    .header("authorization", format!("Bearer {}", compte.token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(avant.status(), StatusCode::OK);

        // L'admini sort l'élève.
        let sortie = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/eleves/{eleve_id}/sortie"))
                    .header(
                        "authorization",
                        format!("Bearer {}", cast.administratif.token),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"date_sortie":"{jour}","motif_sortie":"demenagement"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(sortie.status(), StatusCode::OK);

        // Après sortie : la session du compte face est révoquée (401).
        let apres = app
            .oneshot(
                Request::builder()
                    .uri(format!("/edt/mes-creneaux?jour={jour}"))
                    .header("authorization", format!("Bearer {}", compte.token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(apres.status(), StatusCode::UNAUTHORIZED);
    }

    /// Journal des modifications exposé en HTTP : une bascule de classe est
    /// tracée et lisible par la famille (compte parent lié).
    #[tokio::test]
    async fn api_journal_classe_change_visible_famille() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let cast = state.ent.seed_labo_cast().await.expect("cast");
        let app = app(state);
        let jour = Utc::now().date_naive().to_string();

        // Admini change la classe de l'élève.
        let change = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/eleves/{}/classe", cast.eleve_lea_id))
                    .header(
                        "authorization",
                        format!("Bearer {}", cast.administratif.token),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"nouvelle_classe":"5A","date_effet":"{jour}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(change.status(), StatusCode::OK);

        // Le parent lié lit le journal et voit la bascule.
        let journal = app
            .oneshot(
                Request::builder()
                    .uri(format!("/eleves/{}/journal", cast.eleve_lea_id))
                    .header(
                        "authorization",
                        format!("Bearer {}", cast.parent_lea.token),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(journal.status(), StatusCode::OK);
        let entries = json_body(journal).await;
        let arr = entries.as_array().expect("tableau journal");
        assert!(arr
            .iter()
            .any(|e| e["action"].as_str() == Some("classe_change")));
    }

    #[tokio::test]
    async fn api_copilote_ask_et_roles() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let cast = state.ent.seed_labo_cast().await.expect("cast");
        let app = app(state);

        // 1) Effectif 4B (administratif) — Harry, Hermione, Ginny.
        let eff = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/copilote/ask")
                    .header(
                        "authorization",
                        format!("Bearer {}", cast.administratif.token),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"question":"effectif 4B"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(eff.status(), StatusCode::OK);
        let body = json_body(eff).await;
        assert_eq!(body["intention"], "effectif");
        assert_eq!(body["donnees"]["effectif"].as_i64(), Some(3));
        assert_eq!(body["refusee"], false);

        // 2) Impayés : refusés au prof (capacité), OK pour la compta.
        let prof = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/copilote/ask")
                    .header("authorization", format!("Bearer {}", cast.prof.token))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"question":"factures impayées"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(prof.status(), StatusCode::OK);
        assert_eq!(json_body(prof).await["refusee"], true);

        let compta = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/copilote/ask")
                    .header("authorization", format!("Bearer {}", cast.compta.token))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"question":"factures impayées"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(compta.status(), StatusCode::OK);
        let cbody = json_body(compta).await;
        assert_eq!(cbody["intention"], "impayes");
        assert_eq!(cbody["refusee"], false);

        // 3) Parent → 403 (réservé au personnel).
        let parent = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/copilote/ask")
                    .header(
                        "authorization",
                        format!("Bearer {}", cast.parent_lea.token),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"question":"effectif"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(parent.status(), StatusCode::FORBIDDEN);

        // 4) Question vide → 400.
        let vide = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/copilote/ask")
                    .header(
                        "authorization",
                        format!("Bearer {}", cast.administratif.token),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"question":"   "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(vide.status(), StatusCode::BAD_REQUEST);

        // 5) Capacités (aide UI).
        let caps = app
            .oneshot(
                Request::builder()
                    .uri("/copilote/capacites")
                    .header(
                        "authorization",
                        format!("Bearer {}", cast.administratif.token),
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(caps.status(), StatusCode::OK);
        assert!(json_body(caps).await["capacites"].as_array().is_some());
    }

    #[tokio::test]
    async fn api_forgot_reset_mot_de_passe_anti_enumeration() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let app = app(state);
        let etab = Uuid::new_v4();
        let email = format!("reset-{}@ent.test", &Uuid::new_v4().to_string()[..8]);
        let _ = register_and_login(&app, etab, &email).await;

        // Compte connu → 202 (même réponse que compte inconnu).
        let connu = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/forgot-password")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"email":"{email}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(connu.status(), StatusCode::ACCEPTED);

        // Compte inconnu → 202 aussi (anti-énumération).
        let inconnu = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/forgot-password")
                    .header("content-type", "application/json")
                    // E-mail unique par exécution : le seau par identifiant
                    // (3/heure) persiste en base entre deux lancements.
                    .body(Body::from(format!(
                        r#"{{"email":"personne-{}@nulle.part"}}"#,
                        Uuid::new_v4().simple()
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(inconnu.status(), StatusCode::ACCEPTED);

        // Jeton bidon → 400 (pas 500).
        let bidon = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/reset-password")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"token":"deadbeef","new_password":"NouveauPass1!2026"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bidon.status(), StatusCode::BAD_REQUEST);

        // Mot de passe trop court → 400 (politique avant le jeton).
        let court = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/reset-password")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"token":"deadbeef","new_password":"court"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(court.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn api_annonces_gate_role_validation_et_isolation() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let app = app(state);
        let etab = Uuid::new_v4();

        // La direction publie une annonce interne (audience personnels → pas de
        // diffusion famille nécessaire, donc aucun élève requis).
        let email_dir = format!("dir-{}@ent.test", &Uuid::new_v4().to_string()[..8]);
        let token_dir = register_and_login_role(&app, etab, &email_dir, "direction").await;
        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/annonces")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_dir}"))
                    .body(Body::from(
                        r#"{"titre":"Note de service","corps":"Interne.","audience":"personnels"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);
        let created = json_body(created).await;
        assert_eq!(created["audience"], "personnels");
        let annonce_id = created["id"].as_str().unwrap().to_string();

        // Ciblage « classe » sans classe → 400 (validation domaine → HTTP).
        let invalide = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/annonces")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_dir}"))
                    .body(Body::from(
                        r#"{"titre":"Sortie","corps":"RDV 8h","audience":"classe"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalide.status(), StatusCode::BAD_REQUEST);

        // La liste staff contient l'annonce publiée.
        let liste = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/annonces")
                    .header("authorization", format!("Bearer {token_dir}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(liste.status(), StatusCode::OK);
        let liste = json_body(liste).await;
        assert!(liste
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["id"].as_str() == Some(annonce_id.as_str())));

        // Un prof ne publie pas d'annonce → 403.
        let email_prof = format!("prof-{}@ent.test", &Uuid::new_v4().to_string()[..8]);
        let token_prof = register_and_login_role(&app, etab, &email_prof, "prof").await;
        let refuse = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/annonces")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_prof}"))
                    .body(Body::from(
                        r#"{"titre":"Interdit","corps":"Non.","audience":"parents"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(refuse.status(), StatusCode::FORBIDDEN);

        // Isolation tenant : l'annonce d'un autre établissement n'apparaît pas.
        let etab2 = Uuid::new_v4();
        let email_dir2 = format!("dir2-{}@ent.test", &Uuid::new_v4().to_string()[..8]);
        let token_dir2 = register_and_login_role(&app, etab2, &email_dir2, "direction").await;
        let liste2 = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/annonces")
                    .header("authorization", format!("Bearer {token_dir2}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let liste2 = json_body(liste2).await;
        assert!(!liste2
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["id"].as_str() == Some(annonce_id.as_str())));
    }

    #[tokio::test]
    async fn api_retards_gate_role_validation_et_isolation() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let app = app(state);
        let etab = Uuid::new_v4();

        // Un admin crée un élève (4B) dans l'établissement.
        let email_admin = format!("adm-{}@ent.test", &Uuid::new_v4().to_string()[..8]);
        let token_admin = register_and_login(&app, etab, &email_admin).await;
        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/eleves")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_admin}"))
                    .body(Body::from(r#"{"nom":"Rtd","prenom":"Test","classe":"4B"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);
        let eleve_id = json_body(created).await["id"].as_str().unwrap().to_string();

        // La vie scolaire déclare un retard → 201.
        let email_cpe = format!("cpe-{}@ent.test", &Uuid::new_v4().to_string()[..8]);
        let token_cpe = register_and_login_role(&app, etab, &email_cpe, "cpe").await;
        let declare = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/eleves/{eleve_id}/retards"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_cpe}"))
                    .body(Body::from(
                        r#"{"jour":"2026-10-12","minutes":15,"motif":"Bus"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(declare.status(), StatusCode::CREATED);
        let declare = json_body(declare).await;
        assert_eq!(declare["minutes"], 15);
        assert_eq!(declare["jour"], "2026-10-12");

        // Le staff relit l'historique.
        let liste = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/eleves/{eleve_id}/retards"))
                    .header("authorization", format!("Bearer {token_cpe}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(liste.status(), StatusCode::OK);
        let liste = json_body(liste).await;
        assert_eq!(liste.as_array().unwrap().len(), 1);

        // Durée aberrante → 400 (validation domaine → HTTP).
        let invalide = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/eleves/{eleve_id}/retards"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_cpe}"))
                    .body(Body::from(r#"{"jour":"2026-10-12","minutes":9999}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalide.status(), StatusCode::BAD_REQUEST);

        // Un parent ne déclare pas un retard → 403.
        let email_parent = format!("par-{}@ent.test", &Uuid::new_v4().to_string()[..8]);
        let token_parent = register_and_login_role(&app, etab, &email_parent, "parent").await;
        let refuse = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/eleves/{eleve_id}/retards"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_parent}"))
                    .body(Body::from(r#"{"jour":"2026-10-12"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(refuse.status(), StatusCode::FORBIDDEN);

        // Un parent non lié à l'élève ne lit pas ses retards → 403.
        let lecture_parent = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/eleves/{eleve_id}/retards"))
                    .header("authorization", format!("Bearer {token_parent}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(lecture_parent.status(), StatusCode::FORBIDDEN);

        // Isolation tenant : un staff d'un autre établissement → 404.
        let etab2 = Uuid::new_v4();
        let email2 = format!("adm2-{}@ent.test", &Uuid::new_v4().to_string()[..8]);
        let token2 = register_and_login(&app, etab2, &email2).await;
        let hors = app
            .oneshot(
                Request::builder()
                    .uri(format!("/eleves/{eleve_id}/retards"))
                    .header("authorization", format!("Bearer {token2}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(hors.status(), StatusCode::NOT_FOUND);
    }

    /// Compétences / LSU — gate rôle, validation domaine→HTTP, isolation tenant,
    /// et livret PDF publié comme document famille.
    #[tokio::test]
    async fn api_competences_gate_role_validation_et_isolation() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let app = app(state);
        let etab = Uuid::new_v4();

        let email_admin = format!("adm-c-{}@ent.test", &Uuid::new_v4().to_string()[..8]);
        let token_admin = register_and_login(&app, etab, &email_admin).await;

        // Un élève 4B.
        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/eleves")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_admin}"))
                    .body(Body::from(r#"{"nom":"Comp","prenom":"Test","classe":"4B"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);
        let eleve_id = json_body(created).await["id"].as_str().unwrap().to_string();

        // Une période de notes (T1) pour porter les évaluations.
        let periode = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/note-periodes")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_admin}"))
                    .body(Body::from(
                        r#"{"code":"T1","libelle":"Trimestre 1","date_debut":"2026-09-01","date_fin":"2026-12-20"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(periode.status(), StatusCode::CREATED);
        let periode_id = json_body(periode).await["id"].as_str().unwrap().to_string();

        // Admin : crée un item du référentiel.
        let item = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/competences")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_admin}"))
                    .body(Body::from(
                        r#"{"domaine":"D1","code":"D1.1","libelle":"Comprendre et s'exprimer à l'oral"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(item.status(), StatusCode::CREATED);
        let item_id = json_body(item).await["id"].as_str().unwrap().to_string();

        // Référentiel lisible, item invalidé → 400.
        let item_invalide = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/competences")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_admin}"))
                    .body(Body::from(r#"{"domaine":"D1","code":"D1.9","libelle":"  "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(item_invalide.status(), StatusCode::BAD_REQUEST);

        // Prof : évalue (niveau + code LSU renvoyés).
        let email_prof = format!("prof-c-{}@ent.test", &Uuid::new_v4().to_string()[..8]);
        let token_prof = register_and_login_role(&app, etab, &email_prof, "prof").await;
        rattacher_prof_classe(etab, &email_prof, "4B").await;
        let evalue = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/eleves/{eleve_id}/competences"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_prof}"))
                    .body(Body::from(format!(
                        r#"{{"item_id":"{item_id}","periode_id":"{periode_id}","niveau":"tres_bonne","commentaire":"bravo"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(evalue.status(), StatusCode::CREATED);
        let evalue = json_body(evalue).await;
        assert_eq!(evalue["niveau"], "tres_bonne");
        assert_eq!(evalue["code_lsu"], "4");
        assert_eq!(evalue["domaine"], "D1");

        // Niveau inconnu → 400.
        let niveau_invalide = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/eleves/{eleve_id}/competences"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_prof}"))
                    .body(Body::from(format!(
                        r#"{{"item_id":"{item_id}","periode_id":"{periode_id}","niveau":"genial"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(niveau_invalide.status(), StatusCode::BAD_REQUEST);

        // Un parent n'évalue pas → 403 ; et ne lit pas un élève non lié → 403.
        let email_parent = format!("par-c-{}@ent.test", &Uuid::new_v4().to_string()[..8]);
        let token_parent = register_and_login_role(&app, etab, &email_parent, "parent").await;
        let parent_evalue = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/eleves/{eleve_id}/competences"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_parent}"))
                    .body(Body::from(format!(
                        r#"{{"item_id":"{item_id}","periode_id":"{periode_id}","niveau":"4"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(parent_evalue.status(), StatusCode::FORBIDDEN);
        let parent_lit = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/eleves/{eleve_id}/competences?periode_id={periode_id}"))
                    .header("authorization", format!("Bearer {token_parent}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(parent_lit.status(), StatusCode::FORBIDDEN);

        // Livret publié comme document famille.
        let livret = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/eleves/{eleve_id}/livret-competences"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token_prof}"))
                    .body(Body::from(format!(r#"{{"periode_id":"{periode_id}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(livret.status(), StatusCode::CREATED);
        let livret = json_body(livret).await;
        assert_eq!(livret["tiroir"], "famille");
        assert_eq!(livret["code"], "LIVRET_T1");

        // Isolation tenant : le référentiel d'un autre établissement est vide,
        // et l'élève d'un autre établissement est introuvable.
        let etab2 = Uuid::new_v4();
        let email2 = format!("adm2-c-{}@ent.test", &Uuid::new_v4().to_string()[..8]);
        let token2 = register_and_login(&app, etab2, &email2).await;
        let ref2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/competences")
                    .header("authorization", format!("Bearer {token2}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ref2.status(), StatusCode::OK);
        assert!(json_body(ref2).await.as_array().unwrap().is_empty());
        let hors = app
            .oneshot(
                Request::builder()
                    .uri(format!("/eleves/{eleve_id}/competences"))
                    .header("authorization", format!("Bearer {token2}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(hors.status(), StatusCode::NOT_FOUND);
    }

    /// Passkeys (WebAuthn) via HTTP : enrôlement authentifié, 2ᵉ facteur exigé
    /// à la connexion, session par assertion, gestion, garde de rôle et
    /// isolation multi-tenant. L'authentificateur est logiciel (SoftPasskey) :
    /// cryptographie WebAuthn réelle sur le vrai routeur axum.
    #[tokio::test]
    async fn api_passkeys_2fa_passwordless_et_isolation() {
        use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator};
        use webauthn_authenticator_rs::softpasskey::SoftPasskey;

        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let app = app(state);
        let etab = Uuid::new_v4();
        let etab2 = Uuid::new_v4();
        let email = format!("pk-{}@ent.local", Uuid::new_v4());
        let password = "DevLocalPass1!";
        let origin = Url::parse("http://localhost:3000").unwrap();

        // 1) Compte parent : rôle non privilégié (aucun droit admin).
        let token = register_and_login_role(&app, etab, &email, "parent").await;

        // Aucune passkey au départ (et /me le dit).
        let me = app
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
        assert_eq!(me.status(), StatusCode::OK);
        assert_eq!(json_body(me).await["passkey_count"], 0);

        // 2) Enrôlement (session requise) : options → authentificateur → finish.
        let start = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/passkey/register/start")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(start.status(), StatusCode::OK);
        let start_body = json_body(start).await;
        let reg_token = start_body["token"].as_str().unwrap().to_string();
        let ccr: ent_auth::CreationChallengeResponse =
            serde_json::from_value(start_body["options"].clone()).unwrap();

        let mut device = WebauthnAuthenticator::new(SoftPasskey::new(true));
        let registration = device
            .do_registration(origin.clone(), ccr)
            .expect("enregistrement WebAuthn logiciel");

        let finish = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/passkey/register/finish")
                    .header("authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "token": reg_token,
                            "credential": registration,
                            "name": "Pixel L2",
                            "current_password": password,
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(finish.status(), StatusCode::CREATED);
        let created = json_body(finish).await;
        assert_eq!(created["name"], "Pixel L2");
        let passkey_id = created["id"].as_str().unwrap().to_string();

        // La liste expose la passkey au propriétaire.
        let liste = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/auth/passkeys")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(liste.status(), StatusCode::OK);
        assert_eq!(json_body(liste).await.as_array().unwrap().len(), 1);

        // 3) Le mot de passe seul ne suffit plus : réponse 2FA **sans** session.
        let res = app
            .clone()
            .oneshot(login_request(etab, &email, password))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = json_body(res).await;
        assert_eq!(body["mfa_required"], true);
        assert!(body["token"].is_null(), "aucune session avant la passkey");
        let auth_token = body["passkey"]["token"].as_str().unwrap().to_string();
        let rcr: ent_auth::RequestChallengeResponse =
            serde_json::from_value(body["passkey"]["options"].clone()).unwrap();

        // Assertion → session.
        let assertion = device
            .do_authentication(origin.clone(), rcr)
            .expect("assertion WebAuthn logicielle");
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/passkey/login/finish")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "token": auth_token,
                            "credential": assertion,
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let session = json_body(res).await;
        let session_token = session["token"].as_str().unwrap().to_string();
        assert_eq!(session["etablissement_id"], etab.to_string());

        // /me reflète le 2ᵉ facteur actif.
        let me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/auth/me")
                    .header("authorization", format!("Bearer {session_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(json_body(me).await["passkey_count"], 1);

        // 4) Isolation : un non-membre d'un autre établissement ne peut rien.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/passkey/login/start")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "email": email,
                            "etablissement_id": etab2,
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        // Anti-énumération (M4) : même forme de réponse qu'un compte avec passkey
        // (200 + challenge factice), plus de 401 « aucune passkey ».
        assert_eq!(res.status(), StatusCode::OK);
        let faux = json_body(res).await;
        assert!(faux["token"].is_string() && faux["options"]["publicKey"]["challenge"].is_string());

        // 5) Garde de rôle : un parent ne réinitialise pas les passkeys d'autrui.
        let admin_email = format!("pkadm-{}@ent.local", Uuid::new_v4());
        let admin = register_and_login_role(&app, etab, &admin_email, "admin").await;
        let admin_me = json_body(
            app.clone()
                .oneshot(
                    Request::builder()
                        .uri("/auth/me")
                        .header("authorization", format!("Bearer {admin}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        let admin_id = admin_me["user_id"].as_str().unwrap().to_string();
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/auth/users/{admin_id}/passkeys"))
                    .header("authorization", format!("Bearer {session_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        // 6) Retrait par le propriétaire → le 2ᵉ facteur retombe.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/auth/passkeys/{passkey_id}"))
                    .header("authorization", format!("Bearer {session_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({"current_password": password})).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
        let res = app
            .clone()
            .oneshot(login_request(etab, &email, password))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = json_body(res).await;
        assert_eq!(body["mfa_required"], false);
        assert!(body["token"].is_string(), "session directe rétablie");

        // 7) Sans passkey, la réponse reste un 200 avec challenge factice (M4) ;
        //    l'étape `finish` le refusera comme tout jeton inconnu.
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/passkey/login/start")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "email": email,
                            "etablissement_id": etab,
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert!(json_body(res).await["token"].is_string());
    }

    /// SSO OIDC via HTTP : configuration validée par découverte (Direction /
    /// Admin seulement), vue publique minimale (ni issuer, ni client_id, ni
    /// secret), parcours PKCE + `nonce` jusqu'à la session, `state` à usage
    /// unique, refus des identités inconnues, isolation multi-tenant.
    #[tokio::test]
    async fn api_sso_oidc_connexion_et_isolation() {
        use ent_oidc::testing::{FakeOidcTransport, TestIdp};
        use std::sync::Arc;

        fn param(url: &str, key: &str) -> String {
            // Les valeurs OIDC du test (state, nonce, code_challenge) sont en
            // base64url : aucun échappement à décoder.
            url.split_once('?')
                .map(|(_, q)| q)
                .unwrap_or_default()
                .split('&')
                .filter_map(|kv| kv.split_once('='))
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| panic!("paramètre {key} absent de {url}"))
        }

        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let app = app(state.clone());
        let etab = Uuid::new_v4();
        let etab2 = Uuid::new_v4();

        let mut idp = TestIdp::new();
        idp.email = format!("sso-{}@ent.local", Uuid::new_v4());
        idp.subject = format!("sub-{}", Uuid::new_v4());
        let transport = Arc::new(FakeOidcTransport::new(idp.clone()));
        state.ent.set_oidc_transport(transport.clone());

        // Le compte local existe **déjà** : le SSO ne crée pas de compte.
        let direction = register_and_login_role(&app, etab, &idp.email, "direction").await;
        let prof_email = format!("prof-sso-{}@ent.local", Uuid::new_v4());
        let prof = register_and_login_role(&app, etab, &prof_email, "prof").await;
        let parent_email = format!("par-sso-{}@ent.local", Uuid::new_v4());
        let parent = register_and_login_role(&app, etab, &parent_email, "parent").await;

        // 1) Aucun fournisseur exposé tant que rien n'est configuré.
        let publics = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/auth/oidc/public?etablissement_id={etab}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(publics.status(), StatusCode::OK);
        assert!(json_body(publics).await.as_array().unwrap().is_empty());

        // 2) Configuration réservée à Direction / Admin.
        let upsert_body = |idp: &TestIdp| {
            serde_json::to_vec(&json!({
                "slug": "idp-test",
                "libelle": "Annuaire de l'établissement",
                "issuer": idp.issuer,
                "client_id": idp.client_id,
                "client_secret": idp.client_secret,
            }))
            .unwrap()
        };
        let refuse = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/auth/oidc/providers")
                    .header("authorization", format!("Bearer {prof}"))
                    .header("content-type", "application/json")
                    .body(Body::from(upsert_body(&idp)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(refuse.status(), StatusCode::FORBIDDEN);

        // Secret vide → refus 400 (config incomplète).
        let incomplet = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/auth/oidc/providers")
                    .header("authorization", format!("Bearer {direction}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "slug": "vide",
                            "libelle": "Vide",
                            "issuer": idp.issuer,
                            "client_id": idp.client_id,
                            "client_secret": "",
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(incomplet.status(), StatusCode::BAD_REQUEST);

        // Issuer injoignable → l'IdP ne répond pas : 502 (pas de config à moitié).
        let injoignable = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/auth/oidc/providers")
                    .header("authorization", format!("Bearer {direction}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "slug": "casse",
                            "libelle": "Casse",
                            "issuer": "https://idp-inexistant.test/oidc",
                            "client_id": "x",
                            "client_secret": "y",
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(injoignable.status(), StatusCode::BAD_GATEWAY);

        // Configuration valide : le secret ne ressort jamais dans la réponse.
        let ok = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/auth/oidc/providers")
                    .header("authorization", format!("Bearer {direction}"))
                    .header("content-type", "application/json")
                    .body(Body::from(upsert_body(&idp)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK);
        let dto = json_body(ok).await;
        assert_eq!(dto["slug"], "idp-test");
        assert_eq!(dto["issuer"], idp.issuer);
        assert_eq!(dto["actif"], true);
        assert!(dto.get("client_secret").is_none(), "secret jamais exposé");

        // 3) Vue publique : slug + libellé seulement.
        let publics = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/auth/oidc/public?etablissement_id={etab}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let publics = json_body(publics).await;
        let liste = publics.as_array().unwrap();
        assert_eq!(liste.len(), 1);
        assert_eq!(liste[0]["slug"], "idp-test");
        assert_eq!(liste[0]["libelle"], "Annuaire de l'établissement");
        assert!(liste[0].get("issuer").is_none() && liste[0].get("client_id").is_none());

        // 4) Parcours complet : start → (IdP) → complete → session.
        let start = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/oidc/idp-test/start")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({ "etablissement_id": etab })).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(start.status(), StatusCode::OK);
        // A7 : cookie de liaison au navigateur, exigé au retour.
        let cookie_liaison = start
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .expect("cookie de liaison posé au start")
            .to_string();
        let url = json_body(start).await["url"].as_str().unwrap().to_string();
        assert!(url.starts_with(&idp.authorization_endpoint()));
        assert!(url.contains("code_challenge_method=S256"));
        let state_param = param(&url, "state");
        transport.arm_nonce(&param(&url, "nonce"));
        transport.arm_code_challenge(&param(&url, "code_challenge"));

        let complete = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/oidc/complete")
                    .header("content-type", "application/json")
                    .header("cookie", &cookie_liaison)
                    .body(Body::from(
                        serde_json::to_vec(&json!({ "code": "code-1", "state": state_param }))
                            .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(complete.status(), StatusCode::OK);
        let session = json_body(complete).await;
        assert_eq!(session["mfa_required"], false);
        assert_eq!(session["etablissement_id"], etab.to_string());
        assert_eq!(session["role"], "direction");
        assert_eq!(session["email"], idp.email);
        assert!(session["token"].is_string(), "session ouverte après vérification");

        // 5) Le `state` est à usage unique : rejouer le même retour échoue.
        let replay = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/oidc/complete")
                    .header("content-type", "application/json")
                    .header("cookie", &cookie_liaison)
                    .body(Body::from(
                        serde_json::to_vec(&json!({ "code": "code-1", "state": state_param }))
                            .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(replay.status(), StatusCode::UNAUTHORIZED);

        // 6) Identité inconnue → refus : aucun compte créé à la volée.
        let mut inconnu = TestIdp::new();
        inconnu.email = format!("inconnu-{}@ent.local", Uuid::new_v4());
        inconnu.subject = format!("sub-inconnu-{}", Uuid::new_v4());
        let transport_inconnu = Arc::new(FakeOidcTransport::new(inconnu));
        state.ent.set_oidc_transport(transport_inconnu.clone());
        let start = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/oidc/idp-test/start")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({ "etablissement_id": etab })).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let url = json_body(start).await["url"].as_str().unwrap().to_string();
        transport_inconnu.arm_nonce(&param(&url, "nonce"));
        transport_inconnu.arm_code_challenge(&param(&url, "code_challenge"));
        let inconnu_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/oidc/complete")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "code": "code-2",
                            "state": param(&url, "state"),
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(inconnu_res.status(), StatusCode::UNAUTHORIZED);

        // 7) Isolation : le fournisseur n'existe que dans son établissement.
        let hors = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/oidc/idp-test/start")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({ "etablissement_id": etab2 })).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(hors.status(), StatusCode::NOT_FOUND);
        let publics2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/auth/oidc/public?etablissement_id={etab2}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(json_body(publics2).await.as_array().unwrap().is_empty());

        // 8) Gestion : le personnel liste, un parent est refusé.
        let staff_liste = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/auth/oidc/providers")
                    .header("authorization", format!("Bearer {prof}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(staff_liste.status(), StatusCode::OK);
        assert_eq!(json_body(staff_liste).await.as_array().unwrap().len(), 1);
        let parent_liste = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/auth/oidc/providers")
                    .header("authorization", format!("Bearer {parent}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(parent_liste.status(), StatusCode::FORBIDDEN);

        // 9) La page de retour SSO est servie par l'UI (redirection de l'IdP).
        let callback = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/auth/oidc/callback?code=x&state=y")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(callback.status(), StatusCode::OK);

        // 10) Suppression : réservée aussi, puis le fournisseur disparaît.
        let del_refuse = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/auth/oidc/providers/idp-test")
                    .header("authorization", format!("Bearer {prof}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(del_refuse.status(), StatusCode::FORBIDDEN);
        let del = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/auth/oidc/providers/idp-test")
                    .header("authorization", format!("Bearer {direction}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(del.status(), StatusCode::NO_CONTENT);
        let apres = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/oidc/idp-test/start")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({ "etablissement_id": etab })).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(apres.status(), StatusCode::NOT_FOUND);
    }

    /// Corps texte (métriques / Prometheus).
    async fn body_text(res: axum::response::Response) -> String {
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8_lossy(&bytes).to_string()
    }

    /// Valeur d'une série dont la ligne commence par `prefixe`.
    fn valeur_metrique(texte: &str, prefixe: &str) -> f64 {
        texte
            .lines()
            .filter(|l| l.starts_with(prefixe))
            .filter_map(|l| l.rsplit(' ').next())
            .filter_map(|v| v.parse::<f64>().ok())
            .next()
            .unwrap_or_else(|| panic!("métrique absente: {prefixe}\n---\n{texte}"))
    }

    fn get(uri: &str) -> axum::http::Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    fn get_bearer(uri: &str, jeton: &str) -> axum::http::Request<Body> {
        Request::builder()
            .uri(uri)
            .header("authorization", format!("Bearer {jeton}"))
            .body(Body::empty())
            .unwrap()
    }

    fn post_bearer(uri: &str, jeton: &str, corps: &str) -> axum::http::Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {jeton}"))
            .body(Body::from(corps.to_string()))
            .unwrap()
    }

    /// Observabilité : `/metrics` (désactivé / ouvert / jeton), comptage par
    /// **gabarit** de route, histogramme de latence, sondes base et `ent_info`.
    #[tokio::test]
    async fn api_metriques_et_comptage_par_gabarit() {
        let Some(mut state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };

        // 1) Par défaut : aucune surface exposée (404, pas 403 : on ne révèle
        //    même pas l'existence de l'endpoint).
        state.metrics_acces = obs::MetricsAcces::Desactive;
        let app = crate::app(state.clone());
        let ferme = app.clone().oneshot(get("/metrics")).await.unwrap();
        assert_eq!(ferme.status(), StatusCode::NOT_FOUND);

        // 2) Ouvert (scrape local) : le texte Prometheus est servi.
        state.metrics_acces = obs::MetricsAcces::Ouvert;
        let app = crate::app(state.clone());

        let health = app.clone().oneshot(get("/health")).await.unwrap();
        assert_eq!(health.status(), StatusCode::OK);
        let health_json = json_body(health).await;
        assert_eq!(
            health_json["migration_tag"].as_str(),
            Some(ent_db::MIGRATION_TAG)
        );

        // Route paramétrée sans session → 401, mais le gabarit doit apparaître.
        let uuid_brut = Uuid::new_v4();
        let sans_session = app
            .clone()
            .oneshot(get(&format!("/personnel/{uuid_brut}")))
            .await
            .unwrap();
        assert_eq!(sans_session.status(), StatusCode::UNAUTHORIZED);

        let metriques = app.clone().oneshot(get("/metrics")).await.unwrap();
        assert_eq!(metriques.status(), StatusCode::OK);
        assert!(metriques
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .starts_with("text/plain"));
        let texte = body_text(metriques).await;

        assert!(texte.contains("# TYPE ent_http_requetes_total counter"), "{texte}");
        assert!(
            valeur_metrique(
                &texte,
                "ent_http_requetes_total{methode=\"GET\",route=\"/health\",statut=\"200\"} "
            ) >= 1.0,
            "{texte}"
        );
        // Cardinalité bornée : gabarit, jamais l'identifiant brut.
        assert!(texte.contains("route=\"/personnel/{id}\""), "{texte}");
        assert!(!texte.contains(&uuid_brut.to_string()), "{texte}");
        assert!(texte.contains("ent_http_duree_secondes_count"), "{texte}");
        // La jauge « en cours » compte la requête /metrics elle-même.
        assert!(valeur_metrique(&texte, "ent_http_requetes_en_cours ") >= 1.0, "{texte}");

        // Sondes base + info de build.
        assert!(texte.contains("ent_db_pool_connexions{etat=\"maximum\"} 5"), "{texte}");
        assert!(valeur_metrique(&texte, "ent_sessions_actives ") >= 0.0, "{texte}");
        assert!(valeur_metrique(&texte, "ent_sondes_erreur ") == 0.0, "{texte}");
        assert!(
            texte.contains(&format!("migration_tag=\"{}\"", ent_db::MIGRATION_TAG)),
            "{texte}"
        );

        // 3) Jeton : 401 sans / avec un mauvais jeton, 200 avec le bon.
        state.metrics_acces = obs::MetricsAcces::Jeton("jeton-obs-l2".into());
        let app = crate::app(state.clone());
        let anonyme = app.clone().oneshot(get("/metrics")).await.unwrap();
        assert_eq!(anonyme.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            anonyme
                .headers()
                .get("www-authenticate")
                .and_then(|v| v.to_str().ok()),
            Some("Bearer")
        );
        let mauvais = app
            .clone()
            .oneshot(get_bearer("/metrics", "pas-le-bon"))
            .await
            .unwrap();
        assert_eq!(mauvais.status(), StatusCode::UNAUTHORIZED);
        let bon = app
            .clone()
            .oneshot(get_bearer("/metrics", "jeton-obs-l2"))
            .await
            .unwrap();
        assert_eq!(bon.status(), StatusCode::OK);
        assert!(body_text(bon).await.contains("ent_info"));
    }

    /// PRA : inventaire d'intégrité **par établissement**, détection de dérive
    /// (preuve de fidélité d'une restauration) et cloisonnement strict.
    #[tokio::test]
    async fn api_pra_inventaire_integrite_et_isolation() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let app = app(state);
        let etab = Uuid::new_v4();
        let etab2 = Uuid::new_v4();

        let direction =
            register_and_login_role(&app, etab, &format!("dir-pra-{}@ent.local", Uuid::new_v4()), "direction")
                .await;
        let prof =
            register_and_login_role(&app, etab, &format!("prof-pra-{}@ent.local", Uuid::new_v4()), "prof")
                .await;
        let parent =
            register_and_login_role(&app, etab, &format!("par-pra-{}@ent.local", Uuid::new_v4()), "parent")
                .await;

        // 1) Accès : anonyme 401, professeur / parent 403, direction 200.
        assert_eq!(
            app.clone()
                .oneshot(get("/pra/inventaire"))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        for (jeton, attendu) in [(&prof, StatusCode::FORBIDDEN), (&parent, StatusCode::FORBIDDEN)] {
            let res = app
                .clone()
                .oneshot(get_bearer("/pra/inventaire", jeton))
                .await
                .unwrap();
            assert_eq!(res.status(), attendu);
        }
        let res = app
            .clone()
            .oneshot(get_bearer("/pra/inventaire", &direction))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let inv1 = json_body(res).await;

        assert_eq!(inv1["version"].as_u64(), Some(1));
        assert_eq!(inv1["portee"].as_str(), Some(etab.to_string().as_str()));
        assert_eq!(inv1["migration_tag"].as_str(), Some(ent_db::MIGRATION_TAG));
        let tables = inv1["tables"].as_array().unwrap();
        assert!(tables.len() > 20, "inventaire trop maigre: {}", tables.len());
        assert!(tables.iter().any(|t| t["nom"] == "ent_eleve"));
        // Les tables volatiles ne sont jamais exposées ni sauvegardées.
        for volatile in ent_db::TABLES_VOLATILES {
            assert!(
                !tables.iter().any(|t| t["nom"].as_str() == Some(volatile)),
                "{volatile} ne doit pas figurer à l'inventaire"
            );
        }
        let eleve_inv = |inv: &Value| -> (i64, String) {
            let t = inv["tables"]
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["nom"] == "ent_eleve")
                .expect("ent_eleve");
            (
                t["lignes"].as_i64().unwrap(),
                t["digest"].as_str().unwrap().to_string(),
            )
        };
        assert_eq!(eleve_inv(&inv1).0, 0, "établissement neuf");

        // 2) Un élève est créé après l'inventaire → la vérification détecte la
        //    dérive sur la bonne table (c'est ce qui prouve une restauration).
        let created = app
            .clone()
            .oneshot(post_bearer(
                "/eleves",
                &direction,
                r#"{"nom":"Pra","prenom":"Integrite","classe":"4B"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);

        let corps = serde_json::to_string(&inv1).unwrap();
        let res = app
            .clone()
            .oneshot(post_bearer("/pra/inventaire/verifier", &direction, &corps))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let rapport = json_body(res).await;
        assert_eq!(rapport["conforme"], Value::Bool(false));
        let ecarts = rapport["ecarts"].as_array().unwrap();
        let ecart = ecarts
            .iter()
            .find(|e| e["nom"] == "ent_eleve")
            .expect("écart ent_eleve attendu");
        assert_eq!(ecart["lignes_attendues"].as_i64(), Some(0));
        assert_eq!(ecart["lignes_obtenues"].as_i64(), Some(1));

        // 3) Nouvel inventaire → vérification conforme (restauration fidèle).
        let inv2 = json_body(
            app.clone()
                .oneshot(get_bearer("/pra/inventaire", &direction))
                .await
                .unwrap(),
        )
        .await;
        let res = app
            .clone()
            .oneshot(post_bearer(
                "/pra/inventaire/verifier",
                &direction,
                &serde_json::to_string(&inv2).unwrap(),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let rapport2 = json_body(res).await;
        assert_eq!(rapport2["conforme"], Value::Bool(true), "{rapport2}");
        assert!(rapport2["ecarts"].as_array().unwrap().is_empty());
        assert_eq!(
            rapport2["tables_conformes"].as_u64(),
            Some(inv2["tables"].as_array().unwrap().len() as u64)
        );

        // 4) Cloisonnement : un autre établissement ne peut pas servir de
        //    référence, et ses écritures ne changent pas notre inventaire.
        let direction2 = register_and_login_role(
            &app,
            etab2,
            &format!("dir2-pra-{}@ent.local", Uuid::new_v4()),
            "direction",
        )
        .await;
        let res = app
            .clone()
            .oneshot(post_bearer(
                "/pra/inventaire/verifier",
                &direction2,
                &serde_json::to_string(&inv2).unwrap(),
            ))
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            StatusCode::FORBIDDEN,
            "un inventaire d'un autre établissement doit être refusé"
        );

        let created2 = app
            .clone()
            .oneshot(post_bearer(
                "/eleves",
                &direction2,
                r#"{"nom":"Autre","prenom":"Etab","classe":"5A"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(created2.status(), StatusCode::CREATED);

        let inv3 = json_body(
            app.clone()
                .oneshot(get_bearer("/pra/inventaire", &direction))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(
            eleve_inv(&inv3),
            eleve_inv(&inv2),
            "les écritures d'un autre établissement ne doivent pas bouger l'inventaire"
        );
    }

    /// L1 — `storage_key` envoyé par un client : ignoré (201), jamais utilisé pour lire un objet.
    #[tokio::test]
    async fn api_depot_document_ignore_storage_key_client() {
        use base64::Engine as _;
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let cast = state.ent.seed_labo_cast().await.expect("cast");
        let ent = state.ent.clone();
        let app = app(state);
        let lea = cast.eleve_lea_id;
        let etab = cast.etablissement_id;
        let b64 = base64::engine::general_purpose::STANDARD;

        let post = |token: String, body: Value| {
            let app = app.clone();
            async move {
                app.oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!("/eleves/{lea}/documents"))
                        .header("authorization", format!("Bearer {token}"))
                        .header("content-type", "application/json")
                        .body(Body::from(body.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap()
            }
        };
        let get = |token: String, id: Uuid| {
            let app = app.clone();
            async move {
                app.oneshot(
                    Request::builder()
                        .uri(format!("/documents/{id}/fichier"))
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap()
            }
        };

        // Le personnel dépose un document santé avec contenu.
        let r = post(
            cast.admin.token.clone(),
            json!({"titre":"Ordonnance L1","tiroir":"sante","confidentialite":"sante",
                   "mime_type":"application/pdf","contenu_base64": b64.encode(b"%PDF-1.4 SECRET-SANTE-API")}),
        )
        .await;
        assert_eq!(r.status(), StatusCode::CREATED);
        let sante_id = Uuid::parse_str(json_body(r).await["id"].as_str().unwrap()).unwrap();
        let sante_key = format!("{etab}/documents/{sante_id}.pdf");

        // Le parent envoie storage_key vers ce fichier : 201, champ ignoré, rien à lire.
        for key in [sante_key.as_str(), "C:/Windows/win.ini", "../../etc/passwd"] {
            let r = post(
                cast.parent_lea.token.clone(),
                json!({"titre":"Piège","tiroir":"sante","confidentialite":"sante","storage_key": key}),
            )
            .await;
            assert_eq!(r.status(), StatusCode::CREATED, "{key}");
            let id = Uuid::parse_str(json_body(r).await["id"].as_str().unwrap()).unwrap();
            let docs = ent
                .list_documents_meta_eleve_for_actor(etab, cast.admin.user_id, "admin", lea)
                .await
                .unwrap();
            let doc = docs.into_iter().map(|(d, _)| d).find(|d| d.id == id).expect("doc");
            assert_eq!(doc.storage_key, None);
            let dl = get(cast.parent_lea.token.clone(), id).await;
            assert_ne!(dl.status(), StatusCode::OK, "pas de fichier sans clé");
        }
        // Le parent ne lit toujours pas le document santé.
        assert_eq!(
            get(cast.parent_lea.token.clone(), sante_id).await.status(),
            StatusCode::FORBIDDEN
        );

        // Ligne héritée avec clé hostile : le téléchargement ne sert que le stub.
        for key in [sante_key.clone(), "C:/Windows/win.ini".to_string()] {
            let doc = ent
                .deposer_document(domain_eleve::DeposerDocument {
                    etablissement_id: etab,
                    eleve_id: lea,
                    tiroir: domain_eleve::DocTiroir::Famille,
                    titre: "Héritée".into(),
                    code: None,
                    mime_type: Some("application/pdf".into()),
                    storage_key: Some(key),
                    confidentialite: domain_eleve::DocConfidentialite::Standard,
                    actor_user_id: cast.admin.user_id,
                })
                .await
                .unwrap();
            let dl = get(cast.parent_lea.token.clone(), doc.id).await;
            assert_eq!(dl.status(), StatusCode::OK);
            let bytes = dl.into_body().collect().await.unwrap().to_bytes();
            let txt = String::from_utf8_lossy(&bytes).to_string();
            assert!(txt.starts_with("Document ENT (lab)"), "{txt}");
            assert!(!txt.contains("SECRET"));
        }

        // Dépôt légitime avec contenu : relu.
        let r = post(
            cast.parent_lea.token.clone(),
            json!({"titre":"Mon fichier","tiroir":"famille","confidentialite":"standard",
                   "mime_type":"application/pdf","contenu_base64": b64.encode(b"%PDF-ok")}),
        )
        .await;
        assert_eq!(r.status(), StatusCode::CREATED);
        let id = Uuid::parse_str(json_body(r).await["id"].as_str().unwrap()).unwrap();
        let dl = get(cast.parent_lea.token.clone(), id).await;
        assert_eq!(dl.status(), StatusCode::OK);
        assert_eq!(
            dl.into_body().collect().await.unwrap().to_bytes().as_ref(),
            b"%PDF-ok"
        );

        // Le dossier n'expose jamais de clé objet (documents avec clé existants).
        for token in [cast.admin.token.clone(), cast.parent_lea.token.clone()] {
            let res = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/eleves/{lea}/dossier"))
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK);
            let v = json_body(res).await;
            let docs = v["documents"].as_array().expect("documents");
            assert!(!docs.is_empty());
            for d in docs {
                assert!(d["storage_key"].is_null(), "storage_key exposé : {d}");
            }
            assert!(v["eleve"]["photo_key"].is_null());
        }
    }

    /// L2 (I1/I2) : niveau d'identité unique par rôle sur toutes les routes
    /// qui renvoient une identité d'élève ; pas d'oracle de badge.
    #[tokio::test]
    async fn api_identite_eleve_niveaux_par_role() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2 identité: ENT_DATABASE_URL");
            return;
        };
        let cast = state.ent.seed_labo_cast().await.expect("cast");
        let app = app(state);
        let jour = Utc::now().date_naive().to_string();
        let lire = |uri: String, tok: String| {
            let app = app.clone();
            async move {
                let r = app.oneshot(get_bearer(&uri, &tok)).await.unwrap();
                assert_eq!(r.status(), StatusCode::OK, "{uri}");
                json_body(r).await
            }
        };
        const SENSIBLES: [&str; 8] = [
            "ine",
            "numero_badge",
            "regime",
            "mef",
            "motif_sortie",
            "date_entree",
            "date_sortie",
            "redoublant",
        ];
        let admin = lire(
            format!("/eleves/{}", cast.eleve_lea_id),
            cast.administratif.token.clone(),
        )
        .await;
        let classe = admin["classe"].as_str().expect("classe Léa").to_string();
        let badge = admin["numero_badge"].as_str().expect("badge Léa").to_string();
        assert!(!admin["ine"].is_null() && !admin["date_naissance"].is_null());

        // Extrait l'élève Léa d'une réponse selon la route.
        let uri_eleve = format!("/eleves/{}", cast.eleve_lea_id);
        let uri_dossier = format!("/eleves/{}/dossier", cast.eleve_lea_id);
        let uri_effectif = format!("/classes/{classe}/effectif?jour={jour}");
        let uri_search = "/eleves/search?q=a".to_string();
        let extraire = |route: usize, v: &Value, id: Uuid| -> Value {
            match route {
                0 => v.clone(),
                1 => v["eleve"].clone(),
                _ => v["eleves"]
                    .as_array()
                    .or_else(|| v.as_array())
                    .and_then(|a| a.iter().find(|e| e["id"] == id.to_string()))
                    .cloned()
                    .unwrap_or(Value::Null),
            }
        };
        let routes = [&uri_eleve, &uri_dossier, &uri_effectif, &uri_search];
        // (jeton, sexe/naissance attendus, champs sensibles attendus)
        let roles: [(&str, &String, bool, bool); 6] = [
            ("administratif", &cast.administratif.token, true, true),
            ("prof", &cast.prof.token, false, false),
            ("compta", &cast.compta.token, false, false),
            ("psy", &cast.psys[0].token, false, false),
            ("infirmerie", &cast.infirmieres[0].token, true, false),
            ("cpe", &cast.cpe.token, false, false),
        ];
        for (nom, tok, sante, complet) in roles {
            for (i, uri) in routes.iter().enumerate() {
                let v = lire((*uri).clone(), tok.clone()).await;
                let e = extraire(if i == 0 { 0 } else if i == 1 { 1 } else { 2 }, &v, cast.eleve_lea_id);
                if e.is_null() {
                    // La recherche « a » peut ne pas ramener Léa : on contrôle alors
                    // tous les éléments renvoyés.
                    assert_eq!(i, 3, "{nom} {uri}: élève Léa absent");
                }
                let liste: Vec<Value> = if i == 3 {
                    v.as_array().cloned().unwrap_or_default()
                } else {
                    vec![e]
                };
                for e in liste {
                    for champ in SENSIBLES {
                        let vide = e[champ].is_null() || e[champ] == false;
                        assert!(complet || vide, "{nom} {uri}: champ {champ} fuit");
                    }
                    if !complet && !sante {
                        for champ in ["sexe", "date_naissance"] {
                            assert!(e[champ].is_null(), "{nom} {uri}: {champ} fuit");
                        }
                    }
                }
            }
        }
        // Infirmerie : sexe / naissance conservés sur la fiche de Léa.
        let inf = lire(uri_eleve.clone(), cast.infirmieres[0].token.clone()).await;
        assert!(!inf["date_naissance"].is_null(), "infirmerie sans naissance");

        // Pas d'oracle badge : Prof / Compta / Psy / Infirmerie / CPE → aucun résultat.
        for tok in [
            &cast.prof.token,
            &cast.compta.token,
            &cast.psys[0].token,
            &cast.infirmieres[0].token,
            &cast.cpe.token,
        ] {
            let r = lire(format!("/eleves/search?q={badge}"), tok.clone()).await;
            assert!(r.as_array().unwrap().is_empty(), "badge interrogeable");
        }
        let r = lire(
            format!("/eleves/search?q={badge}"),
            cast.administratif.token.clone(),
        )
        .await;
        assert!(!r.as_array().unwrap().is_empty(), "gestionnaire sans recherche badge");

        // Famille : inchangé (identité complète de son propre enfant).
        let p = lire(uri_eleve.clone(), cast.parent_lea.token.clone()).await;
        assert!(!p["ine"].is_null() && !p["numero_badge"].is_null());
    }

    /// Relecture finale (I1/I3/M1/M8) : le dossier est minimisé en entier (âge,
    /// périodes régime / MEF, années de scolarité, `redoublant` null) ; la
    /// recherche rejette l'octet NUL (400) et traite `%` comme un littéral.
    #[tokio::test]
    async fn api_dossier_minimisation_complete_et_recherche() {
        let Some(state) = state_local().await else {
            eprintln!("skip dossier minimisation: ENT_DATABASE_URL");
            return;
        };
        let cast = state.ent.seed_labo_cast().await.expect("cast");
        let app = app(state);
        let lire = |uri: String, tok: String| {
            let app = app.clone();
            async move {
                let r = app.oneshot(get_bearer(&uri, &tok)).await.unwrap();
                assert_eq!(r.status(), StatusCode::OK, "{uri}");
                json_body(r).await
            }
        };
        let uri = format!("/eleves/{}/dossier", cast.eleve_lea_id);
        let adm = lire(uri.clone(), cast.administratif.token.clone()).await;
        // Administratif : dossier complet (âge, redoublant booléen).
        assert!(!adm["eleve"]["date_naissance"].is_null());
        assert!(adm["age_ans"].is_number(), "administratif sans age_ans");
        assert!(adm["eleve"]["redoublant"].is_boolean());

        for (nom, tok, sante) in [
            ("prof", &cast.prof.token, false),
            ("compta", &cast.compta.token, false),
            ("psy", &cast.psys[0].token, false),
            ("infirmerie", &cast.infirmieres[0].token, true),
        ] {
            let d = lire(uri.clone(), tok.clone()).await;
            if sante {
                assert!(d["age_ans"].is_number(), "{nom}: age_ans perdu");
            } else {
                assert!(d["age_ans"].is_null(), "{nom}: age_ans fuit");
            }
            assert!(d["eleve"]["redoublant"].is_null(), "{nom}: redoublant non null");
            for p in d["periodes"].as_array().expect("periodes") {
                let k = p["kind"].as_str().unwrap_or_default();
                assert!(k != "regime" && k != "mef", "{nom}: période {k} fuit");
            }
            for a in d["continuum"].as_array().expect("continuum") {
                for champ in ["mef", "regime", "redoublement_possible"] {
                    assert!(a[champ].is_null(), "{nom}: continuum.{champ} fuit");
                }
            }
        }
        // Compta garde les sections de facturation (repas) malgré la minimisation.
        let cpt = lire(uri.clone(), cast.compta.token.clone()).await;
        assert_eq!(cpt["sections"]["repas"], true);

        // M1 : octet NUL → 400 (et non 500).
        let r = app
            .clone()
            .oneshot(get_bearer("/eleves/search?q=%00", &cast.administratif.token))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        // M8 : `%` et `_` sont littéraux, pas des jokers.
        for q in ["%25", "_", "%5C"] {
            let r = lire(format!("/eleves/search?q={q}"), cast.administratif.token.clone()).await;
            assert!(r.as_array().unwrap().is_empty(), "q={q} agit comme joker");
        }
    }

    /// L2 : annuaire élèves, foyers, dossier Compta, journal / points VS —
    /// minimisation des données personnelles.
    #[tokio::test]
    async fn api_annuaire_foyers_dossier_minimisation() {
        let Some(state) = state_local().await else {
            eprintln!("skip L2: ENT_DATABASE_URL");
            return;
        };
        let cast = state.ent.seed_labo_cast().await.expect("cast");
        let app = app(state);
        let jour = Utc::now().date_naive().to_string();
        let statut = |req: Request<Body>| {
            let app = app.clone();
            async move { app.oneshot(req).await.unwrap() }
        };

        // search / labels : refusés au parent et à l'élève.
        let eleve_tok = cast
            .eleves
            .iter()
            .find_map(|e| e.compte.as_ref().map(|c| c.token.clone()))
            .expect("compte élève");
        for tok in [&cast.parent_lea.token, &eleve_tok] {
            assert_eq!(
                statut(get_bearer("/eleves/search?q=a", tok)).await.status(),
                StatusCode::FORBIDDEN
            );
            assert_eq!(
                statut(get_bearer(
                    &format!("/eleves/labels?ids={}", cast.eleve_lea_id),
                    tok
                ))
                .await
                .status(),
                StatusCode::FORBIDDEN
            );
        }

        // Personnel : gestionnaire = identité complète, autres = DTO minimisé.
        let adm = statut(get_bearer("/eleves/search?q=Harry", &cast.administratif.token)).await;
        assert_eq!(adm.status(), StatusCode::OK);
        let adm = json_body(adm).await;
        assert!(adm
            .as_array()
            .unwrap()
            .iter()
            .any(|e| !e["date_naissance"].is_null() || !e["ine"].is_null()));
        // (l'infirmerie, qui garde sexe / naissance, est couverte par api_identite_eleve_niveaux_par_role)
        for tok in [
            &cast.prof.token,
            &cast.compta.token,
            &cast.psys[0].token,
        ] {
            let r = statut(get_bearer("/eleves/search?q=Harry", tok)).await;
            assert_eq!(r.status(), StatusCode::OK);
            let r = json_body(r).await;
            let arr = r.as_array().unwrap();
            assert!(!arr.is_empty());
            for e in arr {
                for champ in [
                    "ine",
                    "sexe",
                    "date_naissance",
                    "numero_badge",
                    "regime",
                    "mef",
                    "motif_sortie",
                ] {
                    assert!(e[champ].is_null(), "{champ} fuit dans la recherche");
                }
            }
        }

        // absent : personnel, parent lié, mais pas un parent hors lien.
        let url_absent = |id: Uuid| format!("/eleves/{id}/absent?jour={jour}");
        assert_eq!(
            statut(get_bearer(&url_absent(cast.eleve_lea_id), &cast.prof.token))
                .await
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            statut(get_bearer(
                &url_absent(cast.eleve_lea_id),
                &cast.parent_lea.token
            ))
            .await
            .status(),
            StatusCode::OK
        );
        assert_eq!(
            statut(get_bearer(
                &url_absent(cast.eleve_tom_id),
                &cast.parent_lea.token
            ))
            .await
            .status(),
            StatusCode::FORBIDDEN
        );

        // Foyers : Administratif / Compta / CPE oui ; Prof / Infirmerie / Psy non.
        for tok in [&cast.administratif.token, &cast.compta.token, &cast.cpe.token] {
            assert_eq!(
                statut(get_bearer("/foyers", tok)).await.status(),
                StatusCode::OK
            );
            assert_eq!(
                statut(get_bearer(&format!("/foyers/{}", cast.foyer_lea_id), tok))
                    .await
                    .status(),
                StatusCode::OK
            );
        }
        for tok in [
            &cast.prof.token,
            &cast.infirmieres[0].token,
            &cast.psys[0].token,
        ] {
            assert_eq!(
                statut(get_bearer("/foyers", tok)).await.status(),
                StatusCode::FORBIDDEN
            );
            assert_eq!(
                statut(get_bearer(&format!("/foyers/{}", cast.foyer_lea_id), tok))
                    .await
                    .status(),
                StatusCode::FORBIDDEN
            );
        }

        // Dossier Compta : ni notes, ni sanctions, incidents, carnet.
        let d = statut(get_bearer(
            &format!("/eleves/{}/dossier", cast.eleve_lea_id),
            &cast.compta.token,
        ))
        .await;
        assert_eq!(d.status(), StatusCode::OK);
        let d = json_body(d).await;
        for champ in ["notes", "sanctions", "incidents", "carnet", "documents", "dispositifs"] {
            if let Some(a) = d[champ].as_array() {
                assert!(a.is_empty(), "{champ} visible par Compta");
            }
        }
        assert_eq!(d["sections"]["notes"], false);
        assert_eq!(d["sections"]["vs"], false);

        // journal / points-vs : parent sans peut_voir_vs (Draco) refusé ; avec (Harry) accepté.
        for chemin in ["journal", "points-vs"] {
            assert_eq!(
                statut(get_bearer(
                    &format!("/eleves/{}/{chemin}", cast.eleve_tom_id),
                    &cast.parent_tom.token
                ))
                .await
                .status(),
                StatusCode::FORBIDDEN,
                "{chemin}"
            );
            assert_eq!(
                statut(get_bearer(
                    &format!("/eleves/{}/{chemin}", cast.eleve_lea_id),
                    &cast.parent_lea.token
                ))
                .await
                .status(),
                StatusCode::OK,
                "{chemin}"
            );
        }
    }
}

/// Preuves de la séparation labo / production (L4) : compilés SANS la feature.
#[cfg(all(test, not(feature = "labo")))]
mod sans_labo {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn state_local() -> Option<AppState> {
        let url = std::env::var("ENT_DATABASE_URL")
            .ok()
            .filter(|u| !u.trim().is_empty())?;
        let mut ent = PgEnt::connect(&url).await.ok()?;
        ent.migrate().await.ok()?;
        ent.install_brain_p0();
        Some(AppState::nouveau(ent))
    }

    fn post(uri: &str, corps: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(corps.to_string()))
            .unwrap()
    }

    /// Rien n'est enregistré sans la feature, quel que soit `ENT_ENV`.
    #[tokio::test]
    async fn routes_labo_et_register_absentes_404() {
        let Some(state) = state_local().await else {
            eprintln!("skip L4: ENT_DATABASE_URL");
            return;
        };
        let app = app(state);
        let r = app.clone().oneshot(post("/labo/bootstrap", "{}")).await.unwrap();
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        let corps = format!(
            r#"{{"email":"x@example.test","password":"MotDePasse-Test-1","etablissement_id":"{}","role":"admin"}}"#,
            uuid::Uuid::new_v4()
        );
        let r = app.oneshot(post("/auth/register", &corps)).await.unwrap();
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn page_login_sans_mot_de_passe_public() {
        let Some(state) = state_local().await else {
            eprintln!("skip L4: ENT_DATABASE_URL");
            return;
        };
        let app = app(state);
        let r = app
            .oneshot(Request::builder().uri("/login").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let octets = r.into_body().collect().await.unwrap().to_bytes();
        let html = String::from_utf8_lossy(&octets);
        assert!(!html.contains(&["DevLocal", "Pass1!"].concat()));
        assert!(!html.contains("/labo/bootstrap"));
        assert!(html.contains("loginForm"));
    }
}
