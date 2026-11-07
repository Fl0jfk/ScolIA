//! Écritures vers l'API depuis un écran : une action à la fois, retour annoncé (notification et
//! zone `aria-live`), erreur visible dans la page, arrêt sur 401 (reconnexion), rechargement ensuite.

use ent_web_api::ApiError;
use ent_web_ui::{use_notifications, Alerte, Annonce, Niveau, Notifications};
use leptos::prelude::*;
use leptos::task::spawn_local;

use super::reconnexion;

/// Message d'échec d'une écriture ; une session perdue ouvre la fenêtre de reconnexion.
pub fn message_echec(e: ApiError) -> String {
    match e {
        ApiError::NonConnecte => {
            reconnexion::signaler_session_perdue();
            reconnexion::MESSAGE_SESSION_EXPIREE.to_string()
        }
        autre => autre.message_utilisateur(),
    }
}

/// Contexte d'une écriture : une action à la fois, retour annoncé (notification + zone `aria-live`),
/// erreur visible dans la page, rechargement des données ensuite.
#[derive(Clone, Copy)]
pub struct Ecriture {
    pub en_cours: RwSignal<bool>,
    pub erreur: RwSignal<Option<String>>,
    pub annonce: RwSignal<String>,
    notifications: Notifications,
    recharger: Callback<()>,
}

impl Ecriture {
    pub fn nouveau(recharger: Callback<()>) -> Self {
        Self {
            en_cours: RwSignal::new(false),
            erreur: RwSignal::new(None),
            annonce: RwSignal::new(String::new()),
            notifications: use_notifications(),
            recharger,
        }
    }

    /// Lance l'écriture (ignorée si une autre est en cours). `Ok(message)` : confirmation annoncée.
    pub fn lancer<F>(&self, futur: F)
    where
        F: std::future::Future<Output = Result<String, ApiError>> + 'static,
    {
        if self.en_cours.get_untracked() {
            return;
        }
        self.erreur.set(None);
        self.en_cours.set(true);
        let ctx = *self;
        spawn_local(async move {
            match futur.await {
                Ok(message) => {
                    ctx.notifications.succes(message.clone());
                    ctx.annonce.set(message);
                    ctx.recharger.run(());
                }
                Err(e) => ctx.erreur.set(Some(message_echec(e))),
            }
            ctx.en_cours.set(false);
        });
    }
}

/// Zone commune : annonce `aria-live` et erreur de la dernière écriture.
#[component]
pub fn RetourEcriture(ecriture: Ecriture) -> impl IntoView {
    view! {
        <Annonce message=Signal::derive(move || ecriture.annonce.get())/>
        {move || ecriture.erreur.get().map(|m| view! { <Alerte niveau=Niveau::Erreur titre=m/> })}
    }
}
