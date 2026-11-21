//! Liaison entre une « cible » (la chose sur laquelle on agit) et l'ouverture d'un `<dialog>`.
//!
//! Piège corrigé (démo P1, 2026-11-08) : deux `Effect` qui se répondent (`ouvert ← cible`,
//! `cible ← None` quand `ouvert` est faux) bouclaient à l'infini dans le navigateur, car
//! `RwSignal::set` notifie même une valeur inchangée : la fiche élève se figeait dès son
//! affichage. Toujours comparer avant d'écrire.

use leptos::prelude::*;

/// Ouvre le dialogue quand une cible est choisie ; efface la cible quand il se ferme
/// (Échap, bouton Annuler…). Sans effet si rien ne change.
pub fn lier_ouverture<T>(ouvert: RwSignal<bool>, cible: RwSignal<Option<T>>)
where
    T: Send + Sync + 'static,
{
    Effect::new(move |_| {
        let demande = cible.with(Option::is_some);
        if ouvert.get_untracked() != demande {
            ouvert.set(demande);
        }
    });
    Effect::new(move |_| {
        if !ouvert.get() && cible.with_untracked(Option::is_some) {
            cible.set(None);
        }
    });
}
