//! En-tête commun : identité de l'utilisateur et déconnexion.

use ent_web_api::{ApiError, Moi};
use ent_web_ui::{Bouton, Variante};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::session;

pub fn libelle_role(role: &str) -> &'static str {
    match role {
        "admin" => "Administration",
        "direction" => "Direction",
        "administratif" => "Secrétariat",
        "cpe" => "CPE",
        "compta" => "Comptabilité",
        "prof" => "Enseignant",
        "infirmerie" => "Infirmerie",
        "psy" => "Psychologue",
        "parent" => "Famille",
        "eleve" => "Élève",
        _ => "Utilisateur",
    }
}

#[component]
pub fn EnTete(moi: LocalResource<Result<Moi, ApiError>>) -> impl IntoView {
    let navigate = use_navigate();
    let deconnecter = Callback::new(move |_| {
        let navigate = navigate.clone();
        spawn_local(async move {
            // La révocation serveur est tentée ; le jeton local est effacé quoi qu'il arrive.
            let _ = session::client().deconnexion().await;
            session::effacer();
            navigate("/web/connexion", Default::default());
        });
    });

    view! {
        <header class="barre">
            <a class="barre__marque" href="/web/appel">"ScolIA"</a>
            <nav class="barre__nav" aria-label="Navigation principale">
                <a href="/web/appel" aria-current="page">"Appel"</a>
            </nav>
            <div class="barre__compte">
                {move || match moi.get() {
                    Some(Ok(m)) => {
                        view! {
                            <span class="barre__identite">
                                <span class="barre__email">{m.email.clone()}</span>
                                <span class="barre__role">{libelle_role(&m.role)}</span>
                            </span>
                        }
                            .into_any()
                    }
                    _ => view! { <span class="barre__identite"></span> }.into_any(),
                }}
                <Bouton variante=Variante::Discret on_click=deconnecter>
                    "Se déconnecter"
                </Bouton>
            </div>
        </header>
    }
}
