//! Boîte de réception : liste des conversations, filtres, recherche côté client.

use ent_web_api::FilBoite;
use ent_web_ui::{
    Badge, Bouton, EnTetePage, EtatVide, LienBouton, OptionSegment, Recherche, Segmente, Selection,
    Squelette, TonEtat, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;

use super::modele::{
    dossiers, en_local, est_non_lu, filtrer, libelle_date_court, libelle_nombre,
    libelle_non_lus_fil, Filtres, Vue,
};
use super::NOUVEAU;
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::navigation;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;

#[component]
pub fn PageMessages() -> impl IntoView {
    let droits = use_droits();
    let vue = RwSignal::new(Vue::Actifs.cle().to_string());
    let recherche = RwSignal::new(String::new());
    let dossier = RwSignal::new(String::new());
    // Les archivés ne sont renvoyés par le serveur que sur demande (personnel).
    let archives = Memo::new(move |_| Vue::depuis(&vue.get()) == Vue::Archives);
    let fils = LocalResource::new(move || {
        let avec_archives = archives.get();
        async move { session::client().boite_messages(avec_archives).await }
    });

    view! {
        <Title text="Messages"/>
        <EnTetePage
            surtitre="Messagerie".to_string()
            titre="Messages".to_string()
            sous_titre="Vos conversations avec l'établissement.".to_string()
        >
            <Bouton
                variante=Variante::Secondaire
                libelle_accessible="Actualiser la liste des conversations"
                on_click=Callback::new(move |_| fils.refetch())
            >
                "Actualiser"
            </Bouton>
            <LienBouton href=NOUVEAU>"Nouveau message"</LienBouton>
        </EnTetePage>

        <form class="msg-outils" role="search" aria-label="Filtrer les conversations" on:submit=|ev| ev.prevent_default()>
            <Recherche
                id="msg-recherche"
                libelle="Rechercher une conversation"
                placeholder="Objet, élève ou dossier"
                valeur=recherche
            />
            {move || {
                let personnel = droits.get().messagerie_personnel;
                let mut options = vec![
                    OptionSegment::new(Vue::Actifs.cle(), "Toutes"),
                    OptionSegment::new(Vue::NonLus.cle(), "Non lues"),
                ];
                if personnel {
                    options.push(OptionSegment::new(Vue::Archives.cle(), "Archivées"));
                }
                view! { <Segmente nom="msg-vue" legende="Afficher" options=options valeur=vue/> }
            }}
            {move || {
                let liste = fils.get().and_then(Result::ok).unwrap_or_default();
                let ds = dossiers(&liste);
                (droits.get().messagerie_personnel && !ds.is_empty())
                    .then(|| {
                        let mut options = vec![(String::new(), "Tous les dossiers".to_string())];
                        options.extend(ds.into_iter().map(|d| (d.clone(), d)));
                        view! { <Selection id="msg-dossier" libelle="Dossier" valeur=dossier options=options/> }
                    })
            }}
        </form>

        {move || match fils.get() {
            None => view! { <Squelette lignes=5 libelle="Chargement des conversations…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| fils.refetch())/>
            }
                .into_any(),
            Some(Ok(liste)) => view! {
                <ListeFils fils=liste vue=vue recherche=recherche dossier=dossier/>
            }
                .into_any(),
        }}
    }
}

#[component]
fn ListeFils(
    fils: Vec<FilBoite>,
    vue: RwSignal<String>,
    recherche: RwSignal<String>,
    dossier: RwSignal<String>,
) -> impl IntoView {
    let droits = use_droits();
    let aucun_fil = fils.is_empty();
    let fils = StoredValue::new(fils);
    let visibles = Memo::new(move |_| {
        let filtres = Filtres {
            vue: Vue::depuis(&vue.get()),
            recherche: recherche.get(),
            dossier: dossier.get(),
            avec_archives: !droits.get().messagerie_personnel,
        };
        fils.with_value(|l| filtrer(l, &filtres))
    });

    view! {
        <p class="ui-sr" role="status" aria-live="polite" aria-atomic="true">
            {move || libelle_nombre(visibles.with(Vec::len))}
        </p>
        {move || {
            if visibles.with(Vec::is_empty) {
                let (titre, message) = if aucun_fil {
                    ("Aucune conversation", "Vous n'avez pas encore de message. Écrivez le premier.")
                } else if !recherche.with(|r| r.trim().is_empty()) || !dossier.with(String::is_empty) {
                    ("Aucun résultat", "Aucune conversation ne correspond à cette recherche.")
                } else {
                    match Vue::depuis(&vue.get()) {
                        Vue::NonLus => ("Tout est lu", "Aucune conversation ne contient de message non lu."),
                        Vue::Archives => ("Aucune archive", "Aucune conversation archivée."),
                        Vue::Actifs => ("Aucune conversation", "Vous n'avez pas de conversation en cours."),
                    }
                };
                view! {
                    <EtatVide titre=titre message=message>
                        <LienBouton href=NOUVEAU variante=Variante::Secondaire>"Nouveau message"</LienBouton>
                    </EtatVide>
                }
                    .into_any()
            } else {
                view! {
                    <ul class="msg-liste" aria-label="Conversations">
                        <For
                            each=move || visibles.get()
                            key=|f| (f.id, f.nb_non_lus, f.last_message_at.clone(), f.archive, f.priorite.clone(), f.dossier.clone())
                            children=|f| view! { <LigneFil fil=f/> }
                        />
                    </ul>
                }
                    .into_any()
            }
        }}
    }
}

#[component]
fn LigneFil(fil: FilBoite) -> impl IntoView {
    // Priorité et dossier sont des repères internes du personnel : la famille ne les voit pas.
    let personnel = use_droits().get_untracked().messagerie_personnel;
    let non_lu = est_non_lu(&fil);
    let date = en_local(&fil.last_message_at)
        .map(|d| libelle_date_court(d, aujourdhui()))
        .unwrap_or_default();
    let href = format!("{}/{}", navigation::MESSAGES, fil.id);
    let eleve = fil.eleve_label.clone().unwrap_or_default();
    view! {
        <li class="msg-ligne" class:msg-ligne--non-lu=non_lu>
            <a class="msg-ligne__lien" href=href>
                <span class="msg-ligne__principal">
                    <span class="msg-ligne__sujet">{fil.sujet.clone()}</span>
                    <span class="msg-ligne__detail">
                        {eleve}
                        {fil.dossier.clone().filter(|_| personnel).map(|d| format!(" · dossier {d}"))}
                    </span>
                </span>
                <span class="msg-ligne__meta">
                    <time class="msg-ligne__date" datetime=fil.last_message_at.clone()>{date}</time>
                    <span class="msg-ligne__badges">
                        {(personnel && fil.est_prioritaire()).then(|| view! { <Badge libelle="Prioritaire" ton=TonEtat::Avertissement/> })}
                        {fil.archive.then(|| view! { <Badge libelle="Archivée"/> })}
                        {non_lu.then(|| view! { <Badge libelle=libelle_non_lus_fil(fil.nb_non_lus) ton=TonEtat::Info/> })}
                    </span>
                </span>
            </a>
        </li>
    }
}
