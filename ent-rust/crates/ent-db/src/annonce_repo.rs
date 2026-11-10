//! Persistance des annonces de l'établissement.
use chrono::{DateTime, Utc};
use domain_eleve::{Annonce, AudienceAnnonce, DomainError, StatutAnnonce};
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
                auteur_user_id, created_at, statut, classe_id,
                apercu_nb_eleves, apercu_nb_responsables
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
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
        .bind(a.statut.as_str())
        .bind(a.classe_id)
        .bind(a.apercu_nb_eleves)
        .bind(a.apercu_nb_responsables)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get(&self, etablissement_id: Uuid, id: Uuid) -> Result<Option<Annonce>, DomainError> {
        let row = sqlx::query_as::<_, AnnonceSql>(&format!(
            "SELECT {COLONNES} FROM ent_annonce WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(AnnonceSql::into_annonce))
    }

    /// Passe un aperçu à « publiée » (une seule fois : `false` si l'annonce n'est plus un aperçu).
    pub async fn confirmer(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
        par: Uuid,
    ) -> Result<bool, DomainError> {
        let r = sqlx::query(
            "UPDATE ent_annonce SET statut = 'publiee', confirmee_par = $3, confirmee_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'apercu'",
        )
        .bind(etablissement_id)
        .bind(id)
        .bind(par)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(r.rows_affected() == 1)
    }

    /// Retire une annonce publiée (`false` si elle ne l'est pas ou a déjà été retirée).
    pub async fn retirer(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
        par: Uuid,
        motif: &str,
    ) -> Result<bool, DomainError> {
        let r = sqlx::query(
            "UPDATE ent_annonce
                SET statut = 'retiree', retiree_par = $3, retiree_le = now(), retiree_motif = $4
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'publiee'",
        )
        .bind(etablissement_id)
        .bind(id)
        .bind(par)
        .bind(motif)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(r.rows_affected() == 1)
    }

    /// Annonces de l'établissement, les plus récentes d'abord (vue staff : aperçus compris).
    pub async fn list(&self, etablissement_id: Uuid, limit: i64) -> Result<Vec<Annonce>, DomainError> {
        let rows = sqlx::query_as::<_, AnnonceSql>(&format!(
            "SELECT {COLONNES} FROM ent_annonce
              WHERE etablissement_id = $1
              ORDER BY created_at DESC
              LIMIT $2"
        ))
        .bind(etablissement_id)
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(AnnonceSql::into_annonce).collect())
    }

    /// Annonces **publiées** visibles par les familles de ces classes (M0b, D18 : par
    /// identifiant de classe ; `classe_libelle` ne sert qu'aux anciennes annonces dont le libellé
    /// ne désignait aucune classe du référentiel).
    pub async fn list_visibles_famille(
        &self,
        etablissement_id: Uuid,
        classe_ids: &[Uuid],
        classe_libelle: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Annonce>, DomainError> {
        let rows = sqlx::query_as::<_, AnnonceSql>(&format!(
            "SELECT {COLONNES} FROM ent_annonce
              WHERE etablissement_id = $1 AND statut = 'publiee'
                AND (
                      audience IN ('tous', 'parents')
                   OR (audience = 'classe'
                       AND ((classe_id IS NOT NULL AND classe_id = ANY($2))
                            OR (classe_id IS NULL AND cible_classe IS NOT NULL
                                AND cible_classe = $3)))
                )
              ORDER BY created_at DESC
              LIMIT $4"
        ))
        .bind(etablissement_id)
        .bind(classe_ids)
        .bind(classe_libelle)
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(AnnonceSql::into_annonce).collect())
    }
}

const COLONNES: &str = "id, etablissement_id, titre, corps, audience, cible_classe, \
    auteur_user_id, created_at, statut, classe_id, apercu_nb_eleves, apercu_nb_responsables";

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
    statut: String,
    classe_id: Option<Uuid>,
    apercu_nb_eleves: Option<i32>,
    apercu_nb_responsables: Option<i32>,
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
            statut: StatutAnnonce::parse(&self.statut)?,
            classe_id: self.classe_id,
            apercu_nb_eleves: self.apercu_nb_eleves,
            apercu_nb_responsables: self.apercu_nb_responsables,
        })
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("annonce_repo", err)
}
