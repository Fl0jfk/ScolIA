use domain_eleve::DomainError;
use ent_push::{PushPlatform, PushSendStatus};
use sqlx::PgPool;
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

#[derive(Clone)]
pub struct PushDevice {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    pub platform: String,
    pub token: String,
    pub app_id: Option<String>,
}

#[derive(Clone)]
pub struct RegisterPushDevice {
    pub etablissement_id: Uuid,
    pub user_id: Uuid,
    /// Plateforme déclarée (validée par le dépôt, APRÈS le contrôle de session : F-3).
    pub platform: String,
    pub token: String,
    pub app_id: Option<String>,
    /// Session qui inscrit le jeton (DS1). `None` seulement par la variante explicite du labo et des
    /// tests (inscription sans session : réinscription de moins de 30 jours exigée pour l'envoi).
    pub session_id: Option<Uuid>,
}

pub struct InsertPushOutbox {
    pub etablissement_id: Uuid,
    pub eleve_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub platform: Option<String>,
    /// Appareil visé (référence) ; l'empreinte du jeton en est déduite. Jamais le jeton (règle 3).
    pub device_id: Option<Uuid>,
    pub status: PushSendStatus,
    pub error: Option<String>,
    pub source_aggregate: Option<String>,
    pub source_id: Option<Uuid>,
    pub message_id: Option<String>,
}

pub struct PgPushRepo {
    pool: PgPool,
}

impl PgPushRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Inscrit le jeton de notification d'un appareil, lié à la session qui l'inscrit (DS1).
    /// Règle 2 : un jeton physique n'a qu'un titulaire. L'inscription d'un jeton actif pour une
    /// **autre personne** (tous établissements) révoque l'ancienne (`reaffecte`, avec trace) et
    /// renvoie les anciens titulaires pour l'avis de sécurité. La même personne avec une nouvelle
    /// session met à jour `session_id` sans révocation (M3). Refus (`InvalidSession`) si la session
    /// n'est plus vivante au moment de l'inscription (M1 : verrou partagé contre la course à la
    /// révocation).
    pub async fn upsert_device(
        &self,
        cmd: RegisterPushDevice,
    ) -> Result<(PushDevice, Vec<Uuid>), PushRepoError> {
        let mut txw = TenantTx::begin(&self.pool, cmd.etablissement_id).await.map_err(map_sqlx)?;
        if let Some(sid) = cmd.session_id {
            // Règle 1 : la même définition que l'envoi (`ent_session_vivante`, = `resolve_session`).
            let vivante: Option<(i32,)> = sqlx::query_as(
                r#"SELECT 1 FROM ent_session
                    WHERE id = $1 AND user_id = $2 AND etablissement_id = $3
                      AND ent_session_vivante(id)
                    FOR SHARE"#,
            )
            .bind(sid)
            .bind(cmd.user_id)
            .bind(cmd.etablissement_id)
            .fetch_optional(&mut *txw)
            .await
            .map_err(map_sqlx)?;
            if vivante.is_none() {
                return Err(PushRepoError::SessionMorte);
            }
        }
        // F-3 : le corps n'est validé qu'APRÈS le contrôle de session (une session morte : 401 d'abord).
        let token = cmd.token.trim().to_string();
        if token.is_empty() || token.len() > 4096 {
            return Err(PushRepoError::Domain(DomainError::EmptyAttributValeur));
        }
        let platform = PushPlatform::parse(&cmd.platform).ok_or(PushRepoError::PlateformeInconnue)?;
        let platform = platform.as_str();
        let empreinte = empreinte_jeton(&token);
        let anciens: Vec<(Uuid,)> = sqlx::query_as("SELECT ent_push_reaffecter($1, $2, $3)")
            .bind(&empreinte)
            .bind(cmd.user_id)
            .bind(cmd.etablissement_id)
            .fetch_all(&mut *txw)
            .await
            .map_err(map_sqlx)?;
        // F5 : l'insertion concurrente du même jeton ne donne jamais d'erreur d'unicité.
        let row = sqlx::query_as::<_, DeviceRow>(
            r#"INSERT INTO ent_push_device (
                   id, etablissement_id, user_id, platform, token, app_id,
                   session_id, token_empreinte, last_seen_at, created_at, revoked_at
               ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now(), now(), NULL)
               ON CONFLICT (etablissement_id, token_empreinte) WHERE revoked_at IS NULL
               DO UPDATE SET platform = EXCLUDED.platform, app_id = EXCLUDED.app_id,
                             session_id = EXCLUDED.session_id, last_seen_at = now()
                  WHERE ent_push_device.user_id = EXCLUDED.user_id
               RETURNING id, etablissement_id, user_id, platform, token, app_id"#,
        )
        .bind(Uuid::new_v4())
        .bind(cmd.etablissement_id)
        .bind(cmd.user_id)
        .bind(platform)
        .bind(&token)
        .bind(&cmd.app_id)
        .bind(cmd.session_id)
        .bind(&empreinte)
        .fetch_optional(&mut *txw)
        .await
        .map_err(map_sqlx)?;
        // Aucune ligne : une autre personne détient une inscription active de ce jeton dans cet
        // établissement ET ne l'a pas passée par `ent_push_reaffecter` (écriture directe concurrente :
        // toute inscription de l'API prend le même verrou consultatif, donc ne peut pas s'intercaler ;
        // la ligne rivale ne peut être qu'une écriture hors API non encore validée). On refuse plutôt
        // que de réaffecter en silence : conflit (409), jamais une erreur serveur (F-3).
        let Some(row) = row else {
            return Err(PushRepoError::Conflit);
        };
        txw.commit().await.map_err(map_sqlx)?;
        let mut anciens: Vec<Uuid> = anciens.into_iter().map(|(u,)| u).collect();
        anciens.sort();
        anciens.dedup();
        Ok((row.into_device(), anciens))
    }

    /// Relit le jeton d'un appareil **au moment de l'envoi** : `None` si l'inscription n'est plus
    /// active (révoquée, session morte) entre la sélection et l'envoi : rien n'est alors envoyé.
    pub async fn relire_jeton(
        &self,
        etablissement_id: Uuid,
        device_id: Uuid,
    ) -> Result<Option<String>, DomainError> {
        let mut tx = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let r: Option<(String,)> = sqlx::query_as(
            r#"SELECT token FROM ent_push_device
                WHERE id = $1 AND etablissement_id = $2
                  AND ent_push_actif(revoked_at, session_id, last_seen_at, user_id, etablissement_id)"#,
        )
        .bind(device_id)
        .bind(etablissement_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        Ok(r.map(|(t,)| t))
    }

    /// DS4 : désinscription par la personne propriétaire. `false` si rien à révoquer.
    pub async fn desinscrire_device(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<bool, DomainError> {
        let mut txw = TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?;
        let n = sqlx::query(
            r#"UPDATE ent_push_device SET revoked_at = now(), revoque_motif = 'desinscription'
                WHERE id = $1 AND etablissement_id = $2 AND user_id = $3 AND revoked_at IS NULL"#,
        )
        .bind(id)
        .bind(etablissement_id)
        .bind(user_id)
        .execute(&mut *txw)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        txw.commit().await.map_err(map_sqlx)?;
        Ok(n > 0)
    }

    pub async fn list_active_for_users(
        &self,
        etablissement_id: Uuid,
        user_ids: &[Uuid],
    ) -> Result<Vec<PushDevice>, DomainError> {
        if user_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows = sqlx::query_as::<_, DeviceRow>(
            r#"
            SELECT id, etablissement_id, user_id, platform, token, app_id
            FROM ent_push_device
            WHERE etablissement_id = $1
              AND user_id = ANY($2)
              AND ent_push_actif(revoked_at, session_id, last_seen_at, user_id, etablissement_id)
            "#,
        )
        .bind(etablissement_id)
        .bind(user_ids)
        .fetch_all(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(DeviceRow::into_device).collect())
    }

    pub async fn insert_outbox(
        &self,
        row: InsertPushOutbox,
    ) -> Result<Uuid, DomainError> {
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(&self.pool, row.etablissement_id).await.map_err(map_sqlx)?;
        sqlx::query(
            r#"
            INSERT INTO ent_push_outbox (
                id, etablissement_id, eleve_id, user_id, kind, title, body,
                platform, device_id, token_empreinte, status, error, source_aggregate, source_id,
                message_id, created_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,
                (SELECT token_empreinte FROM ent_push_device WHERE id = $9 AND etablissement_id = $2),
                $10,$11,$12,$13,$14, now())
            "#,
        )
        .bind(id)
        .bind(row.etablissement_id)
        .bind(row.eleve_id)
        .bind(row.user_id)
        .bind(&row.kind)
        .bind(&row.title)
        .bind(&row.body)
        .bind(&row.platform)
        .bind(row.device_id)
        .bind(status_str(row.status))
        .bind(&row.error)
        .bind(&row.source_aggregate)
        .bind(row.source_id)
        .bind(&row.message_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
        tx.commit().await.map_err(map_sqlx)?;
        Ok(id)
    }

    pub async fn count_by_kind(
        &self,
        etablissement_id: Uuid,
        kind: &str,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_push_outbox
            WHERE etablissement_id = $1 AND kind = $2
            "#,
        )
        .bind(etablissement_id)
        .bind(kind)
        .fetch_one(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.0)
    }

    pub async fn count_by_source_kind(
        &self,
        etablissement_id: Uuid,
        source_id: Uuid,
        kind: &str,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)::bigint FROM ent_push_outbox
            WHERE etablissement_id = $1 AND source_id = $2 AND kind = $3
            "#,
        )
        .bind(etablissement_id)
        .bind(source_id)
        .bind(kind)
        .fetch_one(&mut *TenantTx::begin(&self.pool, etablissement_id).await.map_err(map_sqlx)?)
        .await
        .map_err(map_sqlx)?;
        Ok(row.0)
    }

}

/// Échec d'inscription : session morte (401) ou erreur de domaine.
#[derive(Debug)]
pub enum PushRepoError {
    SessionMorte,
    /// Plateforme non reconnue (403, comme avant).
    PlateformeInconnue,
    /// Une autre personne détient déjà ce jeton (409) : voir `upsert_device`.
    Conflit,
    Domain(DomainError),
}

impl From<DomainError> for PushRepoError {
    fn from(e: DomainError) -> Self {
        Self::Domain(e)
    }
}

/// Empreinte SHA-256 (hexadécimal) du jeton : on le retrouve sans le relire (DS3).
fn empreinte_jeton(token: &str) -> String {
    ent_auth::hash_token(token)
}

fn status_str(s: PushSendStatus) -> &'static str {
    match s {
        PushSendStatus::DryRun => "dry_run",
        PushSendStatus::Sent => "sent",
        PushSendStatus::SkippedNoConfig => "skipped_no_config",
        PushSendStatus::Failed => "failed",
    }
}

fn map_sqlx(err: sqlx::Error) -> DomainError {
    crate::pg_err::map_sqlx("push_repo", err)
}

#[derive(sqlx::FromRow)]
struct DeviceRow {
    id: Uuid,
    etablissement_id: Uuid,
    user_id: Uuid,
    platform: String,
    token: String,
    app_id: Option<String>,
}

impl DeviceRow {
    fn into_device(self) -> PushDevice {
        PushDevice {
            id: self.id,
            etablissement_id: self.etablissement_id,
            user_id: self.user_id,
            platform: self.platform,
            token: self.token,
            app_id: self.app_id,
        }
    }
}

impl std::fmt::Debug for PushDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PushDevice")
            .field("id", &self.id)
            .field("etablissement_id", &self.etablissement_id)
            .field("user_id", &self.user_id)
            .field("platform", &self.platform)
            .field("app_id", &self.app_id)
            .field("token", &"[masqué]")
            .finish()
    }
}
