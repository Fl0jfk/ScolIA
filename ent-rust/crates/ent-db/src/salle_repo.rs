use chrono::{DateTime, Utc};
use domain_eleve::{DomainError, Salle};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgSalleRepo {
    pool: PgPool,
}

impl PgSalleRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, s: &Salle) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_salle (
                id, etablissement_id, code, libelle, capacite, site, actif,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(s.id)
        .bind(s.etablissement_id)
        .bind(&s.code)
        .bind(&s.libelle)
        .bind(s.capacite)
        .bind(&s.site)
        .bind(s.actif)
        .bind(s.created_by_user_id)
        .bind(s.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Salle>, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, code, libelle, capacite, site, actif,
                   created_by_user_id, created_at
            FROM ent_salle
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(Row::into_salle))
    }

    pub async fn get_by_code(
        &self,
        etablissement_id: Uuid,
        code: &str,
    ) -> Result<Option<Salle>, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, code, libelle, capacite, site, actif,
                   created_by_user_id, created_at
            FROM ent_salle
            WHERE etablissement_id = $1 AND code = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(Row::into_salle))
    }

    pub async fn list(
        &self,
        etablissement_id: Uuid,
        actifs_seulement: bool,
    ) -> Result<Vec<Salle>, DomainError> {
        let rows = if actifs_seulement {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, code, libelle, capacite, site, actif,
                       created_by_user_id, created_at
                FROM ent_salle
                WHERE etablissement_id = $1 AND actif = true
                ORDER BY code
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, code, libelle, capacite, site, actif,
                       created_by_user_id, created_at
                FROM ent_salle
                WHERE etablissement_id = $1
                ORDER BY code
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().map(Row::into_salle).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] salle sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    code: String,
    libelle: String,
    capacite: Option<i32>,
    site: Option<String>,
    actif: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl Row {
    fn into_salle(self) -> Salle {
        Salle {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            libelle: self.libelle,
            capacite: self.capacite,
            site: self.site,
            actif: self.actif,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}
