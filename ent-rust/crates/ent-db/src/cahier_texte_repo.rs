//! Cahier de texte — persistance.

use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{CahierTexteEntree, DomainError};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgCahierTexteRepo {
    pool: PgPool,
}

impl PgCahierTexteRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, e: &CahierTexteEntree) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_cahier_texte (
                id, etablissement_id, classe, matiere_libelle, titre, consigne,
                pour_le, cree_le, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(e.id)
        .bind(e.etablissement_id)
        .bind(&e.classe)
        .bind(&e.matiere_libelle)
        .bind(&e.titre)
        .bind(&e.consigne)
        .bind(e.pour_le)
        .bind(e.cree_le)
        .bind(e.created_by_user_id)
        .bind(e.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_classe(
        &self,
        etablissement_id: Uuid,
        classe: &str,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
    ) -> Result<Vec<CahierTexteEntree>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, classe, matiere_libelle, titre, consigne,
                   pour_le, cree_le, created_by_user_id, created_at
            FROM ent_cahier_texte
            WHERE etablissement_id = $1
              AND classe = $2
              AND ($3::date IS NULL OR pour_le >= $3)
              AND ($4::date IS NULL OR pour_le <= $4)
            ORDER BY pour_le ASC, created_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(classe)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Row::into_entree).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    classe: String,
    matiere_libelle: String,
    titre: String,
    consigne: String,
    pour_le: NaiveDate,
    cree_le: NaiveDate,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl Row {
    fn into_entree(self) -> CahierTexteEntree {
        CahierTexteEntree {
            id: self.id,
            etablissement_id: self.etablissement_id,
            classe: self.classe,
            matiere_libelle: self.matiere_libelle,
            titre: self.titre,
            consigne: self.consigne,
            pour_le: self.pour_le,
            cree_le: self.cree_le,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("cahier_texte_repo", err)
}
