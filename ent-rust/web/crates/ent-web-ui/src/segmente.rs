//! Contrôle segmenté : groupe de boutons radio natifs présentés côte à côte.
//! Clavier (flèches) et lecteurs d'écran fournis par le navigateur.

use leptos::prelude::*;

/// Une option du contrôle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionSegment {
    pub valeur: String,
    pub libelle: String,
    pub desactivee: bool,
}

impl OptionSegment {
    pub fn new(valeur: impl Into<String>, libelle: impl Into<String>) -> Self {
        Self {
            valeur: valeur.into(),
            libelle: libelle.into(),
            desactivee: false,
        }
    }
}

/// Contrôle segmenté. `nom` doit être unique dans la page (groupe radio).
#[component]
pub fn Segmente(
    #[prop(into)] nom: String,
    #[prop(into)] legende: String,
    options: Vec<OptionSegment>,
    valeur: RwSignal<String>,
    #[prop(optional, into)] desactive: Signal<bool>,
) -> impl IntoView {
    view! {
        <fieldset class="ui-segmente" disabled=move || desactive.get()>
            <legend>{legende}</legend>
            {options
                .into_iter()
                .map(|o| {
                    let v = o.valeur.clone();
                    let v_coche = o.valeur.clone();
                    view! {
                        <label class="ui-segmente__option">
                            <input
                                type="radio"
                                name=nom.clone()
                                value=o.valeur.clone()
                                disabled=o.desactivee
                                prop:checked=move || valeur.with(|x| *x == v_coche)
                                on:change=move |_| valeur.set(v.clone())
                            />
                            <span>{o.libelle}</span>
                        </label>
                    }
                })
                .collect_view()}
        </fieldset>
    }
}
