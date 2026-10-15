use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use domain_eleve::{
    CommissionDossier, CommissionStatut, CommissionVs, DecisionCommission, DomainError, Incident,
    IncidentType, Sanction, SanctionMotif, SanctionStatut, SanctionType,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgSanctionRepo {
    pool: PgPool,
}

impl PgSanctionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_incident_type(&self, t: &IncidentType) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_incident_type (
                id, etablissement_id, code, libelle, actif, created_at
            ) VALUES ($1,$2,$3,$4,$5, now())
            ON CONFLICT (etablissement_id, code) DO UPDATE SET
                libelle = EXCLUDED.libelle,
                actif = EXCLUDED.actif,
                id = ent_incident_type.id
            "#,
        )
        .bind(t.id)
        .bind(t.etablissement_id)
        .bind(&t.code)
        .bind(&t.libelle)
        .bind(t.actif)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_incident_type(
        &self,
        etablissement_id: Uuid,
        type_id: Uuid,
    ) -> Result<Option<IncidentType>, DomainError> {
        let row = sqlx::query_as::<_, TypeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, actif
            FROM ent_incident_type
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(type_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(TypeRow::into_incident_type))
    }

    pub async fn get_incident_type_by_code(
        &self,
        etablissement_id: Uuid,
        code: &str,
    ) -> Result<Option<IncidentType>, DomainError> {
        let row = sqlx::query_as::<_, TypeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, actif
            FROM ent_incident_type
            WHERE etablissement_id = $1 AND code = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(TypeRow::into_incident_type))
    }

    pub async fn list_incident_types(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<IncidentType>, DomainError> {
        let rows = sqlx::query_as::<_, TypeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, actif
            FROM ent_incident_type
            WHERE etablissement_id = $1
            ORDER BY code
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(TypeRow::into_incident_type).collect())
    }

    pub async fn upsert_sanction_type(&self, t: &SanctionType) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_sanction_type (
                id, etablissement_id, code, libelle, actif, created_at
            ) VALUES ($1,$2,$3,$4,$5, now())
            ON CONFLICT (etablissement_id, code) DO UPDATE SET
                libelle = EXCLUDED.libelle,
                actif = EXCLUDED.actif,
                id = ent_sanction_type.id
            "#,
        )
        .bind(t.id)
        .bind(t.etablissement_id)
        .bind(&t.code)
        .bind(&t.libelle)
        .bind(t.actif)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_sanction_type(
        &self,
        etablissement_id: Uuid,
        type_id: Uuid,
    ) -> Result<Option<SanctionType>, DomainError> {
        let row = sqlx::query_as::<_, TypeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, actif
            FROM ent_sanction_type
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(type_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(TypeRow::into_sanction_type))
    }

    pub async fn get_sanction_type_by_code(
        &self,
        etablissement_id: Uuid,
        code: &str,
    ) -> Result<Option<SanctionType>, DomainError> {
        let row = sqlx::query_as::<_, TypeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, actif
            FROM ent_sanction_type
            WHERE etablissement_id = $1 AND code = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(TypeRow::into_sanction_type))
    }

    pub async fn list_sanction_types(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<SanctionType>, DomainError> {
        let rows = sqlx::query_as::<_, TypeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, actif
            FROM ent_sanction_type
            WHERE etablissement_id = $1
            ORDER BY code
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(TypeRow::into_sanction_type).collect())
    }

    pub async fn upsert_sanction_motif(&self, m: &SanctionMotif) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_sanction_motif (
                id, etablissement_id, code, libelle, actif, created_at
            ) VALUES ($1,$2,$3,$4,$5, now())
            ON CONFLICT (etablissement_id, code) DO UPDATE SET
                libelle = EXCLUDED.libelle,
                actif = EXCLUDED.actif,
                id = ent_sanction_motif.id
            "#,
        )
        .bind(m.id)
        .bind(m.etablissement_id)
        .bind(&m.code)
        .bind(&m.libelle)
        .bind(m.actif)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_sanction_motif(
        &self,
        etablissement_id: Uuid,
        motif_id: Uuid,
    ) -> Result<Option<SanctionMotif>, DomainError> {
        let row = sqlx::query_as::<_, TypeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, actif
            FROM ent_sanction_motif
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(motif_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(TypeRow::into_sanction_motif))
    }

    pub async fn get_sanction_motif_by_code(
        &self,
        etablissement_id: Uuid,
        code: &str,
    ) -> Result<Option<SanctionMotif>, DomainError> {
        let row = sqlx::query_as::<_, TypeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, actif
            FROM ent_sanction_motif
            WHERE etablissement_id = $1 AND code = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(TypeRow::into_sanction_motif))
    }

    pub async fn list_sanction_motifs(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<SanctionMotif>, DomainError> {
        let rows = sqlx::query_as::<_, TypeRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, actif
            FROM ent_sanction_motif
            WHERE etablissement_id = $1
            ORDER BY code
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(TypeRow::into_sanction_motif).collect())
    }

    pub async fn upsert_incident(&self, i: &Incident) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_incident (
                id, etablissement_id, eleve_id, type_id, jour, description,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7, now())
            ON CONFLICT (id) DO UPDATE SET
                description = EXCLUDED.description
            WHERE ent_incident.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(i.id)
        .bind(i.etablissement_id)
        .bind(i.eleve_id)
        .bind(i.type_id)
        .bind(i.jour)
        .bind(&i.description)
        .bind(i.created_by_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_incident(
        &self,
        etablissement_id: Uuid,
        incident_id: Uuid,
    ) -> Result<Option<Incident>, DomainError> {
        let row = sqlx::query_as::<_, IncidentRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, type_id, jour, description, created_by_user_id
            FROM ent_incident
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(incident_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(IncidentRow::into_incident))
    }

    pub async fn list_incidents_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<Incident>, DomainError> {
        let rows = sqlx::query_as::<_, IncidentRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, type_id, jour, description, created_by_user_id
            FROM ent_incident
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY jour DESC, created_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(IncidentRow::into_incident).collect())
    }

    pub async fn upsert_sanction(&self, s: &Sanction) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_sanction (
                id, etablissement_id, eleve_id, type_id, motif_id, incident_id,
                jour, duree_minutes, note, statut, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11, now())
            ON CONFLICT (id) DO UPDATE SET
                motif_id = EXCLUDED.motif_id,
                incident_id = EXCLUDED.incident_id,
                duree_minutes = EXCLUDED.duree_minutes,
                note = EXCLUDED.note,
                statut = EXCLUDED.statut
            WHERE ent_sanction.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(s.id)
        .bind(s.etablissement_id)
        .bind(s.eleve_id)
        .bind(s.type_id)
        .bind(s.motif_id)
        .bind(s.incident_id)
        .bind(s.jour)
        .bind(s.duree_minutes)
        .bind(&s.note)
        .bind(s.statut.as_str())
        .bind(s.created_by_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn mark_sanction_annulee(
        &self,
        etablissement_id: Uuid,
        sanction_id: Uuid,
        actor_user_id: Uuid,
    ) -> Result<(), DomainError> {
        let res = sqlx::query(
            r#"
            UPDATE ent_sanction
            SET statut = 'annulee',
                annulee_par_user_id = $3,
                annulee_at = now()
            WHERE etablissement_id = $1 AND id = $2 AND statut <> 'annulee'
            "#,
        )
        .bind(etablissement_id)
        .bind(sanction_id)
        .bind(actor_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }

    pub async fn get_sanction(
        &self,
        etablissement_id: Uuid,
        sanction_id: Uuid,
    ) -> Result<Option<Sanction>, DomainError> {
        let row = sqlx::query_as::<_, SanctionRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, type_id, motif_id, incident_id,
                   jour, duree_minutes, note, statut, created_by_user_id
            FROM ent_sanction
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(sanction_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(SanctionRow::into_sanction))
    }

    pub async fn list_sanctions_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<Sanction>, DomainError> {
        let rows = sqlx::query_as::<_, SanctionRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, type_id, motif_id, incident_id,
                   jour, duree_minutes, note, statut, created_by_user_id
            FROM ent_sanction
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY jour DESC, created_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(SanctionRow::into_sanction).collect())
    }

    pub async fn insert_commission(
        &self,
        c: &CommissionVs,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_vs_commission (
                id, etablissement_id, jour, heure, objet, lieu, statut,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(c.id)
        .bind(c.etablissement_id)
        .bind(c.jour)
        .bind(c.heure)
        .bind(&c.objet)
        .bind(&c.lieu)
        .bind(c.statut.as_str())
        .bind(c.created_by_user_id)
        .bind(c.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_commission(
        &self,
        etablissement_id: Uuid,
        commission_id: Uuid,
    ) -> Result<Option<CommissionVs>, DomainError> {
        let row = sqlx::query_as::<_, CommissionRow>(
            r#"
            SELECT id, etablissement_id, jour, heure, objet, lieu, statut,
                   created_by_user_id, created_at
            FROM ent_vs_commission
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(commission_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(CommissionRow::into_commission))
    }

    pub async fn update_commission_statut(
        &self,
        c: &CommissionVs,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_vs_commission SET statut = $3
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

    pub async fn list_commissions_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
        limit: i64,
    ) -> Result<Vec<CommissionVs>, DomainError> {
        let rows = sqlx::query_as::<_, CommissionRow>(
            r#"
            SELECT id, etablissement_id, jour, heure, objet, lieu, statut,
                   created_by_user_id, created_at
            FROM ent_vs_commission
            WHERE etablissement_id = $1 AND jour = $2
            ORDER BY heure ASC
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(CommissionRow::into_commission).collect())
    }

    pub async fn insert_commission_dossier(
        &self,
        d: &CommissionDossier,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_vs_commission_dossier (
                id, etablissement_id, commission_id, eleve_id, motifs, note_interne,
                sanction_id, incident_id, decision, decision_texte, points_retires,
                signale_famille, decided_at, decided_by_user_id,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
            "#,
        )
        .bind(d.id)
        .bind(d.etablissement_id)
        .bind(d.commission_id)
        .bind(d.eleve_id)
        .bind(&d.motifs)
        .bind(&d.note_interne)
        .bind(d.sanction_id)
        .bind(d.incident_id)
        .bind(d.decision.map(|x| x.as_str()))
        .bind(&d.decision_texte)
        .bind(d.points_retires)
        .bind(d.signale_famille)
        .bind(d.decided_at)
        .bind(d.decided_by_user_id)
        .bind(d.created_by_user_id)
        .bind(d.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_commission_dossier(
        &self,
        etablissement_id: Uuid,
        dossier_id: Uuid,
    ) -> Result<Option<CommissionDossier>, DomainError> {
        let row = sqlx::query_as::<_, DossierRow>(
            r#"
            SELECT id, etablissement_id, commission_id, eleve_id, motifs, note_interne,
                   sanction_id, incident_id, decision, decision_texte, points_retires,
                   signale_famille, decided_at, decided_by_user_id,
                   created_by_user_id, created_at
            FROM ent_vs_commission_dossier
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

    pub async fn update_commission_dossier(
        &self,
        d: &CommissionDossier,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_vs_commission_dossier SET
                decision = $3,
                decision_texte = $4,
                points_retires = $5,
                decided_at = $6,
                decided_by_user_id = $7
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(d.etablissement_id)
        .bind(d.id)
        .bind(d.decision.map(|x| x.as_str()))
        .bind(&d.decision_texte)
        .bind(d.points_retires)
        .bind(d.decided_at)
        .bind(d.decided_by_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_dossiers_commission(
        &self,
        etablissement_id: Uuid,
        commission_id: Uuid,
    ) -> Result<Vec<CommissionDossier>, DomainError> {
        let rows = sqlx::query_as::<_, DossierRow>(
            r#"
            SELECT id, etablissement_id, commission_id, eleve_id, motifs, note_interne,
                   sanction_id, incident_id, decision, decision_texte, points_retires,
                   signale_famille, decided_at, decided_by_user_id,
                   created_by_user_id, created_at
            FROM ent_vs_commission_dossier
            WHERE etablissement_id = $1 AND commission_id = $2
            ORDER BY created_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(commission_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(DossierRow::into_dossier).collect())
    }

    pub async fn list_dossiers_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        limit: i64,
    ) -> Result<Vec<CommissionDossier>, DomainError> {
        let rows = sqlx::query_as::<_, DossierRow>(
            r#"
            SELECT id, etablissement_id, commission_id, eleve_id, motifs, note_interne,
                   sanction_id, incident_id, decision, decision_texte, points_retires,
                   signale_famille, decided_at, decided_by_user_id,
                   created_by_user_id, created_at
            FROM ent_vs_commission_dossier
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(DossierRow::into_dossier).collect())
    }

    pub async fn sum_points_retires_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<i32, DomainError> {
        let row: (Option<i64>,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(points_retires), 0)::bigint
            FROM ent_vs_commission_dossier
            WHERE etablissement_id = $1 AND eleve_id = $2 AND decision IS NOT NULL
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.0.unwrap_or(0).clamp(0, i64::from(i32::MAX)) as i32)
    }
}

#[derive(Debug, sqlx::FromRow)]
struct TypeRow {
    id: Uuid,
    etablissement_id: Uuid,
    code: String,
    libelle: String,
    actif: bool,
}

impl TypeRow {
    fn into_incident_type(self) -> IncidentType {
        IncidentType {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            libelle: self.libelle,
            actif: self.actif,
        }
    }

    fn into_sanction_type(self) -> SanctionType {
        SanctionType {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            libelle: self.libelle,
            actif: self.actif,
        }
    }

    fn into_sanction_motif(self) -> SanctionMotif {
        SanctionMotif {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            libelle: self.libelle,
            actif: self.actif,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct IncidentRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    type_id: Uuid,
    jour: chrono::NaiveDate,
    description: Option<String>,
    created_by_user_id: Option<Uuid>,
}

impl IncidentRow {
    fn into_incident(self) -> Incident {
        Incident {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            type_id: self.type_id,
            jour: self.jour,
            description: self.description,
            created_by_user_id: self.created_by_user_id,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SanctionRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    type_id: Uuid,
    motif_id: Option<Uuid>,
    incident_id: Option<Uuid>,
    jour: chrono::NaiveDate,
    duree_minutes: Option<i32>,
    note: Option<String>,
    statut: String,
    created_by_user_id: Option<Uuid>,
}

impl SanctionRow {
    fn into_sanction(self) -> Option<Sanction> {
        Some(Sanction {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            type_id: self.type_id,
            motif_id: self.motif_id,
            incident_id: self.incident_id,
            jour: self.jour,
            duree_minutes: self.duree_minutes,
            note: self.note,
            statut: SanctionStatut::parse(&self.statut)?,
            created_by_user_id: self.created_by_user_id,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct CommissionRow {
    id: Uuid,
    etablissement_id: Uuid,
    jour: NaiveDate,
    heure: NaiveTime,
    objet: String,
    lieu: String,
    statut: String,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl CommissionRow {
    fn into_commission(self) -> Option<CommissionVs> {
        Some(CommissionVs {
            id: self.id,
            etablissement_id: self.etablissement_id,
            jour: self.jour,
            heure: self.heure,
            objet: self.objet,
            lieu: self.lieu,
            statut: CommissionStatut::parse(&self.statut)?,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DossierRow {
    id: Uuid,
    etablissement_id: Uuid,
    commission_id: Uuid,
    eleve_id: Uuid,
    motifs: String,
    note_interne: String,
    sanction_id: Option<Uuid>,
    incident_id: Option<Uuid>,
    decision: Option<String>,
    decision_texte: String,
    points_retires: i32,
    signale_famille: bool,
    decided_at: Option<DateTime<Utc>>,
    decided_by_user_id: Option<Uuid>,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl DossierRow {
    fn into_dossier(self) -> Option<CommissionDossier> {
        let decision = match self.decision.as_deref() {
            None => None,
            Some(s) => Some(DecisionCommission::parse(s)?),
        };
        Some(CommissionDossier {
            id: self.id,
            etablissement_id: self.etablissement_id,
            commission_id: self.commission_id,
            eleve_id: self.eleve_id,
            motifs: self.motifs,
            note_interne: self.note_interne,
            sanction_id: self.sanction_id,
            incident_id: self.incident_id,
            decision,
            decision_texte: self.decision_texte,
            points_retires: self.points_retires,
            signale_famille: self.signale_famille,
            decided_at: self.decided_at,
            decided_by_user_id: self.decided_by_user_id,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] sanction sqlx error: {err}");
    DomainError::NotFound
}
