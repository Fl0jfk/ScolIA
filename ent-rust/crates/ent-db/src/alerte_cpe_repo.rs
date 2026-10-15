use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{AlerteCpe, AlerteCpeSeverite, AlerteCpeStatut, DomainError};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgAlerteCpeRepo {
    pool: PgPool,
}

impl PgAlerteCpeRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, a: &AlerteCpe) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_alerte_cpe (
                id, etablissement_id, eleve_id, titre, detail, severite, statut,
                visible_famille, jour, created_by_user_id, created_at,
                closed_by_user_id, closed_at, cloture_note
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
            "#,
        )
        .bind(a.id)
        .bind(a.etablissement_id)
        .bind(a.eleve_id)
        .bind(&a.titre)
        .bind(&a.detail)
        .bind(a.severite.as_str())
        .bind(a.statut.as_str())
        .bind(a.visible_famille)
        .bind(a.jour)
        .bind(a.created_by_user_id)
        .bind(a.created_at)
        .bind(a.closed_by_user_id)
        .bind(a.closed_at)
        .bind(&a.cloture_note)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn update_cloture(&self, a: &AlerteCpe) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_alerte_cpe
            SET statut = $3, closed_by_user_id = $4, closed_at = $5, cloture_note = $6
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(a.etablissement_id)
        .bind(a.id)
        .bind(a.statut.as_str())
        .bind(a.closed_by_user_id)
        .bind(a.closed_at)
        .bind(&a.cloture_note)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        alerte_id: Uuid,
    ) -> Result<Option<AlerteCpe>, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, eleve_id, titre, detail, severite, statut,
                   visible_famille, jour, created_by_user_id, created_at,
                   closed_by_user_id, closed_at, cloture_note
            FROM ent_alerte_cpe
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(alerte_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(Row::into_alerte))
    }

    pub async fn list_ouvertes(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<AlerteCpe>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, eleve_id, titre, detail, severite, statut,
                   visible_famille, jour, created_by_user_id, created_at,
                   closed_by_user_id, closed_at, cloture_note
            FROM ent_alerte_cpe
            WHERE etablissement_id = $1 AND statut = 'ouverte'
            ORDER BY
                CASE severite WHEN 'urgent' THEN 0 WHEN 'attention' THEN 1 ELSE 2 END,
                jour DESC
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(Row::into_alerte).collect())
    }

    pub async fn list_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        famille_seulement: bool,
    ) -> Result<Vec<AlerteCpe>, DomainError> {
        let rows = if famille_seulement {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, eleve_id, titre, detail, severite, statut,
                       visible_famille, jour, created_by_user_id, created_at,
                       closed_by_user_id, closed_at, cloture_note
                FROM ent_alerte_cpe
                WHERE etablissement_id = $1 AND eleve_id = $2 AND visible_famille = true
                ORDER BY created_at DESC
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
                SELECT id, etablissement_id, eleve_id, titre, detail, severite, statut,
                       visible_famille, jour, created_by_user_id, created_at,
                       closed_by_user_id, closed_at, cloture_note
                FROM ent_alerte_cpe
                WHERE etablissement_id = $1 AND eleve_id = $2
                ORDER BY created_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().filter_map(Row::into_alerte).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] alerte_cpe sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    titre: String,
    detail: String,
    severite: String,
    statut: String,
    visible_famille: bool,
    jour: NaiveDate,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
    closed_by_user_id: Option<Uuid>,
    closed_at: Option<DateTime<Utc>>,
    cloture_note: String,
}

impl Row {
    fn into_alerte(self) -> Option<AlerteCpe> {
        Some(AlerteCpe {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            titre: self.titre,
            detail: self.detail,
            severite: AlerteCpeSeverite::parse(&self.severite)?,
            statut: AlerteCpeStatut::parse(&self.statut)?,
            visible_famille: self.visible_famille,
            jour: self.jour,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
            closed_by_user_id: self.closed_by_user_id,
            closed_at: self.closed_at,
            cloture_note: self.cloture_note,
        })
    }
}
