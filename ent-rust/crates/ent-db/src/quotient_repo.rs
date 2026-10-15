use chrono::{DateTime, Utc};
use domain_eleve::{DomainError, QuotientFoyer};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgQuotientRepo {
    pool: PgPool,
}

impl PgQuotientRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, q: &QuotientFoyer) -> Result<QuotientFoyer, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            INSERT INTO ent_foyer_quotient (
                id, etablissement_id, foyer_id, annee_scolaire,
                quotient_centimes, parts_x100, updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            ON CONFLICT (etablissement_id, foyer_id, annee_scolaire) DO UPDATE SET
                quotient_centimes = EXCLUDED.quotient_centimes,
                parts_x100 = EXCLUDED.parts_x100,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at,
                id = ent_foyer_quotient.id
            RETURNING id, etablissement_id, foyer_id, annee_scolaire,
                      quotient_centimes, parts_x100, updated_by_user_id, updated_at
            "#,
        )
        .bind(q.id)
        .bind(q.etablissement_id)
        .bind(q.foyer_id)
        .bind(&q.annee_scolaire)
        .bind(q.quotient_centimes)
        .bind(q.parts_x100)
        .bind(q.updated_by_user_id)
        .bind(q.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.into_quotient())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        foyer_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<Option<QuotientFoyer>, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, foyer_id, annee_scolaire,
                   quotient_centimes, parts_x100, updated_by_user_id, updated_at
            FROM ent_foyer_quotient
            WHERE etablissement_id = $1 AND foyer_id = $2 AND annee_scolaire = $3
            "#,
        )
        .bind(etablissement_id)
        .bind(foyer_id)
        .bind(annee_scolaire)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(Row::into_quotient))
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] quotient sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    foyer_id: Uuid,
    annee_scolaire: String,
    quotient_centimes: i64,
    parts_x100: i32,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl Row {
    fn into_quotient(self) -> QuotientFoyer {
        QuotientFoyer {
            id: self.id,
            etablissement_id: self.etablissement_id,
            foyer_id: self.foyer_id,
            annee_scolaire: self.annee_scolaire,
            quotient_centimes: self.quotient_centimes,
            parts_x100: self.parts_x100,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        }
    }
}
