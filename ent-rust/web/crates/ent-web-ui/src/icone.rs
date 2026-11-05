//! Icônes : SVG en ligne, trait de 1,8 px, couleur du texte (`currentColor`).
//! Toujours décoratives (`aria-hidden`) : le sens est porté par le libellé voisin
//! ou par l'`aria-label` du bouton qui les contient.

use leptos::prelude::*;

/// Icônes disponibles (jeu volontairement réduit, étendu à la demande).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NomIcone {
    Ecole,
    Accueil,
    Appel,
    Agenda,
    Messages,
    Annonce,
    Recherche,
    Deconnexion,
    Passkey,
    ChevronGauche,
    ChevronDroite,
    Horloge,
    Salle,
    /// Personne (vie scolaire, absences).
    Personne,
}

fn traces(nom: NomIcone) -> &'static str {
    match nom {
        NomIcone::Ecole => {
            r#"<path d="M3 9l9-5 9 5-9 5-9-5z"/><path d="M7 11.5V16c0 1.5 2.2 3 5 3s5-1.5 5-3v-4.5"/>"#
        }
        NomIcone::Accueil => {
            r#"<rect x="3" y="3" width="7" height="9" rx="2"/><rect x="14" y="3" width="7" height="5" rx="2"/><rect x="14" y="12" width="7" height="9" rx="2"/><rect x="3" y="16" width="7" height="5" rx="2"/>"#
        }
        NomIcone::Appel => {
            r#"<rect x="4" y="3" width="16" height="18" rx="3"/><path d="M9 11l2 2 4-4"/>"#
        }
        NomIcone::Agenda => {
            r#"<rect x="3" y="4" width="18" height="17" rx="3"/><path d="M3 9h18M8 2v4M16 2v4"/>"#
        }
        NomIcone::Messages => r#"<path d="M4 5h16v11H8l-4 4V5z"/>"#,
        NomIcone::Annonce => {
            r#"<path d="M3 11l15-6v14L3 13v-2z"/><path d="M7 13v4a2 2 0 002 2h1"/>"#
        }
        NomIcone::Recherche => r#"<circle cx="11" cy="11" r="7"/><path d="M20 20l-3.5-3.5"/>"#,
        NomIcone::Deconnexion => {
            r#"<path d="M15 4h3a2 2 0 012 2v12a2 2 0 01-2 2h-3"/><path d="M10 17l-5-5 5-5M5 12h11"/>"#
        }
        NomIcone::Passkey => {
            r#"<circle cx="9" cy="8" r="4"/><path d="M2 21c0-3.5 3-6 7-6 1.4 0 2.6.3 3.6.8"/><circle cx="18" cy="15" r="2.5"/><path d="M18 17.5V22M18 20h2"/>"#
        }
        NomIcone::ChevronGauche => r#"<path d="M15 18l-6-6 6-6"/>"#,
        NomIcone::ChevronDroite => r#"<path d="M9 18l6-6-6-6"/>"#,
        NomIcone::Horloge => r#"<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>"#,
        NomIcone::Personne => {
            r#"<circle cx="12" cy="8" r="4"/><path d="M4 21c0-4 3.6-7 8-7s8 3 8 7"/>"#
        }
        NomIcone::Salle => {
            r#"<path d="M4 21V5a2 2 0 012-2h12a2 2 0 012 2v16"/><path d="M2 21h20M15 12h.01"/>"#
        }
    }
}

/// Icône décorative.
#[component]
pub fn Icone(nom: NomIcone, #[prop(optional)] petite: bool) -> impl IntoView {
    view! {
        <svg
            class=if petite { "ui-icone ui-icone--petite" } else { "ui-icone" }
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            focusable="false"
            inner_html=traces(nom)
        ></svg>
    }
}
