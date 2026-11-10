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
