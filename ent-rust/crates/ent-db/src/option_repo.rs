use domain_eleve::{DomainError, EleveOption, OptionNature};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgOptionRepo {
    pool: PgPool,
}

impl PgOptionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, opt: &EleveOption) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_eleve_option (
                id, etablissement_id, eleve_id, nature, code, libelle, date_debut, date_fin, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now())
            ON CONFLICT (id) DO UPDATE SET
                nature = EXCLUDED.nature,
                code = EXCLUDED.code,
                libelle = EXCLUDED.libelle,
                date_debut = EXCLUDED.date_debut,
                date_fin = EXCLUDED.date_fin
            WHERE ent_eleve_option.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(opt.id)
        .bind(opt.etablissement_id)
        .bind(opt.eleve_id)
        .bind(opt.nature.as_str())
        .bind(&opt.code)
        .bind(&opt.libelle)
        .bind(opt.date_debut)
        .bind(opt.date_fin)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_for_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<EleveOption>, DomainError> {
        let rows = sqlx::query_as::<_, OptRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, nature, code, libelle, date_debut, date_fin
            FROM ent_eleve_option
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY nature, code
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(OptRow::into_opt).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct OptRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    nature: String,
    code: String,
    libelle: String,
    date_debut: chrono::NaiveDate,
    date_fin: Option<chrono::NaiveDate>,
}

impl OptRow {
    fn into_opt(self) -> Option<EleveOption> {
        let nature = OptionNature::parse(&self.nature)?;
        Some(EleveOption {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            nature,
            code: self.code,
            libelle: self.libelle,
            date_debut: self.date_debut,
            date_fin: self.date_fin,
        })
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("option_repo", err)
}
