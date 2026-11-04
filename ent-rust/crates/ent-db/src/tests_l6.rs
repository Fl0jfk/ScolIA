//! Lot L6 — erreurs d'infrastructure, traçabilité des accès sensibles, journal
//! en ajout seul. Les tests base sont ignorés (skip) sans `ENT_DATABASE_URL`.

use super::*;
use chrono::NaiveDate;
use domain_eleve::{CreateEleve, DomainError};
use sqlx::Row;
use uuid::Uuid;

async fn connect_local() -> Option<PgEnt> {
    let url = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())?;
    let ent = PgEnt::connect(&url).await.ok()?;
    ent.migrate().await.ok()?;
    Some(ent)
}

async fn compter(ent: &PgEnt, etab: Uuid, domaine: &str, action: &str) -> i64 {
    sqlx::query(
        "SELECT count(*) AS n FROM ent_journal_modif \
         WHERE etablissement_id = $1 AND domaine = $2 AND action = $3",
    )
    .bind(etab)
    .bind(domaine)
    .bind(action)
    .fetch_one(ent.pool())
    .await
    .expect("count")
    .get::<i64, _>("n")
}

#[test]
fn map_sqlx_distingue_introuvable_et_panne() {
    use crate::pg_err::map_sqlx;
    assert_eq!(
        map_sqlx("t", sqlx::Error::RowNotFound),
        DomainError::NotFound
    );
    assert_eq!(
        map_sqlx("t", sqlx::Error::PoolClosed),
        DomainError::Infrastructure
    );
    assert_eq!(
        map_sqlx("t", sqlx::Error::PoolTimedOut),
        DomainError::Infrastructure
    );
    assert_eq!(
        map_sqlx("t", sqlx::Error::Protocol("x".into())),
        DomainError::Infrastructure
    );
}

#[tokio::test]
async fn pool_ferme_donne_infrastructure_et_non_introuvable() {
    let Some(ent) = connect_local().await else {
        eprintln!("skip L6: ENT_DATABASE_URL");
        return;
    };
    // Pool séparé, fermé : toute requête échoue côté infrastructure.
    let url = std::env::var("ENT_DATABASE_URL").unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    pool.close().await;
    let ferme = PgEnt::from_pool(pool);
    let r = ferme.get_eleve(Uuid::new_v4(), Uuid::new_v4()).await;
    match r {
        Err(EntDbError::Domain(DomainError::Infrastructure)) | Err(EntDbError::Sqlx(_)) => {}
        autre => panic!("une panne base ne doit pas être un « introuvable » : {autre:?}"),
    }
    drop(ent);
}

#[test]
fn split_statements_respecte_les_blocs_dollar() {
    let v = crate::migrate::split_statements(
        "CREATE TABLE a (x int);\nCREATE FUNCTION f() RETURNS trigger AS $$ BEGIN RETURN NEW; END; $$ LANGUAGE plpgsql;\nSELECT 1;",
    );
    let v: Vec<_> = v.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    assert_eq!(v.len(), 3, "{v:?}");
    assert!(v[1].contains("RETURN NEW; END;"));
}

#[tokio::test]
async fn echec_de_connexion_et_verrou_sont_traces_sans_email_en_clair() {
    let Some(ent) = connect_local().await else {
        eprintln!("skip L6: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let email = format!("l6-{}@ent.local", Uuid::new_v4());
    ent.register_user(&email, "DevLocalPass1!", None, etab, "admin")
        .await
        .expect("register");
    for _ in 0..12 {
        let _ = ent.login(&email, "MauvaisPass1!", etab).await;
    }
    let echecs = compter(&ent, etab, "auth", "connexion_echec").await;
    let verrous = compter(&ent, etab, "auth", "connexion_verrou").await;
    assert_eq!(echecs, 1, "échec tracé une seule fois par fenêtre (dédup atomique)");
    assert_eq!(verrous, 1, "le verrou doit être tracé une seule fois (dédup)");

    // Aucune trace de l'adresse en clair dans les lignes du journal.
    let lignes = sqlx::query("SELECT payload::text AS p, libelle FROM ent_journal_modif WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_all(ent.pool())
        .await
        .unwrap();
    assert!(!lignes.is_empty());
    for l in lignes {
        let p: String = l.get("p");
        let lib: String = l.get("libelle");
        assert!(!p.contains(&email) && !lib.contains(&email));
    }
}

#[tokio::test]
async fn consultation_pai_et_fiche_sante_sont_tracees() {
    let Some(ent) = connect_local().await else {
        eprintln!("skip L6: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let eleve = ent
        .create_eleve(CreateEleve {
            etablissement_id: etab,
            nom: "Trace".into(),
            prenom: "L6".into(),
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
        .expect("eleve");
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
    ent.get_pai_for_actor(etab, user.id, "infirmerie", eleve.id)
        .await
        .expect("pai");
    ent.get_fiche_sante_for_actor(etab, user.id, "infirmerie", eleve.id)
        .await
        .expect("fiche");
    assert_eq!(compter(&ent, etab, "consultation", "pai").await, 1);
    assert_eq!(compter(&ent, etab, "consultation", "fiche_sante").await, 1);

    // Un accès refusé ne laisse pas de trace de consultation.
    let avant = compter(&ent, etab, "consultation", "pai").await;
    let refus = ent
        .get_pai_for_actor(etab, user.id, "inconnu", eleve.id)
        .await;
    assert!(refus.is_err());
    assert_eq!(compter(&ent, etab, "consultation", "pai").await, avant);

    // Trace d'un téléchargement / d'un export (appel utilisé par l'API).
    ent.tracer_acces_sensible(
        etab,
        user.id,
        "admin",
        None,
        "export",
        "ogec",
        "Export OGEC",
    )
    .await;
    assert_eq!(compter(&ent, etab, "export", "ogec").await, 1);
}

#[tokio::test]
async fn journal_en_ajout_seul_refuse_update_et_delete() {
    let Some(ent) = connect_local().await else {
        eprintln!("skip L6: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, domaine, action, libelle) \
         VALUES ($1, $2, 'export', 'test_ajout_seul', 'Ligne de test')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .execute(ent.pool())
    .await
    .expect("insert autorisé");

    let upd = sqlx::query("UPDATE ent_journal_modif SET libelle = 'falsifié' WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await;
    assert!(upd.is_err(), "UPDATE du journal doit être refusé");
    let del = sqlx::query("DELETE FROM ent_journal_modif WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await;
    assert!(del.is_err(), "DELETE du journal doit être refusé");
    assert_eq!(compter(&ent, etab, "export", "test_ajout_seul").await, 1);
}

#[test]
fn debug_session_info_masque_le_jeton() {
    let s = SessionInfo {
        session_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        etablissement_id: Uuid::new_v4(),
        email: "personne@ent.local".into(),
        role: "admin".into(),
        token: "jeton-opaque-tres-secret".into(),
        expires_at: chrono::Utc::now(),
    };
    let d = format!("{s:?}");
    assert!(!d.contains("jeton-opaque-tres-secret"));
    assert!(!d.contains("personne@ent.local"));
    assert!(d.contains("[masqué]"));
}

#[tokio::test]
async fn echecs_anonymes_n_inondent_pas_le_journal() {
    let Some(ent) = connect_local().await.map(std::sync::Arc::new) else {
        eprintln!("skip L6: ENT_DATABASE_URL");
        return;
    };
    // Établissement inexistant (aucun membership) et e-mails inconnus : rien écrit.
    let etab_fantome = Uuid::new_v4();
    for i in 0..15 {
        let _ = ent
            .login(&format!("inconnu-{i}@ent.local"), "Mauvais1234!", etab_fantome)
            .await;
    }
    assert_eq!(compter(&ent, etab_fantome, "auth", "connexion_echec").await, 0);

    // Établissement réel, e-mail inconnu de cet établissement : rien écrit non plus.
    let etab = Uuid::new_v4();
    let email = format!("l6b-{}@ent.local", Uuid::new_v4());
    ent.register_user(&email, "DevLocalPass1!", None, etab, "admin")
        .await
        .expect("register");
    for i in 0..15 {
        let _ = ent
            .login(&format!("autre-{i}@ent.local"), "Mauvais1234!", etab)
            .await;
    }
    assert_eq!(compter(&ent, etab, "auth", "connexion_echec").await, 0);

    // Échecs concurrents sur un vrai compte : exactement une ligne.
    let mut taches = Vec::new();
    for _ in 0..4 {
        let (e, m) = (std::sync::Arc::clone(&ent), email.clone());
        taches.push(tokio::spawn(async move {
            let _ = e.login(&m, "Mauvais1234!", etab).await;
        }));
    }
    for t in taches {
        t.await.unwrap();
    }
    assert_eq!(compter(&ent, etab, "auth", "connexion_echec").await, 1);
}
