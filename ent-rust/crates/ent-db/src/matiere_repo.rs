use domain_eleve::DomainError;
use sqlx::PgPool;
use uuid::Uuid;

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
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
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
        .fetch_optional(&self.pool)
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
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
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
        .fetch_all(&self.pool)
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
    eprintln!("[ent-db] matiere sqlx error: {err}");
    DomainError::NotFound
}
