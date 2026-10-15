//! Rapprochement relevés bancaires ↔ prélèvements SEPA.

use chrono::{DateTime, Utc};
use domain_eleve::DomainError;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BankRapprochementRow {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub end_to_end_id: String,
    pub montant_centimes: i64,
    pub result: String,
    pub motif: Option<String>,
    pub prelevement_id: Option<Uuid>,
    pub action: String,
    pub status: String,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct BankRepo {
    pool: PgPool,
}

impl BankRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_rapprochement(
        &self,
        row: &BankRapprochementRow,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_bank_rapprochement (
                id, etablissement_id, end_to_end_id, montant_centimes,
                result, motif, prelevement_id, action, status, error, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(row.id)
        .bind(row.etablissement_id)
        .bind(&row.end_to_end_id)
        .bind(row.montant_centimes)
        .bind(&row.result)
        .bind(&row.motif)
        .bind(row.prelevement_id)
        .bind(&row.action)
        .bind(&row.status)
        .bind(&row.error)
        .bind(row.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn count_rapprochements(
        &self,
        etablissement_id: Uuid,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_bank_rapprochement
            WHERE etablissement_id = $1
            "#,
        )
        .bind(etablissement_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.0)
    }

    pub async fn count_by_status(
        &self,
        etablissement_id: Uuid,
        status: &str,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_bank_rapprochement
            WHERE etablissement_id = $1 AND status = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.0)
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] bank sqlx error: {err}");
    DomainError::NotFound
}
