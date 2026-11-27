//! Serveur du prototype.
//!
//! Compose sur **une seule origine** :
//! - le routeur `ent_api::app(state)` existant, utilisé tel quel (auth, droits, API,
//!   applis mobiles en Bearer) ;
//! - le relais de session par cookie `HttpOnly` (`ent-web-session`, A6) sous
//!   `/web/session/…` et `/web/api/…` ;
//! - les pages Leptos sous `/web/…`, les fonctions serveur sous `/api/…` et les
//!   fichiers du navigateur (WebAssembly, CSS) sous `/pkg/…`.
//!
//! Même origine ⇒ pas de CORS. Le navigateur n'a plus aucun jeton (ni `localStorage`, ni
//! `sessionStorage`) : le cookie de session est `HttpOnly`.

// Le serveur calcule la disposition des types de vues de toute l'application :
// même limite que `ent-web-app` (voir docs/FRONTEND_LEPTOS.md §8).
#![recursion_limit = "512"]

#[cfg(feature = "labo")]
pub mod labo;

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::extract::Request;
use axum::http::{header, request::Parts, HeaderValue};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::Router;
use ent_web_app::{shell, App};
use leptos::nonce::Nonce;
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

/// Options Leptos : variables `LEPTOS_*` (posées par cargo-leptos ou `scripts/env.sh`),
/// sinon valeurs par défaut du prototype.
pub fn options_leptos() -> LeptosOptions {
    if let Ok(conf) = leptos::config::get_configuration(None) {
        return conf.leptos_options;
    }
    let site_root = std::env::var("LEPTOS_SITE_ROOT").unwrap_or_else(|_| "target/site".into());
    let addr: SocketAddr = std::env::var("LEPTOS_SITE_ADDR")
        .ok()
        .and_then(|a| a.parse().ok())
        .unwrap_or_else(|| "127.0.0.1:8090".parse().unwrap());
    LeptosOptions::builder()
        .output_name("ent-web")
        .site_root(site_root)
        .site_pkg_dir("pkg")
        .site_addr(addr)
        .build()
}

/// Nonce CSP de la requête en cours, posé par `csp_middleware` dans les extensions.
#[derive(Clone)]
struct NonceCsp(Nonce);

/// Politique de sécurité du contenu des pages `/web` et de `/pkg`, pour un nonce donné.
///
/// Scripts : uniquement `'self'` (paquet WebAssembly/JS) et les scripts d'amorçage de
/// l'hydratation portant le nonce de la requête ; `'wasm-unsafe-eval'` pour instancier le
/// wasm. Aucun `'unsafe-inline'`. Le jeton de session est dans un
/// cookie `HttpOnly` (A6) : le jeton n'est plus lisible par un script, et cette CSP limite
/// en plus l'impact d'une injection.
pub fn politique_csp(nonce: &str) -> String {
    format!(
        "default-src 'self'; script-src 'self' 'nonce-{nonce}' 'wasm-unsafe-eval'; \
style-src 'self'; img-src 'self' data:; connect-src 'self'; object-src 'none'; \
base-uri 'none'; frame-ancestors 'self'; form-action 'self'"
    )
}

/// Génère un nonce aléatoire par requête (128 bits, `leptos::nonce`), le met à disposition
/// du rendu Leptos et pose l'en-tête `Content-Security-Policy` correspondant.
async fn csp_middleware(mut req: Request, next: Next) -> Response {
    let nonce = Nonce::new();
    let csp = politique_csp(&nonce);
    req.extensions_mut().insert(NonceCsp(nonce));
    let mut rep = next.run(req).await;
    if let Ok(v) = HeaderValue::from_str(&csp) {
        rep.headers_mut().insert(header::CONTENT_SECURITY_POLICY, v);
    }
    rep
}

/// `Cache-Control: no-store` + `Pragma: no-cache` sur les pages et fonctions serveur ;
/// le paquet statique (`/pkg/…` : wasm, JS, CSS versionnés) garde son cache.
async fn sans_cache_hors_pkg(
    axum::extract::State(prefixe_pkg): axum::extract::State<String>,
    req: Request,
    next: Next,
) -> Response {
    let statique = req.uri().path().starts_with(prefixe_pkg.as_str());
    let mut rep = next.run(req).await;
    if !statique {
        let h = rep.headers_mut();
        h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        h.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    }
    rep
}

/// Remplace le nonce que `leptos_axum` génère lui-même par celui du middleware, pour que
/// l'en-tête CSP et les balises `<script nonce=…>` soient cohérents.
fn fournir_nonce_de_la_requete() {
    if let Some(NonceCsp(n)) =
        use_context::<Parts>().and_then(|p| p.extensions.get::<NonceCsp>().cloned())
    {
        provide_context(n);
    }
}

/// Routes du frontend Leptos (pages SSR + fonctions serveur + fichiers statiques).
pub fn routeur_web(options: LeptosOptions) -> Router {
    let routes = generate_route_list(App);
    let pkg_dir = PathBuf::from(options.site_root.as_ref()).join(options.site_pkg_dir.as_ref());
    let pkg_url = format!("/{}", options.site_pkg_dir);
    Router::new()
        .leptos_routes_with_context(&options, routes, fournir_nonce_de_la_requete, {
            let options = options.clone();
            move || shell(options.clone())
        })
        .with_state(options)
        // Fichiers .br / .gz produits par `cargo leptos build --release --precompress` ;
        // repli sur le fichier brut s'ils sont absents (développement).
        .nest_service(
            &pkg_url,
            ServeDir::new(pkg_dir)
                .precompressed_br()
                .precompressed_gzip(),
        )
        .layer(middleware::from_fn(csp_middleware))
        // Pages /web : jamais de cache (données de session, pages rendues côté serveur).
        .layer(middleware::from_fn_with_state(
            format!("{pkg_url}/"),
            sans_cache_hors_pkg,
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("SAMEORIGIN"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("same-origin"),
        ))
}

/// Démarre l'`ent-api` existant (connexion Postgres locale + migrations) et lui
/// ajoute le relais de session et le frontend. Renvoie le routeur complet et l'adresse d'écoute.
pub async fn routeur_complet() -> Result<(Router, SocketAddr), String> {
    routeur_complet_avec(false).await
}

/// Comme [`routeur_complet`], et démarre les tâches de fond que `ent-api` lance dans son propre
/// `main` : distribution de l'outbox (réactions entre domaines : invitations des responsables,
/// affectations…) et expédition des envois d'inscription. Sans elles, valider une inscription
/// n'envoie aucune invitation (constat de la démo P1). Désactivables comme dans `ent-api`
/// (`ENT_OUTBOX_DISTRIBUTEUR=0`). Réservé au binaire ; les tests utilisent `routeur_complet`.
pub async fn routeur_complet_avec_taches() -> Result<(Router, SocketAddr), String> {
    routeur_complet_avec(true).await
}

async fn routeur_complet_avec(taches: bool) -> Result<(Router, SocketAddr), String> {
    let (state, _bind_api) = ent_api::boot_from_env().await?;
    let distributeur_actif = !matches!(
        std::env::var("ENT_OUTBOX_DISTRIBUTEUR")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "0" | "false" | "non" | "off"
    );
    if taches && distributeur_actif {
        let d = std::sync::Arc::new(
            state
                .ent
                .distributeur(ent_db::outbox::ConfigDistribution::default()),
        );
        std::mem::drop(d.clone().lancer());
        let ent = state.ent.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = ent.expedier_envois_inscriptions(50).await {
                    eprintln!("expédition des envois d'inscription : {e}");
                }
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });
    }
    let options = options_leptos();
    let addr = options.site_addr;
    let session = config_session(addr, |n| std::env::var(n).ok())?;
    let app = assembler(options, ent_api::app(state), session);
    Ok((app, addr))
}

/// Assemble le serveur : relais de session (cookie) + pages Leptos + `ent-api` tel quel.
/// Le relais est monté avant `api` : `/web/session/…` et `/web/api/…` ne chevauchent aucune
/// route d'`ent-api` ni de Leptos.
pub fn assembler(options: LeptosOptions, api: Router, session: ent_web_session::Config) -> Router {
    ent_web_session::passerelle(api.clone(), session)
        .merge(routeur_web(options))
        .merge(api)
}

/// Configuration du relais de session depuis l'environnement (`lire` = lecture d'une variable).
///
/// - `ENT_WEB_ORIGINES` : origines publiques autorisées, séparées par des virgules (D-012,
///   une par établissement). Défaut local : `http://localhost:<port>` et `http://127.0.0.1:<port>`.
/// - `ENT_WEB_COOKIE_SECURE` : `0` désactive `Secure` et le préfixe `__Host-` — **réservé à
///   l'écoute locale** (refusé sinon). Défaut : activé.
/// - `ENT_SESSION_WEB_MAX_HEURES` / `ENT_SESSION_WEB_INACTIVITE_MINUTES` : mêmes variables
///   que la base (défauts 12 h / 120 min) ; le relais s'en sert pour plafonner le cookie et
///   annoncer l'inactivité, **la base reste seule juge** de l'expiration.
pub fn config_session(
    addr: SocketAddr,
    lire: impl Fn(&str) -> Option<String>,
) -> Result<ent_web_session::Config, String> {
    let origines: Vec<String> = match lire("ENT_WEB_ORIGINES") {
        Some(v) => v
            .split(',')
            .map(|o| o.trim().trim_end_matches('/').to_string())
            .filter(|o| !o.is_empty())
            .collect(),
        None => vec![
            format!("http://localhost:{}", addr.port()),
            format!("http://127.0.0.1:{}", addr.port()),
        ],
    };
    if origines.is_empty() {
        return Err("ENT_WEB_ORIGINES est vide".into());
    }
    let secure = lire("ENT_WEB_COOKIE_SECURE")
        .map(|v| v.trim() != "0")
        .unwrap_or(true);
    if !secure && !addr.ip().is_loopback() {
        return Err("ENT_WEB_COOKIE_SECURE=0 n'est permis qu'en écoute locale".into());
    }
    let nombre = |nom: &str, defaut: u64| -> u64 {
        lire(nom)
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(defaut)
    };
    Ok(ent_web_session::Config {
        origines,
        secure,
        duree_max: std::time::Duration::from_secs(nombre("ENT_SESSION_WEB_MAX_HEURES", 12) * 3600),
        inactivite: std::time::Duration::from_secs(
            nombre("ENT_SESSION_WEB_INACTIVITE_MINUTES", 120) * 60,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(paires: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |n| {
            paires
                .iter()
                .find(|(k, _)| *k == n)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn config_session_par_defaut_locale() {
        let c = config_session("127.0.0.1:8090".parse().unwrap(), env(&[])).unwrap();
        assert!(c.secure);
        assert_eq!(c.nom_cookie(), "__Host-ent_session");
        assert!(c.origines.contains(&"http://localhost:8090".to_string()));
        assert_eq!(c.duree_max.as_secs(), 12 * 3600);
        assert_eq!(c.inactivite.as_secs(), 2 * 3600);
    }

    #[test]
    fn config_session_depuis_l_environnement() {
        let c = config_session(
            "127.0.0.1:8090".parse().unwrap(),
            env(&[
                (
                    "ENT_WEB_ORIGINES",
                    " https://lpnb.scolia.fr/ , https://autre.scolia.fr ",
                ),
                ("ENT_SESSION_WEB_MAX_HEURES", "8"),
                ("ENT_SESSION_WEB_INACTIVITE_MINUTES", "30"),
            ]),
        )
        .unwrap();
        assert_eq!(
            c.origines,
            vec!["https://lpnb.scolia.fr", "https://autre.scolia.fr"]
        );
        assert_eq!(c.duree_max.as_secs(), 8 * 3600);
        assert_eq!(c.inactivite.as_secs(), 30 * 60);
        // Valeurs invalides : défauts.
        let c = config_session(
            "127.0.0.1:8090".parse().unwrap(),
            env(&[("ENT_SESSION_WEB_MAX_HEURES", "zéro")]),
        )
        .unwrap();
        assert_eq!(c.duree_max.as_secs(), 12 * 3600);
    }

    #[test]
    fn cookie_non_securise_refuse_hors_ecoute_locale() {
        let e = env(&[("ENT_WEB_COOKIE_SECURE", "0")]);
        let c = config_session("127.0.0.1:8090".parse().unwrap(), &e).unwrap();
        assert!(!c.secure);
        assert_eq!(c.nom_cookie(), "ent_session");
        assert!(config_session("10.0.0.5:8090".parse().unwrap(), &e).is_err());
        assert!(config_session(
            "127.0.0.1:8090".parse().unwrap(),
            env(&[("ENT_WEB_ORIGINES", " , ")])
        )
        .is_err());
    }
}
