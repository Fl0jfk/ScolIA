//! E9a — liste et recherche des élèves (`/web/eleves`).
//!
//! Pour qui : secrétariat, vie scolaire, direction, professeurs (leur périmètre). Ce qui est affiché
//! est décidé par le serveur ; la ligne ne porte que la désignation, la classe et le statut. Une
//! recherche (tolérante aux accents et aux fautes), des filtres (classe, statut, pastille) et la
//! pagination se font côté serveur : l'écran ne charge jamais tout l'annuaire.

use ent_types::eleve_synthese::{CompteurPastilleDto, EleveListeDto};
use ent_web_api::FiltreListeEleves;
use ent_web_ui::{Annonce, Avatar, Badge, Bouton, EnTetePage, EtatVide, FiltrePastille, Recherche, Squelette, Variante};
use leptos::ev;
use leptos::prelude::*;
use leptos_meta::Title;
use std::time::Duration;

use super::modele::{
    designation, libelle_statut, message_liste_vide, message_portee, page_precedente,
    page_suivante, recherche_a_envoyer, resume_page, ton_statut, ouvre_la_recherche, teinte_gravite,
    MESSAGE_TRONQUE, PAGE_TAILLE, STATUTS,
};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;

/// Délai entre la dernière frappe et la requête (la liste suit la frappe sans l'inonder).
const DELAI_FRAPPE_MS: u64 = 250;

#[component]
pub fn PageListeEleves() -> impl IntoView {
    let saisie = RwSignal::new(String::new());
    let recherche = RwSignal::new(String::new());
    let classe = RwSignal::new(String::new());
    let statut = RwSignal::new(String::new());
    let pastille = RwSignal::new(String::new());
    let decalage = RwSignal::new(0u32);
    let version = RwSignal::new(0u64);
    let derniere = RwSignal::new(None::<EleveListeDto>);
    let classes_connues = RwSignal::new(Vec::<String>::new());
    let annonce = RwSignal::new(String::new());

    // La recherche part après une pause de frappe ; une seule lettre ne cherche rien.
    Effect::new(move |_| {
        let s = saisie.get();
        let v = version.get_untracked().wrapping_add(1);
        version.set(v);
        set_timeout(
            move || {
                if version.get_untracked() != v {
                    return;
                }
                let envoyee = recherche_a_envoyer(&s);
                if recherche.get_untracked() != envoyee {
                    recherche.set(envoyee);
                }
            },
            Duration::from_millis(DELAI_FRAPPE_MS),
        );
    });

    // Un filtre qui change ramène à la première page.
    Effect::new(move |precedent: Option<(String, String, String, String)>| {
        let cle = (recherche.get(), classe.get(), statut.get(), pastille.get());
        if precedent.as_ref().is_some_and(|p| *p != cle) && decalage.get_untracked() != 0 {
            decalage.set(0);
        }
        cle
    });

    // Raccourci « / » : met le focus sur la recherche (jamais pendant une saisie ailleurs).
    Effect::new(move |_| {
        let ecoute = window_event_listener(ev::keydown, move |ev| {
            let en_saisie = cible_en_saisie(&ev);
            if ouvre_la_recherche(&ev.key(), ev.ctrl_key(), ev.meta_key(), ev.alt_key(), en_saisie) {
                ev.prevent_default();
                focaliser("eleves-recherche");
            }
        });
        on_cleanup(move || ecoute.remove());
    });

    let compteurs = LocalResource::new(|| async { session::client().eleves_compteurs_pastilles().await });
    let liste = LocalResource::new(move || {
        let f = FiltreListeEleves {
            q: recherche.get(),
            classe: classe.get(),
            statut: statut.get(),
            pastille: pastille.get(),
            limite: PAGE_TAILLE,
            decalage: decalage.get(),
        };
        async move { session::client().eleves_liste(&f).await }
    });
    Effect::new(move |_| {
        if let Some(Ok(l)) = liste.get() {
            annonce.set(resume_page(l.total, l.decalage, l.eleves.len()));
            classes_connues.update(|c| {
                for x in &l.classes {
                    if !c.contains(x) {
                        c.push(x.clone());
                    }
                }
                c.sort();
            });
            derniere.set(Some(l));
        }
    });
    let recharger = Callback::new(move |_| liste.refetch());
    let filtres_actifs = Memo::new(move |_| {
        !recherche.get().is_empty() || !classe.get().is_empty() || !statut.get().is_empty() || !pastille.get().is_empty()
    });
    let reinitialiser = Callback::new(move |_| {
        saisie.set(String::new());
        recherche.set(String::new());
        classe.set(String::new());
        statut.set(String::new());
        pastille.set(String::new());
        decalage.set(0);
    });

    view! {
        <Title text="Élèves"/>
        <EnTetePage
            surtitre="Établissement".to_string()
            titre="Élèves".to_string()
            sous_titre="Retrouvez un élève, puis ouvrez sa fiche. Vous ne voyez que ce que vos droits permettent.".to_string()
        />
        <Annonce message=annonce/>
        <section class="eleves-outils" aria-label="Recherche et filtres">
            <div class="eleves-outils__recherche">
                <Recherche
                    id="eleves-recherche"
                    libelle="Rechercher un élève"
                    valeur=saisie
                    placeholder="Nom, prénom ou classe"
                />
                <p class="eleves-aide">
                    "Appuyez sur "<kbd>"/"</kbd>" pour chercher. Les accents et les petites fautes de frappe sont tolérés."
                </p>
            </div>
            <div class="eleves-outils__selecteurs">
                <div class="ui-champ">
                    <label class="ui-champ__libelle" for="eleves-classe">"Classe"</label>
                    <select
                        id="eleves-classe"
                        class="ui-champ__controle"
                        on:change=move |ev| classe.set(event_target_value(&ev))
                        prop:value=move || classe.get()
                    >
                        <option value="">"Toutes les classes"</option>
                        <For
                            each=move || classes_connues.get()
                            key=|c| c.clone()
                            children=move |c| view! { <option value=c.clone()>{c.clone()}</option> }
                        />
                    </select>
                </div>
                <div class="ui-champ">
                    <label class="ui-champ__libelle" for="eleves-statut">"Statut"</label>
                    <select
                        id="eleves-statut"
                        class="ui-champ__controle"
                        on:change=move |ev| statut.set(event_target_value(&ev))
                        prop:value=move || statut.get()
                    >
                        {STATUTS
                            .iter()
                            .map(|(valeur, libelle)| view! { <option value=*valeur>{*libelle}</option> })
                            .collect_view()}
                    </select>
                </div>
                <Show when=move || filtres_actifs.get()>
                    <Bouton variante=Variante::Secondaire on_click=reinitialiser>"Effacer les filtres"</Bouton>
                </Show>
            </div>
            {move || match compteurs.get() {
                Some(Ok(c)) if !c.is_empty() => {
                    view! { <FiltresPastilles compteurs=c pastille=pastille/> }.into_any()
                }
                _ => ().into_any(),
            }}
        </section>
        {move || match (liste.get(), derniere.get()) {
            (Some(Err(e)), _) => {
                view! { <ErreurChargement erreur=e on_reessayer=recharger/> }.into_any()
            }
            (_, None) => view! { <Squelette lignes=8 libelle="Chargement des élèves…"/> }.into_any(),
            (en_cours, Some(l)) => {
                let occupe = en_cours.is_none();
                view! { <ResultatListe liste=l occupe=occupe decalage=decalage filtres_actifs=filtres_actifs.get() reinitialiser=reinitialiser/> }
                    .into_any()
            }
        }}
    }
}

/// Pastilles du catalogue que le lecteur peut voir, avec leur effectif : chacune est un filtre.
#[component]
fn FiltresPastilles(compteurs: Vec<CompteurPastilleDto>, pastille: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="eleves-pastilles" role="group" aria-label="Filtrer par pastille">
            {compteurs
                .into_iter()
                .filter(|c| c.nombre > 0)
                .map(|c| {
                    let code = c.code.clone();
                    let code_actif = c.code.clone();
                    let teinte = teinte_gravite(&c.gravite).to_string();
                    let nombre = usize::try_from(c.nombre).unwrap_or(usize::MAX);
                    view! {
                        <FiltrePastille
                            libelle=c.libelle.clone()
                            nombre=Signal::derive(move || nombre)
                            actif=Signal::derive(move || pastille.get() == code_actif)
                            teinte=teinte
                            on_click=Callback::new(move |_| {
                                let c = code.clone();
                                pastille.update(|p| *p = if *p == c { String::new() } else { c });
                            })
                        />
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn ResultatListe(
    liste: EleveListeDto,
    occupe: bool,
    decalage: RwSignal<u32>,
    filtres_actifs: bool,
    reinitialiser: Callback<()>,
) -> impl IntoView {
    let resume = resume_page(liste.total, liste.decalage, liste.eleves.len());
    let portee = message_portee(&liste.portee);
    let precedente = page_precedente(decalage.get_untracked(), PAGE_TAILLE);
    let suivante = page_suivante(decalage.get_untracked(), PAGE_TAILLE, liste.total);
    if liste.eleves.is_empty() {
        let (titre, message) = message_liste_vide(filtres_actifs);
        return view! {
            <EtatVide titre=titre message=message>
                {filtres_actifs
                    .then(|| view! { <Bouton variante=Variante::Secondaire on_click=reinitialiser>"Effacer les filtres"</Bouton> })}
            </EtatVide>
            {portee.map(|m| view! { <p class="eleves-portee">{m}</p> })}
        }
        .into_any();
    }
    let tronque = liste.tronque;
    view! {
        <div class="ui-tableau-cadre" aria-busy=occupe.then_some("true")>
            <div class="ui-tableau-outils">
                <span class="eleves-compteur">{resume}</span>
            </div>
            <div class="ui-tableau-defile">
                <table class="ui-tableau eleves-tableau" class:eleves-tableau--occupe=occupe>
                    <caption class="ui-sr">"Liste des élèves"</caption>
                    <thead>
                        <tr>
                            <th scope="col">"Élève"</th>
                            <th scope="col">"Classe"</th>
                            <th scope="col">"Statut"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || liste.eleves.clone()
                            key=|e| format!("{e:?}")
                            children=move |e| {
                                let nom = designation(&e.prenom, &e.nom);
                                let href = format!("{}/{}", navigation::ELEVES, e.id);
                                let classe = e.classe.clone().filter(|c| !c.trim().is_empty());
                                view! {
                                    <tr>
                                        <th scope="row" class="eleves-nom">
                                            <Avatar nom=nom.clone()/>
                                            <a href=href>{nom.clone()}</a>
                                        </th>
                                        <td data-label="Classe">{classe.unwrap_or_else(|| "—".to_string())}</td>
                                        <td data-label="Statut">
                                            <Badge libelle=libelle_statut(&e.statut) ton=ton_statut(&e.statut)/>
                                        </td>
                                    </tr>
                                }
                            }
                        />
                    </tbody>
                </table>
            </div>
            <nav class="eleves-pagination" aria-label="Pagination de la liste">
                <Bouton
                    variante=Variante::Secondaire
                    desactive=precedente.is_none()
                    on_click=Callback::new(move |_| {
                        if let Some(d) = precedente {
                            decalage.set(d);
                        }
                    })
                >
                    "Page précédente"
                </Bouton>
                <Bouton
                    variante=Variante::Secondaire
                    desactive=suivante.is_none()
                    on_click=Callback::new(move |_| {
                        if let Some(d) = suivante {
                            decalage.set(d);
                        }
                    })
                >
                    "Page suivante"
                </Bouton>
            </nav>
        </div>
        {tronque.then(|| view! { <p class="eleves-portee" role="note">{MESSAGE_TRONQUE}</p> })}
        {portee.map(|m| view! { <p class="eleves-portee">{m}</p> })}
    }
    .into_any()
}

/// La cible de l'évènement est-elle une zone de saisie (champ, liste, zone de texte, contenu éditable) ?
pub(super) fn cible_en_saisie(ev: &leptos::ev::KeyboardEvent) -> bool {
    use leptos::wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        .is_some_and(|el| {
            let editable = el.get_attribute("contenteditable").is_some_and(|v| v != "false");
            super::modele::est_zone_de_saisie(&el.tag_name(), editable)
        })
}

/// Met le focus sur l'élément d'identifiant donné (navigateur seulement ; sans effet au rendu serveur).
pub(super) fn focaliser(id: &str) {
    use leptos::wasm_bindgen::JsCast;
    if let Some(el) = document().get_element_by_id(id).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
        let _ = el.focus();
    }
}
