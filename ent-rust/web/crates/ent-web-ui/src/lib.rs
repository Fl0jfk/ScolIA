//! Composants d'interface communs à tous les modules de l'ENT.
//!
//! Règles (voir `docs/DESIGN_SYSTEM.md`) :
//! - éléments HTML natifs d'abord (bouton, case, radio, `<dialog>`) : accessibilité
//!   et clavier fournis par le navigateur ;
//! - états explicites : chargement, vide, erreur, succès, accès refusé ;
//! - aucun JavaScript applicatif : tout passe par Leptos / web-sys.
//!
//! Les styles correspondants sont dans `web/style/ent-web.css` (classes `ui-*`).

use leptos::prelude::*;

/// Apparence d'un bouton.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variante {
    #[default]
    Primaire,
    Secondaire,
    Discret,
    Danger,
}

impl Variante {
    fn classe(self) -> &'static str {
        match self {
            Variante::Primaire => "ui-btn ui-btn--primaire",
            Variante::Secondaire => "ui-btn ui-btn--secondaire",
            Variante::Discret => "ui-btn ui-btn--discret",
            Variante::Danger => "ui-btn ui-btn--danger",
        }
    }
}

/// Bouton natif. `type="button"` par défaut pour ne jamais soumettre un formulaire par erreur.
#[component]
pub fn Bouton(
    #[prop(optional)] variante: Variante,
    #[prop(optional, into)] desactive: Signal<bool>,
    #[prop(optional)] soumettre: bool,
    #[prop(optional, into)] libelle_accessible: Option<String>,
    #[prop(optional)] on_click: Option<Callback<()>>,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            type=if soumettre { "submit" } else { "button" }
            class=variante.classe()
            disabled=move || desactive.get()
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

/// Niveau d'un message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Niveau {
    Info,
    Succes,
    Avertissement,
    Erreur,
}

/// Message encadré. Les erreurs utilisent `role="alert"` (annoncées immédiatement),
/// les autres `role="status"` (annoncées poliment).
#[component]
pub fn Alerte(
    niveau: Niveau,
    #[prop(into)] titre: String,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let (classe, role) = match niveau {
        Niveau::Info => ("ui-alerte ui-alerte--info", "status"),
        Niveau::Succes => ("ui-alerte ui-alerte--succes", "status"),
        Niveau::Avertissement => ("ui-alerte ui-alerte--avert", "status"),
        Niveau::Erreur => ("ui-alerte ui-alerte--erreur", "alert"),
    };
    view! {
        <div class=classe role=role>
            <p class="ui-alerte__titre">{titre}</p>
            {children.map(|c| view! { <div class="ui-alerte__corps">{c()}</div> })}
        </div>
    }
}

/// Lignes grises pendant un chargement (annoncé une seule fois aux lecteurs d'écran).
#[component]
pub fn Squelette(#[prop(default = 6)] lignes: usize, #[prop(into)] libelle: String) -> impl IntoView {
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

/// Zone annoncée aux lecteurs d'écran (changements de statut, résultats d'actions).
#[component]
pub fn Annonce(#[prop(into)] message: Signal<String>) -> impl IntoView {
    view! {
        <div class="ui-sr" role="status" aria-live="polite" aria-atomic="true">
            {move || message.get()}
        </div>
    }
}

/// Boîte de dialogue modale basée sur `<dialog>` natif : piège du focus, touche
/// Échap et retour du focus gérés par le navigateur.
///
/// `ouvert` pilote l'ouverture ; la fermeture par Échap remet `ouvert` à `false`.
#[component]
pub fn Dialogue(
    ouvert: RwSignal<bool>,
    #[prop(into)] titre: String,
    #[prop(into)] id: String,
    children: Children,
) -> impl IntoView {
    let noeud = NodeRef::<leptos::html::Dialog>::new();
    let titre_id = format!("{id}-titre");
    let reference_titre = titre_id.clone();

    // Les effets ne s'exécutent que dans le navigateur : aucun appel DOM côté serveur.
    Effect::new(move |_| {
        let veut_ouvrir = ouvert.get();
        if let Some(dialog) = noeud.get() {
            let el: &web_sys::HtmlDialogElement = &dialog;
            if veut_ouvrir && !el.open() {
                let _ = el.show_modal();
            } else if !veut_ouvrir && el.open() {
                el.close();
            }
        }
    });

    view! {
        <dialog
            node_ref=noeud
            id=id
            class="ui-dialogue"
            aria-labelledby=reference_titre
            on:close=move |_| ouvert.set(false)
        >
            <h2 class="ui-dialogue__titre" id=titre_id>{titre}</h2>
            {children()}
        </dialog>
    }
}

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
