//! Emploi du temps — créneaux hebdo + exceptions jour J + A/B + journal d'import CSV.

use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc};
use domain_eleve::{DomainError, EdtCalendrierAb, EdtException, EdtExceptionKind, EdtSemaineAb};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdtCreneau {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub classe: String,
    pub matiere_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    /// ISO 1=lundi … 7=dimanche.
    pub jour_semaine: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub salle: Option<String>,
    /// Site pédagogique (autre campus) — vide = site principal / non renseigné.
    pub site: String,
    pub semaine_ab: EdtSemaineAb,
}

pub struct PgEdtRepo {
    pool: PgPool,
}

impl PgEdtRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, c: &EdtCreneau) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_edt_creneau (
                id, etablissement_id, classe, matiere_id, matiere_libelle,
                prof_user_id, jour_semaine, heure_debut, heure_fin, salle, site, semaine_ab
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            "#,
        )
        .bind(c.id)
        .bind(c.etablissement_id)
        .bind(&c.classe)
        .bind(c.matiere_id)
        .bind(&c.matiere_libelle)
        .bind(c.prof_user_id)
        .bind(c.jour_semaine)
        .bind(c.heure_debut)
        .bind(c.heure_fin)
        .bind(c.salle.as_deref())
        .bind(&c.site)
        .bind(c.semaine_ab.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
    ) -> Result<Option<EdtCreneau>, DomainError> {
        let row = sqlx::query_as::<_, CreneauRow>(
            r#"
            SELECT id, etablissement_id, classe, matiere_id, matiere_libelle,
                   prof_user_id, jour_semaine, heure_debut, heure_fin, salle, site, semaine_ab
            FROM ent_edt_creneau
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(CreneauRow::into_creneau))
    }

    pub async fn list_pour_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<EdtCreneau>, DomainError> {
        let js = jour_semaine_iso(jour);
        let rows = sqlx::query_as::<_, CreneauRow>(
            r#"
            SELECT id, etablissement_id, classe, matiere_id, matiere_libelle,
                   prof_user_id, jour_semaine, heure_debut, heure_fin, salle, site, semaine_ab
            FROM ent_edt_creneau
            WHERE etablissement_id = $1 AND jour_semaine = $2
            ORDER BY heure_debut, classe
            "#,
        )
        .bind(etablissement_id)
        .bind(js)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(CreneauRow::into_creneau).collect())
    }

    pub async fn update_affectation(
        &self,
        etablissement_id: Uuid,
        creneau_id: Uuid,
        prof_user_id: Uuid,
        salle: Option<&str>,
    ) -> Result<(), DomainError> {
        let res = sqlx::query(
            r#"
            UPDATE ent_edt_creneau
            SET prof_user_id = $3, salle = $4
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(creneau_id)
        .bind(prof_user_id)
        .bind(salle)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }

    pub async fn creneau_labels(
        &self,
        etablissement_id: Uuid,
        creneau_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, DomainError> {
        if creneau_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(Uuid, String, String, NaiveTime)> = sqlx::query_as(
            r#"
            SELECT id, classe, matiere_libelle, heure_debut
            FROM ent_edt_creneau
            WHERE etablissement_id = $1 AND id = ANY($2)
            "#,
        )
        .bind(etablissement_id)
        .bind(creneau_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(|(id, classe, matiere, debut)| {
                (
                    id,
                    format!("{} {} {}", classe, matiere, debut.format("%H:%M")),
                )
            })
            .collect())
    }

    pub async fn list_all(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<EdtCreneau>, DomainError> {
        let rows = sqlx::query_as::<_, CreneauRow>(
            r#"
            SELECT id, etablissement_id, classe, matiere_id, matiere_libelle,
                   prof_user_id, jour_semaine, heure_debut, heure_fin, salle, site, semaine_ab
            FROM ent_edt_creneau
            WHERE etablissement_id = $1
            ORDER BY jour_semaine, heure_debut, classe
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(CreneauRow::into_creneau).collect())
    }

    pub async fn insert_import_journal(
        &self,
        id: Uuid,
        etablissement_id: Uuid,
        dry_run: bool,
        lines_count: i32,
        applied_count: i32,
        conflicts_count: i32,
        report_json: serde_json::Value,
        created_by_user_id: Uuid,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_edt_import (
                id, etablissement_id, dry_run, lines_count, applied_count,
                conflicts_count, report_json, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now())
            "#,
        )
        .bind(id)
        .bind(etablissement_id)
        .bind(dry_run)
        .bind(lines_count)
        .bind(applied_count)
        .bind(conflicts_count)
        .bind(report_json)
        .bind(created_by_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn count_imports(
        &self,
        etablissement_id: Uuid,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_edt_import
            WHERE etablissement_id = $1
            "#,
        )
        .bind(etablissement_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.0)
    }

    pub async fn list_pour_prof_jour(
        &self,
        etablissement_id: Uuid,
        prof_user_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<EdtCreneau>, DomainError> {
        let js = jour_semaine_iso(jour);
        let rows = sqlx::query_as::<_, CreneauRow>(
            r#"
            SELECT id, etablissement_id, classe, matiere_id, matiere_libelle,
                   prof_user_id, jour_semaine, heure_debut, heure_fin, salle, site, semaine_ab
            FROM ent_edt_creneau
            WHERE etablissement_id = $1
              AND prof_user_id = $2
              AND jour_semaine = $3
            ORDER BY heure_debut, classe
            "#,
        )
        .bind(etablissement_id)
        .bind(prof_user_id)
        .bind(js)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(CreneauRow::into_creneau).collect())
    }

    pub async fn upsert_calendrier_ab(&self, c: &EdtCalendrierAb) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_edt_calendrier_ab (
                etablissement_id, reference_lundi, created_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4)
            ON CONFLICT (etablissement_id) DO UPDATE SET
                reference_lundi = EXCLUDED.reference_lundi,
                created_by_user_id = EXCLUDED.created_by_user_id,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(c.etablissement_id)
        .bind(c.reference_lundi)
        .bind(c.created_by_user_id)
        .bind(c.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_calendrier_ab(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Option<EdtCalendrierAb>, DomainError> {
        let row = sqlx::query_as::<_, CalendrierAbRow>(
            r#"
            SELECT etablissement_id, reference_lundi, created_by_user_id, updated_at
            FROM ent_edt_calendrier_ab
            WHERE etablissement_id = $1
            "#,
        )
        .bind(etablissement_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(CalendrierAbRow::into_calendrier))
    }

    pub async fn upsert_exception(&self, e: &EdtException) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_edt_exception (
                id, etablissement_id, creneau_id, jour, kind,
                remplacant_user_id, nouvelle_salle, motif,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            ON CONFLICT (etablissement_id, creneau_id, jour) DO UPDATE SET
                id = EXCLUDED.id,
                kind = EXCLUDED.kind,
                remplacant_user_id = EXCLUDED.remplacant_user_id,
                nouvelle_salle = EXCLUDED.nouvelle_salle,
                motif = EXCLUDED.motif,
                created_by_user_id = EXCLUDED.created_by_user_id,
                created_at = EXCLUDED.created_at
            "#,
        )
        .bind(e.id)
        .bind(e.etablissement_id)
        .bind(e.creneau_id)
        .bind(e.jour)
        .bind(e.kind.as_str())
        .bind(e.remplacant_user_id)
        .bind(e.nouvelle_salle.as_deref())
        .bind(&e.motif)
        .bind(e.created_by_user_id)
        .bind(e.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_exception(
        &self,
        etablissement_id: Uuid,
        creneau_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<EdtException>, DomainError> {
        let row = sqlx::query_as::<_, ExceptionRow>(
            r#"
            SELECT id, etablissement_id, creneau_id, jour, kind,
                   remplacant_user_id, nouvelle_salle, motif,
                   created_by_user_id, created_at
            FROM ent_edt_exception
            WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3
            "#,
        )
        .bind(etablissement_id)
        .bind(creneau_id)
        .bind(jour)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(ExceptionRow::into_exception))
    }

    pub async fn list_exceptions_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<EdtException>, DomainError> {
        let rows = sqlx::query_as::<_, ExceptionRow>(
            r#"
            SELECT id, etablissement_id, creneau_id, jour, kind,
                   remplacant_user_id, nouvelle_salle, motif,
                   created_by_user_id, created_at
            FROM ent_edt_exception
            WHERE etablissement_id = $1 AND jour = $2
            ORDER BY created_at
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .filter_map(ExceptionRow::into_exception)
            .collect())
    }
}

pub fn jour_semaine_iso(jour: NaiveDate) -> i16 {
    jour.weekday().number_from_monday() as i16
}

#[derive(Debug, sqlx::FromRow)]
struct CreneauRow {
    id: Uuid,
    etablissement_id: Uuid,
    classe: String,
    matiere_id: Option<Uuid>,
    matiere_libelle: String,
    prof_user_id: Uuid,
    jour_semaine: i16,
    heure_debut: NaiveTime,
    heure_fin: NaiveTime,
    salle: Option<String>,
    site: String,
    semaine_ab: String,
}

impl CreneauRow {
    fn into_creneau(self) -> Option<EdtCreneau> {
        let semaine_ab = EdtSemaineAb::parse(&self.semaine_ab)?;
        Some(EdtCreneau {
            id: self.id,
            etablissement_id: self.etablissement_id,
            classe: self.classe,
            matiere_id: self.matiere_id,
            matiere_libelle: self.matiere_libelle,
            prof_user_id: self.prof_user_id,
            jour_semaine: self.jour_semaine,
            heure_debut: self.heure_debut,
            heure_fin: self.heure_fin,
            salle: self.salle,
            site: self.site,
            semaine_ab,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct CalendrierAbRow {
    etablissement_id: Uuid,
    reference_lundi: NaiveDate,
    created_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl CalendrierAbRow {
    fn into_calendrier(self) -> EdtCalendrierAb {
        EdtCalendrierAb {
            etablissement_id: self.etablissement_id,
            reference_lundi: self.reference_lundi,
            created_by_user_id: self.created_by_user_id,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ExceptionRow {
    id: Uuid,
    etablissement_id: Uuid,
    creneau_id: Uuid,
    jour: NaiveDate,
    kind: String,
    remplacant_user_id: Option<Uuid>,
    nouvelle_salle: Option<String>,
    motif: String,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl ExceptionRow {
    fn into_exception(self) -> Option<EdtException> {
        let kind = EdtExceptionKind::parse(&self.kind)?;
        Some(EdtException {
            id: self.id,
            etablissement_id: self.etablissement_id,
            creneau_id: self.creneau_id,
            jour: self.jour,
            kind,
            remplacant_user_id: self.remplacant_user_id,
            nouvelle_salle: self.nouvelle_salle,
            motif: self.motif,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] edt sqlx error: {err}");
    DomainError::NotFound
}
