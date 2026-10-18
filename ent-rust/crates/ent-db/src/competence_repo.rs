//! Persistance compétences / LSU (`ent_competence_item`, `ent_competence_evaluation`).
use chrono::{DateTime, Utc};
use domain_eleve::{CompetenceItem, DomainError, EvaluationCompetence, NiveauMaitrise};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgCompetenceRepo {
    pool: PgPool,
}

/// Ligne d'évaluation enrichie du libellé d'item (pour livrets / vues famille).
#[derive(Debug, Clone)]
pub struct EvaluationLue {
    pub evaluation_id: Uuid,
    pub item_id: Uuid,
    pub domaine: String,
    pub item_libelle: String,
    pub item_code: String,
    pub ordre: i32,
    pub niveau: NiveauMaitrise,
    pub commentaire: Option<String>,
    pub maj_at: DateTime<Utc>,
}

impl PgCompetenceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Crée ou met à jour un item du référentiel (clé : établissement + code).
    pub async fn upsert_item(&self, item: &CompetenceItem) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_competence_item (
                id, etablissement_id, domaine, code, libelle, ordre, actif, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7, now())
            ON CONFLICT (etablissement_id, code) DO UPDATE
                SET domaine = EXCLUDED.domaine,
                    libelle = EXCLUDED.libelle,
                    ordre = EXCLUDED.ordre
            "#,
        )
        .bind(item.id)
        .bind(item.etablissement_id)
        .bind(&item.domaine)
        .bind(&item.code)
        .bind(&item.libelle)
        .bind(item.ordre)
        .bind(item.actif)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_items(
        &self,
        etablissement_id: Uuid,
        inclure_inactifs: bool,
    ) -> Result<Vec<CompetenceItem>, DomainError> {
        let rows = sqlx::query_as::<_, ItemRow>(
            r#"
            SELECT id, etablissement_id, domaine, code, libelle, ordre, actif
            FROM ent_competence_item
            WHERE etablissement_id = $1
              AND ($2 OR actif)
            ORDER BY domaine, ordre, code
            "#,
        )
        .bind(etablissement_id)
        .bind(inclure_inactifs)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(ItemRow::into_item).collect())
    }

    pub async fn get_item(
        &self,
        etablissement_id: Uuid,
        item_id: Uuid,
    ) -> Result<Option<CompetenceItem>, DomainError> {
        let row = sqlx::query_as::<_, ItemRow>(
            r#"
            SELECT id, etablissement_id, domaine, code, libelle, ordre, actif
            FROM ent_competence_item
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(ItemRow::into_item))
    }

    /// Évalue (ou ré-évalue) un item pour un élève sur une période.
    pub async fn upsert_evaluation(&self, e: &EvaluationCompetence) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_competence_evaluation (
                id, etablissement_id, eleve_id, item_id, periode_id,
                niveau, commentaire, auteur_user_id, maj_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now())
            ON CONFLICT (etablissement_id, eleve_id, item_id, periode_id) DO UPDATE
                SET niveau = EXCLUDED.niveau,
                    commentaire = EXCLUDED.commentaire,
                    auteur_user_id = EXCLUDED.auteur_user_id,
                    maj_at = now()
            "#,
        )
        .bind(e.id)
        .bind(e.etablissement_id)
        .bind(e.eleve_id)
        .bind(e.item_id)
        .bind(e.periode_id)
        .bind(e.niveau.as_str())
        .bind(&e.commentaire)
        .bind(e.auteur_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    /// Évaluations d'un élève sur une période, enrichies du libellé d'item.
    pub async fn list_evaluations_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        periode_id: Option<Uuid>,
    ) -> Result<Vec<EvaluationLue>, DomainError> {
        let rows = sqlx::query_as::<_, EvalRow>(
            r#"
            SELECT e.id AS evaluation_id, e.item_id, i.domaine, i.libelle AS item_libelle,
                   i.code AS item_code, i.ordre, e.niveau, e.commentaire, e.maj_at
            FROM ent_competence_evaluation e
            JOIN ent_competence_item i ON i.id = e.item_id
            WHERE e.etablissement_id = $1
              AND e.eleve_id = $2
              AND ($3::uuid IS NULL OR e.periode_id = $3)
            ORDER BY i.domaine, i.ordre, i.code
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(periode_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        rows.into_iter().map(EvalRow::into_lue).collect()
    }

    /// Évaluations de tous les élèves d'une classe sur une période (export livret).
    pub async fn list_evaluations_classe(
        &self,
        etablissement_id: Uuid,
        eleve_ids: &[Uuid],
        periode_id: Uuid,
    ) -> Result<Vec<(Uuid, EvaluationLue)>, DomainError> {
        if eleve_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, EvalEleveRow>(
            r#"
            SELECT e.eleve_id, e.id AS evaluation_id, e.item_id, i.domaine,
                   i.libelle AS item_libelle, i.code AS item_code, i.ordre,
                   e.niveau, e.commentaire, e.maj_at
            FROM ent_competence_evaluation e
            JOIN ent_competence_item i ON i.id = e.item_id
            WHERE e.etablissement_id = $1
              AND e.periode_id = $2
              AND e.eleve_id = ANY($3)
            ORDER BY e.eleve_id, i.domaine, i.ordre, i.code
            "#,
        )
        .bind(etablissement_id)
        .bind(periode_id)
        .bind(eleve_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        rows.into_iter()
            .map(|r| {
                let eleve_id = r.eleve_id;
                Ok((eleve_id, r.into_lue()?))
            })
            .collect()
    }
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: Uuid,
    etablissement_id: Uuid,
    domaine: String,
    code: String,
    libelle: String,
    ordre: i32,
    actif: bool,
}

impl ItemRow {
    fn into_item(self) -> CompetenceItem {
        CompetenceItem {
            id: self.id,
            etablissement_id: self.etablissement_id,
            domaine: self.domaine,
            code: self.code,
            libelle: self.libelle,
            ordre: self.ordre,
            actif: self.actif,
        }
    }
}

#[derive(sqlx::FromRow)]
struct EvalEleveRow {
    eleve_id: Uuid,
    evaluation_id: Uuid,
    item_id: Uuid,
    domaine: String,
    item_libelle: String,
    item_code: String,
    ordre: i32,
    niveau: String,
    commentaire: Option<String>,
    maj_at: DateTime<Utc>,
}

impl EvalEleveRow {
    fn into_lue(self) -> Result<EvaluationLue, DomainError> {
        Ok(EvaluationLue {
            evaluation_id: self.evaluation_id,
            item_id: self.item_id,
            domaine: self.domaine,
            item_libelle: self.item_libelle,
            item_code: self.item_code,
            ordre: self.ordre,
            niveau: niveau_from_db(&self.niveau)?,
            commentaire: self.commentaire,
            maj_at: self.maj_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct EvalRow {
    evaluation_id: Uuid,
    item_id: Uuid,
    domaine: String,
    item_libelle: String,
    item_code: String,
    ordre: i32,
    niveau: String,
    commentaire: Option<String>,
    maj_at: DateTime<Utc>,
}

impl EvalRow {
    fn into_lue(self) -> Result<EvaluationLue, DomainError> {
        Ok(EvaluationLue {
            evaluation_id: self.evaluation_id,
            item_id: self.item_id,
            domaine: self.domaine,
            item_libelle: self.item_libelle,
            item_code: self.item_code,
            ordre: self.ordre,
            niveau: niveau_from_db(&self.niveau)?,
            commentaire: self.commentaire,
            maj_at: self.maj_at,
        })
    }
}

fn niveau_from_db(s: &str) -> Result<NiveauMaitrise, DomainError> {
    NiveauMaitrise::parse(s).ok_or(DomainError::CompetenceInvalide)
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("competence_repo", err)
}
