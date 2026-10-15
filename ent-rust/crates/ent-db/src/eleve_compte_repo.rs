use chrono::{DateTime, Utc};
use domain_eleve::{DomainError, Eleve, EleveInvitation, InvitationStatut};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EleveCompte {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub user_id: Uuid,
}

pub struct PgEleveCompteRepo {
    pool: PgPool,
}

impl PgEleveCompteRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn link(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        user_id: Uuid,
    ) -> Result<EleveCompte, DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id, created_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (etablissement_id, eleve_id) DO UPDATE SET user_id = EXCLUDED.user_id
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(EleveCompte {
            etablissement_id,
            eleve_id,
            user_id,
        })
    }

    pub async fn eleve_id_for_user(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Uuid>, DomainError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT eleve_id FROM ent_eleve_compte
            WHERE etablissement_id = $1 AND user_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(|r| r.0))
    }

    pub async fn has_compte(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<bool, DomainError> {
        let row: Option<(i32,)> = sqlx::query_as(
            r#"
            SELECT 1 FROM ent_eleve_compte
            WHERE etablissement_id = $1 AND eleve_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.is_some())
    }

    pub async fn list_eleves_sans_compte(
        &self,
        etablissement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Eleve>, DomainError> {
        let rows = sqlx::query_as::<_, SansCompteRow>(
            r#"
            SELECT e.id, e.etablissement_id, e.nom, e.prenom, e.ine, e.sexe, e.date_naissance,
                   e.numero_badge, e.redoublant, e.photo_key, e.status, e.classe, e.regime,
                   e.mef, e.site, e.date_entree, e.date_sortie, e.motif_sortie
            FROM ent_eleve e
            LEFT JOIN ent_eleve_compte c
              ON c.etablissement_id = e.etablissement_id AND c.eleve_id = e.id
            WHERE e.etablissement_id = $1
              AND e.status = 'actif'
              AND c.eleve_id IS NULL
            ORDER BY e.nom, e.prenom
            LIMIT $2
            "#,
        )
        .bind(etablissement_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(SansCompteRow::into_eleve).collect())
    }

    pub async fn insert_invitation(&self, inv: &EleveInvitation) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_eleve_invitation (
                id, etablissement_id, eleve_id, email, statut,
                invited_by_user_id, activated_user_id, created_at, expires_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(inv.id)
        .bind(inv.etablissement_id)
        .bind(inv.eleve_id)
        .bind(&inv.email)
        .bind(inv.statut.as_str())
        .bind(inv.invited_by_user_id)
        .bind(inv.activated_user_id)
        .bind(inv.created_at)
        .bind(inv.expires_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_invitations(
        &self,
        etablissement_id: Uuid,
        limit: i64,
    ) -> Result<Vec<EleveInvitation>, DomainError> {
        let rows = sqlx::query_as::<_, InvitationRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, email, statut,
                   invited_by_user_id, activated_user_id, created_at, expires_at
            FROM ent_eleve_invitation
            WHERE etablissement_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(etablissement_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .filter_map(InvitationRow::into_invitation)
            .collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SansCompteRow {
    id: Uuid,
    etablissement_id: Uuid,
    nom: String,
    prenom: String,
    ine: Option<String>,
    sexe: Option<String>,
    date_naissance: Option<chrono::NaiveDate>,
    numero_badge: Option<String>,
    redoublant: bool,
    photo_key: Option<String>,
    status: String,
    classe: Option<String>,
    regime: Option<String>,
    mef: Option<String>,
    site: Option<String>,
    date_entree: Option<chrono::NaiveDate>,
    date_sortie: Option<chrono::NaiveDate>,
    motif_sortie: Option<String>,
}

impl SansCompteRow {
    fn into_eleve(self) -> Eleve {
        Eleve {
            id: self.id,
            etablissement_id: self.etablissement_id,
            nom: self.nom,
            prenom: self.prenom,
            ine: self.ine,
            sexe: self.sexe,
            date_naissance: self.date_naissance,
            numero_badge: self.numero_badge,
            redoublant: self.redoublant,
            photo_key: self.photo_key,
            status: match self.status.as_str() {
                "sorti" => domain_eleve::EleveStatus::Sorti,
                "preinscrit" => domain_eleve::EleveStatus::Preinscrit,
                _ => domain_eleve::EleveStatus::Actif,
            },
            classe: self.classe,
            regime: self.regime,
            mef: self.mef,
            site: self.site,
            date_entree: self.date_entree,
            date_sortie: self.date_sortie,
            motif_sortie: self.motif_sortie,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct InvitationRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    email: String,
    statut: String,
    invited_by_user_id: Uuid,
    activated_user_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl InvitationRow {
    fn into_invitation(self) -> Option<EleveInvitation> {
        let statut = InvitationStatut::parse(&self.statut)?;
        Some(EleveInvitation {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            email: self.email,
            statut,
            invited_by_user_id: self.invited_by_user_id,
            activated_user_id: self.activated_user_id,
            created_at: self.created_at,
            expires_at: self.expires_at,
        })
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] eleve_compte sqlx error: {err}");
    DomainError::NotFound
}
