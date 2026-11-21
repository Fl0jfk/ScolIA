//! « Mes liens à confirmer » (socle 2) : la personne déclarée responsable d'un élève (par
//! invitation, saisie du secrétariat ou inscription) confirme ou refuse. Aucun accès n'est ouvert
//! avant sa réponse. Routes : `GET /dossier/mes-liens-en-attente`, `POST …/{id}/reponse`.

pub mod modele;

use ent_web_api::MonLienEnAttenteDto;
use ent_web_ui::{Bouton, EnTetePage, EtatVide, Squelette, Taille, Variante};
use leptos::prelude::*;
use leptos_meta::Title;
use uuid::Uuid;

use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use modele::{libelle, libelle_bouton, message_issue, trier};

#[component]
pub fn PageMesLiens() -> impl IntoView {
    let liens = LocalResource::new(|| async { session::client().mes_liens_en_attente().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| liens.refetch()));
    view! {
        <Title text="Mes liens à confirmer"/>
        <EnTetePage
            titre="Mes liens à confirmer".to_string()
            sous_titre="L'établissement vous a déclaré(e) responsable d'un élève. Vérifiez et répondez.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || match liens.get() {
            None => view! { <Squelette lignes=3 libelle="Chargement des confirmations…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| liens.refetch())/>
            }.into_any(),
            Some(Ok(l)) if l.is_empty() => view! {
                <EtatVide titre="Aucune confirmation en attente"/>
            }.into_any(),
            Some(Ok(l)) => view! { <ListeLiens liens=trier(l) ecriture=ecriture/> }.into_any(),
        }}
    }
}

fn repondre(ecriture: Ecriture, id: Uuid, accepter: bool) {
    ecriture.lancer(async move {
        session::client()
            .repondre_lien_en_attente(id, accepter)
            .await
            .map(|()| message_issue(accepter).to_string())
    });
}

#[component]
fn ListeLiens(liens: Vec<MonLienEnAttenteDto>, ecriture: Ecriture) -> impl IntoView {
    view! {
        <ul class="ui-liste">
            {liens.into_iter().map(|l| {
                let phrase = libelle(&l);
                let confirmer = libelle_bouton("Confirmer", &l);
                let refuser = libelle_bouton("Ce n'est pas moi / Refuser", &l);
                let id = l.id;
                view! {
                    <li class="ui-liste__element">
                        <div class="ui-liste__principal"><span class="ui-liste__titre">{phrase}</span></div>
                        <Bouton variante=Variante::Primaire taille=Taille::Compacte
                            desactive=Signal::derive(move || ecriture.en_cours.get())
                            libelle_accessible=confirmer
                            on_click=Callback::new(move |_| repondre(ecriture, id, true))>
                            "Confirmer"
                        </Bouton>
                        <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                            desactive=Signal::derive(move || ecriture.en_cours.get())
                            libelle_accessible=refuser
                            on_click=Callback::new(move |_| repondre(ecriture, id, false))>
                            "Ce n'est pas moi / Refuser"
                        </Bouton>
                    </li>
                }
            }).collect_view()}
        </ul>
    }
}
