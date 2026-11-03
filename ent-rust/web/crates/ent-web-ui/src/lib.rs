//! Design system de l'ENT : composants d'interface communs à tous les modules.
//!
//! Règles (voir `docs/DESIGN_SYSTEM_LEPTOS.md` et `docs/FRONTEND_LEPTOS.md`) :
//! - éléments HTML natifs d'abord (bouton, champ, case, radio, `<dialog>`) :
//!   accessibilité et clavier fournis par le navigateur ;
//! - états explicites : chargement, vide, erreur, succès, accès refusé ;
//! - aucun appel réseau ni règle métier ; aucun JavaScript applicatif.
//!
//! Styles : `style/ui.css` (jetons, base, classes `ui-*`), importé par l'application.

mod alerte;
mod bouton;
mod champ;
mod dialogue;
mod etats;
mod filtre;
mod notification;

pub use alerte::{Alerte, Annonce, Niveau};
pub use bouton::{classes_bouton, Bouton, LienBouton, Taille, Variante};
pub use champ::{Case, ChampTexte, GroupeChoix, ResumeErreurs, Selection, ZoneTexte};
pub use dialogue::{Confirmation, Dialogue};
pub use etats::{AccesRefuse, Chargement, EtatErreur, EtatVide, Squelette};
pub use filtre::FiltrePastille;
pub use notification::{
    fournir_notifications, use_notifications, Notification, Notifications, Ton, ZoneNotifications,
    DUREE_NOTIFICATION_MS,
};

#[cfg(test)]
mod tests;
