use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use domain_eleve::{
    DomainError, DossierPsy, EntretienPsy, EntretienPsyKind, RdvPsy, RdvPsyStatut,
};
use crate::champs_chiffres::ChampsSensibles;
use crate::tenant_tx::TenantTx;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgPsyRepo {
    pool: PgPool,
    champs: ChampsSensibles,
}

const T_ENTRETIEN: &str = "ent_psy_entretien";
const T_RDV: &str = "ent_psy_rdv";
const T_DOSSIER: &str = "ent_psy_dossier";

impl PgPsyRepo {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            champs: ChampsSensibles::depuis_env(),
        }
    }

    pub async fn insert(&self, e: &EntretienPsy) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, e.etablissement_id).await.map_err(map_sqlx)?;
        let note = self
            .champs
            .sceller(T_ENTRETIEN, "note_confidentielle", e.id, &e.note_confidentielle)?;
        sqlx::query(
            r#"
            INSERT INTO ent_psy_entretien (
                id, etablissement_id, eleve_id, jour, kind, duree_minutes,
                note_confidentielle, signale_famille, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(e.id)
        .bind(e.etablissement_id)
        .bind(e.eleve_id)
        .bind(e.jour)
        .bind(e.kind.as_str())
        .bind(e.duree_minutes)
        .bind(&note)
        .bind(e.signale_famille)
        .bind(e.created_by_user_id)
        .bind(e.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        limit: i64,
    ) -> Result<Vec<EntretienPsy>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, EntretienRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, kind, duree_minutes,
                   note_confidentielle, signale_famille, created_by_user_id, created_at
            FROM ent_psy_entretien
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY jour DESC, created_at DESC
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        rows.into_iter()
            .filter_map(|r| r.into_entretien(&self.champs).transpose())
            .collect()
    }

    pub async fn list_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
        limit: i64,
    ) -> Result<Vec<EntretienPsy>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, EntretienRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, kind, duree_minutes,
                   note_confidentielle, signale_famille, created_by_user_id, created_at
            FROM ent_psy_entretien
            WHERE etablissement_id = $1 AND jour = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        rows.into_iter()
            .filter_map(|r| r.into_entretien(&self.champs).transpose())
            .collect()
    }

    pub async fn insert_rdv(&self, r: &RdvPsy) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, r.etablissement_id).await.map_err(map_sqlx)?;
        let motif = self.champs.sceller(T_RDV, "motif", r.id, &r.motif)?;
        let note_interne = self.champs.sceller(T_RDV, "note_interne", r.id, &r.note_interne)?;
        sqlx::query(
            r#"
            INSERT INTO ent_psy_rdv (
                id, etablissement_id, eleve_id, jour, heure_debut, heure_fin, kind,
                motif, note_interne, statut, signale_famille, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            "#,
        )
        .bind(r.id)
        .bind(r.etablissement_id)
        .bind(r.eleve_id)
        .bind(r.jour)
        .bind(r.heure_debut)
        .bind(r.heure_fin)
        .bind(r.kind.as_str())
        .bind(&motif)
        .bind(&note_interne)
        .bind(r.statut.as_str())
        .bind(r.signale_famille)
        .bind(r.created_by_user_id)
        .bind(r.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_rdv(
        &self,
        etablissement_id: Uuid,
        rdv_id: Uuid,
    ) -> Result<Option<RdvPsy>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row = sqlx::query_as::<_, RdvRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, heure_debut, heure_fin, kind,
                   motif, note_interne, statut, signale_famille, created_by_user_id, created_at
            FROM ent_psy_rdv
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(rdv_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        row.and_then(|r| r.into_rdv(&self.champs).transpose()).transpose()
    }

    pub async fn update_rdv_statut(&self, r: &RdvPsy) -> Result<(), DomainError> {
        let mut tx = TenantTx::begin(&self.pool, r.etablissement_id).await.map_err(map_sqlx)?;
        let n = sqlx::query(
            r#"
            UPDATE ent_psy_rdv SET statut = $3
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(r.etablissement_id)
        .bind(r.id)
        .bind(r.statut.as_str())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        if n == 0 {
            return Err(DomainError::NotFound);
        }
        tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_rdv_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
        limit: i64,
    ) -> Result<Vec<RdvPsy>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, RdvRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, heure_debut, heure_fin, kind,
                   motif, note_interne, statut, signale_famille, created_by_user_id, created_at
            FROM ent_psy_rdv
            WHERE etablissement_id = $1 AND jour = $2
            ORDER BY heure_debut, created_at
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        rows.into_iter()
            .filter_map(|r| r.into_rdv(&self.champs).transpose())
            .collect()
    }

    pub async fn list_rdv_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        limit: i64,
    ) -> Result<Vec<RdvPsy>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let rows = sqlx::query_as::<_, RdvRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, heure_debut, heure_fin, kind,
                   motif, note_interne, statut, signale_famille, created_by_user_id, created_at
            FROM ent_psy_rdv
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY jour DESC, heure_debut
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        rows.into_iter()
            .filter_map(|r| r.into_rdv(&self.champs).transpose())
            .collect()
    }

    pub async fn upsert_dossier(&self, d: &DossierPsy) -> Result<DossierPsy, DomainError> {
        // Lecture préalable, scellement, écriture et éventuelle reprise : une seule
        // transaction `TenantTx` (RLS).
        let mut tx = TenantTx::begin(&self.pool, d.etablissement_id).await.map_err(map_sqlx)?;
        // L'id de la ligne est conservé en cas de mise à jour : l'AAD doit l'utiliser.
        let existant: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_psy_dossier WHERE etablissement_id = $1 AND eleve_id = $2",
        )
        .bind(d.etablissement_id)
        .bind(d.eleve_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        let id_ligne = existant.map(|(i,)| i).unwrap_or(d.id);
        let (axes, contacts, note) = self.sceller_dossier(id_ligne, d)?;
        let row = sqlx::query_as::<_, DossierRow>(
            r#"
            INSERT INTO ent_psy_dossier (
                id, etablissement_id, eleve_id, axes_suivi, contacts_externes,
                note_confidentielle, suivi_actif, signale_famille,
                updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            ON CONFLICT (etablissement_id, eleve_id) DO UPDATE SET
                id = ent_psy_dossier.id,
                axes_suivi = EXCLUDED.axes_suivi,
                contacts_externes = EXCLUDED.contacts_externes,
                note_confidentielle = EXCLUDED.note_confidentielle,
                suivi_actif = EXCLUDED.suivi_actif,
                signale_famille = EXCLUDED.signale_famille,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at
            RETURNING id, etablissement_id, eleve_id, axes_suivi, contacts_externes,
                      note_confidentielle, suivi_actif, signale_famille,
                      updated_by_user_id, updated_at
            "#,
        )
        .bind(id_ligne)
        .bind(d.etablissement_id)
        .bind(d.eleve_id)
        .bind(&axes)
        .bind(&contacts)
        .bind(&note)
        .bind(d.suivi_actif)
        .bind(d.signale_famille)
        .bind(d.updated_by_user_id)
        .bind(d.updated_at)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        if row.id != id_ligne {
            // Course : ligne créée entre-temps sous un autre id → rechiffrement avec l'id stocké.
            let (axes, contacts, note) = self.sceller_dossier(row.id, d)?;
            sqlx::query(
                "UPDATE ent_psy_dossier SET axes_suivi = $2, contacts_externes = $3, \
                 note_confidentielle = $4 WHERE id = $1 AND etablissement_id = $5",
            )
            .bind(row.id)
            .bind(&axes)
            .bind(&contacts)
            .bind(&note)
            .bind(row.etablissement_id)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
            tx.commit().await.map_err(map_sqlx)?;
            return Ok(DossierPsy {
                id: row.id,
                ..d.clone()
            });
        }
        tx.commit().await.map_err(map_sqlx)?;
        row.into_dossier(&self.champs)
    }

    fn sceller_dossier(
        &self,
        id: Uuid,
        d: &DossierPsy,
    ) -> Result<(String, String, String), DomainError> {
        let c = &self.champs;
        Ok((
            c.sceller(T_DOSSIER, "axes_suivi", id, &d.axes_suivi)?,
            c.sceller(T_DOSSIER, "contacts_externes", id, &d.contacts_externes)?,
            c.sceller(T_DOSSIER, "note_confidentielle", id, &d.note_confidentielle)?,
        ))
    }

    pub async fn get_dossier(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<DossierPsy>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let row = sqlx::query_as::<_, DossierRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, axes_suivi, contacts_externes,
                   note_confidentielle, suivi_actif, signale_famille,
                   updated_by_user_id, updated_at
            FROM ent_psy_dossier
            WHERE etablissement_id = $1 AND eleve_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        row.map(|r| r.into_dossier(&self.champs)).transpose()
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DossierRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    axes_suivi: String,
    contacts_externes: String,
    note_confidentielle: String,
    suivi_actif: bool,
    signale_famille: bool,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl DossierRow {
    fn into_dossier(self, c: &ChampsSensibles) -> Result<DossierPsy, DomainError> {
        Ok(DossierPsy {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            axes_suivi: c.ouvrir(T_DOSSIER, "axes_suivi", self.id, &self.axes_suivi)?,
            contacts_externes: c.ouvrir(T_DOSSIER, "contacts_externes", self.id, &self.contacts_externes)?,
            note_confidentielle: c.ouvrir(T_DOSSIER, "note_confidentielle", self.id, &self.note_confidentielle)?,
            suivi_actif: self.suivi_actif,
            signale_famille: self.signale_famille,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct RdvRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    heure_debut: NaiveTime,
    heure_fin: NaiveTime,
    kind: String,
    motif: String,
    note_interne: String,
    statut: String,
    signale_famille: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl RdvRow {
    fn into_rdv(self, c: &ChampsSensibles) -> Result<Option<RdvPsy>, DomainError> {
        let motif = c.ouvrir(T_RDV, "motif", self.id, &self.motif)?;
        let note_interne = c.ouvrir(T_RDV, "note_interne", self.id, &self.note_interne)?;
        let (Some(kind), Some(statut)) = (
            EntretienPsyKind::parse(&self.kind),
            RdvPsyStatut::parse(&self.statut),
        ) else {
            return Ok(None);
        };
        Ok(Some(RdvPsy {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            jour: self.jour,
            heure_debut: self.heure_debut,
            heure_fin: self.heure_fin,
            kind,
            motif,
            note_interne,
            statut,
            signale_famille: self.signale_famille,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EntretienRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    kind: String,
    duree_minutes: i32,
    note_confidentielle: String,
    signale_famille: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl EntretienRow {
    fn into_entretien(self, c: &ChampsSensibles) -> Result<Option<EntretienPsy>, DomainError> {
        let note = c.ouvrir(T_ENTRETIEN, "note_confidentielle", self.id, &self.note_confidentielle)?;
        let Some(kind) = EntretienPsyKind::parse(&self.kind) else {
            return Ok(None);
        };
        Ok(Some(EntretienPsy {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            jour: self.jour,
            kind,
            duree_minutes: self.duree_minutes,
            note_confidentielle: note,
            signale_famille: self.signale_famille,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }))
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("psy_repo", err)
}
