//! Recherche globale (socle 29, lot P5) : un champ, des résultats groupés par source, un lien par résultat.
//! Route : `GET /recherche?q=`. Tout se passe **côté serveur** (droits, plafonds, protections) : aucune liste
//! n'est envoyée au navigateur pour un classement local. Le texte saisi n'est ni stocké ni journalisé.
//! `/web/blocages-recherche` : l'administrateur voit les recherches bloquées et les lève.

pub mod modele;

use ent_types::recherche::ReponseRecherche;
use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Bouton, ChampTexte, EnTetePage, EtatVide, Niveau, Squelette, Taille, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;

use crate::noyau::ecriture::{message_echec, Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use modele::{annonce, grouper, texte_valide, titre_source};

#[component]
pub fn PageRecherche() -> impl IntoView {
    let saisie = RwSignal::new(String::new());
    let etat = RwSignal::new(None::<Result<ReponseRecherche, String>>);
    let en_cours = RwSignal::new(false);
    let message = RwSignal::new(String::new());
    let lancer = move || {
        let q = match texte_valide(&saisie.get_untracked()) {
            Ok(q) => q,
            Err(m) => {
                etat.set(Some(Err(m.to_string())));
                return;
            }
        };
        if en_cours.get_untracked() {
            return;
        }
        en_cours.set(true);
        spawn_local(async move {
            let r = session::client().rechercher(&q).await;
            match r {
                Ok(rep) => {
                    message.set(annonce(rep.resultats.len()));
                    etat.set(Some(Ok(rep)));
                }
                Err(ApiError::TropDeRequetes) => etat.set(Some(Err(
                    "La recherche est suspendue temporairement. Réessayez plus tard ou contactez l'administrateur.".to_string(),
                ))),
                Err(e) => etat.set(Some(Err(message_echec(e)))),
            }
            en_cours.set(false);
        });
    };
    view! {
        <Title text="Recherche"/>
        <EnTetePage
            titre="Recherche".to_string()
            sous_titre="Élèves de votre portée, responsables, personnels, classes et salles.".to_string()
        />
        <form on:submit=move |ev| { ev.prevent_default(); lancer(); }>
            <ChampTexte id="recherche-globale" libelle="Rechercher" valeur=saisie genre="search"
                autocomplete="off" aide="Au moins deux caractères ; accents et majuscules sans importance."/>
            <Bouton variante=Variante::Primaire desactive=Signal::derive(move || en_cours.get())
                on_click=Callback::new(move |_| lancer())>"Rechercher"</Bouton>
        </form>
        <p class="visuellement-cache" role="status" aria-live="polite">{move || message.get()}</p>
        {move || match etat.get() {
            None => ().into_any(),
            Some(Err(m)) => view! { <Alerte niveau=Niveau::Avertissement titre=m/> }.into_any(),
            Some(Ok(r)) if r.resultats.is_empty() => view! {
                <EtatVide titre="Aucun résultat"/>
            }.into_any(),
            Some(Ok(r)) => grouper(r.resultats).into_iter().map(|(source, liste)| view! {
                <section aria-label=titre_source(source)>
                    <h2>{titre_source(source)}</h2>
                    <ul class="ui-liste">
                        {liste.into_iter().map(|x| view! {
                            <li class="ui-liste__element">
                                <div class="ui-liste__principal"><span class="ui-liste__titre">{x.libelle}</span></div>
                                {x.lien.map(|l| view! {
                                    <a class="ui-bouton ui-bouton--secondaire" href=l>"Ouvrir"</a>
                                })}
                            </li>
                        }).collect_view()}
                    </ul>
                </section>
            }).collect_view().into_any(),
        }}
    }
}

#[component]
pub fn PageBlocagesRecherche() -> impl IntoView {
    let liste = LocalResource::new(|| async { session::client().recherche_blocages().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| liste.refetch()));
    view! {
        <Title text="Recherches bloquées"/>
        <EnTetePage
            titre="Recherches bloquées".to_string()
            sous_titre="Plafond quotidien ou recherche systématique de lettres. Aucun texte de recherche n'est conservé.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || match liste.get() {
            None => view! { <Squelette lignes=2 libelle="Chargement…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/>
            }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! { <EtatVide titre="Aucune recherche bloquée"/> }.into_any(),
            Some(Ok(l)) => view! {
                <ul class="ui-liste">
                    {l.into_iter().map(|b| {
                        let id = b.compte_id;
                        view! {
                            <li class="ui-liste__element">
                                <div class="ui-liste__principal">
                                    <span class="ui-liste__titre">{b.nom}</span>
                                    <span class="ui-liste__detail">{format!("{} recherches aujourd'hui", b.recherches_du_jour)}</span>
                                </div>
                                <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                                    on_click=Callback::new(move |_| ecriture.lancer(async move {
                                        session::client().recherche_debloquer(id).await
                                            .map(|()| "Blocage levé.".to_string())
                                    }))>"Lever le blocage"</Bouton>
                            </li>
                        }
                    }).collect_view()}
                </ul>
            }.into_any(),
        }}
    }
}
