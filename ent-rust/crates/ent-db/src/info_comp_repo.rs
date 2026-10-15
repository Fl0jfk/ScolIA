use chrono::{DateTime, Utc};
use domain_eleve::{DomainError, InfoCompChamp, InfoCompType, InfoCompValeur};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgInfoCompRepo {
    pool: PgPool,
}

impl PgInfoCompRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_champ(&self, c: &InfoCompChamp) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_info_comp_champ (
                id, etablissement_id, code, libelle, type_valeur, obligatoire,
                visible_famille, actif, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(c.id)
        .bind(c.etablissement_id)
        .bind(&c.code)
        .bind(&c.libelle)
        .bind(c.type_valeur.as_str())
        .bind(c.obligatoire)
        .bind(c.visible_famille)
        .bind(c.actif)
        .bind(c.created_by_user_id)
        .bind(c.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_champ(
        &self,
        etablissement_id: Uuid,
        champ_id: Uuid,
    ) -> Result<Option<InfoCompChamp>, DomainError> {
        let row = sqlx::query_as::<_, ChampRow>(
            r#"
            SELECT id, etablissement_id, code, libelle, type_valeur, obligatoire,
                   visible_famille, actif, created_by_user_id, created_at
            FROM ent_info_comp_champ
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(champ_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(ChampRow::into_champ))
    }

    pub async fn list_champs(
        &self,
        etablissement_id: Uuid,
        actifs_seulement: bool,
    ) -> Result<Vec<InfoCompChamp>, DomainError> {
        let rows = if actifs_seulement {
            sqlx::query_as::<_, ChampRow>(
                r#"
                SELECT id, etablissement_id, code, libelle, type_valeur, obligatoire,
                       visible_famille, actif, created_by_user_id, created_at
                FROM ent_info_comp_champ
                WHERE etablissement_id = $1 AND actif = true
                ORDER BY libelle
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, ChampRow>(
                r#"
                SELECT id, etablissement_id, code, libelle, type_valeur, obligatoire,
                       visible_famille, actif, created_by_user_id, created_at
                FROM ent_info_comp_champ
                WHERE etablissement_id = $1
                ORDER BY libelle
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().filter_map(ChampRow::into_champ).collect())
    }

    pub async fn upsert_valeur(&self, v: &InfoCompValeur) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_info_comp_valeur (
                id, etablissement_id, eleve_id, champ_id, valeur,
                updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7)
            ON CONFLICT (etablissement_id, eleve_id, champ_id) DO UPDATE SET
                valeur = EXCLUDED.valeur,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at,
                id = ent_info_comp_valeur.id
            "#,
        )
        .bind(v.id)
        .bind(v.etablissement_id)
        .bind(v.eleve_id)
        .bind(v.champ_id)
        .bind(&v.valeur)
        .bind(v.updated_by_user_id)
        .bind(v.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_valeurs_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<InfoCompValeur>, DomainError> {
        let rows = sqlx::query_as::<_, ValRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, champ_id, valeur,
                   updated_by_user_id, updated_at
            FROM ent_info_comp_valeur
            WHERE etablissement_id = $1 AND eleve_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(ValRow::into_val).collect())
    }
}

fn map_sqlx(e: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] info_comp sqlx: {e}");
    DomainError::NotFound
}

#[derive(sqlx::FromRow)]
struct ChampRow {
    id: Uuid,
    etablissement_id: Uuid,
    code: String,
    libelle: String,
    type_valeur: String,
    obligatoire: bool,
    visible_famille: bool,
    actif: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl ChampRow {
    fn into_champ(self) -> Option<InfoCompChamp> {
        Some(InfoCompChamp {
            id: self.id,
            etablissement_id: self.etablissement_id,
            code: self.code,
            libelle: self.libelle,
            type_valeur: InfoCompType::parse(&self.type_valeur)?,
            obligatoire: self.obligatoire,
            visible_famille: self.visible_famille,
            actif: self.actif,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ValRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    champ_id: Uuid,
    valeur: String,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl ValRow {
    fn into_val(self) -> InfoCompValeur {
        InfoCompValeur {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            champ_id: self.champ_id,
            valeur: self.valeur,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        }
    }
}
