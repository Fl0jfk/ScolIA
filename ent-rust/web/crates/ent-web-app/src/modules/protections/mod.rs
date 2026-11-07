//! Écran direction « Protections » : adresses à protéger mises de côté et déclarations
//! « ne pas solliciter » à confirmer. Routes : `GET /dossier/alertes-protection`,
//! `POST /dossier/adresses-protegees/{id}/ouvrir` (journalisé), `POST /dossier/personnes/{id}/ne-pas-solliciter`.
//! L'adresse ouverte n'est gardée que le temps de l'alerte affichée : fermer l'alerte l'efface.

pub mod modele;

use ent_web_api::{AdresseParqueeDto, AlertesProtectionDto, ApiError, ProtectionADeclarerDto};
use ent_web_ui::{
    AccesRefuse, Alerte, Bouton, EnTetePage, EtatVide, Niveau, Squelette, Taille, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use uuid::Uuid;

use crate::noyau::ecriture::{message_echec, Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use modele::{
    libelle_origine, message_ne_pas_solliciter, nom_affiche, trier_adresses, trier_declarations,
    MESSAGE_RESERVE_DIRECTION,
};

#[component]
pub fn PageProtections() -> impl IntoView {
    let alertes = LocalResource::new(|| async { session::client().alertes_protection().await });
    let ecriture = Ecriture::nouveau(Callback::new(move |_| alertes.refetch()));
    view! {
        <Title text="Protections"/>
        <EnTetePage
            titre="Protections".to_string()
            surtitre="Direction".to_string()
            sous_titre="Adresses à protéger et personnes à ne pas solliciter.".to_string()
        />
        <RetourEcriture ecriture=ecriture/>
        {move || match alertes.get() {
            None => view! { <Squelette lignes=4 libelle="Chargement des protections…"/> }.into_any(),
            Some(Err(ApiError::Interdit(_))) => view! {
                <AccesRefuse message=MESSAGE_RESERVE_DIRECTION/>
            }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| alertes.refetch())/>
            }.into_any(),
            Some(Ok(a)) => view! { <Contenu alertes=a ecriture=ecriture/> }.into_any(),
        }}
    }
}

#[component]
fn Contenu(alertes: AlertesProtectionDto, ecriture: Ecriture) -> impl IntoView {
    let adresses = trier_adresses(alertes.adresses_a_traiter);
    let declarations = trier_declarations(alertes.ne_pas_solliciter_a_confirmer);
    view! {
        <section class="classe-section" aria-labelledby="prot-adresses">
            <h2 id="prot-adresses">"Adresses à protéger"</h2>
            {if adresses.is_empty() {
                view! { <EtatVide titre="Aucune adresse à traiter"/> }.into_any()
            } else {
                view! { <ListeAdresses adresses=adresses/> }.into_any()
            }}
        </section>
        <section class="classe-section" aria-labelledby="prot-nps">
            <h2 id="prot-nps">"« Ne pas solliciter » à confirmer"</h2>
            {if declarations.is_empty() {
                view! { <EtatVide titre="Aucune déclaration à confirmer"/> }.into_any()
            } else {
                view! { <ListeDeclarations declarations=declarations ecriture=ecriture/> }.into_any()
            }}
        </section>
    }
}

#[component]
fn ListeAdresses(adresses: Vec<AdresseParqueeDto>) -> impl IntoView {
    // Adresse affichée : uniquement le temps de l'alerte (jamais rechargée, jamais stockée).
    let affichee = RwSignal::new(None::<(String, String)>);
    let erreur = RwSignal::new(None::<String>);
    let en_cours = RwSignal::new(false);
    let ouvrir = move |id: Uuid, nom: String| {
        if en_cours.get_untracked() {
            return;
        }
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            match session::client().ouvrir_adresse_protegee(id).await {
                Ok(a) => affichee.set(Some((nom, a.adresse))),
                Err(e) => erreur.set(Some(message_echec(e))),
            }
            en_cours.set(false);
        });
    };
    view! {
        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
        {move || affichee.get().map(|(nom, adresse)| view! {
            <Alerte niveau=Niveau::Avertissement titre=format!("Adresse protégée de {nom}")>
                <p>{adresse}</p>
                <p class="texte-attenue">"Cette consultation est journalisée. Ne la recopiez pas."</p>
                <p>
                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| affichee.set(None))>
                        "Fermer"
                    </Bouton>
                </p>
            </Alerte>
        })}
        <ul class="ui-liste">
            {adresses.into_iter().map(|a| {
                let nom = nom_affiche(&a.nom, a.prenom.as_deref());
                let detail = format!(
                    "{} · {}",
                    libelle_origine(&a.origine),
                    a.created_at.format("%d/%m/%Y")
                );
                let (id, pour, acces) = (a.id, nom.clone(), format!("Ouvrir l'adresse de {nom}"));
                view! {
                    <li class="ui-liste__element">
                        <div class="ui-liste__principal">
                            <span class="ui-liste__titre">{nom}</span>
                            <span class="ui-liste__detail">{detail}</span>
                        </div>
                        <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                            desactive=Signal::derive(move || en_cours.get())
                            libelle_accessible=acces
                            on_click=Callback::new(move |_| ouvrir(id, pour.clone()))>
                            "Ouvrir l'adresse"
                        </Bouton>
                    </li>
                }
            }).collect_view()}
        </ul>
    }
}

fn definir(ecriture: Ecriture, personne: Uuid, nom: String, actif: bool) {
    ecriture.lancer(async move {
        session::client()
            .definir_ne_pas_solliciter(personne, actif, None)
            .await
            .map(|()| message_ne_pas_solliciter(actif, &nom))
    });
}

#[component]
fn ListeDeclarations(
    declarations: Vec<ProtectionADeclarerDto>,
    ecriture: Ecriture,
) -> impl IntoView {
    view! {
        <ul class="ui-liste">
            {declarations.into_iter().map(|d| {
                let nom = nom_affiche(&d.nom, d.prenom.as_deref());
                let detail = d
                    .declaree_le
                    .map(|t| format!("Déclaré le {}", t.format("%d/%m/%Y")))
                    .unwrap_or_else(|| "Date de déclaration inconnue".into());
                let id = d.personne_id;
                let (n1, n2) = (nom.clone(), nom.clone());
                let (a1, a2) = (format!("Confirmer pour {nom}"), format!("Lever pour {nom}"));
                view! {
                    <li class="ui-liste__element">
                        <div class="ui-liste__principal">
                            <span class="ui-liste__titre">{nom}</span>
                            <span class="ui-liste__detail">{detail}</span>
                        </div>
                        <Bouton variante=Variante::Primaire taille=Taille::Compacte
                            desactive=Signal::derive(move || ecriture.en_cours.get())
                            libelle_accessible=a1
                            on_click=Callback::new(move |_| definir(ecriture, id, n1.clone(), true))>
                            "Confirmer"
                        </Bouton>
                        <Bouton variante=Variante::Secondaire taille=Taille::Compacte
                            desactive=Signal::derive(move || ecriture.en_cours.get())
                            libelle_accessible=a2
                            on_click=Callback::new(move |_| definir(ecriture, id, n2.clone(), false))>
                            "Lever"
                        </Bouton>
                    </li>
                }
            }).collect_view()}
        </ul>
    }
}
