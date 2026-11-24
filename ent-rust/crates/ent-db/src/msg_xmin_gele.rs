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

/// Première observation d'un `xmin` **tenu** par une transaction ouverte (`xmax > xmin`).
#[derive(Debug, Clone, Copy)]
struct Base {
    xmin: u64,
    xmax: u64,
    depuis: DateTime<Utc>,
}

#[derive(Debug, Default)]
pub struct SuiviXmin {
    base: Option<Base>,
    derniere_alerte: Option<DateTime<Utc>>,
}

/// Ce que l'observation décide.
#[derive(Debug, PartialEq, Eq)]
pub enum Observation {
    Rien,
    /// `xmin` gelé depuis cette durée (au-delà du seuil) : alerter.
    Gele(Duration),
    /// `xmin` a de nouveau avancé (ou la base est au repos) après une alerte.
    Libere,
}

impl SuiviXmin {
    /// `maintenant` est l'heure de la **base** (jamais l'horloge du processus : testable, règle 33).
    ///
    /// Un gel n'est constaté que sur **deux observations d'une transaction ouverte** (`xmax > xmin`), espacées d'au moins
    /// le seuil, de même `xmin`, avec un `xmax` qui a **avancé** entre elles (des identifiants ont été consommés derrière
    /// la transaction). Une observation au repos (`xmax == xmin`) n'est jamais une base : après un long repos, une
    /// transaction brève ne ressemble pas à un gel (la borne n'avait pas été observée tenue).
    pub fn observer(
        &mut self,
        xmin: u64,
        xmax: u64,
        maintenant: DateTime<Utc>,
        seuil: Duration,
    ) -> Observation {
        let libere = |s: &mut Self| {
            if s.derniere_alerte.take().is_some() {
                Observation::Libere
            } else {
                Observation::Rien
            }
        };
        if xmax <= xmin {
            // Rien d'ouvert : borne au repos.
            self.base = None;
            return libere(self);
        }
        let Some(base) = self.base.filter(|b| b.xmin == xmin) else {
            // Première observation tenue, ou `xmin` a bougé : nouvelle base.
            self.base = Some(Base {
                xmin,
                xmax,
                depuis: maintenant,
            });
            return libere(self);
        };
        let gele = maintenant - base.depuis;
        if gele < seuil || xmax <= base.xmax {
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
            "première observation (la base)"
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

    #[test]
    fn repos_longue_pause_puis_transaction_breve_n_alerte_pas() {
        let seuil = Duration::seconds(600);
        let mut s = SuiviXmin::default();
        assert_eq!(
            s.observer(500, 500, t(0), seuil),
            Observation::Rien,
            "repos"
        );
        // Une heure plus tard, une transaction brève porte un identifiant : xmin identique, xmax avancé.
        assert_eq!(
            s.observer(500, 501, t(3600), seuil),
            Observation::Rien,
            "la borne n'avait jamais été observée tenue : pas de gel"
        );
        // Elle se termine : la borne avance.
        assert_eq!(s.observer(502, 502, t(3601), seuil), Observation::Rien);
        // Même chose avec une observation tenue brève dans l'intervalle.
        assert_eq!(s.observer(502, 503, t(7200), seuil), Observation::Rien);
        assert_eq!(s.observer(504, 504, t(7201), seuil), Observation::Rien);
    }

    #[test]
    fn un_xmax_qui_n_avance_pas_n_est_pas_un_gel() {
        // Transaction ouverte mais aucune autre écriture : les identifiants ne sont pas consommés derrière elle.
        let seuil = Duration::seconds(600);
        let mut s = SuiviXmin::default();
        assert_eq!(s.observer(100, 101, t(0), seuil), Observation::Rien);
        assert_eq!(s.observer(100, 101, t(5000), seuil), Observation::Rien);
        // Dès que des identifiants sont consommés derrière elle : gel constaté.
        assert_eq!(
            s.observer(100, 140, t(5001), seuil),
            Observation::Gele(Duration::seconds(5001))
        );
        // Le retour au repos libère.
        assert_eq!(s.observer(141, 141, t(5100), seuil), Observation::Libere);
    }
}
