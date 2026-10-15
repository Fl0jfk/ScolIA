use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{DispenseVs, DispenseVsMotif, DispenseVsStatut, DomainError};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgDispenseVsRepo {
    pool: PgPool,
}

impl PgDispenseVsRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, d: &DispenseVs) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_dispense_vs (
                id, etablissement_id, eleve_id, motif, debut, fin, commentaire,
                statut, created_by_user_id, created_at, closed_by_user_id, closed_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            "#,
        )
        .bind(d.id)
        .bind(d.etablissement_id)
        .bind(d.eleve_id)
        .bind(d.motif.as_str())
        .bind(d.debut)
        .bind(d.fin)
        .bind(&d.commentaire)
        .bind(d.statut.as_str())
        .bind(d.created_by_user_id)
        .bind(d.created_at)
        .bind(d.closed_by_user_id)
        .bind(d.closed_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn update_cloture(&self, d: &DispenseVs) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_dispense_vs
            SET statut = $3, closed_by_user_id = $4, closed_at = $5
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(d.etablissement_id)
        .bind(d.id)
        .bind(d.statut.as_str())
        .bind(d.closed_by_user_id)
        .bind(d.closed_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        dispense_id: Uuid,
    ) -> Result<Option<DispenseVs>, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, eleve_id, motif, debut, fin, commentaire,
                   statut, created_by_user_id, created_at, closed_by_user_id, closed_at
            FROM ent_dispense_vs
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(dispense_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(Row::into_dispense))
    }

    pub async fn list_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        actives_seulement: bool,
    ) -> Result<Vec<DispenseVs>, DomainError> {
        let rows = if actives_seulement {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, eleve_id, motif, debut, fin, commentaire,
                       statut, created_by_user_id, created_at, closed_by_user_id, closed_at
                FROM ent_dispense_vs
                WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'active'
                ORDER BY debut DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, eleve_id, motif, debut, fin, commentaire,
                       statut, created_by_user_id, created_at, closed_by_user_id, closed_at
                FROM ent_dispense_vs
                WHERE etablissement_id = $1 AND eleve_id = $2
                ORDER BY debut DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().filter_map(Row::into_dispense).collect())
    }

    pub async fn list_actives_au_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<DispenseVs>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, eleve_id, motif, debut, fin, commentaire,
                   statut, created_by_user_id, created_at, closed_by_user_id, closed_at
            FROM ent_dispense_vs
            WHERE etablissement_id = $1 AND statut = 'active'
              AND debut <= $2 AND fin >= $2
            ORDER BY debut
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(Row::into_dispense).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] dispense_vs sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    motif: String,
    debut: NaiveDate,
    fin: NaiveDate,
    commentaire: String,
    statut: String,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
    closed_by_user_id: Option<Uuid>,
    closed_at: Option<DateTime<Utc>>,
}

impl Row {
    fn into_dispense(self) -> Option<DispenseVs> {
        Some(DispenseVs {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            motif: DispenseVsMotif::parse(&self.motif)?,
            debut: self.debut,
            fin: self.fin,
            commentaire: self.commentaire,
            statut: DispenseVsStatut::parse(&self.statut)?,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
            closed_by_user_id: self.closed_by_user_id,
            closed_at: self.closed_at,
        })
    }
}
