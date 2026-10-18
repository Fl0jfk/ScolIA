use domain_eleve::DomainError;
use ent_push::{PushPlatform, PushSendStatus};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PushDevice {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub platform: String,
    pub token: String,
    pub app_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RegisterPushDevice {
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub platform: PushPlatform,
    pub token: String,
    pub app_id: Option<String>,
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

    pub async fn upsert_device(
        &self,
        cmd: RegisterPushDevice,
    ) -> Result<PushDevice, DomainError> {
        let token = cmd.token.trim().to_string();
        if token.is_empty() {
            return Err(DomainError::EmptyAttributValeur);
        }
        let id = Uuid::new_v4();
        let platform = cmd.platform.as_str();
        let row = sqlx::query_as::<_, DeviceRow>(
            r#"
            INSERT INTO ent_push_device (
                id, etablissement_id, user_id, platform, token, app_id,
                last_seen_at, created_at, revoked_at
            ) VALUES ($1,$2,$3,$4,$5,$6, now(), now(), NULL)
            ON CONFLICT (etablissement_id, token) DO UPDATE SET
                user_id = EXCLUDED.user_id,
                platform = EXCLUDED.platform,
                app_id = EXCLUDED.app_id,
                last_seen_at = now(),
                revoked_at = NULL
            RETURNING id, etablissement_id, user_id, platform, token, app_id
            "#,
        )
        .bind(id)
        .bind(cmd.etablissement_id)
        .bind(cmd.user_id)
        .bind(platform)
        .bind(&token)
        .bind(&cmd.app_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.into_device())
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
            "#,
        )
        .bind(etablissement_id)
        .bind(user_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(DeviceRow::into_device).collect())
    }

    pub async fn insert_outbox(
        &self,
        row: InsertPushOutbox,
    ) -> Result<Uuid, DomainError> {
        let id = Uuid::new_v4();
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
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
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
        .fetch_one(&self.pool)
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
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.0)
    }

    /// Anti-doublon **par destinataire** (élève) — diffusions multi-familles.
    pub async fn count_by_source_kind_eleve(
        &self,
        etablissement_id: Uuid,
        source_id: Uuid,
        kind: &str,
        eleve_id: Uuid,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_push_outbox
            WHERE etablissement_id = $1 AND source_id = $2 AND kind = $3 AND eleve_id = $4
            "#,
        )
        .bind(etablissement_id)
        .bind(source_id)
        .bind(kind)
        .bind(eleve_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.0)
    }
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
