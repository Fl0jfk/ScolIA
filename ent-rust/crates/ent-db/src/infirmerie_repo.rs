use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{
    DomainError, FicheSante, Pai, PaiPiece, PassageInfirmerie, PassageInfirmerieMotif,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgInfirmerieRepo {
    pool: PgPool,
}

impl PgInfirmerieRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, p: &PassageInfirmerie) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_infirmerie_passage (
                id, etablissement_id, eleve_id, jour, motif, note_soins,
                visible_famille, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(p.id)
        .bind(p.etablissement_id)
        .bind(p.eleve_id)
        .bind(p.jour)
        .bind(p.motif.as_str())
        .bind(&p.note_soins)
        .bind(p.visible_famille)
        .bind(p.created_by_user_id)
        .bind(p.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        limit: i64,
    ) -> Result<Vec<PassageInfirmerie>, DomainError> {
        let rows = sqlx::query_as::<_, PassageRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, motif, note_soins,
                   visible_famille, created_by_user_id, created_at
            FROM ent_infirmerie_passage
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY jour DESC, created_at DESC
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(PassageRow::into_passage).collect())
    }

    pub async fn list_jour(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
        limit: i64,
    ) -> Result<Vec<PassageInfirmerie>, DomainError> {
        let rows = sqlx::query_as::<_, PassageRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, jour, motif, note_soins,
                   visible_famille, created_by_user_id, created_at
            FROM ent_infirmerie_passage
            WHERE etablissement_id = $1 AND jour = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(jour)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(PassageRow::into_passage).collect())
    }

    pub async fn upsert_fiche(&self, f: &FicheSante) -> Result<FicheSante, DomainError> {
        let row = sqlx::query_as::<_, FicheRow>(
            r#"
            INSERT INTO ent_fiche_sante (
                id, etablissement_id, eleve_id, allergies, traitements, contre_indications,
                pai_actif, medecin_nom, urgence_tel, note_confidentielle,
                updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            ON CONFLICT (etablissement_id, eleve_id) DO UPDATE SET
                id = ent_fiche_sante.id,
                allergies = EXCLUDED.allergies,
                traitements = EXCLUDED.traitements,
                contre_indications = EXCLUDED.contre_indications,
                pai_actif = EXCLUDED.pai_actif,
                medecin_nom = EXCLUDED.medecin_nom,
                urgence_tel = EXCLUDED.urgence_tel,
                note_confidentielle = EXCLUDED.note_confidentielle,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at
            RETURNING id, etablissement_id, eleve_id, allergies, traitements, contre_indications,
                      pai_actif, medecin_nom, urgence_tel, note_confidentielle,
                      updated_by_user_id, updated_at
            "#,
        )
        .bind(f.id)
        .bind(f.etablissement_id)
        .bind(f.eleve_id)
        .bind(&f.allergies)
        .bind(&f.traitements)
        .bind(&f.contre_indications)
        .bind(f.pai_actif)
        .bind(f.medecin_nom.as_deref())
        .bind(f.urgence_tel.as_deref())
        .bind(&f.note_confidentielle)
        .bind(f.updated_by_user_id)
        .bind(f.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.into_fiche())
    }

    pub async fn get_fiche(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<FicheSante>, DomainError> {
        let row = sqlx::query_as::<_, FicheRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, allergies, traitements, contre_indications,
                   pai_actif, medecin_nom, urgence_tel, note_confidentielle,
                   updated_by_user_id, updated_at
            FROM ent_fiche_sante
            WHERE etablissement_id = $1 AND eleve_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(FicheRow::into_fiche))
    }

    pub async fn upsert_pai(&self, p: &Pai) -> Result<Pai, DomainError> {
        let row = sqlx::query_as::<_, PaiRow>(
            r#"
            INSERT INTO ent_pai (
                id, etablissement_id, eleve_id, motif, mesures_accueil, protocole_urgence,
                note_interne, date_debut, date_fin, actif,
                signe_famille_at, signe_famille_user_id,
                updated_by_user_id, updated_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
            ON CONFLICT (etablissement_id, eleve_id) DO UPDATE SET
                id = ent_pai.id,
                motif = EXCLUDED.motif,
                mesures_accueil = EXCLUDED.mesures_accueil,
                protocole_urgence = EXCLUDED.protocole_urgence,
                note_interne = EXCLUDED.note_interne,
                date_debut = EXCLUDED.date_debut,
                date_fin = EXCLUDED.date_fin,
                actif = EXCLUDED.actif,
                signe_famille_at = EXCLUDED.signe_famille_at,
                signe_famille_user_id = EXCLUDED.signe_famille_user_id,
                updated_by_user_id = EXCLUDED.updated_by_user_id,
                updated_at = EXCLUDED.updated_at
            RETURNING id, etablissement_id, eleve_id, motif, mesures_accueil, protocole_urgence,
                      note_interne, date_debut, date_fin, actif,
                      signe_famille_at, signe_famille_user_id,
                      updated_by_user_id, updated_at
            "#,
        )
        .bind(p.id)
        .bind(p.etablissement_id)
        .bind(p.eleve_id)
        .bind(&p.motif)
        .bind(&p.mesures_accueil)
        .bind(&p.protocole_urgence)
        .bind(&p.note_interne)
        .bind(p.date_debut)
        .bind(p.date_fin)
        .bind(p.actif)
        .bind(p.signe_famille_at)
        .bind(p.signe_famille_user_id)
        .bind(p.updated_by_user_id)
        .bind(p.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.into_pai())
    }

    pub async fn update_pai_signature(&self, p: &Pai) -> Result<Pai, DomainError> {
        let row = sqlx::query_as::<_, PaiRow>(
            r#"
            UPDATE ent_pai SET
                signe_famille_at = $3,
                signe_famille_user_id = $4,
                updated_at = $5
            WHERE etablissement_id = $1 AND eleve_id = $2
            RETURNING id, etablissement_id, eleve_id, motif, mesures_accueil, protocole_urgence,
                      note_interne, date_debut, date_fin, actif,
                      signe_famille_at, signe_famille_user_id,
                      updated_by_user_id, updated_at
            "#,
        )
        .bind(p.etablissement_id)
        .bind(p.eleve_id)
        .bind(p.signe_famille_at)
        .bind(p.signe_famille_user_id)
        .bind(Utc::now())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?
        .ok_or(DomainError::NotFound)?;
        Ok(row.into_pai())
    }

    pub async fn get_pai(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Option<Pai>, DomainError> {
        let row = sqlx::query_as::<_, PaiRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, motif, mesures_accueil, protocole_urgence,
                   note_interne, date_debut, date_fin, actif,
                   signe_famille_at, signe_famille_user_id,
                   updated_by_user_id, updated_at
            FROM ent_pai
            WHERE etablissement_id = $1 AND eleve_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(PaiRow::into_pai))
    }

    pub async fn insert_pai_piece(&self, piece: &PaiPiece) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_pai_piece (
                id, etablissement_id, eleve_id, pai_id, nom, mime_type,
                taille_octets, contenu, storage_key, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(piece.id)
        .bind(piece.etablissement_id)
        .bind(piece.eleve_id)
        .bind(piece.pai_id)
        .bind(&piece.nom)
        .bind(&piece.mime_type)
        .bind(piece.taille_octets)
        .bind(&piece.contenu)
        .bind(&piece.storage_key)
        .bind(piece.created_by_user_id)
        .bind(piece.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_pai_pieces(
        &self,
        etablissement_id: Uuid,
        pai_id: Uuid,
    ) -> Result<Vec<PaiPiece>, DomainError> {
        let rows = sqlx::query_as::<_, PaiPieceRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, pai_id, nom, mime_type,
                   taille_octets, COALESCE(contenu, '\x'::bytea) AS contenu,
                   storage_key, created_by_user_id, created_at
            FROM ent_pai_piece
            WHERE etablissement_id = $1 AND pai_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(pai_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(PaiPieceRow::into_piece).collect())
    }

    pub async fn get_pai_piece(
        &self,
        etablissement_id: Uuid,
        piece_id: Uuid,
    ) -> Result<Option<PaiPiece>, DomainError> {
        let row = sqlx::query_as::<_, PaiPieceRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, pai_id, nom, mime_type,
                   taille_octets, COALESCE(contenu, '\x'::bytea) AS contenu,
                   storage_key, created_by_user_id, created_at
            FROM ent_pai_piece
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(piece_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(PaiPieceRow::into_piece))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PaiPieceRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    pai_id: Uuid,
    nom: String,
    mime_type: String,
    taille_octets: i32,
    contenu: Vec<u8>,
    storage_key: Option<String>,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl PaiPieceRow {
    fn into_piece(self) -> PaiPiece {
        PaiPiece {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            pai_id: self.pai_id,
            nom: self.nom,
            mime_type: self.mime_type,
            taille_octets: self.taille_octets,
            contenu: self.contenu,
            storage_key: self.storage_key,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PaiRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    motif: String,
    mesures_accueil: String,
    protocole_urgence: String,
    note_interne: String,
    date_debut: NaiveDate,
    date_fin: Option<NaiveDate>,
    actif: bool,
    signe_famille_at: Option<DateTime<Utc>>,
    signe_famille_user_id: Option<Uuid>,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl PaiRow {
    fn into_pai(self) -> Pai {
        Pai {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            motif: self.motif,
            mesures_accueil: self.mesures_accueil,
            protocole_urgence: self.protocole_urgence,
            note_interne: self.note_interne,
            date_debut: self.date_debut,
            date_fin: self.date_fin,
            actif: self.actif,
            signe_famille_at: self.signe_famille_at,
            signe_famille_user_id: self.signe_famille_user_id,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct FicheRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    allergies: String,
    traitements: String,
    contre_indications: String,
    pai_actif: bool,
    medecin_nom: Option<String>,
    urgence_tel: Option<String>,
    note_confidentielle: String,
    updated_by_user_id: Uuid,
    updated_at: DateTime<Utc>,
}

impl FicheRow {
    fn into_fiche(self) -> FicheSante {
        FicheSante {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            allergies: self.allergies,
            traitements: self.traitements,
            contre_indications: self.contre_indications,
            pai_actif: self.pai_actif,
            medecin_nom: self.medecin_nom,
            urgence_tel: self.urgence_tel,
            note_confidentielle: self.note_confidentielle,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PassageRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    motif: String,
    note_soins: String,
    visible_famille: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl PassageRow {
    fn into_passage(self) -> Option<PassageInfirmerie> {
        let motif = PassageInfirmerieMotif::parse(&self.motif)?;
        Some(PassageInfirmerie {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            jour: self.jour,
            motif,
            note_soins: self.note_soins,
            visible_famille: self.visible_famille,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        })
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] infirmerie sqlx error: {err}");
    DomainError::NotFound
}
