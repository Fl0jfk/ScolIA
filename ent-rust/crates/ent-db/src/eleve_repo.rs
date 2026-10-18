use chrono::NaiveDate;
use domain_eleve::{DomainError, Eleve, EleveStatus};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgEleveRepo {
    pool: PgPool,
}

impl PgEleveRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<Eleve>, DomainError> {
        let row = sqlx::query_as::<_, EleveRow>(
            r#"
            SELECT id, etablissement_id, nom, prenom, ine, sexe, date_naissance, numero_badge,
                   redoublant, photo_key, status, classe, regime, mef, site,
                   date_entree, date_sortie, motif_sortie
            FROM ent_eleve
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;

        Ok(row.map(EleveRow::into_eleve))
    }

    pub async fn get_by_badge(
        &self,
        etablissement_id: Uuid,
        numero_badge: &str,
    ) -> Result<Option<Eleve>, DomainError> {
        let badge = numero_badge.trim();
        if badge.is_empty() {
            return Ok(None);
        }
        let row = sqlx::query_as::<_, EleveRow>(
            r#"
            SELECT id, etablissement_id, nom, prenom, ine, sexe, date_naissance, numero_badge,
                   redoublant, photo_key, status, classe, regime, mef, site,
                   date_entree, date_sortie, motif_sortie
            FROM ent_eleve
            WHERE etablissement_id = $1
              AND lower(numero_badge) = lower($2)
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(badge)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(EleveRow::into_eleve))
    }

    pub async fn labels(
        &self,
        etablissement_id: Uuid,
        eleve_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, DomainError> {
        if eleve_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(Uuid, String, String)> = sqlx::query_as(
            r#"
            SELECT id, prenom, nom
            FROM ent_eleve
            WHERE etablissement_id = $1 AND id = ANY($2)
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(|(id, prenom, nom)| {
                let label = format!("{} {}", prenom.trim(), nom.trim())
                    .trim()
                    .to_string();
                (id, if label.is_empty() { nom } else { label })
            })
            .collect())
    }

    pub async fn search(
        &self,
        etablissement_id: Uuid,
        q: &str,
        limit: i64,
    ) -> Result<Vec<Eleve>, DomainError> {
        let like = format!("%{}%", q.trim().to_lowercase());
        let rows = sqlx::query_as::<_, EleveRow>(
            r#"
            SELECT id, etablissement_id, nom, prenom, ine, sexe, date_naissance, numero_badge,
                   redoublant, photo_key, status, classe, regime, mef, site,
                   date_entree, date_sortie, motif_sortie
            FROM ent_eleve
            WHERE etablissement_id = $1
              AND (
                lower(nom) LIKE $2
                OR lower(prenom) LIKE $2
                OR lower(coalesce(classe, '')) LIKE $2
                OR lower(coalesce(numero_badge, '')) LIKE $2
              )
            ORDER BY nom, prenom
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(like)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(EleveRow::into_eleve).collect())
    }

    /// Effectif d'une classe **au jour J** = périodes + dates d'entrée/sortie.
    pub async fn list_en_classe_au(
        &self,
        etablissement_id: Uuid,
        classe: &str,
        jour: NaiveDate,
    ) -> Result<Vec<Eleve>, DomainError> {
        let rows = sqlx::query_as::<_, EleveRow>(
            r#"
            SELECT e.id, e.etablissement_id, e.nom, e.prenom, e.ine, e.sexe, e.date_naissance,
                   e.numero_badge, e.redoublant, e.photo_key, e.status, e.classe, e.regime,
                   e.mef, e.site, e.date_entree, e.date_sortie, e.motif_sortie
            FROM ent_eleve e
            INNER JOIN ent_eleve_periode p
                ON p.etablissement_id = e.etablissement_id
               AND p.eleve_id = e.id
               AND p.kind = 'classe'
               AND p.valeur = $2
               AND p.date_debut <= $3
               AND (p.date_fin IS NULL OR p.date_fin >= $3)
            WHERE e.etablissement_id = $1
              AND (e.date_entree IS NULL OR e.date_entree <= $3)
              AND (e.date_sortie IS NULL OR e.date_sortie > $3)
            ORDER BY e.nom, e.prenom
            "#,
        )
        .bind(etablissement_id)
        .bind(classe)
        .bind(jour)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;

        Ok(rows.into_iter().map(EleveRow::into_eleve).collect())
    }

    /// Effectif actif à une date donnée (entrées/sorties datées), sans jointure
    /// de période : réponse au « combien d'élèves ? » du copilote.
    pub async fn count_actifs_au(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<i64, DomainError> {
        let (n,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint
            FROM ent_eleve
            WHERE etablissement_id = $1
              AND (date_entree IS NULL OR date_entree <= $2)
              AND (date_sortie IS NULL OR date_sortie > $2)
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(n)
    }

    /// Liste des élèves actifs à une date (entrées/sorties datées) — base des
    /// diffusions établissement → familles.
    pub async fn list_actifs_au(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Eleve>, DomainError> {
        let rows = sqlx::query_as::<_, EleveRow>(
            r#"
            SELECT id, etablissement_id, nom, prenom, ine, sexe, date_naissance, numero_badge,
                   redoublant, photo_key, status, classe, regime, mef, site,
                   date_entree, date_sortie, motif_sortie
            FROM ent_eleve
            WHERE etablissement_id = $1
              AND (date_entree IS NULL OR date_entree <= $2)
              AND (date_sortie IS NULL OR date_sortie > $2)
            ORDER BY nom, prenom
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(EleveRow::into_eleve).collect())
    }

    pub async fn upsert(&self, eleve: &Eleve) -> Result<(), DomainError> {
        let result = sqlx::query(
            r#"
            INSERT INTO ent_eleve (
                id, etablissement_id, nom, prenom, ine, sexe, date_naissance, numero_badge,
                redoublant, photo_key, status, classe, regime, mef, site,
                date_entree, date_sortie, motif_sortie, updated_at
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18, now()
            )
            ON CONFLICT (id) DO UPDATE SET
                nom = EXCLUDED.nom,
                prenom = EXCLUDED.prenom,
                ine = EXCLUDED.ine,
                sexe = EXCLUDED.sexe,
                date_naissance = EXCLUDED.date_naissance,
                numero_badge = EXCLUDED.numero_badge,
                redoublant = EXCLUDED.redoublant,
                photo_key = EXCLUDED.photo_key,
                status = EXCLUDED.status,
                classe = EXCLUDED.classe,
                regime = EXCLUDED.regime,
                mef = EXCLUDED.mef,
                site = EXCLUDED.site,
                date_entree = EXCLUDED.date_entree,
                date_sortie = EXCLUDED.date_sortie,
                motif_sortie = EXCLUDED.motif_sortie,
                updated_at = now()
            WHERE ent_eleve.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(eleve.id)
        .bind(eleve.etablissement_id)
        .bind(&eleve.nom)
        .bind(&eleve.prenom)
        .bind(&eleve.ine)
        .bind(&eleve.sexe)
        .bind(eleve.date_naissance)
        .bind(&eleve.numero_badge)
        .bind(eleve.redoublant)
        .bind(&eleve.photo_key)
        .bind(eleve.status.as_str())
        .bind(&eleve.classe)
        .bind(&eleve.regime)
        .bind(&eleve.mef)
        .bind(&eleve.site)
        .bind(eleve.date_entree)
        .bind(eleve.date_sortie)
        .bind(&eleve.motif_sortie)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::TenantMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EleveRow {
    id: Uuid,
    etablissement_id: Uuid,
    nom: String,
    prenom: String,
    ine: Option<String>,
    sexe: Option<String>,
    date_naissance: Option<NaiveDate>,
    numero_badge: Option<String>,
    redoublant: bool,
    photo_key: Option<String>,
    status: String,
    classe: Option<String>,
    regime: Option<String>,
    mef: Option<String>,
    site: Option<String>,
    date_entree: Option<NaiveDate>,
    date_sortie: Option<NaiveDate>,
    motif_sortie: Option<String>,
}

impl EleveRow {
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
            status: parse_status(&self.status),
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

fn parse_status(s: &str) -> EleveStatus {
    match s {
        "sorti" => EleveStatus::Sorti,
        "preinscrit" => EleveStatus::Preinscrit,
        _ => EleveStatus::Actif,
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("eleve_repo", err)
}
