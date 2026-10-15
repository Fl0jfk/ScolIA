use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{DocASigner, DocSignerStatut, DomainError};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgDocSignerRepo {
    pool: PgPool,
}

impl PgDocSignerRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, d: &DocASigner) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_doc_a_signer (
                id, etablissement_id, eleve_id, code, titre, corps, annee_scolaire,
                statut, publie_par_user_id, publie_at, echeance, signe_at, signe_par_user_id
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            "#,
        )
        .bind(d.id)
        .bind(d.etablissement_id)
        .bind(d.eleve_id)
        .bind(&d.code)
        .bind(&d.titre)
        .bind(&d.corps)
        .bind(&d.annee_scolaire)
        .bind(d.statut.as_str())
        .bind(d.publie_par_user_id)
        .bind(d.publie_at)
        .bind(d.echeance)
        .bind(d.signe_at)
        .bind(d.signe_par_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn update_signature(&self, d: &DocASigner) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_doc_a_signer
            SET statut = $3, signe_at = $4, signe_par_user_id = $5
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(d.etablissement_id)
        .bind(d.id)
        .bind(d.statut.as_str())
        .bind(d.signe_at)
        .bind(d.signe_par_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn update_statut(&self, d: &DocASigner) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_doc_a_signer SET statut = $3
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(d.etablissement_id)
        .bind(d.id)
        .bind(d.statut.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        doc_id: Uuid,
    ) -> Result<Option<DocASigner>, DomainError> {
        let row = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, eleve_id, code, titre, corps, annee_scolaire,
                   statut, publie_par_user_id, publie_at, echeance, signe_at, signe_par_user_id
            FROM ent_doc_a_signer
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(doc_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(Row::into_doc))
    }

    pub async fn list_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        a_signer_seulement: bool,
    ) -> Result<Vec<DocASigner>, DomainError> {
        let rows = if a_signer_seulement {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, eleve_id, code, titre, corps, annee_scolaire,
                       statut, publie_par_user_id, publie_at, echeance, signe_at, signe_par_user_id
                FROM ent_doc_a_signer
                WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'a_signer'
                ORDER BY publie_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, Row>(
                r#"
                SELECT id, etablissement_id, eleve_id, code, titre, corps, annee_scolaire,
                       statut, publie_par_user_id, publie_at, echeance, signe_at, signe_par_user_id
                FROM ent_doc_a_signer
                WHERE etablissement_id = $1 AND eleve_id = $2 AND statut <> 'annule'
                ORDER BY publie_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().filter_map(Row::into_doc).collect())
    }

    pub async fn list_a_signer_etab(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<DocASigner>, DomainError> {
        let rows = sqlx::query_as::<_, Row>(
            r#"
            SELECT id, etablissement_id, eleve_id, code, titre, corps, annee_scolaire,
                   statut, publie_par_user_id, publie_at, echeance, signe_at, signe_par_user_id
            FROM ent_doc_a_signer
            WHERE etablissement_id = $1 AND statut = 'a_signer'
            ORDER BY publie_at DESC
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(Row::into_doc).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] doc_signer sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    code: String,
    titre: String,
    corps: String,
    annee_scolaire: String,
    statut: String,
    publie_par_user_id: Uuid,
    publie_at: DateTime<Utc>,
    echeance: Option<NaiveDate>,
    signe_at: Option<DateTime<Utc>>,
    signe_par_user_id: Option<Uuid>,
}

impl Row {
    fn into_doc(self) -> Option<DocASigner> {
        Some(DocASigner {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            code: self.code,
            titre: self.titre,
            corps: self.corps,
            annee_scolaire: self.annee_scolaire,
            statut: DocSignerStatut::parse(&self.statut)?,
            publie_par_user_id: self.publie_par_user_id,
            publie_at: self.publie_at,
            echeance: self.echeance,
            signe_at: self.signe_at,
            signe_par_user_id: self.signe_par_user_id,
        })
    }
}
