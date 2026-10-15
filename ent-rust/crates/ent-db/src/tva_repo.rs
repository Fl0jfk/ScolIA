use chrono::{DateTime, Utc};
use domain_eleve::{DomainError, TauxTva};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgTvaRepo {
    pool: PgPool,
}

impl PgTvaRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, t: &TauxTva) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_taux_tva (
                id, etablissement_id, code, libelle, taux_bps, actif,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(t.id)
        .bind(t.etablissement_id)
        .bind(&t.code)
        .bind(&t.libelle)
        .bind(t.taux_bps)
        .bind(t.actif)
        .bind(t.created_by_user_id)
        .bind(t.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list(
        &self,
        etablissement_id: Uuid,
        actifs_seulement: bool,
    ) -> Result<Vec<TauxTva>, DomainError> {
        let rows = if actifs_seulement {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, code, libelle, taux_bps, actif,
                       created_by_user_id, created_at
                FROM ent_taux_tva
                WHERE etablissement_id = $1 AND actif = true
                ORDER BY taux_bps, code
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, code, libelle, taux_bps, actif,
                       created_by_user_id, created_at
                FROM ent_taux_tva
                WHERE etablissement_id = $1
                ORDER BY taux_bps, code
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().map(Row::into_taux).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] tva sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    code: String,
    libelle: String,
    taux_bps: i32,
    actif: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl Row {
    fn into_taux(self) -> TauxTva {
        TauxTva {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            libelle: self.libelle,
            taux_bps: self.taux_bps,
            actif: self.actif,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}
