use chrono::{DateTime, Utc};
use domain_eleve::{DomainError, SignalAbsence, SignalAbsenceStatut};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgSignalRepo {
    pool: PgPool,
}

impl PgSignalRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert signal pour (etab, élève, année) — met à jour le compteur.
    /// Un signal déjà `clos` reste clos (la clôture VS est un acte métier).
    pub async fn upsert_ouvert(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        annee_scolaire: &str,
        seuil: i32,
        compteur: i32,
    ) -> Result<SignalAbsence, DomainError> {
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, SignalRow>(
            r#"
            INSERT INTO ent_absence_signal (
                id, etablissement_id, eleve_id, annee_scolaire, seuil, compteur, statut,
                note_vs, traite_par_user_id, traite_at, fil_alerte_id, commission_dossier_id,
                created_at, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,'ouvert', NULL, NULL, NULL, NULL, NULL, now(), now())
            ON CONFLICT (etablissement_id, eleve_id, annee_scolaire) DO UPDATE SET
                compteur = EXCLUDED.compteur,
                seuil = EXCLUDED.seuil,
                updated_at = now(),
                statut = CASE
                    WHEN ent_absence_signal.statut = 'clos' THEN 'clos'
                    WHEN ent_absence_signal.statut = 'en_cours' THEN 'en_cours'
                    ELSE 'ouvert'
                END
            RETURNING id, etablissement_id, eleve_id, annee_scolaire, seuil, compteur, statut,
                      note_vs, traite_par_user_id, fil_alerte_id, commission_dossier_id
            "#,
        )
        .bind(id)
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(annee_scolaire)
        .bind(seuil)
        .bind(compteur)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        row.into_signal().ok_or(DomainError::NotFound)
    }

    /// Attache le fil d'alerte famille une seule fois (idempotent).
    /// Retourne `true` si le lien a été posé maintenant.
    pub async fn set_fil_alerte(
        &self,
        etablissement_id: Uuid,
        signal_id: Uuid,
        fil_id: Uuid,
    ) -> Result<bool, DomainError> {
        let n = sqlx::query(
            r#"
            UPDATE ent_absence_signal
            SET fil_alerte_id = $3, updated_at = now()
            WHERE etablissement_id = $1
              AND id = $2
              AND fil_alerte_id IS NULL
            "#,
        )
        .bind(etablissement_id)
        .bind(signal_id)
        .bind(fil_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        Ok(n > 0)
    }

    /// Attache le dossier de commission une seule fois (idempotent).
    pub async fn set_commission_dossier(
        &self,
        etablissement_id: Uuid,
        signal_id: Uuid,
        dossier_id: Uuid,
    ) -> Result<bool, DomainError> {
        let n = sqlx::query(
            r#"
            UPDATE ent_absence_signal
            SET commission_dossier_id = $3, updated_at = now()
            WHERE etablissement_id = $1
              AND id = $2
              AND commission_dossier_id IS NULL
            "#,
        )
        .bind(etablissement_id)
        .bind(signal_id)
        .bind(dossier_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        Ok(n > 0)
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        signal_id: Uuid,
    ) -> Result<Option<SignalAbsence>, DomainError> {
        let row = sqlx::query_as::<_, SignalRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, annee_scolaire, seuil, compteur, statut,
                   note_vs, traite_par_user_id, fil_alerte_id, commission_dossier_id
            FROM ent_absence_signal
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(signal_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(SignalRow::into_signal))
    }

    pub async fn update_workflow(&self, s: &SignalAbsence) -> Result<(), DomainError> {
        let traite_at: Option<DateTime<Utc>> = if s.statut == SignalAbsenceStatut::Clos
            || s.statut == SignalAbsenceStatut::EnCours
        {
            Some(Utc::now())
        } else {
            None
        };
        let n = sqlx::query(
            r#"
            UPDATE ent_absence_signal
            SET statut = $3,
                note_vs = $4,
                traite_par_user_id = $5,
                traite_at = COALESCE($6, traite_at),
                commission_dossier_id = COALESCE($7, commission_dossier_id),
                updated_at = now()
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(s.etablissement_id)
        .bind(s.id)
        .bind(s.statut.as_str())
        .bind(&s.note_vs)
        .bind(s.traite_par_user_id)
        .bind(traite_at)
        .bind(s.commission_dossier_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }

    /// File VS active : ouverts + en cours.
    pub async fn list_actifs(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<SignalAbsence>, DomainError> {
        let rows = sqlx::query_as::<_, SignalRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, annee_scolaire, seuil, compteur, statut,
                   note_vs, traite_par_user_id, fil_alerte_id, commission_dossier_id
            FROM ent_absence_signal
            WHERE etablissement_id = $1 AND statut IN ('ouvert', 'en_cours')
            ORDER BY
                CASE statut WHEN 'ouvert' THEN 0 WHEN 'en_cours' THEN 1 ELSE 2 END,
                compteur DESC,
                updated_at DESC
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(SignalRow::into_signal).collect())
    }

    pub async fn get_ouvert(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<Option<SignalAbsence>, DomainError> {
        let row = sqlx::query_as::<_, SignalRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, annee_scolaire, seuil, compteur, statut,
                   note_vs, traite_par_user_id, fil_alerte_id, commission_dossier_id
            FROM ent_absence_signal
            WHERE etablissement_id = $1
              AND eleve_id = $2
              AND annee_scolaire = $3
              AND statut IN ('ouvert', 'en_cours')
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(annee_scolaire)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(SignalRow::into_signal))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SignalRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    annee_scolaire: String,
    seuil: i32,
    compteur: i32,
    statut: String,
    note_vs: Option<String>,
    traite_par_user_id: Option<Uuid>,
    fil_alerte_id: Option<Uuid>,
    commission_dossier_id: Option<Uuid>,
}

impl SignalRow {
    fn into_signal(self) -> Option<SignalAbsence> {
        Some(SignalAbsence {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            annee_scolaire: self.annee_scolaire,
            seuil: self.seuil,
            compteur: self.compteur,
            statut: SignalAbsenceStatut::parse(&self.statut)?,
            note_vs: self.note_vs,
            traite_par_user_id: self.traite_par_user_id,
            fil_alerte_id: self.fil_alerte_id,
            commission_dossier_id: self.commission_dossier_id,
        })
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("signal_repo", err)
}
