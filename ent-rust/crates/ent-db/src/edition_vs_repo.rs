use domain_eleve::{DomainError, EditionVsTemplate};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgEditionVsRepo {
    pool: PgPool,
}

impl PgEditionVsRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, t: &EditionVsTemplate) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_edition_vs (
                id, etablissement_id, code, titre, corps,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7)
            ON CONFLICT (etablissement_id, code) DO UPDATE SET
                id = EXCLUDED.id,
                titre = EXCLUDED.titre,
                corps = EXCLUDED.corps,
                created_by_user_id = EXCLUDED.created_by_user_id,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(t.id)
        .bind(t.etablissement_id)
        .bind(&t.code)
        .bind(&t.titre)
        .bind(&t.corps)
        .bind(t.created_by_user_id)
        .bind(t.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
    ) -> Result<Option<EditionVsTemplate>, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, code, titre, corps,
                   created_by_user_id, created_at
            FROM ent_edition_vs
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(Row::into_template))
    }

    pub async fn get_by_code(
        &self,
        etablissement_id: Uuid,
        code: &str,
    ) -> Result<Option<EditionVsTemplate>, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, code, titre, corps,
                   created_by_user_id, created_at
            FROM ent_edition_vs
            WHERE etablissement_id = $1 AND code = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(Row::into_template))
    }

    pub async fn list(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<EditionVsTemplate>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, code, titre, corps,
                   created_by_user_id, created_at
            FROM ent_edition_vs
            WHERE etablissement_id = $1
            ORDER BY code
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Row::into_template).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] edition_vs sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    code: String,
    titre: String,
    corps: String,
    created_by_user_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl Row {
    fn into_template(self) -> EditionVsTemplate {
        EditionVsTemplate {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            titre: self.titre,
            corps: self.corps,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}
