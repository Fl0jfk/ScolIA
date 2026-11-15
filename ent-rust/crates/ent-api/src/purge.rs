//! MSE-D01 (socle 28, MS0) : purge **périodique** des sessions et des jetons expirés.
//!
//! Avant ce lot, la purge n'avait lieu qu'au démarrage : un serveur qui tourne trois mois
//! gardait trois mois de jetons expirés. Une passe est idempotente et ne supprime que ce qui
//! est déjà expiré (les requêtes d'authentification ignorent de toute façon les jetons
//! expirés : la purge ne change aucun droit, elle limite la taille des tables et la durée de
//! vie des secrets stockés).

use ent_db::PgEnt;
use std::sync::Arc;
use std::time::Duration;

/// Période par défaut : une heure (MSE-D01).
pub const PERIODE_PAR_DEFAUT: Duration = Duration::from_secs(3600);

/// Compte des lignes purgées par catégorie lors d'une passe.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BilanPurge {
    pub sessions: u64,
    pub reinitialisations_mot_de_passe: u64,
    pub recuperations_passkey: u64,
    pub changements_email: u64,
    /// Catégories dont la purge a échoué (journalisées ; la passe continue).
    pub echecs: u32,
}

impl BilanPurge {
    pub fn total(&self) -> u64 {
        self.sessions
            + self.reinitialisations_mot_de_passe
            + self.recuperations_passkey
            + self.changements_email
    }
}

/// Une passe de purge. Un échec d'une catégorie n'empêche pas les autres.
pub async fn une_passe(ent: &PgEnt) -> BilanPurge {
    let mut b = BilanPurge::default();
    macro_rules! cat {
        ($champ:ident, $appel:expr, $nom:literal) => {
            match $appel.await {
                Ok(n) => b.$champ = n,
                Err(e) => {
                    b.echecs += 1;
                    tracing::warn!(categorie = $nom, "purge périodique impossible : {e}");
                }
            }
        };
    }
    cat!(sessions, ent.purge_expired_sessions(), "sessions");
    cat!(
        reinitialisations_mot_de_passe,
        ent.purge_expired_password_resets(),
        "reinitialisations_mot_de_passe"
    );
    cat!(
        recuperations_passkey,
        ent.purger_recuperations_passkey(),
        "recuperations_passkey"
    );
    cat!(
        changements_email,
        ent.purger_changements_email(),
        "changements_email"
    );
    b
}

/// Période lue dans `ENT_PURGE_PERIODE_SECONDES` (entre 60 s et 24 h, sinon la valeur par défaut).
pub fn periode_depuis_env() -> Duration {
    std::env::var("ENT_PURGE_PERIODE_SECONDES")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|s| (60..=86_400).contains(s))
        .map(Duration::from_secs)
        .unwrap_or(PERIODE_PAR_DEFAUT)
}

/// Lance la tâche périodique (la première passe a lieu après une période : le démarrage a déjà purgé).
pub fn lancer(ent: Arc<PgEnt>, periode: Duration) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(periode).await;
            let b = une_passe(&ent).await;
            if b.total() > 0 {
                tracing::info!(?b, "purge périodique des sessions et jetons");
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periode_par_defaut_et_bornes() {
        // Valeur hors bornes ou absente → une heure.
        std::env::remove_var("ENT_PURGE_PERIODE_SECONDES");
        assert_eq!(periode_depuis_env(), PERIODE_PAR_DEFAUT);
    }

    #[test]
    fn total_somme_les_categories() {
        let b = BilanPurge {
            sessions: 2,
            reinitialisations_mot_de_passe: 1,
            recuperations_passkey: 3,
            changements_email: 4,
            echecs: 0,
        };
        assert_eq!(b.total(), 10);
    }
}
