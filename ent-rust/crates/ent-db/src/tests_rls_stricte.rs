//! Exécution des suites de domaine **en mode strict** (docs/securite/RLS_STRICTE.md §5).
//!
//! `ENT_RLS_MODE=strict` (la variable de production) bascule le harnais des suites converties :
//! - la base est migrée et le jeu de labo semé par le **propriétaire** (préparation, hors périmètre
//!   testé) ;
//! - le code testé (`PgEnt` et tous ses dépôts) tourne sur un rôle applicatif **non propriétaire,
//!   sans BYPASSRLS, `app.rls_strict = on` au niveau du rôle** : un accès qui oublie `TenantTx`
//!   voit 0 ligne et ne peut rien écrire (fail-closed) ;
//! - les vérifications SQL directes des tests passent par `PgEnt::pool_verif()` (propriétaire).
//!
//! Sans la variable, ce harnais est transparent : mêmes suites, rôle de CI habituel.
//! Une suite qui passe dans les deux modes prouve qu'elle n'utilise aucun chemin sans contexte.

use super::*;
use crate::tenant_tx::{forcer_mode_test, RlsMode};
use sqlx::postgres::PgPoolOptions;

pub(crate) const ROLE_STRICT: &str = "ent_suite_stricte";

pub(crate) fn mode_strict_demande() -> bool {
    RlsMode::from_env() == RlsMode::Strict
}

/// Rôle applicatif strict (idempotent, sérialisé : plusieurs processus de test partagent la base).
async fn assurer_role_strict(proprietaire: &sqlx::PgPool) {
    let mut c = proprietaire.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock(7770042)").execute(&mut *c).await.unwrap();
    for sql in [
        format!(
            "DO $$ BEGIN CREATE ROLE {ROLE_STRICT} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB \
             NOCREATEROLE; EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL; END $$"
        ),
        format!("GRANT USAGE ON SCHEMA public TO {ROLE_STRICT}"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {ROLE_STRICT}"),
        format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {ROLE_STRICT}"),
        format!("ALTER ROLE {ROLE_STRICT} SET app.rls_strict = 'on'"),
        // Les migrations rejouées (DROP + CREATE de vues…) ne doivent pas faire perdre les droits.
        format!(
            "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO {ROLE_STRICT}"
        ),
        format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO {ROLE_STRICT}"),
    ] {
        sqlx::query(&sql).execute(&mut *c).await.unwrap();
    }
    sqlx::query("SELECT pg_advisory_unlock(7770042)").execute(&mut *c).await.unwrap();
}

/// Base prête pour une suite : migrée, jeu de labo semé. En mode strict, le `PgEnt` rendu est celui du
/// rôle applicatif strict, `pool_verif()` donne le propriétaire. `installer` règle le `PgEnt` testé
/// (dry-run, brain…) avant tout usage.
pub(crate) async fn labo_selon_mode(
    url: &str,
    installer: impl FnOnce(&mut PgEnt),
) -> (PgEnt, LaboCast) {
    assert_local_postgres_url(url).expect("ENT_DATABASE_URL doit être locale");
    let mut proprietaire = PgEnt::connect(url).await.expect("connect");
    proprietaire.migrate().await.expect("migrate");
    if !mode_strict_demande() {
        installer(&mut proprietaire);
        let cast = proprietaire.seed_labo_cast().await.expect("seed cast");
        return (proprietaire, cast);
    }
    assurer_role_strict(proprietaire.pool()).await;
    let mut u = url::Url::parse(url).unwrap();
    u.set_username(ROLE_STRICT).unwrap();
    u.set_password(Some("rls")).unwrap();
    let pool_app = PgPoolOptions::new().max_connections(5).connect(u.as_str()).await.unwrap();
    let mut strict = PgEnt::from_pool(pool_app);
    installer(&mut strict);
    // Préparation par le propriétaire (le jeu de labo n'est pas le périmètre testé).
    let cast = proprietaire.seed_labo_cast().await.expect("seed cast");
    strict.pool_verif = Some(proprietaire.pool().clone());
    forcer_mode_test(Some(RlsMode::Strict));
    (strict, cast)
}

/// Jeu de labo supplémentaire (autre établissement) : préparation par le propriétaire, comme `labo_selon_mode`.
pub(crate) async fn seed_supplementaire(ent: &PgEnt) -> LaboCast {
    let proprietaire = PgEnt::from_pool(ent.pool_verif().clone());
    proprietaire.seed_labo_cast().await.expect("seed cast")
}

/// Le harnais strict n'est pas un faux positif : sous le rôle strict, une requête **sans contexte**
/// ne voit rien, et un `TenantTx` strict voit exactement son établissement.
#[tokio::test]
async fn harnais_strict_est_bien_strict() {
    let Some(url) = crate::url_guard::url_labo_ou_echec() else { return };
    if !mode_strict_demande() {
        return;
    }
    let (ent, cast) = labo_selon_mode(&url, |_| {}).await;
    let (sans_contexte,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve").fetch_one(ent.pool()).await.unwrap();
    assert_eq!(sans_contexte, 0, "sans contexte, le rôle strict ne doit rien voir");
    let (role,): (String,) = sqlx::query_as("SELECT current_user::text").fetch_one(ent.pool()).await.unwrap();
    assert_eq!(role, ROLE_STRICT);
    let mut tx = crate::tenant_tx::TenantTx::begin(ent.pool(), cast.etablissement_id).await.unwrap();
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_membership WHERE etablissement_id = $1")
        .bind(cast.etablissement_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert!(n > 0);
}
