//! Messagerie, passe 2 de la contre-revue E1 (`docs/socles/16_MESSAGERIE_M12_E1_LIVRAISON.md`, « Passe 2 ») :
//! M-1 (annulation tout ou rien, avis compris), M-2 (curseur borné), M-3 (garde à jeton de transaction), F-1 (cluster),
//! F-3 (réglage sur A, versions de B).
//!
//! Aucune heure réelle : les instants viennent de la base (règle 33) ; l'expéditeur reçoit un « maintenant » calculé
//! à partir de l'heure de la base.

use super::tests_messagerie_m1::{compter, ecrit, labo, role};
use super::tests_messagerie_m5::fil_de_lily;
use super::tests_messagerie_m6::dernier_message;
use super::tests_messagerie_revue::{conflit_exact, depart, flux, message_differe, restes};
use super::*;
use chrono::{DateTime, Duration, Utc};
use ent_auth::Role;
use uuid::Uuid;

/// Avis `push` planifié (dans deux heures) pour un message différé, comme l'écriture d'un différé le crée.
async fn avis_planifie(ent: &PgEnt, etab: Uuid, lily: Uuid, eleve: Uuid, m: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_notification_envoi
            (id, etablissement_id, destinataire_user_id, canal, kind, titre, corps, eleve_id,
             source_type, source_id, cle_dedup, droit_requis, planifie_pour)
         VALUES ($1,$2,$3,'push','messagerie','Nouveau message','Un message vous attend',$4,'systeme',$5,$6,'lien',
                 now() + interval '2 hours')",
    )
    .bind(id)
    .bind(etab)
    .bind(lily)
    .bind(eleve)
    .bind(m)
    .bind(format!("msg-avis:{m}"))
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}

async fn statut_avis(ent: &PgEnt, id: Uuid) -> (String, Option<String>) {
    sqlx::query_as("SELECT statut, motif FROM ent_notification_envoi WHERE id = $1")
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

/// « Maintenant » de l'expéditeur : l'heure de la base plus trois heures (après l'échéance de l'avis, deux heures).
async fn apres_l_echeance(ent: &PgEnt) -> DateTime<Utc> {
    let (maintenant,): (DateTime<Utc>,) = sqlx::query_as("SELECT now()")
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    maintenant + Duration::hours(3)
}

fn expediteur(ent: &PgEnt, etab: Uuid) -> crate::notifications::Expediteur {
    ent.expediteur(crate::notifications::ConfigExpedition {
        taille_lot: 500,
        etablissement: Some(etab),
        ..crate::notifications::ConfigExpedition::default()
    })
}

// ───────────────────────── M-1 : l'annulation emporte l'avis, dans la même transaction ─────────────────────────

#[tokio::test]
async fn p2_m1_un_echec_de_l_annulation_laisse_l_avis_intact_et_un_succes_n_en_laisse_aucun() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
    );
    let (fil, _) = fil_de_lily(&ent, &cast, "Avis et annulation").await;
    let m = message_differe(&ent, etab, dir, fil, "Parti plus tard").await;
    let avis = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, m).await;

    // Échec simulé **après** la suppression des lignes dépendantes et de l'avis : une référence inconnue de
    // l'annulation fait refuser la suppression du message (23503). Tout est défait, l'avis aussi : le message est encore
    // planifié, son avis encore en attente, cohérents.
    let table = format!("p2_ref_{}", Uuid::new_v4().simple());
    sqlx::query(&format!(
        "CREATE TABLE {table} (etablissement_id UUID NOT NULL, message_id UUID NOT NULL,
            FOREIGN KEY (etablissement_id, message_id) REFERENCES ent_msg (etablissement_id, id))"
    ))
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(&format!("INSERT INTO {table} VALUES ($1, $2)"))
        .bind(etab)
        .bind(m)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let e = ent
        .annuler_message_differe_for_actor(etab, dir, role(Role::Direction), m)
        .await
        .err();
    sqlx::query(&format!("DROP TABLE {table}"))
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let e = e.expect("refus de la base");
    assert!(
        conflit_exact(
            &e,
            "annulation refusée : le message n'a pas été annulé et sera remis normalement"
        ),
        "{e:?}"
    );
    assert_eq!(
        statut_avis(&ent, avis).await.0,
        "en_attente",
        "l'avis n'a pas été annulé seul"
    );
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_msg WHERE id = $1", &[m]).await,
        1,
        "le message est resté"
    );

    // Succès : le message et l'avis disparaissent ensemble ; aucun avis ne reste `en_attente` ni `envoi`.
    ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), m)
        .await
        .expect("annulation");
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_msg WHERE id = $1", &[m]).await,
        0
    );
    assert_eq!(
        statut_avis(&ent, avis).await,
        ("annule".to_string(), Some("message_annule".to_string()))
    );
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_notification_envoi WHERE source_id = $1 AND statut IN ('en_attente', 'envoi')",
            &[m]
        )
        .await,
        0
    );
}

#[tokio::test]
async fn p2_m1_course_annulation_et_distributeur_rien_ne_part_pour_un_message_annule() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
    );
    let (fil, _) = fil_de_lily(&ent, &cast, "Course à l'échéance").await;
    let apres = apres_l_echeance(&ent).await;

    // 1. Annulé avant l'échéance : au passage du distributeur, rien ne part, l'avis reste annulé.
    let m1 = message_differe(&ent, etab, dir, fil, "Annulé avant").await;
    let a1 = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, m1).await;
    ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), m1)
        .await
        .expect("annulation");
    let bilan = expediteur(&ent, etab).expedier(apres).await.unwrap();
    assert_eq!(
        statut_avis(&ent, a1).await.0,
        "annule",
        "jamais repris par le distributeur : {bilan:?}"
    );

    // 2. Course : le distributeur a **déjà pris** l'avis (`envoi`) quand l'annulation arrive : l'avis passe à `annule`
    // dans la transaction de l'annulation ; la finalisation du distributeur (`WHERE statut = 'envoi'`) ne le ressuscite pas.
    let m2 = message_differe(&ent, etab, dir, fil, "Pris puis annulé").await;
    let a2 = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, m2).await;
    sqlx::query(
        "UPDATE ent_notification_envoi SET statut = 'envoi', tentative_le = now() WHERE id = $1",
    )
    .bind(a2)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), m2)
        .await
        .expect("annulation");
    assert_eq!(
        statut_avis(&ent, a2).await.0,
        "annule",
        "l'avis `envoi` est annulé avec le message"
    );
    let finalise = sqlx::query(
        "UPDATE ent_notification_envoi SET statut = 'envoye', traite_le = now() WHERE id = $1 AND statut = 'envoi'",
    )
    .bind(a2)
    .execute(ent.pool_verif())
    .await
    .unwrap()
    .rows_affected();
    assert_eq!(
        finalise, 0,
        "la finalisation du distributeur ne touche plus l'avis annulé"
    );

    // 3. Défense en profondeur : un avis dont le message n'existe plus (suppression hors annulation) est écarté, sous
    // verrou, avant l'envoi : le distributeur vérifie l'existence du message.
    let m3 = message_differe(&ent, etab, dir, fil, "Disparu").await;
    let a3 = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, m3).await;
    sqlx::query("DELETE FROM ent_msg WHERE id = $1")
        .bind(m3)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    expediteur(&ent, etab).expedier(apres).await.unwrap();
    assert_eq!(
        statut_avis(&ent, a3).await,
        ("ignore".to_string(), Some("message_annule".to_string())),
        "rien ne part pour un message qui n'existe plus"
    );

    // Témoin : un avis dont le message existe suit son cours (le contrôle n'écarte pas tout).
    let m4 = message_differe(&ent, etab, dir, fil, "Vivant").await;
    let a4 = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, m4).await;
    expediteur(&ent, etab).expedier(apres).await.unwrap();
    let (statut, motif) = statut_avis(&ent, a4).await;
    assert_ne!(
        motif.as_deref(),
        Some("message_annule"),
        "témoin : {statut}"
    );
}

// ───────────────────────── M-3 : garde à jeton de transaction, search_path fixé ─────────────────────────

/// Un message remis, corrigé une fois (donc une version en ajout seul).
async fn remis_avec_version(ent: &PgEnt, etab: Uuid, dir: Uuid, fil: Uuid, texte: &str) -> Uuid {
    assert!(ecrit(ent, etab, dir, Role::Direction, fil, texte).await);
    let remis = dernier_message(ent, etab, fil, dir).await.id;
    ent.corriger_message_fil_for_actor(etab, dir, role(Role::Direction), remis, "corrigé")
        .await
        .expect("correction");
    assert_eq!(
        compter(
            ent,
            "SELECT count(*) FROM ent_msg_version WHERE message_id = $1",
            &[remis]
        )
        .await,
        1
    );
    remis
}

fn refus_exact(r: Result<sqlx::postgres::PgQueryResult, sqlx::Error>) {
    let e = r.expect_err("suppression refusée par la garde");
    let d = e.as_database_error().expect("erreur de la base");
    assert_eq!(d.code().as_deref(), Some("23514"), "{}", d.message());
    assert!(d.message().contains("ajout seul"), "{}", d.message());
}

#[tokio::test]
async fn p2_m3_ni_une_table_temporaire_homonyme_ni_un_reglage_pose_a_la_main_n_ouvrent_la_garde() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Garde à jeton").await;
    let remis = remis_avec_version(&ent, etab, dir, fil, "Déjà remis").await;
    let supprimer = "DELETE FROM ent_msg_version WHERE message_id = $1";

    // Table temporaire homonyme de `ent_msg` : une ligne « future » pour ce message remis (l'attaque de M-3).
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("CREATE TEMP TABLE ent_msg (etablissement_id UUID, id UUID, remise_prevue_le TIMESTAMPTZ) ON COMMIT DROP")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_msg VALUES ($1, $2, now() + interval '1 hour')")
        .bind(etab)
        .bind(remis)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('ent.msg_annulation', $1, true)")
        .bind(remis.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    refus_exact(sqlx::query(supprimer).bind(remis).execute(&mut *tx).await);
    drop(tx);

    // Même attaque doublée d'une fausse table de garde (jeton de la transaction courante, `pg_temp` en tête de recherche).
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SET LOCAL search_path = pg_temp, public")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("CREATE TEMP TABLE ent_msg (etablissement_id UUID, id UUID, remise_prevue_le TIMESTAMPTZ) ON COMMIT DROP")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("CREATE TEMP TABLE ent_msg_annulation_garde (etablissement_id UUID, message_id UUID, xid xid8) ON COMMIT DROP")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_msg VALUES ($1, $2, now() + interval '1 hour')")
        .bind(etab)
        .bind(remis)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_msg_annulation_garde VALUES ($1, $2, pg_current_xact_id())")
        .bind(etab)
        .bind(remis)
        .execute(&mut *tx)
        .await
        .unwrap();
    refus_exact(sqlx::query(supprimer).bind(remis).execute(&mut *tx).await);
    drop(tx);

    // Réglage libre posé à la main sur un message déjà remis, sans jeton : refus.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.msg_annulation', $1, true)")
        .bind(remis.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    refus_exact(sqlx::query(supprimer).bind(remis).execute(&mut *tx).await);
    drop(tx);

    // Le jeton lui-même est refusé pour un message remis (la fonction d'ouverture vérifie l'échéance).
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let e = sqlx::query("SELECT ent_msg_annulation_ouvrir($1, $2)")
        .bind(etab)
        .bind(remis)
        .execute(&mut *tx)
        .await
        .expect_err("refus");
    assert_eq!(
        e.as_database_error().and_then(|d| d.code()).as_deref(),
        Some("23514")
    );
    drop(tx);
    assert_eq!(
        compter(
            &ent,
            "SELECT count(*) FROM ent_msg_version WHERE message_id = $1",
            &[remis]
        )
        .await,
        1,
        "rien supprimé"
    );

    // Fonction de garde : `search_path` fixé, `pg_temp` en dernier.
    let reglages: Vec<(String,)> = sqlx::query_as(
        "SELECT unnest(proconfig) FROM pg_proc WHERE proname IN ('ent_msg_ajout_seul_sauf_annulation', 'ent_msg_annulation_ouvrir', 'ent_msg_retire_xid')",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(reglages.len(), 3);
    assert!(
        reglages
            .iter()
            .all(|(r,)| r == "search_path=pg_catalog, public, pg_temp"),
        "{reglages:?}"
    );
}

#[tokio::test]
async fn p2_m3_un_jeton_pour_a_ne_supprime_pas_les_versions_de_b_ni_celles_d_un_autre_etablissement(
) {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Deux différés").await;
    let a = message_differe(&ent, etab, dir, fil, "A v1").await;
    let b = message_differe(&ent, etab, dir, fil, "B v1").await;
    for m in [a, b] {
        ent.corriger_message_fil_for_actor(etab, dir, role(Role::Direction), m, "v2")
            .await
            .expect("correction");
    }
    // Jeton ouvert pour A : les versions de B, pourtant d'un message encore planifié, restent protégées.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT ent_msg_annulation_ouvrir($1, $2)")
        .bind(etab)
        .bind(a)
        .execute(&mut *tx)
        .await
        .expect("jeton pour A");
    refus_exact(
        sqlx::query("DELETE FROM ent_msg_version WHERE message_id = $1")
            .bind(b)
            .execute(&mut *tx)
            .await,
    );
    drop(tx);
    // Témoin : la même transaction, pour A, passe.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT ent_msg_annulation_ouvrir($1, $2)")
        .bind(etab)
        .bind(a)
        .execute(&mut *tx)
        .await
        .unwrap();
    let n = sqlx::query("DELETE FROM ent_msg_version WHERE message_id = $1")
        .bind(a)
        .execute(&mut *tx)
        .await
        .expect("A passe");
    assert_eq!(n.rows_affected(), 1);
    drop(tx);

    // Inter-établissement : un jeton demandé sous le contexte d'un établissement pour le message d'un autre est refusé.
    let autre = ent.seed_labo_cast().await.expect("autre établissement");
    assert_ne!(
        autre.etablissement_id, etab,
        "précondition : un autre établissement"
    );
    let (fil_b, _) = fil_de_lily(&ent, &autre, "Autre établissement").await;
    let m_autre = message_differe(
        &ent,
        autre.etablissement_id,
        autre.direction.user_id,
        fil_b,
        "Ailleurs",
    )
    .await;
    let mut tx = crate::tenant_tx::TenantTx::begin_mode(
        ent.pool(),
        etab,
        crate::tenant_tx::RlsMode::Permissif,
    )
    .await
    .unwrap();
    let e = sqlx::query("SELECT ent_msg_annulation_ouvrir($1, $2)")
        .bind(autre.etablissement_id)
        .bind(m_autre)
        .execute(&mut *tx)
        .await
        .expect_err("refus inter-établissements");
    assert_eq!(
        e.as_database_error().and_then(|d| d.code()).as_deref(),
        Some("23514"),
        "{e:?}"
    );
    drop(tx);
    // Le message de B n'a rien perdu.
    assert_eq!(restes(&ent, b).await[..2], [("message", 1), ("version", 1)]);
}

// ───────────────────────── M-2 et F-1 : curseur borné, lié au cluster ─────────────────────────

#[tokio::test]
async fn p2_m2_un_curseur_fabrique_est_ramene_au_point_de_resynchronisation() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lily) = (cast.etablissement_id, cast.parent_lea.user_id);
    let sain = depart(&ent, etab).await;
    let base = crate::PositionFlux::decoder(&sain).expect("curseur lisible");
    // Témoin : le curseur sain n'est pas ramené.
    assert!(
        !flux(&ent, etab, lily, Role::Parent, &sain).await.resynchro,
        "témoin"
    );

    let mut forges: Vec<(&str, crate::PositionFlux)> = Vec::new();
    forges.push((
        "emis dans le futur",
        crate::PositionFlux {
            emis: base.emis + Duration::hours(1),
            ..base.clone()
        },
    ));
    forges.push((
        "differe dans le futur",
        crate::PositionFlux {
            differe: base.differe + Duration::hours(1),
            ..base.clone()
        },
    ));
    forges.push((
        "differe hors fenêtre",
        crate::PositionFlux {
            differe: base.differe - Duration::days(2),
            ..base.clone()
        },
    ));
    forges.push((
        "xid au-delà de la borne",
        crate::PositionFlux {
            xid: base.xid + 1_000_000,
            ..base.clone()
        },
    ));
    forges.push((
        "xid maximal",
        crate::PositionFlux {
            xid: u64::MAX,
            ..base.clone()
        },
    ));
    forges.push((
        "autre cluster (F-1)",
        crate::PositionFlux {
            cluster: base.cluster ^ 0x5555,
            ..base.clone()
        },
    ));
    for (nom, c) in forges {
        let r = flux(&ent, etab, lily, Role::Parent, &c.encoder()).await;
        assert!(
            r.resynchro && r.fils.is_empty() && !r.tronque,
            "{nom} : ramené au point de resynchronisation : {r:?}"
        );
        let rendu = crate::PositionFlux::decoder(&r.curseur).expect("curseur rendu lisible");
        assert_eq!(
            rendu.cluster, base.cluster,
            "{nom} : le curseur rendu est celui de ce cluster"
        );
        assert!(
            rendu.emis > c.emis.min(base.emis) - Duration::hours(2)
                && rendu.xid <= base.xid + 100_000,
            "{nom} : point sain"
        );
    }
}

#[test]
fn p2_m2_bornes_du_curseur_xid_zero_et_marge() {
    use crate::msg_repo::{curseur_borne as curseur_borne_pour_tests, FLUX_MARGE_XID};
    use chrono::TimeZone;
    let maintenant = Utc.with_ymd_and_hms(2026, 11, 20, 10, 0, 0).unwrap();
    let xmin = 3 * FLUX_MARGE_XID;
    let sain = crate::PositionFlux::depart(xmin, maintenant, 42);
    assert!(curseur_borne_pour_tests(&sain, maintenant, xmin, 42));
    // xid 0 (curseur fabriqué pour relire tout l'historique) : hors de la marge.
    assert!(!curseur_borne_pour_tests(
        &crate::PositionFlux {
            xid: 0,
            ..sain.clone()
        },
        maintenant,
        xmin,
        42
    ));
    assert!(!curseur_borne_pour_tests(
        &crate::PositionFlux {
            xid: xmin - FLUX_MARGE_XID - 1,
            ..sain.clone()
        },
        maintenant,
        xmin,
        42
    ));
    assert!(curseur_borne_pour_tests(
        &crate::PositionFlux {
            xid: xmin - FLUX_MARGE_XID,
            ..sain.clone()
        },
        maintenant,
        xmin,
        42
    ));
    assert!(!curseur_borne_pour_tests(
        &crate::PositionFlux {
            xid: xmin + 1,
            ..sain.clone()
        },
        maintenant,
        xmin,
        42
    ));
    // emis et differe dans le futur, cluster étranger.
    assert!(!curseur_borne_pour_tests(
        &crate::PositionFlux {
            emis: maintenant + Duration::minutes(1),
            ..sain.clone()
        },
        maintenant,
        xmin,
        42
    ));
    assert!(!curseur_borne_pour_tests(
        &crate::PositionFlux {
            differe: maintenant + Duration::minutes(1),
            ..sain.clone()
        },
        maintenant,
        xmin,
        42
    ));
    assert!(!curseur_borne_pour_tests(&sain, maintenant, xmin, 43));
}

#[tokio::test]
async fn p2_f1_le_curseur_porte_l_identite_du_cluster_et_s_encode_a_l_identique() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let sain = depart(&ent, cast.etablissement_id).await;
    let p = crate::PositionFlux::decoder(&sain).expect("lisible");
    assert!(sain.starts_with("v3."));
    let (identifiant,): (i64,) =
        sqlx::query_as("SELECT system_identifier FROM pg_control_system()")
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(p.cluster, identifiant);
    assert_eq!(p.encoder(), sain, "aller-retour");
    // Un curseur d'avant (v2, sans identité) n'est plus compris : resynchronisation.
    assert!(crate::PositionFlux::decoder("v2.1.-.-.0.-.0").is_none());
}

// ───────────────────────── F-6 : le fil dont le premier message est annulé ─────────────────────────

#[tokio::test]
async fn p2_f6_constat_le_fil_vide_apres_annulation_reste_liste_a_la_famille() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
    );
    let (fil, premier) = fil_de_lily(&ent, &cast, "Sujet-confidentiel-annule").await;
    // Le premier message du fil est programmé pour plus tard : la famille ne voit encore rien.
    sqlx::query("UPDATE ent_msg SET remise_prevue_le = now() + interval '1 hour' WHERE id = $1")
        .bind(premier)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let vu = |fils: &[domain_eleve::MsgFil]| fils.iter().any(|f| f.id == fil);
    let avant = ent
        .list_fils_for_actor(etab, lily, role(Role::Parent), None, false, None)
        .await
        .unwrap();
    assert!(
        !vu(&avant),
        "précondition : fil sans message remis, invisible de la famille"
    );
    ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), premier)
        .await
        .expect("annulation");
    let apres = ent
        .list_fils_for_actor(etab, lily, role(Role::Parent), None, false, None)
        .await
        .unwrap();
    // CONSTAT (F-6, confirmé) : `ent_msg_fil_lisible` (485) garde un fil lisible de la famille tant qu'il n'a **aucun**
    // message (branche `NOT EXISTS`), donc le fil vidé par l'annulation de son seul message reste listé avec son sujet.
    // Correctif proposé (migration 493, hors périmètre sûr de la passe 2 : vue partagée par toute la messagerie) : ne
    // garder cette branche que pour le côté établissement. Quand elle sera corrigée, inverser cette assertion.
    assert!(
        vu(&apres),
        "constat F-6 : le fil vide reste listé (voir le commentaire)"
    );
}

// ───────────── E-1 (contre-revue de la passe 2, correctif du pilote) : seules les sources « message » sont contrôlées ─────────────

/// Avis planifié (dans deux heures) de clé et de genre donnés, dont la source est `source`.
async fn avis_de(ent: &PgEnt, etab: Uuid, user: Uuid, kind: &str, source: Uuid, cle: String) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_notification_envoi
            (id, etablissement_id, destinataire_user_id, canal, kind, titre, corps, eleve_id,
             source_type, source_id, cle_dedup, droit_requis, planifie_pour)
         VALUES ($1,$2,$3,'push',$4,'Nouveau message','Ouvrez le portail famille',NULL,'systeme',$5,$6,NULL,
                 now() + interval '2 hours')",
    )
    .bind(id)
    .bind(etab)
    .bind(user)
    .bind(kind)
    .bind(source)
    .bind(cle)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}

/// La relance « réponse attendue » (`msgreponse:`, source = un **fil**) n'est jamais écartée comme « message annulé » :
/// le distributeur la traite. Témoin : un avis `msgfam:` dont le message n'existe pas est, lui, écarté.
#[tokio::test]
async fn p2_e1_la_relance_reponse_attendue_n_est_pas_ecartee_comme_message_annule() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lily) = (cast.etablissement_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Relance réponse attendue").await;
    let relance = avis_de(&ent, etab, lily, "messagerie_famille", fil, format!("msgreponse:{fil}:{lily}")).await;
    let absent = Uuid::new_v4();
    let temoin = avis_de(&ent, etab, lily, "messagerie_famille", absent, format!("msgfam:{absent}")).await;
    let apres = apres_l_echeance(&ent).await;
    expediteur(&ent, etab).expedier(apres).await.unwrap();
    let (statut, motif) = statut_avis(&ent, relance).await;
    assert_ne!(statut, "en_attente", "la relance a été traitée par le distributeur");
    assert_ne!(motif.as_deref(), Some("message_annule"), "la source d'une relance est un fil, pas un message : {statut}");
    assert_eq!(
        statut_avis(&ent, temoin).await,
        ("ignore".to_string(), Some("message_annule".to_string())),
        "témoin : un avis de message dont le message n'existe plus est écarté"
    );
}
