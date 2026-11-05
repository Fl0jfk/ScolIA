use chrono::NaiveDate;
use domain_eleve::DomainError;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppelStatut {
    Ouvert,
    Clos,
}

impl AppelStatut {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ouvert => "ouvert",
            Self::Clos => "clos",
        }
    }

    pub fn parse(s: &str) -> Self {
        if s == "clos" {
            Self::Clos
        } else {
            Self::Ouvert
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LigneEtat {
    Present,
    Absent,
    Retard,
}

impl LigneEtat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Absent => "absent",
            Self::Retard => "retard",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "present" => Some(Self::Present),
            "absent" => Some(Self::Absent),
            "retard" => Some(Self::Retard),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Appel {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub classe: String,
    pub jour: NaiveDate,
    pub auteur_user_id: Uuid,
    pub statut: AppelStatut,
    pub creneau_id: Option<Uuid>,
    pub matiere_libelle: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppelLigne {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub appel_id: Uuid,
    pub eleve_id: Uuid,
    pub etat: LigneEtat,
}

pub struct PgAppelRepo {
    pool: PgPool,
}

impl PgAppelRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_appel(&self, a: &Appel) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_appel (
                id, etablissement_id, classe, jour, auteur_user_id, statut,
                creneau_id, matiere_libelle
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(a.id)
        .bind(a.etablissement_id)
        .bind(&a.classe)
        .bind(a.jour)
        .bind(a.auteur_user_id)
        .bind(a.statut.as_str())
        .bind(a.creneau_id)
        .bind(a.matiere_libelle.as_deref())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
    ) -> Result<Option<Appel>, DomainError> {
        let row = sqlx::query_as::<_, AppelRow>(
            r#"
            SELECT id, etablissement_id, classe, jour, auteur_user_id, statut,
                   creneau_id, matiere_libelle
            FROM ent_appel
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(AppelRow::into_appel))
    }

    pub async fn get_by_creneau_jour(
        &self,
        etablissement_id: Uuid,
        creneau_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<Appel>, DomainError> {
        let row = sqlx::query_as::<_, AppelRow>(
            r#"
            SELECT id, etablissement_id, classe, jour, auteur_user_id, statut,
                   creneau_id, matiere_libelle
            FROM ent_appel
            WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3
            "#,
        )
        .bind(etablissement_id)
        .bind(creneau_id)
        .bind(jour)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(AppelRow::into_appel))
    }

    pub async fn insert_ligne(&self, l: &AppelLigne) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_appel_ligne (id, etablissement_id, appel_id, eleve_id, etat)
            VALUES ($1,$2,$3,$4,$5)
            ON CONFLICT (appel_id, eleve_id) DO UPDATE SET etat = EXCLUDED.etat
            "#,
        )
        .bind(l.id)
        .bind(l.etablissement_id)
        .bind(l.appel_id)
        .bind(l.eleve_id)
        .bind(l.etat.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    /// Modifie l'état d'une ligne **uniquement si l'appel est ouvert**, de façon
    /// sérialisée avec la clôture : verrou partagé (`FOR SHARE`) sur la ligne de
    /// l'appel, statut contrôlé sur la ligne verrouillée, écriture dans la même
    /// transaction. `FOR SHARE` (et non `FOR UPDATE`) suffit : il bloque la clôture
    /// (`FOR UPDATE`) tant que la modification n'est pas validée, sans sérialiser
    /// inutilement les marquages d'élèves différents. `Ok(false)` si l'appel est
    /// absent ou clos, ou si la ligne n'existe pas.
    pub async fn set_etat_si_ouvert(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
        eleve_id: Uuid,
        etat: &LigneEtat,
    ) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let statut: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT statut FROM ent_appel
            WHERE etablissement_id = $1 AND id = $2
            FOR SHARE
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_optional(&mut *tx)
        .await?;
        if statut.as_ref().map(|s| s.0.as_str()) != Some("ouvert") {
            return Ok(false);
        }
        // Défense en profondeur : la condition de statut est aussi portée par l'écriture.
        let result = sqlx::query(
            r#"
            UPDATE ent_appel_ligne l
            SET etat = $4
            WHERE l.etablissement_id = $1 AND l.appel_id = $2 AND l.eleve_id = $3
              AND EXISTS (
                SELECT 1 FROM ent_appel a
                WHERE a.id = l.appel_id AND a.etablissement_id = l.etablissement_id
                  AND a.statut = 'ouvert'
              )
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .bind(eleve_id)
        .bind(etat.as_str())
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(false);
        }
        tx.commit().await?;
        Ok(true)
    }

    /// Clôture atomique, dans la transaction de l'appelant : verrou exclusif
    /// (`FOR UPDATE`) sur l'appel, statut contrôlé, lignes lues **sous verrou**,
    /// passage à « clos ». L'appelant y écrit aussi absences, retards et
    /// événements (outbox) puis valide le tout en une fois. `Ok(None)` si absent
    /// ou déjà clos.
    pub async fn clore_avec_lignes_tx(
        &self,
        tx: &mut sqlx::PgConnection,
        etablissement_id: Uuid,
        appel_id: Uuid,
    ) -> Result<Option<Vec<AppelLigne>>, sqlx::Error> {
        let statut: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT statut FROM ent_appel
            WHERE etablissement_id = $1 AND id = $2
            FOR UPDATE
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_optional(&mut *tx)
        .await?;
        if statut.as_ref().map(|s| s.0.as_str()) != Some("ouvert") {
            return Ok(None);
        }
        let rows = sqlx::query_as::<_, LigneRow>(
            r#"
            SELECT id, etablissement_id, appel_id, eleve_id, etat
            FROM ent_appel_ligne
            WHERE etablissement_id = $1 AND appel_id = $2
            ORDER BY eleve_id
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_all(&mut *tx)
        .await?;
        let result = sqlx::query(
            r#"
            UPDATE ent_appel
            SET statut = 'clos', closed_at = now()
            WHERE etablissement_id = $1 AND id = $2 AND statut = 'ouvert'
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }
        Ok(Some(rows.into_iter().map(LigneRow::into_ligne).collect()))
    }

    pub async fn list_lignes(
        &self,
        etablissement_id: Uuid,
        appel_id: Uuid,
    ) -> Result<Vec<AppelLigne>, DomainError> {
        let rows = sqlx::query_as::<_, LigneRow>(
            r#"
            SELECT id, etablissement_id, appel_id, eleve_id, etat
            FROM ent_appel_ligne
            WHERE etablissement_id = $1 AND appel_id = $2
            ORDER BY eleve_id
            "#,
        )
        .bind(etablissement_id)
        .bind(appel_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(LigneRow::into_ligne).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AppelRow {
    id: Uuid,
    etablissement_id: Uuid,
    classe: String,
    jour: NaiveDate,
    auteur_user_id: Uuid,
    statut: String,
    creneau_id: Option<Uuid>,
    matiere_libelle: Option<String>,
}

impl AppelRow {
    fn into_appel(self) -> Appel {
        Appel {
            id: self.id,
            etablissement_id: self.etablissement_id,
            classe: self.classe,
            jour: self.jour,
            auteur_user_id: self.auteur_user_id,
            statut: AppelStatut::parse(&self.statut),
            creneau_id: self.creneau_id,
            matiere_libelle: self.matiere_libelle,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct LigneRow {
    id: Uuid,
    etablissement_id: Uuid,
    appel_id: Uuid,
    eleve_id: Uuid,
    etat: String,
}

impl LigneRow {
    fn into_ligne(self) -> AppelLigne {
        AppelLigne {
            id: self.id,
            etablissement_id: self.etablissement_id,
            appel_id: self.appel_id,
            eleve_id: self.eleve_id,
            etat: LigneEtat::parse(&self.etat).unwrap_or(LigneEtat::Present),
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("appel_repo", err)
}
