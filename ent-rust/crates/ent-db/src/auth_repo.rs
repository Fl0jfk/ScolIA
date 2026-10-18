use chrono::{DateTime, Duration, Utc};
use ent_auth::{
    assert_password_policy, generate_session_token, hash_password, hash_token, normalize_email,
    verify_password, AuthError, Role,
};
use sqlx::PgPool;
use uuid::Uuid;

/// Durée de vie d'une session (jours). Configurable par déploiement.
fn session_ttl_days() -> i64 {
    std::env::var("ENT_SESSION_TTL_DAYS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|d| *d > 0)
        .unwrap_or(14)
}

/// Nombre d'échecs de connexion avant verrouillage temporaire.
fn login_max_failed() -> i32 {
    std::env::var("ENT_LOGIN_MAX_FAILED")
        .ok()
        .and_then(|v| v.trim().parse::<i32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(5)
}

/// Seuil d'échecs par **adresse IP** (défense en profondeur : freine le
/// bourrage d'identifiants réparti sur de nombreux comptes). Volontairement
/// plus haut que le seuil par compte : une école derrière un NAT partage une IP.
fn login_max_failed_ip() -> i32 {
    std::env::var("ENT_LOGIN_MAX_FAILED_IP")
        .ok()
        .and_then(|v| v.trim().parse::<i32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(50)
}

/// Fenêtre glissante de comptage des échecs (minutes).
fn login_window_minutes() -> i64 {
    std::env::var("ENT_LOGIN_WINDOW_MINUTES")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(15)
}

/// Durée du verrouillage après trop d'échecs (minutes).
fn login_lock_minutes() -> i64 {
    std::env::var("ENT_LOGIN_LOCK_MINUTES")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(15)
}

/// Hash factice (généré une fois) : on vérifie un mot de passe même quand
/// l'e-mail est inconnu, pour lisser le temps de réponse (anti-énumération).
static DUMMY_PASSWORD_HASH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    hash_password("mot-de-passe-factice-anti-enumeration")
        .unwrap_or_else(|_| "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0$0000000000000000000000000000000000000000000".into())
});

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
}

/// Membre staff d’un établissement (annuaire / pickers RH·EDT).
#[derive(Debug, Clone)]
pub struct StaffMember {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub telephone: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub etablissement_id: Uuid,
    pub email: String,
    pub role: String,
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

/// Identité vérifiée après contrôle du mot de passe et de l'appartenance à
/// l'établissement, **avant** toute émission de session. Permet d'insérer une
/// étape de 2ᵉ facteur (passkey) sans dupliquer la logique de garde.
#[derive(Debug, Clone)]
pub struct LoginIdentity {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub role: String,
}

pub struct PgAuthRepo {
    pool: PgPool,
}

impl PgAuthRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn register(
        &self,
        email: &str,
        password: &str,
        display_name: Option<&str>,
        etablissement_id: Uuid,
        role: &str,
    ) -> Result<AuthUser, AuthError> {
        let email = normalize_email(email)?;
        assert_password_policy(password)?;
        if etablissement_id.is_nil() {
            return Err(AuthError::NotMember);
        }
        let role = Role::parse(role).ok_or(AuthError::NotMember)?.as_str();

        let password_hash = hash_password(password)?;
        let user_id = Uuid::new_v4();

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;

        let insert = sqlx::query(
            r#"
            INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, now(), now())
            "#,
        )
        .bind(user_id)
        .bind(&email)
        .bind(&password_hash)
        .bind(display_name)
        .execute(&mut *tx)
        .await;

        if let Err(e) = insert {
            if let sqlx::Error::Database(db) = &e {
                if db.constraint() == Some("ent_user_email_unique") {
                    return Err(AuthError::EmailTaken);
                }
            }
            return Err(AuthError::InvalidCredentials);
        }

        sqlx::query(
            r#"
            INSERT INTO ent_membership (user_id, etablissement_id, role, created_at)
            VALUES ($1, $2, $3, now())
            "#,
        )
        .bind(user_id)
        .bind(etablissement_id)
        .bind(role)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

        tx.commit()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;

        Ok(AuthUser {
            id: user_id,
            email,
            display_name: display_name.map(|s| s.to_string()),
        })
    }

    /// Contrôle mot de passe + appartenance, **sans** créer de session.
    /// C'est le point d'entrée pour un parcours à 2ᵉ facteur (passkey).
    pub async fn verify_credentials(
        &self,
        email: &str,
        password: &str,
        etablissement_id: Uuid,
        client_ip: Option<&str>,
    ) -> Result<LoginIdentity, AuthError> {
        // Clé de garde dérivée de l'IP (préfixe `ip:` pour ne pas confondre avec
        // un e-mail dans la même table).
        let ip_key = client_ip.map(|ip| format!("ip:{ip}"));
        let email = normalize_email(email)?;
        if etablissement_id.is_nil() {
            return Err(AuthError::NotMember);
        }

        // Verrouillage temporaire après trop d'échecs (compte, puis IP).
        if self.login_lock_until(&email).await?.is_some() {
            return Err(AuthError::TooManyAttempts);
        }
        if let Some(k) = &ip_key {
            if self.login_lock_until(k).await?.is_some() {
                return Err(AuthError::TooManyAttempts);
            }
        }

        let row: Option<(Uuid, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT id, password_hash, display_name FROM ent_user WHERE email = $1
            "#,
        )
        .bind(&email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

        let Some((user_id, password_hash, display_name)) = row else {
            // E-mail inconnu : on hache quand même un mot de passe factice pour
            // éviter de révéler l'existence du compte par le temps de réponse.
            let _ = verify_password(password, DUMMY_PASSWORD_HASH.as_str());
            self.record_login_failure_all(&email, ip_key.as_deref())
                .await?;
            return Err(AuthError::InvalidCredentials);
        };

        if !verify_password(password, &password_hash)? {
            self.record_login_failure_all(&email, ip_key.as_deref())
                .await?;
            return Err(AuthError::InvalidCredentials);
        }

        let membership: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT role FROM ent_membership
            WHERE user_id = $1 AND etablissement_id = $2
            "#,
        )
        .bind(user_id)
        .bind(etablissement_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

        let Some((role,)) = membership else {
            self.record_login_failure_all(&email, ip_key.as_deref())
                .await?;
            return Err(AuthError::NotMember);
        };

        // Connexion réussie : le compteur du compte repart à zéro. Le compteur
        // par IP n'est **pas** remis à zéro (un succès isolé ne doit pas rendre
        // son budget à un attaquant qui partage l'IP) ; il expire avec la fenêtre.
        self.clear_login_failures(&email).await?;

        Ok(LoginIdentity {
            user_id,
            email,
            display_name,
            role,
        })
    }

    /// Émet une session pour une identité **déjà vérifiée** (mot de passe ou
    /// passkey). Centralise la création du token et sa persistance.
    pub async fn issue_session(
        &self,
        user_id: Uuid,
        etablissement_id: Uuid,
        email: &str,
        role: &str,
    ) -> Result<SessionInfo, AuthError> {
        let (raw_token, token_hash) = generate_session_token();
        let session_id = Uuid::new_v4();
        let expires_at = Utc::now() + Duration::days(session_ttl_days());

        sqlx::query(
            r#"
            INSERT INTO ent_session (
                id, user_id, etablissement_id, token_hash, expires_at, created_at
            ) VALUES ($1,$2,$3,$4,$5, now())
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .bind(etablissement_id)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

        Ok(SessionInfo {
            session_id,
            user_id,
            etablissement_id,
            email: email.to_string(),
            role: role.to_string(),
            token: raw_token,
            expires_at,
        })
    }

    /// Verrou encore actif pour cet e-mail (garde anti-force brute).
    async fn login_lock_until(&self, email: &str) -> Result<Option<DateTime<Utc>>, AuthError> {
        let row: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
            r#"
            SELECT locked_until FROM ent_login_guard
            WHERE email = $1 AND locked_until IS NOT NULL AND locked_until > now()
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.and_then(|(locked,)| locked))
    }

    /// Enregistre un échec sur le compte **et** sur l'IP (si connue).
    async fn record_login_failure_all(
        &self,
        email: &str,
        ip_key: Option<&str>,
    ) -> Result<(), AuthError> {
        self.record_login_failure(email, login_max_failed()).await?;
        if let Some(k) = ip_key {
            self.record_login_failure(k, login_max_failed_ip()).await?;
        }
        Ok(())
    }

    /// Enregistre un échec et verrouille temporairement au-delà du seuil.
    /// Le compteur vit en base : la politique est partagée entre instances.
    /// `max_failed` est paramétré (seuil compte vs seuil IP).
    async fn record_login_failure(&self, key: &str, max_failed: i32) -> Result<(), AuthError> {
        let now = Utc::now();
        let window_floor = now - Duration::minutes(login_window_minutes());
        let lock_until = now + Duration::minutes(login_lock_minutes());

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;

        let row: Option<(i32, DateTime<Utc>, Option<DateTime<Utc>>)> = sqlx::query_as(
            r#"
            SELECT failed_count, window_start, locked_until
            FROM ent_login_guard WHERE email = $1
            FOR UPDATE
            "#,
        )
        .bind(key)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

        // Nouvelle entrée : la fenêtre **commence maintenant**. Utiliser le
        // plancher (`now - fenêtre`) ferait croire à chaque appel que la fenêtre
        // est expirée → compteur bloqué à 1, verrou jamais atteint (bug corrigé).
        let (failed, window_start, existing_lock) = row.unwrap_or((0, now, None));

        // Nouvelle fenêtre si la précédente est expirée, ou si le verrou l'est.
        let lock_expired = existing_lock.map(|l| l <= now).unwrap_or(true);
        let reset = window_start < window_floor || (existing_lock.is_some() && lock_expired);
        let failed_new = if reset { 1 } else { failed + 1 };
        let window_start_new = if reset { now } else { window_start };
        let locked_until_new = if failed_new >= max_failed {
            Some(lock_until)
        } else {
            existing_lock.filter(|l| *l > now)
        };

        sqlx::query(
            r#"
            INSERT INTO ent_login_guard (
                email, failed_count, window_start, locked_until, last_attempt_at
            ) VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (email) DO UPDATE SET
                failed_count = EXCLUDED.failed_count,
                window_start = EXCLUDED.window_start,
                locked_until = EXCLUDED.locked_until,
                last_attempt_at = EXCLUDED.last_attempt_at
            "#,
        )
        .bind(key)
        .bind(failed_new)
        .bind(window_start_new)
        .bind(locked_until_new)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

        tx.commit()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;

        // Ménage opportuniste : au-delà d'un jour, un compteur n'a plus d'usage.
        let _ = sqlx::query(
            "DELETE FROM ent_login_guard WHERE last_attempt_at < now() - interval '1 day'",
        )
        .execute(&self.pool)
        .await;

        Ok(())
    }

    async fn clear_login_failures(&self, email: &str) -> Result<(), AuthError> {
        sqlx::query("DELETE FROM ent_login_guard WHERE email = $1")
            .bind(email)
            .execute(&self.pool)
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(())
    }

    /// Rôle d'un utilisateur dans un établissement (`None` s'il n'est pas membre).
    pub async fn role_in(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<String>, AuthError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM ent_membership WHERE user_id = $1 AND etablissement_id = $2",
        )
        .bind(user_id)
        .bind(etablissement_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.map(|(r,)| r))
    }

    /// Écrit directement un nouveau mot de passe (réinitialisation par un tiers).
    pub async fn set_password(&self, user_id: Uuid, new_password: &str) -> Result<(), AuthError> {
        assert_password_policy(new_password)?;
        let hash = hash_password(new_password)?;
        let res =
            sqlx::query("UPDATE ent_user SET password_hash = $2, updated_at = now() WHERE id = $1")
                .bind(user_id)
                .bind(&hash)
                .execute(&self.pool)
                .await
                .map_err(|_| AuthError::InvalidCredentials)?;
        if res.rows_affected() == 0 {
            return Err(AuthError::InvalidCredentials);
        }
        Ok(())
    }

    /// E-mail d'un utilisateur (audit, libellés).
    pub async fn resolve_user_email(&self, user_id: Uuid) -> Result<String, AuthError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT email FROM ent_user WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| AuthError::InvalidCredentials)?;
        row.map(|(email,)| email).ok_or(AuthError::InvalidCredentials)
    }

    /// E-mail + nom affiché d'un utilisateur (enrôlement passkey : le nom
    /// affiché apparaît dans le sélecteur natif de l'appareil).
    pub async fn user_profile(
        &self,
        user_id: Uuid,
    ) -> Result<Option<(String, Option<String>)>, AuthError> {
        let row: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT email, display_name FROM ent_user WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row)
    }

    /// Purge des sessions expirées (maintenance, au démarrage).
    pub async fn purge_expired_sessions(&self) -> Result<u64, AuthError> {
        let res = sqlx::query("DELETE FROM ent_session WHERE expires_at < now()")
            .execute(&self.pool)
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(res.rows_affected())
    }

    pub async fn resolve_bearer(&self, authorization: &str) -> Result<SessionInfo, AuthError> {
        let raw = authorization
            .strip_prefix("Bearer ")
            .or_else(|| authorization.strip_prefix("bearer "))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or(AuthError::InvalidSession)?;

        let token_hash = hash_token(raw);
        let now = Utc::now();

        let row: Option<(Uuid, Uuid, Uuid, String, String, chrono::DateTime<Utc>)> =
            sqlx::query_as(
                r#"
                SELECT s.id, s.user_id, s.etablissement_id, u.email, m.role, s.expires_at
                FROM ent_session s
                JOIN ent_user u ON u.id = s.user_id
                JOIN ent_membership m
                  ON m.user_id = s.user_id AND m.etablissement_id = s.etablissement_id
                WHERE s.token_hash = $1
                  AND s.revoked_at IS NULL
                  AND s.expires_at > $2
                "#,
            )
            .bind(&token_hash)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| AuthError::InvalidSession)?;

        let Some((session_id, user_id, etablissement_id, email, role, expires_at)) = row else {
            return Err(AuthError::InvalidSession);
        };

        Ok(SessionInfo {
            session_id,
            user_id,
            etablissement_id,
            email,
            role,
            token: String::new(),
            expires_at,
        })
    }

    /// Premier membre staff dont le rôle est dans la liste (ordre = priorité).
    pub async fn first_member_with_roles(
        &self,
        etablissement_id: Uuid,
        roles: &[&str],
    ) -> Result<Option<Uuid>, AuthError> {
        if roles.is_empty() {
            return Ok(None);
        }
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT user_id FROM ent_membership
            WHERE etablissement_id = $1 AND role = ANY($2)
            ORDER BY array_position($2::text[], role), created_at
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(roles)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.map(|(id,)| id))
    }

    /// Membres staff dont le rôle est dans la liste (sans doublon user_id).
    pub async fn list_user_ids_with_roles(
        &self,
        etablissement_id: Uuid,
        roles: &[&str],
    ) -> Result<Vec<Uuid>, AuthError> {
        if roles.is_empty() {
            return Ok(vec![]);
        }
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT user_id FROM ent_membership
            WHERE etablissement_id = $1 AND role = ANY($2)
            "#,
        )
        .bind(etablissement_id)
        .bind(roles)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn is_member(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, AuthError> {
        let row: Option<(i32,)> = sqlx::query_as(
            r#"
            SELECT 1
            FROM ent_membership
            WHERE etablissement_id = $1 AND user_id = $2
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.is_some())
    }

    /// Lookup membre établissement par e-mail (import Pronote / annuaire).
    pub async fn find_member_id_by_email(
        &self,
        etablissement_id: Uuid,
        email: &str,
    ) -> Result<Option<Uuid>, AuthError> {
        let email = normalize_email(email)?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT u.id
            FROM ent_user u
            INNER JOIN ent_membership m
              ON m.user_id = u.id AND m.etablissement_id = $1
            WHERE u.email = $2
            LIMIT 1
            "#,
        )
        .bind(etablissement_id)
        .bind(&email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn set_telephone(
        &self,
        user_id: Uuid,
        telephone: &str,
    ) -> Result<(), AuthError> {
        let tel = telephone.trim();
        let tel = if tel.is_empty() { None } else { Some(tel) };
        sqlx::query(
            r#"
            UPDATE ent_user SET telephone = $2, updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(tel)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(())
    }

    pub async fn set_email(&self, user_id: Uuid, email: &str) -> Result<(), AuthError> {
        let email = normalize_email(email)?;
        let updated = sqlx::query(
            r#"
            UPDATE ent_user SET email = $2, updated_at = now()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(&email)
        .execute(&self.pool)
        .await;
        match updated {
            Ok(r) if r.rows_affected() > 0 => Ok(()),
            Ok(_) => Err(AuthError::InvalidCredentials),
            Err(e) => {
                if let sqlx::Error::Database(db) = &e {
                    if db.constraint() == Some("ent_user_email_unique") {
                        return Err(AuthError::EmailTaken);
                    }
                }
                Err(AuthError::InvalidCredentials)
            }
        }
    }

    pub async fn revoke_bearer(&self, authorization: &str) -> Result<(), AuthError> {
        let raw = authorization
            .strip_prefix("Bearer ")
            .or_else(|| authorization.strip_prefix("bearer "))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or(AuthError::InvalidSession)?;
        let token_hash = hash_token(raw);
        sqlx::query(
            r#"
            UPDATE ent_session SET revoked_at = now()
            WHERE token_hash = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(&token_hash)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidSession)?;
        Ok(())
    }

    /// Change le mot de passe d'un utilisateur après vérification de l'actuel.
    /// Révoque ensuite toutes les sessions sauf celle indiquée (si fournie).
    pub async fn change_password(
        &self,
        user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), AuthError> {
        assert_password_policy(new_password)?;

        let row: Option<(String,)> =
            sqlx::query_as("SELECT password_hash FROM ent_user WHERE id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| AuthError::InvalidCredentials)?;

        let Some((password_hash,)) = row else {
            return Err(AuthError::InvalidCredentials);
        };

        if !verify_password(current_password, &password_hash)? {
            return Err(AuthError::InvalidCredentials);
        }

        let new_hash = hash_password(new_password)?;
        sqlx::query("UPDATE ent_user SET password_hash = $2, updated_at = now() WHERE id = $1")
            .bind(user_id)
            .bind(&new_hash)
            .execute(&self.pool)
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;

        Ok(())
    }

    /// Révoque toutes les sessions actives d'un utilisateur sauf une (optionnelle).
    /// Renvoie le nombre de sessions révoquées.
    pub async fn revoke_other_sessions_for_user(
        &self,
        user_id: Uuid,
        keep_session_id: Option<Uuid>,
    ) -> Result<u64, AuthError> {
        let res = sqlx::query(
            r#"
            UPDATE ent_session SET revoked_at = now()
            WHERE user_id = $1 AND revoked_at IS NULL
              AND ($2::uuid IS NULL OR id <> $2)
            "#,
        )
        .bind(user_id)
        .bind(keep_session_id)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidSession)?;
        Ok(res.rows_affected())
    }

    /// Révoque toutes les sessions actives d'un utilisateur (sortie / radiation élève).
    /// Renvoie le nombre de sessions révoquées.
    pub async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<u64, AuthError> {
        let res = sqlx::query(
            r#"
            UPDATE ent_session SET revoked_at = now()
            WHERE user_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidSession)?;
        Ok(res.rows_affected())
    }

    // ---- Réinitialisation de mot de passe (jeton à usage unique) ----

    /// Retrouve l'utilisateur par e-mail normalisé (insensible à la casse).
    pub async fn find_user_id_by_email(&self, email: &str) -> Result<Option<Uuid>, AuthError> {
        let email = normalize_email(email)?;
        let row: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_user WHERE email = $1")
                .bind(&email)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.map(|(id,)| id))
    }

    /// Crée un jeton de réinitialisation. Invalide d'abord les jetons non utilisés
    /// du même utilisateur (un seul lien actif).
    pub async fn create_password_reset(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), AuthError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        sqlx::query(
            "UPDATE ent_password_reset SET used_at = now()
             WHERE user_id = $1 AND used_at IS NULL",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        sqlx::query(
            "INSERT INTO ent_password_reset (id, user_id, token_hash, expires_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        tx.commit()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(())
    }

    /// Consomme un jeton (usage unique, non expiré) et applique le nouveau mot de
    /// passe, en révoquant **toutes** les sessions de l'utilisateur. Atomique :
    /// un jeton rejoué ne peut pas passer deux fois.
    pub async fn consume_password_reset(
        &self,
        token_hash: &str,
        new_password: &str,
    ) -> Result<Uuid, AuthError> {
        assert_password_policy(new_password)?;
        let new_hash = hash_password(new_password)?;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_password_reset SET used_at = now()
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
             RETURNING user_id",
        )
        .bind(token_hash)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        let Some((user_id,)) = row else {
            return Err(AuthError::ResetTokenInvalide);
        };
        sqlx::query("UPDATE ent_user SET password_hash = $2, updated_at = now() WHERE id = $1")
            .bind(user_id)
            .bind(&new_hash)
            .execute(&mut *tx)
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        sqlx::query(
            "UPDATE ent_session SET revoked_at = now()
             WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        tx.commit()
            .await
            .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(user_id)
    }

    /// Purge les jetons de réinitialisation expirés ou consommés (> 1 jour).
    pub async fn prune_password_resets(&self) -> Result<u64, AuthError> {
        let res = sqlx::query(
            "DELETE FROM ent_password_reset
             WHERE (used_at IS NOT NULL OR expires_at < now())
               AND created_at < now() - interval '1 day'",
        )
        .execute(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(res.rows_affected())
    }

    /// Établissements + rôles d'un utilisateur (audit d'une réinit. par jeton).
    pub async fn list_memberships_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(Uuid, String)>, AuthError> {
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT etablissement_id, role FROM ent_membership WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(rows)
    }

    /// Résout des libellés d’affichage pour une liste d’utilisateurs (display_name ou e-mail).
    pub async fn user_labels(
        &self,
        user_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, String>, AuthError> {
        if user_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let rows: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT id, email, display_name
            FROM ent_user
            WHERE id = ANY($1)
            "#,
        )
        .bind(user_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(rows
            .into_iter()
            .map(|(id, email, display_name)| {
                let label = display_name
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or(email);
                (id, label)
            })
            .collect())
    }

    pub async fn get_staff_member(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<StaffMember>, AuthError> {
        let row: Option<(Uuid, String, Option<String>, Option<String>, String)> = sqlx::query_as(
            r#"
            SELECT u.id, u.email, u.display_name, u.telephone, m.role
            FROM ent_user u
            INNER JOIN ent_membership m
              ON m.user_id = u.id AND m.etablissement_id = $1
            WHERE u.id = $2
              AND m.role NOT IN ('parent', 'eleve')
            "#,
        )
        .bind(etablissement_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(row.map(|(user_id, email, display_name, telephone, role)| StaffMember {
            user_id,
            email,
            display_name,
            telephone,
            role,
        }))
    }

    /// Annuaire staff (hors parent / élève) — recherche e-mail, nom affiché ou rôle.
    pub async fn search_staff(
        &self,
        etablissement_id: Uuid,
        q: &str,
        limit: i64,
    ) -> Result<Vec<StaffMember>, AuthError> {
        let needle = q.trim();
        let pattern = if needle.is_empty() {
            "%".to_string()
        } else {
            format!("%{}%", needle)
        };
        let limit = limit.clamp(1, 100);
        let rows: Vec<(Uuid, String, Option<String>, Option<String>, String)> = sqlx::query_as(
            r#"
            SELECT u.id, u.email, u.display_name, u.telephone, m.role
            FROM ent_user u
            INNER JOIN ent_membership m
              ON m.user_id = u.id AND m.etablissement_id = $1
            WHERE m.role NOT IN ('parent', 'eleve')
              AND (
                u.email ILIKE $2
                OR COALESCE(u.display_name, '') ILIKE $2
                OR m.role ILIKE $2
              )
            ORDER BY m.role, COALESCE(u.display_name, u.email)
            LIMIT $3
            "#,
        )
        .bind(etablissement_id)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;
        Ok(rows
            .into_iter()
            .map(|(user_id, email, display_name, telephone, role)| StaffMember {
                user_id,
                email,
                display_name,
                telephone,
                role,
            })
            .collect())
    }
}
