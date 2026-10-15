use chrono::{DateTime, Utc};
use domain_eleve::{DomainError, InscriptionEtapeKind, InscriptionSuivi};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgInscriptionRepo {
    pool: PgPool,
}

impl PgInscriptionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_suivi(&self, s: &InscriptionSuivi) -> Result<InscriptionSuivi, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            INSERT INTO ent_inscription_suivi (
                id, etablissement_id, eleve_id, annee_scolaire, etape, motif_code,
                commentaire, updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            ON CONFLICT (etablissement_id, eleve_id, annee_scolaire) DO UPDATE SET
                etape = EXCLUDED.etape,
                motif_code = EXCLUDED.motif_code,
                commentaire = EXCLUDED.commentaire,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at,
                id = ent_inscription_suivi.id
            RETURNING id, etablissement_id, eleve_id, annee_scolaire, etape, motif_code,
                      commentaire, updated_by_user_id, updated_at
            "#,
        )
        .bind(s.id)
        .bind(s.etablissement_id)
        .bind(s.eleve_id)
        .bind(&s.annee_scolaire)
        .bind(s.etape.as_str())
        .bind(&s.motif_code)
        .bind(&s.commentaire)
        .bind(s.updated_by_user_id)
        .bind(s.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        row.into_suivi().ok_or(DomainError::InscriptionSuiviInvalide)
    }

    pub async fn get_suivi(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<Option<InscriptionSuivi>, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, eleve_id, annee_scolaire, etape, motif_code,
                   commentaire, updated_by_user_id, updated_at
            FROM ent_inscription_suivi
            WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_scolaire = $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(annee_scolaire)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(Row::into_suivi))
    }

    pub async fn list_par_etape(
        &self,
        etablissement_id: Uuid,
        annee_scolaire: &str,
        etape: &str,
    ) -> Result<Vec<InscriptionSuivi>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, eleve_id, annee_scolaire, etape, motif_code,
                   commentaire, updated_by_user_id, updated_at
            FROM ent_inscription_suivi
            WHERE etablissement_id = $1 AND annee_scolaire = $2 AND etape = $3
            ORDER BY updated_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(annee_scolaire)
        .bind(etape)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(Row::into_suivi).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] inscription sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    annee_scolaire: String,
    etape: String,
    motif_code: Option<String>,
    commentaire: String,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl Row {
    fn into_suivi(self) -> Option<InscriptionSuivi> {
        Some(InscriptionSuivi {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            annee_scolaire: self.annee_scolaire,
            etape: InscriptionEtapeKind::parse(&self.etape)?,
            motif_code: self.motif_code,
            commentaire: self.commentaire,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        })
    }
}
