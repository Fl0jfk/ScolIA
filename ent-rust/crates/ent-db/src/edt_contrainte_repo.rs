use domain_eleve::{DomainError, EdtContrainte, EdtContrainteKind};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgEdtContrainteRepo {
    pool: PgPool,
}

impl PgEdtContrainteRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, c: &EdtContrainte) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_edt_contrainte (
                id, etablissement_id, kind, prof_user_id, salle_code,
                jour_semaine, heure_debut, heure_fin, detail,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(c.id)
        .bind(c.etablissement_id)
        .bind(c.kind.as_str())
        .bind(c.prof_user_id)
        .bind(c.salle_code.as_deref())
        .bind(c.jour_semaine)
        .bind(c.heure_debut)
        .bind(c.heure_fin)
        .bind(&c.detail)
        .bind(c.created_by_user_id)
        .bind(c.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<EdtContrainte>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, kind, prof_user_id, salle_code,
                   jour_semaine, heure_debut, heure_fin, detail,
                   created_by_user_id, created_at
            FROM ent_edt_contrainte
            WHERE etablissement_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(Row::into_contrainte).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] edt_contrainte sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    kind: String,
    prof_user_id: Option<Uuid>,
    salle_code: Option<String>,
    jour_semaine: Option<i16>,
    heure_debut: Option<chrono::NaiveTime>,
    heure_fin: Option<chrono::NaiveTime>,
    detail: String,
    created_by_user_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl Row {
    fn into_contrainte(self) -> Option<EdtContrainte> {
        let kind = EdtContrainteKind::parse(&self.kind)?;
        Some(EdtContrainte {
            id: self.id,
            etablissement_id: self.etablissement_id,
            kind,
            prof_user_id: self.prof_user_id,
            salle_code: self.salle_code,
            jour_semaine: self.jour_semaine,
            heure_debut: self.heure_debut,
            heure_fin: self.heure_fin,
            detail: self.detail,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}
