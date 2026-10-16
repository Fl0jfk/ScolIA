use chrono::{DateTime, Utc};
use domain_eleve::{
    DispositifKind, DocConfidentialite, DocTiroir, DocumentGrant, DomainError, EleveDispositif,
    EleveDocument, GrantStatus,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PgDocumentRepo {
    pool: PgPool,
}

impl PgDocumentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_document(&self, doc: &EleveDocument) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_eleve_document (
                id, etablissement_id, eleve_id, tiroir, titre, code, mime_type, storage_key,
                confidentialite, source, created_by_user_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11, now())
            ON CONFLICT (id) DO UPDATE SET
                titre = EXCLUDED.titre,
                code = EXCLUDED.code,
                mime_type = EXCLUDED.mime_type,
                storage_key = EXCLUDED.storage_key,
                confidentialite = EXCLUDED.confidentialite
            WHERE ent_eleve_document.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(doc.id)
        .bind(doc.etablissement_id)
        .bind(doc.eleve_id)
        .bind(doc.tiroir.as_str())
        .bind(&doc.titre)
        .bind(&doc.code)
        .bind(&doc.mime_type)
        .bind(&doc.storage_key)
        .bind(doc.confidentialite.as_str())
        .bind(&doc.source)
        .bind(doc.created_by_user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    /// Titres de documents par id (file grants / UI).
    pub async fn document_labels(
        &self,
        etablissement_id: Uuid,
        document_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, DomainError> {
        if document_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT id, titre, code
            FROM ent_eleve_document
            WHERE etablissement_id = $1 AND id = ANY($2)
            "#,
        )
        .bind(etablissement_id)
        .bind(document_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows
            .into_iter()
            .map(|(id, titre, code)| {
                let label = if titre.trim().is_empty() {
                    code.filter(|c| !c.trim().is_empty())
                        .unwrap_or_else(|| "Sans titre".into())
                } else {
                    titre
                };
                (id, label)
            })
            .collect())
    }

    pub async fn get_document(
        &self,
        etablissement_id: Uuid,
        document_id: Uuid,
    ) -> Result<Option<EleveDocument>, DomainError> {
        let row = sqlx::query_as::<_, DocRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, tiroir, titre, code, mime_type, storage_key,
                   confidentialite, source, created_by_user_id
            FROM ent_eleve_document
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(DocRow::into_doc))
    }

    pub async fn list_documents_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<EleveDocument>, DomainError> {
        let rows = sqlx::query_as::<_, DocRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, tiroir, titre, code, mime_type, storage_key,
                   confidentialite, source, created_by_user_id
            FROM ent_eleve_document
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(DocRow::into_doc).collect())
    }

    pub async fn upsert_grant(&self, g: &DocumentGrant) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_document_grant (
                id, etablissement_id, document_id, grantee_user_id, status, duration_days,
                decided_by_user_id, decided_at, expires_at, note, created_at
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,
                CASE WHEN $5 IN ('granted','refused') THEN now() ELSE NULL END,
                $8,$9, now()
            )
            ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status,
                decided_by_user_id = EXCLUDED.decided_by_user_id,
                decided_at = CASE
                    WHEN EXCLUDED.status IN ('granted','refused')
                         AND ent_document_grant.status IS DISTINCT FROM EXCLUDED.status
                    THEN now()
                    ELSE ent_document_grant.decided_at
                END,
                expires_at = EXCLUDED.expires_at,
                note = EXCLUDED.note
            WHERE ent_document_grant.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(g.id)
        .bind(g.etablissement_id)
        .bind(g.document_id)
        .bind(g.grantee_user_id)
        .bind(g.status.as_str())
        .bind(g.duration_days)
        .bind(g.decided_by_user_id)
        .bind(g.expires_at)
        .bind(&g.note)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn get_grant(
        &self,
        etablissement_id: Uuid,
        grant_id: Uuid,
    ) -> Result<Option<DocumentGrant>, DomainError> {
        let row = sqlx::query_as::<_, GrantRow>(
            r#"
            SELECT id, etablissement_id, document_id, grantee_user_id, status, duration_days,
                   decided_by_user_id, expires_at, note
            FROM ent_document_grant
            WHERE etablissement_id = $1 AND id = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(grant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.and_then(GrantRow::into_grant))
    }

    pub async fn list_grants_actifs_user_doc(
        &self,
        etablissement_id: Uuid,
        document_id: Uuid,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Vec<DocumentGrant>, DomainError> {
        let rows = sqlx::query_as::<_, GrantRow>(
            r#"
            SELECT id, etablissement_id, document_id, grantee_user_id, status, duration_days,
                   decided_by_user_id, expires_at, note
            FROM ent_document_grant
            WHERE etablissement_id = $1
              AND document_id = $2
              AND grantee_user_id = $3
              AND status = 'granted'
              AND expires_at IS NOT NULL
              AND expires_at > $4
            "#,
        )
        .bind(etablissement_id)
        .bind(document_id)
        .bind(user_id)
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(GrantRow::into_grant).collect())
    }

    pub async fn latest_grant_status_for_user_doc(
        &self,
        etablissement_id: Uuid,
        document_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<String>, DomainError> {
        let row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT status
            FROM ent_document_grant
            WHERE etablissement_id = $1
              AND document_id = $2
              AND grantee_user_id = $3
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(document_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(|(s,)| s))
    }

    pub async fn list_grants_pending(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<DocumentGrant>, DomainError> {
        let rows = sqlx::query_as::<_, GrantRow>(
            r#"
            SELECT id, etablissement_id, document_id, grantee_user_id, status, duration_days,
                   decided_by_user_id, expires_at, note
            FROM ent_document_grant
            WHERE etablissement_id = $1 AND status = 'pending'
            ORDER BY created_at DESC
            "#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(GrantRow::into_grant).collect())
    }

    pub async fn upsert_dispositif(&self, d: &EleveDispositif) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO ent_eleve_dispositif (
                id, etablissement_id, eleve_id, kind, libelle, document_id, date_debut, date_fin, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now())
            ON CONFLICT (id) DO UPDATE SET
                libelle = EXCLUDED.libelle,
                document_id = EXCLUDED.document_id,
                date_fin = EXCLUDED.date_fin
            WHERE ent_eleve_dispositif.etablissement_id = EXCLUDED.etablissement_id
            "#,
        )
        .bind(d.id)
        .bind(d.etablissement_id)
        .bind(d.eleve_id)
        .bind(d.kind.as_str())
        .bind(&d.libelle)
        .bind(d.document_id)
        .bind(d.date_debut)
        .bind(d.date_fin)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    pub async fn list_dispositifs(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<EleveDispositif>, DomainError> {
        let rows = sqlx::query_as::<_, DispRow>(
            r#"
            SELECT id, etablissement_id, eleve_id, kind, libelle, document_id, date_debut, date_fin
            FROM ent_eleve_dispositif
            WHERE etablissement_id = $1 AND eleve_id = $2
            ORDER BY date_debut DESC
            "#,
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().filter_map(DispRow::into_disp).collect())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DocRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    tiroir: String,
    titre: String,
    code: Option<String>,
    mime_type: Option<String>,
    storage_key: Option<String>,
    confidentialite: String,
    source: String,
    created_by_user_id: Option<Uuid>,
}

impl DocRow {
    fn into_doc(self) -> Option<EleveDocument> {
        Some(EleveDocument {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            tiroir: DocTiroir::parse(&self.tiroir)?,
            titre: self.titre,
            code: self.code,
            mime_type: self.mime_type,
            storage_key: self.storage_key,
            confidentialite: DocConfidentialite::parse(&self.confidentialite)?,
            source: self.source,
            created_by_user_id: self.created_by_user_id,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct GrantRow {
    id: Uuid,
    etablissement_id: Uuid,
    document_id: Uuid,
    grantee_user_id: Uuid,
    status: String,
    duration_days: i32,
    decided_by_user_id: Option<Uuid>,
    expires_at: Option<DateTime<Utc>>,
    note: Option<String>,
}

impl GrantRow {
    fn into_grant(self) -> Option<DocumentGrant> {
        Some(DocumentGrant {
            id: self.id,
            etablissement_id: self.etablissement_id,
            document_id: self.document_id,
            grantee_user_id: self.grantee_user_id,
            status: GrantStatus::parse(&self.status)?,
            duration_days: self.duration_days,
            decided_by_user_id: self.decided_by_user_id,
            expires_at: self.expires_at,
            note: self.note,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DispRow {
    id: Uuid,
    etablissement_id: Uuid,
    eleve_id: Uuid,
    kind: String,
    libelle: Option<String>,
    document_id: Option<Uuid>,
    date_debut: chrono::NaiveDate,
    date_fin: Option<chrono::NaiveDate>,
}

impl DispRow {
    fn into_disp(self) -> Option<EleveDispositif> {
        Some(EleveDispositif {
            id: self.id,
            etablissement_id: self.etablissement_id,
            eleve_id: self.eleve_id,
            kind: DispositifKind::parse(&self.kind)?,
            libelle: self.libelle,
            document_id: self.document_id,
            date_debut: self.date_debut,
            date_fin: self.date_fin,
        })
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    eprintln!("[ent-db] document sqlx error: {err}");
    DomainError::NotFound
}
