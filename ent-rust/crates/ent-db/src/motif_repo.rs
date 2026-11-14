use domain_eleve::DomainError;
use sqlx::PgPool;
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsenceMotif {
    pub code: String,
    pub libelle: String,
    /// `None` = référentiel global.
    pub etablissement_id: Option<Uuid>,
}

pub struct PgMotifRepo {
    pool: PgPool,
}

impl PgMotifRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_disponibles(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<AbsenceMotif>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let globals = sqlx::query_as::<_, MotifRow>(
            r#"SELECT code, libelle, NULL::uuid AS etablissement_id FROM ent_absence_motif ORDER BY code"#,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;

        let etab = sqlx::query_as::<_, MotifRow>(
            r#"
            SELECT code, libelle, etablissement_id
            FROM ent_absence_motif_etab
            WHERE etablissement_id = $1
            ORDER BY code
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;

        Ok(globals
            .into_iter()
            .chain(etab)
            .map(MotifRow::into_motif)
            .collect())
    }

    pub async fn est_connu(
        &self,
        etablissement_id: Uuid,
        code: &str,
    ) -> Result<bool, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let (global,): (bool,) = sqlx::query_as(
            r#"SELECT EXISTS(SELECT 1 FROM ent_absence_motif WHERE code = $1)"#,
        )
        .bind(code)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if global {
            return Ok(true);
        }
        let (local,): (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM ent_absence_motif_etab
                WHERE etablissement_id = $1 AND code = $2
            )
            "#,
        )
        .bind(etablissement_id)
        .bind(code)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        Ok(local)
    }

    pub async fn create_etab(
        &self,
        etablissement_id: Uuid,
        code: &str,
        libelle: &str,
    ) -> Result<AbsenceMotif, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_absence_motif_etab (etablissement_id, code, libelle, created_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (etablissement_id, code) DO UPDATE SET libelle = EXCLUDED.libelle
            "#,
        )
        .bind(etablissement_id)
        .bind(code)
        .bind(libelle)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(AbsenceMotif {
            code: code.to_string(),
            libelle: libelle.to_string(),
            etablissement_id: Some(etablissement_id),
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct MotifRow {
    code: String,
    libelle: String,
    etablissement_id: Option<Uuid>,
}

impl MotifRow {
    fn into_motif(self) -> AbsenceMotif {
        AbsenceMotif {
            code: self.code,
            libelle: self.libelle,
            etablissement_id: self.etablissement_id,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("motif_repo", err)
}
