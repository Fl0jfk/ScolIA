//! Échéances de la session web (D-011) : logique **pure**, testée en natif.
//!
//! Le serveur (base) fait foi : inactivité de 2 h et durée maximale de 12 h. Le navigateur
//! n'en garde qu'une **estimation** pour prévenir la personne avant l'expiration :
//! - l'expiration absolue vient de `GET /web/session/etat` (lue en base) ;
//! - l'inactivité se déduit de la dernière réponse du serveur qui a reconnu la session
//!   (ce onglet ou un autre, via la diffusion entre onglets).
//!
//! L'estimation est volontairement prudente ([`MARGE_S`]) : le serveur n'écrit l'activité
//! qu'une fois par minute au plus, son délai d'inactivité peut donc tomber un peu plus tôt.

/// Avertissement affiché quand il reste au plus ce délai (secondes).
pub const AVERTISSEMENT_S: i64 = 300;

/// Marge de prudence retirée du délai d'inactivité (écriture d'activité amortie côté base).
pub const MARGE_S: i64 = 60;

/// Échéances connues, en secondes Unix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Echeances {
    /// Expiration absolue (durée maximale).
    pub absolue: i64,
    /// Dernière réponse du serveur qui a reconnu la session.
    pub derniere_activite: i64,
    /// Inactivité maximale annoncée par le serveur.
    pub inactivite_max: i64,
}

/// Où en est la session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Plus de [`AVERTISSEMENT_S`] devant soi.
    Active,
    /// Expire bientôt : proposer de rester connecté.
    Avertir { reste: i64 },
    /// Échéance dépassée : reconnexion nécessaire.
    Expiree,
}

impl Echeances {
    /// État renvoyé par le serveur à l'instant `maintenant`.
    pub fn depuis_etat(maintenant: i64, expire_dans: u64, inactivite_max: u64) -> Self {
        Self {
            absolue: maintenant.saturating_add(expire_dans.min(i64::MAX as u64 / 2) as i64),
            derniere_activite: maintenant,
            inactivite_max: inactivite_max.min(i64::MAX as u64 / 2) as i64,
        }
    }

    /// Une activité plus récente (autre onglet, nouvel appel) repousse l'inactivité ;
    /// jamais l'expiration absolue, et jamais en arrière.
    pub fn avec_activite(self, instant: i64) -> Self {
        Self {
            derniere_activite: self.derniere_activite.max(instant),
            ..self
        }
    }

    /// Secondes avant la perte de la session si la personne n'agit plus.
    pub fn reste(&self, maintenant: i64) -> i64 {
        let absolue = self.absolue - maintenant;
        let inactivite = self.derniere_activite + self.inactivite_max - MARGE_S - maintenant;
        absolue.min(inactivite)
    }

    pub fn phase(&self, maintenant: i64) -> Phase {
        match self.reste(maintenant) {
            r if r <= 0 => Phase::Expiree,
            r if r <= AVERTISSEMENT_S => Phase::Avertir { reste: r },
            _ => Phase::Active,
        }
    }
}

/// « 5 min », « 1 min », « moins d'une minute » (arrondi à la minute supérieure).
pub fn libelle_reste(reste: i64) -> String {
    if reste <= 60 {
        "moins d'une minute".into()
    } else {
        format!("{} min", (reste + 59) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_000_000;

    fn session() -> Echeances {
        // 12 h maximum, 2 h d'inactivité.
        Echeances::depuis_etat(T0, 12 * 3600, 2 * 3600)
    }

    #[test]
    fn active_puis_avertissement_puis_expiree_par_inactivite() {
        let e = session();
        assert_eq!(e.phase(T0), Phase::Active);
        // Inactivité 2 h moins la marge de 60 s.
        let fin = T0 + 2 * 3600 - MARGE_S;
        assert_eq!(e.reste(T0), 2 * 3600 - MARGE_S);
        assert_eq!(e.phase(fin - AVERTISSEMENT_S - 1), Phase::Active);
        assert_eq!(
            e.phase(fin - AVERTISSEMENT_S),
            Phase::Avertir {
                reste: AVERTISSEMENT_S
            }
        );
        assert_eq!(e.phase(fin - 1), Phase::Avertir { reste: 1 });
        assert_eq!(e.phase(fin), Phase::Expiree);
        assert_eq!(e.phase(fin + 10_000), Phase::Expiree);
    }

    #[test]
    fn l_activite_repousse_l_inactivite_pas_l_expiration_absolue() {
        let e = session().avec_activite(T0 + 3600);
        assert_eq!(e.reste(T0 + 3600), 2 * 3600 - MARGE_S);
        // Activité ancienne ou identique : ignorée (jamais en arrière).
        assert_eq!(e.avec_activite(T0).derniere_activite, T0 + 3600);
        // Près de la durée maximale : c'est l'échéance absolue qui compte.
        let tard = T0 + 12 * 3600 - 120;
        let e = session().avec_activite(tard);
        assert_eq!(e.reste(tard), 120);
        assert_eq!(e.phase(tard), Phase::Avertir { reste: 120 });
        assert_eq!(e.phase(T0 + 12 * 3600), Phase::Expiree);
    }

    #[test]
    fn session_d_enrolement_sans_inactivite_courte() {
        // Session d'enrôlement : 15 min absolues ; l'inactivité (2 h) ne joue pas.
        let e = Echeances::depuis_etat(T0, 15 * 60, 2 * 3600);
        assert_eq!(e.reste(T0), 15 * 60);
        assert_eq!(e.phase(T0 + 15 * 60 - 200), Phase::Avertir { reste: 200 });
    }

    #[test]
    fn valeurs_extremes_sans_debordement() {
        let e = Echeances::depuis_etat(T0, u64::MAX, u64::MAX);
        assert_eq!(e.phase(T0), Phase::Active);
        assert!(e.reste(T0) > 0);
    }

    #[test]
    fn libelles_de_delai() {
        assert_eq!(libelle_reste(300), "5 min");
        assert_eq!(libelle_reste(241), "5 min");
        assert_eq!(libelle_reste(61), "2 min");
        assert_eq!(libelle_reste(60), "moins d'une minute");
        assert_eq!(libelle_reste(1), "moins d'une minute");
    }
}
