use chrono::{DateTime, Utc};
use domain_eleve::{
    DomainError, Message, MsgAccuse, MsgFil, MsgModele, MsgPiece, MsgPieceMeta, MsgPriorite,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgMsgRepo {
    pool: PgPool,
}

impl PgMsgRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_fil(&self, fil: &MsgFil) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_msg_fil (
                id, etablissement_id, eleve_id, sujet, created_by_user_id,
                created_at, last_message_at, archive, modele_id, priorite, dossier
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(fil.id)
        .bind(fil.etablissement_id)
        .bind(fil.eleve_id)
        .bind(&fil.sujet)
        .bind(fil.created_by_user_id)
        .bind(fil.created_at)
        .bind(fil.last_message_at)
        .bind(fil.archive)
        .bind(fil.modele_id)
        .bind(fil.priorite.as_str())
        .bind(fil.dossier.as_deref())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn set_fil_archive(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        archive: bool,
        archived_at: Option<DateTime<Utc>>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_msg_fil
            SET archive = $3, archived_at = $4
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(archive)
        .bind(archived_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn set_fil_boite(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        priorite: MsgPriorite,
        dossier: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_msg_fil
            SET priorite = $3, dossier = $4
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(priorite.as_str())
        .bind(dossier)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn insert_modele(&self, m: &MsgModele) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_msg_modele (
                id, etablissement_id, titre, sujet, corps, actif,
                created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(m.id)
        .bind(m.etablissement_id)
        .bind(&m.titre)
        .bind(&m.sujet)
        .bind(&m.corps)
        .bind(m.actif)
        .bind(m.created_by_user_id)
        .bind(m.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_modele(
        &self,
        etablissement_id: Uuid,
        modele_id: Uuid,
    ) -> Result<Option<MsgModele>, DomainError> {
        let row = sqlx::query_as::<_, ModeleRow>(
            r#"
            SELECT id, etablissement_id, titre, sujet, corps, actif,
                   created_by_user_id, created_at
            FROM ent_msg_modele
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(modele_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(ModeleRow::into_modele))
    }

    pub async fn list_modeles(
        &self,
        etablissement_id: Uuid,
        actifs_seulement: bool,
    ) -> Result<Vec<MsgModele>, DomainError> {
        let rows = if actifs_seulement {
            sqlx::query_as::<_, ModeleRow>(
                r#"
                SELECT id, etablissement_id, titre, sujet, corps, actif,
                       created_by_user_id, created_at
                FROM ent_msg_modele
                WHERE etablissement_id = $1 AND actif = true
                ORDER BY titre ASC
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, ModeleRow>(
                r#"
                SELECT id, etablissement_id, titre, sujet, corps, actif,
                       created_by_user_id, created_at
                FROM ent_msg_modele
                WHERE etablissement_id = $1
                ORDER BY titre ASC
                "#,
            )
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().map(ModeleRow::into_modele).collect())
    }

    pub async fn insert_message(&self, msg: &Message) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_msg (
                id, etablissement_id, fil_id, auteur_user_id, corps, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6)
            "#,
        )
        .bind(msg.id)
        .bind(msg.etablissement_id)
        .bind(msg.fil_id)
        .bind(msg.auteur_user_id)
        .bind(&msg.corps)
        .bind(msg.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn touch_fil_last(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            UPDATE ent_msg_fil SET last_message_at = $3
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .bind(at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_fil(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<Option<MsgFil>, DomainError> {
        let row = sqlx::query_as::<_, FilRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, sujet, created_by_user_id,
                   created_at, last_message_at, archive, modele_id, priorite, dossier
            FROM ent_msg_fil
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(FilRow::into_fil))
    }

    pub async fn list_fils_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<MsgFil>, DomainError> {
        let rows = sqlx::query_as::<_, FilRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, sujet, created_by_user_id,
                   created_at, last_message_at, archive, modele_id, priorite, dossier
            FROM ent_msg_fil
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY CASE priorite WHEN 'haute' THEN 0 ELSE 1 END, last_message_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(FilRow::into_fil).collect())
    }

    pub async fn list_fils_eleves(
        &self,
        etablissement_id: Uuid,
        eleve_ids: &[Uuid],
    ) -> Result<Vec<MsgFil>, DomainError> {
        if eleve_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, FilRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, sujet, created_by_user_id,
                   created_at, last_message_at, archive, modele_id, priorite, dossier
            FROM ent_msg_fil
            WHERE etablissement_id = $1 AND eleve_id = ANY($2)
            ORDER BY CASE priorite WHEN 'haute' THEN 0 ELSE 1 END, last_message_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(FilRow::into_fil).collect())
    }

    /// Boîte établissement : par défaut hors archive ; `include_archives` pour tout voir.
    /// `dossier` filtre optionnel (`Some("")` ou None = tous).
    pub async fn list_fils_etab(
        &self,
        etablissement_id: Uuid,
        limit: i64,
        include_archives: bool,
        dossier: Option<&str>,
    ) -> Result<Vec<MsgFil>, DomainError> {
        let dossier_filter = dossier.map(str::trim).filter(|d| !d.is_empty());
        let rows = if include_archives {
            sqlx::query_as::<_, FilRow>(
                r#"
                SELECT id, etablissement_id, eleve_id, sujet, created_by_user_id,
                       created_at, last_message_at, archive, modele_id, priorite, dossier
                FROM ent_msg_fil
                WHERE etablissement_id = $1
                  AND ($3::text IS NULL OR dossier = $3)
                ORDER BY CASE priorite WHEN 'haute' THEN 0 ELSE 1 END, last_message_at DESC
                LIMIT $2
                "#,
            )
            .bind(etablissement_id)
            .bind(limit)
            .bind(dossier_filter)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        } else {
            sqlx::query_as::<_, FilRow>(
                r#"
                SELECT id, etablissement_id, eleve_id, sujet, created_by_user_id,
                       created_at, last_message_at, archive, modele_id, priorite, dossier
                FROM ent_msg_fil
                WHERE etablissement_id = $1 AND archive = false
                  AND ($3::text IS NULL OR dossier = $3)
                ORDER BY CASE priorite WHEN 'haute' THEN 0 ELSE 1 END, last_message_at DESC
                LIMIT $2
                "#,
            )
            .bind(etablissement_id)
            .bind(limit)
            .bind(dossier_filter)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?
        };
        Ok(rows.into_iter().filter_map(FilRow::into_fil).collect())
    }

    pub async fn list_messages(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<Vec<Message>, DomainError> {
        let rows = sqlx::query_as::<_, MsgRow>(
            r#"
            SELECT id, etablissement_id, fil_id, auteur_user_id, corps, created_at
            FROM ent_msg
            WHERE etablissement_id = $1 AND fil_id = $2
            ORDER BY created_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(MsgRow::into_msg).collect())
    }

    /// Insert accusés ; ignore les doublons (message_id, lecteur_user_id).
    pub async fn upsert_accuses(&self, accuses: &[MsgAccuse]) -> Result<usize, DomainError> {
        let mut inserted = 0usize;
        for a in accuses {
            let res = sqlx::query(
                r#"
                INSERT INTO ent_msg_accuse (
                    id, etablissement_id, fil_id, message_id, lecteur_user_id, lu_at
                ) VALUES ($1,$2,$3,$4,$5,$6)
                ON CONFLICT (message_id, lecteur_user_id) DO NOTHING
                "#,
            )
            .bind(a.id)
            .bind(a.etablissement_id)
            .bind(a.fil_id)
            .bind(a.message_id)
            .bind(a.lecteur_user_id)
            .bind(a.lu_at)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
            inserted += res.rows_affected() as usize;
        }
        Ok(inserted)
    }

    pub async fn list_accuses_fil(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<Vec<MsgAccuse>, DomainError> {
        let rows = sqlx::query_as::<_, AccuseRow>(
            r#"
            SELECT id, etablissement_id, fil_id, message_id, lecteur_user_id, lu_at
            FROM ent_msg_accuse
            WHERE etablissement_id = $1 AND fil_id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(AccuseRow::into_accuse).collect())
    }

    /// Messages non accusés par le lecteur (hors ses propres messages), par fil.
    pub async fn count_non_lus_par_fils(
        &self,
        etablissement_id: Uuid,
        lecteur_user_id: Uuid,
        fil_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, i64>, DomainError> {
        if fil_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows = sqlx::query_as::<_, (Uuid, i64)>(
            r#"
            SELECT m.fil_id, COUNT(*)::bigint
            FROM ent_msg m
            WHERE m.etablissement_id = $1
              AND m.fil_id = ANY($2)
              AND m.auteur_user_id <> $3
              AND NOT EXISTS (
                SELECT 1 FROM ent_msg_accuse a
                WHERE a.message_id = m.id AND a.lecteur_user_id = $3
              )
            GROUP BY m.fil_id
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_ids)
        .bind(lecteur_user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().collect())
    }

    pub async fn insert_piece(&self, piece: &MsgPiece) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_msg_piece (
                id, etablissement_id, fil_id, message_id, nom, mime_type,
                taille_octets, contenu, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(piece.id)
        .bind(piece.etablissement_id)
        .bind(piece.fil_id)
        .bind(piece.message_id)
        .bind(&piece.nom)
        .bind(&piece.mime_type)
        .bind(piece.taille_octets)
        .bind(&piece.contenu)
        .bind(piece.created_by_user_id)
        .bind(piece.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_pieces_meta_fil(
        &self,
        etablissement_id: Uuid,
        fil_id: Uuid,
    ) -> Result<Vec<MsgPieceMeta>, DomainError> {
        let rows = sqlx::query_as::<_, PieceMetaRow>(
            r#"
            SELECT id, message_id, nom, mime_type, taille_octets,
                   created_by_user_id, created_at
            FROM ent_msg_piece
            WHERE etablissement_id = $1 AND fil_id = $2
            ORDER BY created_at ASC
            "#,
        )
        .bind(etablissement_id)
        .bind(fil_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(PieceMetaRow::into_meta).collect())
    }

    pub async fn get_piece(
        &self,
        etablissement_id: Uuid,
        piece_id: Uuid,
    ) -> Result<Option<MsgPiece>, DomainError> {
        let row = sqlx::query_as::<_, PieceRow>(
            r#"
            SELECT id, etablissement_id, fil_id, message_id, nom, mime_type,
                   taille_octets, contenu, created_by_user_id, created_at
            FROM ent_msg_piece
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(piece_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(PieceRow::into_piece))
    }

    pub async fn get_message(
        &self,
        etablissement_id: Uuid,
        message_id: Uuid,
    ) -> Result<Option<Message>, DomainError> {
        let row = sqlx::query_as::<_, MsgRow>(
            r#"
            SELECT id, etablissement_id, fil_id, auteur_user_id, corps, created_at
            FROM ent_msg
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(MsgRow::into_msg))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct FilRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    sujet: String,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
    last_message_at: DateTime<Utc>,
    archive: bool,
    modele_id: Option<Uuid>,
    priorite: String,
    dossier: Option<String>,
}

impl FilRow {
    fn into_fil(self) -> Option<MsgFil> {
        let priorite = MsgPriorite::parse(&self.priorite)?;
        Some(MsgFil {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            sujet: self.sujet,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
            last_message_at: self.last_message_at,
            archive: self.archive,
            modele_id: self.modele_id,
            priorite,
            dossier: self.dossier,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ModeleRow {
    id: Uuid,
    etablissement_id: Uuid,
    titre: String,
    sujet: String,
    corps: String,
    actif: bool,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl ModeleRow {
    fn into_modele(self) -> MsgModele {
        MsgModele {
            id: self.id,
            etablissement_id: self.etablissement_id,
            titre: self.titre,
            sujet: self.sujet,
            corps: self.corps,
            actif: self.actif,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct MsgRow {
    id: Uuid,
    etablissement_id: Uuid,
    fil_id: Uuid,
    auteur_user_id: Uuid,
    corps: String,
    created_at: DateTime<Utc>,
}

impl MsgRow {
    fn into_msg(self) -> Message {
        Message {
            id: self.id,
            etablissement_id: self.etablissement_id,
            fil_id: self.fil_id,
            auteur_user_id: self.auteur_user_id,
            corps: self.corps,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AccuseRow {
    id: Uuid,
    etablissement_id: Uuid,
    fil_id: Uuid,
    message_id: Uuid,
    lecteur_user_id: Uuid,
    lu_at: DateTime<Utc>,
}

impl AccuseRow {
    fn into_accuse(self) -> MsgAccuse {
        MsgAccuse {
            id: self.id,
            etablissement_id: self.etablissement_id,
            fil_id: self.fil_id,
            message_id: self.message_id,
            lecteur_user_id: self.lecteur_user_id,
            lu_at: self.lu_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PieceMetaRow {
    id: Uuid,
    message_id: Uuid,
    nom: String,
    mime_type: String,
    taille_octets: i32,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl PieceMetaRow {
    fn into_meta(self) -> MsgPieceMeta {
        MsgPieceMeta {
            id: self.id,
            message_id: self.message_id,
            nom: self.nom,
            mime_type: self.mime_type,
            taille_octets: self.taille_octets,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PieceRow {
    id: Uuid,
    etablissement_id: Uuid,
    fil_id: Uuid,
    message_id: Uuid,
    nom: String,
    mime_type: String,
    taille_octets: i32,
    contenu: Vec<u8>,
    created_by_user_id: Uuid,
    created_at: DateTime<Utc>,
}

impl PieceRow {
    fn into_piece(self) -> MsgPiece {
        MsgPiece {
            id: self.id,
            etablissement_id: self.etablissement_id,
            fil_id: self.fil_id,
            message_id: self.message_id,
            nom: self.nom,
            mime_type: self.mime_type,
            taille_octets: self.taille_octets,
            contenu: self.contenu,
            created_by_user_id: self.created_by_user_id,
            created_at: self.created_at,
        }
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] msg sqlx error: {err}");
    DomainError::NotFound
}
