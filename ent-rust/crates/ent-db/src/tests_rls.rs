//! Socle 1 / C4 — RLS pilote (migration 100) : tests avec un rôle applicatif
//! **non propriétaire**, sans BYPASSRLS, dans la base locale de test. Un
//! superutilisateur contourne toujours la RLS : c'est pourquoi la connexion de
//! test habituelle (propriétaire) sert uniquement à préparer les données.
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use crate::tenant_tx::{RlsMode, TenantTx};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

const ROLE: &str = "ent_rls_app";

struct Env {
    admin: PgPool,
    app: PgPool,
    etab_a: Uuid,
    etab_b: Uuid,
    eleve: Uuid,
    user: Uuid,
    fiche_a: Uuid,
    fiche_b: Uuid,
}

async fn preparer() -> Option<Env> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    let admin = PgPoolOptions::new().max_connections(2).connect(&url).await.unwrap();

    // Rôle applicatif : ni propriétaire, ni BYPASSRLS, ni superutilisateur.
    sqlx::query(&format!(
        "DO $$ BEGIN CREATE ROLE {ROLE} LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; \
         EXCEPTION WHEN duplicate_object THEN NULL; END $$"
    ))
    .execute(&admin)
    .await
    .unwrap();
    for sql in [
        format!("GRANT USAGE ON SCHEMA public TO {ROLE}"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ent_fiche_sante, ent_psy_dossier TO {ROLE}"),
        // Fail-closed au niveau du rôle : toute connexion qui oublie TenantTx ne voit rien.
        format!("ALTER ROLE {ROLE} SET app.rls_strict = 'on'"),
    ] {
        sqlx::query(&sql).execute(&admin).await.unwrap();
    }
    // Connexion applicative = même base, autre rôle.
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username(ROLE).unwrap();
    u.set_password(Some("rls")).unwrap();
    let app = PgPoolOptions::new().max_connections(2).connect(u.as_str()).await.unwrap();

    let (etab_a, etab_b) = (cast.etablissement_id, Uuid::new_v4());
    let (eleve, user) = (cast.eleve_lea_id, cast.direction.user_id);
    let (fiche_a, fiche_b) = (Uuid::new_v4(), Uuid::new_v4());
    // Une fiche par établissement pour le même élève (l'unicité est par établissement).
    sqlx::query("DELETE FROM ent_fiche_sante WHERE eleve_id = $1").bind(eleve).execute(&admin).await.unwrap();
    for (id, etab, note) in [(fiche_a, etab_a, "note A"), (fiche_b, etab_b, "note B")] {
        sqlx::query(
            "INSERT INTO ent_fiche_sante (id, etablissement_id, eleve_id, note_confidentielle, updated_by_user_id) \
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(id).bind(etab).bind(eleve).bind(note).bind(user)
        .execute(&admin).await.unwrap();
    }
    Some(Env { admin, app, etab_a, etab_b, eleve, user, fiche_a, fiche_b })
}

async fn nettoyer(e: &Env) {
    sqlx::query("DELETE FROM ent_fiche_sante WHERE eleve_id = $1")
        .bind(e.eleve)
        .execute(&e.admin)
        .await
        .unwrap();
}

async fn compter(c: &mut sqlx::PgConnection) -> i64 {
    sqlx::query("SELECT count(*) FROM ent_fiche_sante")
        .fetch_one(c)
        .await
        .unwrap()
        .get(0)
}

#[tokio::test]
async fn rls_strict_sans_etablissement_ne_voit_rien() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Connexion « oubli de TenantTx » : variable absente + rôle strict -> 0 ligne.
    let mut c = e.app.acquire().await.unwrap();
    assert_eq!(compter(&mut c).await, 0);
    assert_eq!(
        sqlx::query("SELECT count(*) FROM ent_psy_dossier").fetch_one(&mut *c).await.unwrap().get::<i64, _>(0),
        0
    );
    // Une écriture sans établissement est refusée (WITH CHECK).
    let r = sqlx::query(
        "INSERT INTO ent_fiche_sante (id, etablissement_id, eleve_id, updated_by_user_id) VALUES ($1,$2,$3,$4)",
    )
    .bind(Uuid::new_v4()).bind(e.etab_a).bind(Uuid::new_v4()).bind(e.user)
    .execute(&mut *c).await;
    assert!(r.is_err(), "insertion sans app.etablissement_id doit être refusée");
    // Une variable vide équivaut à absente.
    drop(c);
    let mut tx = e.app.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', '', true)").execute(&mut *tx).await.unwrap();
    assert_eq!(compter(&mut tx).await, 0);
    drop(tx);
    nettoyer(&e).await;
}

#[tokio::test]
async fn rls_strict_etablissement_pose_isole_lecture_ecriture() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Sans filtre applicatif sur etablissement_id : c'est la base qui isole.
    let mut a = TenantTx::begin_mode(&e.app, e.etab_a, RlsMode::Strict).await.unwrap();
    assert_eq!(compter(&mut a).await, 1);
    let ids: Vec<Uuid> = sqlx::query("SELECT id FROM ent_fiche_sante")
        .fetch_all(&mut *a).await.unwrap().iter().map(|r| r.get(0)).collect();
    assert_eq!(ids, vec![e.fiche_a]);
    // Lecture ciblée de la ligne de B : invisible.
    let vu: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM ent_fiche_sante WHERE id = $1")
        .bind(e.fiche_b).fetch_optional(&mut *a).await.unwrap();
    assert!(vu.is_none());
    // Mise à jour de B depuis A : 0 ligne touchée ; de A : 1.
    let n = sqlx::query("UPDATE ent_fiche_sante SET note_confidentielle = 'piratée' WHERE id = $1")
        .bind(e.fiche_b).execute(&mut *a).await.unwrap().rows_affected();
    assert_eq!(n, 0);
    let n = sqlx::query("UPDATE ent_fiche_sante SET allergies = 'noix' WHERE id = $1")
        .bind(e.fiche_a).execute(&mut *a).await.unwrap().rows_affected();
    assert_eq!(n, 1);
    // Suppression de B depuis A : 0 ligne.
    let n = sqlx::query("DELETE FROM ent_fiche_sante WHERE id = $1")
        .bind(e.fiche_b).execute(&mut *a).await.unwrap().rows_affected();
    assert_eq!(n, 0);
    a.commit().await.unwrap();

    // Insertion d'une ligne de B depuis A : refusée (WITH CHECK, 42501).
    let mut a = TenantTx::begin_mode(&e.app, e.etab_a, RlsMode::Strict).await.unwrap();
    let r = sqlx::query(
        "INSERT INTO ent_fiche_sante (id, etablissement_id, eleve_id, updated_by_user_id) VALUES ($1,$2,$3,$4)",
    )
    .bind(Uuid::new_v4()).bind(e.etab_b).bind(e.eleve).bind(e.user)
    .execute(&mut *a).await;
    let err = r.expect_err("insertion inter-établissement refusée");
    assert_eq!(err.as_database_error().and_then(|d| d.code()).as_deref(), Some("42501"));
    drop(a);

    // Symétrique : B voit sa ligne, pas celle de A ; A n'a pas été modifiée par B.
    let mut b = TenantTx::begin_mode(&e.app, e.etab_b, RlsMode::Strict).await.unwrap();
    let notes: Vec<String> = sqlx::query("SELECT note_confidentielle FROM ent_fiche_sante")
        .fetch_all(&mut *b).await.unwrap().iter().map(|r| r.get(0)).collect();
    assert_eq!(notes, vec!["note B".to_string()]);
    drop(b);
    // Vérité terrain côté propriétaire (contourne la RLS) : B intacte, A mise à jour.
    let rows = sqlx::query("SELECT id, note_confidentielle, allergies FROM ent_fiche_sante WHERE eleve_id = $1")
        .bind(e.eleve).fetch_all(&e.admin).await.unwrap();
    assert_eq!(rows.len(), 2);
    for r in rows {
        let (id, note, all): (Uuid, String, String) = (r.get(0), r.get(1), r.get(2));
        if id == e.fiche_b {
            assert_eq!((note.as_str(), all.as_str()), ("note B", ""));
        } else {
            assert_eq!(all, "noix");
        }
    }

    // La variable est locale à la transaction : la connexion rendue au pool est « propre ».
    let mut c = e.app.acquire().await.unwrap();
    assert_eq!(compter(&mut c).await, 0);
    drop(c);
    nettoyer(&e).await;
}

#[tokio::test]
async fn rls_non_strict_sans_variable_reste_permissive() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Mode « permissif » (comportement actuel tant que le mode strict n'est pas posé).
    let mut tx = e.app.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.rls_strict', 'off', true)").execute(&mut *tx).await.unwrap();
    assert!(compter(&mut tx).await >= 2, "permissif : les deux établissements visibles");
    drop(tx);
    // Même en permissif, une variable posée filtre.
    let mut tx = TenantTx::begin_mode(&e.app, e.etab_b, RlsMode::Permissif).await.unwrap();
    sqlx::query("SELECT set_config('app.rls_strict', 'off', true)").execute(&mut *tx).await.unwrap();
    assert_eq!(compter(&mut tx).await, 1);
    drop(tx);
    nettoyer(&e).await;
}

#[tokio::test]
async fn rls_mode_off_ne_pose_rien() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let mut tx = TenantTx::begin_mode(&e.app, e.etab_a, RlsMode::Off).await.unwrap();
    let v: Option<String> = sqlx::query("SELECT current_setting('app.etablissement_id', true)")
        .fetch_one(&mut *tx).await.unwrap().get(0);
    assert!(v.unwrap_or_default().is_empty());
    drop(tx);
    nettoyer(&e).await;
}

#[tokio::test]
async fn rls_politiques_presentes_et_forcees() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let n: i64 = sqlx::query(
        "SELECT count(*) FROM pg_class c JOIN pg_policies p ON p.tablename = c.relname \
         WHERE c.relrowsecurity AND c.relforcerowsecurity AND p.policyname = 'ent_rls_etablissement'",
    )
    .fetch_one(&e.admin).await.unwrap().get(0);
    assert_eq!(n, 22, "22 tables pilotes avec RLS activée et forcée");
    // Le rôle de test n'est ni propriétaire ni contournement.
    let (bypass, sup): (bool, bool) = sqlx::query_as("SELECT rolbypassrls, rolsuper FROM pg_roles WHERE rolname = $1")
        .bind(ROLE).fetch_one(&e.admin).await.unwrap();
    assert!(!bypass && !sup);
    nettoyer(&e).await;
}

#[tokio::test]
async fn role_applicatif_ne_peut_pas_desactiver_le_trigger_du_journal() {
    let Some(e) = preparer().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    // Non propriétaire : ni DDL, ni désactivation du trigger d'ajout seul.
    for sql in [
        "ALTER TABLE ent_journal_modif DISABLE TRIGGER ent_journal_modif_ajout_seul_trg",
        "ALTER TABLE ent_fiche_sante DISABLE ROW LEVEL SECURITY",
        "DROP POLICY ent_rls_etablissement ON ent_fiche_sante",
    ] {
        let err = sqlx::query(sql).execute(&e.app).await.expect_err(sql);
        assert_eq!(err.as_database_error().and_then(|d| d.code()).as_deref(), Some("42501"), "{sql}");
    }
    nettoyer(&e).await;
}

#[tokio::test]
async fn migrations_via_url_proprietaire_dediee() {
    let Some(url) = std::env::var("ENT_DATABASE_URL").ok() else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ent = PgEnt::connect(&url).await.expect("connect");
    // Variable absente / vide : comportement historique.
    std::env::remove_var("ENT_DATABASE_MIGRATION_URL");
    ent.migrate().await.expect("migrate sans URL dédiée");
    std::env::set_var("ENT_DATABASE_MIGRATION_URL", "  ");
    ent.migrate().await.expect("migrate URL vide");
    // URL propriétaire locale : utilisée.
    std::env::set_var("ENT_DATABASE_MIGRATION_URL", &url);
    let ok = ent.migrate().await;
    // Hôte distant refusé (garde-fou), jamais de connexion.
    std::env::set_var("ENT_DATABASE_MIGRATION_URL", "postgres://u:p@51.158.57.80:5432/db");
    let ko = ent.migrate().await;
    std::env::remove_var("ENT_DATABASE_MIGRATION_URL");
    ok.expect("migrate via URL propriétaire");
    assert!(matches!(ko, Err(EntDbError::Url(_))));
}
