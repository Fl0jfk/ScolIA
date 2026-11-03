//! États d'un contenu : chargement, vide, erreur, accès refusé.
//! Chaque écran doit afficher explicitement l'un de ces états plutôt qu'une zone blanche.

use leptos::prelude::*;

use crate::bouton::{Bouton, Variante};

/// Indicateur de chargement en ligne (texte toujours présent, pas seulement une animation).
#[component]
pub fn Chargement(#[prop(into)] libelle: String) -> impl IntoView {
    view! {
        <span class="ui-chargement" role="status">
            <span class="ui-chargement__rond" aria-hidden="true"></span>
            <span>{libelle}</span>
        </span>
    }
}

/// Lignes grises pendant un chargement de liste (annoncé une seule fois).
#[component]
pub fn Squelette(
    #[prop(default = 6)] lignes: usize,
    #[prop(into)] libelle: String,
) -> impl IntoView {
    view! {
        <div class="ui-squelette" role="status" aria-live="polite">
            <span class="ui-sr">{libelle}</span>
            {(0..lignes)
                .map(|_| view! { <div class="ui-squelette__ligne" aria-hidden="true"></div> })
                .collect_view()}
        </div>
    }
}

/// État vide (aucun résultat, aucune séance…).
#[component]
pub fn EtatVide(
    #[prop(into)] titre: String,
    #[prop(optional, into)] message: Option<String>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="ui-vide">
            <p class="ui-vide__titre">{titre}</p>
            {message.map(|m| view! { <p class="ui-vide__message">{m}</p> })}
            {children.map(|c| c())}
        </div>
    }
}

/// Erreur de chargement, avec « Réessayer » si une action de reprise est fournie.
#[component]
pub fn EtatErreur(
    #[prop(into)] titre: String,
    #[prop(optional, into)] message: Option<String>,
    #[prop(optional)] on_reessayer: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <div class="ui-etat ui-etat--erreur" role="alert">
            <p class="ui-etat__titre">{titre}</p>
            {message.map(|m| view! { <p class="ui-etat__message">{m}</p> })}
            {on_reessayer.map(|cb| view! {
                <Bouton variante=Variante::Secondaire on_click=cb>"Réessayer"</Bouton>
            })}
        </div>
    }
}

/// Accès refusé : l'utilisateur est connecté mais n'a pas les droits.
#[component]
pub fn AccesRefuse(#[prop(optional, into)] message: Option<String>) -> impl IntoView {
    view! {
        <div class="ui-etat" role="status">
            <p class="ui-etat__titre">"Accès refusé"</p>
            <p class="ui-etat__message">
                {message.unwrap_or_else(|| "Vous n'avez pas les droits nécessaires pour afficher cette page.".into())}
            </p>
        </div>
    }
}
