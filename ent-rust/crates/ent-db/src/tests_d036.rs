//! D-036 — les 13 colonnes JSON reprises dans des tables typées (lots 1 et 2) ont été **retirées**
//! (migrations 668 et 669), et le lanceur tient un **registre des migrations appliquées**
//! (`ent_migration_appliquee` : nom, empreinte SHA-256, date) : chaque fichier est appliqué une seule
//! fois, dans une transaction, et un fichier modifié après application est refusé.
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use uuid::Uuid;

/// Les 13 colonnes retirées : (table, colonne).
const COLONNES_RETIREES: &[(&str, &str)] = &[
    ("ent_delegation", "droits"),
    ("ent_inscription_dossier", "resolutions"),
    ("ent_inscription_dossier", "scolarite_actuelle"),
    ("ent_signalement_assiduite", "dossier"),
    ("ent_etablissement_profil", "jours_ouvres"),
    ("ent_parametre", "valeur"),
    ("ent_import_ligne", "donnees"),
    ("ent_personne_preference", "joignable"),
    ("ent_personne_preference_version", "joignable"),
    ("ent_eleve_fusion", "conflits_resolus"),
    ("ent_eleve_fusion", "doublons_touches"),
    ("ent_eleve_fusion_ligne", "cle"),
    ("ent_personne_fusion", "instantane"),
];

/// Fonctions de lecture seule qui accompagnaient ces colonnes.
const FONCTIONS_RETIREES: &[&str] = &[
    "ent_delegation_droits_lecture_seule",
    "ent_signalement_dossier_lecture_seule",
    "ent_profil_jours_ouvres_lecture_seule",
    "ent_parametre_valeur_lecture_seule",
    "ent_import_ligne_donnees_lecture_seule",
    "ent_personne_preference_joignable_lecture_seule",
    "ent_eleve_fusion_json_lecture_seule",
    "ent_personne_fusion_instantane_lecture_seule",
];

async fn base() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

async fn colonnes_presentes(ent: &PgEnt) -> Vec<String> {
    let mut presentes = Vec::new();
    for (table, colonne) in COLONNES_RETIREES {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM information_schema.columns
              WHERE table_schema = 'public' AND table_name = $1 AND column_name = $2",
        )
        .bind(table)
        .bind(colonne)
        .fetch_one(ent.pool())
        .await
        .unwrap();
        if n > 0 {
            presentes.push(format!("{table}.{colonne}"));
        }
    }
    presentes
}

#[tokio::test]
async fn les_treize_colonnes_json_sont_retirees_et_leurs_declencheurs_aussi() {
    let Some(ent) = base().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    assert_eq!(colonnes_presentes(&ent).await, Vec::<String>::new(), "colonnes D-036 encore présentes");
    for f in FONCTIONS_RETIREES {
        let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM pg_proc WHERE proname = $1")
            .bind(f)
            .fetch_one(ent.pool())
            .await
            .unwrap();
        assert_eq!(n, 0, "fonction {f} encore présente");
    }
}

/// Garde de dérive : la copie typée d'une ligne supprimée par une fusion de personnes a exactement
/// les colonnes de la table d'origine (sinon la défusion ne restaurerait plus à l'identique).
#[tokio::test]
async fn copies_de_fusion_de_personnes_alignees_sur_les_tables_d_origine() {
    let Some(ent) = base().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    for (copie, origine) in [
        ("ent_personne_fusion_lien_supprime", "ent_eleve_lien"),
        ("ent_personne_fusion_contact_supprime", "ent_eleve_contact_urgence"),
    ] {
        let attendu: Vec<(String, String)> = sqlx::query_as(
            "SELECT column_name, data_type FROM information_schema.columns
              WHERE table_schema = 'public' AND table_name = $1 AND column_name <> 'etablissement_id' ORDER BY column_name",
        )
        .bind(origine)
        .fetch_all(ent.pool())
        .await
        .unwrap();
        let copie_cols: Vec<(String, String)> = sqlx::query_as(
            "SELECT column_name, data_type FROM information_schema.columns
              WHERE table_schema = 'public' AND table_name = $1
                AND column_name NOT IN ('etablissement_id', 'fusion_id', 'cree_le') ORDER BY column_name",
        )
        .bind(copie)
        .fetch_all(ent.pool())
        .await
        .unwrap();
        assert_eq!(
            copie_cols, attendu,
            "{copie} diverge de {origine} : ajouter une migration (≥ 671) et adapter le code de (dé)fusion"
        );
    }
}

// ───────────────────────── Registre des migrations appliquées ─────────────────────────

use crate::migrate::{appliquer_dans_une_transaction, appliquer_jusqu_a, empreinte, MIGRATIONS};

/// Crée une base jetable (même serveur local que `ENT_DATABASE_URL`) et rend son URL.
async fn base_jetable(etiquette: &str) -> Option<(String, String, sqlx::PgPool)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let nom = format!("d036_{etiquette}_{}", &Uuid::new_v4().simple().to_string()[..8]);
    let admin = sqlx::PgPool::connect(&url).await.expect("connexion admin");
    sqlx::query(&format!("CREATE DATABASE {nom}")).execute(&admin).await.expect("CREATE DATABASE");
    let mut u = url::Url::parse(&url).unwrap();
    u.set_path(&format!("/{nom}"));
    Some((u.to_string(), nom, admin))
}

async fn supprimer_base(admin: &sqlx::PgPool, nom: &str) {
    let _ = sqlx::query(&format!("DROP DATABASE IF EXISTS {nom} WITH (FORCE)")).execute(admin).await;
}

async fn sans_colonnes_retirees(pool: &sqlx::PgPool) -> bool {
    for (table, colonne) in COLONNES_RETIREES {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM information_schema.columns WHERE table_schema = 'public' AND table_name = $1 AND column_name = $2",
        )
        .bind(table)
        .bind(colonne)
        .fetch_one(pool)
        .await
        .unwrap();
        if n > 0 {
            return false;
        }
    }
    true
}

async fn compte_registre(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ent_migration_appliquee").fetch_one(pool).await.unwrap()
}

/// Cas 1 et 2 : base NEUVE (tout est appliqué dans l'ordre et inscrit), puis DEUX démarrages de suite
/// (rien n'est rejoué : le registre ne change pas, aucune colonne retirée ne revient).
#[tokio::test]
async fn base_neuve_puis_deux_demarrages_de_suite() {
    let Some((url, nom, admin)) = base_jetable("neuve").await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("premier démarrage");
    assert_eq!(compte_registre(ent.pool()).await, MIGRATIONS.len() as i64, "chaque fichier est inscrit");
    let avant: Vec<(String, String, chrono::DateTime<chrono::Utc>)> =
        sqlx::query_as("SELECT nom, empreinte, appliquee_le FROM ent_migration_appliquee ORDER BY nom")
            .fetch_all(ent.pool()).await.unwrap();
    for tour in 2..=3 {
        ent.migrate().await.unwrap_or_else(|e| panic!("démarrage {tour} : {e}"));
        assert!(sans_colonnes_retirees(ent.pool()).await, "démarrage {tour} : colonne retirée présente");
    }
    let apres: Vec<(String, String, chrono::DateTime<chrono::Utc>)> =
        sqlx::query_as("SELECT nom, empreinte, appliquee_le FROM ent_migration_appliquee ORDER BY nom")
            .fetch_all(ent.pool()).await.unwrap();
    assert_eq!(avant, apres, "un démarrage de plus n'applique ni ne ré-inscrit rien");
    assert!(avant.iter().all(|(n, e, _)| MIGRATIONS.iter().any(|(m, sql)| m == n && empreinte(sql) == *e)));
    drop(ent);
    supprimer_base(&admin, &nom).await;
}

/// Cas 3 : base EXISTANTE, antérieure au registre (migrée jusqu'à 667, anciennes colonnes présentes avec
/// des lignes « héritées », aucun registre) : le premier démarrage applique tout une dernière fois dans
/// l'ordre (la reprise a lieu avant le retrait des colonnes) et remplit le registre.
#[tokio::test]
async fn base_existante_sans_registre_migree_jusqu_a_667() {
    let Some((url, nom, admin)) = base_jetable("existante").await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    appliquer_jusqu_a(ent.pool(), "667_d036_personne_fusion_normalisee").await.expect("migrations jusqu'à 667");
    assert!(!sans_colonnes_retirees(ent.pool()).await, "à 667 les anciennes colonnes existent encore");
    let (reg,): (bool,) = sqlx::query_as("SELECT to_regclass('public.ent_migration_appliquee') IS NOT NULL")
        .fetch_one(ent.pool()).await.unwrap();
    assert!(!reg, "aucun registre avant le premier démarrage du nouveau lanceur");
    let etab = Uuid::new_v4();
    for (table, decl) in [
        ("ent_parametre", "ent_parametre_valeur_lecture_seule"),
        ("ent_etablissement_profil", "ent_profil_jours_ouvres_lecture_seule"),
    ] {
        sqlx::query(&format!("ALTER TABLE {table} DISABLE TRIGGER {decl}")).execute(ent.pool()).await.unwrap();
    }
    sqlx::query("INSERT INTO ent_parametre (etablissement_id, cle, valeur, socle_proprietaire) VALUES ($1, 'structure.duree_seance_defaut_min', '45'::jsonb, 5)")
        .bind(etab).execute(ent.pool()).await.unwrap();
    sqlx::query("INSERT INTO ent_etablissement_profil (etablissement_id, nom_officiel, jours_ouvres) VALUES ($1, 'Collège mise à niveau', '{\"lun\":[\"matin\"],\"mar\":[\"matin\",\"apres_midi\"]}'::jsonb)")
        .bind(etab).execute(ent.pool()).await.unwrap();

    ent.migrate().await.expect("premier démarrage du nouveau lanceur");
    assert!(sans_colonnes_retirees(ent.pool()).await, "les 13 colonnes ont disparu");
    assert_eq!(compte_registre(ent.pool()).await, MIGRATIONS.len() as i64);
    let v: serde_json::Value = sqlx::query_scalar(
        "SELECT ent_parametre_valeur_json(p) FROM ent_parametre p WHERE etablissement_id = $1 AND cle = 'structure.duree_seance_defaut_min'",
    ).bind(etab).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(v, serde_json::json!(45), "la valeur a été reprise avant le retrait");
    let semaine: serde_json::Value = sqlx::query_scalar("SELECT ent_etablissement_jours_ouvres($1)")
        .bind(etab).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(semaine, serde_json::json!({"lun": ["matin"], "mar": ["matin", "apres_midi"]}));
    ent.migrate().await.expect("démarrage suivant");
    assert!(sans_colonnes_retirees(ent.pool()).await);
    drop(ent);
    supprimer_base(&admin, &nom).await;
}

/// Un fichier de migration modifié après son application est refusé, avant toute écriture.
#[tokio::test]
async fn un_fichier_modifie_apres_application_est_refuse() {
    let Some((url, nom, admin)) = base_jetable("modifie").await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    // Simule « le fichier 560 a changé depuis son application » : l'empreinte enregistrée n'est plus la sienne.
    sqlx::query("UPDATE ent_migration_appliquee SET empreinte = repeat('0', 64) WHERE nom = '560_delegation_droits_normalises'")
        .execute(ent.pool()).await.unwrap();
    let err = ent.migrate().await.expect_err("fichier modifié refusé");
    let msg = err.to_string();
    assert!(msg.contains("560_delegation_droits_normalises") && msg.contains("modifiée"), "{msg}");
    // Rétabli : le démarrage redevient sain.
    let bonne = empreinte(MIGRATIONS.iter().find(|(n, _)| *n == "560_delegation_droits_normalises").unwrap().1);
    sqlx::query("UPDATE ent_migration_appliquee SET empreinte = $1 WHERE nom = '560_delegation_droits_normalises'")
        .bind(bonne).execute(ent.pool()).await.unwrap();
    ent.migrate().await.expect("redevient sain");
    drop(ent);
    supprimer_base(&admin, &nom).await;
}

/// Une transaction par fichier : un ordre qui échoue ne laisse ni demi-fichier ni inscription.
#[tokio::test]
async fn un_fichier_en_echec_ne_laisse_aucune_trace() {
    let Some((url, nom, admin)) = base_jetable("echec").await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let avant = compte_registre(ent.pool()).await;
    let r = appliquer_dans_une_transaction(ent.pool(), "999_echec_de_test", "CREATE TABLE ent_d036_trace (a INTEGER);\nSELECT 1 / 0;").await;
    assert!(r.is_err(), "l'ordre en échec remonte");
    let (table,): (bool,) = sqlx::query_as("SELECT to_regclass('public.ent_d036_trace') IS NOT NULL").fetch_one(ent.pool()).await.unwrap();
    assert!(!table, "le premier ordre a été annulé avec le fichier");
    assert_eq!(compte_registre(ent.pool()).await, avant, "aucune inscription");
    drop(ent);
    supprimer_base(&admin, &nom).await;
}
