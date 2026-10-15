use domain_eleve::{ConventionStage, DomainError, Entreprise};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgStageRepo {
    pool: PgPool,
}

impl PgStageRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_entreprise(&self, e: &Entreprise) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_entreprise (
                id, etablissement_id, raison_sociale, siret, adresse,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7)
            "#,
        )
        .bind(e.id)
        .bind(e.etablissement_id)
        .bind(&e.raison_sociale)
        .bind(e.siret.as_deref())
        .bind(&e.adresse)
        .bind(e.created_by_user_id)
        .bind(e.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_entreprises(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<Entreprise>, DomainError> {
        let rows = sqlx::query_as::<_, EntRow>(
            r#"
            SELECT id, etablissement_id, raison_sociale, siret, adresse,
                   created_by_user_id, created_at
            FROM ent_entreprise
            WHERE etablissement_id = $1
            ORDER BY raison_sociale
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(EntRow::into_entreprise).collect())
    }

    pub async fn entreprise_labels(
        &self,
        etablissement_id: Uuid,
        entreprise_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, DomainError> {
        if entreprise_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            r#"
            SELECT id, raison_sociale
            FROM ent_entreprise
            WHERE etablissement_id = $1 AND id = ANY($2)
            "#,
        )
        .bind(etablissement_id)
        .bind(entreprise_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().collect())
    }

    pub async fn get_entreprise(
        &self,
        etablissement_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Entreprise>, DomainError> {
        let row = sqlx::query_as::<_, EntRow>(
            r#"
            SELECT id, etablissement_id, raison_sociale, siret, adresse,
                   created_by_user_id, created_at
            FROM ent_entreprise
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(EntRow::into_entreprise))
    }

    pub async fn insert_convention(&self, c: &ConventionStage) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_convention_stage (
                id, etablissement_id, eleve_id, entreprise_id, debut, fin,
                tuteur_nom, statut, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(c.id)
        .bind(c.etablissement_id)
        .bind(c.eleve_id)
        .bind(c.entreprise_id)
        .bind(c.debut)
        .bind(c.fin)
        .bind(&c.tuteur_nom)
        .bind(&c.statut)
        .bind(c.created_by_user_id)
        .bind(c.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_conventions_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<ConventionStage>, DomainError> {
        let rows = sqlx::query_as::<_, ConvRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, entreprise_id, debut, fin,
                   tuteur_nom, statut, created_by_user_id, created_at
            FROM ent_convention_stage
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY debut DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(ConvRow::into_convention).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] stage sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct EntRow {
    id: Uuid,
    etablissement_id: Uuid,
    raison_sociale: String,
    siret: Option<String>,
    adresse: String,
    created_by_user_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl EntRow {
    fn into_entreprise(self) -> Entreprise {
        Entreprise {
            id: self.id,
            etablissement_id: self.etablissement_id,
            raison_sociale: self.raison_sociale,
            siret: self.siret,
            adresse: self.adresse,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ConvRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    entreprise_id: Uuid,
    debut: chrono::NaiveDate,
    fin: chrono::NaiveDate,
    tuteur_nom: String,
    statut: String,
    created_by_user_id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl ConvRow {
    fn into_convention(self) -> ConventionStage {
        ConventionStage {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            entreprise_id: self.entreprise_id,
            debut: self.debut,
            fin: self.fin,
            tuteur_nom: self.tuteur_nom,
            statut: self.statut,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}
