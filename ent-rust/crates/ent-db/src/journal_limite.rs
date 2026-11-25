//! Limiteur de journalisation (suites E1, passe 5, faible 4) : une base indisponible faisait journaliser la même ligne
//! toutes les 5 s (cadence du distributeur d'avis). Au plus **une ligne par intervalle et par cause**.
//!
//! L'instant est **passé par l'appelant** (`Instant` monotone) : le limiteur ne lit aucune horloge, il est testable sans
//! horloge réelle. Ce n'est pas une date métier (règle 33) : seulement un espacement de lignes de journal.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Intervalle minimal entre deux lignes de la même cause : une minute.
pub(crate) const INTERVALLE_JOURNAL: Duration = Duration::from_secs(60);

#[derive(Debug, Default)]
pub(crate) struct LimiteurJournal {
    derniere: HashMap<&'static str, Instant>,
}

impl LimiteurJournal {
    /// Vrai si une ligne pour `cause` peut être écrite à `maintenant` (et l'enregistre).
    pub(crate) fn autoriser(
        &mut self,
        cause: &'static str,
        maintenant: Instant,
        intervalle: Duration,
    ) -> bool {
        match self.derniere.get(cause) {
            Some(d) if maintenant.saturating_duration_since(*d) < intervalle => false,
            _ => {
                self.derniere.insert(cause, maintenant);
                true
            }
        }
    }
}

/// Limiteur du processus, pour les journaux émis hors d'une boucle propriétaire.
static GLOBAL: Mutex<Option<LimiteurJournal>> = Mutex::new(None);

pub(crate) fn autoriser_global(cause: &'static str) -> bool {
    // Verrou empoisonné : on journalise (mieux vaut une ligne de trop qu'un silence).
    match GLOBAL.lock() {
        Ok(mut g) => g.get_or_insert_with(LimiteurJournal::default).autoriser(
            cause,
            Instant::now(),
            INTERVALLE_JOURNAL,
        ),
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_ligne_par_intervalle_et_par_cause_sans_horloge_reelle() {
        let t0 = Instant::now();
        let mut l = LimiteurJournal::default();
        let m = INTERVALLE_JOURNAL;
        assert!(l.autoriser("a", t0, m), "première ligne");
        assert!(
            !l.autoriser("a", t0 + Duration::from_secs(5), m),
            "5 s plus tard : retenue"
        );
        assert!(!l.autoriser("a", t0 + Duration::from_secs(59), m));
        assert!(
            l.autoriser("b", t0 + Duration::from_secs(5), m),
            "autre cause : indépendante"
        );
        assert!(
            l.autoriser("a", t0 + Duration::from_secs(60), m),
            "intervalle écoulé"
        );
        assert!(
            !l.autoriser("a", t0 + Duration::from_secs(61), m),
            "l'intervalle repart de la dernière ligne"
        );
        assert!(l.autoriser("a", t0 + Duration::from_secs(120), m));
    }
}
