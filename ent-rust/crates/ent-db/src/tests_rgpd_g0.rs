//! Socle 27, lot G0 : traces sensibles bloquantes (RG-D01, R-RG8) et journal sans `TRUNCATE`
//! (RG-D02). Base exigée sous `CI` ou `ENT_EXIGER_BASE` (RG-D03).

use super::*;
use chrono::NaiveDate;
use domain_eleve::CreateEleve;
use sqlx::Row;
use uuid::Uuid;

async fn base() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    crate::url_guard::assert_local_postgres_url(&url).expect("base locale");
    let ent = PgEnt::connect(&url).await.expect("connexion");
    ent.migrate().await.expect("migrations");
    Some(ent)
}

async fn compter(ent: &PgEnt, etab: Uuid, action: &str) -> i64 {
    sqlx::query(
        "SELECT count(*) AS n FROM ent_journal_modif WHERE etablissement_id = $1 AND action = $2",
    )
    .bind(etab)
    .bind(action)
    .fetch_one(ent.pool())
    .await
    .expect("compte")
    .get::<i64, _>("n")
}

/// Pose un déclencheur qui refuse toute écriture du journal pour (établissement, action) donnés.
async fn refuser_journal(ent: &PgEnt, etab: Uuid, action: &str) -> String {
    let nom = format!("g0_refus_{}", Uuid::new_v4().simple());
    let fonction = format!("{nom}_fn");
    sqlx::query(&format!(
        "CREATE FUNCTION {fonction}() RETURNS trigger AS $$ BEGIN RAISE EXCEPTION 'trace refusée (test G0)'; END; $$ LANGUAGE plpgsql"
    ))
    .execute(ent.pool())
    .await
    .expect("fonction");
    sqlx::query(&format!(
        "CREATE TRIGGER {nom} BEFORE INSERT ON ent_journal_modif FOR EACH ROW \
         WHEN (NEW.etablissement_id = '{etab}'::uuid AND NEW.action = '{action}') EXECUTE FUNCTION {fonction}()"
    ))
    .execute(ent.pool())
    .await
    .expect("déclencheur");
    nom
}

async fn lever_refus(ent: &PgEnt, nom: &str) {
    sqlx::query(&format!("DROP TRIGGER {nom} ON ent_journal_modif"))
        .execute(ent.pool())
        .await
        .expect("drop trigger");
    sqlx::query(&format!("DROP FUNCTION {nom}_fn()"))
        .execute(ent.pool())
        .await
        .expect("drop fn");
}

#[tokio::test]
async fn truncate_du_journal_est_refuse() {
    let Some(ent) = base().await else { return };
    let r = sqlx::query("TRUNCATE ent_journal_modif")
        .execute(ent.pool())
        .await;
    let e = r.expect_err("TRUNCATE doit être refusé (RG-D02)");
    let msg = e.to_string();
    assert!(msg.contains("ajout seul"), "message inattendu : {msg}");
    // UPDATE et DELETE restent refusés (091).
    let r = sqlx::query("DELETE FROM ent_journal_modif WHERE false")
        .execute(ent.pool())
        .await;
    assert!(
        r.is_ok(),
        "un DELETE qui ne touche aucune ligne ne déclenche rien"
    );
}

#[tokio::test]
async fn une_trace_sensible_qui_echoue_est_une_erreur() {
    let Some(ent) = base().await else { return };
    let etab = Uuid::new_v4();
    ent.activer_module_sante_test(etab).await;
    // La trace référence un compte réel (clé étrangère) : on en crée un.
    let user = ent
        .register_user(
            &format!("dir-{}@ent.local", Uuid::new_v4()),
            "DevLocalPass1!",
            None,
            etab,
            "direction",
        )
        .await
        .expect("compte")
        .id;
    // Cas nominal : trace écrite.
    ent.tracer_acces_sensible(
        etab,
        user,
        "direction",
        None,
        "consultation",
        "g0_nominal",
        "test",
    )
    .await
    .expect("trace");
    assert_eq!(compter(&ent, etab, "g0_nominal").await, 1);
    // Échec : l'erreur remonte, rien n'est écrit.
    let nom = refuser_journal(&ent, etab, "g0_refusee").await;
    let r = ent
        .tracer_acces_sensible(
            etab,
            user,
            "direction",
            None,
            "consultation",
            "g0_refusee",
            "test",
        )
        .await;
    lever_refus(&ent, &nom).await;
    assert!(
        r.is_err(),
        "une trace qui échoue doit renvoyer une erreur (RG-D01)"
    );
    assert_eq!(compter(&ent, etab, "g0_refusee").await, 0);
}

#[tokio::test]
async fn lecture_pai_non_servie_si_la_trace_echoue() {
    let Some(ent) = base().await else { return };
    let etab = Uuid::new_v4();
    ent.activer_module_sante_test(etab).await;
    let eleve = ent
        .create_eleve(CreateEleve {
            etablissement_id: etab,
            nom: "Trace".into(),
            prenom: "G0".into(),
            ine: None,
            sexe: None,
            date_naissance: None,
            numero_badge: None,
            redoublant: false,
            classe: Some("6A".into()),
            regime: None,
            mef: None,
            site: None,
            date_entree: Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
            date_sortie: None,
            motif_sortie: None,
            actor_user_id: None,
        })
        .await
        .expect("élève");
    let user = ent
        .register_user(
            &format!("inf-{}@ent.local", Uuid::new_v4()),
            "DevLocalPass1!",
            None,
            etab,
            "infirmerie",
        )
        .await
        .expect("user");
    // Trace en échec : la lecture sensible n'est pas servie.
    let nom = refuser_journal(&ent, etab, "pai").await;
    let refus = ent
        .get_pai_for_actor(etab, user.id, "infirmerie", eleve.id)
        .await;
    let refus_fiche = {
        // La fiche de santé suit la même règle (action « fiche_sante »).
        lever_refus(&ent, &nom).await;
        let nom2 = refuser_journal(&ent, etab, "fiche_sante").await;
        let r = ent
            .get_fiche_sante_for_actor(etab, user.id, "infirmerie", eleve.id)
            .await;
        lever_refus(&ent, &nom2).await;
        r
    };
    assert!(
        refus.is_err(),
        "lecture du PAI servie malgré l'échec de la trace"
    );
    assert!(
        refus_fiche.is_err(),
        "lecture de la fiche de santé servie malgré l'échec de la trace"
    );
    assert_eq!(compter(&ent, etab, "pai").await, 0);
    // Une fois la panne levée, la lecture réussit et laisse sa trace.
    ent.get_pai_for_actor(etab, user.id, "infirmerie", eleve.id)
        .await
        .expect("pai");
    assert_eq!(compter(&ent, etab, "pai").await, 1);
}
