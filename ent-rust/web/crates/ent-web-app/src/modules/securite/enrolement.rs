//! Enrôlement obligatoire d'une passkey : écran plein, sans navigation ni lien de sortie
//! autre que « Se déconnecter ». Hors de la coquille : aucune page métier n'est atteignable
//! tant que la passkey n'est pas créée (le serveur refuse de toute façon en 403).

use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Annonce, Bouton, ChampTexte, Chargement, Icone, Niveau, NomIcone, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::use_navigate;

use super::enroler_passkey;
use super::modele::{explication_obligation_enrolement, message_action_sensible};
use crate::noyau::droits::libelle_role;
use crate::noyau::portee::{self, Portee};
use crate::noyau::session;

#[component]
pub fn PageEnrolement() -> impl IntoView {
    let navigate = use_navigate();
    let nom = RwSignal::new(String::new());
    let en_cours = RwSignal::new(false);
    let erreur = RwSignal::new(None::<String>);
    let annonce = RwSignal::new(String::new());
    let en_deconnexion = RwSignal::new(false);

    // Ancien jeton `localStorage` (avant A6) : effacé et révoqué (navigateur seulement).
    Effect::new(move |_| session::purger_ancien_jeton());
    let moi = LocalResource::new(|| async { session::client().moi_securite().await });
    {
        let navigate = navigate.clone();
        Effect::new(move |_| match moi.get() {
            // Pas de session (cookie absent ou expiré) : connexion.
            Some(Err(ApiError::NonConnecte)) => {
                navigate(&session::adresse_connexion("/web"), Default::default());
            }
            // Pas (ou plus) d'enrôlement à faire : accueil.
            Some(Ok(m)) => {
                if let Some(cible) =
                    portee::decision_redirection(Portee::depuis(&m.portee), portee::ENROLEMENT)
                {
                    navigate(cible, Default::default());
                }
            }
            _ => {}
        });
    }

    let creer = {
        let navigate = navigate.clone();
        Callback::new(move |_: ()| {
            if en_cours.get_untracked() {
                return;
            }
            en_cours.set(true);
            erreur.set(None);
            annonce.set("Validation sur votre appareil en cours…".into());
            let navigate = navigate.clone();
            spawn_local(async move {
                // Session « enrôlement » : le serveur accepte un mot de passe actuel vide.
                match enroler_passkey(&nom.get_untracked(), "").await {
                    Ok(_) => {
                        annonce.set("Passkey enregistrée. Redirection vers l'accueil…".into());
                        navigate(portee::ACCUEIL, Default::default());
                    }
                    Err(e) => {
                        if e.session_perdue() {
                            navigate(&session::adresse_connexion("/web"), Default::default());
                            return;
                        }
                        let message = e.message();
                        annonce.set(message.clone());
                        erreur.set(Some(message));
                        en_cours.set(false);
                    }
                }
            });
        })
    };

    let deconnecter = {
        let navigate = navigate.clone();
        Callback::new(move |_: ()| {
            if en_deconnexion.get_untracked() {
                return;
            }
            en_deconnexion.set(true);
            session::deconnecter(navigate.clone());
        })
    };

    view! {
        <Title text="Enregistrer une passkey"/>
        <main id="contenu" class="connexion" tabindex="-1">
            <div class="connexion__marque">
                <span class="connexion__logo"><Icone nom=NomIcone::Passkey/></span>
                <p class="connexion__etab">"ScolIA"</p>
                <h1>"Enregistrez votre passkey"</h1>
            </div>
            <div class="connexion__carte">
                {move || match moi.get() {
                    None => view! { <Chargement libelle="Vérification de votre compte…"/> }.into_any(),
                    Some(Err(e)) => {
                        view! {
                            <Alerte niveau=Niveau::Erreur titre=message_action_sensible(&e)>
                                <p>
                                    <Bouton variante=Variante::Secondaire on_click=Callback::new(move |_| moi.refetch())>
                                        "Réessayer"
                                    </Bouton>
                                </p>
                            </Alerte>
                        }
                            .into_any()
                    }
                    Some(Ok(m)) => {
                        view! {
                            <p>{explication_obligation_enrolement(libelle_role(&m.role))}</p>
                            <ul class="enrolement__points">
                                <li>"Aucun mot de passe à retenir : vous validez avec l'empreinte, le visage ou le code de votre appareil."</li>
                                <li>"Votre passkey reste sur cet appareil ; l'ENT n'en reçoit que la partie publique."</li>
                                <li>"Vous pourrez ensuite en ajouter d'autres (autre appareil, clé de sécurité) dans « Compte »."</li>
                            </ul>
                            <form
                                class="connexion__formulaire"
                                on:submit=move |ev| {
                                    ev.prevent_default();
                                    creer.run(());
                                }
                                novalidate
                            >
                                <ChampTexte
                                    id="nom-passkey"
                                    libelle="Nom de cet appareil"
                                    valeur=nom
                                    aide="Facultatif. Par exemple : « Téléphone de Marie »."
                                    autocomplete="off"
                                />
                                {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                                <Bouton soumettre=true variante=Variante::Primaire en_cours=en_cours>
                                    <Icone nom=NomIcone::Passkey/>
                                    "Créer ma passkey"
                                </Bouton>
                            </form>
                        }
                            .into_any()
                    }
                }}
                <Annonce message=Signal::derive(move || annonce.get())/>
            </div>
            <p class="connexion__pied">
                "Tant que la passkey n'est pas enregistrée, le reste de l'ENT reste fermé. "
                <button
                    type="button"
                    class="connexion__lien"
                    disabled=move || en_deconnexion.get() || en_cours.get()
                    on:click=move |_| deconnecter.run(())
                >
                    "Se déconnecter"
                </button>
            </p>
        </main>
    }
}
