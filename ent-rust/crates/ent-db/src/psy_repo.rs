use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use domain_eleve::{
    DomainError, DossierPsy, EntretienPsy, EntretienPsyKind, RdvPsy, RdvPsyStatut,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgPsyRepo {
    pool: PgPool,
}

impl PgPsyRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, e: &EntretienPsy) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_psy_entretien (
                id, etablissement_id, eleve_id, jour, kind, duree_minutes,
                note_confidentielle, signale_famille, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(e.id)
        .bind(e.etablissement_id)
        .bind(e.eleve_id)
        .bind(e.jour)
        .bind(e.kind.as_str())
        .bind(e.duree_minutes)
        .bind(&e.note_confidentielle)
        .bind(e.signale_famille)
        .bind(e.created_by_user_id)
        .bind(e.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        limit: i64,
    ) -> Result<Vec<EntretienPsy>, DomainError> {
        let rows = sqlx::query_as::<_, EntretienRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, kind, duree_minutes,
                   note_confidentielle, signale_famille, created_by_user_id, created_at
            FROM ent_psy_entretien
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY jour DESC, created_at DESC
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(EntretienRow::into_entretien).collect())
    }

    pub async fn list_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
        limit: i64,
    ) -> Result<Vec<EntretienPsy>, DomainError> {
        let rows = sqlx::query_as::<_, EntretienRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, kind, duree_minutes,
                   note_confidentielle, signale_famille, created_by_user_id, created_at
            FROM ent_psy_entretien
            WHERE etablissement_id = $1 AND jour = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(EntretienRow::into_entretien).collect())
    }

    pub async fn insert_rdv(&self, r: &RdvPsy) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_psy_rdv (
                id, etablissement_id, eleve_id, jour, heure_debut, heure_fin, kind,
                motif, note_interne, statut, signale_famille, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            "#,
        )
        .bind(r.id)
        .bind(r.etablissement_id)
        .bind(r.eleve_id)
        .bind(r.jour)
        .bind(r.heure_debut)
        .bind(r.heure_fin)
        .bind(r.kind.as_str())
        .bind(&r.motif)
        .bind(&r.note_interne)
        .bind(r.statut.as_str())
        .bind(r.signale_famille)
        .bind(r.created_by_user_id)
        .bind(r.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_rdv(
        &self,
        etablissement_id: Uuid,
        rdv_id: Uuid,
    ) -> Result<Option<RdvPsy>, DomainError> {
        let row = sqlx::query_as::<_, RdvRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, heure_debut, heure_fin, kind,
                   motif, note_interne, statut, signale_famille, created_by_user_id, created_at
            FROM ent_psy_rdv
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(rdv_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(RdvRow::into_rdv))
    }

    pub async fn update_rdv_statut(&self, r: &RdvPsy) -> Result<(), DomainError> {
        let n = sqlx::query(
            r#"
            UPDATE ent_psy_rdv SET statut = $3
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(r.etablissement_id)
        .bind(r.id)
        .bind(r.statut.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }

    pub async fn list_rdv_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
        limit: i64,
    ) -> Result<Vec<RdvPsy>, DomainError> {
        let rows = sqlx::query_as::<_, RdvRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, heure_debut, heure_fin, kind,
                   motif, note_interne, statut, signale_famille, created_by_user_id, created_at
            FROM ent_psy_rdv
            WHERE etablissement_id = $1 AND jour = $2
            ORDER BY heure_debut, created_at
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(RdvRow::into_rdv).collect())
    }

    pub async fn list_rdv_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        limit: i64,
    ) -> Result<Vec<RdvPsy>, DomainError> {
        let rows = sqlx::query_as::<_, RdvRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, heure_debut, heure_fin, kind,
                   motif, note_interne, statut, signale_famille, created_by_user_id, created_at
            FROM ent_psy_rdv
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY jour DESC, heure_debut
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(RdvRow::into_rdv).collect())
    }

    pub async fn upsert_dossier(&self, d: &DossierPsy) -> Result<DossierPsy, DomainError> {
        let row = sqlx::query_as::<_, DossierRow>(
            r#"
            INSERT INTO ent_psy_dossier (
                id, etablissement_id, eleve_id, axes_suivi, contacts_externes,
                note_confidentielle, suivi_actif, signale_famille,
                updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            ON CONFLICT (etablissement_id, eleve_id) DO UPDATE SET
                id = ent_psy_dossier.id,
                axes_suivi = EXCLUDED.axes_suivi,
                contacts_externes = EXCLUDED.contacts_externes,
                note_confidentielle = EXCLUDED.note_confidentielle,
                suivi_actif = EXCLUDED.suivi_actif,
                signale_famille = EXCLUDED.signale_famille,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at
            RETURNING id, etablissement_id, eleve_id, axes_suivi, contacts_externes,
                      note_confidentielle, suivi_actif, signale_famille,
                      updated_by_user_id, updated_at
            "#,
        )
        .bind(d.id)
        .bind(d.etablissement_id)
        .bind(d.eleve_id)
        .bind(&d.axes_suivi)
        .bind(&d.contacts_externes)
        .bind(&d.note_confidentielle)
        .bind(d.suivi_actif)
        .bind(d.signale_famille)
        .bind(d.updated_by_user_id)
        .bind(d.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.into_dossier())
    }

    pub async fn get_dossier(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<DossierPsy>, DomainError> {
        let row = sqlx::query_as::<_, DossierRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, axes_suivi, contacts_externes,
                   note_confidentielle, suivi_actif, signale_famille,
                   updated_by_user_id, updated_at
            FROM ent_psy_dossier
            WHERE etablissement_id = $1 AND eleve_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(DossierRow::into_dossier))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DossierRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    axes_suivi: String,
    contacts_externes: String,
    note_confidentielle: String,
    suivi_actif: bool,
    signale_famille: bool,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl DossierRow {
    fn into_dossier(self) -> DossierPsy {
        DossierPsy {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            axes_suivi: self.axes_suivi,
            contacts_externes: self.contacts_externes,
            note_confidentielle: self.note_confidentielle,
            suivi_actif: self.suivi_actif,
            signale_famille: self.signale_famille,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct RdvRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    heure_debut: NaiveTime,
    heure_fin: NaiveTime,
    kind: String,
    motif: String,
    note_interne: String,
    statut: String,
    signale_famille: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl RdvRow {
    fn into_rdv(self) -> Option<RdvPsy> {
        Some(RdvPsy {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            jour: self.jour,
            heure_debut: self.heure_debut,
            heure_fin: self.heure_fin,
            kind: EntretienPsyKind::parse(&self.kind)?,
            motif: self.motif,
            note_interne: self.note_interne,
            statut: RdvPsyStatut::parse(&self.statut)?,
            signale_famille: self.signale_famille,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EntretienRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    kind: String,
    duree_minutes: i32,
    note_confidentielle: String,
    signale_famille: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl EntretienRow {
    fn into_entretien(self) -> Option<EntretienPsy> {
        let kind = EntretienPsyKind::parse(&self.kind)?;
        Some(EntretienPsy {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            jour: self.jour,
            kind,
            duree_minutes: self.duree_minutes,
            note_confidentielle: self.note_confidentielle,
            signale_famille: self.signale_famille,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] psy sqlx error: {err}");
    DomainError::NotFound
}
