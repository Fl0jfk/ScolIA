//! Persistance SSO OIDC (`ent_oidc_provider`, `ent_oidc_identity`, `ent_oidc_state`).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Fournisseur OIDC d'un établissement (le secret ne sort jamais vers le
/// portail public — voir [`OidcProviderPublic`]).
#[derive(Debug, Clone)]
pub struct OidcProviderRow {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub slug: String,
    pub libelle: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: String,
    pub redirect_uri: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    pub actif: bool,
}

/// Vue publique (page de connexion) : aucune donnée sensible.
#[derive(Debug, Clone)]
pub struct OidcProviderPublic {
    pub slug: String,
    pub libelle: String,
}

/// État de parcours d'autorisation, consommé au retour de l'IdP.
#[derive(Debug, Clone)]
pub struct OidcStateRow {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub etablissement_id: Uuid,
    pub code_verifier: String,
    pub nonce: String,
    pub redirect_uri: String,
}

/// Paramètres d'enregistrement d'un fournisseur (endpoints déjà résolus par
/// découverte). Regroupés en structure : une longue liste d'arguments se lit
/// mal et invite à inverser `client_id` et `client_secret`.
pub struct OidcProviderUpsert<'a> {
    pub etablissement_id: Uuid,
    pub slug: &'a str,
    pub libelle: &'a str,
    pub issuer: &'a str,
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub scopes: &'a str,
    pub redirect_uri: &'a str,
    pub authorization_endpoint: &'a str,
    pub token_endpoint: &'a str,
    pub jwks_uri: &'a str,
}

/// Paramètres d'ouverture d'un état de parcours (state/PKCE/nonce).
pub struct OidcStateInsert<'a> {
    pub token_hash: &'a str,
    pub provider_id: Uuid,
    pub etablissement_id: Uuid,
    pub code_verifier: &'a str,
    pub nonce: &'a str,
    pub redirect_uri: &'a str,
    pub expires_at: DateTime<Utc>,
}

type ProviderCols = (
    Uuid,
    Uuid,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    bool,
);

fn provider_from_row(r: ProviderCols) -> OidcProviderRow {
    OidcProviderRow {
        id: r.0,
        etablissement_id: r.1,
        slug: r.2,
        libelle: r.3,
        issuer: r.4,
        client_id: r.5,
        client_secret: r.6,
        scopes: r.7,
        redirect_uri: r.8,
        authorization_endpoint: r.9,
        token_endpoint: r.10,
        jwks_uri: r.11,
        actif: r.12,
    }
}

const PROVIDER_SELECT: &str = r#"
    SELECT id, etablissement_id, slug, libelle, issuer, client_id, client_secret,
           scopes, redirect_uri, authorization_endpoint, token_endpoint, jwks_uri, actif
    FROM ent_oidc_provider
"#;

pub struct PgOidcRepo {
    pool: PgPool,
}

impl PgOidcRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Crée ou met à jour un fournisseur (clé : établissement + slug).
    /// Les endpoints résolus par découverte sont fournis par l'appelant.
    pub async fn upsert_provider(
        &self,
        p: OidcProviderUpsert<'_>,
    ) -> Result<Uuid, sqlx::Error> {
        let id: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO ent_oidc_provider (
                id, etablissement_id, slug, libelle, issuer, client_id, client_secret,
                scopes, redirect_uri, authorization_endpoint, token_endpoint, jwks_uri
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            ON CONFLICT (etablissement_id, slug) DO UPDATE SET
                libelle = EXCLUDED.libelle,
                issuer = EXCLUDED.issuer,
                client_id = EXCLUDED.client_id,
                client_secret = EXCLUDED.client_secret,
                scopes = EXCLUDED.scopes,
                redirect_uri = EXCLUDED.redirect_uri,
                authorization_endpoint = EXCLUDED.authorization_endpoint,
                token_endpoint = EXCLUDED.token_endpoint,
                jwks_uri = EXCLUDED.jwks_uri,
                actif = true,
                updated_at = now()
            RETURNING id
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(p.etablissement_id)
        .bind(p.slug)
        .bind(p.libelle)
        .bind(p.issuer)
        .bind(p.client_id)
        .bind(p.client_secret)
        .bind(p.scopes)
        .bind(p.redirect_uri)
        .bind(p.authorization_endpoint)
        .bind(p.token_endpoint)
        .bind(p.jwks_uri)
        .fetch_one(&self.pool)
        .await?;
        Ok(id.0)
    }

    /// Fournisseurs d'un établissement (vue gestion, staff).
    pub async fn list_providers(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<OidcProviderRow>, sqlx::Error> {
        let sql = format!("{PROVIDER_SELECT} WHERE etablissement_id = $1 ORDER BY libelle");
        let rows: Vec<ProviderCols> = sqlx::query_as(&sql)
            .bind(etablissement_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(provider_from_row).collect())
    }

    /// Fournisseurs **actifs** exposés publiquement (page de connexion).
    pub async fn list_public(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<OidcProviderPublic>, sqlx::Error> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            r#"SELECT slug, libelle FROM ent_oidc_provider
               WHERE etablissement_id = $1 AND actif = true ORDER BY libelle"#,
        )
        .bind(etablissement_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(slug, libelle)| OidcProviderPublic { slug, libelle })
            .collect())
    }

    pub async fn get_provider(
        &self,
        etablissement_id: Uuid,
        slug: &str,
    ) -> Result<Option<OidcProviderRow>, sqlx::Error> {
        let sql = format!("{PROVIDER_SELECT} WHERE etablissement_id = $1 AND slug = $2");
        let row: Option<ProviderCols> = sqlx::query_as(&sql)
            .bind(etablissement_id)
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(provider_from_row))
    }

    pub async fn get_provider_by_id(&self, id: Uuid) -> Result<Option<OidcProviderRow>, sqlx::Error> {
        let sql = format!("{PROVIDER_SELECT} WHERE id = $1");
        let row: Option<ProviderCols> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(provider_from_row))
    }

    /// Supprime un fournisseur (les rattachements partent en cascade).
    pub async fn delete_provider(
        &self,
        etablissement_id: Uuid,
        slug: &str,
    ) -> Result<bool, sqlx::Error> {
        let res = sqlx::query(
            "DELETE FROM ent_oidc_provider WHERE etablissement_id = $1 AND slug = $2",
        )
        .bind(etablissement_id)
        .bind(slug)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    /// Compte local rattaché à un `sub` d'IdP.
    pub async fn find_identity(
        &self,
        provider_id: Uuid,
        subject: &str,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT user_id FROM ent_oidc_identity WHERE provider_id = $1 AND subject = $2",
        )
        .bind(provider_id)
        .bind(subject)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(u,)| u))
    }

    /// Rattache un `sub` à un compte local (première connexion réussie).
    pub async fn link_identity(
        &self,
        provider_id: Uuid,
        subject: &str,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO ent_oidc_identity (id, provider_id, subject, user_id)
               VALUES ($1,$2,$3,$4)
               ON CONFLICT (provider_id, subject) DO UPDATE SET user_id = EXCLUDED.user_id"#,
        )
        .bind(Uuid::new_v4())
        .bind(provider_id)
        .bind(subject)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Ouvre un état de parcours (state/PKCE/nonce), à usage unique.
    pub async fn insert_state(&self, s: OidcStateInsert<'_>) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO ent_oidc_state (
                id, token_hash, provider_id, etablissement_id, code_verifier,
                nonce, redirect_uri, created_at, expires_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7, now(), $8)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(s.token_hash)
        .bind(s.provider_id)
        .bind(s.etablissement_id)
        .bind(s.code_verifier)
        .bind(s.nonce)
        .bind(s.redirect_uri)
        .bind(s.expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Consomme un état : `DELETE ... RETURNING` le rend **à usage unique**
    /// (un `state` rejoué ne trouve plus rien) et l'expiration est vérifiée.
    pub async fn take_state(&self, token_hash: &str) -> Result<Option<OidcStateRow>, sqlx::Error> {
        let row: Option<(Uuid, Uuid, Uuid, String, String, String)> = sqlx::query_as(
            r#"DELETE FROM ent_oidc_state
               WHERE token_hash = $1 AND expires_at > now()
               RETURNING id, provider_id, etablissement_id, code_verifier, nonce, redirect_uri"#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(id, provider_id, etablissement_id, code_verifier, nonce, redirect_uri)| {
                OidcStateRow {
                    id,
                    provider_id,
                    etablissement_id,
                    code_verifier,
                    nonce,
                    redirect_uri,
                }
            },
        ))
    }

    pub async fn prune_expired_states(&self) -> Result<u64, sqlx::Error> {
        let res = sqlx::query("DELETE FROM ent_oidc_state WHERE expires_at <= now()")
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }
}
