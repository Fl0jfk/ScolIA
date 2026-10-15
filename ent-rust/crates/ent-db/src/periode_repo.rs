use domain_eleve::{AttributKind, CutoverOp, DomainError, ElevePeriode};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgPeriodeRepo {
    pool: PgPool,
}

impl PgPeriodeRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn open(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        kind: AttributKind,
    ) -> Result<Option<ElevePeriode>, DomainError> {
        let row = sqlx::query_as::<_, PeriodeRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, kind, valeur, date_debut, date_fin
            FROM ent_eleve_periode
            WHERE etablissement_id = $1
              AND eleve_id = $2
              AND kind = $3
              AND date_fin IS NULL
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(kind.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(PeriodeRow::into_periode))
    }

    pub async fn list(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<ElevePeriode>, DomainError> {
        let rows = sqlx::query_as::<_, PeriodeRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, kind, valeur, date_debut, date_fin
            FROM ent_eleve_periode
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY kind, date_debut
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(PeriodeRow::into_periode).collect())
    }

    pub async fn apply_cutover(&self, op: &CutoverOp) -> Result<(), DomainError> {
        match op {
            CutoverOp::Noop => Ok(()),
            CutoverOp::ReplaceOpen {
                period_id,
                nouvelle_valeur,
            } => {
                let result = sqlx::query(
                    r#"
                    UPDATE ent_eleve_periode
                    SET valeur = $2
                    WHERE id = $1 AND date_fin IS NULL
                    "#,
                )
                .bind(period_id)
                .bind(nouvelle_valeur)
                .execute(&self.pool)
                .await
                .map_err(map_sqlx)?;
                if result.rows_affected() == 0 {
                    return Err(DomainError::NotFound);
                }
                Ok(())
            }
            CutoverOp::CloseAndOpen {
                close_id,
                close_date_fin,
                open,
            } => {
                let mut tx = self.pool.begin().await.map_err(map_sqlx)?;
                let closed = sqlx::query(
                    r#"
                    UPDATE ent_eleve_periode
                    SET date_fin = $2
                    WHERE id = $1
                      AND etablissement_id = $3
                      AND eleve_id = $4
                      AND date_fin IS NULL
                    "#,
                )
                .bind(close_id)
                .bind(close_date_fin)
                .bind(open.etablissement_id)
                .bind(open.eleve_id)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx)?;
                if closed.rows_affected() == 0 {
                    return Err(DomainError::NotFound);
                }
                sqlx::query(
                    r#"
                    INSERT INTO ent_eleve_periode (
                        id, etablissement_id, eleve_id, kind, valeur, date_debut, date_fin
                    ) VALUES ($1,$2,$3,$4,$5,$6,$7)
                    "#,
                )
                .bind(open.id)
                .bind(open.etablissement_id)
                .bind(open.eleve_id)
                .bind(open.kind.as_str())
                .bind(&open.valeur)
                .bind(open.date_debut)
                .bind(open.date_fin)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx)?;
                tx.commit().await.map_err(map_sqlx)?;
                Ok(())
            }
            CutoverOp::OpenFirst { period } => {
                sqlx::query(
                    r#"
                    INSERT INTO ent_eleve_periode (
                        id, etablissement_id, eleve_id, kind, valeur, date_debut, date_fin
                    ) VALUES ($1,$2,$3,$4,$5,$6,$7)
                    "#,
                )
                .bind(period.id)
                .bind(period.etablissement_id)
                .bind(period.eleve_id)
                .bind(period.kind.as_str())
                .bind(&period.valeur)
                .bind(period.date_debut)
                .bind(period.date_fin)
                .execute(&self.pool)
                .await
                .map_err(map_sqlx)?;
                Ok(())
            }
        }
    }

    pub async fn close_open_periodes(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        date_fin: chrono::NaiveDate,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_eleve_periode
            SET date_fin = CASE
                WHEN $3 < date_debut THEN date_debut
                ELSE $3
            END
            WHERE etablissement_id = $1
              AND eleve_id = $2
              AND date_fin IS NULL
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(date_fin)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PeriodeRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    kind: String,
    valeur: String,
    date_debut: chrono::NaiveDate,
    date_fin: Option<chrono::NaiveDate>,
}

impl PeriodeRow {
    fn into_periode(self) -> ElevePeriode {
        ElevePeriode {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            kind: AttributKind::parse(&self.kind).unwrap_or(AttributKind::Classe),
            valeur: self.valeur,
            date_debut: self.date_debut,
            date_fin: self.date_fin,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] periode sqlx error: {err}");
    DomainError::NotFound
}
