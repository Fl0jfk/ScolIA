use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{DomainError, PeriodeVacances, VacancesKind};
use sqlx::PgPool;
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

pub struct PgVacancesRepo {
    pool: PgPool,
}

impl PgVacancesRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, p: &PeriodeVacances) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, p.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_periode_vacances (
                id, etablissement_id, libelle, kind, debut, fin,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(p.id)
        .bind(p.etablissement_id)
        .bind(&p.libelle)
        .bind(p.kind.as_str())
        .bind(p.debut)
        .bind(p.fin)
        .bind(p.created_by_user_id)
        .bind(p.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<PeriodeVacances>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, libelle, kind, debut, fin,
                   created_by_user_id, created_at
            FROM ent_periode_vacances
            WHERE etablissement_id = $1
            ORDER BY debut
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(Row::into_periode).collect())
    }

    pub async fn list_couvrant(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<PeriodeVacances>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, libelle, kind, debut, fin,
                   created_by_user_id, created_at
            FROM ent_periode_vacances
            WHERE etablissement_id = $1 AND debut <= $2 AND fin >= $2
            ORDER BY debut
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(Row::into_periode).collect())
    }

    pub async fn delete(
        &self,
        etablissement_id: Uuid,
        periode_id: Uuid,
    ) -> Result<bool, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let r = sqlx::query(
            r#"
            DELETE FROM ent_periode_vacances
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(periode_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(r.rows_affected() > 0)
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("vacances", e)
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    libelle: String,
    kind: String,
    debut: NaiveDate,
    fin: NaiveDate,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl Row {
    fn into_periode(self) -> Option<PeriodeVacances> {
        Some(PeriodeVacances {
            id: self.id,
            etablissement_id: self.etablissement_id,
            libelle: self.libelle,
            kind: VacancesKind::parse(&self.kind)?,
            debut: self.debut,
            fin: self.fin,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}
