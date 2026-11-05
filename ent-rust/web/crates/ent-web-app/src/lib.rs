//! Application Leptos de l'ENT — tranche 1 : parcours enseignant (connexion, accueil,
//! emploi du temps, appel). Architecture : `docs/FRONTEND_CIBLE.md` §3.
//!
//! Rendu : le serveur produit le HTML (coquille, en-têtes, états de chargement), puis
//! le navigateur l'**hydrate** avec le WebAssembly. Les données métier sont chargées
//! côté navigateur avec la session (voir `noyau/session.rs`).

// Les vues Leptos produisent des types très imbriqués : sans effacement des
// composants (`erase_components`), la limite par défaut (128) est dépassée.
#![recursion_limit = "512"]

mod composants;
pub mod composants_metier;
pub mod modules;
pub mod noyau;

// Chemins historiques (tests contre le backend, serveur).
pub use modules::appel;
pub use modules::connexion::{config_connexion, ParametresConnexion};
pub use noyau::session;

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{ParentRoute, Route, Router, Routes},
    path,
};

/// Document HTML complet rendu par le serveur.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="fr">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover"/>
                <meta name="color-scheme" content="light"/>
                <meta name="theme-color" content="#f5f5f7"/>
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
    ent_web_ui::fournir_notifications();
    // À la racine : uniquement les balises leptos_meta et <Router> (voir FRONTEND_LEPTOS.md §8).
    view! {
        <Stylesheet id="ent-web" href="/pkg/ent-web.css"/>
        <Title formatter=|titre: String| format!("{titre} · ScolIA")/>
        <Router>
            <Routes fallback=PageIntrouvable>
                <Route path=path!("/web/connexion") view=modules::connexion::PageConnexion/>
                <Route path=path!("/web/_composants") view=composants::PageComposants/>
                <ParentRoute path=path!("/web") view=noyau::coquille::AppCoquille>
                    <Route path=path!("") view=modules::accueil::PageAccueil/>
                    <Route path=path!("emploi-du-temps") view=modules::edt::PageEmploiDuTemps/>
                    <Route path=path!("appel") view=modules::appel::PageAppel/>
                    <Route path=path!("classes") view=modules::classes::PageClasses/>
                    <Route path=path!("classes/:id") view=modules::classes::PageClasse/>
                    <Route path=path!("mes-classes") view=modules::classes::PageMesClasses/>
                    <Route path=path!("vie-scolaire") view=modules::vie_scolaire::PageVieScolaire/>
                    <Route path=path!("absences") view=modules::absences::PageAbsences/>
                    <Route path=path!("messages") view=modules::messagerie::PageMessages/>
                    <Route path=path!("messages/nouveau") view=modules::messagerie::PageNouveauMessage/>
                    <Route path=path!("messages/:id") view=modules::messagerie::PageConversation/>
                </ParentRoute>
            </Routes>
            <ent_web_ui::ZoneNotifications/>
        </Router>
    }
}

#[component]
fn PageIntrouvable() -> impl IntoView {
    view! {
        <Title text="Page introuvable"/>
        <main id="contenu" class="page page--etroite">
            <h1>"Page introuvable"</h1>
            <p><a href="/web">"Revenir à l'accueil"</a></p>
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
