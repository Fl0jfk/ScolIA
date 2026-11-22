use domain_eleve::DomainError;
use ent_push::{PushPlatform, PushSendStatus};
use sqlx::PgPool;
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

#[derive(Clone)]
pub struct PushDevice {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub platform: String,
    pub token: String,
    pub app_id: Option<String>,
}

#[derive(Clone)]
pub struct RegisterPushDevice {
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub platform: PushPlatform,
    pub token: String,
    pub app_id: Option<String>,
    /// Session qui inscrit le jeton (DS1) ; absente seulement pour les anciens appelants.
    pub session_id: Option<Uuid>,
}

pub struct InsertPushOutbox {
    pub etablissement_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub platform: Option<String>,
    pub token: Option<String>,
    pub status: PushSendStatus,
    pub error: Option<String>,
    pub source_aggregate: Option<String>,
    pub source_id: Option<Uuid>,
    pub message_id: Option<String>,
}

pub struct PgPushRepo {
    pool: PgPool,
}

impl PgPushRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Inscrit le jeton de notification d'un appareil, lié à la session qui l'inscrit (DS1).
    /// Un jeton déjà actif pour une autre personne ou une autre session n'est **jamais** réaffecté
    /// en silence (DS3) : l'ancienne inscription est révoquée (`reaffecte`, avec trace) et la liste
    /// des anciens titulaires est renvoyée pour l'avis de sécurité.
    pub async fn upsert_device(
        &self,
        cmd: RegisterPushDevice,
    ) -> Result<(PushDevice, Vec<Uuid>), DomainError> {
        let token = cmd.token.trim().to_string();
        if token.is_empty() || token.len() > 4096 {
            return Err(DomainError::EmptyAttributValeur);
        }
        let platform = cmd.platform.as_str();
        let empreinte = empreinte_jeton(&token);
        let mut txw = TenantTx::begin(&self.pool, cmd.etablissement_id).await.map_err(map_sqlx)?;
        let anciens: Vec<(Uuid,)> = sqlx::query_as(
            r#"UPDATE ent_push_device SET revoked_at = now(), revoque_motif = 'reaffecte'
                WHERE etablissement_id = $1 AND token = $2 AND revoked_at IS NULL
                  AND (user_id <> $3 OR session_id IS DISTINCT FROM $4)
              RETURNING user_id"#,
        )
        .bind(cmd.etablissement_id)
        .bind(&token)
        .bind(cmd.user_id)
        .bind(cmd.session_id)
        .fetch_all(&mut *txw)
        .await
        .map_err(map_sqlx)?;
        let existant = sqlx::query_as::<_, DeviceRow>(
            r#"UPDATE ent_push_device SET platform = $3, app_id = $4, last_seen_at = now()
                WHERE etablissement_id = $1 AND token = $2 AND revoked_at IS NULL
              RETURNING id, etablissement_id, user_id, platform, token, app_id"#,
        )
        .bind(cmd.etablissement_id)
        .bind(&token)
        .bind(platform)
        .bind(&cmd.app_id)
        .fetch_optional(&mut *txw)
        .await
        .map_err(map_sqlx)?;
        let row = match existant {
            Some(r) => r,
            None => sqlx::query_as::<_, DeviceRow>(
                r#"INSERT INTO ent_push_device (
                       id, etablissement_id, user_id, platform, token, app_id,
                       session_id, token_empreinte, last_seen_at, created_at, revoked_at
                   ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now(), now(), NULL)
                   RETURNING id, etablissement_id, user_id, platform, token, app_id"#,
            )
            .bind(Uuid::new_v4())
            .bind(cmd.etablissement_id)
            .bind(cmd.user_id)
            .bind(platform)
            .bind(&token)
            .bind(&cmd.app_id)
            .bind(cmd.session_id)
            .bind(&empreinte)
            .fetch_one(&mut *txw)
            .await
            .map_err(map_sqlx)?,
        };
        txw.commit().await.map_err(map_sqlx)?;
        let mut anciens: Vec<Uuid> = anciens.into_iter().map(|(u,)| u).collect();
        anciens.sort();
        anciens.dedup();
        Ok((row.into_device(), anciens))
    }

    /// DS4 : désinscription par la personne propriétaire. `false` si rien à révoquer.
    pub async fn desinscrire_device(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<bool, DomainError> {
        let mut txw = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let n = sqlx::query(
            r#"UPDATE ent_push_device SET revoked_at = now(), revoque_motif = 'desinscription'
                WHERE id = $1 AND etablissement_id = $2 AND user_id = $3 AND revoked_at IS NULL"#,
        )
        .bind(id)
        .bind(etablissement_id)
        .bind(user_id)
        .execute(&mut *txw)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        txw.commit().await.map_err(map_sqlx)?;
        Ok(n > 0)
    }

    pub async fn list_active_for_users(
        &self,
        etablissement_id: Uuid,
        user_ids: &[Uuid],
    ) -> Result<Vec<PushDevice>, DomainError> {
        if user_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows = sqlx::query_as::<_, DeviceRow>(
            r#"
            SELECT id, etablissement_id, user_id, platform, token, app_id
            FROM ent_push_device
            WHERE etablissement_id = $1
              AND user_id = ANY($2)
              AND revoked_at IS NULL
              AND (session_id IS NOT NULL OR last_seen_at > now() - interval '30 days')
            "#,
        )
        .bind(etablissement_id)
        .bind(user_ids)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(DeviceRow::into_device).collect())
    }

    pub async fn insert_outbox(
        &self,
        row: InsertPushOutbox,
    ) -> Result<Uuid, DomainError> {
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(&self.pool, row.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_push_outbox (
                id, etablissement_id, eleve_id, user_id, kind, title, body,
                platform, token, status, error, source_aggregate, source_id,
                message_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14, now())
            "#,
        )
        .bind(id)
        .bind(row.etablissement_id)
        .bind(row.eleve_id)
        .bind(row.user_id)
        .bind(&row.kind)
        .bind(&row.title)
        .bind(&row.body)
        .bind(&row.platform)
        .bind(&row.token)
        .bind(status_str(row.status))
        .bind(&row.error)
        .bind(&row.source_aggregate)
        .bind(row.source_id)
        .bind(&row.message_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(id)
    }

    pub async fn count_by_kind(
        &self,
        etablissement_id: Uuid,
        kind: &str,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_push_outbox
            WHERE etablissement_id = $1 AND kind = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(kind)
        .fetch_one(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.0)
    }

    pub async fn count_by_source_kind(
        &self,
        etablissement_id: Uuid,
        source_id: Uuid,
        kind: &str,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_push_outbox
            WHERE etablissement_id = $1 AND source_id = $2 AND kind = $3
            "#,
        )
        .bind(etablissement_id)
        .bind(source_id)
        .bind(kind)
        .fetch_one(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.0)
    }

}

/// Empreinte SHA-256 (hexadécimal) du jeton : on le retrouve sans le relire (DS3).
fn empreinte_jeton(token: &str) -> String {
    ent_auth::hash_token(token)
}

fn status_str(s: PushSendStatus) -> &'static str {
    match s {
        PushSendStatus::DryRun => "dry_run",
        PushSendStatus::Sent => "sent",
        PushSendStatus::SkippedNoConfig => "skipped_no_config",
        PushSendStatus::Failed => "failed",
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("push_repo", err)
}

#[derive(sqlx::FromRow)]
struct DeviceRow {
    id: Uuid,
    etablissement_id: Uuid,
    user_id: Uuid,
    platform: String,
    token: String,
    app_id: Option<String>,
}

impl DeviceRow {
    fn into_device(self) -> PushDevice {
        PushDevice {
            id: self.id,
            etablissement_id: self.etablissement_id,
            user_id: self.user_id,
            platform: self.platform,
            token: self.token,
            app_id: self.app_id,
        }
    }
}

impl std::fmt::Debug for PushDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PushDevice")
            .field("id", &self.id)
            .field("etablissement_id", &self.etablissement_id)
            .field("user_id", &self.user_id)
            .field("platform", &self.platform)
            .field("app_id", &self.app_id)
            .field("token", &"[masqué]")
            .finish()
    }
}
