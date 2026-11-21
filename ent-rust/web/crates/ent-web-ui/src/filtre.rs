//! Filtres.

use leptos::prelude::*;

/// Compteur cliquable servant de filtre (bouton bascule `aria-pressed`).
#[component]
pub fn FiltrePastille(
    #[prop(into)] libelle: String,
    #[prop(into)] nombre: Signal<usize>,
    #[prop(into)] actif: Signal<bool>,
    #[prop(into)] teinte: String,
    on_click: Callback<()>,
) -> impl IntoView {
    let classe = format!("ui-pastille ui-pastille--{teinte}");
    view! {
        <button
            type="button"
            class=classe
            aria-pressed=move || if actif.get() { "true" } else { "false" }
            on:click=move |_| on_click.run(())
        >
            <span class="ui-pastille__libelle">{libelle}</span>
            <span class="ui-pastille__nombre">{move || nombre.get()}</span>
        </button>
    }
}
