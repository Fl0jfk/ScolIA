use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use domain_eleve::{
    ConseilClasse, ConseilClasseDossier, ConseilClasseStatut, DomainError, MentionConseil,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgConseilClasseRepo {
    pool: PgPool,
}

impl PgConseilClasseRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_conseil(&self, c: &ConseilClasse) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_conseil_classe (
                id, etablissement_id, classe, periode_libelle, jour, heure, lieu,
                statut, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(c.id)
        .bind(c.etablissement_id)
        .bind(&c.classe)
        .bind(&c.periode_libelle)
        .bind(c.jour)
        .bind(c.heure)
        .bind(&c.lieu)
        .bind(c.statut.as_str())
        .bind(c.created_by_user_id)
        .bind(c.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn update_statut(&self, c: &ConseilClasse) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_conseil_classe SET statut = $3
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(c.etablissement_id)
        .bind(c.id)
        .bind(c.statut.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_conseil(
        &self,
        etablissement_id: Uuid,
        conseil_id: Uuid,
    ) -> Result<Option<ConseilClasse>, DomainError> {
        let row = sqlx::query_as::<_, ConseilRow>(
            r#"
            SELECT id, etablissement_id, classe, periode_libelle, jour, heure, lieu,
                   statut, created_by_user_id, created_at
            FROM ent_conseil_classe
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(conseil_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(ConseilRow::into_conseil))
    }

    pub async fn list_conseils(
        &self,
        etablissement_id: Uuid,
        classe: Option<&str>,
    ) -> Result<Vec<ConseilClasse>, DomainError> {
        let rows = if let Some(cl) = classe {
            sqlx::query_as::<_, ConseilRow>(
                r#"
                SELECT id, etablissement_id, classe, periode_libelle, jour, heure, lieu,
                       statut, created_by_user_id, created_at
                FROM ent_conseil_classe
                WHERE etablissement_id = $1 AND classe = $2
                ORDER BY jour DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(cl)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, ConseilRow>(
                r#"
                SELECT id, etablissement_id, classe, periode_libelle, jour, heure, lieu,
                       statut, created_by_user_id, created_at
                FROM ent_conseil_classe
                WHERE etablissement_id = $1
                ORDER BY jour DESC
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().filter_map(ConseilRow::into_conseil).collect())
    }

    pub async fn insert_dossier(&self, d: &ConseilClasseDossier) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_conseil_classe_dossier (
                id, etablissement_id, conseil_id, eleve_id, appreciation, mention,
                decision_texte, visible_famille, decided_at, decided_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(d.id)
        .bind(d.etablissement_id)
        .bind(d.conseil_id)
        .bind(d.eleve_id)
        .bind(&d.appreciation)
        .bind(d.mention.map(|m| m.as_str()))
        .bind(&d.decision_texte)
        .bind(d.visible_famille)
        .bind(d.decided_at)
        .bind(d.decided_by_user_id)
        .bind(d.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn update_dossier(&self, d: &ConseilClasseDossier) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_conseil_classe_dossier
            SET appreciation = $3, mention = $4, decision_texte = $5,
                visible_famille = $6, decided_at = $7, decided_by_user_id = $8
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(d.etablissement_id)
        .bind(d.id)
        .bind(&d.appreciation)
        .bind(d.mention.map(|m| m.as_str()))
        .bind(&d.decision_texte)
        .bind(d.visible_famille)
        .bind(d.decided_at)
        .bind(d.decided_by_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_dossier(
        &self,
        etablissement_id: Uuid,
        dossier_id: Uuid,
    ) -> Result<Option<ConseilClasseDossier>, DomainError> {
        let row = sqlx::query_as::<_, DossierRow>(
            r#"
            SELECT id, etablissement_id, conseil_id, eleve_id, appreciation, mention,
                   decision_texte, visible_famille, decided_at, decided_by_user_id, created_at
            FROM ent_conseil_classe_dossier
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(dossier_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(DossierRow::into_dossier))
    }

    pub async fn list_dossiers_conseil(
        &self,
        etablissement_id: Uuid,
        conseil_id: Uuid,
    ) -> Result<Vec<ConseilClasseDossier>, DomainError> {
        let rows = sqlx::query_as::<_, DossierRow>(
            r#"
            SELECT id, etablissement_id, conseil_id, eleve_id, appreciation, mention,
                   decision_texte, visible_famille, decided_at, decided_by_user_id, created_at
            FROM ent_conseil_classe_dossier
            WHERE etablissement_id = $1 AND conseil_id = $2
            ORDER BY created_at
            "#,
        )
        .bind(etablissement_id)
        .bind(conseil_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(DossierRow::into_dossier).collect())
    }

    pub async fn list_dossiers_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        famille_seulement: bool,
    ) -> Result<Vec<ConseilClasseDossier>, DomainError> {
        let rows = if famille_seulement {
            sqlx::query_as::<_, DossierRow>(
                r#"
                SELECT id, etablissement_id, conseil_id, eleve_id, appreciation, mention,
                       decision_texte, visible_famille, decided_at, decided_by_user_id, created_at
                FROM ent_conseil_classe_dossier
                WHERE etablissement_id = $1 AND eleve_id = $2
                  AND visible_famille = true AND decided_at IS NOT NULL
                ORDER BY decided_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, DossierRow>(
                r#"
                SELECT id, etablissement_id, conseil_id, eleve_id, appreciation, mention,
                       decision_texte, visible_famille, decided_at, decided_by_user_id, created_at
                FROM ent_conseil_classe_dossier
                WHERE etablissement_id = $1 AND eleve_id = $2
                ORDER BY created_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().filter_map(DossierRow::into_dossier).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] conseil_classe sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct ConseilRow {
    id: Uuid,
    etablissement_id: Uuid,
    classe: String,
    periode_libelle: String,
    jour: NaiveDate,
    heure: NaiveTime,
    lieu: String,
    statut: String,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl ConseilRow {
    fn into_conseil(self) -> Option<ConseilClasse> {
        Some(ConseilClasse {
            id: self.id,
            etablissement_id: self.etablissement_id,
            classe: self.classe,
            periode_libelle: self.periode_libelle,
            jour: self.jour,
            heure: self.heure,
            lieu: self.lieu,
            statut: ConseilClasseStatut::parse(&self.statut)?,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct DossierRow {
    id: Uuid,
    etablissement_id: Uuid,
    conseil_id: Uuid,
    eleve_id: Uuid,
    appreciation: String,
    mention: Option<String>,
    decision_texte: String,
    visible_famille: bool,
    decided_at: Option<DateTime<Utc>>,
    decided_by_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
}

impl DossierRow {
    fn into_dossier(self) -> Option<ConseilClasseDossier> {
        let mention = match self.mention {
            None => None,
            Some(s) => Some(MentionConseil::parse(&s)?),
        };
        Some(ConseilClasseDossier {
            id: self.id,
            etablissement_id: self.etablissement_id,
            conseil_id: self.conseil_id,
            eleve_id: self.eleve_id,
            appreciation: self.appreciation,
            mention,
            decision_texte: self.decision_texte,
            visible_famille: self.visible_famille,
            decided_at: self.decided_at,
            decided_by_user_id: self.decided_by_user_id,
            created_at: self.created_at,
        })
    }
}
