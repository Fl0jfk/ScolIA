//! Serveur du prototype.
//!
//! Compose sur **une seule origine** :
//! - le routeur `ent_api::app(state)` existant, utilisé tel quel (auth, droits, API) ;
//! - les pages Leptos sous `/web/…`, les fonctions serveur sous `/api/…` et les
//!   fichiers du navigateur (WebAssembly, CSS) sous `/pkg/…`.
//!
//! Même origine ⇒ pas de CORS, et le jeton de l'interface actuelle (`localStorage`)
//! est partagé.

pub mod labo;

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::http::{header, HeaderValue};
use axum::Router;
use ent_web_app::{shell, App};
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

/// Routes du frontend Leptos (pages SSR + fonctions serveur + fichiers statiques).
pub fn routeur_web(options: LeptosOptions) -> Router {
    let routes = generate_route_list(App);
    let pkg_dir = PathBuf::from(options.site_root.as_ref()).join(options.site_pkg_dir.as_ref());
    let pkg_url = format!("/{}", options.site_pkg_dir);
    Router::new()
        .leptos_routes(&options, routes, {
            let options = options.clone();
            move || shell(options.clone())
        })
        .with_state(options)
        // Fichiers .br / .gz produits par `cargo leptos build --release --precompress` ;
        // repli sur le fichier brut s'ils sont absents (développement).
        .nest_service(&pkg_url, ServeDir::new(pkg_dir).precompressed_br().precompressed_gzip())
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
