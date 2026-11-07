//! Persistance de la récupération de passkey en libre-service (D-018,
//! table `ent_recuperation_passkey`).
//!
//! Cycle de vie : `demandee` (lien de confirmation envoyé) → `en_attente`
//! (confirmée, délai de sécurité en cours) → `terminee` (passkeys supprimées)
//! ou `annulee` / `expiree`. Chaque transition est **une requête atomique**
//! (`UPDATE … WHERE statut = … RETURNING`) : un jeton rejoué ne passe jamais
//! deux fois. Seuls les SHA-256 des jetons sont stockés.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Délai de sécurité par défaut (heures) et bornes.
const DELAI_DEFAUT_HEURES: i64 = 24;
const DELAI_MIN_HEURES: i64 = 1;
const DELAI_MAX_HEURES: i64 = 168;
/// Durée de validité du lien de confirmation (minutes).
pub const CONFIRMATION_TTL_MINUTES: i64 = 60;
/// Fenêtre de finalisation après la fin du délai (heures).
pub const FINALISATION_FENETRE_HEURES: i64 = 72;

/// Délai de sécurité configurable : `ENT_RECUPERATION_PASSKEY_DELAI_HEURES`
/// (défaut 24, borné à 1–168 ; valeur invalide → défaut).
pub fn delai_securite_heures() -> i64 {
    std::env::var("ENT_RECUPERATION_PASSKEY_DELAI_HEURES")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .map(|h| h.clamp(DELAI_MIN_HEURES, DELAI_MAX_HEURES))
        .unwrap_or(DELAI_DEFAUT_HEURES)
}

/// Demande confirmée, passée « en attente ».
#[derive(Debug, Clone)]
pub struct DemandeConfirmee {
    pub id: Uuid,
    pub user_id: Uuid,
    pub effective_at: DateTime<Utc>,
}

/// Issue d'une tentative de finalisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finalisation {
    /// Passkeys supprimées, sessions révoquées (nombres).
    Terminee {
        user_id: Uuid,
        passkeys: u64,
        sessions: u64,
    },
    /// Délai de sécurité pas encore écoulé (rien n'est consommé).
    DelaiEnCours(DateTime<Utc>),
    /// Jeton inconnu, déjà utilisé, annulé ou expiré.
    Invalide,
}

pub struct PgRecuperationRepo {
    pool: PgPool,
}

impl PgRecuperationRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Crée une demande `demandee`. Une demande non confirmée précédente est
    /// remplacée (un seul lien de confirmation actif) ; une demande déjà
    /// « en attente » n'est jamais écrasée (`Ok(None)`).
    pub async fn creer(
        &self,
        user_id: Uuid,
        etablissement_id: Uuid,
        confirm_hash: &str,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        // Sérialise les demandes d'une même personne.
        sqlx::query("SELECT id FROM ent_user WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
        let (en_attente,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_recuperation_passkey
             WHERE user_id = $1 AND statut = 'en_attente'",
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        if en_attente > 0 {
            tx.rollback().await?;
            return Ok(None);
        }
        sqlx::query(
            "UPDATE ent_recuperation_passkey
             SET statut = 'annulee', closed_at = now()
             WHERE user_id = $1 AND statut = 'demandee'",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_recuperation_passkey
                 (id, user_id, etablissement_id, confirm_hash, confirm_expires_at)
             VALUES ($1, $2, $3, $4, now() + make_interval(mins => $5::int))",
        )
        .bind(id)
        .bind(user_id)
        .bind(etablissement_id)
        .bind(confirm_hash)
        .bind(CONFIRMATION_TTL_MINUTES as i32)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(id))
    }

    /// Abandonne une demande `demandee` (envoi du lien impossible).
    pub async fn abandonner(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE ent_recuperation_passkey
             SET statut = 'annulee', closed_at = now()
             WHERE id = $1 AND statut IN ('demandee', 'en_attente')",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Confirme (usage unique, non expiré) : passe en « en attente », fixe la
    /// fin du délai et enregistre les empreintes des jetons d'annulation et de
    /// finalisation.
    pub async fn confirmer(
        &self,
        confirm_hash: &str,
        annul_hash: &str,
        final_hash: &str,
        delai_heures: i64,
    ) -> Result<Option<DemandeConfirmee>, sqlx::Error> {
        let row: Option<(Uuid, Uuid, DateTime<Utc>)> = sqlx::query_as(
            "UPDATE ent_recuperation_passkey
             SET statut = 'en_attente',
                 confirmed_at = now(),
                 effective_at = now() + make_interval(hours => $4::int),
                 final_expires_at = now() + make_interval(hours => $4::int + $5::int),
                 annul_hash = $2,
                 final_hash = $3
             WHERE confirm_hash = $1 AND statut = 'demandee' AND confirm_expires_at > now()
               AND NOT EXISTS (SELECT 1 FROM ent_user u
                               WHERE u.id = ent_recuperation_passkey.user_id
                                 AND u.recuperation_gel_jusqu > now())
             RETURNING id, user_id, effective_at",
        )
        .bind(confirm_hash)
        .bind(annul_hash)
        .bind(final_hash)
        .bind(delai_heures as i32)
        .bind(FINALISATION_FENETRE_HEURES as i32)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(id, user_id, effective_at)| DemandeConfirmee {
            id,
            user_id,
            effective_at,
        }))
    }

    /// Annulation par le lien d'annulation. Renvoie la personne concernée.
    pub async fn annuler_par_jeton(&self, annul_hash: &str) -> Result<Option<Uuid>, sqlx::Error> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_recuperation_passkey
             SET statut = 'annulee', closed_at = now()
             WHERE annul_hash = $1 AND statut = 'en_attente'
             RETURNING user_id",
        )
        .bind(annul_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(u,)| u))
    }

    /// Annule toute demande active de la personne (connexion réussie par
    /// passkey pendant le délai). Renvoie le nombre de demandes annulées.
    pub async fn annuler_pour_user(&self, user_id: Uuid) -> Result<u64, sqlx::Error> {
        let res = sqlx::query(
            "UPDATE ent_recuperation_passkey
             SET statut = 'annulee', closed_at = now()
             WHERE user_id = $1 AND statut IN ('demandee', 'en_attente')",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }

    /// Finalise (une seule transaction) : délai écoulé et jeton valide →
    /// suppression de **toutes** les passkeys, révocation de **toutes** les
    /// sessions, invalidation des liens de mot de passe encore actifs.
    pub async fn finaliser(&self, final_hash: &str) -> Result<Finalisation, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(Uuid, Uuid, DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, user_id, effective_at, final_expires_at
             FROM ent_recuperation_passkey
             WHERE final_hash = $1 AND statut = 'en_attente'
             FOR UPDATE",
        )
        .bind(final_hash)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((id, user_id, effective_at, final_expires_at)) = row else {
            return Ok(Finalisation::Invalide);
        };
        // RF-01 : gel de 72 h après un changement d'e-mail par un tiers.
        let (gele,): (bool,) = sqlx::query_as(
            "SELECT COALESCE(recuperation_gel_jusqu > now(), false) FROM ent_user WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        if gele {
            tx.rollback().await?;
            return Ok(Finalisation::Invalide);
        }
        let now: (DateTime<Utc>,) = sqlx::query_as("SELECT now()").fetch_one(&mut *tx).await?;
        let now = now.0;
        if now >= final_expires_at {
            sqlx::query(
                "UPDATE ent_recuperation_passkey SET statut = 'expiree', closed_at = now()
                 WHERE id = $1",
            )
            .bind(id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(Finalisation::Invalide);
        }
        if now < effective_at {
            tx.rollback().await?;
            return Ok(Finalisation::DelaiEnCours(effective_at));
        }
        let passkeys = sqlx::query("DELETE FROM ent_passkey_credential WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        sqlx::query("DELETE FROM ent_passkey_challenge WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        let sessions = sqlx::query(
            "UPDATE ent_session SET revoked_at = now()
             WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        sqlx::query(
            "UPDATE ent_password_reset SET used_at = now()
             WHERE user_id = $1 AND used_at IS NULL",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE ent_recuperation_passkey SET statut = 'terminee', closed_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Finalisation::Terminee {
            user_id,
            passkeys,
            sessions,
        })
    }

    /// Purge les demandes closes ou périmées de plus de 30 jours (maintenance).
    pub async fn purger(&self) -> Result<u64, sqlx::Error> {
        let res = sqlx::query(
            "DELETE FROM ent_recuperation_passkey
             WHERE created_at < $1
               AND (statut IN ('annulee', 'terminee', 'expiree')
                    OR COALESCE(final_expires_at, confirm_expires_at) < now())",
        )
        .bind(Utc::now() - Duration::days(30))
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delai_configurable_borne() {
        // Une seule fonction de test touche cette variable (pas de course).
        let cle = "ENT_RECUPERATION_PASSKEY_DELAI_HEURES";
        for (v, attendu) in [
            ("", 24),
            ("abc", 24),
            ("0", 1),
            ("-5", 1),
            ("1", 1),
            ("48", 48),
            ("168", 168),
            ("9999", 168),
        ] {
            std::env::set_var(cle, v);
            assert_eq!(delai_securite_heures(), attendu, "valeur « {v} »");
        }
        std::env::remove_var(cle);
        assert_eq!(delai_securite_heures(), 24);
    }
}
