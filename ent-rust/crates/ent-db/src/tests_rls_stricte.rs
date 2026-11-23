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
pub(crate) async fn assurer_role_strict(proprietaire: &sqlx::PgPool) {
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
        // MB0 (M-C) : `ent_push_reaffecter` (SECURITY DEFINER) n'est plus exécutable par PUBLIC ; le rôle
        // applicatif reçoit le droit explicitement (comme `ent_app` par la migration 2220 ou l'exploitation).
        format!("GRANT EXECUTE ON FUNCTION ent_push_reaffecter(TEXT, UUID, UUID) TO {ROLE_STRICT}"),
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

// ---- MB0, passe 2 : registre d'appareils de notification en RLS stricte (F-5, F-6) --------------------

/// Base migrée par le propriétaire + `PgEnt` du rôle strict, **quel que soit `ENT_RLS_MODE`** (ces tests
/// éprouvent le mode strict eux-mêmes ; le réglage de mode du test est local au fil d'exécution).
async fn proprietaire_et_strict(url: &str) -> (PgEnt, PgEnt) {
    assert_local_postgres_url(url).expect("ENT_DATABASE_URL doit être locale");
    let proprietaire = PgEnt::connect(url).await.expect("connect");
    proprietaire.migrate().await.expect("migrate");
    assurer_role_strict(proprietaire.pool()).await;
    let mut u = url::Url::parse(url).unwrap();
    u.set_username(ROLE_STRICT).unwrap();
    u.set_password(Some("rls")).unwrap();
    let pool_app = PgPoolOptions::new()
        .max_connections(5)
        .connect(u.as_str())
        .await
        .unwrap();
    let mut strict = PgEnt::from_pool(pool_app);
    strict.pool_verif = Some(proprietaire.pool().clone());
    forcer_mode_test(Some(RlsMode::Strict));
    (proprietaire, strict)
}

const MDP_MB0: &str = "Mobile-Mb0-Stricte-1!";

async fn compte_et_session(proprietaire: &PgEnt, etab: uuid::Uuid) -> (uuid::Uuid, SessionInfo) {
    let email = format!("mb0s-{}@ent.local", uuid::Uuid::new_v4().simple());
    let u = proprietaire
        .register_user(&email, MDP_MB0, None, etab, "prof")
        .await
        .expect("inscription");
    let b = uuid::Uuid::new_v4().into_bytes();
    let ip = format!("10.{}.{}.{}", b[0], b[1], b[2]);
    let s = match proprietaire
        .login_start_client(
            &email,
            MDP_MB0,
            etab,
            Some(&ip),
            ClientSession::Web,
            Some("test".into()),
        )
        .await
        .expect("connexion")
    {
        LoginOutcome::Session(s) => s,
        autre => panic!("session attendue : {autre:?}"),
    };
    (u.id, s)
}

/// F-6 : la règle 2 (un jeton, un titulaire, tous établissements) sous RLS stricte : le rôle applicatif ne
/// voit rien d'un autre établissement, la fonction `SECURITY DEFINER` parcourt les établissements, révoque
/// avec trace dans l'établissement d'origine, et ne rend rien de l'autre établissement. Mesure le coût de la
/// boucle sur un grand nombre d'établissements.
#[tokio::test]
async fn reaffectation_entre_etablissements_en_rls_stricte_et_cout_de_la_boucle() {
    let Some(url) = crate::url_guard::url_labo_ou_echec() else {
        return;
    };
    let (proprietaire, strict) = proprietaire_et_strict(&url).await;
    let (etab_a, etab_b) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let (a, sa) = compte_et_session(&proprietaire, etab_a).await;
    let (b, sb) = compte_et_session(&proprietaire, etab_b).await;
    let jeton = format!("tok-{}", uuid::Uuid::new_v4());
    let da = strict
        .register_push_device_for_actor(etab_a, a, "android", &jeton, None, sa.session_id)
        .await
        .expect("A inscrit (rôle strict)");
    let db = strict
        .register_push_device_for_actor(etab_b, b, "android", &jeton, None, sb.session_id)
        .await
        .expect("B inscrit (rôle strict)");
    // Trace chez A (vérifiée par le propriétaire) ; B seul actif.
    let (revoque, motif): (bool, Option<String>) =
        sqlx::query_as("SELECT revoked_at IS NOT NULL, revoque_motif FROM ent_push_device WHERE id = $1 AND etablissement_id = $2")
            .bind(da.id)
            .bind(etab_a)
            .fetch_one(proprietaire.pool())
            .await
            .expect("ligne de A");
    assert!(revoque);
    assert_eq!(motif.as_deref(), Some("reaffecte"));
    // Le rôle strict, dans le contexte de B, ne voit ni la ligne de A, ni l'identité de A.
    let mut tx = crate::tenant_tx::TenantTx::begin(strict.pool(), etab_b)
        .await
        .unwrap();
    let vus: Vec<(uuid::Uuid,)> =
        sqlx::query_as("SELECT user_id FROM ent_push_device WHERE token_empreinte = $1")
            .bind(ent_auth::hash_token(&jeton))
            .fetch_all(&mut *tx)
            .await
            .unwrap();
    assert_eq!(vus, vec![(b,)], "B ne voit que sa propre ligne");
    drop(tx);
    let evenements: Vec<String> =
        sqlx::query_scalar("SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2")
            .bind(etab_b)
            .bind(db.id)
            .fetch_all(proprietaire.pool())
            .await
            .unwrap();
    assert!(evenements
        .iter()
        .all(|e| !e.contains(&a.to_string()) && !e.contains(&etab_a.to_string())));

    // Coût de la boucle : de nombreux établissements (créés puis retirés par le propriétaire).
    const N: usize = 300;
    let marque = uuid::Uuid::new_v4().simple().to_string();
    sqlx::query(
        "INSERT INTO ent_etablissement (id, slug, nom)
         SELECT gen_random_uuid(), 'mb0-' || $1 || '-' || g, 'MB0 mesure ' || g FROM generate_series(1, $2) g",
    )
    .bind(&marque)
    .bind(N as i32)
    .execute(proprietaire.pool())
    .await
    .expect("établissements de mesure");
    let (total,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM (SELECT id FROM ent_etablissement UNION SELECT etablissement_id FROM ent_membership) q",
    )
    .fetch_one(proprietaire.pool())
    .await
    .unwrap();
    let empreinte = ent_auth::hash_token(&format!("tok-{}", uuid::Uuid::new_v4()));
    let debut = std::time::Instant::now();
    let mut tx = crate::tenant_tx::TenantTx::begin(strict.pool(), etab_b)
        .await
        .unwrap();
    let rendus: Vec<(uuid::Uuid,)> = sqlx::query_as("SELECT ent_push_reaffecter($1, $2, $3)")
        .bind(&empreinte)
        .bind(b)
        .bind(etab_b)
        .fetch_all(&mut *tx)
        .await
        .expect("appel sous rôle strict");
    drop(tx);
    let ecoule = debut.elapsed();
    sqlx::query("DELETE FROM ent_etablissement WHERE slug LIKE $1")
        .bind(format!("mb0-{marque}-%"))
        .execute(proprietaire.pool())
        .await
        .expect("nettoyage");
    eprintln!("MB0 règle 2 : boucle sur {total} établissements, un appel : {ecoule:?}");
    assert!(rendus.is_empty());
    assert!(
        ecoule < std::time::Duration::from_secs(5),
        "boucle trop lente : {ecoule:?} pour {total} établissements"
    );
    forcer_mode_test(None);
}

/// F-5 : les mises à jour de données de la migration 2220 atteignent bien les lignes **même si le lanceur de
/// migrations tourne avec `app.rls_strict` actif** : sans contexte, un `UPDATE` naïf ne touche rien (0 ligne, sans
/// erreur) ; le bloc `DEBUT-DONNEES … FIN-DONNEES` pose le contexte établissement par établissement.
#[tokio::test]
async fn migration_2220_met_a_jour_les_donnees_meme_sous_rls_stricte() {
    let Some(url) = crate::url_guard::url_labo_ou_echec() else {
        return;
    };
    let (proprietaire, strict) = proprietaire_et_strict(&url).await;
    let sql = include_str!("../../../migrations/2220_mob_mb0_appareils_push.sql");
    let (etab_1, etab_2) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let (u1, _) = compte_et_session(&proprietaire, etab_1).await;
    let (u2, _) = compte_et_session(&proprietaire, etab_2).await;
    // État antérieur au lot : empreinte absente, jeton en clair dans l'historique d'envoi.
    sqlx::query(
        "ALTER TABLE ent_push_outbox DROP CONSTRAINT IF EXISTS ent_push_outbox_sans_jeton_chk",
    )
    .execute(proprietaire.pool())
    .await
    .unwrap();
    let mut anciens = Vec::new();
    for (etab, user, age) in [(etab_1, u1, "40 days"), (etab_2, u2, "1 hour")] {
        let id = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_push_device (id, etablissement_id, user_id, platform, token, last_seen_at)
             VALUES ($1, $2, $3, 'ios', $4, now() - $5::interval)",
        )
        .bind(id)
        .bind(etab)
        .bind(user)
        .bind(format!("legacy-{id}"))
        .bind(age)
        .execute(proprietaire.pool())
        .await
        .unwrap();
        sqlx::query("UPDATE ent_push_device SET token_empreinte = NULL WHERE id = $1")
            .bind(id)
            .execute(proprietaire.pool())
            .await
            .unwrap();
        let sortie = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_push_outbox (id, etablissement_id, kind, title, body, status, token)
             VALUES ($1, $2, 'test', 't', 'b', 'sent', $3)",
        )
        .bind(sortie)
        .bind(etab)
        .bind(format!("legacy-{id}"))
        .execute(proprietaire.pool())
        .await
        .unwrap();
        anciens.push((id, sortie));
    }
    // Préalable : sous le rôle strict, sans contexte, un UPDATE ordinaire ne touche aucune ligne.
    let naif = sqlx::query(
        "UPDATE ent_push_device SET token_empreinte = 'x' WHERE token_empreinte IS NULL",
    )
    .execute(strict.pool())
    .await
    .expect("UPDATE sans contexte");
    assert_eq!(
        naif.rows_affected(),
        0,
        "préalable : la RLS stricte masque tout sans contexte"
    );
    // Le bloc de données de la migration, exécuté par le rôle strict.
    let debut = sql.find("-- DEBUT-DONNEES").expect("repère de début") + "-- DEBUT-DONNEES".len();
    let fin = sql.find("-- FIN-DONNEES").expect("repère de fin");
    sqlx::raw_sql(&sql[debut..fin])
        .execute(strict.pool())
        .await
        .expect("bloc de données sous rôle strict");
    for (i, (id, sortie)) in anciens.iter().enumerate() {
        let (empreinte, revoque, motif): (Option<String>, bool, Option<String>) = sqlx::query_as(
            "SELECT token_empreinte, revoked_at IS NOT NULL, revoque_motif FROM ent_push_device WHERE id = $1",
        )
        .bind(id)
        .fetch_one(proprietaire.pool())
        .await
        .unwrap();
        assert!(empreinte.is_some(), "empreinte remplie ({i})");
        // La première inscription (40 jours, sans session) est révoquée, la seconde (1 h) reste.
        assert_eq!(
            (revoque, motif.as_deref()),
            if i == 0 {
                (true, Some("expire"))
            } else {
                (false, None)
            }
        );
        let (token, empreinte_sortie): (Option<String>, Option<String>) =
            sqlx::query_as("SELECT token, token_empreinte FROM ent_push_outbox WHERE id = $1")
                .bind(sortie)
                .fetch_one(proprietaire.pool())
                .await
                .unwrap();
        assert!(
            token.is_none() && empreinte_sortie.is_some(),
            "jeton retiré de l'historique ({i})"
        );
    }
    // Remet la contrainte (rejeu complet par le propriétaire).
    sqlx::raw_sql(sql)
        .execute(proprietaire.pool())
        .await
        .expect("rejeu");
    forcer_mode_test(None);
}
