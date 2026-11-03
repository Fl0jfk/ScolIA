//! Boutons et liens présentés comme des boutons.

use leptos::prelude::*;

/// Apparence d'un bouton.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variante {
    #[default]
    Primaire,
    Secondaire,
    Discret,
    Danger,
    /// Fond encre : action principale neutre (connexion par passkey).
    Sombre,
}

/// Taille d'un bouton. `Normale` respecte la cible tactile de 44 px.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Taille {
    #[default]
    Normale,
    Compacte,
}

/// Classes CSS d'un bouton (utilisées aussi par [`LienBouton`]).
pub fn classes_bouton(variante: Variante, taille: Taille) -> String {
    let v = match variante {
        Variante::Primaire => "ui-btn--primaire",
        Variante::Secondaire => "ui-btn--secondaire",
        Variante::Discret => "ui-btn--discret",
        Variante::Danger => "ui-btn--danger",
        Variante::Sombre => "ui-btn--sombre",
    };
    match taille {
        Taille::Normale => format!("ui-btn {v}"),
        Taille::Compacte => format!("ui-btn {v} ui-btn--compact"),
    }
}

/// Bouton natif. `type="button"` par défaut pour ne jamais soumettre un formulaire par erreur.
/// Pendant `en_cours`, le bouton est désactivé et annoncé occupé (`aria-busy`).
#[component]
pub fn Bouton(
    #[prop(optional)] variante: Variante,
    #[prop(optional)] taille: Taille,
    #[prop(optional, into)] desactive: Signal<bool>,
    #[prop(optional, into)] en_cours: Signal<bool>,
    #[prop(optional)] soumettre: bool,
    #[prop(optional, into)] libelle_accessible: Option<String>,
    #[prop(optional)] on_click: Option<Callback<()>>,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            type=if soumettre { "submit" } else { "button" }
            class=classes_bouton(variante, taille)
            disabled=move || desactive.get() || en_cours.get()
            aria-busy=move || en_cours.get().then_some("true")
            aria-label=libelle_accessible
            on:click=move |_| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
        >
            {children()}
        </button>
    }
}

/// Lien de navigation présenté comme un bouton (`<a>` : reste un lien pour les
/// lecteurs d'écran et le clic droit « ouvrir dans un onglet »).
#[component]
pub fn LienBouton(
    #[prop(into)] href: String,
    #[prop(optional)] variante: Variante,
    #[prop(optional)] taille: Taille,
    children: Children,
) -> impl IntoView {
    view! { <a href=href class=classes_bouton(variante, taille)>{children()}</a> }
}
