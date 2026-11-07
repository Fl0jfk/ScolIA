//! Préférences de notification (D0) : canaux, résumé quotidien, heures calmes, traçabilité.
//!
//! Données : routes `/notifications/*` d'`ent-api`. Aucune règle métier ici : le serveur
//! applique les préférences, les plafonds et les droits au moment de l'envoi.

pub mod modele;

use ent_web_ui::{Badge, Bouton, Case, ChampTexte, EnTetePage, EtatVide, Squelette, Variante};
use leptos::prelude::*;
use leptos_meta::Title;

use crate::noyau::ecriture::{Ecriture, RetourEcriture};
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::session;
use crate::noyau::temps::date_heure_paris;
use modele::{
    composer, libelle_motif, libelle_statut_envoi, ligne_envoi, ton_statut_envoi, SaisiePreferences,
};

#[component]
pub fn PagePreferencesNotification() -> impl IntoView {
    let prefs = LocalResource::new(|| async { session::client().preferences_notification().await });
    let envois = LocalResource::new(|| async { session::client().mes_envois_notification().await });
    view! {
        <Title text="Notifications"/>
        <EnTetePage
            surtitre="Compte".to_string()
            titre="Mes notifications".to_string()
            sous_titre="Choisissez comment être prévenu. Les notifications dans l'ENT sont toujours actives.".to_string()
        />
        {move || match prefs.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement des préférences…"/> }.into_any(),
            Some(Err(e)) => view! {
                <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| prefs.refetch())/>
            }
            .into_any(),
            Some(Ok(p)) => view! { <FormulairePreferences initial=SaisiePreferences::depuis(&p) envois=envois/> }
                .into_any(),
        }}
        <section class="ra-section" aria-labelledby="notif-envois">
            <h2 id="notif-envois" class="vs-section-titre">"Derniers envois"</h2>
            {move || match envois.get() {
                None => view! { <Squelette lignes=3 libelle="Chargement des envois…"/> }.into_any(),
                Some(Err(e)) => view! {
                    <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| envois.refetch())/>
                }
                .into_any(),
                Some(Ok(l)) if l.is_empty() => view! {
                    <EtatVide titre="Aucun envoi" message="Aucune notification n'a encore été envoyée par e-mail, SMS ou mobile."/>
                }
                .into_any(),
                Some(Ok(l)) => view! {
                    <ul class="jvs-liste">
                        {l.into_iter().map(|e| {
                            let motif = libelle_motif(e.motif.as_deref());
                            view! {
                                <li class="jvs-ligne">
                                    <div>
                                        <strong>{ligne_envoi(&e)}</strong>
                                        <span class="texte-attenue">{format!(" · {}", date_heure_paris(e.planifie_pour))}</span>
                                        {motif.map(|m| view! { <p class="texte-attenue">{m}</p> })}
                                    </div>
                                    <Badge libelle=libelle_statut_envoi(&e.statut) ton=ton_statut_envoi(&e.statut)/>
                                </li>
                            }
                        }).collect_view()}
                    </ul>
                }
                .into_any(),
            }}
        </section>
    }
}

#[component]
fn FormulairePreferences(
    initial: SaisiePreferences,
    envois: LocalResource<Result<Vec<ent_types::communication::EnvoiDto>, ent_web_api::ApiError>>,
) -> impl IntoView {
    let email = RwSignal::new(initial.email);
    let sms = RwSignal::new(initial.sms);
    let push = RwSignal::new(initial.push);
    let quotidien = RwSignal::new(initial.quotidien);
    let calme = RwSignal::new(initial.calme);
    let debut = RwSignal::new(initial.debut);
    let fin = RwSignal::new(initial.fin);
    let ecriture = Ecriture::nouveau(Callback::new(move |_| envois.refetch()));
    let erreur_saisie = RwSignal::new(None::<String>);
    let enregistrer = Callback::new(move |_| {
        let saisie = SaisiePreferences {
            email: email.get_untracked(),
            sms: sms.get_untracked(),
            push: push.get_untracked(),
            quotidien: quotidien.get_untracked(),
            calme: calme.get_untracked(),
            debut: debut.get_untracked(),
            fin: fin.get_untracked(),
        };
        match composer(&saisie) {
            Err(m) => erreur_saisie.set(Some(m)),
            Ok(corps) => {
                erreur_saisie.set(None);
                ecriture.lancer(async move {
                    session::client()
                        .definir_preferences_notification(&corps)
                        .await?;
                    Ok("Préférences enregistrées.".to_string())
                });
            }
        }
    });
    view! {
        <RetourEcriture ecriture=ecriture/>
        <form
            class="ra-section"
            on:submit=move |ev| {
                ev.prevent_default();
                enregistrer.run(());
            }
        >
            <fieldset class="ui-champ">
                <legend class="ui-champ__libelle">"Canaux utilisés en plus de l'ENT"</legend>
                <Case id="pref-email" libelle="E-mail" coche=email/>
                <Case id="pref-push" libelle="Notification sur mobile" coche=push/>
                <Case id="pref-sms" libelle="SMS (désactivé tant que vous ne l'avez pas choisi)" coche=sms/>
            </fieldset>
            <Case
                id="pref-quotidien"
                libelle="Regrouper les messages courants en un résumé chaque matin (e-mail et mobile)"
                coche=quotidien
            />
            <Case id="pref-calme" libelle="Respecter des heures calmes (heure de Paris)" coche=calme/>
            {move || calme.get().then(|| view! {
                <div class="ra-actions">
                    <ChampTexte id="pref-debut" libelle="Début" valeur=debut genre="time"/>
                    <ChampTexte id="pref-fin" libelle="Fin" valeur=fin genre="time"/>
                </div>
            })}
            {move || erreur_saisie.get().map(|m| view! { <p class="ui-champ__erreur" role="alert">{m}</p> })}
            <Bouton variante=Variante::Primaire soumettre=true en_cours=ecriture.en_cours>
                "Enregistrer"
            </Bouton>
        </form>
    }
}
