use chrono::NaiveDate;
use domain_eleve::{Absence, AbsenceStatut, DomainError};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgAbsenceRepo {
    pool: PgPool,
}

impl PgAbsenceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, absence: &Absence) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_absence (
                id, etablissement_id, eleve_id, debut, fin, motif, statut,
                motif_justif, justifie_par_user_id, justifie_at,
                decision_note, decision_par_user_id, decision_at, created_at
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,
                CASE WHEN $7 = 'justifiee' THEN now() ELSE NULL END,
                $10,$11,
                CASE WHEN $7 IN ('justifiee','refusee','annulee_present') THEN now() ELSE NULL END,
                now()
            )
            ON CONFLICT (id) DO UPDATE SET
                debut = EXCLUDED.debut,
                fin = EXCLUDED.fin,
                motif = EXCLUDED.motif,
                statut = EXCLUDED.statut,
                motif_justif = EXCLUDED.motif_justif,
                justifie_par_user_id = EXCLUDED.justifie_par_user_id,
                decision_note = EXCLUDED.decision_note,
                decision_par_user_id = EXCLUDED.decision_par_user_id,
                justifie_at = CASE
                    WHEN EXCLUDED.statut = 'justifiee'
                         AND ent_absence.statut IS DISTINCT FROM 'justifiee'
                    THEN now()
                    ELSE ent_absence.justifie_at
                END,
                decision_at = CASE
                    WHEN EXCLUDED.statut IN ('justifiee','refusee','annulee_present')
                         AND ent_absence.statut IS DISTINCT FROM EXCLUDED.statut
                    THEN now()
                    ELSE ent_absence.decision_at
                END
            WHERE ent_absence.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(absence.id)
        .bind(absence.etablissement_id)
        .bind(absence.eleve_id)
        .bind(absence.debut)
        .bind(absence.fin)
        .bind(&absence.motif)
        .bind(absence.statut.as_str())
        .bind(&absence.motif_justif)
        .bind(absence.justifie_par_user_id)
        .bind(&absence.decision_note)
        .bind(absence.decision_par_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        absence_id: Uuid,
    ) -> Result<Option<Absence>, DomainError> {
        let row = sqlx::query_as::<_, AbsenceRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, debut, fin, motif, statut,
                   motif_justif, justifie_par_user_id, decision_note, decision_par_user_id
            FROM ent_absence
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(absence_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(AbsenceRow::into_absence))
    }

    pub async fn list_for_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<Absence>, DomainError> {
        let rows = sqlx::query_as::<_, AbsenceRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, debut, fin, motif, statut,
                   motif_justif, justifie_par_user_id, decision_note, decision_par_user_id
            FROM ent_absence
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY debut DESC, created_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(AbsenceRow::into_absence).collect())
    }

    pub async fn list_a_traiter_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Absence>, DomainError> {
        let rows = sqlx::query_as::<_, AbsenceRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, debut, fin, motif, statut,
                   motif_justif, justifie_par_user_id, decision_note, decision_par_user_id
            FROM ent_absence
            WHERE etablissement_id = $1
              AND statut = 'a_traiter'
              AND debut <= $2
              AND fin >= $2
            ORDER BY debut, eleve_id
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(AbsenceRow::into_absence).collect())
    }

    pub async fn count_actives_depuis(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        depuis: NaiveDate,
    ) -> Result<i64, DomainError> {
        let (n,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_absence
            WHERE etablissement_id = $1
              AND eleve_id = $2
              AND debut >= $3
              AND statut <> 'annulee_present'
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(depuis)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(n)
    }

    pub async fn est_absent(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, DomainError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT id FROM ent_absence
            WHERE etablissement_id = $1
              AND eleve_id = $2
              AND debut <= $3
              AND fin >= $3
              AND statut <> 'annulee_present'
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(jour)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.is_some())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AbsenceRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    debut: NaiveDate,
    fin: NaiveDate,
    motif: String,
    statut: String,
    motif_justif: Option<String>,
    justifie_par_user_id: Option<Uuid>,
    decision_note: Option<String>,
    decision_par_user_id: Option<Uuid>,
}

impl AbsenceRow {
    fn into_absence(self) -> Absence {
        Absence {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            debut: self.debut,
            fin: self.fin,
            motif: self.motif,
            statut: AbsenceStatut::parse(&self.statut),
            motif_justif: self.motif_justif,
            justifie_par_user_id: self.justifie_par_user_id,
            decision_note: self.decision_note,
            decision_par_user_id: self.decision_par_user_id,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] absence sqlx error: {err}");
    DomainError::NotFound
}
