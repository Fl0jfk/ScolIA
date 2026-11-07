use domain_eleve::DomainError;
use ent_mail::MailSendStatus;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MailOutboxRow {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub kind: String,
    pub to_email: String,
    pub subject: String,
    pub body_text: String,
    pub status: String,
    pub error: Option<String>,
    pub source_aggregate: Option<String>,
    pub source_id: Option<Uuid>,
    pub message_id: Option<String>,
}

pub struct InsertMailOutbox {
    pub etablissement_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub kind: String,
    pub to_email: String,
    pub subject: String,
    pub body_text: String,
    pub status: MailSendStatus,
    pub error: Option<String>,
    pub source_aggregate: Option<String>,
    pub source_id: Option<Uuid>,
    pub message_id: Option<String>,
}

pub struct PgMailRepo {
    pool: PgPool,
}

impl PgMailRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, row: InsertMailOutbox) -> Result<MailOutboxRow, DomainError> {
        let id = Uuid::new_v4();
        let status = status_str(row.status);
        sqlx::query(
            r#"
            INSERT INTO ent_mail_outbox (
                id, etablissement_id, eleve_id, kind, to_email, subject, body_text,
                status, error, source_aggregate, source_id, message_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12, now())
            "#,
        )
        .bind(id)
        .bind(row.etablissement_id)
        .bind(row.eleve_id)
        .bind(&row.kind)
        .bind(&row.to_email)
        .bind(&row.subject)
        .bind(&row.body_text)
        .bind(status)
        .bind(&row.error)
        .bind(&row.source_aggregate)
        .bind(row.source_id)
        .bind(&row.message_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;

        Ok(MailOutboxRow {
            id,
            etablissement_id: row.etablissement_id,
            eleve_id: row.eleve_id,
            kind: row.kind,
            to_email: row.to_email,
            subject: row.subject,
            body_text: row.body_text,
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
            SELECT COUNT(*)::bigint FROM ent_mail_outbox
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
            SELECT COUNT(*)::bigint FROM ent_mail_outbox
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

    /// Anti-doublon **par destinataire** (élève) — pour les diffusions
    /// (annonce établissement → plusieurs familles partageant la même source).
    pub async fn count_by_source_kind_eleve(
        &self,
        etablissement_id: Uuid,
        source_id: Uuid,
        kind: &str,
        eleve_id: Uuid,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_mail_outbox
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

    pub async fn list_recent(
        &self,
        etablissement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<MailOutboxRow>, DomainError> {
        let rows = sqlx::query_as::<_, MailOutboxSql>(
            r#"
            SELECT id, etablissement_id, eleve_id, kind, to_email, subject, body_text,
                   status, error, source_aggregate, source_id, message_id
            FROM ent_mail_outbox
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
        Ok(rows.into_iter().map(MailOutboxSql::into_row).collect())
    }
}

fn status_str(s: MailSendStatus) -> &'static str {
    match s {
        MailSendStatus::DryRun => "dry_run",
        MailSendStatus::Sent => "sent",
        MailSendStatus::SkippedNoConfig => "skipped_no_config",
        MailSendStatus::Failed => "failed",
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("mail_repo", err)
}

#[derive(sqlx::FromRow)]
struct MailOutboxSql {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Option<Uuid>,
    kind: String,
    to_email: String,
    subject: String,
    body_text: String,
    status: String,
    error: Option<String>,
    source_aggregate: Option<String>,
    source_id: Option<Uuid>,
    message_id: Option<String>,
}

impl MailOutboxSql {
    fn into_row(self) -> MailOutboxRow {
        MailOutboxRow {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            kind: self.kind,
            to_email: self.to_email,
            subject: self.subject,
            body_text: self.body_text,
            status: self.status,
            error: self.error,
            source_aggregate: self.source_aggregate,
            source_id: self.source_id,
            message_id: self.message_id,
        }
    }
}
