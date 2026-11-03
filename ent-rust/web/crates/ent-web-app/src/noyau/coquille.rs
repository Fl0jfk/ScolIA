//! Coquille authentifiée : charge la personne connectée une seule fois, protège les
//! pages, construit la navigation selon les droits, gère la déconnexion (y compris
//! dans les autres onglets).

use ent_web_api::ApiError;
use ent_web_ui::{Coquille, Identite};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::Outlet;
use leptos_router::hooks::{use_location, use_navigate};

use super::droits::{droits_du_role, libelle_role, Droits};
use super::navigation;
use super::session::{self, ContexteSession};
use crate::modules::connexion::config_connexion;

/// Droits de la personne connectée (aucun tant que `/auth/me` n'a pas répondu).
pub fn use_droits() -> Signal<Droits> {
    let ctx = session::use_session();
    Signal::derive(move || {
        ctx.moi
            .get()
            .and_then(Result::ok)
            .map(|m| droits_du_role(&m.role))
            .unwrap_or_default()
    })
}

#[component]
pub fn AppCoquille() -> impl IntoView {
    let navigate = use_navigate();
    let location = use_location();
    // LocalResource (non sérialisée) : lue hors <Suspense/>, un Resource provoquerait
    // l'avertissement Leptos et des erreurs d'hydratation.
    let config = LocalResource::new(|| async { config_connexion().await });

    let chemin_complet = move || {
        session::chemin_complet(
            &location.pathname.get_untracked(),
            &location.search.get_untracked(),
        )
    };

    // Sans session : connexion, puis retour ici (navigateur uniquement).
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if session::jeton().is_none() {
                navigate(
                    &session::adresse_connexion(&chemin_complet()),
                    Default::default(),
                );
            }
        });
    }

    let moi = LocalResource::new(|| async { session::client().moi().await });
    provide_context(ContexteSession { moi });

    // Session refusée par le serveur (expirée, révoquée) : retour à la connexion.
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if let Some(Err(ApiError::NonConnecte)) = moi.get() {
                session::effacer();
                navigate(
                    &session::adresse_connexion(&chemin_complet()),
                    Default::default(),
                );
            }
        });
    }

    // Déconnexion dans un autre onglet : `localStorage` vidé → cet onglet suit.
    #[cfg(target_arch = "wasm32")]
    {
        let navigate = navigate.clone();
        // Jeton avec lequel cet onglet a chargé son identité affichée.
        let jeton_onglet = session::jeton();
        let handle = window_event_listener(leptos::ev::storage, move |ev| {
            match session::suite_evenement_stockage(
                ev.key().as_deref(),
                ev.new_value().as_deref(),
                jeton_onglet.as_deref(),
            ) {
                session::SuiteStockage::Ignorer => {}
                session::SuiteStockage::Deconnexion => {
                    navigate("/web/connexion?fin=autre-onglet", Default::default());
                }
                // Autre compte ailleurs : on repart de zéro (identité, droits, données).
                session::SuiteStockage::Recharger => {
                    let _ = window().location().reload();
                }
            }
        });
        on_cleanup(move || handle.remove());
    }

    // Après chaque changement de page : focus sur <main> et annonce du titre.
    #[cfg(target_arch = "wasm32")]
    {
        let chemin = Memo::new(move |_| location.pathname.get());
        Effect::new(move |precedent: Option<()>| {
            chemin.track();
            if precedent.is_some() {
                apres_navigation();
            }
        });
    }

    let droits = Signal::derive(move || {
        moi.get()
            .and_then(Result::ok)
            .map(|m| droits_du_role(&m.role))
            .unwrap_or_default()
    });
    let entrees = Signal::derive(move || navigation::entrees(droits.get()));
    let courant = Signal::derive(move || location.pathname.get());
    let rail = Signal::derive(move || navigation::ecran_de_saisie(&location.pathname.get()));
    let identite = Signal::derive(move || {
        moi.get().and_then(Result::ok).map(|m| Identite {
            nom: m.email.clone(),
            detail: libelle_role(&m.role).to_string(),
        })
    });
    let etablissement = Signal::derive(move || match config.get().and_then(Result::ok) {
        Some(c) if c.labo => "Environnement de test".to_string(),
        _ => String::new(),
    });

    let deconnecter = Callback::new(move |_| {
        let navigate = navigate.clone();
        spawn_local(async move {
            // Révocation en base ; même en cas d'échec réseau, la session locale est effacée.
            let _ = session::client().deconnexion().await;
            session::effacer();
            navigate("/web/connexion?fin=deconnexion", Default::default());
        });
    });

    view! {
        <Coquille
            entrees=entrees
            courant=courant
            etablissement=etablissement
            identite=identite
            rail=rail
            on_deconnexion=deconnecter
        >
            <div id="annonce-page" class="ui-sr" role="status" aria-live="polite" aria-atomic="true"></div>
            <Outlet/>
        </Coquille>
    }
}

/// Après un changement de page (navigateur uniquement) : donne le focus à `<main>` et
/// annonce le titre de la page dans la région `aria-live` polie `#annonce-page` si elle
/// existe. Le focus n'est pas volé si l'utilisateur est déjà dans la page (par ex. un
/// champ de l'appel qui l'a pris volontairement).
#[cfg(target_arch = "wasm32")]
fn apres_navigation() {
    use leptos::wasm_bindgen::JsCast;
    // Laisse `leptos_meta` mettre à jour `document.title` et la page se dessiner.
    set_timeout(
        move || {
            let doc = document();
            if let Some(main) = doc
                .get_element_by_id("contenu")
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let deja_dans_la_page = doc
                    .active_element()
                    .is_some_and(|a| a.id() != "contenu" && main.contains(Some(&a)));
                if !deja_dans_la_page {
                    let _ = main.focus();
                }
            }
            if let Some(zone) = doc.get_element_by_id("annonce-page") {
                zone.set_text_content(Some(&doc.title()));
            }
        },
        std::time::Duration::from_millis(80),
    );
}
