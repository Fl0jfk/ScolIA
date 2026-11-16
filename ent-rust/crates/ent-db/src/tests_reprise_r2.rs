//! Socle 28, lot R2 : zone de reprise, simulation, correspondance, contre une vraie base. Export fictif
//! décodable (aucune donnée réelle). Sans `ENT_DATABASE_URL`, ignoré en local (échec en CI).

use crate::reprise::{self, CibleExecution, ModeExecution};
use crate::reprise_zone::{self, executer_simulation};
use crate::tenant_tx::TenantTx;
use chrono::{TimeZone, Utc};
use ent_reprise::correspondance::Statut;
use ent_reprise::decodage::OptionsDecodage;
use ent_reprise::jeu_fictif;
use sqlx::{PgPool, Row};
use std::path::PathBuf;
use uuid::Uuid;

async fn base() -> Option<PgPool> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    crate::url_guard::assert_local_postgres_url(&url).expect("base locale");
    let pool = PgPool::connect(&url).await.expect("connect");
    crate::migrate::apply_socle_migrations(&pool)
        .await
        .expect("migrate");
    Some(pool)
}

fn jeu() -> (PathBuf, OptionsDecodage) {
    let d = std::env::temp_dir().join(format!("ent-r2-db-{}", Uuid::new_v4()));
    jeu_fictif::generer_decodable(&d, 3).expect("jeu fictif");
    let o = OptionsDecodage {
        dump: d.join("dump.sql"),
        objets_s3: Some(d.join("objets_s3.tsv")),
        contenus_s3: Some(d.join("contenus_s3")),
        maintenant: Utc.with_ymd_and_hms(2026, 11, 16, 12, 0, 0).unwrap(),
    };
    (d, o)
}

/// Lignes de chaque table du schéma `public` hors `rep_*`.
async fn instantane_metier(pool: &PgPool) -> Vec<(String, i64)> {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT c.relname::text FROM pg_class c WHERE c.relnamespace = 'public'::regnamespace
            AND c.relkind IN ('r', 'p') AND c.relname NOT LIKE 'rep\\_%' ORDER BY 1",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    let mut out = Vec::new();
    for t in tables {
        let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM \"{t}\""))
            .fetch_one(pool)
            .await
            .unwrap();
        out.push((t, n));
    }
    out
}

async fn compter(pool: &PgPool, etab: Uuid, table: &str) -> i64 {
    let mut tx = TenantTx::begin(pool, etab).await.unwrap();
    let n = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM {table} WHERE etablissement_id = $1"
    ))
    .bind(etab)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    n
}

#[tokio::test]
async fn la_simulation_n_ecrit_rien_dans_le_metier() {
    let Some(pool) = base().await else { return };
    let (_, o) = jeu();
    let etab = Uuid::new_v4();
    let avant = instantane_metier(&pool).await;
    let r = executer_simulation(&pool, etab, &o, CibleExecution::Jetable, None)
        .await
        .expect("simulation");
    let apres = instantane_metier(&pool).await;
    assert_eq!(avant, apres, "la simulation a écrit dans une table métier");
    assert_eq!(r.simulation.ecritures_metier, 0);
    assert!(r.simulation.invariants_ok());
    assert!(r.simulation.total.creees > 0 && r.simulation.total.a_decider > 0);
    // Ni correspondance enregistrée par une simulation.
    assert_eq!(compter(&pool, etab, "rep_correspondance").await, 0);
    // La trace existe : exécution terminée, comptes, lignes à décider, refus (santé).
    assert_eq!(compter(&pool, etab, "rep_execution").await, 1);
    assert_eq!(
        compter(&pool, etab, "rep_execution_compte").await,
        r.simulation.par_objet.len() as i64
    );
    assert_eq!(
        compter(&pool, etab, "rep_execution_a_decider").await,
        r.simulation.a_decider.len() as i64
    );
    assert_eq!(
        compter(&pool, etab, "rep_refus").await,
        r.decodage.refus.len() as i64
    );
    let mut tx = TenantTx::begin(&pool, etab).await.unwrap();
    let ligne = sqlx::query("SELECT etat, mode, rapport_empreinte, numero FROM rep_execution WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(ligne.get::<String, _>("etat"), "terminee");
    assert_eq!(ligne.get::<String, _>("mode"), "simulation");
    assert_eq!(
        ligne.get::<String, _>("rapport_empreinte"),
        r.rapport_empreinte
    );
    // Somme des comptes en base = total du rapport.
    let somme: i64 = sqlx::query_scalar("SELECT COALESCE(sum(lues), 0)::bigint FROM rep_execution_compte WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(somme as u64, r.simulation.total.lues);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn zone_chargee_relue_puis_purgee() {
    let Some(pool) = base().await else { return };
    let (_, o) = jeu();
    let etab = Uuid::new_v4();
    let r = executer_simulation(&pool, etab, &o, CibleExecution::Jetable, None)
        .await
        .unwrap();
    assert!(r.zone["reprise.next_travel"] > 0);
    assert_eq!(
        reprise_zone::compter_zone(&pool, etab, r.execution.id)
            .await
            .unwrap(),
        r.zone
    );
    // Une autre exécution ou un autre établissement ne voit rien de cette zone.
    assert!(reprise_zone::compter_zone(&pool, etab, Uuid::new_v4())
        .await
        .unwrap()
        .is_empty());
    assert!(
        reprise_zone::compter_zone(&pool, Uuid::new_v4(), r.execution.id)
            .await
            .unwrap()
            .is_empty()
    );
    // Documents S3 en attente : typés, jeton non conservé dans la référence.
    assert_eq!(r.zone["reprise.next_s3_rentree_depot"], 1);
    assert_eq!(r.zone["reprise.next_s3_demande_attente"], 1);
    assert_eq!(r.zone["reprise.next_s3_demande_piece"], 1);
    assert_eq!(r.zone["reprise.next_s3_demande_enfant"], 1);
    let refs: Vec<String> = sqlx::query_scalar("SELECT reference FROM reprise.next_s3_rentree_depot WHERE _execution_id = $1 AND _etablissement_cible = $2")
        .bind(r.execution.id)
        .bind(etab)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(refs.iter().all(|x| x.len() == 64 && !x.contains("jeton")));
    // Secrets : colonne absente de la zone.
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns WHERE table_schema = 'reprise' AND table_name = 'next_account'
            AND column_name IN ('password', 'access_token', 'refresh_token', 'id_token')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 0);
    // Purge.
    let supprimees = reprise_zone::vider_zone(&pool, etab, r.execution.id)
        .await
        .unwrap();
    assert_eq!(supprimees, r.zone.values().sum::<u64>());
    assert!(reprise_zone::compter_zone(&pool, etab, r.execution.id)
        .await
        .unwrap()
        .is_empty());
    reprise::noter_zone_videe(&pool, etab, r.execution)
        .await
        .unwrap();
    let mut tx = TenantTx::begin(&pool, etab).await.unwrap();
    let vide: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
        "SELECT zone_videe_le FROM rep_execution WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(r.execution.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert!(vide.is_some());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn zone_separee_du_metier_et_invisible_des_autres_roles() {
    let Some(pool) = base().await else { return };
    // Aucune table de zone dans `public`, aucune clé étrangère ni RLS en zone.
    let hors: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_class WHERE relnamespace = 'public'::regnamespace AND relname LIKE 'next\\_%'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(hors, 0);
    let fk: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_constraint c JOIN pg_class t ON t.oid = c.conrelid WHERE t.relnamespace = 'reprise'::regnamespace AND c.contype = 'f'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(fk, 0, "la zone ne référence pas le métier");
    let rls: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_class WHERE relnamespace = 'reprise'::regnamespace AND relkind = 'r' AND relrowsecurity")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rls, 0, "pas de RLS métier en zone");
    // Toutes les tables de zone du catalogue existent, et il n'y en a pas d'autres.
    let en_base: Vec<String> = sqlx::query_scalar("SELECT 'reprise.' || relname::text FROM pg_class WHERE relnamespace = 'reprise'::regnamespace AND relkind = 'r' ORDER BY 1")
        .fetch_all(&pool)
        .await
        .unwrap();
    let mut attendu = ent_reprise::zone::noms_tables_zone();
    attendu.sort();
    assert_eq!(en_base, attendu);
    // Aucun droit pour PUBLIC ; un rôle quelconque (l'application) ne peut ni voir le schéma ni lire.
    let public: i64 = sqlx::query_scalar("SELECT count(*) FROM information_schema.role_table_grants WHERE table_schema = 'reprise' AND grantee = 'PUBLIC'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(public, 0);
    sqlx::query("DO $$ BEGIN IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ent_test_r2_app') THEN BEGIN CREATE ROLE ent_test_r2_app NOLOGIN; EXCEPTION WHEN duplicate_object THEN NULL; END; END IF; END $$")
        .execute(&pool)
        .await
        .unwrap();
    let usage: bool =
        sqlx::query_scalar("SELECT has_schema_privilege('ent_test_r2_app', 'reprise', 'USAGE')")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(!usage);
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE ent_test_r2_app")
        .execute(&mut *tx)
        .await
        .unwrap();
    let e = sqlx::query("SELECT count(*) FROM reprise.next_travel")
        .fetch_one(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(
        e.as_database_error().and_then(|d| d.code()).as_deref(),
        Some("42501"),
        "{e}"
    );
    drop(tx);
    // Le rôle de l'outil, s'il existe (création permise), a l'usage du schéma.
    let outil: Option<bool> = sqlx::query_scalar("SELECT CASE WHEN EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ent_reprise') THEN has_schema_privilege('ent_reprise', 'reprise', 'USAGE') END")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(outil, Some(false));
}

#[tokio::test]
async fn la_base_refuse_un_compte_qui_viole_l_invariant() {
    let Some(pool) = base().await else { return };
    let etab = Uuid::new_v4();
    let exec = reprise::ouvrir_execution(
        &pool,
        etab,
        "next-schema-v1",
        ModeExecution::Simulation,
        CibleExecution::Jetable,
        None,
    )
    .await
    .unwrap();
    let mut tx = TenantTx::begin(&pool, etab).await.unwrap();
    let e = sqlx::query(
        "INSERT INTO rep_execution_compte (execution_id, etablissement_id, objet, lues, creees, mises_a_jour, inchangees, archivees, ignorees, refusees, a_decider)
         VALUES ($1, $2, 'travel', 10, 1, 0, 0, 0, 0, 0, 0)",
    )
    .bind(exec.id)
    .bind(etab)
    .execute(&mut *tx)
    .await
    .unwrap_err();
    assert!(
        e.to_string().contains("rep_execution_compte_invariant_chk"),
        "{e}"
    );
}

#[tokio::test]
async fn simulations_identiques_meme_empreinte_et_numeros_successifs() {
    let Some(pool) = base().await else { return };
    let (_, o) = jeu();
    let etab = Uuid::new_v4();
    let a = executer_simulation(&pool, etab, &o, CibleExecution::Jetable, None)
        .await
        .unwrap();
    let b = executer_simulation(&pool, etab, &o, CibleExecution::Jetable, None)
        .await
        .unwrap();
    assert_eq!((a.execution.numero, b.execution.numero), (1, 2));
    assert_eq!(
        a.rapport_empreinte, b.rapport_empreinte,
        "une simulation rejouée donne le même rapport"
    );
    assert_eq!(a.simulation.total, b.simulation.total);
    // Deux établissements cibles : numérotations indépendantes.
    let c = executer_simulation(&pool, Uuid::new_v4(), &o, CibleExecution::Jetable, None)
        .await
        .unwrap();
    assert_eq!(c.execution.numero, 1);
    // Les identifiants cibles dépendent de l'établissement cible.
    assert_ne!(
        a.simulation.propositions[0].cible_id,
        c.simulation.propositions[0].cible_id
    );
}

#[tokio::test]
async fn correspondance_stable_rejouable_et_jamais_ecrasee() {
    let Some(pool) = base().await else { return };
    let (_, o) = jeu();
    let etab = Uuid::new_v4();
    let sim = executer_simulation(&pool, etab, &o, CibleExecution::Jetable, None)
        .await
        .unwrap();
    // Une simulation ne peut pas enregistrer de correspondance.
    let e = reprise::enregistrer_correspondances(&pool, etab, sim.execution, &sim.simulation)
        .await
        .unwrap_err();
    assert!(e.to_string().contains("simulation"), "{e}");
    // Répétition : enregistre les correspondances (aucun objet métier créé).
    let avant = instantane_metier(&pool).await;
    let rep = reprise::ouvrir_execution(
        &pool,
        etab,
        "next-schema-v1",
        ModeExecution::Repetition,
        CibleExecution::Jetable,
        None,
    )
    .await
    .unwrap();
    let n = reprise::enregistrer_correspondances(&pool, etab, rep, &sim.simulation)
        .await
        .unwrap();
    assert_eq!(
        n as usize,
        sim.simulation.propositions.len() + sim.simulation.a_decider.len()
    );
    assert_eq!(avant, instantane_metier(&pool).await);
    // Rejouer : aucun changement.
    assert_eq!(
        reprise::enregistrer_correspondances(&pool, etab, rep, &sim.simulation)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        compter(&pool, etab, "rep_correspondance").await as usize,
        sim.simulation.propositions.len() + sim.simulation.a_decider.len()
    );
    // Identifiants stables : ceux de la simulation, relus.
    let ex = reprise::correspondances(&pool, etab).await.unwrap();
    let p = &sim.simulation.propositions[0];
    assert_eq!(
        ex[&(p.objet.clone(), p.source_id.clone())].cible_id,
        p.cible_id
    );
    assert_eq!(
        ex[&(p.objet.clone(), p.source_id.clone())].statut,
        Statut::Creee
    );
    // Nouvelle simulation : tout ce qui avait été créé est inchangé ; le « à décider » reste à décider.
    let s2 = executer_simulation(&pool, etab, &o, CibleExecution::Jetable, None)
        .await
        .unwrap();
    assert_eq!(s2.simulation.total.creees, 0);
    assert_eq!(s2.simulation.total.mises_a_jour, 0);
    assert_eq!(s2.simulation.total.inchangees, sim.simulation.total.creees);
    assert_eq!(
        s2.simulation.total.a_decider,
        sim.simulation.total.a_decider
    );
    // Un objet « modifié depuis la reprise » n'est jamais écrasé : à décider, ligne intacte.
    let (objet, sid) = (p.objet.clone(), p.source_id.clone());
    let mut tx = TenantTx::begin(&pool, etab).await.unwrap();
    sqlx::query(
        "UPDATE rep_correspondance SET modifie_depuis_reprise = true, empreinte_source = repeat('0', 64)
          WHERE etablissement_id = $1 AND source_objet = $2 AND source_id = $3",
    )
    .bind(etab)
    .bind(&objet)
    .bind(&sid)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let s3 = executer_simulation(&pool, etab, &o, CibleExecution::Jetable, None)
        .await
        .unwrap();
    assert!(s3
        .simulation
        .a_decider
        .iter()
        .any(|a| a.motif == "modifie_depuis_reprise" && a.objet == objet && a.source_id == sid));
    assert_eq!(s3.simulation.total.mises_a_jour, 0);
    reprise::enregistrer_correspondances(&pool, etab, rep, &s3.simulation)
        .await
        .unwrap();
    let ex = reprise::correspondances(&pool, etab).await.unwrap();
    assert_eq!(
        ex[&(objet, sid)].empreinte_source.as_deref(),
        Some("0".repeat(64).as_str())
    );
    // Autre établissement : aucune correspondance visible.
    assert!(reprise::correspondances(&pool, Uuid::new_v4())
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn les_refus_tracés_ne_contiennent_aucune_donnee() {
    let Some(pool) = base().await else { return };
    let (_, o) = jeu();
    let etab = Uuid::new_v4();
    let r = executer_simulation(&pool, etab, &o, CibleExecution::Jetable, None)
        .await
        .unwrap();
    assert!(!r.decodage.refus.is_empty());
    let mut tx = TenantTx::begin(&pool, etab).await.unwrap();
    let messages: Vec<String> = sqlx::query_scalar("SELECT message || ' ' || COALESCE(source_id, '') || ' ' || regle_code FROM rep_refus WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    let tout = messages.join("\n");
    for s in jeu_fictif::SENTINELLES {
        assert!(!tout.contains(s), "{s} dans rep_refus");
    }
    assert!(tout.contains("sante_hds"));
}

#[tokio::test]
async fn export_en_ecart_de_schema_laisse_une_trace_sans_rien_charger_de_faux() {
    let Some(pool) = base().await else { return };
    let (d, o) = jeu();
    let src = std::fs::read_to_string(d.join("dump.sql")).unwrap();
    std::fs::write(
        d.join("dump.sql"),
        src.replacen("COPY public.tarif (id,", "COPY public.tarif (id_x,", 1),
    )
    .unwrap();
    let etab = Uuid::new_v4();
    let r = executer_simulation(&pool, etab, &o, CibleExecution::Jetable, None)
        .await
        .unwrap();
    assert!(!r.decodage.ecarts_schema.is_empty());
    assert_eq!(
        r.simulation.par_objet["tarif"].refusees,
        r.simulation.par_objet["tarif"].lues
    );
    assert!(!r.zone.contains_key("reprise.next_tarif"));
}
