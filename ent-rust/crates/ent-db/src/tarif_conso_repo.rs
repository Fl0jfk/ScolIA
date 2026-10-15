use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{DomainError, TarifConso};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgTarifConsoRepo {
    pool: PgPool,
}

impl PgTarifConsoRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, t: &TarifConso) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_tarif_conso (
                id, etablissement_id, code, libelle, montant_centimes,
                debut, fin, actif, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(t.id)
        .bind(t.etablissement_id)
        .bind(&t.code)
        .bind(&t.libelle)
        .bind(t.montant_centimes)
        .bind(t.debut)
        .bind(t.fin)
        .bind(t.actif)
        .bind(t.created_by_user_id)
        .bind(t.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list(
        &self,
        etablissement_id: Uuid,
        actifs_seulement: bool,
    ) -> Result<Vec<TarifConso>, DomainError> {
        let rows = if actifs_seulement {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, code, libelle, montant_centimes,
                       debut, fin, actif, created_by_user_id, created_at
                FROM ent_tarif_conso
                WHERE etablissement_id = $1 AND actif = true
                ORDER BY code, debut
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, code, libelle, montant_centimes,
                       debut, fin, actif, created_by_user_id, created_at
                FROM ent_tarif_conso
                WHERE etablissement_id = $1
                ORDER BY code, debut
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().map(Row::into_tarif).collect())
    }

    pub async fn list_au_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<TarifConso>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, code, libelle, montant_centimes,
                   debut, fin, actif, created_by_user_id, created_at
            FROM ent_tarif_conso
            WHERE etablissement_id = $1 AND actif = true
              AND debut <= $2 AND (fin IS NULL OR fin >= $2)
            ORDER BY code
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Row::into_tarif).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] tarif_conso sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    code: String,
    libelle: String,
    montant_centimes: i64,
    debut: NaiveDate,
    fin: Option<NaiveDate>,
    actif: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl Row {
    fn into_tarif(self) -> TarifConso {
        TarifConso {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            libelle: self.libelle,
            montant_centimes: self.montant_centimes,
            debut: self.debut,
            fin: self.fin,
            actif: self.actif,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}
