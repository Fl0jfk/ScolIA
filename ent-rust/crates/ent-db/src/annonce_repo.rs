//! Persistance des annonces de l'établissement.
use chrono::{DateTime, Utc};
use domain_eleve::{Annonce, AudienceAnnonce, DomainError};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgAnnonceRepo {
    pool: PgPool,
}

impl PgAnnonceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, a: &Annonce) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_annonce (
                id, etablissement_id, titre, corps, audience, cible_classe,
                auteur_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(a.id)
        .bind(a.etablissement_id)
        .bind(&a.titre)
        .bind(&a.corps)
        .bind(a.audience.as_str())
        .bind(&a.cible_classe)
        .bind(a.auteur_user_id)
        .bind(a.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    /// Annonces de l'établissement, les plus récentes d'abord (vue staff).
    pub async fn list(&self, etablissement_id: Uuid, limit: i64) -> Result<Vec<Annonce>, DomainError> {
        let rows = sqlx::query_as::<_, AnnonceSql>(
            r#"
            SELECT id, etablissement_id, titre, corps, audience, cible_classe,
                   auteur_user_id, created_at
            FROM ent_annonce
            WHERE etablissement_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(etablissement_id)
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(AnnonceSql::into_annonce).collect())
    }

    /// Annonces visibles par les familles d'une classe (`None` = pas de classe).
    pub async fn list_visibles_famille(
        &self,
        etablissement_id: Uuid,
        classe: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Annonce>, DomainError> {
        let rows = sqlx::query_as::<_, AnnonceSql>(
            r#"
            SELECT id, etablissement_id, titre, corps, audience, cible_classe,
                   auteur_user_id, created_at
            FROM ent_annonce
            WHERE etablissement_id = $1
              AND (
                    audience IN ('tous', 'parents')
                 OR (audience = 'classe' AND cible_classe IS NOT NULL AND cible_classe = $2)
              )
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(classe)
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(AnnonceSql::into_annonce).collect())
    }
}

#[derive(sqlx::FromRow)]
struct AnnonceSql {
    id: Uuid,
    etablissement_id: Uuid,
    titre: String,
    corps: String,
    audience: String,
    cible_classe: Option<String>,
    auteur_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl AnnonceSql {
    fn into_annonce(self) -> Option<Annonce> {
        let audience = AudienceAnnonce::parse(&self.audience)?;
        Some(Annonce {
            id: self.id,
            etablissement_id: self.etablissement_id,
            titre: self.titre,
            corps: self.corps,
            audience,
            cible_classe: self.cible_classe,
            auteur_user_id: self.auteur_user_id,
            created_at: self.created_at,
        })
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("annonce_repo", err)
}
