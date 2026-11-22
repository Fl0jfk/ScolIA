//! Limite de débit et de concurrence par compte pour `GET /messages/flux` (M12, F1).
//!
//! Une interrogation longue garde une connexion et une connexion de base à chaque tour : sans borne, un compte (ou un
//! client en boucle avec `attente=0`) martèle la base. Règles, par compte :
//! - **un seul appel à la fois** (un second, pendant qu'un premier attend, reçoit `429`) ;
//! - **au plus un appel par seconde** (`attente=0` ne permet donc pas le martèlement). Dépassement : `429`.
//!
//! État en mémoire du processus (comme l'anti-abus de connexion) ; les entrées inactives sont purgées.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use uuid::Uuid;

/// Intervalle minimal entre deux débuts d'appel d'un même compte.
pub const FLUX_INTERVALLE_MIN: Duration = Duration::from_secs(1);

#[derive(Debug, PartialEq, Eq)]
pub enum RefusFlux {
    /// Un appel de ce compte est déjà en cours.
    Concurrent,
    /// Trop rapproché du précédent.
    TropVite,
}

struct Etat {
    en_cours: bool,
    dernier: Instant,
}

#[derive(Clone, Default)]
pub struct LimiteFlux {
    inner: Arc<Mutex<HashMap<Uuid, Etat>>>,
}

/// Jeton rendu à l'entrée ; la place est libérée à sa chute (y compris sur erreur ou annulation de la requête).
pub struct PlaceFlux {
    limite: LimiteFlux,
    user: Uuid,
}

impl Drop for PlaceFlux {
    fn drop(&mut self) {
        if let Ok(mut m) = self.limite.inner.lock() {
            if let Some(e) = m.get_mut(&self.user) {
                e.en_cours = false;
            }
        }
    }
}

impl LimiteFlux {
    pub fn entrer(&self, user: Uuid) -> Result<PlaceFlux, RefusFlux> {
        let mut m = self.inner.lock().map_err(|_| RefusFlux::Concurrent)?;
        let maintenant = Instant::now();
        // Purge : un compte inactif depuis plus d'une minute n'occupe plus rien.
        if m.len() > 1024 {
            m.retain(|_, e| {
                e.en_cours || maintenant.duration_since(e.dernier) < Duration::from_secs(60)
            });
        }
        if let Some(e) = m.get_mut(&user) {
            if e.en_cours {
                return Err(RefusFlux::Concurrent);
            }
            if maintenant.duration_since(e.dernier) < FLUX_INTERVALLE_MIN {
                return Err(RefusFlux::TropVite);
            }
            e.en_cours = true;
            e.dernier = maintenant;
        } else {
            m.insert(
                user,
                Etat {
                    en_cours: true,
                    dernier: maintenant,
                },
            );
        }
        drop(m);
        Ok(PlaceFlux {
            limite: self.clone(),
            user,
        })
    }
}

/// Limiteur du processus (une seule instance).
pub fn limite_flux() -> &'static LimiteFlux {
    static L: OnceLock<LimiteFlux> = OnceLock::new();
    L.get_or_init(LimiteFlux::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_seul_appel_a_la_fois_et_pas_de_martelement() {
        let l = LimiteFlux::default();
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let place = l.entrer(a).expect("premier appel");
        assert_eq!(
            l.entrer(a).err(),
            Some(RefusFlux::Concurrent),
            "second appel pendant le premier"
        );
        assert!(l.entrer(b).is_ok(), "un autre compte n'est pas gêné");
        drop(place);
        assert_eq!(
            l.entrer(a).err(),
            Some(RefusFlux::TropVite),
            "rappel immédiat (attente=0)"
        );
    }
}
