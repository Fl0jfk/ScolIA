use crate::champs_chiffres::ChampsSensibles;
use chrono::{DateTime, Utc};
use domain_eleve::DomainError;
use sqlx::PgPool;
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

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
    champs: ChampsSensibles,
}

const T_HISTO: &str = "ent_foyer_iban_histo";

impl PgIbanHistoRepo {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            champs: ChampsSensibles::depuis_env(),
        }
    }

    pub async fn insert(&self, h: &IbanHisto) -> Result<(), DomainError> {
        // D3 : IBAN, BIC et titulaire chiffrés (AAD = table + colonne + id de la ligne).
        let iban = self.champs.sceller(T_HISTO, "iban", h.id, &h.iban)?;
        let bic = self.champs.sceller_opt(T_HISTO, "bic", h.id, h.bic.as_deref())?;
        let titulaire = self.champs.sceller(T_HISTO, "titulaire", h.id, &h.titulaire)?;
        let mut tx = TenantTx::begin(&self.pool, h.etablissement_id).await.map_err(map_sqlx)?;
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
        .bind(&iban)
        .bind(&bic)
        .bind(&titulaire)
        .bind(h.mandat_id)
        .bind(h.created_by_user_id)
        .bind(h.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
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
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        rows.into_iter().map(|r| r.into_histo(&self.champs)).collect()
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("iban_histo", e)
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
    fn into_histo(self, champs: &ChampsSensibles) -> Result<IbanHisto, DomainError> {
        Ok(IbanHisto {
            id: self.id,
            etablissement_id: self.etablissement_id,
            foyer_id: self.foyer_id,
            iban: champs.ouvrir(T_HISTO, "iban", self.id, &self.iban)?,
            bic: champs.ouvrir_opt(T_HISTO, "bic", self.id, self.bic)?,
            titulaire: champs.ouvrir(T_HISTO, "titulaire", self.id, &self.titulaire)?,
            mandat_id: self.mandat_id,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}
