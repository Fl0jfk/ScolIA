use chrono::NaiveDate;
use domain_eleve::{
    Appreciation, Devoir, DomainError, NotePeriode, NotePeriodeStatut,
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Note {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub matiere_id: Uuid,
    pub valeur: f64,
    pub sur: f64,
    pub date_note: NaiveDate,
    pub auteur_user_id: Uuid,
    pub libelle: Option<String>,
    pub devoir_id: Option<Uuid>,
    pub periode_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct SaisirNote {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub matiere_id: Uuid,
    pub valeur: f64,
    pub sur: f64,
    pub date_note: NaiveDate,
    pub auteur_user_id: Uuid,
    pub libelle: Option<String>,
    pub devoir_id: Option<Uuid>,
    pub periode_id: Option<Uuid>,
}

pub struct PgNoteRepo {
    pool: PgPool,
}

impl PgNoteRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_periode(&self, p: &NotePeriode) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_note_periode (
                id, etablissement_id, code, libelle, date_debut, date_fin, statut
            ) VALUES ($1,$2,$3,$4,$5,$6,$7)
            "#,
        )
        .bind(p.id)
        .bind(p.etablissement_id)
        .bind(&p.code)
        .bind(&p.libelle)
        .bind(p.date_debut)
        .bind(p.date_fin)
        .bind(p.statut.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn set_periode_statut(
        &self,
        etablissement_id: Uuid,
        periode_id: Uuid,
        statut: NotePeriodeStatut,
    ) -> Result<(), DomainError> {
        let r = sqlx::query(
            r#"
            UPDATE ent_note_periode SET statut = $3
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(periode_id)
        .bind(statut.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        if r.rows_affected() == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }

    pub async fn get_periode(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
    ) -> Result<Option<NotePeriode>, DomainError> {
        let row = sqlx::query_as::<_, PeriodeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, date_debut, date_fin, statut
            FROM ent_note_periode
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(PeriodeRow::into_periode))
    }

    pub async fn list_periodes(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<NotePeriode>, DomainError> {
        let rows = sqlx::query_as::<_, PeriodeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, date_debut, date_fin, statut
            FROM ent_note_periode
            WHERE etablissement_id = $1
            ORDER BY date_debut
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(PeriodeRow::into_periode).collect())
    }

    pub async fn insert_devoir(&self, d: &Devoir) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_devoir (
                id, etablissement_id, periode_id, matiere_id, classe, titre,
                date_devoir, coef, sur, created_by_user_id
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(d.id)
        .bind(d.etablissement_id)
        .bind(d.periode_id)
        .bind(d.matiere_id)
        .bind(&d.classe)
        .bind(&d.titre)
        .bind(d.date_devoir)
        .bind(d.coef)
        .bind(d.sur)
        .bind(d.created_by_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_devoir(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Devoir>, DomainError> {
        let row = sqlx::query_as::<_, DevoirRow>(
            r#"
            SELECT id, etablissement_id, periode_id, matiere_id, classe, titre,
                   date_devoir, coef, sur, created_by_user_id
            FROM ent_devoir
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(DevoirRow::into_devoir))
    }

    pub async fn list_devoirs_periode(
        &self,
        etablissement_id: Uuid,
        periode_id: Uuid,
        classe: Option<&str>,
    ) -> Result<Vec<Devoir>, DomainError> {
        let rows = if let Some(c) = classe.map(str::trim).filter(|s| !s.is_empty()) {
            sqlx::query_as::<_, DevoirRow>(
                r#"
                SELECT id, etablissement_id, periode_id, matiere_id, classe, titre,
                       date_devoir, coef, sur, created_by_user_id
                FROM ent_devoir
                WHERE etablissement_id = $1 AND periode_id = $2
                  AND lower(classe) = lower($3)
                ORDER BY date_devoir DESC, titre
                "#,
            )
            .bind(etablissement_id)
            .bind(periode_id)
            .bind(c)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, DevoirRow>(
                r#"
                SELECT id, etablissement_id, periode_id, matiere_id, classe, titre,
                       date_devoir, coef, sur, created_by_user_id
                FROM ent_devoir
                WHERE etablissement_id = $1 AND periode_id = $2
                ORDER BY date_devoir DESC, titre
                "#,
            )
            .bind(etablissement_id)
            .bind(periode_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().map(DevoirRow::into_devoir).collect())
    }

    pub async fn count_notes_devoir(
        &self,
        etablissement_id: Uuid,
        devoir_id: Uuid,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint
            FROM ent_note
            WHERE etablissement_id = $1 AND devoir_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(devoir_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.0)
    }

    pub async fn delete_devoir(
        &self,
        etablissement_id: Uuid,
        devoir_id: Uuid,
    ) -> Result<bool, DomainError> {
        let r = sqlx::query(
            r#"
            DELETE FROM ent_devoir
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(devoir_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(r.rows_affected() > 0)
    }

    pub async fn insert(&self, n: &Note) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_note (
                id, etablissement_id, eleve_id, matiere_id, valeur, sur,
                date_note, auteur_user_id, libelle, devoir_id, periode_id
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(n.id)
        .bind(n.etablissement_id)
        .bind(n.eleve_id)
        .bind(n.matiere_id)
        .bind(n.valeur)
        .bind(n.sur)
        .bind(n.date_note)
        .bind(n.auteur_user_id)
        .bind(&n.libelle)
        .bind(n.devoir_id)
        .bind(n.periode_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_for_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<Note>, DomainError> {
        let rows = sqlx::query_as::<_, NoteRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, matiere_id, valeur, sur,
                   date_note, auteur_user_id, libelle, devoir_id, periode_id
            FROM ent_note
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY date_note DESC, created_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(NoteRow::into_note).collect())
    }

    /// Upsert appréciation (matière ou générale) — une seule par clé.
    pub async fn upsert_appreciation(
        &self,
        a: &Appreciation,
    ) -> Result<Appreciation, DomainError> {
        let updated = if let Some(matiere_id) = a.matiere_id {
            sqlx::query_as::<_, AppreciationRow>(
                r#"
                UPDATE ent_note_appreciation SET
                    texte = $5,
                    auteur_user_id = $6,
                    updated_at = now()
                WHERE etablissement_id = $1 AND periode_id = $2
                  AND eleve_id = $3 AND matiere_id = $4
                RETURNING id, etablissement_id, periode_id, eleve_id, matiere_id,
                          texte, auteur_user_id
                "#,
            )
            .bind(a.etablissement_id)
            .bind(a.periode_id)
            .bind(a.eleve_id)
            .bind(matiere_id)
            .bind(&a.texte)
            .bind(a.auteur_user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, AppreciationRow>(
                r#"
                UPDATE ent_note_appreciation SET
                    texte = $4,
                    auteur_user_id = $5,
                    updated_at = now()
                WHERE etablissement_id = $1 AND periode_id = $2
                  AND eleve_id = $3 AND matiere_id IS NULL
                RETURNING id, etablissement_id, periode_id, eleve_id, matiere_id,
                          texte, auteur_user_id
                "#,
            )
            .bind(a.etablissement_id)
            .bind(a.periode_id)
            .bind(a.eleve_id)
            .bind(&a.texte)
            .bind(a.auteur_user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        if let Some(row) = updated {
            return Ok(row.into_appreciation());
        }
        sqlx::query(
            r#"
            INSERT INTO ent_note_appreciation (
                id, etablissement_id, periode_id, eleve_id, matiere_id,
                texte, auteur_user_id, created_at, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7, now(), now())
            "#,
        )
        .bind(a.id)
        .bind(a.etablissement_id)
        .bind(a.periode_id)
        .bind(a.eleve_id)
        .bind(a.matiere_id)
        .bind(&a.texte)
        .bind(a.auteur_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(a.clone())
    }

    pub async fn list_appreciations_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        periode_id: Option<Uuid>,
    ) -> Result<Vec<Appreciation>, DomainError> {
        let rows = if let Some(pid) = periode_id {
            sqlx::query_as::<_, AppreciationRow>(
                r#"
                SELECT id, etablissement_id, periode_id, eleve_id, matiere_id,
                       texte, auteur_user_id
                FROM ent_note_appreciation
                WHERE etablissement_id = $1 AND eleve_id = $2 AND periode_id = $3
                ORDER BY matiere_id NULLS FIRST, updated_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .bind(pid)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, AppreciationRow>(
                r#"
                SELECT id, etablissement_id, periode_id, eleve_id, matiere_id,
                       texte, auteur_user_id
                FROM ent_note_appreciation
                WHERE etablissement_id = $1 AND eleve_id = $2
                ORDER BY periode_id, matiere_id NULLS FIRST, updated_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows
            .into_iter()
            .map(AppreciationRow::into_appreciation)
            .collect())
    }

    /// Notes d'un élève pour une période + matière (pour moyenne).
    pub async fn list_for_moyenne(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        periode_id: Uuid,
        matiere_id: Uuid,
    ) -> Result<Vec<(f64, f64, f64)>, DomainError> {
        let rows = sqlx::query_as::<_, MoyenneRow>(
            r#"
            SELECT n.valeur, n.sur, COALESCE(d.coef, 1.0) AS coef
            FROM ent_note n
            LEFT JOIN ent_devoir d
              ON d.id = n.devoir_id AND d.etablissement_id = n.etablissement_id
            WHERE n.etablissement_id = $1
              AND n.eleve_id = $2
              AND n.periode_id = $3
              AND n.matiere_id = $4
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(periode_id)
        .bind(matiere_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(|r| (r.valeur, r.sur, r.coef)).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PeriodeRow {
    id: Uuid,
    etablissement_id: Uuid,
    code: String,
    libelle: String,
    date_debut: NaiveDate,
    date_fin: NaiveDate,
    statut: String,
}

impl PeriodeRow {
    fn into_periode(self) -> NotePeriode {
        NotePeriode {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            libelle: self.libelle,
            date_debut: self.date_debut,
            date_fin: self.date_fin,
            statut: NotePeriodeStatut::parse(&self.statut)
                .unwrap_or(NotePeriodeStatut::Ouverte),
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DevoirRow {
    id: Uuid,
    etablissement_id: Uuid,
    periode_id: Uuid,
    matiere_id: Uuid,
    classe: String,
    titre: String,
    date_devoir: NaiveDate,
    coef: f64,
    sur: f64,
    created_by_user_id: Option<Uuid>,
}

impl DevoirRow {
    fn into_devoir(self) -> Devoir {
        Devoir {
            id: self.id,
            etablissement_id: self.etablissement_id,
            periode_id: self.periode_id,
            matiere_id: self.matiere_id,
            classe: self.classe,
            titre: self.titre,
            date_devoir: self.date_devoir,
            coef: self.coef,
            sur: self.sur,
            created_by_user_id: self.created_by_user_id,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct NoteRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    matiere_id: Uuid,
    valeur: f64,
    sur: f64,
    date_note: NaiveDate,
    auteur_user_id: Uuid,
    libelle: Option<String>,
    devoir_id: Option<Uuid>,
    periode_id: Option<Uuid>,
}

impl NoteRow {
    fn into_note(self) -> Note {
        Note {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            matiere_id: self.matiere_id,
            valeur: self.valeur,
            sur: self.sur,
            date_note: self.date_note,
            auteur_user_id: self.auteur_user_id,
            libelle: self.libelle,
            devoir_id: self.devoir_id,
            periode_id: self.periode_id,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct MoyenneRow {
    valeur: f64,
    sur: f64,
    coef: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct AppreciationRow {
    id: Uuid,
    etablissement_id: Uuid,
    periode_id: Uuid,
    eleve_id: Uuid,
    matiere_id: Option<Uuid>,
    texte: String,
    auteur_user_id: Uuid,
}

impl AppreciationRow {
    fn into_appreciation(self) -> Appreciation {
        Appreciation {
            id: self.id,
            etablissement_id: self.etablissement_id,
            periode_id: self.periode_id,
            eleve_id: self.eleve_id,
            matiere_id: self.matiere_id,
            texte: self.texte,
            auteur_user_id: self.auteur_user_id,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] note sqlx error: {err}");
    DomainError::NotFound
}
