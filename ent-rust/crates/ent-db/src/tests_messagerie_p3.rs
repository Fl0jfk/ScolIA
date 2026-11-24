//! Messagerie, suites E1 (passe 3, migration 493) : F-6 (fil vide), garde d'annulation propriétaire, contrôle d'existence
//! juste avant chaque envoi, bornes réelles d'un curseur forgé.
//!
//! Aucune heure réelle : les instants viennent de la base (règle 33) ; les attentes sont des durées de verrou côté base
//! ou des sondes de l'état des verrous, jamais un jour ou une heure.

use super::tests_messagerie_m1::{compter, labo, role};
use super::tests_messagerie_m5::fil_de_lily;
use super::tests_messagerie_p2::{apres_l_echeance, avis_planifie, statut_avis};
use super::tests_messagerie_revue::{depart, flux, message_differe};
use super::*;
use chrono::Duration;
use ent_auth::Role;
use uuid::Uuid;

pub(super) fn expediteur_avec(
    ent: &PgEnt,
    etab: Uuid,
    attente_verrou: std::time::Duration,
) -> crate::notifications::Expediteur {
    ent.expediteur(crate::notifications::ConfigExpedition {
        taille_lot: 500,
        etablissement: Some(etab),
        attente_verrou,
        ..crate::notifications::ConfigExpedition::default()
    })
}

// ───────────────────────── F-6 : le fil dont le seul message est annulé ─────────────────────────

/// Après l'annulation du seul message d'un fil : le créateur du fil (le premier message en deux temps d'un parent) et le
/// côté établissement le gardent ; une famille qui n'en est pas le créateur ne le voit plus. Témoin : un fil dont le
/// message est remis reste listé.
#[tokio::test]
async fn p3_f6_le_createur_et_l_etablissement_gardent_le_fil_vide_pas_la_famille() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
    );
    let (fil, premier) = fil_de_lily(&ent, &cast, "Sujet-du-fil-vide").await;
    sqlx::query("UPDATE ent_msg SET remise_prevue_le = now() + interval '1 hour' WHERE id = $1")
        .bind(premier)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), premier)
        .await
        .expect("annulation");
    let voit = |user: Uuid, r: Role| {
        let ent = &ent;
        async move {
            ent.list_fils_for_actor(etab, user, role(r), None, false, None)
                .await
                .unwrap()
                .iter()
                .any(|f| f.id == fil)
        }
    };
    assert!(
        !voit(lily, Role::Parent).await,
        "la famille ne voit plus le fil vidé"
    );
    assert!(
        voit(dir, Role::Direction).await,
        "le côté établissement garde son fil vide"
    );
    // Le créateur du fil côté famille (premier message en deux temps : le fil naît avant son message) le garde.
    sqlx::query("UPDATE ent_msg_fil SET created_by_user_id = $2 WHERE id = $1")
        .bind(fil)
        .bind(lily)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    assert!(
        voit(lily, Role::Parent).await,
        "le créateur du fil le voit, même sans message"
    );
    // Témoin : un fil dont le message est remis reste listé à la famille.
    let (autre, _) = fil_de_lily(&ent, &cast, "Fil-avec-message-remis").await;
    let fils = ent
        .list_fils_for_actor(etab, lily, role(Role::Parent), None, false, None)
        .await
        .unwrap();
    assert!(
        fils.iter().any(|f| f.id == autre),
        "témoin : message remis, fil listé"
    );
}

// ───────────────────────── Garde d'annulation : jeton, auteur, propriétaire ─────────────────────────

pub(super) const ROLE_APP: &str = "ent_s16_p3_app";

/// Rôle applicatif non propriétaire, avec les privilèges du script de déploiement (`GRANT … ON ALL TABLES`), puis la 493
/// rejouée (elle est rejouable) : elle retire l'écriture de la table de garde, quel que soit le grantee.
pub(super) async fn pool_applicatif(ent: &PgEnt) -> sqlx::PgPool {
    let admin = ent.pool_verif();
    sqlx::query(&format!(
        "DO $$ BEGIN CREATE ROLE {ROLE_APP} LOGIN PASSWORD 'p3' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; \
         EXCEPTION WHEN duplicate_object THEN NULL; END $$"
    ))
    .execute(admin)
    .await
    .unwrap();
    for sql in [
        format!("GRANT USAGE ON SCHEMA public TO {ROLE_APP}"),
        format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {ROLE_APP}"
        ),
        format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {ROLE_APP}"),
    ] {
        sqlx::query(&sql).execute(admin).await.unwrap();
    }
    // La migration rejouée retire l'écriture de la table de garde au rôle qui vient de la recevoir.
    sqlx::raw_sql(include_str!(
        "../../../migrations/493_msg_fil_vide_et_garde_proprietaire.sql"
    ))
    .execute(admin)
    .await
    .unwrap();
    // 494 (M1) : le droit d'exécution du jeton va aux rôles qui écrivent dans `ent_msg` ; rejouée après la création du rôle.
    sqlx::raw_sql(include_str!(
        "../../../migrations/494_msg_garde_jeton_droit_execution.sql"
    ))
    .execute(admin)
    .await
    .unwrap();
    let url = crate::url_guard::url_labo_ou_echec().expect("url labo");
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username(ROLE_APP).unwrap();
    u.set_password(Some("p3")).unwrap();
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(u.as_str())
        .await
        .unwrap()
}

pub(super) fn code(e: &sqlx::Error) -> Option<String> {
    e.as_database_error()
        .and_then(|d| d.code())
        .map(|c| c.to_string())
}

#[tokio::test]
async fn p3_la_table_de_garde_n_est_plus_inscriptible_par_le_role_applicatif_et_l_auteur_est_verifie(
) {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
    );
    let (fil, _) = fil_de_lily(&ent, &cast, "Garde propriétaire").await;
    let m = message_differe(&ent, etab, dir, fil, "A v1").await;
    ent.corriger_message_fil_for_actor(etab, dir, role(Role::Direction), m, "A v2")
        .await
        .expect("correction : une version à protéger");
    let app = pool_applicatif(&ent).await;

    // Précondition : le rôle applicatif n'est ni propriétaire des objets, ni superutilisateur, et la fonction est
    // `SECURITY DEFINER` à `search_path` fixé.
    let (prop_table, prop_fonction, definer, conf, super_): (String, String, bool, Vec<String>, bool) = sqlx::query_as(
        "SELECT pg_get_userbyid(c.relowner), pg_get_userbyid(p.proowner), p.prosecdef, coalesce(p.proconfig, '{}'),
                (SELECT rolsuper FROM pg_roles WHERE rolname = $1)
           FROM pg_class c, pg_proc p
          WHERE c.oid = 'public.ent_msg_annulation_garde'::regclass AND p.proname = 'ent_msg_annulation_ouvrir'",
    )
    .bind(ROLE_APP)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_ne!(prop_table, ROLE_APP);
    assert_ne!(prop_fonction, ROLE_APP);
    assert!(!super_);
    assert!(definer, "ent_msg_annulation_ouvrir est SECURITY DEFINER");
    assert!(
        conf.iter()
            .any(|c| c == "search_path=pg_catalog, public, pg_temp"),
        "search_path fixé : {conf:?}"
    );
    // Une seule fonction `ent_msg_annulation_ouvrir` : la version à deux arguments (492) a disparu.
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM pg_proc WHERE proname = 'ent_msg_annulation_ouvrir' AND pronargs = 3",
            &[]
        )
        .await,
        1
    );
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM pg_proc WHERE proname = 'ent_msg_annulation_ouvrir'",
            &[]
        )
        .await,
        1
    );

    // 1. Sous le rôle applicatif : ni INSERT, ni UPDATE, ni DELETE, ni TRUNCATE directs sur la table de garde (42501).
    for sql in [
        "INSERT INTO ent_msg_annulation_garde (etablissement_id, message_id, xid)
         VALUES ($1, $2, pg_current_xact_id())",
        "UPDATE ent_msg_annulation_garde SET cree_le = now() WHERE etablissement_id = $1 AND message_id = $2",
        "DELETE FROM ent_msg_annulation_garde WHERE etablissement_id = $1 AND message_id = $2",
    ] {
        let mut tx = app.begin().await.unwrap();
        sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
            .bind(etab.to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        let e = sqlx::query(sql).bind(etab).bind(m).execute(&mut *tx).await.expect_err(sql);
        assert_eq!(code(&e).as_deref(), Some("42501"), "{sql} : {e:?}");
    }
    let e = sqlx::query("TRUNCATE ent_msg_annulation_garde")
        .execute(&app)
        .await
        .expect_err("TRUNCATE");
    assert_eq!(code(&e).as_deref(), Some("42501"), "{e:?}");
    // Le jeton n'a donc pas pu être forgé : la suppression d'une version est refusée par la garde.
    let mut tx = app.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let e = sqlx::query("DELETE FROM ent_msg_version WHERE message_id = $1")
        .bind(m)
        .execute(&mut *tx)
        .await
        .expect_err("sans jeton");
    assert_eq!(code(&e).as_deref(), Some("23514"), "{e:?}");
    drop(tx);

    // 2. Ouvrir le jeton sur le message d'un autre auteur est refusé (23514) ; l'auteur lui-même l'obtient.
    let mut tx = app.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let e = sqlx::query("SELECT ent_msg_annulation_ouvrir($1, $2, $3)")
        .bind(etab)
        .bind(m)
        .bind(lily)
        .execute(&mut *tx)
        .await
        .expect_err("message d'un autre auteur");
    assert_eq!(code(&e).as_deref(), Some("23514"), "{e:?}");
    assert!(e.to_string().contains("autre auteur"), "{e}");
    drop(tx);
    // Témoin : sous le rôle applicatif, l'auteur ouvre le jeton (la fonction s'exécute avec les droits du propriétaire) et
    // la suppression de la version passe dans cette transaction.
    let mut tx = app.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SELECT ent_msg_annulation_ouvrir($1, $2, $3)")
        .bind(etab)
        .bind(m)
        .bind(dir)
        .execute(&mut *tx)
        .await
        .expect("l'auteur ouvre le jeton");
    let n = sqlx::query("DELETE FROM ent_msg_version WHERE message_id = $1")
        .bind(m)
        .execute(&mut *tx)
        .await
        .expect("la suppression passe avec le jeton");
    assert_eq!(n.rows_affected(), 1);
    drop(tx);
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_msg_version WHERE message_id = $1",
            &[m]
        )
        .await,
        1,
        "rien n'a été validé"
    );
}

// ───────────────────────── Contrôle juste avant chaque envoi ─────────────────────────

/// Pid de la connexion qui **tient** le verrou : la sonde ne regarde que les sessions bloquées par elle (F8 : plus de
/// recherche par texte de requête, qui attrapait n'importe quelle session d'un autre test sur la même base).
pub(super) async fn pid_de(connexion: &mut sqlx::PgConnection) -> i32 {
    let (pid,): (i32,) = sqlx::query_as("SELECT pg_backend_pid()").fetch_one(connexion).await.unwrap();
    pid
}

/// Attend (sonde de verrou, pas d'heure) qu'`au moins `attendus` sessions soient bloquées **par la connexion `pid`**
/// (`pg_blocking_pids`) : celle du test lui-même, jamais une session étrangère.
pub(super) async fn attendre_bloques_par(ent: &PgEnt, pid: i32, attendus: i64) {
    for _ in 0..400 {
        let n: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM pg_stat_activity
              WHERE wait_event_type = 'Lock' AND $1 = ANY (pg_blocking_pids(pid)) AND pid <> pg_backend_pid()",
        )
        .bind(pid)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        if n.0 >= attendus {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("aucune session n'a jamais été bloquée par la connexion {pid} (attendues : {attendus})");
}

/// Vraie course : une annulation (message verrouillé `FOR UPDATE`, avis annulé, message supprimé) reste **ouverte** sur
/// une seconde connexion pendant que le distributeur tourne. Le contrôle d'existence, juste avant l'envoi, attend la fin
/// de l'annulation puis écarte l'avis : rien ne part pour le message annulé.
#[tokio::test]
async fn p3_course_reelle_une_annulation_ouverte_pendant_le_distributeur_rien_ne_part() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
    );
    let (fil, _) = fil_de_lily(&ent, &cast, "Course réelle").await;
    let m = message_differe(&ent, etab, dir, fil, "Annulé pendant le passage").await;
    let avis = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, m).await;
    let apres = apres_l_echeance(&ent).await;

    // Annulation « en cours » : le message est verrouillé, rien n'est encore validé.
    let mut annulation = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *annulation)
        .await
        .unwrap();
    sqlx::query("SELECT 1 FROM ent_msg WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
        .bind(etab)
        .bind(m)
        .fetch_one(&mut *annulation)
        .await
        .unwrap();

    let pid_annulation = pid_de(&mut annulation).await;
    let exp = expediteur_avec(&ent, etab, std::time::Duration::from_secs(60));
    let passage = tokio::spawn(async move { exp.expedier(apres).await });
    attendre_bloques_par(&ent, pid_annulation, 1).await;
    assert_eq!(
        statut_avis(&ent, avis).await.0,
        "envoi",
        "le distributeur a pris l'avis et attend le message"
    );

    // L'annulation s'achève : avis annulé, message supprimé, validation.
    sqlx::query(
        "UPDATE ent_notification_envoi SET statut = 'annule', motif = 'message_annule', traite_le = now()
          WHERE etablissement_id = $1 AND source_id = $2 AND statut IN ('en_attente', 'envoi')",
    )
    .bind(etab)
    .bind(m)
    .execute(&mut *annulation)
    .await
    .unwrap();
    sqlx::query("DELETE FROM ent_msg WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(m)
        .execute(&mut *annulation)
        .await
        .unwrap();
    annulation.commit().await.unwrap();

    let bilan = passage.await.unwrap().unwrap();
    assert_eq!(bilan.reclames, 1, "{bilan:?}");
    assert_eq!(
        bilan.envoyes + bilan.simules + bilan.regroupes,
        0,
        "rien ne part pour le message annulé : {bilan:?}"
    );
    assert_eq!(bilan.ignores, 1, "{bilan:?}");
    assert_eq!(
        statut_avis(&ent, avis).await,
        ("annule".to_string(), Some("message_annule".to_string())),
        "l'avis reste annulé, jamais ressuscité"
    );
}

/// Une erreur de base pendant le contrôle (ici : un verrou d'annulation qui dépasse l'attente) laisse l'avis `en_attente`
/// au lieu d'interrompre le lot : l'avis d'un message vivant, lui, part dans le même passage ; au passage suivant (la
/// transaction d'annulation défaite), l'avis reporté part aussi.
#[tokio::test]
async fn p3_une_erreur_pendant_le_controle_reporte_l_avis_sans_interrompre_le_lot() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
    );
    let (fil, _) = fil_de_lily(&ent, &cast, "Contrôle en erreur").await;
    let verrouille =
        message_differe(&ent, etab, dir, fil, "Verrouillé par une annulation lente").await;
    let vivant = message_differe(&ent, etab, dir, fil, "Vivant").await;
    let avis_verrouille = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, verrouille).await;
    let avis_vivant = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, vivant).await;
    let apres = apres_l_echeance(&ent).await;

    let mut lente = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *lente)
        .await
        .unwrap();
    sqlx::query("SELECT 1 FROM ent_msg WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
        .bind(etab)
        .bind(verrouille)
        .fetch_one(&mut *lente)
        .await
        .unwrap();

    // Attente de verrou courte (durée côté base, pas une heure) : le contrôle échoue (55P03).
    let bilan = expediteur_avec(&ent, etab, std::time::Duration::from_millis(200))
        .expedier(apres)
        .await
        .expect("le lot n'est pas interrompu par l'erreur d'un contrôle");
    assert_eq!(bilan.reportes, 1, "{bilan:?}");
    assert_eq!(
        statut_avis(&ent, avis_verrouille).await.0,
        "en_attente",
        "avis reporté, pas perdu : {bilan:?}"
    );
    let (statut_vivant, _) = statut_avis(&ent, avis_vivant).await;
    assert!(
        !matches!(statut_vivant.as_str(), "en_attente" | "envoi"),
        "l'avis du message vivant est traité dans le même passage : {statut_vivant}"
    );
    lente.rollback().await.unwrap();

    // Passage suivant : l'annulation est défaite, l'avis reporté part.
    let bilan = expediteur_avec(&ent, etab, std::time::Duration::from_secs(5))
        .expedier(apres)
        .await
        .unwrap();
    assert_eq!(bilan.reportes, 0, "{bilan:?}");
    let (statut, motif) = statut_avis(&ent, avis_verrouille).await;
    assert!(
        !matches!(statut.as_str(), "en_attente" | "envoi"),
        "traité au passage suivant : {statut}"
    );
    assert_ne!(
        motif.as_deref(),
        Some("message_annule"),
        "le message existe : {statut}"
    );
}

// ───────────────────────── M-2 : bornes réelles d'un curseur forgé ─────────────────────────

/// Les curseurs forgés **au ras des bornes** (et leurs témoins du bon côté) : la borne `xmin` est épinglée par une
/// transaction écrivante ouverte, donc connue exactement ; les bornes de temps sont éprouvées avec une marge de deux
/// minutes (jamais l'instant exact, que le temps de l'appel ferait osciller).
#[tokio::test]
async fn p3_m2_les_bornes_reelles_du_curseur_forge_sont_celles_de_la_base() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lily) = (cast.etablissement_id, cast.parent_lea.user_id);
    // Transaction écrivante ouverte : elle porte un identifiant et épingle `xmin` sur lui.
    let mut epingle = ent.pool_verif().begin().await.unwrap();
    let (x,): (String,) = sqlx::query_as("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut *epingle)
        .await
        .unwrap();
    let x: u64 = x.parse().unwrap();
    let sain = depart(&ent, etab).await;
    let base = crate::PositionFlux::decoder(&sain).expect("lisible");
    assert_eq!(
        base.xid, x,
        "précondition : la borne est épinglée sur la transaction ouverte"
    );

    let tolerance = Duration::seconds(crate::msg_repo::FLUX_TOLERANCE_HORLOGE_SECONDES);
    let fenetre = Duration::seconds(crate::msg_repo::FLUX_FENETRE_SECONDES as i64);
    let marge = Duration::minutes(2);
    let valides = [
        (
            "xid = borne",
            crate::PositionFlux {
                xid: x,
                ..base.clone()
            },
        ),
        (
            "emis en deçà de la fenêtre",
            crate::PositionFlux {
                emis: base.emis - fenetre + marge,
                ..base.clone()
            },
        ),
        (
            "differe en deçà de la fenêtre doublée",
            crate::PositionFlux {
                differe: base.differe - fenetre * 2 + marge,
                ..base.clone()
            },
        ),
    ];
    for (nom, c) in valides {
        let r = flux(&ent, etab, lily, Role::Parent, &c.encoder()).await;
        assert!(!r.resynchro, "{nom} : accepté : {r:?}");
    }
    let forges = [
        (
            "xid = borne + 1",
            crate::PositionFlux {
                xid: x + 1,
                ..base.clone()
            },
        ),
        (
            "emis au-delà de la tolérance",
            crate::PositionFlux {
                emis: base.emis + tolerance + marge,
                ..base.clone()
            },
        ),
        (
            "emis plus vieux que la fenêtre",
            crate::PositionFlux {
                emis: base.emis - fenetre - marge,
                ..base.clone()
            },
        ),
        (
            "differe au-delà de la tolérance",
            crate::PositionFlux {
                differe: base.differe + tolerance + marge,
                ..base.clone()
            },
        ),
        (
            "differe plus vieux que deux fenêtres",
            crate::PositionFlux {
                differe: base.differe - fenetre * 2 - marge,
                ..base.clone()
            },
        ),
    ];
    for (nom, c) in forges {
        let r = flux(&ent, etab, lily, Role::Parent, &c.encoder()).await;
        assert!(
            r.resynchro && r.fils.is_empty(),
            "{nom} : ramené au point de resynchronisation : {r:?}"
        );
    }
    epingle.rollback().await.unwrap();
}
