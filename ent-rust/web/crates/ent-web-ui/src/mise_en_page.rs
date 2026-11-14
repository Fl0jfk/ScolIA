//! Mise en page : en-tête de page, grille Bento et tuiles, statistiques, badges,
//! barre d'action collante, champ de recherche.
//!
//! Règle de la direction artistique : le Bento sert les tableaux de bord
//! (accueil, synthèses), jamais les écrans de saisie.

use leptos::prelude::*;

use crate::icone::{Icone, NomIcone};

/// En-tête de page : surtitre (date, fil d'Ariane), titre, sous-titre, actions à droite.
#[component]
pub fn EnTetePage(
    #[prop(into)] titre: Signal<String>,
    #[prop(optional, into)] surtitre: Option<Signal<String>>,
    #[prop(optional, into)] sous_titre: Option<Signal<String>>,
    /// Titre plus petit (écrans de travail).
    #[prop(optional)]
    compacte: bool,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <header class=if compacte { "ui-entete-page ui-entete-page--compacte" } else { "ui-entete-page" }>
            <div class="ui-entete-page__textes">
                {surtitre.map(|s| view! { <p class="ui-entete-page__surtitre">{move || s.get()}</p> })}
                <h1 class="ui-entete-page__titre">{move || titre.get()}</h1>
                {sous_titre.map(|s| view! { <p class="ui-entete-page__sous-titre">{move || s.get()}</p> })}
            </div>
            {children.map(|c| view! { <div class="ui-entete-page__actions">{c()}</div> })}
        </header>
    }
}

/// Grille Bento : 12 colonnes sur ordinateur, une seule sur téléphone.
#[component]
pub fn GrilleBento(
    #[prop(optional, into)] libelle: Option<String>,
    children: Children,
) -> impl IntoView {
    view! { <div class="ui-bento" role=libelle.as_ref().map(|_| "region") aria-label=libelle>{children()}</div> }
}

/// Largeur d'une tuile sur ordinateur (en colonnes sur 12).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Largeur {
    Quatre,
    Six,
    Huit,
    #[default]
    Douze,
}

impl Largeur {
    fn classe(self) -> &'static str {
        match self {
            Largeur::Quatre => "ui-tuile ui-tuile--4",
            Largeur::Six => "ui-tuile ui-tuile--6",
            Largeur::Huit => "ui-tuile ui-tuile--8",
            Largeur::Douze => "ui-tuile",
        }
    }
}

/// Tuile Bento : une information et une action. `titre` devient un `<h2>`
/// relié à la section (`aria-labelledby`).
#[component]
pub fn Tuile(
    #[prop(into)] id: String,
    #[prop(optional, into)] titre: Option<String>,
    #[prop(optional)] largeur: Largeur,
    /// Fond teinté (annonce, information mise en avant).
    #[prop(optional)]
    accent: bool,
    /// Lien ou action à droite du titre.
    #[prop(optional)]
    action: Option<ViewFn>,
    children: Children,
) -> impl IntoView {
    let classe = format!(
        "{}{}",
        largeur.classe(),
        if accent { " ui-tuile--accent" } else { "" }
    );
    let id_titre = format!("{id}-titre");
    let labelledby = titre.as_ref().map(|_| id_titre.clone());
    view! {
        <section id=id class=classe aria-labelledby=labelledby>
            {titre.map(|t| {
                view! {
                    <div class="ui-tuile__entete">
                        <h2 id=id_titre class="ui-tuile__titre">{t}</h2>
                        {action.map(|a| a.run())}
                    </div>
                }
            })}
            <div class="ui-tuile__corps">{children()}</div>
        </section>
    }
}

/// Ton d'une statistique ou d'un badge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TonEtat {
    #[default]
    Neutre,
    Info,
    Ok,
    Avertissement,
    Mal,
}

/// Chiffre clé avec son libellé (chiffres tabulaires).
#[component]
pub fn Statistique(
    #[prop(into)] libelle: String,
    #[prop(into)] valeur: Signal<String>,
    #[prop(optional)] ton: TonEtat,
) -> impl IntoView {
    let classe = match ton {
        TonEtat::Ok => "ui-stat ui-stat--ok",
        TonEtat::Mal => "ui-stat ui-stat--mal",
        TonEtat::Avertissement => "ui-stat ui-stat--avert",
        _ => "ui-stat",
    };
    view! {
        <div class=classe>
            <span class="ui-stat__libelle">{libelle}</span>
            <span class="ui-stat__valeur">{move || valeur.get()}</span>
        </div>
    }
}

/// Badge d'état : toujours un libellé lisible, la couleur n'est qu'un renfort.
#[component]
pub fn Badge(#[prop(into)] libelle: String, #[prop(optional)] ton: TonEtat) -> impl IntoView {
    let classe = match ton {
        TonEtat::Neutre => "ui-badge",
        TonEtat::Info => "ui-badge ui-badge--info",
        TonEtat::Ok => "ui-badge ui-badge--ok",
        TonEtat::Avertissement => "ui-badge ui-badge--avert",
        TonEtat::Mal => "ui-badge ui-badge--mal",
    };
    view! { <span class=classe>{libelle}</span> }
}

/// Barre collante en bas d'un écran de saisie : état à gauche, action principale à droite.
#[component]
pub fn BarreAction(
    #[prop(optional, into)] libelle: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="ui-barre-action" role="region" aria-label=libelle.unwrap_or_else(|| "Actions".into())>
            {children()}
        </div>
    }
}

/// Champ de recherche compact, avec icône. `libelle` est l'étiquette accessible.
#[component]
pub fn Recherche(
    #[prop(into)] id: String,
    #[prop(into)] libelle: String,
    valeur: RwSignal<String>,
    #[prop(optional, into)] placeholder: Option<String>,
) -> impl IntoView {
    let placeholder = placeholder.unwrap_or_else(|| libelle.clone());
    let pour = id.clone();
    view! {
        <label class="ui-recherche" for=pour>
            <Icone nom=NomIcone::Recherche petite=true/>
            <span class="ui-sr">{libelle}</span>
            <input
                id=id
                type="search"
                autocomplete="off"
                placeholder=placeholder
                prop:value=move || valeur.get()
                on:input=move |ev| valeur.set(event_target_value(&ev))
            />
        </label>
    }
}
