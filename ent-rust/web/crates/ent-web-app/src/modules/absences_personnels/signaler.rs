//! Signaler une absence (ABP-102, 103) : l'accueil note un appel téléphonique, la vie scolaire constate
//! qu'un agent n'est pas là à 8 h 05. **Aucune nature** : l'organisation s'en saisit tout de suite, et
//! l'agent est invité à confirmer (ABP-112). Le serveur n'annonce rien à l'extérieur tant que la
//! personne n'a pas confirmé (R-AP46).

use ent_types::absences_personnels::DeclarationDto;
use ent_types::personnels::AgentResumeDto;
use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Bouton, ChampTexte, LienBouton, Niveau, ResumeErreurs, Selection, Squelette, Variante,
};
use leptos::prelude::*;
use leptos_meta::Title;

use super::commun::GardePersonnel;
use super::modele::{composer_signalement, ErreursSaisie, SaisieSignalement};
use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;

#[component]
pub fn PageSignalerAbsence() -> impl IntoView {
    view! {
        <Title text="Signaler une absence"/>
        <GardePersonnel
            titre="Signaler une absence"
            sous_titre="Une personne est absente et n'a pas encore déclaré son absence : notez-le, sans motif."
        >
            <Contenu/>
        </GardePersonnel>
    }
}

#[component]
fn Contenu() -> impl IntoView {
    let annuaire = LocalResource::new(|| async {
        let mut a = session::client().absp_annuaire().await?;
        a.retain(|x| x.etat == "actif");
        a.sort_by(|x, y| x.nom_affiche.cmp(&y.nom_affiche));
        Ok::<Vec<AgentResumeDto>, ApiError>(a)
    });
    view! {
        {move || match annuaire.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement de l'annuaire…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| annuaire.refetch())/> }.into_any(),
            Some(Ok(a)) => view! { <Formulaire agents=a/> }.into_any(),
        }}
    }
}

#[component]
fn Formulaire(agents: Vec<AgentResumeDto>) -> impl IntoView {
    let fait = RwSignal::new(None::<DeclarationDto>);
    let ecriture = Ecriture::nouveau(Callback::new(|_| {}));
    let agent = RwSignal::new(String::new());
    let debut = RwSignal::new(String::new());
    let fin = RwSignal::new(String::new());
    let heure_debut = RwSignal::new(String::new());
    let heure_fin = RwSignal::new(String::new());
    let erreurs = RwSignal::new(ErreursSaisie::new());
    let mut options = vec![(String::new(), "Choisir…".to_string())];
    options.extend(
        agents
            .into_iter()
            .map(|a| (a.agent_id.to_string(), a.nom_affiche)),
    );
    let erreur_de = move |champ: &'static str| {
        Signal::derive(move || {
            erreurs
                .get()
                .into_iter()
                .find(|(c, _)| c == champ)
                .map(|(_, m)| m)
        })
    };
    let envoyer = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let saisie = SaisieSignalement {
            agent: agent.get_untracked(),
            debut: debut.get_untracked(),
            fin: fin.get_untracked(),
            heure_debut: heure_debut.get_untracked(),
            heure_fin: heure_fin.get_untracked(),
        };
        match composer_signalement(&saisie) {
            Err(e) => erreurs.set(e),
            Ok(corps) => {
                erreurs.set(Vec::new());
                ecriture.lancer(async move {
                    let d = session::client().absp_signaler(&corps).await?;
                    fait.set(Some(d));
                    Ok("Absence signalée : la personne sera invitée à la confirmer.".to_string())
                });
            }
        }
    };
    view! {
        <RetourEcriture ecriture=ecriture/>
        {move || match fait.get() {
            Some(_) => view! {
                <Alerte niveau=Niveau::Succes titre="Absence signalée">
                    <p>"L'adjoint et la vie scolaire peuvent organiser les remplacements. La personne est invitée à confirmer ; rien n'est annoncé aux familles avant sa confirmation."</p>
                </Alerte>
                <p class="absp-actions">
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| fait.set(None))>"Signaler une autre absence"</Bouton>
                    <LienBouton href="/web/personnel/ce-matin" variante=Variante::Secondaire>"Voir les remplacements du jour"</LienBouton>
                </p>
            }
            .into_any(),
            None => view! {
                <form class="absp-formulaire" on:submit=envoyer novalidate=true>
                    <ResumeErreurs erreurs=Signal::derive(move || erreurs.get())/>
                    <Selection id="sig-agent" libelle="Personne absente" valeur=agent options=options.clone() requis=true erreur=erreur_de("sig-agent")/>
                    <div class="absp-dates">
                        <ChampTexte id="sig-debut" libelle="Premier jour" genre="date" valeur=debut requis=true erreur=erreur_de("sig-debut")/>
                        <ChampTexte
                            id="sig-fin"
                            libelle="Dernier jour (facultatif)"
                            genre="date"
                            valeur=fin
                            aide="Sans dernier jour : la même journée."
                            erreur=erreur_de("sig-fin")
                        />
                    </div>
                    <div class="absp-dates">
                        <ChampTexte id="sig-heure-debut" libelle="À partir de (facultatif)" genre="time" valeur=heure_debut erreur=erreur_de("sig-heure-debut")/>
                        <ChampTexte id="sig-heure-fin" libelle="Jusqu'à (facultatif)" genre="time" valeur=heure_fin erreur=erreur_de("sig-heure-fin")/>
                    </div>
                    <p class="texte-attenue">"Ne notez aucun motif : la personne le précisera elle-même, et seules les personnes qui instruisent le liront."</p>
                    <Bouton soumettre=true en_cours=ecriture.en_cours>"Signaler l'absence"</Bouton>
                </form>
            }
            .into_any(),
        }}
    }
}
