use domain_eleve::DomainError;
use ent_sms::SmsSendStatus;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SmsOutboxRow {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub kind: String,
    pub to_phone: String,
    pub body: String,
    pub status: String,
    pub error: Option<String>,
    pub source_aggregate: Option<String>,
    pub source_id: Option<Uuid>,
    pub message_id: Option<String>,
}

pub struct InsertSmsOutbox {
    pub etablissement_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub kind: String,
    pub to_phone: String,
    pub body: String,
    pub status: SmsSendStatus,
    pub error: Option<String>,
    pub source_aggregate: Option<String>,
    pub source_id: Option<Uuid>,
    pub message_id: Option<String>,
}

pub struct PgSmsRepo {
    pool: PgPool,
}

impl PgSmsRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, row: InsertSmsOutbox) -> Result<SmsOutboxRow, DomainError> {
        let id = Uuid::new_v4();
        let status = status_str(row.status);
        sqlx::query(
            r#"
            INSERT INTO ent_sms_outbox (
                id, etablissement_id, eleve_id, kind, to_phone, body,
                status, error, source_aggregate, source_id, message_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11, now())
            "#,
        )
        .bind(id)
        .bind(row.etablissement_id)
        .bind(row.eleve_id)
        .bind(&row.kind)
        .bind(&row.to_phone)
        .bind(&row.body)
        .bind(status)
        .bind(&row.error)
        .bind(&row.source_aggregate)
        .bind(row.source_id)
        .bind(&row.message_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;

        Ok(SmsOutboxRow {
            id,
            etablissement_id: row.etablissement_id,
            eleve_id: row.eleve_id,
            kind: row.kind,
            to_phone: row.to_phone,
            body: row.body,
            status: status.to_string(),
            error: row.error,
            source_aggregate: row.source_aggregate,
            source_id: row.source_id,
            message_id: row.message_id,
        })
    }

    pub async fn count_by_kind(
        &self,
        etablissement_id: Uuid,
        kind: &str,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_sms_outbox
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
            SELECT COUNT(*)::bigint FROM ent_sms_outbox
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

    pub async fn list_recent(
        &self,
        etablissement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<SmsOutboxRow>, DomainError> {
        let rows = sqlx::query_as::<_, SmsOutboxSql>(
            r#"
            SELECT id, etablissement_id, eleve_id, kind, to_phone, body,
                   status, error, source_aggregate, source_id, message_id
            FROM ent_sms_outbox
            WHERE etablissement_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(etablissement_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(SmsOutboxSql::into_row).collect())
    }
}

#[derive(sqlx::FromRow)]
struct SmsOutboxSql {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Option<Uuid>,
    kind: String,
    to_phone: String,
    body: String,
    status: String,
    error: Option<String>,
    source_aggregate: Option<String>,
    source_id: Option<Uuid>,
    message_id: Option<String>,
}

impl SmsOutboxSql {
    fn into_row(self) -> SmsOutboxRow {
        SmsOutboxRow {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            kind: self.kind,
            to_phone: self.to_phone,
            body: self.body,
            status: self.status,
            error: self.error,
            source_aggregate: self.source_aggregate,
            source_id: self.source_id,
            message_id: self.message_id,
        }
    }
}

fn status_str(s: SmsSendStatus) -> &'static str {
    match s {
        SmsSendStatus::DryRun => "dry_run",
        SmsSendStatus::Sent => "sent",
        SmsSendStatus::SkippedNoConfig => "skipped_no_config",
        SmsSendStatus::Failed => "failed",
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("sms_repo", err)
}
