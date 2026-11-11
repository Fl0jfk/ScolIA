//! D-036 — les 13 colonnes JSON reprises dans des tables typées (lots 1 et 2) ont été **retirées**
//! (migrations 668 et 669), et les fichiers de reprise devenus caducs sont **figés** (migration 670) :
//! rejouer les migrations à chaque démarrage ne doit ni échouer ni ressusciter une colonne retirée.
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

#[tokio::test]
async fn rejouer_les_migrations_ne_ressuscite_aucune_colonne_retiree() {
    let Some(mut ent) = base().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    for _ in 0..2 {
        ent.migrate().await.expect("migrate rejoué sans échec (fichiers de reprise figés)");
        assert_eq!(colonnes_presentes(&ent).await, Vec::<String>::new());
    }
    // Les fichiers figés sont exactement ceux qui lisent ou recréent les colonnes retirées.
    let figes: Vec<(String,)> = sqlx::query_as("SELECT nom FROM ent_migration_figee ORDER BY nom")
        .fetch_all(ent.pool())
        .await
        .unwrap();
    assert_eq!(figes.len(), 13, "13 fichiers figés : {figes:?}");
    assert!(figes.iter().any(|(n,)| n == "560_delegation_droits_normalises"));
    assert!(figes.iter().any(|(n,)| n == "667_d036_personne_fusion_normalisee"));
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

// ───────────────────────── Mécanisme des fichiers figés (migration 670) ─────────────────────────

use crate::migrate::{appliquer_jusqu_a, MIGRATIONS};

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

/// Cas 1 et 2 : base NEUVE (tout est joué dans l'ordre, puis 670 fige), puis DEUX démarrages de suite
/// sur la même base (les fichiers figés sont sautés : ni échec, ni colonne ressuscitée).
#[tokio::test]
async fn base_neuve_puis_deux_demarrages_de_suite() {
    let Some((url, nom, admin)) = base_jetable("neuve").await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    for tour in 1..=3 {
        ent.migrate().await.unwrap_or_else(|e| panic!("démarrage {tour} : {e}"));
        assert!(sans_colonnes_retirees(ent.pool()).await, "démarrage {tour} : colonne retirée présente");
    }
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_migration_figee").fetch_one(ent.pool()).await.unwrap();
    assert_eq!(n, 13);
    drop(ent);
    supprimer_base(&admin, &nom).await;
}

/// Cas 3 : base DÉJÀ MIGRÉE jusqu'à 667 (anciennes colonnes présentes, avec des lignes « héritées »),
/// puis passage à 670 : la reprise a lieu une dernière fois, les colonnes disparaissent, les données
/// vivent dans les tables typées.
#[tokio::test]
async fn base_migree_jusqu_a_667_puis_passage_a_670() {
    let Some((url, nom, admin)) = base_jetable("mise_a_niveau").await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    appliquer_jusqu_a(ent.pool(), "667_d036_personne_fusion_normalisee").await.expect("migrations jusqu'à 667");
    assert!(!sans_colonnes_retirees(ent.pool()).await, "à 667 les anciennes colonnes existent encore");
    let etab = Uuid::new_v4();
    // Lignes héritées : paramètre (valeur JSON) et profil (semaine type JSON), déclencheurs suspendus.
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

    ent.migrate().await.expect("passage de 667 à 670");
    assert!(sans_colonnes_retirees(ent.pool()).await, "les 13 colonnes ont disparu");
    let v: serde_json::Value = sqlx::query_scalar(
        "SELECT ent_parametre_valeur_json(p) FROM ent_parametre p WHERE etablissement_id = $1 AND cle = 'structure.duree_seance_defaut_min'",
    ).bind(etab).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(v, serde_json::json!(45), "la valeur a été reprise avant le retrait");
    let semaine: serde_json::Value = sqlx::query_scalar("SELECT ent_etablissement_jours_ouvres($1)")
        .bind(etab).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(semaine, serde_json::json!({"lun": ["matin"], "mar": ["matin", "apres_midi"]}));
    // Et un démarrage de plus (fichiers figés) reste sans effet.
    ent.migrate().await.expect("démarrage suivant");
    assert!(sans_colonnes_retirees(ent.pool()).await);
    drop(ent);
    supprimer_base(&admin, &nom).await;
}

/// Le lanceur refuse un nom inconnu dans `ent_migration_figee` (on ne fige pas à l'aveugle).
#[tokio::test]
async fn le_lanceur_refuse_de_figer_un_fichier_inconnu() {
    let Some((url, nom, admin)) = base_jetable("figer_inconnu").await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    sqlx::query("INSERT INTO ent_migration_figee (nom, raison) VALUES ('999_fichier_inexistant', 'test')")
        .execute(ent.pool()).await.unwrap();
    let err = ent.migrate().await.expect_err("nom inconnu refusé");
    assert!(err.to_string().contains("999_fichier_inexistant"), "{err}");
    sqlx::query("DELETE FROM ent_migration_figee WHERE nom = '999_fichier_inexistant'").execute(ent.pool()).await.unwrap();
    ent.migrate().await.expect("redevient sain une fois la ligne retirée");
    drop(ent);
    supprimer_base(&admin, &nom).await;
}

/// Figer un fichier qui CRÉE une table ne doit jamais en faire perdre une : toute table créée par un
/// fichier figé existe après migration (elle a été créée avant le figeage, sur base neuve comme existante).
#[tokio::test]
async fn les_tables_creees_par_les_fichiers_figes_existent() {
    let Some(ent) = base().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let figes: Vec<(String,)> = sqlx::query_as("SELECT nom FROM ent_migration_figee").fetch_all(ent.pool()).await.unwrap();
    assert!(!figes.is_empty());
    for (nom,) in figes {
        let (_, sql) = MIGRATIONS.iter().find(|(n, _)| *n == nom).expect("fichier connu");
        for ligne in sql.lines() {
            let l = ligne.trim_start().to_lowercase();
            if let Some(reste) = l.strip_prefix("create table if not exists ") {
                let table = reste.split(|c: char| c.is_whitespace() || c == '(').next().unwrap();
                let (existe,): (bool,) = sqlx::query_as("SELECT to_regclass($1) IS NOT NULL")
                    .bind(format!("public.{table}")).fetch_one(ent.pool()).await.unwrap();
                assert!(existe, "{nom} crée {table}, absente du schéma");
            }
        }
    }
}
