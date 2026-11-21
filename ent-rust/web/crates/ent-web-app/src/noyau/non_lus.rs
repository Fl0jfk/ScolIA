//! Compteur de messages non lus, partagé entre la coquille (pastille de navigation) et
//! la messagerie (qui le rafraîchit après lecture ou envoi). Aucune dépendance à un module.

use ent_web_api::FilMessages;
use leptos::prelude::*;

/// Messages non lus des fils non archivés (les valeurs négatives comptent pour zéro).
pub fn total_non_lus(fils: &[FilMessages]) -> i64 {
    fils.iter()
        .filter(|f| !f.archive)
        .map(|f| f.nb_non_lus.max(0))
        .sum()
}

/// Contexte fourni par la coquille.
#[derive(Clone, Copy)]
pub struct NonLus {
    /// Dernier total connu.
    pub total: RwSignal<i64>,
    /// Demande une relecture du total auprès du serveur.
    pub rafraichir: Callback<()>,
}

/// Compteur de la coquille (absent hors coquille).
pub fn use_non_lus() -> Option<NonLus> {
    use_context::<NonLus>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn fil(non_lus: i64, archive: bool) -> FilMessages {
        FilMessages {
            id: Uuid::nil(),
            eleve_label: None,
            sujet: String::new(),
            last_message_at: String::new(),
            archive,
            nb_non_lus: non_lus,
        }
    }

    #[test]
    fn total_sans_archives_ni_valeurs_negatives() {
        assert_eq!(total_non_lus(&[]), 0);
        assert_eq!(
            total_non_lus(&[fil(2, false), fil(5, true), fil(-4, false), fil(1, false)]),
            3
        );
    }
}
