//! Notifications brèves (« toasts ») : confirmations non bloquantes.
//!
//! Règles : jamais pour une erreur bloquante (utiliser `Alerte` dans la page) ;
//! annoncées aux lecteurs d'écran (zone `role="status"`) ; fermables ; disparition
//! automatique après quelques secondes.
//!
//! Utilisation : `fournir_notifications()` une fois (racine de l'application),
//! `<ZoneNotifications/>` une fois dans la page, puis `use_notifications().succes("…")`.

use leptos::prelude::*;

use crate::bouton::{Bouton, Taille, Variante};

/// Ton d'une notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ton {
    Info,
    Succes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub id: u64,
    pub ton: Ton,
    pub texte: String,
}

/// Accès aux notifications (copiable, à récupérer avec [`use_notifications`]).
#[derive(Clone, Copy)]
pub struct Notifications {
    liste: RwSignal<Vec<Notification>>,
    prochain: RwSignal<u64>,
    // Utilisée seulement dans le navigateur (minuterie de disparition).
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    duree_ms: u64,
}

/// Durée d'affichage par défaut.
pub const DUREE_NOTIFICATION_MS: u64 = 6_000;

impl Notifications {
    /// Crée un gestionnaire (sans le placer dans le contexte).
    pub fn new(duree_ms: u64) -> Self {
        Self {
            liste: RwSignal::new(Vec::new()),
            prochain: RwSignal::new(1),
            duree_ms,
        }
    }

    pub fn succes(&self, texte: impl Into<String>) -> u64 {
        self.ajouter(Ton::Succes, texte.into())
    }

    pub fn info(&self, texte: impl Into<String>) -> u64 {
        self.ajouter(Ton::Info, texte.into())
    }

    pub fn fermer(&self, id: u64) {
        self.liste.update(|l| l.retain(|n| n.id != id));
    }

    pub fn liste(&self) -> Vec<Notification> {
        self.liste.get()
    }

    fn ajouter(&self, ton: Ton, texte: String) -> u64 {
        let id = self.prochain.get_untracked();
        self.prochain.set(id + 1);
        self.liste
            .update(|l| l.push(Notification { id, ton, texte }));
        #[cfg(target_arch = "wasm32")]
        if self.duree_ms > 0 {
            let moi = *self;
            set_timeout(
                move || moi.fermer(id),
                std::time::Duration::from_millis(self.duree_ms),
            );
        }
        id
    }
}

/// Place le gestionnaire de notifications dans le contexte et le renvoie.
pub fn fournir_notifications() -> Notifications {
    let n = Notifications::new(DUREE_NOTIFICATION_MS);
    provide_context(n);
    n
}

/// Gestionnaire de notifications du contexte.
pub fn use_notifications() -> Notifications {
    use_context::<Notifications>()
        .expect("fournir_notifications() doit être appelé à la racine de l'application")
}

/// Zone d'affichage des notifications (une seule par page).
#[component]
pub fn ZoneNotifications() -> impl IntoView {
    let n = use_notifications();
    view! {
        <ol class="ui-notifications" role="status" aria-live="polite" aria-label="Notifications">
            <For
                each=move || n.liste()
                key=|x| x.id
                children=move |x| {
                    let classe = match x.ton {
                        Ton::Succes => "ui-notification ui-notification--succes",
                        Ton::Info => "ui-notification ui-notification--info",
                    };
                    let id = x.id;
                    view! {
                        <li class=classe>
                            <p class="ui-notification__texte">{x.texte}</p>
                            <Bouton
                                variante=Variante::Discret
                                taille=Taille::Compacte
                                libelle_accessible="Fermer la notification"
                                on_click=Callback::new(move |_| n.fermer(id))
                            >
                                <span aria-hidden="true">"×"</span>
                            </Bouton>
                        </li>
                    }
                }
            />
        </ol>
    }
}
