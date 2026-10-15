use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{AbsenceRh, DomainError};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgAbsenceRhRepo {
    pool: PgPool,
}

impl PgAbsenceRhRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, a: &AbsenceRh) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_absence_rh (
                id, etablissement_id, user_id, debut, fin, motif, statut,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(a.id)
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .bind(a.debut)
        .bind(a.fin)
        .bind(&a.motif)
        .bind(&a.statut)
        .bind(a.created_by_user_id)
        .bind(a.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn update_statut(&self, a: &AbsenceRh) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_absence_rh
            SET statut = $3, created_by_user_id = $4
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(a.etablissement_id)
        .bind(a.id)
        .bind(&a.statut)
        .bind(a.created_by_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
    ) -> Result<Option<AbsenceRh>, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, user_id, debut, fin, motif, statut,
                   created_by_user_id, created_at
            FROM ent_absence_rh
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(Row::into_absence))
    }

    pub async fn list_periode(
        &self,
        etablissement_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<AbsenceRh>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, user_id, debut, fin, motif, statut,
                   created_by_user_id, created_at
            FROM ent_absence_rh
            WHERE etablissement_id = $1
              AND debut <= $3 AND fin >= $2
            ORDER BY debut, user_id
            "#,
        )
        .bind(etablissement_id)
        .bind(debut)
        .bind(fin)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Row::into_absence).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] absence_rh sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    user_id: Uuid,
    debut: NaiveDate,
    fin: NaiveDate,
    motif: String,
    statut: String,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl Row {
    fn into_absence(self) -> AbsenceRh {
        AbsenceRh {
            id: self.id,
            etablissement_id: self.etablissement_id,
            user_id: self.user_id,
            debut: self.debut,
            fin: self.fin,
            motif: self.motif,
            statut: self.statut,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}
