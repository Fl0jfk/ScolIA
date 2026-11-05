//! Messagerie : boîte de réception, conversation, nouveau message — pour tous les rôles
//! autorisés (personnel de la messagerie, famille, élève). Routes existantes d'`ent-api`
//! (`/messages/fils*`, `/messages/modeles`, `/messages/{id}/pieces`, `/messages/pieces/{id}`,
//! `/famille/enfants`, `/eleves/search`) ; droits, cloisonnement par établissement et accusés
//! de lecture sont décidés par le serveur, l'interface ne fait que s'y adapter.

mod boite;
mod fichiers;
mod fil;
pub mod modele;
mod nouveau;

pub use boite::PageMessages;
pub use fil::PageConversation;
pub use nouveau::PageNouveauMessage;

use ent_web_api::ApiError;
use leptos_router::NavigateOptions;

use crate::noyau::session;

/// Adresse du formulaire de nouveau message.
pub const NOUVEAU: &str = "/web/messages/nouveau";

/// Suite commune à un échec d'écriture : renvoie le message à afficher ; sur 401 la session
/// locale est effacée et la personne est renvoyée à la connexion (retour sur `chemin`).
pub(crate) fn traiter_echec<N: Fn(&str, NavigateOptions)>(
    erreur: &ApiError,
    navigate: &N,
    chemin: &str,
) -> String {
    if matches!(erreur, ApiError::NonConnecte) {
        session::effacer();
        navigate(&session::adresse_connexion(chemin), Default::default());
    }
    erreur.message_utilisateur()
}

/// Donne le focus à l'élément `id` (navigateur uniquement), après le prochain dessin.
#[cfg(target_arch = "wasm32")]
pub(crate) fn focaliser(id: &'static str) {
    use leptos::wasm_bindgen::JsCast;
    leptos::prelude::set_timeout(
        move || {
            if let Some(el) = leptos::prelude::document()
                .get_element_by_id(id)
                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = el.focus();
            }
        },
        std::time::Duration::from_millis(60),
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn focaliser(_id: &'static str) {}

/// Donne le focus à l'élément `id` seulement si rien dans la page n'a le focus (arrivée sur
/// une page après une action : le formulaire précédent a disparu). Navigateur uniquement.
#[cfg(target_arch = "wasm32")]
pub(crate) fn focaliser_si_perdu(id: &'static str) {
    use leptos::wasm_bindgen::JsCast;
    leptos::prelude::set_timeout(
        move || {
            let doc = leptos::prelude::document();
            let perdu = doc
                .active_element()
                .is_none_or(|a| a.tag_name().eq_ignore_ascii_case("body"));
            if perdu {
                if let Some(el) = doc
                    .get_element_by_id(id)
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                {
                    let _ = el.focus();
                }
            }
        },
        std::time::Duration::from_millis(250),
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn focaliser_si_perdu(_id: &'static str) {}
