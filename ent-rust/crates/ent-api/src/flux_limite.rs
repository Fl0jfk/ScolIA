//! Limite de débit et de concurrence pour `GET /messages/flux` (M12, F1 puis M-D).
//!
//! Une interrogation longue garde une connexion et une connexion de base à chaque tour : sans borne, un client en
//! boucle avec `attente=0` martèle la base. Règles :
//! - **par session** (et non par compte : un compte a un appareil par session, tous doivent recevoir le temps réel) :
//!   un seul appel à la fois et au plus un début d'appel par seconde ; dépassement : `429` avec `Retry-After` ;
//! - **plafond par compte** : au plus [`FLUX_APPELS_MAX_PAR_COMPTE`] appels simultanés sur l'ensemble des sessions
//!   (protège la base d'un compte aux nombreuses sessions) ; il ne s'applique **pas** à un rappel de pagination (F2) ;
//! - **jamais de refus de pagination** : quand la dernière réponse d'une session était `tronque`, le rappel
//!   immédiat est accepté sans attendre l'intervalle (c'est le serveur qui s'en souvient, pas un paramètre du
//!   client, donc aucun contournement par `apres_fil`).
//!
//! État en mémoire de l'instance de [`LimiteFlux`] (une par `AppState`, comme l'anti-abus de connexion, jamais un état
//! global partagé entre tests) ; les entrées inactives sont purgées.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use uuid::Uuid;

/// Intervalle minimal entre deux débuts d'appel d'une même session.
pub const FLUX_INTERVALLE_MIN: Duration = Duration::from_secs(1);
/// Appels simultanés au plus par compte (toutes sessions confondues).
pub const FLUX_APPELS_MAX_PAR_COMPTE: usize = 8;

#[derive(Debug, PartialEq, Eq)]
pub enum RefusFlux {
    /// Un appel de cette session est déjà en cours, ou le plafond du compte est atteint.
    Concurrent,
    /// Trop rapproché du précédent.
    TropVite,
}

struct EtatSession {
    en_cours: bool,
    dernier: Instant,
    /// La dernière réponse était tronquée : le rappel immédiat est la pagination, jamais refusée.
    rappel_libre: bool,
}

#[derive(Default)]
struct Etat {
    sessions: HashMap<Uuid, EtatSession>,
    par_compte: HashMap<Uuid, usize>,
}

#[derive(Clone)]
pub struct LimiteFlux {
    inner: Arc<Mutex<Etat>>,
    /// Intervalle minimal entre deux débuts d'appel d'une même session (réglable : les tests HTTP en fixent un très
    /// long pour que « trop vite » ne dépende d'aucune horloge).
    intervalle: Duration,
}

impl Default for LimiteFlux {
    fn default() -> Self {
        Self::avec_intervalle(FLUX_INTERVALLE_MIN)
    }
}

/// Jeton rendu à l'entrée ; la place est libérée à sa chute (y compris sur erreur ou annulation de la requête).
pub struct PlaceFlux {
    limite: LimiteFlux,
    user: Uuid,
    session: Uuid,
    tronque: bool,
}

impl PlaceFlux {
    /// Signale que la réponse rendue est tronquée : le prochain appel de la session n'attend pas l'intervalle.
    pub fn marquer_tronque(&mut self, tronque: bool) {
        self.tronque = tronque;
    }
}

impl Drop for PlaceFlux {
    fn drop(&mut self) {
        if let Ok(mut m) = self.limite.inner.lock() {
            if let Some(e) = m.sessions.get_mut(&self.session) {
                e.en_cours = false;
                e.rappel_libre = self.tronque;
            }
            if let Some(n) = m.par_compte.get_mut(&self.user) {
                *n = n.saturating_sub(1);
                if *n == 0 {
                    m.par_compte.remove(&self.user);
                }
            }
        }
    }
}

impl LimiteFlux {
    /// Limiteur à intervalle minimal réglé.
    pub fn avec_intervalle(intervalle: Duration) -> Self {
        Self {
            inner: Arc::default(),
            intervalle,
        }
    }

    /// Valeur de l'en-tête `Retry-After` (secondes) d'un refus.
    pub fn retry_after_secondes(&self, refus: &RefusFlux) -> u64 {
        match refus {
            RefusFlux::Concurrent => 2,
            RefusFlux::TropVite => self.intervalle.as_secs().max(1),
        }
    }

    pub fn entrer(&self, user: Uuid, session: Uuid) -> Result<PlaceFlux, RefusFlux> {
        self.entrer_a(user, session, Instant::now())
    }

    /// Variante à horloge injectée (tests sans attente).
    pub fn entrer_a(
        &self,
        user: Uuid,
        session: Uuid,
        maintenant: Instant,
    ) -> Result<PlaceFlux, RefusFlux> {
        let mut m = self.inner.lock().map_err(|_| RefusFlux::Concurrent)?;
        // Purge : une session inactive depuis plus d'une minute n'occupe plus rien.
        if m.sessions.len() > 1024 {
            m.sessions.retain(|_, e| {
                e.en_cours
                    || maintenant.saturating_duration_since(e.dernier) < Duration::from_secs(60)
            });
        }
        // F2 : le rappel d'une pagination (dernière réponse de cette session tronquée, aucun appel en cours) n'est jamais
        // refusé, plafond du compte compris.
        let rappel_libre = m
            .sessions
            .get(&session)
            .is_some_and(|e| !e.en_cours && e.rappel_libre);
        if !rappel_libre
            && m.par_compte.get(&user).copied().unwrap_or(0) >= FLUX_APPELS_MAX_PAR_COMPTE
        {
            return Err(RefusFlux::Concurrent);
        }
        if let Some(e) = m.sessions.get_mut(&session) {
            if e.en_cours {
                return Err(RefusFlux::Concurrent);
            }
            if !e.rappel_libre && maintenant.saturating_duration_since(e.dernier) < self.intervalle
            {
                return Err(RefusFlux::TropVite);
            }
            e.en_cours = true;
            e.rappel_libre = false;
            e.dernier = maintenant;
        } else {
            m.sessions.insert(
                session,
                EtatSession {
                    en_cours: true,
                    dernier: maintenant,
                    rappel_libre: false,
                },
            );
        }
        *m.par_compte.entry(user).or_insert(0) += 1;
        drop(m);
        Ok(PlaceFlux {
            limite: self.clone(),
            user,
            session,
            tronque: false,
        })
    }

    /// Une place est-elle occupée par cette session ? (tests : barrière à la place d'une attente).
    #[cfg(test)]
    pub fn occupee(&self, session: Uuid) -> bool {
        self.inner
            .lock()
            .map(|m| m.sessions.get(&session).is_some_and(|e| e.en_cours))
            .unwrap_or(false)
    }

    /// Appels en cours pour ce compte, toutes sessions confondues (tests : barrière à la place d'une attente).
    #[cfg(test)]
    pub fn appels_en_cours(&self, user: Uuid) -> usize {
        self.inner
            .lock()
            .map(|m| m.par_compte.get(&user).copied().unwrap_or(0))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_seul_appel_a_la_fois_par_session_et_pas_de_martelement() {
        let l = LimiteFlux::default();
        let (u, a, b) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let t0 = Instant::now();
        let place = l.entrer_a(u, a, t0).expect("premier appel");
        assert_eq!(
            l.entrer_a(u, a, t0).err(),
            Some(RefusFlux::Concurrent),
            "second appel de la même session"
        );
        assert!(
            l.entrer_a(u, b, t0).is_ok(),
            "une autre session du même compte n'est pas gênée (M-D)"
        );
        drop(place);
        assert_eq!(
            l.entrer_a(u, a, t0).err(),
            Some(RefusFlux::TropVite),
            "rappel immédiat (attente=0)"
        );
        assert!(
            l.entrer_a(u, a, t0 + FLUX_INTERVALLE_MIN).is_ok(),
            "après l'intervalle minimal"
        );
    }

    #[test]
    fn deux_sessions_d_un_meme_compte_recoivent_toutes_deux() {
        let l = LimiteFlux::default();
        let (u, s1, s2) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let t0 = Instant::now();
        let p1 = l.entrer_a(u, s1, t0).expect("appareil 1");
        let p2 = l.entrer_a(u, s2, t0).expect("appareil 2 en même temps");
        drop((p1, p2));
        let t1 = t0 + FLUX_INTERVALLE_MIN;
        assert!(l.entrer_a(u, s1, t1).is_ok() && l.entrer_a(u, s2, t1).is_ok());
    }

    #[test]
    fn plafond_par_compte_et_liberation() {
        let l = LimiteFlux::default();
        let u = Uuid::new_v4();
        let t0 = Instant::now();
        let places: Vec<_> = (0..FLUX_APPELS_MAX_PAR_COMPTE)
            .map(|_| l.entrer_a(u, Uuid::new_v4(), t0).expect("sous le plafond"))
            .collect();
        assert_eq!(
            l.entrer_a(u, Uuid::new_v4(), t0).err(),
            Some(RefusFlux::Concurrent),
            "plafond du compte"
        );
        assert!(
            l.entrer_a(Uuid::new_v4(), Uuid::new_v4(), t0).is_ok(),
            "un autre compte n'est pas gêné"
        );
        drop(places);
        assert!(l.entrer_a(u, Uuid::new_v4(), t0).is_ok(), "libéré");
    }

    #[test]
    fn la_pagination_tronquee_n_est_jamais_refusee() {
        let l = LimiteFlux::default();
        let (u, s) = (Uuid::new_v4(), Uuid::new_v4());
        let t0 = Instant::now();
        for _ in 0..50 {
            let mut place = l
                .entrer_a(u, s, t0)
                .expect("rappel d'une réponse tronquée : jamais de 429");
            place.marquer_tronque(true);
        }
        // Dès qu'une réponse n'est plus tronquée, l'intervalle s'applique de nouveau.
        let place = l
            .entrer_a(u, s, t0)
            .expect("dernier appel de la pagination");
        drop(place);
        assert_eq!(l.entrer_a(u, s, t0).err(), Some(RefusFlux::TropVite));
    }

    #[test]
    fn retry_after_est_fourni() {
        let l = LimiteFlux::default();
        assert_eq!(l.retry_after_secondes(&RefusFlux::TropVite), 1);
        assert!(l.retry_after_secondes(&RefusFlux::Concurrent) >= 1);
        let long = LimiteFlux::avec_intervalle(Duration::from_secs(3600));
        assert_eq!(long.retry_after_secondes(&RefusFlux::TropVite), 3600);
    }

    #[test]
    fn le_plafond_du_compte_n_ecrase_pas_une_pagination() {
        // F2 : un compte au plafond (8 appels longs en cours) ne refuse pas le rappel d'une pagination tronquée ;
        // une session sans pagination en cours, elle, est refusée.
        let l = LimiteFlux::default();
        let (u, pagine, autre) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let t0 = Instant::now();
        let mut p = l.entrer_a(u, pagine, t0).expect("premier appel");
        p.marquer_tronque(true);
        drop(p);
        let longs: Vec<_> = (0..FLUX_APPELS_MAX_PAR_COMPTE)
            .map(|_| l.entrer_a(u, Uuid::new_v4(), t0).expect("sous le plafond"))
            .collect();
        assert_eq!(
            l.entrer_a(u, autre, t0).err(),
            Some(RefusFlux::Concurrent),
            "témoin : une session sans pagination est refusée au plafond"
        );
        let suite = l
            .entrer_a(u, pagine, t0)
            .expect("le rappel d'une pagination passe au-dessus du plafond");
        drop(suite);
        drop(longs);
    }
}
