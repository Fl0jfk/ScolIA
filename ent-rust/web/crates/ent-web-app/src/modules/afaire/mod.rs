//! Écran « À faire » (socle 29, lot P3) : la seule liste transverse de ce que la personne doit faire,
//! regroupée par enfant, avec des cartes d'action en deux gestes (aperçu, puis confirmation).
//! Route : `GET /afaire`, `POST /afaire/{id}/apercu`, `POST /afaire/{id}/executer`.
//! Le serveur décide de tout (droits, neutralité entre parents, plage calme) ; l'écran n'affiche
//! que ce qu'il reçoit, jamais de saisie à l'ouverture d'une adresse.

pub mod modele;

use ent_types::afaire::{AFaire, ActionTache, ApercuAction, GroupeAfaire, TacheAfaire};
use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Bouton, ChampTexte, EnTetePage, EtatVide, Niveau, Selection, Squelette, Taille,
    Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::noyau::ecriture::{message_echec, Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use modele::{
    decision_valide, est_actionnable, libelle_choix, libelle_echeance, libelle_priorite,
    mention_traitee, motif_valide, titre_compteur,
};

#[component]
pub fn PageAFaire() -> impl IntoView {
    let liste = LocalResource::new(|| async { session::client().a_faire().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| liste.refetch()));
    view! {
        <Title text="À faire"/>
        <EnTetePage
            titre="À faire".to_string()
            sous_titre="Ce qui vous attend, enfant par enfant.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || match liste.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de vos démarches…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liste.refetch())/>
            }.into_any(),
            Some(Ok(a)) => view! { <Contenu a_faire=a ecriture=ecriture/> }.into_any(),
        }}
    }
}

#[component]
fn Contenu(a_faire: AFaire, ecriture: Ecriture) -> impl IntoView {
    let AFaire {
        groupes,
        compteur,
        mention,
        ..
    } = a_faire;
    // Une seule carte ouverte à la fois : l'aperçu d'une tâche remplace celui d'une autre.
    let ouverte = RwSignal::new(None::<Uuid>);
    view! {
        <p class="ui-liste__detail" role="status">{titre_compteur(compteur)}</p>
        {mention.map(|m| view! { <p class="texte-attenue">{m}</p> })}
        {if groupes.is_empty() {
            view! { <EtatVide titre="Aucune démarche en cours"/> }.into_any()
        } else {
            groupes.into_iter().map(|g| view! {
                <Groupe groupe=g ouverte=ouverte ecriture=ecriture/>
            }).collect_view().into_any()
        }}
    }
}

#[component]
fn Groupe(
    groupe: GroupeAfaire,
    ouverte: RwSignal<Option<Uuid>>,
    ecriture: Ecriture,
) -> impl IntoView {
    let GroupeAfaire {
        eleve_prenom,
        taches,
        ..
    } = groupe;
    let titre = eleve_prenom.unwrap_or_else(|| "Pour vous".to_string());
    let id_titre = format!("afaire-{}", titre.to_lowercase().replace(' ', "-"));
    let pour = id_titre.clone();
    view! {
        <section class="classe-section" aria-labelledby=pour>
            <h2 id=id_titre>{titre}</h2>
            <ul class="ui-liste">
                {taches.into_iter().map(|t| view! {
                    <Carte tache=t ouverte=ouverte ecriture=ecriture/>
                }).collect_view()}
            </ul>
        </section>
    }
}

#[component]
fn Carte(tache: TacheAfaire, ouverte: RwSignal<Option<Uuid>>, ecriture: Ecriture) -> impl IntoView {
    let id = tache.id;
    let maintenant = chrono::Utc::now();
    let echeance = libelle_echeance(tache.echeance_le, maintenant);
    let priorite = libelle_priorite(tache.priorite);
    let traitee = mention_traitee(&tache);
    let actionnable = est_actionnable(&tache);
    let nouveau = tache.nouveau;
    let action = tache.action.clone();
    view! {
        <li class="ui-liste__element">
            <div class="ui-liste__principal">
                <span class="ui-liste__titre">
                    {tache.titre.clone()}
                    {nouveau.then(|| view! { <span class="ui-badge">" Nouveau"</span> })}
                </span>
                <span class="ui-liste__detail">
                    {priorite.map(|p| format!("{p} · ")).unwrap_or_default()}
                    {echeance.unwrap_or_default()}
                    {tache.annee.clone().map(|a| format!(" · {a}")).unwrap_or_default()}
                </span>
                {traitee.map(|m| view! { <span class="texte-attenue">{m}</span> })}
            </div>
            {match (actionnable, action) {
                (true, Some(ActionTache::Lien { libelle, lien })) => view! {
                    <a class="ui-bouton ui-bouton--secondaire" href=lien>{libelle}</a>
                }.into_any(),
                (true, Some(ActionTache::Carte { libelle })) => view! {
                    <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                        libelle_accessible=format!("{libelle} : {}", tache.titre)
                        on_click=Callback::new(move |_| ouverte.set(Some(id)))>
                        {libelle}
                    </Bouton>
                }.into_any(),
                _ => ().into_any(),
            }}
        </li>
        {move || (ouverte.get() == Some(id)).then(|| view! {
            <li class="ui-liste__element"><PanneauAction tache_id=id ouverte=ouverte ecriture=ecriture/></li>
        })}
    }
}

/// Aperçu puis confirmation : rien n'est écrit tant que l'utilisateur n'a pas confirmé.
#[component]
fn PanneauAction(
    tache_id: Uuid,
    ouverte: RwSignal<Option<Uuid>>,
    ecriture: Ecriture,
) -> impl IntoView {
    let apercu =
        LocalResource::new(move || async move { session::client().a_faire_apercu(tache_id).await });
    let motif = RwSignal::new(String::new());
    let decision = RwSignal::new(String::new());
    let erreur = RwSignal::new(None::<String>);
    view! {
        {move || match apercu.get() {
            None => view! { <Squelette lignes=2 libelle="Préparation de l'aperçu…"/> }.into_any(),
            Some(Err(ApiError::Conflit(m))) => view! {
                <Alerte niveau=Niveau::Info titre=m>
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| {
                        ouverte.set(None);
                    })>"Fermer"</Bouton>
                </Alerte>
            }.into_any(),
            Some(Err(e)) => view! { <Alerte niveau=Niveau::Erreur titre=message_echec(e)/> }.into_any(),
            Some(Ok(a)) => {
                let ApercuAction { texte, champs, .. } = a;
                let champs_saisie = champs.clone();
                let a_motif = champs.iter().any(|c| c.code == "motif");
                // Carte à choix (autorisation, consentement) : liste fermée, jamais pré-sélectionnée.
                let options_decision = champs
                    .iter()
                    .find(|c| c.code == "decision")
                    .map(|c| {
                        std::iter::once((String::new(), "Choisir…".to_string()))
                            .chain(c.choix.iter().map(|v| (v.clone(), libelle_choix(v).to_string())))
                            .collect::<Vec<_>>()
                    });
                view! {
                    <div role="group" aria-label="Confirmer l'action">
                        <p>{texte}</p>
                        {a_motif.then(|| view! {
                            <ChampTexte id=format!("motif-{tache_id}") libelle="Motif de l'absence" valeur=motif requis=true/>
                        })}
                        {options_decision.map(|options| view! {
                            <Selection id=format!("decision-{tache_id}") libelle="Votre réponse"
                                valeur=decision options=options requis=true/>
                        })}
                        {move || erreur.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
                        <Bouton variante=Variante::Primaire on_click=Callback::new(move |_| {
                            let choix = decision_valide(&champs_saisie, &decision.get_untracked());
                            match motif_valide(&champs_saisie, &motif.get_untracked())
                                .and_then(|m| choix.map(|d| (m, d)))
                            {
                                Err(m) => erreur.set(Some(m.to_string())),
                                Ok((m, d)) => {
                                    erreur.set(None);
                                    ouverte.set(None);
                                    ecriture.lancer(async move {
                                        session::client()
                                            .a_faire_executer(tache_id, m, d)
                                            .await
                                            .map(|r| r.message)
                                    });
                                }
                            }
                        })>"Confirmer"</Bouton>
                        <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| ouverte.set(None))>
                            "Annuler"
                        </Bouton>
                    </div>
                }.into_any()
            }
        }}
    }
}
