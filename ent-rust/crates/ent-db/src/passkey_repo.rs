//! Persistance passkeys (`ent_passkey_credential`, `ent_passkey_challenge`).
//!
//! La structure WebAuthn elle-même (clé publique COSE, compteur) est stockée
//! telle quelle en JSONB : on ne réimplémente pas le format du vérificateur.
use chrono::{DateTime, Utc};
use ent_auth::{CredentialID, Passkey};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PasskeyRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Ligne brute d'un challenge (état WebAuthn opaque + métadonnées).
#[derive(Debug, Clone)]
pub struct ChallengeRow {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub state: serde_json::Value,
    pub expires_at: DateTime<Utc>,
    /// Type de client de la connexion en cours (`web` / `mobile`).
    pub client: String,
}

/// Issue d'une insertion de passkey sous plafond.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertPasskey {
    Stored,
    Duplicate,
    Plafond,
}

type CredRow = (
    Uuid,
    Uuid,
    String,
    serde_json::Value,
    Option<String>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
);

pub struct PgPasskeyRepo {
    pool: PgPool,
}

impl PgPasskeyRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Enregistre une passkey **sous plafond atomique** : un verrou consultatif
    /// par utilisateur (tenu jusqu'à la fin de la transaction) sérialise les
    /// enrôlements concurrents ; comptage et insertion partagent la transaction.
    /// `Duplicate` si l'identifiant de credential est déjà connu.
    pub async fn insert_credential_plafonne(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        passkey: &Passkey,
        name: Option<&str>,
        max: i64,
    ) -> Result<InsertPasskey, sqlx::Error> {
        let cred_id = credential_id_b64(passkey.cred_id());
        let payload = serde_json::to_value(passkey).unwrap_or(serde_json::Value::Null);
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('passkey:' || $1::uuid::text, 0))")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        let (n,): (i64,) =
            sqlx::query_as("SELECT count(*) FROM ent_passkey_credential WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?;
        if n >= max {
            return Ok(InsertPasskey::Plafond);
        }
        let res = sqlx::query(
            r#"
            INSERT INTO ent_passkey_credential (
                id, etablissement_id, user_id, cred_id, passkey, name, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6, now())
            ON CONFLICT (cred_id) DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(etablissement_id)
        .bind(user_id)
        .bind(&cred_id)
        .bind(&payload)
        .bind(name)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(if res.rows_affected() > 0 {
            InsertPasskey::Stored
        } else {
            InsertPasskey::Duplicate
        })
    }

    /// Identifiants de credential déjà enrôlés (liste d'exclusion WebAuthn).
    pub async fn list_credential_ids(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<CredentialID>, sqlx::Error> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"SELECT cred_id FROM ent_passkey_credential WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|(s,)| serde_json::from_value::<CredentialID>(serde_json::Value::String(s)).ok())
            .collect())
    }

    /// Passkeys d'une **personne**, tous établissements confondus (D-013) :
    /// le 2ᵉ facteur se décide sur le compte, jamais sur l'établissement visé
    /// (sinon un membre de deux établissements contournerait sa passkey).
    pub async fn list_passkeys(&self, user_id: Uuid) -> Result<Vec<Passkey>, sqlx::Error> {
        let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
            r#"SELECT passkey FROM ent_passkey_credential
               WHERE user_id = $1
               ORDER BY created_at"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|(v,)| serde_json::from_value::<Passkey>(v).ok())
            .collect())
    }

    /// Métadonnées (sans la matière cryptographique) pour l'écran de gestion.
    pub async fn list_rows(&self, user_id: Uuid) -> Result<Vec<PasskeyRow>, sqlx::Error> {
        let rows = self.list_cred_rows(user_id).await?;
        Ok(rows
            .into_iter()
            .map(|(id, user_id, _cred, _pk, name, created_at, last_used_at)| PasskeyRow {
                id,
                user_id,
                name,
                created_at,
                last_used_at,
            })
            .collect())
    }

    async fn list_cred_rows(&self, user_id: Uuid) -> Result<Vec<CredRow>, sqlx::Error> {
        sqlx::query_as::<_, CredRow>(
            r#"SELECT id, user_id, cred_id, passkey, name, created_at, last_used_at
               FROM ent_passkey_credential
               WHERE user_id = $1
               ORDER BY created_at"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Nombre de passkeys d'un compte (déclencheur du 2ᵉ facteur).
    pub async fn count_for_user(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let (n,): (i64,) =
            sqlx::query_as(r#"SELECT count(*) FROM ent_passkey_credential WHERE user_id = $1"#)
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(n)
    }

    /// Met à jour le compteur d'usage d'un credential (anti-clonage).
    pub async fn update_credential(
        &self,
        passkey: &Passkey,
    ) -> Result<(), sqlx::Error> {
        let cred_id = credential_id_b64(passkey.cred_id());
        let payload = serde_json::to_value(passkey).unwrap_or(serde_json::Value::Null);
        sqlx::query(
            r#"UPDATE ent_passkey_credential
               SET passkey = $2, last_used_at = now()
               WHERE cred_id = $1"#,
        )
        .bind(&cred_id)
        .bind(&payload)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Supprime une passkey d'un compte. Renvoie `false` si absente.
    pub async fn delete_credential(&self, user_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
        let res = sqlx::query(
            r#"DELETE FROM ent_passkey_credential
               WHERE id = $1 AND user_id = $2"#,
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    /// Suppression administrateur : toutes les passkeys de la personne
    /// (récupération après perte d'appareil). La passkey étant liée à la
    /// personne (D-013), une suppression partielle laisserait un 2ᵉ facteur
    /// inutilisable ou incohérent selon l'établissement.
    pub async fn delete_all_for_user(&self, user_id: Uuid) -> Result<u64, sqlx::Error> {
        let res = sqlx::query(r#"DELETE FROM ent_passkey_credential WHERE user_id = $1"#)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }

    /// Ouvre un challenge (enrôlement ou authentification).
    pub async fn insert_challenge(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        kind: &str,
        state: &serde_json::Value,
        token_hash: &str,
        expires_at: DateTime<Utc>,
        client: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO ent_passkey_challenge (
                id, token_hash, etablissement_id, user_id, kind, state, created_at, expires_at,
                client
            ) VALUES ($1,$2,$3,$4,$5,$6, now(), $7, $8)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(token_hash)
        .bind(etablissement_id)
        .bind(user_id)
        .bind(kind)
        .bind(state)
        .bind(expires_at)
        .bind(client)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Consomme un challenge **à usage unique** : la ligne est supprimée et
    /// retournée seulement si elle correspond au type attendu et n'est pas
    /// expirée. Bloque le rejeu même en cas de double soumission.
    pub async fn take_challenge(
        &self,
        token_hash: &str,
        kind: &str,
    ) -> Result<Option<ChallengeRow>, sqlx::Error> {
        let row: Option<(Uuid, Uuid, Uuid, serde_json::Value, DateTime<Utc>, String)> =
            sqlx::query_as(
            r#"DELETE FROM ent_passkey_challenge
               WHERE token_hash = $1 AND kind = $2 AND expires_at > now()
               RETURNING id, etablissement_id, user_id, state, expires_at, client"#,
        )
        .bind(token_hash)
        .bind(kind)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(id, etablissement_id, user_id, state, expires_at, client)| ChallengeRow {
                id,
                etablissement_id,
                user_id,
                state,
                expires_at,
                client,
            },
        ))
    }

    /// Purge des challenges expirés (appelée opportunément avant création).
    pub async fn prune_expired_challenges(&self) -> Result<u64, sqlx::Error> {
        let res = sqlx::query(r#"DELETE FROM ent_passkey_challenge WHERE expires_at <= now()"#)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }
}

/// Représentation base64url stable d'un identifiant de credential.
fn credential_id_b64(cred_id: &CredentialID) -> String {
    match serde_json::to_value(cred_id) {
        Ok(serde_json::Value::String(s)) => s,
        _ => String::new(),
    }
}
