use chrono::{DateTime, Utc};
use domain_eleve::{DomainError, Filiere, VoeuInscription};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgFiliereRepo {
    pool: PgPool,
}

impl PgFiliereRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_filiere(&self, f: &Filiere) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_filiere (
                id, etablissement_id, code, libelle, mef, actif,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(f.id)
        .bind(f.etablissement_id)
        .bind(&f.code)
        .bind(&f.libelle)
        .bind(&f.mef)
        .bind(f.actif)
        .bind(f.created_by_user_id)
        .bind(f.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_filiere(
        &self,
        etablissement_id: Uuid,
        filiere_id: Uuid,
    ) -> Result<Option<Filiere>, DomainError> {
        let row = sqlx::query_as::<_, FiliereRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, mef, actif,
                   created_by_user_id, created_at
            FROM ent_filiere
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(filiere_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(FiliereRow::into_filiere))
    }

    pub async fn list_filieres(
        &self,
        etablissement_id: Uuid,
        actifs_seulement: bool,
    ) -> Result<Vec<Filiere>, DomainError> {
        let rows = if actifs_seulement {
            sqlx::query_as::<_, FiliereRow>(
                r#"
                SELECT id, etablissement_id, code, libelle, mef, actif,
                       created_by_user_id, created_at
                FROM ent_filiere
                WHERE etablissement_id = $1 AND actif = true
                ORDER BY code
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, FiliereRow>(
                r#"
                SELECT id, etablissement_id, code, libelle, mef, actif,
                       created_by_user_id, created_at
                FROM ent_filiere
                WHERE etablissement_id = $1
                ORDER BY code
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().map(FiliereRow::into_filiere).collect())
    }

    pub async fn upsert_voeu(&self, v: &VoeuInscription) -> Result<VoeuInscription, DomainError> {
        let row = sqlx::query_as::<_, VoeuRow>(
            r#"
            INSERT INTO ent_voeu_inscription (
                id, etablissement_id, eleve_id, filiere_id, rang, annee_scolaire,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            ON CONFLICT (etablissement_id, eleve_id, annee_scolaire, rang) DO UPDATE SET
                filiere_id = EXCLUDED.filiere_id,
                created_by_user_id = EXCLUDED.created_by_user_id,
                created_at = EXCLUDED.created_at,
                id = ent_voeu_inscription.id
            RETURNING id, etablissement_id, eleve_id, filiere_id, rang, annee_scolaire,
                      created_by_user_id, created_at
            "#,
        )
        .bind(v.id)
        .bind(v.etablissement_id)
        .bind(v.eleve_id)
        .bind(v.filiere_id)
        .bind(v.rang)
        .bind(&v.annee_scolaire)
        .bind(v.created_by_user_id)
        .bind(v.created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.into_voeu())
    }

    pub async fn list_voeux_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<Vec<VoeuInscription>, DomainError> {
        let rows = sqlx::query_as::<_, VoeuRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, filiere_id, rang, annee_scolaire,
                   created_by_user_id, created_at
            FROM ent_voeu_inscription
            WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_scolaire = $3
            ORDER BY rang
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(annee_scolaire)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(VoeuRow::into_voeu).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] filiere sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct FiliereRow {
    id: Uuid,
    etablissement_id: Uuid,
    code: String,
    libelle: String,
    mef: Option<String>,
    actif: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl FiliereRow {
    fn into_filiere(self) -> Filiere {
        Filiere {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            libelle: self.libelle,
            mef: self.mef,
            actif: self.actif,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct VoeuRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    filiere_id: Uuid,
    rang: i16,
    annee_scolaire: String,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl VoeuRow {
    fn into_voeu(self) -> VoeuInscription {
        VoeuInscription {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            filiere_id: self.filiere_id,
            rang: self.rang,
            annee_scolaire: self.annee_scolaire,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}
