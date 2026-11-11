use domain_eleve::DomainError;
use sqlx::PgPool;
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

#[derive(Debug, Clone)]
pub struct Matiere {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub source_nomenclature: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateMatiere {
    pub etablissement_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub source_nomenclature: Option<String>,
}

pub struct PgMatiereRepo {
    pool: PgPool,
}

impl PgMatiereRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, m: &Matiere) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, m.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_matiere (id, etablissement_id, code, libelle, source_nomenclature)
            VALUES ($1,$2,$3,$4,$5)
            "#,
        )
        .bind(m.id)
        .bind(m.etablissement_id)
        .bind(&m.code)
        .bind(&m.libelle)
        .bind(&m.source_nomenclature)
        .execute(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.constraint() == Some("ent_matiere_code_uq") => {
                DomainError::MatiereCodeDejaPris
            }
            _ => map_sqlx(e),
        })?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        matiere_id: Uuid,
    ) -> Result<Option<Matiere>, DomainError> {
        let row = sqlx::query_as::<_, MatiereRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, source_nomenclature
            FROM ent_matiere
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(matiere_id)
        .fetch_optional(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(MatiereRow::into_matiere))
    }

    pub async fn update_libelle(
        &self,
        etablissement_id: Uuid,
        matiere_id: Uuid,
        libelle: &str,
    ) -> Result<Option<Matiere>, DomainError> {
        let mut txw = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row = sqlx::query_as::<_, MatiereRow>(
            r#"
            UPDATE ent_matiere
            SET libelle = $3
            WHERE etablissement_id = $1 AND id = $2
            RETURNING id, etablissement_id, code, libelle, source_nomenclature
            "#,
        )
        .bind(etablissement_id)
        .bind(matiere_id)
        .bind(libelle.trim())
        .fetch_optional(&mut *txw)
        .await
        .map_err(map_sqlx)?;
        txw.commit().await.map_err(map_sqlx)?;
        Ok(row.map(MatiereRow::into_matiere))
    }

    pub async fn list(&self, etablissement_id: Uuid) -> Result<Vec<Matiere>, DomainError> {
        let rows = sqlx::query_as::<_, MatiereRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, source_nomenclature
            FROM ent_matiere
            WHERE etablissement_id = $1
            ORDER BY libelle
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(MatiereRow::into_matiere).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct MatiereRow {
    id: Uuid,
    etablissement_id: Uuid,
    code: String,
    libelle: String,
    source_nomenclature: Option<String>,
}

impl MatiereRow {
    fn into_matiere(self) -> Matiere {
        Matiere {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            libelle: self.libelle,
            source_nomenclature: self.source_nomenclature,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("matiere_repo", err)
}
