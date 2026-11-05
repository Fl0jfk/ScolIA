//! Coquille applicative : navigation principale + zone de contenu.
//!
//! Une seule liste de liens, trois présentations en CSS seulement (pas de
//! détection d'appareil) : barre latérale (≥ 1200 px), rail d'icônes
//! (768–1199 px, ou forcé sur les écrans de saisie), barre d'onglets en bas (téléphone).
//! Aucune notion métier : les entrées sont fournies par l'application.

use leptos::prelude::*;

use crate::avatar::Avatar;
use crate::icone::{Icone, NomIcone};

/// Une entrée de navigation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntreeNav {
    pub libelle: String,
    pub href: String,
    pub icone: NomIcone,
    /// Petit compteur (ex. messages non lus).
    pub badge: Option<String>,
}

/// Vrai si `href` correspond à la page courante (`/web` n'est actif que sur lui-même).
pub fn lien_actif(href: &str, courant: &str) -> bool {
    let courant = courant.trim_end_matches('/');
    let href = href.trim_end_matches('/');
    courant == href || (href.matches('/').count() > 1 && courant.starts_with(&format!("{href}/")))
}

/// Personne connectée, affichée en pied de navigation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identite {
    pub nom: String,
    pub detail: String,
}

#[component]
pub fn Coquille(
    #[prop(into)] entrees: Signal<Vec<EntreeNav>>,
    #[prop(into)] courant: Signal<String>,
    #[prop(into)] etablissement: Signal<String>,
    #[prop(into)] identite: Signal<Option<Identite>>,
    /// Rail d'icônes même sur grand écran (écrans de saisie).
    #[prop(optional, into)]
    rail: Signal<bool>,
    on_deconnexion: Callback<()>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="ui-coquille" class:ui-coquille--rail=move || rail.get()>
            <a class="lien-evitement" href="#contenu">"Aller au contenu"</a>
            <nav class="ui-coquille__nav" aria-label="Navigation principale">
                <div class="ui-coquille__marque">
                    <span class="ui-coquille__logo"><Icone nom=NomIcone::Ecole/></span>
                    <span class="ui-coquille__nom">
                        <span class="ui-coquille__produit">"ScolIA"</span>
                        <span class="ui-coquille__etab">{move || etablissement.get()}</span>
                    </span>
                </div>
                <ul class="ui-coquille__liens">
                    <For
                        each=move || entrees.get()
                        key=|e| (e.href.clone(), e.badge.clone())
                        children=move |e| {
                            let href = e.href.clone();
                            let libelle = e.libelle.clone();
                            let titre = e.libelle.clone();
                            view! {
                                <li>
                                    <a
                                        class="ui-coquille__lien"
                                        href=e.href.clone()
                                        title=titre
                                        aria-current=move || courant.with(|c| lien_actif(&href, c)).then_some("page")
                                    >
                                        <Icone nom=e.icone/>
                                        <span class="ui-coquille__libelle">{libelle}</span>
                                        {e.badge.clone().map(|b| view! { <span class="ui-badge ui-badge--compteur">{b}</span> })}
                                    </a>
                                </li>
                            }
                        }
                    />
                </ul>
                <div class="ui-coquille__pied">
                    {move || {
                        identite
                            .get()
                            .map(|i| {
                                view! {
                                    <div class="ui-coquille__compte">
                                        <Avatar nom=i.nom.clone() accent=true/>
                                        <span class="ui-coquille__identite">
                                            <strong>{i.nom}</strong>
                                            <span>{i.detail}</span>
                                        </span>
                                    </div>
                                }
                            })
                    }}
                    <button
                        type="button"
                        class="ui-coquille__lien ui-coquille__lien--bouton"
                        title="Se déconnecter"
                        on:click=move |_| on_deconnexion.run(())
                    >
                        <Icone nom=NomIcone::Deconnexion/>
                        <span class="ui-coquille__libelle">"Se déconnecter"</span>
                    </button>
                </div>
            </nav>
            <div class="ui-coquille__contenu">
                <main id="contenu" class="ui-coquille__principal" tabindex="-1">{children()}</main>
            </div>
        </div>
    }
}
