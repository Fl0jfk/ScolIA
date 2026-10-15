use chrono::{DateTime, Utc};
use domain_eleve::{DomainError, JournalEntree};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgJournalRepo {
    pool: PgPool,
}

impl PgJournalRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, e: &JournalEntree) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_journal_modif (
                id, etablissement_id, eleve_id, domaine, action, libelle,
                payload, visible_famille, actor_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(e.id)
        .bind(e.etablissement_id)
        .bind(e.eleve_id)
        .bind(&e.domaine)
        .bind(&e.action)
        .bind(&e.libelle)
        .bind(&e.payload)
        .bind(e.visible_famille)
        .bind(e.actor_user_id)
        .bind(e.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        famille_seulement: bool,
        limit: i64,
    ) -> Result<Vec<JournalEntree>, DomainError> {
        let rows = if famille_seulement {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, eleve_id, domaine, action, libelle,
                       payload, visible_famille, actor_user_id, created_at
                FROM ent_journal_modif
                WHERE etablissement_id = $1 AND eleve_id = $2 AND visible_famille = true
                ORDER BY created_at DESC
                LIMIT $3
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, eleve_id, domaine, action, libelle,
                       payload, visible_famille, actor_user_id, created_at
                FROM ent_journal_modif
                WHERE etablissement_id = $1 AND eleve_id = $2
                ORDER BY created_at DESC
                LIMIT $3
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().map(Row::into_entree).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] journal sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Option<Uuid>,
    domaine: String,
    action: String,
    libelle: String,
    payload: serde_json::Value,
    visible_famille: bool,
    actor_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

impl Row {
    fn into_entree(self) -> JournalEntree {
        JournalEntree {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            domaine: self.domaine,
            action: self.action,
            libelle: self.libelle,
            payload: self.payload,
            visible_famille: self.visible_famille,
            actor_user_id: self.actor_user_id,
            created_at: self.created_at,
        }
    }
}
