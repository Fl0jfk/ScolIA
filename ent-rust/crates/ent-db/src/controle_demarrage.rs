//! Contrôles au démarrage du serveur : prérequis de la base que les migrations ne peuvent pas garantir seules.
//!
//! Aujourd'hui : le droit d'exécution du rôle applicatif sur `ent_msg_annulation_ouvrir` (migration 494). Le `GRANT`
//! de 494 n'est joué qu'une fois, pour les rôles existants à ce moment-là ; un rôle applicatif créé ou refait ensuite
//! ne l'a pas, et toute annulation de message différé échoue alors en 42501 (fermé, mais panne silencieuse). Le
//! contrôle ne fait qu'alerter : il ne corrige rien et n'écrit aucune donnée personnelle dans le journal.

use crate::service::PgEnt;
use sqlx::PgPool;

/// Signature SQL de la fonction du jeton d'annulation.
const FONCTION_ANNULATION: &str = "ent_msg_annulation_ouvrir(uuid,uuid,uuid)";

/// Résultat du contrôle du droit d'exécution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DroitAnnulation {
    /// Le rôle de la connexion peut exécuter la fonction.
    Accorde,
    /// Il ne le peut pas : `GRANT EXECUTE` manquant.
    Manquant,
    /// La fonction n'existe pas (migration 494 non appliquée).
    FonctionAbsente,
    /// Le contrôle n'a pas pu être fait (erreur de lecture) : simple avertissement.
    Indetermine,
}

/// `has_function_privilege(current_user, …, 'EXECUTE')` sur la connexion donnée.
pub(crate) async fn droit_annulation(pool: &PgPool) -> Result<DroitAnnulation, sqlx::Error> {
    let (existe, droit): (bool, Option<bool>) = sqlx::query_as(
        "SELECT to_regprocedure($1) IS NOT NULL,
                CASE WHEN to_regprocedure($1) IS NULL THEN NULL
                     ELSE has_function_privilege(current_user, $1, 'EXECUTE') END",
    )
    .bind(FONCTION_ANNULATION)
    .fetch_one(pool)
    .await?;
    Ok(match (existe, droit) {
        (false, _) => DroitAnnulation::FonctionAbsente,
        (true, Some(true)) => DroitAnnulation::Accorde,
        (true, _) => DroitAnnulation::Manquant,
    })
}

impl PgEnt {
    /// Au démarrage : journalise une **erreur** claire si le rôle applicatif ne peut pas exécuter
    /// `ent_msg_annulation_ouvrir`. Ne bloque pas le démarrage. Rend le résultat pour les tests.
    pub async fn controler_droit_annulation_messagerie(&self) -> DroitAnnulation {
        match droit_annulation(self.pool()).await {
            Ok(d @ (DroitAnnulation::Accorde | DroitAnnulation::Indetermine)) => d,
            Ok(DroitAnnulation::Manquant) => {
                tracing::error!(
                    fonction = FONCTION_ANNULATION,
                    "droit d'exécution manquant : le rôle applicatif ne peut pas exécuter {FONCTION_ANNULATION} \
                     (annulation des messages différés impossible, erreur 42501). Corriger : \
                     GRANT EXECUTE ON FUNCTION {FONCTION_ANNULATION} TO <rôle applicatif> (voir docs/RLS_ISOLATION.md)"
                );
                DroitAnnulation::Manquant
            }
            Ok(DroitAnnulation::FonctionAbsente) => {
                tracing::error!(
                    fonction = FONCTION_ANNULATION,
                    "fonction {FONCTION_ANNULATION} absente : la migration 494 n'est pas appliquée"
                );
                DroitAnnulation::FonctionAbsente
            }
            Err(e) => {
                tracing::warn!("contrôle du droit d'exécution de {FONCTION_ANNULATION} impossible : {e}");
                DroitAnnulation::Indetermine
            }
        }
    }
}
