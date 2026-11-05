//! Erreurs de chargement : un seul comportement pour tous les écrans.
//! 401 → fenêtre de reconnexion par-dessus la page (le chargement est relancé après la
//! reconnexion) ; 403 → accès refusé ; autres → message clair et « Réessayer ».

use ent_web_api::ApiError;
use ent_web_ui::{AccesRefuse, Alerte, Bouton, Niveau, Variante};
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate};

use super::{reconnexion, session};

#[component]
pub fn ErreurChargement(erreur: ApiError, on_reessayer: Callback<()>) -> impl IntoView {
    match erreur {
        ApiError::NonConnecte => {
            // Session expirée ou révoquée (déconnexion dans un autre onglet…).
            match reconnexion::use_reconnexion() {
                Some(reco) => {
                    // Dans la coquille : fenêtre de reconnexion, puis nouveau chargement.
                    let version_initiale = reco.version.get_untracked();
                    Effect::new(move |_| reco.ouverte.set(true));
                    Effect::new(move |_| {
                        if reco.version.get() != version_initiale {
                            on_reessayer.run(());
                        }
                    });
                    view! {
                        <Alerte niveau=Niveau::Info titre="Votre session a expiré.">
                            <p>"Reconnectez-vous dans la fenêtre ouverte : cette page se rechargera ensuite."</p>
                        </Alerte>
                    }
                    .into_any()
                }
                None => {
                    // Hors coquille (page publique) : retour à la connexion.
                    let navigate = use_navigate();
                    let location = use_location();
                    Effect::new(move |_| {
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
            }
        }
        // Session « enrôlement » : le serveur refuse tout sauf l'enrôlement d'une passkey.
        e @ ApiError::Interdit(_) if e.est_enrolement_requis() => {
            let navigate = use_navigate();
            Effect::new(move |_| navigate(super::portee::ENROLEMENT, Default::default()));
            view! {
                <Alerte niveau=Niveau::Info titre="Une passkey doit d'abord être enregistrée.">
                    <p>"Redirection vers l'enregistrement de votre passkey…"</p>
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
