//! Application Leptos de l'ENT — prototype du module d'appel (D-008).
//!
//! Rendu : le serveur produit le HTML de la page (coquille, en-tête, formulaires,
//! états de chargement), puis le navigateur l'**hydrate** avec le WebAssembly.
//! Les données métier sont chargées côté navigateur avec le jeton de session
//! existant (voir `session.rs` et `docs/LEPTOS_CALL_PROTOTYPE.md`).

// Les vues Leptos produisent des types très imbriqués : sans effacement des
// composants (`erase_components`), la limite par défaut (128) est dépassée.
#![recursion_limit = "512"]

pub mod appel;
mod connexion;
mod entete;
pub mod session;

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Redirect, Route, Router, Routes},
    path,
};

pub use connexion::{config_connexion, ParametresConnexion};

/// Document HTML complet rendu par le serveur.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="fr">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="color-scheme" content="light"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="ent-web" href="/pkg/ent-web.css"/>
        <Title formatter=|titre: String| format!("{titre} · ScolIA")/>
        <Router>
            <Routes fallback=PageIntrouvable>
                <Route path=path!("/web") view=|| view! { <Redirect path="/web/appel"/> }/>
                <Route path=path!("/web/connexion") view=connexion::PageConnexion/>
                <Route path=path!("/web/appel") view=appel::PageAppel/>
            </Routes>
        </Router>
    }
}

#[component]
fn PageIntrouvable() -> impl IntoView {
    view! {
        <Title text="Page introuvable"/>
        <main id="contenu" class="page page--etroite">
            <h1>"Page introuvable"</h1>
            <p><a href="/web/appel">"Revenir à l'appel"</a></p>
        </main>
    }
}

/// Point d'entrée WebAssembly : hydrate la page rendue par le serveur.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
