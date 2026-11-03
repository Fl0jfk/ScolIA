//! Page de connexion du prototype (comptes de labo, mot de passe).
//!
//! Utilise la route existante `POST /auth/login`, sans la modifier. Les comptes
//! avec passkey sont renvoyés vers l'interface actuelle : le second facteur n'est
//! pas dans le périmètre de cette mission.

use ent_web_api::Connexion;
use ent_web_ui::{Alerte, Bouton, ChampTexte, Niveau};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::{use_navigate, use_query_map};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::session;

/// Configuration publique de la page de connexion (aucun secret).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametresConnexion {
    /// Établissement proposé par défaut (labo local).
    pub etablissement_id: Option<Uuid>,
    /// Vrai hors production : affiche le bandeau « environnement de test ».
    pub labo: bool,
}

/// Fonction serveur : exécutée sur le serveur, appelée en HTTP par le navigateur
/// (et directement pendant le rendu serveur).
#[server]
pub async fn config_connexion() -> Result<ParametresConnexion, ServerFnError> {
    let etablissement_id = std::env::var("ENT_WEB_ETABLISSEMENT_ID")
        .ok()
        .and_then(|v| v.trim().parse().ok());
    let env = std::env::var("ENT_ENV")
        .unwrap_or_default()
        .to_ascii_lowercase();
    Ok(ParametresConnexion {
        etablissement_id,
        labo: !matches!(env.as_str(), "prod" | "production"),
    })
}

#[component]
pub fn PageConnexion() -> impl IntoView {
    let navigate = use_navigate();
    let query = use_query_map();
    // Ressource sérialisée dans la page : identique au rendu serveur et à l'hydratation.
    let config = Resource::new(|| (), |_| config_connexion());

    let email = RwSignal::new(String::new());
    let mot_de_passe = RwSignal::new(String::new());
    let etablissement = RwSignal::new(String::new());
    let en_cours = RwSignal::new(false);
    let erreur = RwSignal::new(None::<String>);

    // Préremplit l'établissement depuis l'URL (?etab=) ou la configuration serveur.
    Effect::new(move |_| {
        if !etablissement.get_untracked().is_empty() {
            return;
        }
        if let Some(e) = query.with(|q| q.get("etab")) {
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
        let Ok(etab) = etablissement.get_untracked().trim().parse::<Uuid>() else {
            erreur.set(Some("Identifiant d'établissement invalide.".into()));
            return;
        };
        erreur.set(None);
        en_cours.set(true);
        let navigate = navigate.clone();
        let retour = query
            .with_untracked(|q| q.get("retour"))
            .filter(|r| r.starts_with("/web/"))
            .unwrap_or_else(|| "/web/appel".into());
        spawn_local(async move {
            let resultat = session::client()
                .connexion(
                    email.get_untracked().trim(),
                    &mot_de_passe.get_untracked(),
                    etab,
                )
                .await;
            en_cours.set(false);
            match resultat {
                Ok(Connexion::Ouverte(s)) => {
                    session::enregistrer(&s.token);
                    mot_de_passe.set(String::new());
                    navigate(&retour, Default::default());
                }
                Ok(Connexion::PasskeyRequise) => erreur.set(Some(
                    "Ce compte est protégé par une passkey. La connexion par passkey n'est pas encore \
                     disponible dans cette interface : utilisez l'interface actuelle (/app)."
                        .into(),
                )),
                Err(ent_web_api::ApiError::NonConnecte) => {
                    erreur.set(Some("Adresse e-mail ou mot de passe incorrect.".into()))
                }
                Err(e) => erreur.set(Some(e.message_utilisateur())),
            }
        });
    };

    view! {
        <Title text="Connexion"/>
        <main id="contenu" class="page page--etroite connexion">
            <div class="carte connexion__carte">
                <p class="marque">"ScolIA"</p>
                <h1>"Connexion"</h1>
                <Suspense fallback=|| ()>
                    {move || {
                        config
                            .get()
                            .and_then(Result::ok)
                            .filter(|c| c.labo)
                            .map(|_| {
                                view! {
                                    <p class="bandeau-labo">
                                        "Environnement de test — données fictives uniquement."
                                    </p>
                                }
                            })
                    }}
                </Suspense>
                <form class="formulaire" on:submit=soumettre novalidate>
                    <ChampTexte id="email" libelle="Adresse e-mail" valeur=email genre="email"
                        requis=true autocomplete="username"/>
                    <ChampTexte id="mdp" libelle="Mot de passe" valeur=mot_de_passe genre="password"
                        requis=true autocomplete="current-password"/>
                    // Transitoire : l'établissement sera déduit de l'adresse (D-012).
                    <ChampTexte id="etab" libelle="Établissement" valeur=etablissement
                        aide="Identifiant fourni par votre établissement."/>
                    {move || {
                        erreur
                            .get()
                            .map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })
                    }}
                    <Bouton soumettre=true en_cours=Signal::derive(move || en_cours.get())>
                        {move || if en_cours.get() { "Connexion…" } else { "Se connecter" }}
                    </Bouton>
                </form>
            </div>
        </main>
    }
}
