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
    /// Palier des faibles, lot 3 (faible 5 de la contre-revue E1 passe 5) : lignes écartées par cause depuis la dernière
    /// ligne écrite. Le nombre part avec la ligne suivante : aucun échec ne disparaît sans trace.
    supprimees: HashMap<&'static str, u64>,
}

impl LimiteurJournal {
    /// Vrai si une ligne pour `cause` peut être écrite à `maintenant` (et l'enregistre).
    pub(crate) fn autoriser(
        &mut self,
        cause: &'static str,
        maintenant: Instant,
        intervalle: Duration,
    ) -> bool {
        self.autoriser_compte(cause, maintenant, intervalle).is_some()
    }

    /// Comme [`Self::autoriser`], et rend, quand la ligne peut s'écrire, le nombre de lignes écartées pour `cause` depuis
    /// la précédente (remis à zéro).
    pub(crate) fn autoriser_compte(
        &mut self,
        cause: &'static str,
        maintenant: Instant,
        intervalle: Duration,
    ) -> Option<u64> {
        match self.derniere.get(cause) {
            Some(d) if maintenant.saturating_duration_since(*d) < intervalle => {
                *self.supprimees.entry(cause).or_default() += 1;
                None
            }
            _ => {
                self.derniere.insert(cause, maintenant);
                Some(self.supprimees.remove(cause).unwrap_or(0))
            }
        }
    }
}

/// Limiteur du processus, pour les journaux émis hors d'une boucle propriétaire.
static GLOBAL: Mutex<Option<LimiteurJournal>> = Mutex::new(None);

pub(crate) fn autoriser_global(cause: &'static str) -> bool {
    autoriser_global_compte(cause).is_some()
}

/// Limiteur du processus avec le nombre de lignes écartées depuis la précédente (`None` : ligne retenue).
pub(crate) fn autoriser_global_compte(cause: &'static str) -> Option<u64> {
    // Verrou empoisonné : on journalise (mieux vaut une ligne de trop qu'un silence).
    match GLOBAL.lock() {
        Ok(mut g) => g.get_or_insert_with(LimiteurJournal::default).autoriser_compte(
            cause,
            Instant::now(),
            INTERVALLE_JOURNAL,
        ),
        Err(_) => Some(0),
    }
}

/// Abonné de test minimal, partagé (T3) : garde (niveau, message) des événements émis sur le **fil courant**
/// (`tracing::subscriber::with_default`), sans horloge ni base.
#[cfg(test)]
pub(crate) mod collecteur {
    use std::sync::{Arc, Mutex};

    pub(crate) type Journal = Arc<Mutex<Vec<(tracing::Level, String)>>>;

    pub(crate) struct Collecteur(pub(crate) Journal);

    impl tracing::Subscriber for Collecteur {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, e: &tracing::Event<'_>) {
            struct Message(String);
            impl tracing::field::Visit for Message {
                fn record_debug(&mut self, champ: &tracing::field::Field, valeur: &dyn std::fmt::Debug) {
                    if champ.name() == "message" {
                        self.0 = format!("{valeur:?}");
                    }
                }
            }
            let mut m = Message(String::new());
            e.record(&mut m);
            self.0.lock().unwrap().push((*e.metadata().level(), m.0));
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
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

    /// Palier des faibles, lot 3 (faible 5 de E1 passe 5) : les lignes écartées sont comptées par cause et le compte part
    /// avec la ligne suivante, puis repart de zéro ; une autre cause a son propre compte.
    #[test]
    fn les_lignes_ecartees_sont_comptees_par_cause_et_rendues_avec_la_suivante() {
        let t0 = Instant::now();
        let mut l = LimiteurJournal::default();
        let m = INTERVALLE_JOURNAL;
        assert_eq!(l.autoriser_compte("a", t0, m), Some(0));
        for s in 1..=3 {
            assert_eq!(l.autoriser_compte("a", t0 + Duration::from_secs(s), m), None);
        }
        assert_eq!(l.autoriser_compte("b", t0 + Duration::from_secs(4), m), Some(0));
        assert_eq!(l.autoriser_compte("a", t0 + Duration::from_secs(60), m), Some(3));
        assert_eq!(l.autoriser_compte("a", t0 + Duration::from_secs(120), m), Some(0));
    }

    /// T3 (faible 1 de E1 passe 5) : `signaler_echec_etablissement` — le **compteur** compte chaque échec, le **journal** ne
    /// porte qu'une ligne par minute et par contexte (ici : cinq échecs d'un contexte → une ligne ; un autre contexte → la
    /// sienne). Sans base ni horloge (cause propre au test, jamais journalisée ailleurs).
    #[test]
    fn les_echecs_par_etablissement_sont_comptes_tous_mais_journalises_une_fois_par_minute() {
        use super::collecteur::Collecteur;
        let journal = std::sync::Arc::new(Mutex::new(Vec::new()));
        let avant = crate::tenant_tx::echecs_par_etablissement();
        tracing::subscriber::with_default(Collecteur(journal.clone()), || {
            for _ in 0..5 {
                crate::tenant_tx::signaler_echec_etablissement("t3_contexte_a", &"base indisponible");
            }
            crate::tenant_tx::signaler_echec_etablissement("t3_contexte_b", &"base indisponible");
        });
        let lignes = journal.lock().unwrap().len();
        assert_eq!(lignes, 2, "une ligne par contexte, pas une par échec : {lignes}");
        assert!(
            crate::tenant_tx::echecs_par_etablissement() >= avant + 6,
            "le compteur compte les six échecs, journalisés ou non"
        );
    }
}
