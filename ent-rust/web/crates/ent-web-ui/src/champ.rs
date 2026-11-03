//! Champs de formulaire accessibles : étiquette, aide et erreur reliées au contrôle.
//!
//! Règles :
//! - chaque champ a une étiquette visible (`<label for>`) ;
//! - aide et erreur sont reliées par `aria-describedby` ; `aria-invalid` en cas d'erreur ;
//! - la validation dans le navigateur est un confort : le serveur reste l'autorité,
//!   ses erreurs sont affichées dans le champ concerné via `erreur`.

use leptos::prelude::*;

use crate::alerte::{Alerte, Niveau};

fn decrit_par(aide_id: Option<&str>, erreur_id: &str, a_une_erreur: bool) -> Option<String> {
    let mut ids = Vec::new();
    if let Some(a) = aide_id {
        ids.push(a.to_string());
    }
    if a_une_erreur {
        ids.push(erreur_id.to_string());
    }
    (!ids.is_empty()).then(|| ids.join(" "))
}

#[component]
fn Habillage(
    id: String,
    libelle: String,
    requis: bool,
    aide: Option<String>,
    erreur: Signal<Option<String>>,
    children: Children,
) -> impl IntoView {
    let aide_id = format!("{id}-aide");
    let erreur_id = format!("{id}-erreur");
    view! {
        <div class="ui-champ">
            <label class="ui-champ__libelle" for=id>
                {libelle}
                {requis.then(|| view! {
                    <span class="ui-champ__requis" aria-hidden="true">"*"</span>
                    <span class="ui-sr">" (obligatoire)"</span>
                })}
            </label>
            {children()}
            {aide.map(|a| view! { <p id=aide_id class="ui-champ__aide">{a}</p> })}
            {move || erreur.get().map(|e| view! { <p id=erreur_id.clone() class="ui-champ__erreur">{e}</p> })}
        </div>
    }
}

/// Champ de saisie d'une ligne (`text`, `email`, `password`, `search`, `date`, `tel`, `number`).
#[component]
pub fn ChampTexte(
    #[prop(into)] id: String,
    #[prop(into)] libelle: String,
    valeur: RwSignal<String>,
    #[prop(default = "text")] genre: &'static str,
    #[prop(optional, into)] aide: Option<String>,
    #[prop(optional, into)] erreur: Signal<Option<String>>,
    #[prop(optional)] requis: bool,
    #[prop(optional, into)] desactive: Signal<bool>,
    #[prop(optional, into)] autocomplete: Option<String>,
    #[prop(optional, into)] placeholder: Option<String>,
    #[prop(optional, into)] inputmode: Option<String>,
) -> impl IntoView {
    let aide_id = aide.as_ref().map(|_| format!("{id}-aide"));
    let erreur_id = format!("{id}-erreur");
    let controle_id = id.clone();
    view! {
        <Habillage id=id libelle=libelle requis=requis aide=aide erreur=erreur>
            <input
                id=controle_id
                class="ui-champ__controle"
                type=genre
                required=requis
                autocomplete=autocomplete
                placeholder=placeholder
                inputmode=inputmode
                disabled=move || desactive.get()
                aria-invalid=move || erreur.with(Option::is_some).then_some("true")
                aria-describedby={
                    let erreur_id = erreur_id.clone();
                    let aide_id = aide_id.clone();
                    move || decrit_par(aide_id.as_deref(), &erreur_id, erreur.with(Option::is_some))
                }
                prop:value=move || valeur.get()
                on:input=move |ev| valeur.set(event_target_value(&ev))
            />
        </Habillage>
    }
}

/// Zone de texte multiligne.
#[component]
pub fn ZoneTexte(
    #[prop(into)] id: String,
    #[prop(into)] libelle: String,
    valeur: RwSignal<String>,
    #[prop(optional, into)] aide: Option<String>,
    #[prop(optional, into)] erreur: Signal<Option<String>>,
    #[prop(optional)] requis: bool,
    #[prop(optional, into)] desactive: Signal<bool>,
) -> impl IntoView {
    let aide_id = aide.as_ref().map(|_| format!("{id}-aide"));
    let erreur_id = format!("{id}-erreur");
    let controle_id = id.clone();
    view! {
        <Habillage id=id libelle=libelle requis=requis aide=aide erreur=erreur>
            <textarea
                id=controle_id
                class="ui-champ__controle"
                required=requis
                disabled=move || desactive.get()
                aria-invalid=move || erreur.with(Option::is_some).then_some("true")
                aria-describedby=move || decrit_par(aide_id.as_deref(), &erreur_id, erreur.with(Option::is_some))
                prop:value=move || valeur.get()
                on:input=move |ev| valeur.set(event_target_value(&ev))
            ></textarea>
        </Habillage>
    }
}

/// Liste de choix (`<select>` natif). `options` : (valeur, libellé).
#[component]
pub fn Selection(
    #[prop(into)] id: String,
    #[prop(into)] libelle: String,
    valeur: RwSignal<String>,
    options: Vec<(String, String)>,
    #[prop(optional, into)] aide: Option<String>,
    #[prop(optional, into)] erreur: Signal<Option<String>>,
    #[prop(optional)] requis: bool,
    #[prop(optional, into)] desactive: Signal<bool>,
) -> impl IntoView {
    let aide_id = aide.as_ref().map(|_| format!("{id}-aide"));
    let erreur_id = format!("{id}-erreur");
    let controle_id = id.clone();
    view! {
        <Habillage id=id libelle=libelle requis=requis aide=aide erreur=erreur>
            <select
                id=controle_id
                class="ui-champ__controle"
                required=requis
                disabled=move || desactive.get()
                aria-invalid=move || erreur.with(Option::is_some).then_some("true")
                aria-describedby=move || decrit_par(aide_id.as_deref(), &erreur_id, erreur.with(Option::is_some))
                prop:value=move || valeur.get()
                on:change=move |ev| valeur.set(event_target_value(&ev))
            >
                {options
                    .into_iter()
                    .map(|(v, l)| view! { <option value=v>{l}</option> })
                    .collect_view()}
            </select>
        </Habillage>
    }
}

/// Case à cocher avec son étiquette (zone cliquable de 44 px de haut).
#[component]
pub fn Case(
    #[prop(into)] id: String,
    #[prop(into)] libelle: String,
    coche: RwSignal<bool>,
    #[prop(optional, into)] desactive: Signal<bool>,
) -> impl IntoView {
    let pour = id.clone();
    view! {
        <label class="ui-case" for=pour>
            <input
                id=id
                type="checkbox"
                disabled=move || desactive.get()
                prop:checked=move || coche.get()
                on:change=move |ev| coche.set(event_target_checked(&ev))
            />
            <span>{libelle}</span>
        </label>
    }
}

/// Groupe de boutons radio natifs (`<fieldset>` + `<legend>`) ; flèches du clavier gérées par le navigateur.
#[component]
pub fn GroupeChoix(
    #[prop(into)] nom: String,
    #[prop(into)] legende: String,
    valeur: RwSignal<String>,
    options: Vec<(String, String)>,
    #[prop(optional, into)] desactive: Signal<bool>,
) -> impl IntoView {
    view! {
        <fieldset class="ui-champ" disabled=move || desactive.get()>
            <legend class="ui-champ__libelle">{legende}</legend>
            {options
                .into_iter()
                .map(|(v, l)| {
                    let id = format!("{nom}-{v}");
                    let pour = id.clone();
                    let v2 = v.clone();
                    view! {
                        <label class="ui-case" for=pour>
                            <input
                                id=id
                                type="radio"
                                name=nom.clone()
                                value=v.clone()
                                prop:checked=move || valeur.get() == v
                                on:change=move |_| valeur.set(v2.clone())
                            />
                            <span>{l}</span>
                        </label>
                    }
                })
                .collect_view()}
        </fieldset>
    }
}

/// Résumé des erreurs d'un formulaire, avec un lien vers chaque champ. `erreurs` : (id du champ, message).
#[component]
pub fn ResumeErreurs(#[prop(into)] erreurs: Signal<Vec<(String, String)>>) -> impl IntoView {
    move || {
        let e = erreurs.get();
        (!e.is_empty()).then(|| {
            let titre = if e.len() == 1 {
                "Le formulaire contient une erreur".to_string()
            } else {
                format!("Le formulaire contient {} erreurs", e.len())
            };
            view! {
                <div class="ui-resume-erreurs">
                    <Alerte niveau=Niveau::Erreur titre=titre>
                        <ul>
                            {e.into_iter()
                                .map(|(id, m)| view! { <li><a href=format!("#{id}")>{m}</a></li> })
                                .collect_view()}
                        </ul>
                    </Alerte>
                </div>
            }
        })
    }
}
