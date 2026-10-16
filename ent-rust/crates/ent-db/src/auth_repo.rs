use chrono::{Duration, Utc};
use ent_auth::{
    assert_password_policy, generate_session_token, hash_password, hash_token, normalize_email,
    verify_password, AuthError, Role,
};
use sqlx::PgPool;
use uuid::Uuid;

const SESSION_TTL_DAYS: i64 = 14;

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

    pub async fn login(
        &self,
        email: &str,
        password: &str,
        etablissement_id: Uuid,
    ) -> Result<SessionInfo, AuthError> {
        let email = normalize_email(email)?;
        if etablissement_id.is_nil() {
            return Err(AuthError::NotMember);
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

        let Some((user_id, password_hash, _display)) = row else {
            return Err(AuthError::InvalidCredentials);
        };

        if !verify_password(password, &password_hash)? {
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
            return Err(AuthError::NotMember);
        };

        let (raw_token, token_hash) = generate_session_token();
        let session_id = Uuid::new_v4();
        let expires_at = Utc::now() + Duration::days(SESSION_TTL_DAYS);

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
            email,
            role,
            token: raw_token,
            expires_at,
        })
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
