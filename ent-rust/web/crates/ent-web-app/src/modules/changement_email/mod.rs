//! Liens de changement d'adresse e-mail (RF-01) : confirmation (reçue à la NOUVELLE adresse) et
//! annulation (reçue à l'ANCIENNE adresse). Routes d'`ent-api` inchangées :
//! `POST /auth/email/confirmer` et `POST /auth/email/annuler`.
//!
//! Sécurité : ouvrir un lien **n'agit jamais** (un aperçu de messagerie ou un antivirus ne doit
//! pas consommer le jeton) ; l'action part d'un bouton. Le jeton est retiré de la barre
//! d'adresse dès l'arrivée et n'est jamais affiché ni journalisé.

pub mod modele;

use ent_web_ui::{Alerte, Annonce, Bouton, Icone, LienBouton, Niveau, NomIcone, Variante};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::{Meta, Title};
use leptos_router::hooks::use_query_map;

use crate::noyau::session;
use modele::{jeton_valide, message_erreur_lien, EtapeEmail};

/// Lien de confirmation : `/email/confirmer?token=…`.
#[component]
pub fn PageConfirmerEmail() -> impl IntoView {
    view! { <PageLien etape=EtapeEmail::Confirmer/> }
}

/// Lien d'annulation : `/email/annuler?token=…`.
#[component]
pub fn PageAnnulerEmail() -> impl IntoView {
    view! { <PageLien etape=EtapeEmail::Annuler/> }
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum EtatLien {
    Attente,
    Reussi,
    Echec(String),
}

#[component]
fn PageLien(etape: EtapeEmail) -> impl IntoView {
    let query = use_query_map();
    // Le jeton est lu une fois, à la création, puis retiré de l'adresse.
    let jeton = StoredValue::new(jeton_valide(query.read_untracked().get("token").as_deref()));
    let chemin = match etape {
        EtapeEmail::Confirmer => "/email/confirmer",
        EtapeEmail::Annuler => "/email/annuler",
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
            let resultat = match etape {
                EtapeEmail::Confirmer => client.confirmer_changement_email(&j).await,
                EtapeEmail::Annuler => client.annuler_changement_email(&j).await,
            };
            en_cours.set(false);
            match resultat {
                Ok(()) => {
                    annonce.set(etape.message_succes().into());
                    etat.set(EtatLien::Reussi);
                }
                Err(e) => {
                    let m = message_erreur_lien(&e);
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
                <span class="connexion__logo"><Icone nom=NomIcone::Ecole/></span>
                <p class="connexion__etab">"ScolIA"</p>
                <h1>{etape.titre()}</h1>
            </div>
            <div class="connexion__carte">
                {move || match etat.get() {
                    EtatLien::Reussi => {
                        view! {
                            <Alerte niveau=Niveau::Succes titre=etape.message_succes().to_string()/>
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
                        if utilisable {
                            view! {
                                {explications.clone()}
                                {echec.map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
                                <p>
                                    <Bouton
                                        variante=Variante::Primaire
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
                                    <p>"Ouvrez le lien depuis l'e-mail reçu, sans le modifier. Au besoin, contactez votre établissement."</p>
                                </Alerte>
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
