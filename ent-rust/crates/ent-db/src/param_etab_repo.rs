use chrono::{DateTime, Utc};
use domain_eleve::{DomainError, ParamEtab};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgParamEtabRepo {
    pool: PgPool,
}

impl PgParamEtabRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, p: &ParamEtab) -> Result<ParamEtab, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            INSERT INTO ent_param_etab (
                id, etablissement_id, cle, valeur, updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6)
            ON CONFLICT (etablissement_id, cle) DO UPDATE SET
                valeur = EXCLUDED.valeur,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at,
                id = ent_param_etab.id
            RETURNING id, etablissement_id, cle, valeur, updated_by_user_id, updated_at
            "#,
        )
        .bind(p.id)
        .bind(p.etablissement_id)
        .bind(&p.cle)
        .bind(&p.valeur)
        .bind(p.updated_by_user_id)
        .bind(p.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.into_param())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        cle: &str,
    ) -> Result<Option<ParamEtab>, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, cle, valeur, updated_by_user_id, updated_at
            FROM ent_param_etab
            WHERE etablissement_id = $1 AND cle = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(cle)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(Row::into_param))
    }

    pub async fn list(&self, etablissement_id: Uuid) -> Result<Vec<ParamEtab>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, cle, valeur, updated_by_user_id, updated_at
            FROM ent_param_etab
            WHERE etablissement_id = $1
            ORDER BY cle
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Row::into_param).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] param_etab sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    cle: String,
    valeur: String,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl Row {
    fn into_param(self) -> ParamEtab {
        ParamEtab {
            id: self.id,
            etablissement_id: self.etablissement_id,
            cle: self.cle,
            valeur: self.valeur,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        }
    }
}
