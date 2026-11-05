//! Page Direction « Politique de sécurité » : rôles pour lesquels la passkey est obligatoire.
//! Réservée à Admin et Direction (le serveur refuse les autres en 403 ; l'écran ne fait que
//! ne pas proposer ce qui serait refusé).

use ent_web_api::{ApiError, PolitiqueSecurite};
use ent_web_ui::{
    AccesRefuse, Alerte, Annonce, Bouton, Case, Dialogue, EnTetePage, Niveau, Squelette, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;

use super::modele::{EditionPolitique, ROLES_POLITIQUE};
use crate::noyau::coquille::use_droits;
use crate::noyau::erreurs::ErreurChargement;
use crate::noyau::reconnexion;
use crate::noyau::session::{self, use_session};

#[component]
pub fn PagePolitiqueSecurite() -> impl IntoView {
    let droits = use_droits();
    let ctx = use_session();
    // Tant que l'identité n'est pas chargée, les droits sont vides : on attend la réponse
    // plutôt que d'afficher « accès refusé » à tort.
    let identite_chargee = Signal::derive(move || ctx.moi.get().is_some());
    view! {
        <Title text="Politique de sécurité"/>
        <EnTetePage
            titre="Politique de sécurité".to_string()
            surtitre="Établissement".to_string()
            sous_titre="Choisissez les rôles pour lesquels la passkey est obligatoire.".to_string()
        />
        {move || {
            if !identite_chargee.get() {
                view! { <Squelette lignes=4 libelle="Chargement de la politique de sécurité…"/> }.into_any()
            } else if droits.get().gerer_securite {
                view! { <FormulairePolitique/> }.into_any()
            } else {
                view! { <AccesRefuse message="Cette page est réservée à la direction de l'établissement."/> }
                    .into_any()
            }
        }}
    }
}

#[component]
fn FormulairePolitique() -> impl IntoView {
    let ctx = use_session();
    let notifications = ent_web_ui::use_notifications();
    let charge = LocalResource::new(|| async { session::client().politique_securite().await });

    // Un signal par rôle (une case ne redessine que sa ligne) + l'état d'édition.
    let cases: Vec<(&'static str, &'static str, RwSignal<bool>)> = ROLES_POLITIQUE
        .iter()
        .map(|(id, libelle)| (*id, *libelle, RwSignal::new(false)))
        .collect();
    let edition = RwSignal::new(EditionPolitique::nouvelle(&[]));
    let pret = RwSignal::new(false);
    let cases = StoredValue::new(cases);

    // Remplit les cases à l'arrivée des données (et après chaque enregistrement).
    let appliquer = move |p: &PolitiqueSecurite| {
        let nouvelle = EditionPolitique::nouvelle(&p.passkey_obligatoire_roles);
        cases.with_value(|cs| {
            for (id, _, s) in cs {
                s.set(nouvelle.est_coche(id));
            }
        });
        edition.set(nouvelle);
        pret.set(true);
    };
    Effect::new(move |_| {
        if let Some(Ok(p)) = charge.get() {
            appliquer(&p);
        }
    });
    // Les cases pilotent l'état d'édition.
    Effect::new(move |_| {
        if !pret.get() {
            return;
        }
        let etat: Vec<(&'static str, bool)> =
            cases.with_value(|cs| cs.iter().map(|(id, _, s)| (*id, s.get())).collect());
        edition.update(|e| {
            for (id, coche) in etat {
                if e.est_coche(id) != coche {
                    e.definir(id, coche);
                }
            }
        });
    });

    let dialogue_ouvert = RwSignal::new(false);
    let en_cours = RwSignal::new(false);
    let erreur = RwSignal::new(None::<String>);
    let annonce = RwSignal::new(String::new());
    let modifiee = Signal::derive(move || edition.with(EditionPolitique::est_modifiee));
    let resume = Signal::derive(move || edition.with(EditionPolitique::resume_changements));
    let mon_role = Signal::derive(move || ctx.securite.get().map(|m| m.role).unwrap_or_default());
    let mes_passkeys =
        Signal::derive(move || ctx.securite.get().map(|m| m.passkey_count).unwrap_or(0));

    let enregistrer = Callback::new(move |_: ()| {
        if en_cours.get_untracked() {
            return;
        }
        en_cours.set(true);
        erreur.set(None);
        let roles = edition.with_untracked(EditionPolitique::a_envoyer);
        spawn_local(async move {
            let resultat = session::client().definir_politique_securite(&roles).await;
            en_cours.set(false);
            dialogue_ouvert.set(false);
            match resultat {
                Ok(p) => {
                    appliquer(&p);
                    notifications.succes("Politique de sécurité enregistrée.");
                    annonce.set("Politique de sécurité enregistrée.".into());
                    ctx.moi.refetch();
                }
                Err(ApiError::NonConnecte) => {
                    reconnexion::signaler_session_perdue();
                    erreur.set(Some(reconnexion::MESSAGE_SESSION_EXPIREE.into()));
                }
                Err(e) => {
                    let message = e.message_utilisateur();
                    annonce.set(message.clone());
                    erreur.set(Some(message));
                }
            }
        });
    });

    view! {
        {move || match charge.get() {
            None => view! { <Squelette lignes=6 libelle="Chargement de la politique de sécurité…"/> }.into_any(),
            Some(Err(e)) => view! { <ErreurChargement erreur=e on_reessayer=Callback::new(move |_| charge.refetch())/> }.into_any(),
            Some(Ok(_)) => {
                view! {
                    <form
                        class="securite__bloc"
                        on:submit=move |ev| {
                            ev.prevent_default();
                            if modifiee.get_untracked() {
                                dialogue_ouvert.set(true);
                            }
                        }
                    >
                        <fieldset class="securite__roles">
                            <legend class="securite__titre">"Passkey obligatoire pour"</legend>
                            <p class="securite__aide">
                                "Les personnes de ces rôles devront enregistrer une passkey avant de pouvoir utiliser l'ENT. Les autres rôles peuvent en enregistrer une librement."
                            </p>
                            {cases
                                .get_value()
                                .into_iter()
                                .map(|(id, libelle, coche)| {
                                    view! { <Case id=format!("politique-{id}") libelle=libelle coche=coche/> }
                                })
                                .collect_view()}
                        </fieldset>
                        {move || edition.with(|e| e.avertissement_effet()).map(|m| view! { <Alerte niveau=Niveau::Avertissement titre=m/> })}
                        {move || {
                            edition
                                .with(|e| e.avertissement_auto_verrouillage(&mon_role.get(), mes_passkeys.get()))
                                .map(|m| view! { <Alerte niveau=Niveau::Avertissement titre=m/> })
                        }}
                        {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                        <div>
                            <Bouton soumettre=true variante=Variante::Primaire desactive=Signal::derive(move || !modifiee.get()) en_cours=en_cours>
                                "Enregistrer la politique"
                            </Bouton>
                        </div>
                    </form>
                }
                    .into_any()
            }
        }}
        <Dialogue ouvert=dialogue_ouvert titre="Enregistrer la politique de sécurité ?" id="dialogue-politique">
            <p>{move || resume.get()}</p>
            {move || edition.with(|e| e.avertissement_effet()).map(|m| view! { <p>{m}</p> })}
            <div class="ui-dialogue__actions">
                <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| dialogue_ouvert.set(false))>
                    "Annuler"
                </Bouton>
                <Bouton variante=Variante::Primaire en_cours=en_cours on_click=enregistrer>
                    "Confirmer"
                </Bouton>
            </div>
        </Dialogue>
        <Annonce message=Signal::derive(move || annonce.get())/>
    }
}
