//! Alerte d'exploitation : une borne `xmin` **gelée** (passe 2 de la contre-revue E1, F-5).
//!
//! Le curseur du flux M12 ne dépasse jamais `pg_snapshot_xmin(pg_current_snapshot())`. Une transaction très longue,
//! une transaction préparée orpheline (2PC), un slot de réplication ou une session `idle in transaction` qui porte un
//! identifiant gèle cette borne **pour tous les établissements** : le temps réel cesse d'avancer (il rattrape à la
//! libération, rien n'est perdu), et le nettoyage de la base est aussi retenu. Aucune lecture de `pg_stat_activity`
//! (volontairement retirée, droit `pg_read_all_stats`) : on observe seulement que `xmin` ne bouge plus alors que
//! d'autres identifiants ont été consommés (`xmax > xmin`).
//!
//! Seuil : variable de déploiement [`VARIABLE_SEUIL`] (secondes), 10 minutes par défaut. Une alerte `error!` au premier
//! dépassement, rappelée à chaque nouveau seuil écoulé ; un message `info!` à la libération.

use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Duration, Utc};

/// Variable de déploiement du seuil d'alerte (secondes).
pub const VARIABLE_SEUIL: &str = "ENT_MSG_XMIN_ALERTE_SECONDES";
/// Seuil par défaut : 10 minutes.
pub const SEUIL_PAR_DEFAUT_SECONDES: i64 = 600;

#[derive(Debug, Default)]
pub struct SuiviXmin {
    xmin: u64,
    depuis: Option<DateTime<Utc>>,
    derniere_alerte: Option<DateTime<Utc>>,
}

/// Ce que l'observation décide.
#[derive(Debug, PartialEq, Eq)]
pub enum Observation {
    Rien,
    /// `xmin` gelé depuis cette durée (au-delà du seuil) : alerter.
    Gele(Duration),
    /// `xmin` a de nouveau avancé après une alerte.
    Libere,
}

impl SuiviXmin {
    /// `maintenant` est l'heure de la **base** (jamais l'horloge du processus : testable, règle 33).
    pub fn observer(
        &mut self,
        xmin: u64,
        xmax: u64,
        maintenant: DateTime<Utc>,
        seuil: Duration,
    ) -> Observation {
        if self.depuis.is_none() || xmin != self.xmin || xmax <= xmin {
            // Avancée, ou rien d'ouvert (`xmax == xmin` : aucune transaction en cours, borne au repos).
            let etait_alerte = self.derniere_alerte.is_some() && xmin != self.xmin;
            self.xmin = xmin;
            self.depuis = Some(maintenant);
            let avait = self.derniere_alerte.take();
            return if etait_alerte && avait.is_some() {
                Observation::Libere
            } else {
                Observation::Rien
            };
        }
        let depuis = self.depuis.unwrap_or(maintenant);
        let gele = maintenant - depuis;
        if gele < seuil {
            return Observation::Rien;
        }
        if self.derniere_alerte.is_some_and(|a| maintenant - a < seuil) {
            return Observation::Rien;
        }
        self.derniere_alerte = Some(maintenant);
        Observation::Gele(gele)
    }
}

fn seuil_configure() -> Duration {
    static SEUIL: OnceLock<Duration> = OnceLock::new();
    *SEUIL.get_or_init(|| {
        let s = std::env::var(VARIABLE_SEUIL)
            .ok()
            .and_then(|v| v.trim().parse::<i64>().ok())
            .filter(|s| *s > 0)
            .unwrap_or(SEUIL_PAR_DEFAUT_SECONDES);
        Duration::seconds(s)
    })
}

/// Observe la borne du flux et journalise. Appelée à chaque lecture de la borne.
pub fn surveiller(xmin: u64, xmax: u64, maintenant: DateTime<Utc>) {
    static SUIVI: OnceLock<Mutex<SuiviXmin>> = OnceLock::new();
    let suivi = SUIVI.get_or_init(|| Mutex::new(SuiviXmin::default()));
    let Ok(mut s) = suivi.lock() else { return };
    match s.observer(xmin, xmax, maintenant, seuil_configure()) {
        Observation::Rien => {}
        Observation::Gele(d) => tracing::error!(
            xmin,
            gele_depuis_secondes = d.num_seconds(),
            "borne xmin de la messagerie gelée : transaction longue, transaction préparée orpheline ou slot de réplication ; \
             le flux temps réel ne progresse plus pour aucun établissement, le nettoyage de la base est retenu"
        ),
        Observation::Libere => tracing::info!(xmin, "borne xmin de la messagerie de nouveau en mouvement"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(s: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 20, 10, 0, 0).unwrap() + Duration::seconds(s)
    }

    #[test]
    fn xmin_fige_au_dela_du_seuil_alerte_puis_rappelle_puis_se_libere() {
        let seuil = Duration::seconds(600);
        let mut s = SuiviXmin::default();
        assert_eq!(
            s.observer(100, 150, t(0), seuil),
            Observation::Rien,
            "première observation"
        );
        assert_eq!(
            s.observer(100, 160, t(599), seuil),
            Observation::Rien,
            "sous le seuil"
        );
        assert_eq!(
            s.observer(100, 170, t(600), seuil),
            Observation::Gele(Duration::seconds(600))
        );
        assert_eq!(
            s.observer(100, 180, t(900), seuil),
            Observation::Rien,
            "pas de rappel avant un nouveau seuil"
        );
        assert_eq!(
            s.observer(100, 190, t(1200), seuil),
            Observation::Gele(Duration::seconds(1200)),
            "rappel"
        );
        assert_eq!(s.observer(200, 210, t(1210), seuil), Observation::Libere);
        assert_eq!(
            s.observer(200, 220, t(1300), seuil),
            Observation::Rien,
            "repart de zéro"
        );
    }

    #[test]
    fn une_base_au_repos_n_alerte_jamais() {
        // `xmax == xmin` : aucune transaction ouverte, `xmin` ne bouge pas faute d'écriture, pas de gel.
        let seuil = Duration::seconds(600);
        let mut s = SuiviXmin::default();
        for k in 0..10 {
            assert_eq!(s.observer(500, 500, t(k * 3600), seuil), Observation::Rien);
        }
    }
}
