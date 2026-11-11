//! D-036 — les 13 colonnes JSON reprises dans des tables typées (lots 1 et 2) ont été **retirées**
//! (migrations 668 et 669), et les fichiers de reprise devenus caducs sont **figés** (migration 670) :
//! rejouer les migrations à chaque démarrage ne doit ni échouer ni ressusciter une colonne retirée.
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;

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
