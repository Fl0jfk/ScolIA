//! D-036 — reprise des colonnes JSON vers des tables typées : la migration rejouée à chaque
//! démarrage est idempotente, et l'ancienne colonne est en lecture seule.
//! Sans `ENT_DATABASE_URL`, le scénario est ignoré.

use super::*;
use uuid::Uuid;

#[tokio::test]
async fn delegation_droits_repris_une_fois_et_ancienne_colonne_en_lecture_seule() {
    let Some(url) = crate::url_guard::url_labo_ou_echec() else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    let etab = cast.etablissement_id;
    let (delegant, delegue) = (cast.direction.user_id, cast.profs[0].user_id);
    let (id, cible) = (Uuid::new_v4(), Uuid::new_v4());

    // Ligne « héritée » : droits au format JSON d'avant la migration 560 (déclencheur suspendu).
    let mut tx = ent.pool().begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_delegation DISABLE TRIGGER ent_delegation_droits_lecture_seule")
        .execute(&mut *tx).await.unwrap();
    sqlx::query(
        "INSERT INTO ent_delegation (id, etablissement_id, delegue_user_id, delegant_user_id, droits, debut, fin)
         VALUES ($1,$2,$3,$4,$5,current_date,current_date + 5)",
    )
    .bind(id).bind(etab).bind(delegue).bind(delegant)
    .bind(serde_json::json!([
        {"action": "edt.gerer", "portee": "designes", "cibles": [cible]},
        {"action": "cahier.lire", "portee": "etablissement", "cibles": []},
        {"action": "", "portee": "etablissement"}
    ]))
    .execute(&mut *tx).await.unwrap();
    sqlx::query("ALTER TABLE ent_delegation ENABLE TRIGGER ent_delegation_droits_lecture_seule")
        .execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();

    // Deux rejeux de la migration : même résultat (idempotence).
    for _ in 0..2 {
        ent.migrate().await.expect("migrate rejoué");
        let droits: Vec<(i16, String, String)> = sqlx::query_as(
            "SELECT ordre, action, portee FROM ent_delegation_droit WHERE delegation_id = $1 ORDER BY ordre",
        )
        .bind(id).fetch_all(ent.pool()).await.unwrap();
        assert_eq!(
            droits,
            vec![
                (0, "edt.gerer".to_string(), "designes".to_string()),
                (1, "cahier.lire".to_string(), "etablissement".to_string())
            ],
            "le droit sans action n'est pas repris"
        );
        let cibles: Vec<(i16, Uuid)> = sqlx::query_as(
            "SELECT ordre, cible_id FROM ent_delegation_droit_cible WHERE delegation_id = $1",
        )
        .bind(id).fetch_all(ent.pool()).await.unwrap();
        assert_eq!(cibles, vec![(0, cible)]);
    }

    // L'ancienne colonne est en lecture seule : seule la valeur `[]` est acceptée.
    let err = sqlx::query("UPDATE ent_delegation SET droits = '[{\"action\":\"x\"}]'::jsonb WHERE id = $1")
        .bind(id).execute(ent.pool()).await.expect_err("écriture refusée");
    assert_eq!(err.as_database_error().and_then(|d| d.code()).as_deref(), Some("42501"));
    sqlx::query("DELETE FROM ent_delegation WHERE id = $1").bind(id).execute(ent.pool()).await.unwrap();
}

// ───────────────────────── Lot 2 : migrations 661–667 ─────────────────────────

async fn base_labo() -> Option<(PgEnt, Uuid)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast.etablissement_id))
}

/// Exécute `sql` avec les déclencheurs de lecture seule de `table` suspendus (lignes « héritées »).
async fn avec_legacy(ent: &PgEnt, table: &str, declencheurs: &[&str], sql: &str, binds: &[Uuid]) {
    let mut tx = ent.pool().begin().await.unwrap();
    for d in declencheurs {
        sqlx::query(&format!("ALTER TABLE {table} DISABLE TRIGGER {d}")).execute(&mut *tx).await.unwrap();
    }
    let mut q = sqlx::query(sql);
    for b in binds {
        q = q.bind(*b);
    }
    q.execute(&mut *tx).await.unwrap();
    for d in declencheurs {
        sqlx::query(&format!("ALTER TABLE {table} ENABLE TRIGGER {d}")).execute(&mut *tx).await.unwrap();
    }
    tx.commit().await.unwrap();
}

fn code_sqlstate(e: &sqlx::Error) -> Option<String> {
    e.as_database_error().and_then(|d| d.code()).map(|c| c.to_string())
}

#[tokio::test]
async fn profil_jours_ouvres_repris_et_ancienne_colonne_en_lecture_seule() {
    let Some((ent, _)) = base_labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = Uuid::new_v4();
    avec_legacy(
        &ent,
        "ent_etablissement_profil",
        &["ent_profil_jours_ouvres_lecture_seule"],
        "INSERT INTO ent_etablissement_profil (etablissement_id, nom_officiel, jours_ouvres)
         VALUES ($1, 'Collège D036', '{\"lun\":[\"matin\"],\"sam\":[\"matin\",\"apres_midi\"],\"xxx\":[\"matin\"]}'::jsonb)",
        &[etab],
    )
    .await;
    for _ in 0..2 {
        ent.migrate().await.expect("migrate rejoué");
        let v: serde_json::Value = sqlx::query_scalar("SELECT ent_etablissement_jours_ouvres($1)")
            .bind(etab).fetch_one(ent.pool()).await.unwrap();
        assert_eq!(v, serde_json::json!({"lun": ["matin"], "sam": ["matin", "apres_midi"]}));
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_etablissement_jour_ouvre WHERE etablissement_id = $1")
            .bind(etab).fetch_one(ent.pool()).await.unwrap();
        assert_eq!(n, 2, "le jour inconnu n'est pas repris");
    }
    let err = sqlx::query("UPDATE ent_etablissement_profil SET jours_ouvres = '{\"lun\":[\"matin\"]}'::jsonb WHERE etablissement_id = $1")
        .bind(etab).execute(ent.pool()).await.expect_err("écriture refusée");
    assert_eq!(code_sqlstate(&err).as_deref(), Some("42501"));
    sqlx::query("DELETE FROM ent_etablissement_profil WHERE etablissement_id = $1").bind(etab).execute(ent.pool()).await.unwrap();
}

#[tokio::test]
async fn parametre_repris_en_colonnes_typees() {
    let Some((ent, etab)) = base_labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    for (cle, v) in [
        ("d036.entier", "45"),
        ("d036.booleen", "true"),
        ("d036.heure", "\"08:15\""),
        ("d036.invalide", "\"abc\""),
    ] {
        sqlx::query("ALTER TABLE ent_parametre DISABLE TRIGGER ent_parametre_valeur_lecture_seule").execute(ent.pool()).await.unwrap();
        sqlx::query("INSERT INTO ent_parametre (etablissement_id, cle, valeur, socle_proprietaire) VALUES ($1, $2, $3::jsonb, 5)")
            .bind(etab).bind(cle).bind(v).execute(ent.pool()).await.unwrap();
        sqlx::query("ALTER TABLE ent_parametre ENABLE TRIGGER ent_parametre_valeur_lecture_seule").execute(ent.pool()).await.unwrap();
    }
    for _ in 0..2 {
        ent.migrate().await.expect("migrate rejoué");
        let lignes: Vec<(String, Option<serde_json::Value>)> = sqlx::query_as(
            "SELECT cle, ent_parametre_valeur_json(p) FROM ent_parametre p WHERE etablissement_id = $1 AND cle LIKE 'd036.%' ORDER BY cle",
        ).bind(etab).fetch_all(ent.pool()).await.unwrap();
        assert_eq!(
            lignes,
            vec![
                ("d036.booleen".to_string(), Some(serde_json::json!(true))),
                ("d036.entier".to_string(), Some(serde_json::json!(45))),
                ("d036.heure".to_string(), Some(serde_json::json!("08:15"))),
                ("d036.invalide".to_string(), None),
            ]
        );
    }
    let err = sqlx::query("UPDATE ent_parametre SET valeur = '7'::jsonb WHERE etablissement_id = $1 AND cle = 'd036.entier'")
        .bind(etab).execute(ent.pool()).await.expect_err("écriture refusée");
    assert_eq!(code_sqlstate(&err).as_deref(), Some("42501"));
    sqlx::query("DELETE FROM ent_parametre WHERE etablissement_id = $1 AND cle LIKE 'd036.%'").bind(etab).execute(ent.pool()).await.unwrap();
}

#[tokio::test]
async fn import_ligne_reprise_en_colonnes_typees() {
    let Some((ent, etab)) = base_labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (lot, ligne) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO ent_import_lot (id, etablissement_id, format, nom_fichier) VALUES ($1,$2,'scolia_eleves_v1','x.csv')")
        .bind(lot).bind(etab).execute(ent.pool()).await.unwrap();
    avec_legacy(
        &ent,
        "ent_import_ligne",
        &["ent_import_ligne_donnees_lecture_seule"],
        "INSERT INTO ent_import_ligne (id, etablissement_id, lot_id, numero, donnees, statut)
         VALUES ($1,$2,$3,1,'{\"nom\":\"Weasley\",\"prenom\":\"Ron\",\"resp1_email\":\"a@b.fr\",\"inconnue\":\"x\"}'::jsonb,'rejet')",
        &[ligne, etab, lot],
    )
    .await;
    for _ in 0..2 {
        ent.migrate().await.expect("migrate rejoué");
        let v: serde_json::Value = sqlx::query_scalar("SELECT ent_import_ligne_donnees(l) FROM ent_import_ligne l WHERE id = $1")
            .bind(ligne).fetch_one(ent.pool()).await.unwrap();
        assert_eq!(v, serde_json::json!({"nom": "Weasley", "prenom": "Ron", "resp1_email": "a@b.fr"}));
    }
    let err = sqlx::query("UPDATE ent_import_ligne SET donnees = '{}'::jsonb WHERE id = $1")
        .bind(ligne).execute(ent.pool()).await.expect_err("écriture refusée");
    assert_eq!(code_sqlstate(&err).as_deref(), Some("42501"));
    sqlx::query("DELETE FROM ent_import_lot WHERE id = $1").bind(lot).execute(ent.pool()).await.unwrap();
}

#[tokio::test]
async fn joignable_repris_en_plages_typees() {
    let Some((ent, etab)) = base_labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let (pers, vers) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom) VALUES ($1,$2,'D036 Joignable')")
        .bind(pers).bind(etab).execute(ent.pool()).await.unwrap();
    avec_legacy(
        &ent,
        "ent_personne_preference",
        &["ent_personne_preference_joignable_lecture_seule"],
        "INSERT INTO ent_personne_preference (personne_id, etablissement_id, joignable)
         VALUES ($1,$2,'[{\"jours\":[1,2,2],\"de\":\"08:00\",\"a\":\"12:00\"},{\"jours\":[9],\"de\":\"08:00\",\"a\":\"12:00\"},{\"jours\":[3],\"de\":\"14:00\",\"a\":\"13:00\"}]'::jsonb)",
        &[pers, etab],
    )
    .await;
    avec_legacy(
        &ent,
        "ent_personne_preference_version",
        &["ent_personne_preference_version_joignable_lecture_seule"],
        "INSERT INTO ent_personne_preference_version (id, etablissement_id, personne_id, canal_email, canal_sms, courrier_papier, joignable, via)
         VALUES ($3,$2,$1,true,true,false,'[]'::jsonb,'systeme')",
        &[pers, etab, vers],
    )
    .await;
    for _ in 0..2 {
        ent.migrate().await.expect("migrate rejoué");
        let v: serde_json::Value = sqlx::query_scalar("SELECT ent_personne_joignable_json($1,$2)")
            .bind(etab).bind(pers).fetch_one(ent.pool()).await.unwrap();
        assert_eq!(v, serde_json::json!([{"jours": [1, 2], "de": "08:00", "a": "12:00"}]), "plages invalides ignorées");
        let (definie,): (bool,) = sqlx::query_as("SELECT joignable_definie FROM ent_personne_preference WHERE personne_id = $1")
            .bind(pers).fetch_one(ent.pool()).await.unwrap();
        assert!(definie);
        let (vdef,): (bool,) = sqlx::query_as("SELECT joignable_definie FROM ent_personne_preference_version WHERE id = $1")
            .bind(vers).fetch_one(ent.pool()).await.unwrap();
        assert!(vdef, "liste vide mais renseignée");
    }
    let err = sqlx::query("UPDATE ent_personne_preference SET joignable = '[]'::jsonb WHERE personne_id = $1")
        .bind(pers).execute(ent.pool()).await.expect_err("écriture refusée");
    assert_eq!(code_sqlstate(&err).as_deref(), Some("42501"));
    sqlx::query("DELETE FROM ent_personne_preference_version WHERE id = $1").bind(vers).execute(ent.pool()).await.unwrap();
    sqlx::query("DELETE FROM ent_personne_preference WHERE personne_id = $1").bind(pers).execute(ent.pool()).await.unwrap();
}

#[tokio::test]
async fn fusion_eleves_reprise_conflits_doublons_et_cles() {
    let Some(url) = crate::url_guard::url_labo_ou_echec() else { return eprintln!("skip: ENT_DATABASE_URL") };
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    let (etab, absorbe, conserve) = (cast.etablissement_id, cast.eleve_lea_id, cast.eleve_tom_id);
    let (fusion, doublon) = (Uuid::new_v4(), Uuid::new_v4());
    avec_legacy(
        &ent,
        "ent_eleve_fusion",
        &["ent_eleve_fusion_json_lecture_seule"],
        "INSERT INTO ent_eleve_fusion (id, etablissement_id, absorbe_id, conserve_id, motif_code, conflits_resolus, doublons_touches)
         VALUES ($1,$2,$3,$4,'doublon_saisie',
                 '[{\"table\":\"ent_absence\",\"cle\":{\"id\":\"abc\"},\"domaine\":\"Absences\"}]'::jsonb,
                 jsonb_build_array(jsonb_build_object('id', $5::text, 'statut', 'a_examiner')))",
        &[fusion, etab, absorbe, conserve, doublon],
    )
    .await;
    avec_legacy(
        &ent,
        "ent_eleve_fusion_ligne",
        &["ent_eleve_fusion_ligne_cle_lecture_seule"],
        "INSERT INTO ent_eleve_fusion_ligne (fusion_id, etablissement_id, table_nom, cle, empreinte)
         VALUES ($1,$2,'ent_absence','{\"id\":\"xyz\"}'::jsonb,'e1')",
        &[fusion, etab],
    )
    .await;
    for _ in 0..2 {
        ent.migrate().await.expect("migrate rejoué");
        let c: serde_json::Value = sqlx::query_scalar("SELECT ent_eleve_fusion_conflits_json($1,$2)")
            .bind(etab).bind(fusion).fetch_one(ent.pool()).await.unwrap();
        assert_eq!(c, serde_json::json!([{"table": "ent_absence", "cle": {"id": "abc"}, "domaine": "Absences"}]));
        let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_eleve_fusion_doublon WHERE fusion_id = $1 AND statut_avant = 'a_examiner'")
            .bind(fusion).fetch_one(ent.pool()).await.unwrap();
        assert_eq!(n, 1);
        let (k,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_eleve_fusion_ligne_cle k JOIN ent_eleve_fusion_ligne l ON l.id = k.ligne_id
              WHERE l.fusion_id = $1 AND k.colonne = 'id' AND k.valeur = 'xyz'")
            .bind(fusion).fetch_one(ent.pool()).await.unwrap();
        assert_eq!(k, 1);
    }
    // Les tables de fusion sont en ajout seul.
    let err = sqlx::query("DELETE FROM ent_eleve_fusion_doublon WHERE fusion_id = $1").bind(fusion)
        .execute(ent.pool()).await.expect_err("ajout seul");
    assert_eq!(code_sqlstate(&err).as_deref(), Some("42501"));
}

#[tokio::test]
async fn fusion_personnes_reprise_et_copies_alignees_sur_les_tables_d_origine() {
    let Some((ent, etab)) = base_labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    // Garde de dérive : la copie typée a exactement les colonnes de la table d'origine.
    for (copie, origine) in [
        ("ent_personne_fusion_lien_supprime", "ent_eleve_lien"),
        ("ent_personne_fusion_contact_supprime", "ent_eleve_contact_urgence"),
    ] {
        let attendu: Vec<(String, String)> = sqlx::query_as(
            "SELECT column_name, data_type FROM information_schema.columns
              WHERE table_schema = 'public' AND table_name = $1 AND column_name <> 'etablissement_id' ORDER BY column_name",
        ).bind(origine).fetch_all(ent.pool()).await.unwrap();
        let copie_cols: Vec<(String, String)> = sqlx::query_as(
            "SELECT column_name, data_type FROM information_schema.columns
              WHERE table_schema = 'public' AND table_name = $1
                AND column_name NOT IN ('etablissement_id', 'fusion_id', 'cree_le') ORDER BY column_name",
        ).bind(copie).fetch_all(ent.pool()).await.unwrap();
        // `created_at`/`updated_at` de l'origine sont conservés tels quels dans la copie.
        assert_eq!(copie_cols, attendu, "{copie} diverge de {origine} : mettre à jour la migration 667 et le code de (dé)fusion");
    }
    let (src, cib, fusion) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let lien = Uuid::new_v4();
    for (id, nom) in [(src, "Source D036"), (cib, "Cible D036")] {
        sqlx::query("INSERT INTO ent_personne (id, etablissement_id, nom) VALUES ($1,$2,$3)")
            .bind(id).bind(etab).bind(nom).execute(ent.pool()).await.unwrap();
    }
    avec_legacy(
        &ent,
        "ent_personne_fusion",
        &["ent_personne_fusion_instantane_lecture_seule"],
        "INSERT INTO ent_personne_fusion (id, etablissement_id, source_id, cible_id, motif, instantane)
         VALUES ($1,$2,$3,$4,'doublon de test',
                 jsonb_build_object('compte_transfere', NULL, 'liens_deplaces', jsonb_build_array($5::text),
                                    'liens_supprimes', '[]'::jsonb, 'contacts_deplaces', '[]'::jsonb,
                                    'contacts_supprimes', '[]'::jsonb, 'agent_deplace', NULL, 'agents_fusionnes', true))",
        &[fusion, etab, src, cib, lien],
    )
    .await;
    for _ in 0..2 {
        ent.migrate().await.expect("migrate rejoué");
        let (norm, agents): (bool, bool) = sqlx::query_as("SELECT detail_normalise, agents_fusionnes FROM ent_personne_fusion WHERE id = $1")
            .bind(fusion).fetch_one(ent.pool()).await.unwrap();
        assert!(norm && agents);
        let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_personne_fusion_objet WHERE fusion_id = $1 AND nature = 'lien' AND objet_id = $2")
            .bind(fusion).bind(lien).fetch_one(ent.pool()).await.unwrap();
        assert_eq!(n, 1);
    }
}
