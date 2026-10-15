use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{DomainError, MenuCantineJour};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgMenuCantineRepo {
    pool: PgPool,
}

impl PgMenuCantineRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, m: &MenuCantineJour) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_menu_cantine (
                id, etablissement_id, jour, entree, plat, dessert, notes,
                publie, updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            ON CONFLICT (etablissement_id, jour) DO UPDATE SET
                entree = EXCLUDED.entree,
                plat = EXCLUDED.plat,
                dessert = EXCLUDED.dessert,
                notes = EXCLUDED.notes,
                publie = EXCLUDED.publie,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at,
                id = ent_menu_cantine.id
            "#,
        )
        .bind(m.id)
        .bind(m.etablissement_id)
        .bind(m.jour)
        .bind(&m.entree)
        .bind(&m.plat)
        .bind(&m.dessert)
        .bind(&m.notes)
        .bind(m.publie)
        .bind(m.updated_by_user_id)
        .bind(m.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_periode(
        &self,
        etablissement_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
        publies_seulement: bool,
    ) -> Result<Vec<MenuCantineJour>, DomainError> {
        let rows = if publies_seulement {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, jour, entree, plat, dessert, notes,
                       publie, updated_by_user_id, updated_at
                FROM ent_menu_cantine
                WHERE etablissement_id = $1 AND jour BETWEEN $2 AND $3 AND publie = true
                ORDER BY jour
                "#,
            )
            .bind(etablissement_id)
            .bind(debut)
            .bind(fin)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, jour, entree, plat, dessert, notes,
                       publie, updated_by_user_id, updated_at
                FROM ent_menu_cantine
                WHERE etablissement_id = $1 AND jour BETWEEN $2 AND $3
                ORDER BY jour
                "#,
            )
            .bind(etablissement_id)
            .bind(debut)
            .bind(fin)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().map(Row::into_menu).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] menu_cantine sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    jour: NaiveDate,
    entree: String,
    plat: String,
    dessert: String,
    notes: String,
    publie: bool,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl Row {
    fn into_menu(self) -> MenuCantineJour {
        MenuCantineJour {
            id: self.id,
            etablissement_id: self.etablissement_id,
            jour: self.jour,
            entree: self.entree,
            plat: self.plat,
            dessert: self.dessert,
            notes: self.notes,
            publie: self.publie,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        }
    }
}
