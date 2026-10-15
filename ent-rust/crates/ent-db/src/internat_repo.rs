use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{
    AffectationInternat, ChambreGenre, ChambreInternat, DomainError, PresenceInternat,
    PresenceInternatStatut, SuiviInternat,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgInternatRepo {
    pool: PgPool,
}

impl PgInternatRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_chambre(&self, c: &ChambreInternat) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_internat_chambre (
                id, etablissement_id, batiment, etage, code, capacite, genre,
                actif, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(c.id)
        .bind(c.etablissement_id)
        .bind(&c.batiment)
        .bind(c.etage)
        .bind(&c.code)
        .bind(c.capacite)
        .bind(c.genre.as_str())
        .bind(c.actif)
        .bind(c.created_by_user_id)
        .bind(c.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_chambre(
        &self,
        etablissement_id: Uuid,
        chambre_id: Uuid,
    ) -> Result<Option<ChambreInternat>, DomainError> {
        let row = sqlx::query_as::<_, ChambreRow>(
            r#"
            SELECT id, etablissement_id, batiment, etage, code, capacite, genre,
                   actif, created_by_user_id, created_at
            FROM ent_internat_chambre
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(chambre_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(ChambreRow::into_chambre))
    }

    pub async fn list_chambres(
        &self,
        etablissement_id: Uuid,
        actifs_seulement: bool,
    ) -> Result<Vec<ChambreInternat>, DomainError> {
        let rows = if actifs_seulement {
            sqlx::query_as::<_, ChambreRow>(
                r#"
                SELECT id, etablissement_id, batiment, etage, code, capacite, genre,
                       actif, created_by_user_id, created_at
                FROM ent_internat_chambre
                WHERE etablissement_id = $1 AND actif = true
                ORDER BY batiment, etage, code
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, ChambreRow>(
                r#"
                SELECT id, etablissement_id, batiment, etage, code, capacite, genre,
                       actif, created_by_user_id, created_at
                FROM ent_internat_chambre
                WHERE etablissement_id = $1
                ORDER BY batiment, etage, code
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().filter_map(ChambreRow::into_chambre).collect())
    }

    pub async fn count_occupants_actifs(
        &self,
        etablissement_id: Uuid,
        chambre_id: Uuid,
        jour: NaiveDate,
    ) -> Result<i16, DomainError> {
        let n: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM ent_internat_affectation
            WHERE etablissement_id = $1
              AND chambre_id = $2
              AND debut <= $3
              AND (fin IS NULL OR fin >= $3)
            "#,
        )
        .bind(etablissement_id)
        .bind(chambre_id)
        .bind(jour)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(n as i16)
    }

    pub async fn insert_affectation(&self, a: &AffectationInternat) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_internat_affectation (
                id, etablissement_id, eleve_id, chambre_id, lit, debut, fin,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(a.id)
        .bind(a.etablissement_id)
        .bind(a.eleve_id)
        .bind(a.chambre_id)
        .bind(&a.lit)
        .bind(a.debut)
        .bind(a.fin)
        .bind(a.created_by_user_id)
        .bind(a.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn update_affectation_fin(&self, a: &AffectationInternat) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_internat_affectation
            SET fin = $3
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(a.etablissement_id)
        .bind(a.id)
        .bind(a.fin)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_affectation(
        &self,
        etablissement_id: Uuid,
        affectation_id: Uuid,
    ) -> Result<Option<AffectationInternat>, DomainError> {
        let row = sqlx::query_as::<_, AffRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, chambre_id, lit, debut, fin,
                   created_by_user_id, created_at
            FROM ent_internat_affectation
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(affectation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(AffRow::into_aff))
    }

    pub async fn get_affectation_active(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<AffectationInternat>, DomainError> {
        let row = sqlx::query_as::<_, AffRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, chambre_id, lit, debut, fin,
                   created_by_user_id, created_at
            FROM ent_internat_affectation
            WHERE etablissement_id = $1 AND eleve_id = $2
              AND debut <= $3 AND (fin IS NULL OR fin >= $3)
            ORDER BY debut DESC
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(jour)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(AffRow::into_aff))
    }

    /// Affectations actives pour un jour (appel du soir / grille internat).
    pub async fn list_affectations_actives(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<AffectationInternat>, DomainError> {
        let rows = sqlx::query_as::<_, AffRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, chambre_id, lit, debut, fin,
                   created_by_user_id, created_at
            FROM ent_internat_affectation
            WHERE etablissement_id = $1
              AND debut <= $2 AND (fin IS NULL OR fin >= $2)
            ORDER BY chambre_id, lit NULLS LAST, debut
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(AffRow::into_aff).collect())
    }

    pub async fn has_affectation_ouverte(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<bool, DomainError> {
        let n: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM ent_internat_affectation
            WHERE etablissement_id = $1 AND eleve_id = $2 AND fin IS NULL
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(n > 0)
    }

    pub async fn upsert_presence(&self, p: &PresenceInternat) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_internat_presence (
                id, etablissement_id, eleve_id, affectation_id, jour, statut,
                note, actor_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            ON CONFLICT (etablissement_id, eleve_id, jour) DO UPDATE SET
                affectation_id = EXCLUDED.affectation_id,
                statut = EXCLUDED.statut,
                note = EXCLUDED.note,
                actor_user_id = EXCLUDED.actor_user_id,
                created_at = EXCLUDED.created_at,
                id = ent_internat_presence.id
            "#,
        )
        .bind(p.id)
        .bind(p.etablissement_id)
        .bind(p.eleve_id)
        .bind(p.affectation_id)
        .bind(p.jour)
        .bind(p.statut.as_str())
        .bind(&p.note)
        .bind(p.actor_user_id)
        .bind(p.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_presence(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<PresenceInternat>, DomainError> {
        let row = sqlx::query_as::<_, PresenceRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, affectation_id, jour, statut,
                   note, actor_user_id, created_at
            FROM ent_internat_presence
            WHERE etablissement_id = $1 AND eleve_id = $2 AND jour = $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(jour)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(PresenceRow::into_presence))
    }

    pub async fn list_presences_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<PresenceInternat>, DomainError> {
        let rows = sqlx::query_as::<_, PresenceRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, affectation_id, jour, statut,
                   note, actor_user_id, created_at
            FROM ent_internat_presence
            WHERE etablissement_id = $1 AND jour = $2
            ORDER BY created_at
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .filter_map(PresenceRow::into_presence)
            .collect())
    }

    pub async fn insert_suivi(&self, s: &SuiviInternat) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_internat_suivi (
                id, etablissement_id, eleve_id, jour, texte, visible_famille,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(s.id)
        .bind(s.etablissement_id)
        .bind(s.eleve_id)
        .bind(s.jour)
        .bind(&s.texte)
        .bind(s.visible_famille)
        .bind(s.created_by_user_id)
        .bind(s.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_suivis_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        famille_seulement: bool,
    ) -> Result<Vec<SuiviInternat>, DomainError> {
        let rows = if famille_seulement {
            sqlx::query_as::<_, SuiviRow>(
                r#"
                SELECT id, etablissement_id, eleve_id, jour, texte, visible_famille,
                       created_by_user_id, created_at
                FROM ent_internat_suivi
                WHERE etablissement_id = $1 AND eleve_id = $2 AND visible_famille = true
                ORDER BY jour DESC, created_at DESC
                "#,
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, SuiviRow>(
                r#"
                SELECT id, etablissement_id, eleve_id, jour, texte, visible_famille,
                       created_by_user_id, created_at
                FROM ent_internat_suivi
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
        Ok(rows.into_iter().map(SuiviRow::into_suivi).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] internat sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct ChambreRow {
    id: Uuid,
    etablissement_id: Uuid,
    batiment: String,
    etage: i16,
    code: String,
    capacite: i16,
    genre: String,
    actif: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl ChambreRow {
    fn into_chambre(self) -> Option<ChambreInternat> {
        Some(ChambreInternat {
            id: self.id,
            etablissement_id: self.etablissement_id,
            batiment: self.batiment,
            etage: self.etage,
            code: self.code,
            capacite: self.capacite,
            genre: ChambreGenre::parse(&self.genre)?,
            actif: self.actif,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct AffRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    chambre_id: Uuid,
    lit: Option<String>,
    debut: NaiveDate,
    fin: Option<NaiveDate>,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl AffRow {
    fn into_aff(self) -> AffectationInternat {
        AffectationInternat {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            chambre_id: self.chambre_id,
            lit: self.lit,
            debut: self.debut,
            fin: self.fin,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PresenceRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    affectation_id: Uuid,
    jour: NaiveDate,
    statut: String,
    note: String,
    actor_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl PresenceRow {
    fn into_presence(self) -> Option<PresenceInternat> {
        Some(PresenceInternat {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            affectation_id: self.affectation_id,
            jour: self.jour,
            statut: PresenceInternatStatut::parse(&self.statut)?,
            note: self.note,
            actor_user_id: self.actor_user_id,
            created_at: self.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct SuiviRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    texte: String,
    visible_famille: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl SuiviRow {
    fn into_suivi(self) -> SuiviInternat {
        SuiviInternat {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            jour: self.jour,
            texte: self.texte,
            visible_famille: self.visible_famille,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}
