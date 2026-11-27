//! Modification du projet d'une sortie (depuis la fiche) : formulaire prérempli, envoi de
//! `PUT /voyages/{id}` avec la version lue. L'écran n'ajoute aucune règle : un refus du serveur
//! (403, 409, 422) est affiché tel quel ; un conflit de version propose de recharger.

use ent_types::voyages::SortieDetailDto;
use ent_web_ui::{
    Bouton, Case, ChampTexte, ResumeErreurs, Selection, Squelette, Variante, ZoneTexte,
};
use leptos::prelude::*;
use uuid::Uuid;

use super::modele::{SaisieVoyage, DESTINATIONS, FORMES, NATURES};
use super::modele_edition::{composer_modification, est_conflit_de_version, saisie_depuis};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::aujourdhui;

fn options(liste: &[(&str, &str)]) -> Vec<(String, String)> {
    liste
        .iter()
        .map(|(c, l)| ((*c).to_string(), (*l).to_string()))
        .collect()
}

/// Section « Modifier le projet » de la fiche (affichée seulement si `acces.modifier`).
#[component]
pub fn Edition(d: SortieDetailDto, recharger: Callback<()>) -> impl IntoView {
    let id = d.sortie.id;
    // Une écriture propre à la section : l'erreur s'affiche près du formulaire, pas en haut de la fiche.
    let ecriture = Ecriture::nouveau(recharger);
    let initial = saisie_depuis(&d);
    let deja: Vec<Uuid> = initial.classes.clone();
    let classes = LocalResource::new(|| async { session::client().classes(aujourdhui()).await });
    let titre = RwSignal::new(initial.titre.clone());
    let nature = RwSignal::new(initial.nature.clone());
    let forme = RwSignal::new(initial.forme.clone());
    let destination = RwSignal::new(initial.destination.clone());
    let pays = RwSignal::new(initial.pays.clone());
    let lieu = RwSignal::new(initial.lieu.clone());
    let objectifs = RwSignal::new(initial.objectifs.clone());
    let date_debut = RwSignal::new(initial.date_debut.clone());
    let heure_debut = RwSignal::new(initial.heure_debut.clone());
    let date_fin = RwSignal::new(initial.date_fin.clone());
    let heure_fin = RwSignal::new(initial.heure_fin.clone());
    let effectif = RwSignal::new(initial.effectif.clone());
    let adultes = RwSignal::new(initial.adultes.clone());
    let rdv = RwSignal::new(initial.point_rendez_vous.clone());
    let car = RwSignal::new(initial.besoin_car);
    let choisies = RwSignal::new(initial.classes.clone());
    let erreurs = RwSignal::new(Vec::<(String, String)>::new());
    let conflit = RwSignal::new(false);
    let erreur_de = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs
                .get()
                .into_iter()
                .find(|(c, _)| c == champ)
                .map(|(_, m)| m)
        })
    };
    let d_envoi = d.clone();
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisieVoyage {
            titre: titre.get_untracked(),
            nature: nature.get_untracked(),
            forme: forme.get_untracked(),
            destination: destination.get_untracked(),
            pays: pays.get_untracked(),
            lieu: lieu.get_untracked(),
            objectifs: objectifs.get_untracked(),
            date_debut: date_debut.get_untracked(),
            heure_debut: heure_debut.get_untracked(),
            date_fin: date_fin.get_untracked(),
            heure_fin: heure_fin.get_untracked(),
            effectif: effectif.get_untracked(),
            adultes: adultes.get_untracked(),
            point_rendez_vous: rdv.get_untracked(),
            besoin_car: car.get_untracked(),
            classes: choisies.get_untracked(),
        };
        match composer_modification(&saisie, &d_envoi) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                conflit.set(false);
                ecriture.lancer(async move {
                    match session::client().voyage_modifier(id, &corps).await {
                        Ok(s) => Ok(format!("Projet « {} » modifié.", s.titre)),
                        Err(e) => {
                            conflit.set(est_conflit_de_version(&e));
                            Err(e)
                        }
                    }
                });
            }
        }
    };
    view! {
        <section aria-labelledby="voy-modifier">
            <h2 id="voy-modifier" class="vs-section-titre">"Modifier le projet"</h2>
            <details class="voy-registre">
                <summary>"Modifier les informations du projet"</summary>
                <form class="voy-formulaire" on:submit=envoyer>
                    <p class="texte-attenue">
                        "Après la présentation du dossier à la direction, la nature, la forme, la destination et le besoin de car "
                        "ne se changent plus sans elle : le serveur le dira si besoin."
                    </p>
                    <ResumeErreurs erreurs=Signal::derive(move || erreurs.get())/>
                    <ChampTexte id="voy-mod-titre" libelle="Titre" valeur=titre requis=true erreur=erreur_de("voy-titre")/>
                    <Selection id="voy-mod-nature" libelle="Nature" valeur=nature options=options(&NATURES) requis=true erreur=erreur_de("voy-nature")/>
                    <Selection id="voy-mod-forme" libelle="Forme" valeur=forme options=options(&FORMES) requis=true erreur=erreur_de("voy-forme")/>
                    <Selection id="voy-mod-destination" libelle="Destination" valeur=destination options=options(&DESTINATIONS) requis=true erreur=erreur_de("voy-destination")/>
                    {move || (destination.get() == "etranger").then(|| view! {
                        <ChampTexte id="voy-mod-pays" libelle="Pays" valeur=pays requis=true erreur=erreur_de("voy-pays")/>
                    })}
                    <ChampTexte id="voy-mod-lieu" libelle="Lieu" valeur=lieu aide="Nom du lieu ou de la ville."/>
                    <ChampTexte id="voy-mod-date-debut" libelle="Date de départ" genre="date" valeur=date_debut requis=true erreur=erreur_de("voy-date-debut")/>
                    <ChampTexte id="voy-mod-heure-debut" libelle="Heure de départ" genre="time" valeur=heure_debut requis=true erreur=erreur_de("voy-heure-debut")/>
                    <ChampTexte id="voy-mod-date-fin" libelle="Date de retour" genre="date" valeur=date_fin requis=true erreur=erreur_de("voy-date-fin")/>
                    <ChampTexte id="voy-mod-heure-fin" libelle="Heure de retour" genre="time" valeur=heure_fin requis=true erreur=erreur_de("voy-heure-fin")/>
                    <ChampTexte id="voy-mod-effectif" libelle="Nombre d'élèves prévu" genre="number" valeur=effectif erreur=erreur_de("voy-effectif")/>
                    <ChampTexte id="voy-mod-adultes" libelle="Nombre d'adultes prévu" genre="number" valeur=adultes erreur=erreur_de("voy-adultes")/>
                    <ChampTexte id="voy-mod-rdv" libelle="Point de rendez-vous" valeur=rdv/>
                    <Case id="voy-mod-car" libelle="Un car est nécessaire" coche=car/>
                    <ZoneTexte id="voy-mod-objectifs" libelle="Objectifs pédagogiques" valeur=objectifs/>
                    <fieldset id="voy-mod-classes" class="voy-classes">
                        <legend>"Classes concernées"</legend>
                        {move || match classes.get() {
                            None => view! { <Squelette lignes=2 libelle="Chargement des classes…"/> }.into_any(),
                            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| classes.refetch())/> }.into_any(),
                            Some(Ok(l)) => {
                                let deja = deja.clone();
                                l.into_iter()
                                    .map(|c| {
                                        let cid = c.id;
                                        let coche = RwSignal::new(deja.contains(&cid));
                                        // Une classe absente de la liste (hors année courante) reste dans la sélection.
                                        Effect::new(move |_| {
                                            let c = coche.get();
                                            choisies.update(|v| {
                                                v.retain(|x| *x != cid);
                                                if c {
                                                    v.push(cid);
                                                }
                                            });
                                        });
                                        view! { <Case id=format!("voy-mod-classe-{cid}") libelle=c.libelle.clone() coche=coche/> }
                                    })
                                    .collect_view()
                                    .into_any()
                            }
                        }}
                        {move || erreur_de("voy-classes").get().map(|m| view! { <p class="ui-champ__erreur">{m}</p> })}
                    </fieldset>
                    <RetourEcriture ecriture=ecriture/>
                    {move || conflit.get().then(|| view! {
                        <div class="voy-conflit" role="status">
                            <p>"Le dossier a été modifié entre-temps. Rechargez-le pour retrouver la dernière version, puis refaites votre modification."</p>
                            <Bouton variante=Variante::Secondaire on_click=recharger>"Recharger le dossier"</Bouton>
                        </div>
                    })}
                    <Bouton soumettre=true en_cours=ecriture.en_cours>"Enregistrer les modifications"</Bouton>
                </form>
            </details>
        </section>
    }
}
