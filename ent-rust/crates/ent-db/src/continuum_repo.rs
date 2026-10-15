use domain_eleve::{
    DocTiroir, DomainError, ExigenceDocReinscription, OptionAnnee, OptionAnneeStatut, OptionNature,
    ReinscriptionCampagne, ScolariteAnnee, ScolariteAnneeStatut, ScolariteSource,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgContinuumRepo {
    pool: PgPool,
}

impl PgContinuumRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_campagne(
        &self,
        c: &ReinscriptionCampagne,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_reinscription_campagne (
                id, etablissement_id, annee_cible, ouverte, acompte_centimes,
                ouverte_le, created_at
            ) VALUES (
                $1,$2,$3,$4,$5,
                CASE WHEN $4 THEN now() ELSE NULL END,
                now()
            )
            ON CONFLICT (etablissement_id, annee_cible) DO UPDATE SET
                ouverte = EXCLUDED.ouverte,
                acompte_centimes = EXCLUDED.acompte_centimes,
                ouverte_le = CASE
                    WHEN EXCLUDED.ouverte AND NOT ent_reinscription_campagne.ouverte
                    THEN now()
                    ELSE ent_reinscription_campagne.ouverte_le
                END,
                fermee_le = CASE
                    WHEN NOT EXCLUDED.ouverte AND ent_reinscription_campagne.ouverte
                    THEN now()
                    ELSE ent_reinscription_campagne.fermee_le
                END,
                id = ent_reinscription_campagne.id
            "#,
        )
        .bind(c.id)
        .bind(c.etablissement_id)
        .bind(&c.annee_cible)
        .bind(c.ouverte)
        .bind(c.acompte_centimes)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_campagne(
        &self,
        etablissement_id: Uuid,
        annee_cible: &str,
    ) -> Result<Option<ReinscriptionCampagne>, DomainError> {
        let row = sqlx::query_as::<_, CampagneRow>(
            r#"
            SELECT id, etablissement_id, annee_cible, ouverte, acompte_centimes
            FROM ent_reinscription_campagne
            WHERE etablissement_id = $1 AND annee_cible = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(annee_cible)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(CampagneRow::into_campagne))
    }

    pub async fn upsert_scolarite(&self, s: &ScolariteAnnee) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_eleve_scolarite_annee (
                id, etablissement_id, eleve_id, annee_scolaire,
                niveau, mef, classe, regime, statut, source,
                passage_juridique, redoublement_possible, date_montee,
                created_at, updated_at
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13, now(), now()
            )
            ON CONFLICT (etablissement_id, eleve_id, annee_scolaire) DO UPDATE SET
                niveau = EXCLUDED.niveau,
                mef = EXCLUDED.mef,
                classe = EXCLUDED.classe,
                regime = EXCLUDED.regime,
                statut = EXCLUDED.statut,
                source = EXCLUDED.source,
                passage_juridique = EXCLUDED.passage_juridique,
                redoublement_possible = EXCLUDED.redoublement_possible,
                date_montee = COALESCE(EXCLUDED.date_montee, ent_eleve_scolarite_annee.date_montee),
                updated_at = now(),
                id = ent_eleve_scolarite_annee.id
            "#,
        )
        .bind(s.id)
        .bind(s.etablissement_id)
        .bind(s.eleve_id)
        .bind(&s.annee_scolaire)
        .bind(&s.niveau)
        .bind(&s.mef)
        .bind(&s.classe)
        .bind(&s.regime)
        .bind(s.statut.as_str())
        .bind(s.source.as_str())
        .bind(s.passage_juridique)
        .bind(s.redoublement_possible)
        .bind(s.date_montee)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_scolarite(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<Option<ScolariteAnnee>, DomainError> {
        let row = sqlx::query_as::<_, ScolariteRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, annee_scolaire,
                   niveau, mef, classe, regime, statut, source,
                   passage_juridique, redoublement_possible, date_montee
            FROM ent_eleve_scolarite_annee
            WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_scolaire = $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(annee_scolaire)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(ScolariteRow::into_scol))
    }

    pub async fn list_scolarites_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<ScolariteAnnee>, DomainError> {
        let rows = sqlx::query_as::<_, ScolariteRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, annee_scolaire,
                   niveau, mef, classe, regime, statut, source,
                   passage_juridique, redoublement_possible, date_montee
            FROM ent_eleve_scolarite_annee
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY annee_scolaire
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(ScolariteRow::into_scol).collect())
    }

    /// Toutes les scolarités d'une année scolaire pour un établissement (lot montée).
    pub async fn list_scolarites_annee(
        &self,
        etablissement_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<Vec<ScolariteAnnee>, DomainError> {
        let rows = sqlx::query_as::<_, ScolariteRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, annee_scolaire,
                   niveau, mef, classe, regime, statut, source,
                   passage_juridique, redoublement_possible, date_montee
            FROM ent_eleve_scolarite_annee
            WHERE etablissement_id = $1 AND annee_scolaire = $2
            ORDER BY niveau NULLS LAST, classe NULLS LAST, eleve_id
            "#,
        )
        .bind(etablissement_id)
        .bind(annee_scolaire)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(ScolariteRow::into_scol).collect())
    }

    pub async fn list_scolarites_continuum(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        annees: &[&str],
    ) -> Result<Vec<ScolariteAnnee>, DomainError> {
        let all = self.list_scolarites_eleve(etablissement_id, eleve_id).await?;
        Ok(all
            .into_iter()
            .filter(|s| annees.iter().any(|a| *a == s.annee_scolaire.as_str()))
            .collect())
    }

    pub async fn upsert_option_annee(&self, o: &OptionAnnee) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_eleve_option_annee (
                id, etablissement_id, eleve_id, annee_scolaire,
                nature, code, libelle, statut, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now())
            ON CONFLICT (etablissement_id, eleve_id, annee_scolaire, nature, code) DO UPDATE SET
                libelle = EXCLUDED.libelle,
                statut = EXCLUDED.statut,
                id = ent_eleve_option_annee.id
            "#,
        )
        .bind(o.id)
        .bind(o.etablissement_id)
        .bind(o.eleve_id)
        .bind(&o.annee_scolaire)
        .bind(o.nature.as_str())
        .bind(&o.code)
        .bind(&o.libelle)
        .bind(o.statut.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_options_annee(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        annee_scolaire: &str,
    ) -> Result<Vec<OptionAnnee>, DomainError> {
        let rows = sqlx::query_as::<_, OptAnneeRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, annee_scolaire,
                   nature, code, libelle, statut
            FROM ent_eleve_option_annee
            WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_scolaire = $3
            ORDER BY nature, code
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(annee_scolaire)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(OptAnneeRow::into_opt).collect())
    }

    pub async fn list_options_continuum(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        annees: &[&str],
    ) -> Result<Vec<OptionAnnee>, DomainError> {
        let mut out = Vec::new();
        for a in annees {
            out.extend(self.list_options_annee(etablissement_id, eleve_id, a).await?);
        }
        Ok(out)
    }

    pub async fn upsert_exigence_doc(
        &self,
        e: &ExigenceDocReinscription,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_reinscription_exigence_doc (
                id, etablissement_id, annee_cible, code, libelle, tiroir, obligatoire, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7, now())
            ON CONFLICT (etablissement_id, annee_cible, code) DO UPDATE SET
                libelle = EXCLUDED.libelle,
                tiroir = EXCLUDED.tiroir,
                obligatoire = EXCLUDED.obligatoire,
                id = ent_reinscription_exigence_doc.id
            "#,
        )
        .bind(e.id)
        .bind(e.etablissement_id)
        .bind(&e.annee_cible)
        .bind(&e.code)
        .bind(&e.libelle)
        .bind(e.tiroir.as_str())
        .bind(e.obligatoire)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_exigences_doc(
        &self,
        etablissement_id: Uuid,
        annee_cible: &str,
    ) -> Result<Vec<ExigenceDocReinscription>, DomainError> {
        let rows = sqlx::query_as::<_, ExigenceRow>(
            r#"
            SELECT id, etablissement_id, annee_cible, code, libelle, tiroir, obligatoire
            FROM ent_reinscription_exigence_doc
            WHERE etablissement_id = $1 AND annee_cible = $2
            ORDER BY code
            "#,
        )
        .bind(etablissement_id)
        .bind(annee_cible)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(ExigenceRow::into_ex).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ExigenceRow {
    id: Uuid,
    etablissement_id: Uuid,
    annee_cible: String,
    code: String,
    libelle: String,
    tiroir: String,
    obligatoire: bool,
}

impl ExigenceRow {
    fn into_ex(self) -> Option<ExigenceDocReinscription> {
        Some(ExigenceDocReinscription {
            id: self.id,
            etablissement_id: self.etablissement_id,
            annee_cible: self.annee_cible,
            code: self.code,
            libelle: self.libelle,
            tiroir: DocTiroir::parse(&self.tiroir)?,
            obligatoire: self.obligatoire,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct CampagneRow {
    id: Uuid,
    etablissement_id: Uuid,
    annee_cible: String,
    ouverte: bool,
    acompte_centimes: i64,
}

impl CampagneRow {
    fn into_campagne(self) -> ReinscriptionCampagne {
        ReinscriptionCampagne {
            id: self.id,
            etablissement_id: self.etablissement_id,
            annee_cible: self.annee_cible,
            ouverte: self.ouverte,
            acompte_centimes: self.acompte_centimes,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ScolariteRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    annee_scolaire: String,
    niveau: Option<String>,
    mef: Option<String>,
    classe: Option<String>,
    regime: Option<String>,
    statut: String,
    source: String,
    passage_juridique: bool,
    redoublement_possible: bool,
    date_montee: Option<chrono::NaiveDate>,
}

impl ScolariteRow {
    fn into_scol(self) -> Option<ScolariteAnnee> {
        Some(ScolariteAnnee {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            annee_scolaire: self.annee_scolaire,
            niveau: self.niveau,
            mef: self.mef,
            classe: self.classe,
            regime: self.regime,
            statut: ScolariteAnneeStatut::parse(&self.statut)?,
            source: ScolariteSource::parse(&self.source)?,
            passage_juridique: self.passage_juridique,
            redoublement_possible: self.redoublement_possible,
            date_montee: self.date_montee,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct OptAnneeRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    annee_scolaire: String,
    nature: String,
    code: String,
    libelle: String,
    statut: String,
}

impl OptAnneeRow {
    fn into_opt(self) -> Option<OptionAnnee> {
        Some(OptionAnnee {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            annee_scolaire: self.annee_scolaire,
            nature: OptionNature::parse(&self.nature)?,
            code: self.code,
            libelle: self.libelle,
            statut: OptionAnneeStatut::parse(&self.statut)?,
        })
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] continuum sqlx error: {err}");
    DomainError::NotFound
}
