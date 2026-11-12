use crate::tenant_tx::TenantTx;
use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{AbsenceRh, DomainError};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgAbsenceRhRepo {
    pool: PgPool,
}

/// Colonnes lues d'une absence (projection `ent_absence_rh`).
pub(crate) const COLONNES: &str = "id, etablissement_id, user_id, debut, fin, motif, statut,
       created_by_user_id, created_at, valide_par_user_id, valide_le, refuse_par_user_id,
       refuse_le, annule_par_user_id, annule_le, confirmee";

impl PgAbsenceRhRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
    ) -> Result<Option<AbsenceRh>, DomainError> {
        let sql = format!(
            "SELECT {COLONNES} FROM ent_absence_rh WHERE etablissement_id = $1 AND id = $2"
        );
        let row = sqlx::query_as::<_, Row>(&sql)
            .bind(etablissement_id)
            .bind(id)
            .fetch_optional(
                &mut *TenantTx::begin(&self.pool, etablissement_id)
                    .await
                    .map_err(map_sqlx)?,
            )
            .await
            .map_err(map_sqlx)?;
        Ok(row.map(Row::into_absence))
    }

    pub async fn list_periode(
        &self,
        etablissement_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<AbsenceRh>, DomainError> {
        let sql = format!(
            "SELECT {COLONNES} FROM ent_absence_rh
              WHERE etablissement_id = $1 AND debut <= $3 AND fin >= $2
              ORDER BY debut, user_id"
        );
        let rows = sqlx::query_as::<_, Row>(&sql)
            .bind(etablissement_id)
            .bind(debut)
            .bind(fin)
            .fetch_all(
                &mut *TenantTx::begin(&self.pool, etablissement_id)
                    .await
                    .map_err(map_sqlx)?,
            )
            .await
            .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Row::into_absence).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("absence_rh", e)
}

#[derive(sqlx::FromRow)]
pub(crate) struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    user_id: Uuid,
    debut: NaiveDate,
    fin: NaiveDate,
    motif: String,
    statut: String,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
    valide_par_user_id: Option<Uuid>,
    valide_le: Option<DateTime<Utc>>,
    refuse_par_user_id: Option<Uuid>,
    refuse_le: Option<DateTime<Utc>>,
    annule_par_user_id: Option<Uuid>,
    annule_le: Option<DateTime<Utc>>,
    confirmee: bool,
}

impl Row {
    pub(crate) fn into_absence(self) -> AbsenceRh {
        AbsenceRh {
            id: self.id,
            etablissement_id: self.etablissement_id,
            user_id: self.user_id,
            debut: self.debut,
            fin: self.fin,
            motif: self.motif,
            statut: self.statut,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
            valide_par_user_id: self.valide_par_user_id,
            valide_le: self.valide_le,
            refuse_par_user_id: self.refuse_par_user_id,
            refuse_le: self.refuse_le,
            annule_par_user_id: self.annule_par_user_id,
            annule_le: self.annule_le,
            confirmee: self.confirmee,
        }
    }
}
