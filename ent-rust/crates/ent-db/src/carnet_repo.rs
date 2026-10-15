use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{CarnetCategorie, CarnetEntree, DomainError};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgCarnetRepo {
    pool: PgPool,
}

impl PgCarnetRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, e: &CarnetEntree) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_carnet_entree (
                id, etablissement_id, eleve_id, date_entree, categorie, titre, corps,
                visible_famille, created_by_user_id, signe_at, signe_par_user_id, fil_notif_id
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            "#,
        )
        .bind(e.id)
        .bind(e.etablissement_id)
        .bind(e.eleve_id)
        .bind(e.date_entree)
        .bind(e.categorie.as_str())
        .bind(&e.titre)
        .bind(&e.corps)
        .bind(e.visible_famille)
        .bind(e.created_by_user_id)
        .bind(e.signe_at)
        .bind(e.signe_par_user_id)
        .bind(e.fil_notif_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn set_fil_notif(
        &self,
        etablissement_id: Uuid,
        entree_id: Uuid,
        fil_id: Uuid,
    ) -> Result<bool, DomainError> {
        let n = sqlx::query(
            r#"
            UPDATE ent_carnet_entree
            SET fil_notif_id = $3
            WHERE etablissement_id = $1 AND id = $2 AND fil_notif_id IS NULL
            "#,
        )
        .bind(etablissement_id)
        .bind(entree_id)
        .bind(fil_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        Ok(n > 0)
    }

    pub async fn update_signature(&self, e: &CarnetEntree) -> Result<(), DomainError> {
        let result = sqlx::query(
            r#"
            UPDATE ent_carnet_entree
            SET signe_at = $3, signe_par_user_id = $4
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(e.etablissement_id)
        .bind(e.id)
        .bind(e.signe_at)
        .bind(e.signe_par_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
    ) -> Result<Option<CarnetEntree>, DomainError> {
        let row = sqlx::query_as::<_, EntreeRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, date_entree, categorie, titre, corps,
                   visible_famille, created_by_user_id, signe_at, signe_par_user_id, fil_notif_id
            FROM ent_carnet_entree
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(EntreeRow::into_entree))
    }

    pub async fn list_for_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        famille_only: bool,
    ) -> Result<Vec<CarnetEntree>, DomainError> {
        let rows = if famille_only {
            sqlx::query_as::<_, EntreeRow>(
                r#"
                SELECT id, etablissement_id, eleve_id, date_entree, categorie, titre, corps,
                       visible_famille, created_by_user_id, signe_at, signe_par_user_id, fil_notif_id
                FROM ent_carnet_entree
                WHERE etablissement_id = $1 AND eleve_id = $2 AND visible_famille = true
                ORDER BY date_entree DESC, created_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, EntreeRow>(
                r#"
                SELECT id, etablissement_id, eleve_id, date_entree, categorie, titre, corps,
                       visible_famille, created_by_user_id, signe_at, signe_par_user_id, fil_notif_id
                FROM ent_carnet_entree
                WHERE etablissement_id = $1 AND eleve_id = $2
                ORDER BY date_entree DESC, created_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().map(EntreeRow::into_entree).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EntreeRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    date_entree: NaiveDate,
    categorie: String,
    titre: String,
    corps: String,
    visible_famille: bool,
    created_by_user_id: Option<Uuid>,
    signe_at: Option<DateTime<Utc>>,
    signe_par_user_id: Option<Uuid>,
    fil_notif_id: Option<Uuid>,
}

impl EntreeRow {
    fn into_entree(self) -> CarnetEntree {
        CarnetEntree {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            date_entree: self.date_entree,
            categorie: CarnetCategorie::parse(&self.categorie)
                .unwrap_or(CarnetCategorie::Information),
            titre: self.titre,
            corps: self.corps,
            visible_famille: self.visible_famille,
            created_by_user_id: self.created_by_user_id,
            signe_at: self.signe_at,
            signe_par_user_id: self.signe_par_user_id,
            fil_notif_id: self.fil_notif_id,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] carnet sqlx error: {err}");
    DomainError::NotFound
}
