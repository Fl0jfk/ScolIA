use chrono::{DateTime, Utc};
use domain_eleve::DomainError;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IbanHisto {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub foyer_id: Uuid,
    pub iban: String,
    pub bic: Option<String>,
    pub titulaire: String,
    pub mandat_id: Option<Uuid>,
    pub created_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

pub struct PgIbanHistoRepo {
    pool: PgPool,
}

impl PgIbanHistoRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, h: &IbanHisto) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_foyer_iban_histo (
                id, etablissement_id, foyer_id, iban, bic, titulaire,
                mandat_id, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(h.id)
        .bind(h.etablissement_id)
        .bind(h.foyer_id)
        .bind(&h.iban)
        .bind(&h.bic)
        .bind(&h.titulaire)
        .bind(h.mandat_id)
        .bind(h.created_by_user_id)
        .bind(h.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_foyer(
        &self,
        etablissement_id: Uuid,
        foyer_id: Uuid,
        limit: i64,
    ) -> Result<Vec<IbanHisto>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, foyer_id, iban, bic, titulaire,
                   mandat_id, created_by_user_id, created_at
            FROM ent_foyer_iban_histo
            WHERE etablissement_id = $1 AND foyer_id = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(foyer_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Row::into_histo).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] iban_histo sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    foyer_id: Uuid,
    iban: String,
    bic: Option<String>,
    titulaire: String,
    mandat_id: Option<Uuid>,
    created_by_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

impl Row {
    fn into_histo(self) -> IbanHisto {
        IbanHisto {
            id: self.id,
            etablissement_id: self.etablissement_id,
            foyer_id: self.foyer_id,
            iban: self.iban,
            bic: self.bic,
            titulaire: self.titulaire,
            mandat_id: self.mandat_id,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}
