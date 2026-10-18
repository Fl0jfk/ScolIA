//! Persistance des retards (`ent_retard`).
use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{DomainError, Retard};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgRetardRepo {
    pool: PgPool,
}

impl PgRetardRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insère un retard. Idempotent pour un couple (appel, élève) : rejouer la
    /// clôture d'un appel ne crée pas de doublon.
    pub async fn insert(&self, r: &Retard) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_retard (
                id, etablissement_id, eleve_id, jour, minutes, motif,
                source_appel_id, auteur_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            ON CONFLICT (source_appel_id, eleve_id) WHERE source_appel_id IS NOT NULL
            DO NOTHING
            "#,
        )
        .bind(r.id)
        .bind(r.etablissement_id)
        .bind(r.eleve_id)
        .bind(r.jour)
        .bind(r.minutes)
        .bind(&r.motif)
        .bind(r.source_appel_id)
        .bind(r.auteur_user_id)
        .bind(r.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    /// Historique des retards d'un élève, les plus récents d'abord.
    pub async fn list_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Retard>, DomainError> {
        let rows = sqlx::query_as::<_, RetardSql>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, minutes, motif,
                   source_appel_id, auteur_user_id, created_at
            FROM ent_retard
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY jour DESC, created_at DESC
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(RetardSql::into_retard).collect())
    }

    /// Nombre de retards d'un élève depuis une date (compteur VS).
    pub async fn count_eleve_depuis(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        depuis: NaiveDate,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_retard
            WHERE etablissement_id = $1 AND eleve_id = $2 AND jour >= $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(depuis)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.0)
    }
}

#[derive(sqlx::FromRow)]
struct RetardSql {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    minutes: Option<i32>,
    motif: Option<String>,
    source_appel_id: Option<Uuid>,
    auteur_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

impl RetardSql {
    fn into_retard(self) -> Retard {
        Retard {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            jour: self.jour,
            minutes: self.minutes,
            motif: self.motif,
            source_appel_id: self.source_appel_id,
            auteur_user_id: self.auteur_user_id,
            created_at: self.created_at,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("retard_repo", err)
}
