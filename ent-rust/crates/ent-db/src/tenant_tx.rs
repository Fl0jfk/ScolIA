//! Transaction « locataire » : ouvre une transaction et y pose
//! `app.etablissement_id` (`set_config(..., true)` = valable pour la seule
//! transaction, jamais conservé sur la connexion rendue au pool).
//!
//! C'est le pendant applicatif de la RLS (migration 100, `docs/RLS_ISOLATION.md`).
//! Mode `ENT_RLS_MODE` (lu une fois) :
//! - `off` (défaut) : aucune variable posée -> les politiques restent permissives ;
//! - `permissif` : `app.etablissement_id` posé ; une requête sans variable reste permise ;
//! - `strict` : en plus `app.rls_strict = on` pour la transaction (à défaut d'un
//!   réglage au niveau du rôle, recommandé : `ALTER ROLE ... SET app.rls_strict = 'on'`).

use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlsMode {
    Off,
    Permissif,
    Strict,
}

impl RlsMode {
    pub fn parse(v: Option<&str>) -> Self {
        match v.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
            Some("strict") => Self::Strict,
            Some("permissif") | Some("on") | Some("1") => Self::Permissif,
            _ => Self::Off,
        }
    }

    pub fn from_env() -> Self {
        #[cfg(test)]
        if let Some(m) = MODE_TEST.with(|c| c.get()) {
            return m;
        }
        static MODE: OnceLock<RlsMode> = OnceLock::new();
        *MODE.get_or_init(|| Self::parse(std::env::var("ENT_RLS_MODE").ok().as_deref()))
    }
}

// Tests : force le mode pour le fil courant (`#[tokio::test]` = un seul fil), sans
// toucher à la variable d'environnement lue une seule fois par processus.
#[cfg(test)]
thread_local! {
    static MODE_TEST: std::cell::Cell<Option<RlsMode>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn forcer_mode_test(mode: Option<RlsMode>) {
    MODE_TEST.with(|c| c.set(mode));
}

pub struct TenantTx<'c> {
    tx: Transaction<'c, Postgres>,
}

impl TenantTx<'static> {
    /// Ouvre la transaction et pose l'établissement (selon `ENT_RLS_MODE`).
    pub async fn begin(pool: &PgPool, etablissement_id: Uuid) -> Result<Self, sqlx::Error> {
        Self::begin_mode(pool, etablissement_id, RlsMode::from_env()).await
    }

    pub async fn begin_mode(
        pool: &PgPool,
        etablissement_id: Uuid,
        mode: RlsMode,
    ) -> Result<Self, sqlx::Error> {
        let mut tx = pool.begin().await?;
        if mode != RlsMode::Off {
            sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
                .bind(etablissement_id.to_string())
                .execute(&mut *tx)
                .await?;
            if mode == RlsMode::Strict {
                sqlx::query("SELECT set_config('app.rls_strict', 'on', true)")
                    .execute(&mut *tx)
                    .await?;
            }
        }
        Ok(Self { tx })
    }
}

impl TenantTx<'_> {
    pub async fn commit(self) -> Result<(), sqlx::Error> {
        self.tx.commit().await
    }
}

impl Deref for TenantTx<'_> {
    type Target = PgConnection;
    fn deref(&self) -> &PgConnection {
        &self.tx
    }
}

impl DerefMut for TenantTx<'_> {
    fn deref_mut(&mut self) -> &mut PgConnection {
        &mut self.tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_par_defaut_desactive() {
        assert_eq!(RlsMode::parse(None), RlsMode::Off);
        assert_eq!(RlsMode::parse(Some("")), RlsMode::Off);
        assert_eq!(RlsMode::parse(Some("bizarre")), RlsMode::Off);
        assert_eq!(RlsMode::parse(Some("Permissif")), RlsMode::Permissif);
        assert_eq!(RlsMode::parse(Some(" STRICT ")), RlsMode::Strict);
    }
}
