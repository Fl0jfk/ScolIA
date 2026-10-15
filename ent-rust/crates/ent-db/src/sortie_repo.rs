use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{
    AutorisationSortie, ControleSortie, ControleSortieSource, ControleSortieVerdict,
    DemandeSortieExceptionnelle, DemandeSortieStatut, DomainError, SortieCreneau, SortieParamEtab,
    SortieRegime, SortieRegleNiveau, SortieRegleRegime,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgSortieRepo {
    pool: PgPool,
}

impl PgSortieRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_autorisation(&self, a: &AutorisationSortie) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_sortie_autorisation (
                id, etablissement_id, eleve_id, matin, midi, soir,
                interdit_etablissement, updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            ON CONFLICT (etablissement_id, eleve_id) DO UPDATE SET
                matin = EXCLUDED.matin,
                midi = EXCLUDED.midi,
                soir = EXCLUDED.soir,
                interdit_etablissement = EXCLUDED.interdit_etablissement,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at,
                id = ent_sortie_autorisation.id
            "#,
        )
        .bind(a.id)
        .bind(a.etablissement_id)
        .bind(a.eleve_id)
        .bind(a.matin)
        .bind(a.midi)
        .bind(a.soir)
        .bind(a.interdit_etablissement)
        .bind(a.updated_by_user_id)
        .bind(a.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_autorisation(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<AutorisationSortie>, DomainError> {
        let row = sqlx::query_as::<_, AuthRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, matin, midi, soir,
                   interdit_etablissement, updated_by_user_id, updated_at
            FROM ent_sortie_autorisation
            WHERE etablissement_id = $1 AND eleve_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(AuthRow::into_auth))
    }

    pub async fn insert_demande(
        &self,
        d: &DemandeSortieExceptionnelle,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_sortie_demande (
                id, etablissement_id, eleve_id, jour, creneau, motif, statut,
                demande_par_user_id, decided_by_user_id, decided_at, decision_note, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            "#,
        )
        .bind(d.id)
        .bind(d.etablissement_id)
        .bind(d.eleve_id)
        .bind(d.jour)
        .bind(d.creneau.as_str())
        .bind(&d.motif)
        .bind(d.statut.as_str())
        .bind(d.demande_par_user_id)
        .bind(d.decided_by_user_id)
        .bind(d.decided_at)
        .bind(&d.decision_note)
        .bind(d.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn update_demande(
        &self,
        d: &DemandeSortieExceptionnelle,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_sortie_demande SET
                statut = $3,
                decided_by_user_id = $4,
                decided_at = $5,
                decision_note = $6
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(d.etablissement_id)
        .bind(d.id)
        .bind(d.statut.as_str())
        .bind(d.decided_by_user_id)
        .bind(d.decided_at)
        .bind(&d.decision_note)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_demande(
        &self,
        etablissement_id: Uuid,
        demande_id: Uuid,
    ) -> Result<Option<DemandeSortieExceptionnelle>, DomainError> {
        let row = sqlx::query_as::<_, DemandeRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, creneau, motif, statut,
                   demande_par_user_id, decided_by_user_id, decided_at, decision_note, created_at
            FROM ent_sortie_demande
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(demande_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(DemandeRow::into_demande))
    }

    pub async fn list_demandes_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        only_ouvertes: bool,
    ) -> Result<Vec<DemandeSortieExceptionnelle>, DomainError> {
        let rows = if only_ouvertes {
            sqlx::query_as::<_, DemandeRow>(
                r#"
                SELECT id, etablissement_id, eleve_id, jour, creneau, motif, statut,
                       demande_par_user_id, decided_by_user_id, decided_at, decision_note, created_at
                FROM ent_sortie_demande
                WHERE etablissement_id = $1 AND eleve_id = $2 AND statut = 'en_attente'
                ORDER BY jour DESC, created_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, DemandeRow>(
                r#"
                SELECT id, etablissement_id, eleve_id, jour, creneau, motif, statut,
                       demande_par_user_id, decided_by_user_id, decided_at, decision_note, created_at
                FROM ent_sortie_demande
                WHERE etablissement_id = $1 AND eleve_id = $2
                ORDER BY jour DESC, created_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().filter_map(DemandeRow::into_demande).collect())
    }

    pub async fn list_demandes_jour(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<DemandeSortieExceptionnelle>, DomainError> {
        let rows = sqlx::query_as::<_, DemandeRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, creneau, motif, statut,
                   demande_par_user_id, decided_by_user_id, decided_at, decision_note, created_at
            FROM ent_sortie_demande
            WHERE etablissement_id = $1 AND eleve_id = $2 AND jour = $3
            ORDER BY created_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(jour)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(DemandeRow::into_demande).collect())
    }

    pub async fn list_demandes_en_attente(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<DemandeSortieExceptionnelle>, DomainError> {
        let rows = sqlx::query_as::<_, DemandeRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, creneau, motif, statut,
                   demande_par_user_id, decided_by_user_id, decided_at, decision_note, created_at
            FROM ent_sortie_demande
            WHERE etablissement_id = $1 AND statut = 'en_attente'
            ORDER BY jour ASC, created_at ASC
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(DemandeRow::into_demande).collect())
    }

    pub async fn insert_controle(&self, c: &ControleSortie) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_sortie_controle (
                id, etablissement_id, eleve_id, jour, creneau, verdict, source, motif,
                actor_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(c.id)
        .bind(c.etablissement_id)
        .bind(c.eleve_id)
        .bind(c.jour)
        .bind(c.creneau.as_str())
        .bind(c.verdict.as_str())
        .bind(c.source.as_str())
        .bind(&c.motif)
        .bind(c.actor_user_id)
        .bind(c.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_controles_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<ControleSortie>, DomainError> {
        let rows = sqlx::query_as::<_, ControleRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, creneau, verdict, source, motif,
                   actor_user_id, created_at
            FROM ent_sortie_controle
            WHERE etablissement_id = $1 AND jour = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(ControleRow::into_controle).collect())
    }

    pub async fn upsert_regle_niveau(&self, r: &SortieRegleNiveau) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_sortie_regle_niveau (
                id, etablissement_id, niveau, matin_autorise, midi_autorise, soir_autorise,
                choix_parent_interdit, updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            ON CONFLICT (etablissement_id, niveau) DO UPDATE SET
                matin_autorise = EXCLUDED.matin_autorise,
                midi_autorise = EXCLUDED.midi_autorise,
                soir_autorise = EXCLUDED.soir_autorise,
                choix_parent_interdit = EXCLUDED.choix_parent_interdit,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at,
                id = ent_sortie_regle_niveau.id
            "#,
        )
        .bind(r.id)
        .bind(r.etablissement_id)
        .bind(&r.niveau)
        .bind(r.matin_autorise)
        .bind(r.midi_autorise)
        .bind(r.soir_autorise)
        .bind(r.choix_parent_interdit)
        .bind(r.updated_by_user_id)
        .bind(r.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_regle_niveau(
        &self,
        etablissement_id: Uuid,
        niveau: &str,
    ) -> Result<Option<SortieRegleNiveau>, DomainError> {
        let row = sqlx::query_as::<_, RegleRow>(
            r#"
            SELECT id, etablissement_id, niveau, matin_autorise, midi_autorise, soir_autorise,
                   choix_parent_interdit, updated_by_user_id, updated_at
            FROM ent_sortie_regle_niveau
            WHERE etablissement_id = $1 AND niveau = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(niveau)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(RegleRow::into_regle))
    }

    pub async fn list_regles_niveau(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<SortieRegleNiveau>, DomainError> {
        let rows = sqlx::query_as::<_, RegleRow>(
            r#"
            SELECT id, etablissement_id, niveau, matin_autorise, midi_autorise, soir_autorise,
                   choix_parent_interdit, updated_by_user_id, updated_at
            FROM ent_sortie_regle_niveau
            WHERE etablissement_id = $1
            ORDER BY niveau
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(RegleRow::into_regle).collect())
    }

    pub async fn upsert_regle_regime(&self, r: &SortieRegleRegime) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_sortie_regle_regime (
                id, etablissement_id, regime, matin_autorise, midi_autorise, soir_autorise,
                choix_parent_interdit, updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            ON CONFLICT (etablissement_id, regime) DO UPDATE SET
                matin_autorise = EXCLUDED.matin_autorise,
                midi_autorise = EXCLUDED.midi_autorise,
                soir_autorise = EXCLUDED.soir_autorise,
                choix_parent_interdit = EXCLUDED.choix_parent_interdit,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at,
                id = ent_sortie_regle_regime.id
            "#,
        )
        .bind(r.id)
        .bind(r.etablissement_id)
        .bind(r.regime.as_str())
        .bind(r.matin_autorise)
        .bind(r.midi_autorise)
        .bind(r.soir_autorise)
        .bind(r.choix_parent_interdit)
        .bind(r.updated_by_user_id)
        .bind(r.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_regle_regime(
        &self,
        etablissement_id: Uuid,
        regime: &str,
    ) -> Result<Option<SortieRegleRegime>, DomainError> {
        let row = sqlx::query_as::<_, RegleRegimeRow>(
            r#"
            SELECT id, etablissement_id, regime, matin_autorise, midi_autorise, soir_autorise,
                   choix_parent_interdit, updated_by_user_id, updated_at
            FROM ent_sortie_regle_regime
            WHERE etablissement_id = $1 AND regime = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(regime)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(RegleRegimeRow::into_regle))
    }

    pub async fn upsert_param(&self, p: &SortieParamEtab) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_sortie_param (
                etablissement_id, autoriser_changement_site, updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4)
            ON CONFLICT (etablissement_id) DO UPDATE SET
                autoriser_changement_site = EXCLUDED.autoriser_changement_site,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(p.etablissement_id)
        .bind(p.autoriser_changement_site)
        .bind(p.updated_by_user_id)
        .bind(p.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_param(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Option<SortieParamEtab>, DomainError> {
        let row = sqlx::query_as::<_, ParamRow>(
            r#"
            SELECT etablissement_id, autoriser_changement_site, updated_by_user_id, updated_at
            FROM ent_sortie_param
            WHERE etablissement_id = $1
            "#,
        )
        .bind(etablissement_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(ParamRow::into_param))
    }

    pub async fn list_regles_regime(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<SortieRegleRegime>, DomainError> {
        let rows = sqlx::query_as::<_, RegleRegimeRow>(
            r#"
            SELECT id, etablissement_id, regime, matin_autorise, midi_autorise, soir_autorise,
                   choix_parent_interdit, updated_by_user_id, updated_at
            FROM ent_sortie_regle_regime
            WHERE etablissement_id = $1
            ORDER BY regime
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(RegleRegimeRow::into_regle).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] sortie sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct AuthRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    matin: bool,
    midi: bool,
    soir: bool,
    interdit_etablissement: bool,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl AuthRow {
    fn into_auth(self) -> AutorisationSortie {
        AutorisationSortie {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            matin: self.matin,
            midi: self.midi,
            soir: self.soir,
            interdit_etablissement: self.interdit_etablissement,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct DemandeRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    creneau: String,
    motif: String,
    statut: String,
    demande_par_user_id: Uuid,
    decided_by_user_id: Option<Uuid>,
    decided_at: Option<DateTime<Utc>>,
    decision_note: String,
    created_at: DateTime<Utc>,
}

impl DemandeRow {
    fn into_demande(self) -> Option<DemandeSortieExceptionnelle> {
        Some(DemandeSortieExceptionnelle {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            jour: self.jour,
            creneau: SortieCreneau::parse(&self.creneau)?,
            motif: self.motif,
            statut: DemandeSortieStatut::parse(&self.statut)?,
            demande_par_user_id: self.demande_par_user_id,
            decided_by_user_id: self.decided_by_user_id,
            decided_at: self.decided_at,
            decision_note: self.decision_note,
            created_at: self.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ControleRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    creneau: String,
    verdict: String,
    source: String,
    motif: String,
    actor_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl ControleRow {
    fn into_controle(self) -> Option<ControleSortie> {
        Some(ControleSortie {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            jour: self.jour,
            creneau: SortieCreneau::parse(&self.creneau)?,
            verdict: ControleSortieVerdict::parse(&self.verdict)?,
            source: ControleSortieSource::parse(&self.source)?,
            motif: self.motif,
            actor_user_id: self.actor_user_id,
            created_at: self.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct RegleRow {
    id: Uuid,
    etablissement_id: Uuid,
    niveau: String,
    matin_autorise: bool,
    midi_autorise: bool,
    soir_autorise: bool,
    choix_parent_interdit: bool,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl RegleRow {
    fn into_regle(self) -> SortieRegleNiveau {
        SortieRegleNiveau {
            id: self.id,
            etablissement_id: self.etablissement_id,
            niveau: self.niveau,
            matin_autorise: self.matin_autorise,
            midi_autorise: self.midi_autorise,
            soir_autorise: self.soir_autorise,
            choix_parent_interdit: self.choix_parent_interdit,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct RegleRegimeRow {
    id: Uuid,
    etablissement_id: Uuid,
    regime: String,
    matin_autorise: bool,
    midi_autorise: bool,
    soir_autorise: bool,
    choix_parent_interdit: bool,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl RegleRegimeRow {
    fn into_regle(self) -> Option<SortieRegleRegime> {
        Some(SortieRegleRegime {
            id: self.id,
            etablissement_id: self.etablissement_id,
            regime: SortieRegime::parse(&self.regime)?,
            matin_autorise: self.matin_autorise,
            midi_autorise: self.midi_autorise,
            soir_autorise: self.soir_autorise,
            choix_parent_interdit: self.choix_parent_interdit,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ParamRow {
    etablissement_id: Uuid,
    autoriser_changement_site: bool,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl ParamRow {
    fn into_param(self) -> SortieParamEtab {
        SortieParamEtab {
            etablissement_id: self.etablissement_id,
            autoriser_changement_site: self.autoriser_changement_site,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        }
    }
}
