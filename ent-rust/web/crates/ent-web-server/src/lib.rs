//! Serveur du prototype.
//!
//! Compose sur **une seule origine** :
//! - le routeur `ent_api::app(state)` existant, utilisé tel quel (auth, droits, API) ;
//! - les pages Leptos sous `/web/…`, les fonctions serveur sous `/api/…` et les
//!   fichiers du navigateur (WebAssembly, CSS) sous `/pkg/…`.
//!
//! Même origine ⇒ pas de CORS, et le jeton de l'interface actuelle (`localStorage`)
//! est partagé.

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
/// wasm. Aucun `'unsafe-inline'`. Le jeton de session est (transitoirement) dans
/// `localStorage` : cette CSP limite l'impact d'une injection de script.
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
/// ajoute le frontend. Renvoie le routeur complet et l'adresse d'écoute.
pub async fn routeur_complet() -> Result<(Router, SocketAddr), String> {
    let (state, _bind_api) = ent_api::boot_from_env().await?;
    let options = options_leptos();
    let addr = options.site_addr;
    let app = routeur_web(options).merge(ent_api::app(state));
    Ok((app, addr))
}
