//! Erreurs de chargement : un seul comportement pour tous les écrans.
//! 401 → session effacée et retour à la connexion ; 403 → accès refusé ;
//! autres → message clair et « Réessayer ».

use ent_web_api::ApiError;
use ent_web_ui::{AccesRefuse, Alerte, Bouton, Niveau, Variante};
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate};

use super::session;

#[component]
pub fn ErreurChargement(erreur: ApiError, on_reessayer: Callback<()>) -> impl IntoView {
    match erreur {
        ApiError::NonConnecte => {
            // Session expirée ou révoquée (déconnexion dans un autre onglet…).
            let navigate = use_navigate();
            let location = use_location();
            Effect::new(move |_| {
                session::effacer();
                let chemin = session::chemin_complet(
                    &location.pathname.get_untracked(),
                    &location.search.get_untracked(),
                );
                navigate(&session::adresse_connexion(&chemin), Default::default());
            });
            view! {
                <Alerte niveau=Niveau::Info titre="Votre session a expiré.">
                    <p>"Redirection vers la connexion…"</p>
                </Alerte>
            }
            .into_any()
        }
        ApiError::Interdit(_) => view! {
            <AccesRefuse message="Votre profil ne permet pas d'accéder à ces informations."/>
        }
        .into_any(),
        e => view! {
            <Alerte niveau=Niveau::Erreur titre=e.message_utilisateur()>
                <p>
                    <Bouton variante=Variante::Secondaire on_click=on_reessayer>
                        "Réessayer"
                    </Bouton>
                </p>
            </Alerte>
        }
        .into_any(),
    }
}
