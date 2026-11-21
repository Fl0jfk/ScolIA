//! Changement d'adresse e-mail en deux temps (revue socle 1, RF-01).
//!
//! `demandee` → (lien envoyé à la NOUVELLE adresse) → `confirmee` : l'adresse change
//! alors, sessions révoquées, liens de mot de passe et récupérations annulés. Le lien
//! d'annulation, envoyé à l'ANCIENNE adresse, annule une demande non confirmée ou,
//! jusqu'à 72 h après la confirmation, **retablit** l'ancienne adresse (`revertee`).
//! Un changement fait par un tiers gèle 72 h la réinitialisation de mot de passe et la
//! récupération de passkey (`ent_user.recuperation_gel_jusqu`). Chaque transition est
//! atomique ; seuls les SHA-256 des jetons sont stockés.

use ent_auth::AuthError;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Durée du gel et fenêtre de retablissement (heures).
pub const GEL_HEURES: i32 = 72;
/// Validité du lien de confirmation (minutes).
pub const CONFIRMATION_TTL_MINUTES: i32 = 60;

/// Issue d'une confirmation ou d'une annulation.
#[derive(Debug, Clone)]
pub struct ChangementEmail {
    #[allow(dead_code)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub etablissement_id: Uuid,
    pub actor_user_id: Uuid,
    pub tiers: bool,
    pub ancien_email: String,
    pub nouveau_email: String,
    /// Annulation : l'ancienne adresse a été retablie (changement déjà confirmé).
    pub retabli: bool,
    pub sessions_revoquees: u64,
}

pub struct PgEmailChangeRepo {
    pool: PgPool,
}

type Ligne = (Uuid, Uuid, Uuid, Uuid, bool, String, String);

/// Gèle (si `tiers`), annule les récupérations de passkey en cours, invalide les
/// liens de mot de passe et révoque les sessions. Même transaction que le changement.
async fn securiser(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    gel: bool,
) -> Result<u64, sqlx::Error> {
    if gel {
        sqlx::query(
            "UPDATE ent_user SET recuperation_gel_jusqu = GREATEST(
                 COALESCE(recuperation_gel_jusqu, now()), now() + make_interval(hours => $2))
             WHERE id = $1",
        )
        .bind(user_id)
        .bind(GEL_HEURES)
        .execute(&mut **tx)
        .await?;
        // Les identités SSO déjà liées (rattachées à l'ancienne adresse) sont coupées.
        sqlx::query("DELETE FROM ent_oidc_identity WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
    }
    sqlx::query(
        "UPDATE ent_recuperation_passkey SET statut = 'annulee', closed_at = now()
         WHERE user_id = $1 AND statut IN ('demandee', 'en_attente')",
    )
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "UPDATE ent_password_reset SET used_at = now() WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    Ok(sqlx::query(
        "UPDATE ent_session SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(&mut **tx)
    .await?
    .rows_affected())
}

fn email_pris(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.constraint() == Some("ent_user_email_unique"))
}

impl PgEmailChangeRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Enregistre une demande (la précédente, non confirmée, est annulée).
    #[allow(clippy::too_many_arguments)]
    pub async fn creer(
        &self,
        user_id: Uuid,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        tiers: bool,
        ancien: &str,
        nouveau: &str,
        confirm_hash: &str,
        annul_hash: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM ent_user WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE ent_email_change SET statut = 'annulee', closed_at = now()
             WHERE user_id = $1 AND statut = 'demandee'",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_email_change
                 (id, user_id, etablissement_id, actor_user_id, tiers, ancien_email,
                  nouveau_email, confirm_hash, annul_hash, confirm_expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now() + make_interval(mins => $10))",
        )
        .bind(id)
        .bind(user_id)
        .bind(etablissement_id)
        .bind(actor_user_id)
        .bind(tiers)
        .bind(ancien)
        .bind(nouveau)
        .bind(confirm_hash)
        .bind(annul_hash)
        .bind(CONFIRMATION_TTL_MINUTES)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Abandon (envoi impossible) d'une demande non confirmée.
    pub async fn abandonner(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE ent_email_change SET statut = 'annulee', closed_at = now()
             WHERE id = $1 AND statut = 'demandee'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Gel + annulation des récupérations dès la demande d'un tiers (appelé après
    /// l'envoi réussi du lien de confirmation).
    pub async fn geler_et_annuler_recuperations(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        securiser_sans_sessions(&mut tx, user_id).await?;
        tx.commit().await
    }

    /// Confirmation (usage unique, non expirée) : applique le changement.
    /// `Ok(None)` : lien inconnu, utilisé, annulé ou expiré.
    pub async fn confirmer(
        &self,
        confirm_hash: &str,
    ) -> Result<Option<ChangementEmail>, AuthError> {
        let err = |_| AuthError::InvalidCredentials;
        let mut tx = self.pool.begin().await.map_err(err)?;
        // Même ordre de verrous que `creer` : la personne d'abord, la demande ensuite.
        self.verrouiller_personne(&mut tx, "confirm_hash", confirm_hash).await.map_err(err)?;
        let row: Option<Ligne> = sqlx::query_as(
            "UPDATE ent_email_change SET statut = 'confirmee', confirmed_at = now()
             WHERE confirm_hash = $1 AND statut = 'demandee' AND confirm_expires_at > now()
             RETURNING id, user_id, etablissement_id, actor_user_id, tiers,
                       ancien_email, nouveau_email",
        )
        .bind(confirm_hash)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?;
        let Some((id, user_id, etab, actor, tiers, ancien, nouveau)) = row else {
            return Ok(None);
        };
        let maj = sqlx::query("UPDATE ent_user SET email = $2, updated_at = now() WHERE id = $1")
            .bind(user_id)
            .bind(&nouveau)
            .execute(&mut *tx)
            .await;
        match maj {
            Ok(r) if r.rows_affected() > 0 => {}
            Err(e) if email_pris(&e) => return Err(AuthError::EmailTaken),
            _ => return Err(AuthError::InvalidCredentials),
        }
        let sessions = securiser(&mut tx, user_id, tiers).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(Some(ChangementEmail {
            id,
            user_id,
            etablissement_id: etab,
            actor_user_id: actor,
            tiers,
            ancien_email: ancien,
            nouveau_email: nouveau,
            retabli: false,
            sessions_revoquees: sessions,
        }))
    }

    /// Annulation par le lien envoyé à l'ancienne adresse : annule une demande en
    /// attente, ou retablit l'ancienne adresse si la confirmation date de moins de 72 h
    /// (le gel est alors maintenu). `Ok(None)` : lien invalide ou fenêtre dépassée.
    pub async fn annuler(&self, annul_hash: &str) -> Result<Option<ChangementEmail>, AuthError> {
        let err = |_| AuthError::InvalidCredentials;
        let mut tx = self.pool.begin().await.map_err(err)?;
        self.verrouiller_personne(&mut tx, "annul_hash", annul_hash).await.map_err(err)?;
        let row: Option<(Uuid, Uuid, Uuid, Uuid, bool, String, String, String)> = sqlx::query_as(
            "SELECT id, user_id, etablissement_id, actor_user_id, tiers, ancien_email,
                    nouveau_email, statut
             FROM ent_email_change
             WHERE annul_hash = $1
               AND (statut = 'demandee'
                    OR (statut = 'confirmee'
                        AND confirmed_at > now() - make_interval(hours => $2)))
             FOR UPDATE",
        )
        .bind(annul_hash)
        .bind(GEL_HEURES)
        .fetch_optional(&mut *tx)
        .await
        .map_err(err)?;
        let Some((id, user_id, etab, actor, tiers, ancien, nouveau, statut)) = row else {
            return Ok(None);
        };
        let retabli = statut == "confirmee";
        let mut sessions = 0;
        if retabli {
            let maj = sqlx::query(
                "UPDATE ent_user SET email = $2, updated_at = now() WHERE id = $1",
            )
            .bind(user_id)
            .bind(&ancien)
            .execute(&mut *tx)
            .await;
            match maj {
                Ok(_) => {}
                Err(e) if email_pris(&e) => return Err(AuthError::EmailTaken),
                _ => return Err(AuthError::InvalidCredentials),
            }
            // Rétablissement : le gel est (re)posé quel que soit l'auteur initial. Les
            // changements confirmés ensuite (enchaînement) sont annulés avec celui-ci : la
            // personne retrouve l'adresse qu'elle avait avant la chaîne.
            sessions = securiser(&mut tx, user_id, true).await.map_err(err)?;
            sqlx::query(
                "UPDATE ent_email_change SET statut = 'revertee', closed_at = now()
                 WHERE user_id = $1 AND statut = 'confirmee'",
            )
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(err)?;
        }
        sqlx::query(
            "UPDATE ent_email_change SET statut = $2, closed_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(if retabli { "revertee" } else { "annulee" })
        .execute(&mut *tx)
        .await
        .map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(Some(ChangementEmail {
            id,
            user_id,
            etablissement_id: etab,
            actor_user_id: actor,
            tiers,
            ancien_email: ancien,
            nouveau_email: nouveau,
            retabli: retabli,
            sessions_revoquees: sessions,
        }))
    }

    /// Verrouille la ligne `ent_user` de la personne visée par un jeton (avant tout verrou
    /// sur `ent_email_change`, comme `creer`) : évite l'inversion d'ordre de verrous.
    async fn verrouiller_personne(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        colonne: &str,
        hash: &str,
    ) -> Result<(), sqlx::Error> {
        let sql = match colonne {
            "confirm_hash" => "SELECT user_id FROM ent_email_change WHERE confirm_hash = $1",
            _ => "SELECT user_id FROM ent_email_change WHERE annul_hash = $1",
        };
        let u: Option<(Uuid,)> = sqlx::query_as(sql).bind(hash).fetch_optional(&mut **tx).await?;
        if let Some((u,)) = u {
            sqlx::query("SELECT id FROM ent_user WHERE id = $1 FOR UPDATE")
                .bind(u)
                .fetch_optional(&mut **tx)
                .await?;
        }
        Ok(())
    }

    /// Demandes de changement d'e-mail créées pour cette personne sur 24 h.
    pub async fn demandes_recentes(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_email_change
             WHERE user_id = $1 AND created_at > now() - interval '24 hours'",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(n)
    }

    /// Le compte est-il gelé (réinitialisation / récupération suspendues) ?
    pub async fn est_gele(&self, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let r: Option<(bool,)> = sqlx::query_as(
            "SELECT COALESCE(recuperation_gel_jusqu > now(), false) FROM ent_user WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(r.map(|(g,)| g).unwrap_or(false))
    }

    /// Purge des demandes closes ou périmées de plus de 30 jours.
    pub async fn purger(&self) -> Result<u64, sqlx::Error> {
        Ok(sqlx::query(
            "DELETE FROM ent_email_change WHERE created_at < now() - interval '30 days'",
        )
        .execute(&self.pool)
        .await?
        .rows_affected())
    }
}

/// Gel + annulation des récupérations + invalidation des liens, sans toucher aux sessions.
async fn securiser_sans_sessions(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE ent_user SET recuperation_gel_jusqu = GREATEST(
             COALESCE(recuperation_gel_jusqu, now()), now() + make_interval(hours => $2))
         WHERE id = $1",
    )
    .bind(user_id)
    .bind(GEL_HEURES)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "UPDATE ent_recuperation_passkey SET statut = 'annulee', closed_at = now()
         WHERE user_id = $1 AND statut IN ('demandee', 'en_attente')",
    )
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "UPDATE ent_password_reset SET used_at = now() WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
