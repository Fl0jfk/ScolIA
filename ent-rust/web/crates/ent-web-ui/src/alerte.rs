//! Messages encadrés (dans la page) et zone d'annonce pour lecteurs d'écran.

use leptos::prelude::*;

use crate::bouton::{Bouton, Taille, Variante};

/// Niveau d'un message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Niveau {
    Info,
    Succes,
    Avertissement,
    Erreur,
}

/// Message encadré. Les erreurs utilisent `role="alert"` (annoncées immédiatement),
/// les autres `role="status"` (annoncées poliment). Bouton de fermeture si `on_fermer`.
#[component]
pub fn Alerte(
    niveau: Niveau,
    #[prop(into)] titre: String,
    #[prop(optional)] on_fermer: Option<Callback<()>>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let (classe, role) = match niveau {
        Niveau::Info => ("ui-alerte ui-alerte--info", "status"),
        Niveau::Succes => ("ui-alerte ui-alerte--succes", "status"),
        Niveau::Avertissement => ("ui-alerte ui-alerte--avert", "status"),
        Niveau::Erreur => ("ui-alerte ui-alerte--erreur", "alert"),
    };
    let classe = if on_fermer.is_some() {
        format!("{classe} ui-alerte--fermable")
    } else {
        classe.to_string()
    };
    view! {
        <div class=classe role=role>
            <p class="ui-alerte__titre">{titre}</p>
            {children.map(|c| view! { <div class="ui-alerte__corps">{c()}</div> })}
            {on_fermer.map(|cb| view! {
                <span class="ui-alerte__fermer">
                    <Bouton variante=Variante::Discret taille=Taille::Compacte libelle_accessible="Fermer le message" on_click=cb>
                        <span aria-hidden="true">"×"</span>
                    </Bouton>
                </span>
            })}
        </div>
    }
}

/// Zone annoncée aux lecteurs d'écran (changements de statut, résultats d'actions).
#[component]
pub fn Annonce(#[prop(into)] message: Signal<String>) -> impl IntoView {
    view! {
        <div class="ui-sr" role="status" aria-live="polite" aria-atomic="true">
            {move || message.get()}
        </div>
    }
}
