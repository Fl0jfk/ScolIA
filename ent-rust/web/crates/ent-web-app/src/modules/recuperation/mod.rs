//! Récupération de passkey en libre-service (D-018, socle 1 lot A8) : demande publique par
//! e-mail, puis pages d'atterrissage des liens reçus (confirmation, annulation, finalisation).
//!
//! Routes d'`ent-api` utilisées, inchangées : `POST /auth/passkey/recuperation[/confirmer|
//! /annuler|/finaliser]`. Les liens des e-mails pointent vers `/recuperation-passkey/<étape>`.
//!
//! Sécurité : ouvrir un lien **n'agit jamais** (un aperçu de messagerie ou un antivirus ne
//! doit pas consommer le jeton) ; l'action part d'un bouton. Le jeton est retiré de la barre
//! d'adresse dès l'arrivée et n'est jamais affiché ni journalisé.

pub mod modele;

use ent_web_api::ApiError;
use ent_web_ui::{
    Alerte, Annonce, Bouton, ChampTexte, Icone, LienBouton, Niveau, NomIcone, Variante,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::{Meta, Title};
use leptos_router::hooks::use_query_map;

use crate::modules::connexion::config_connexion;
use crate::noyau::session;
use modele::{
    fin_du_delai, jeton_valide, lire_etablissement, message_erreur_demande, message_erreur_lien,
    valider_email, EtapeLien, MESSAGE_DEMANDE_NEUTRE,
};

/// Demande de récupération : `/web/recuperation-passkey`.
#[component]
pub fn PageDemandeRecuperation() -> impl IntoView {
    let query = use_query_map();
    let config = LocalResource::new(|| async { config_connexion().await });

    let email = RwSignal::new(String::new());
    let etablissement = RwSignal::new(String::new());
    let erreur_email = RwSignal::new(None::<String>);
    let erreur_etab = RwSignal::new(None::<String>);
    let erreur = RwSignal::new(None::<String>);
    let envoyee = RwSignal::new(false);
    let en_cours = RwSignal::new(false);
    let annonce = RwSignal::new(String::new());

    // Établissement : adresse (?etab=) puis configuration du serveur ; sinon saisie.
    Effect::new(move |_| {
        if !etablissement.get_untracked().trim().is_empty() {
            return;
        }
        if let Some(e) = query.read().get("etab") {
            etablissement.set(e);
        } else if let Some(Ok(c)) = config.get() {
            if let Some(id) = c.etablissement_id {
                etablissement.set(id.to_string());
            }
        }
    });

    let soumettre = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if en_cours.get_untracked() {
            return;
        }
        let adresse = match valider_email(&email.get_untracked()) {
            Ok(a) => a,
            Err(m) => {
                erreur_email.set(Some(m.into()));
                annonce.set(m.into());
                return;
            }
        };
        let Some(etab) = lire_etablissement(&etablissement.get_untracked()) else {
            let m = "Indiquez l'identifiant de votre établissement.";
            erreur_etab.set(Some(m.into()));
            annonce.set(m.into());
            return;
        };
        erreur_email.set(None);
        erreur_etab.set(None);
        erreur.set(None);
        en_cours.set(true);
        spawn_local(async move {
            let resultat = session::client()
                .demander_recuperation_passkey(&adresse, etab)
                .await;
            en_cours.set(false);
            match resultat {
                Ok(()) => {
                    envoyee.set(true);
                    annonce.set(MESSAGE_DEMANDE_NEUTRE.into());
                }
                Err(e) => {
                    let m = message_erreur_demande(&e);
                    annonce.set(m.clone());
                    erreur.set(Some(m));
                }
            }
        });
    };

    view! {
        <Title text="Passkey perdue"/>
        <Meta name="robots" content="noindex"/>
        <main id="contenu" class="connexion">
            <div class="connexion__marque">
                <span class="connexion__logo"><Icone nom=NomIcone::Ecole/></span>
                <p class="connexion__etab">"ScolIA"</p>
                <h1>"J'ai perdu ma passkey"</h1>
            </div>
            <div class="connexion__carte">
                <p>"Indiquez l'adresse e-mail de votre compte. Nous vous envoyons un lien pour demander la récupération de votre accès."</p>
                <p class="connexion__aide">
                    "Pour votre sécurité, la récupération n'est pas immédiate : après votre confirmation, un délai de sécurité (24 heures en général) s'écoule et vous êtes prévenu par e-mail, avec la possibilité d'annuler."
                </p>
                {move || {
                    if envoyee.get() {
                        view! {
                            <Alerte niveau=Niveau::Succes titre=MESSAGE_DEMANDE_NEUTRE.to_string()/>
                            <p><a href="/web/connexion">"Retour à la connexion"</a></p>
                        }
                            .into_any()
                    } else {
                        view! {
                            <form class="connexion__formulaire" on:submit=soumettre novalidate=true>
                                <ChampTexte
                                    id="recup-email"
                                    libelle="Adresse e-mail"
                                    valeur=email
                                    genre="email"
                                    requis=true
                                    autocomplete="email"
                                    erreur=Signal::derive(move || erreur_email.get())
                                />
                                {move || {
                                    let connu = lire_etablissement(&etablissement.get()).is_some();
                                    (!connu || erreur_etab.get().is_some())
                                        .then(|| {
                                            view! {
                                                <ChampTexte
                                                    id="recup-etab"
                                                    libelle="Établissement"
                                                    valeur=etablissement
                                                    aide="Identifiant fourni par votre établissement."
                                                    erreur=Signal::derive(move || erreur_etab.get())
                                                />
                                            }
                                        })
                                }}
                                {move || erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                                <Bouton soumettre=true variante=Variante::Primaire en_cours=Signal::derive(move || en_cours.get())>
                                    "Envoyer le lien"
                                </Bouton>
                            </form>
                            <p class="connexion__pied"><a href="/web/connexion">"Retour à la connexion"</a></p>
                        }
                            .into_any()
                    }
                }}
            </div>
            <Annonce message=Signal::derive(move || annonce.get())/>
        </main>
    }
}

/// Lien de confirmation : `/recuperation-passkey/confirmer?token=…`.
#[component]
pub fn PageConfirmerRecuperation() -> impl IntoView {
    view! { <PageLien etape=EtapeLien::Confirmer/> }
}

/// Lien d'annulation : `/recuperation-passkey/annuler?token=…`.
#[component]
pub fn PageAnnulerRecuperation() -> impl IntoView {
    view! { <PageLien etape=EtapeLien::Annuler/> }
}

/// Lien de finalisation : `/recuperation-passkey/finaliser?token=…`.
#[component]
pub fn PageFinaliserRecuperation() -> impl IntoView {
    view! { <PageLien etape=EtapeLien::Finaliser/> }
}

/// Enlève le jeton de la barre d'adresse (navigateur seulement).
fn masquer_jeton_de_l_adresse(chemin: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(h) = web_sys::window().and_then(|w| w.history().ok()) {
            let _ =
                h.replace_state_with_url(&leptos::wasm_bindgen::JsValue::NULL, "", Some(chemin));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = chemin;
}

/// Résultat d'une étape.
#[derive(Clone, Debug, PartialEq, Eq)]
enum EtatLien {
    Attente,
    Reussi(Option<String>),
    Echec(String),
}

#[component]
fn PageLien(etape: EtapeLien) -> impl IntoView {
    let query = use_query_map();
    // Le jeton est lu une fois, à la création, puis retiré de l'adresse.
    let jeton = StoredValue::new(jeton_valide(query.read_untracked().get("token").as_deref()));
    let chemin = match etape {
        EtapeLien::Confirmer => "/recuperation-passkey/confirmer",
        EtapeLien::Annuler => "/recuperation-passkey/annuler",
        EtapeLien::Finaliser => "/recuperation-passkey/finaliser",
    };
    Effect::new(move |_| masquer_jeton_de_l_adresse(chemin));

    let etat = RwSignal::new(EtatLien::Attente);
    let en_cours = RwSignal::new(false);
    let annonce = RwSignal::new(String::new());

    let agir = Callback::new(move |_: ()| {
        if en_cours.get_untracked() {
            return;
        }
        let Some(j) = jeton.get_value() else { return };
        en_cours.set(true);
        spawn_local(async move {
            let client = session::client();
            let resultat: Result<Option<String>, ApiError> = match etape {
                EtapeLien::Confirmer => client
                    .confirmer_recuperation_passkey(&j)
                    .await
                    .map(|c| Some(fin_du_delai(c.effective_at))),
                EtapeLien::Annuler => client.annuler_recuperation_passkey(&j).await.map(|_| None),
                EtapeLien::Finaliser => client
                    .finaliser_recuperation_passkey(&j)
                    .await
                    .map(|_| None),
            };
            en_cours.set(false);
            match resultat {
                Ok(fin) => {
                    annonce.set(etape.message_succes().into());
                    etat.set(EtatLien::Reussi(fin));
                }
                Err(e) => {
                    let m = message_erreur_lien(etape, &e);
                    annonce.set(m.clone());
                    etat.set(EtatLien::Echec(m));
                }
            }
        });
    });

    let utilisable = jeton.get_value().is_some();
    let explications = etape
        .explications()
        .iter()
        .map(|p| view! { <p>{*p}</p> })
        .collect_view();

    view! {
        <Title text=etape.titre()/>
        <Meta name="robots" content="noindex"/>
        <main id="contenu" class="connexion">
            <div class="connexion__marque">
                <span class="connexion__logo"><Icone nom=NomIcone::Passkey/></span>
                <p class="connexion__etab">"ScolIA"</p>
                <h1>{etape.titre()}</h1>
            </div>
            <div class="connexion__carte">
                {move || {
                    let etat_courant = etat.get();
                    match etat_courant {
                        EtatLien::Reussi(fin) => {
                            view! {
                                <Alerte niveau=Niveau::Succes titre=etape.message_succes().to_string()>
                                    {fin.map(|f| view! {
                                        <p>
                                            "Délai de sécurité jusqu'au " <strong>{f}</strong> ". "
                                            "Vous recevez un e-mail avec un lien pour annuler et un lien pour finaliser. "
                                            "Rien n'est modifié avant cette échéance."
                                        </p>
                                    })}
                                    {(etape == EtapeLien::Finaliser).then(|| view! {
                                        <p>"Connectez-vous avec votre mot de passe, puis enregistrez une nouvelle passkey."</p>
                                    })}
                                    {(etape == EtapeLien::Annuler).then(|| view! {
                                        <p>"Vous n'êtes pas à l'origine de la demande ? Changez votre mot de passe et prévenez votre établissement."</p>
                                    })}
                                </Alerte>
                                <p>
                                    <LienBouton href="/web/connexion" variante=Variante::Primaire>
                                        "Aller à la connexion"
                                    </LienBouton>
                                </p>
                            }
                                .into_any()
                        }
                        autre => {
                            let echec = match autre {
                                EtatLien::Echec(m) => Some(m),
                                _ => None,
                            };
                            view! {
                                {if utilisable {
                                    view! {
                                        {explications.clone()}
                                        {echec.map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                                        <p>
                                            <Bouton
                                                variante=if etape == EtapeLien::Finaliser { Variante::Danger } else { Variante::Primaire }
                                                en_cours=Signal::derive(move || en_cours.get())
                                                on_click=agir
                                            >
                                                {etape.libelle_bouton()}
                                            </Bouton>
                                        </p>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <Alerte niveau=Niveau::Erreur titre="Ce lien est incomplet ou invalide.">
                                            <p>"Ouvrez le lien depuis l'e-mail reçu, sans le modifier. Au besoin, refaites une demande."</p>
                                        </Alerte>
                                    }
                                        .into_any()
                                }}
                                <p class="connexion__pied"><a href="/web/recuperation-passkey">"Refaire une demande"</a></p>
                            }
                                .into_any()
                        }
                    }
                }}
            </div>
            <Annonce message=Signal::derive(move || annonce.get())/>
        </main>
    }
}
