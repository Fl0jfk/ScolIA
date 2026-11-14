//! Palette de commandes (socle 29, lot P6) : Ctrl / ⌘ + K, « Aller à », « Faire », « Trouver ».
//!
//! * **Aller à** : exactement les entrées de la navigation de l'écran (même liste, mêmes droits).
//! * **Faire** : le registre des capacités (`ent_types::palette`) filtré par les droits de la personne ; une
//!   action **ouvre l'écran**, qui reste maître et revérifie côté serveur. Aucune logique métier ici.
//! * **Trouver** : la recherche globale du lot P5 (`GET /recherche`), côté serveur, après une pause de
//!   frappe et à partir de deux caractères ; le texte n'est jamais conservé.
//!
//! Accessibilité : motif ARIA « combobox » + « listbox », annonce du nombre de résultats, tout au clavier
//! (↑ ↓ Entrée Échap). Les « récents » restent en mémoire de la page : aucun stockage navigateur ni serveur.

pub mod modele;

use ent_types::palette::capacites_permises;
use ent_web_api::ApiError;
use ent_web_ui::{Bouton, Dialogue, EntreeNav, Variante};
use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use std::time::Duration;

use crate::noyau::droits_moteur::DroitsMoteur;
use crate::noyau::session;
use modele::{
    annonce, construire, deplacer, memoriser, ouvre_la_palette, recherche_utile, trouver,
    EntreePalette, Famille, RACCOURCIS,
};

/// Pause de frappe avant la recherche serveur (chaque recherche compte pour le plafond du jour).
const PAUSE_FRAPPE: Duration = Duration::from_millis(450);

/// Liste affichée : récents (saisie vide), puis « Faire », « Aller à », puis « Trouver ».
fn lignes(
    saisie: &str,
    pages: &[(String, String)],
    moteur: &DroitsMoteur,
    recents: &[EntreePalette],
    trouves: &[EntreePalette],
) -> Vec<EntreePalette> {
    let capacites = capacites_permises(|d| moteur.a(d));
    let mut v = Vec::new();
    if saisie.trim().is_empty() {
        v.extend(recents.iter().cloned());
    }
    v.extend(construire(saisie, pages, &capacites));
    v.extend(trouves.iter().cloned());
    v
}

#[component]
pub fn Palette(
    /// Entrées de navigation de l'écran (la source de « Aller à »).
    #[prop(into)]
    entrees: Signal<Vec<EntreeNav>>,
    #[prop(into)] moteur: Signal<DroitsMoteur>,
) -> impl IntoView {
    let ouvert = RwSignal::new(false);
    let saisie = RwSignal::new(String::new());
    let trouves = RwSignal::new(Vec::<EntreePalette>::new());
    let recents = RwSignal::new(Vec::<EntreePalette>::new());
    let actif = RwSignal::new(None::<usize>);
    let version = RwSignal::new(0u32);
    let suspendue = RwSignal::new(false);
    let navigate = use_navigate();

    let pages = Memo::new(move |_| {
        entrees
            .get()
            .into_iter()
            .map(|e| (e.libelle, e.href))
            .collect::<Vec<_>>()
    });
    let liste = Memo::new(move |_| {
        lignes(
            &saisie.get(),
            &pages.get(),
            &moteur.get(),
            &recents.get(),
            &trouves.get(),
        )
    });
    let message = Memo::new(move |_| annonce(liste.get().len()));

    // Ctrl / ⌘ + K, depuis n'importe où dans la page (navigateur seulement).
    Effect::new(move |_| {
        let ecoute = window_event_listener(ev::keydown, move |ev| {
            if ouvre_la_palette(
                &ev.key(),
                ev.ctrl_key(),
                ev.meta_key(),
                ev.alt_key(),
                ev.shift_key(),
            ) {
                ev.prevent_default();
                ouvert.update(|o| *o = !*o);
            }
        });
        on_cleanup(move || ecoute.remove());
    });

    // Fermeture : tout est remis à zéro (rien d'une recherche ne reste à l'écran).
    Effect::new(move |_| {
        if !ouvert.get() {
            saisie.set(String::new());
            trouves.set(Vec::new());
            actif.set(None);
            suspendue.set(false);
        }
    });

    // « Trouver » : après une pause de frappe, côté serveur, si le droit est là et la saisie utile.
    Effect::new(move |_| {
        let q = saisie.get();
        let v = version.get_untracked().wrapping_add(1);
        version.set(v);
        actif.set(None);
        suspendue.set(false);
        let Some(q) = recherche_utile(&q) else {
            trouves.set(Vec::new());
            return;
        };
        if !moteur.with_untracked(|m| m.a("recherche.globale")) || !ouvert.get_untracked() {
            trouves.set(Vec::new());
            return;
        }
        set_timeout(
            move || {
                if version.get_untracked() != v {
                    return;
                }
                spawn_local(async move {
                    let r = session::client().rechercher(&q).await;
                    if version.get_untracked() != v {
                        return;
                    }
                    match r {
                        Ok(rep) => trouves.set(trouver(&rep.resultats)),
                        Err(ApiError::TropDeRequetes) => {
                            trouves.set(Vec::new());
                            suspendue.set(true);
                        }
                        Err(_) => trouves.set(Vec::new()),
                    }
                });
            },
            PAUSE_FRAPPE,
        );
    });

    let activer = {
        let navigate = navigate.clone();
        Callback::new(move |e: EntreePalette| {
            recents.update(|r| memoriser(r, &e));
            ouvert.set(false);
            navigate(&e.lien, Default::default());
        })
    };

    let au_clavier = move |ev: ev::KeyboardEvent| {
        let n = liste.with_untracked(Vec::len);
        match ev.key().as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                actif.set(deplacer(actif.get_untracked(), 1, n));
            }
            "ArrowUp" => {
                ev.prevent_default();
                actif.set(deplacer(actif.get_untracked(), -1, n));
            }
            "Enter" => {
                ev.prevent_default();
                let i = actif.get_untracked().unwrap_or(0);
                if let Some(e) = liste.with_untracked(|l| l.get(i).cloned()) {
                    activer.run(e);
                }
            }
            _ => {}
        }
    };

    view! {
        <div class="palette-barre">
            <Bouton variante=Variante::Secondaire
                on_click=Callback::new(move |_| ouvert.set(true))>
                "Rechercher ou agir  (Ctrl K)"
            </Bouton>
        </div>
        <Dialogue ouvert=ouvert titre="Rechercher ou agir" id="palette">
            <input
                id="palette-saisie"
                class="ui-champ__controle palette-saisie"
                type="text"
                role="combobox"
                aria-label="Aller à, faire ou trouver"
                aria-autocomplete="list"
                aria-controls="palette-liste"
                aria-expanded="true"
                aria-keyshortcuts="Control+K Meta+K"
                autocomplete="off"
                placeholder="Une page, une action, un nom…"
                aria-activedescendant=move || actif.get().map(|i| format!("palette-option-{i}"))
                prop:value=move || saisie.get()
                on:input=move |ev| saisie.set(event_target_value(&ev))
                on:keydown=au_clavier
            />
            <p class="ui-sr" role="status" aria-live="polite">{move || message.get()}</p>
            {move || suspendue.get().then(|| view! {
                <p role="status">"La recherche est suspendue temporairement. Les pages et les actions restent disponibles."</p>
            })}
            <ul id="palette-liste" class="palette-liste" role="listbox" aria-label="Résultats">
                {move || {
                    let l = liste.get();
                    let mut precedente: Option<Famille> = None;
                    l.into_iter().enumerate().map(|(i, e)| {
                        let titre = (precedente != Some(e.famille)).then(|| {
                            view! { <li class="palette-famille" role="presentation">{e.famille.titre()}</li> }
                        });
                        precedente = Some(e.famille);
                        let pour_clic = e.clone();
                        view! {
                            {titre}
                            <li id=format!("palette-option-{i}") class="palette-option" role="option"
                                aria-selected=move || (actif.get() == Some(i)).to_string()
                                on:click=move |_| activer.run(pour_clic.clone())>
                                <span>{e.libelle}</span>
                            </li>
                        }
                    }).collect_view()
                }}
            </ul>
            <ul class="palette-aide" aria-label="Raccourcis clavier">
                {RACCOURCIS.iter().map(|(t, d)| view! { <li><kbd>{*t}</kbd>" "{*d}</li> }).collect_view()}
            </ul>
        </Dialogue>
    }
}
