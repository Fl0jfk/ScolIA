//! Socle 3, lot I5 — **suites, passe 7** (migration 318), contre la base locale de test (feature `labo`).
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés en local.
//!
//! Aucune assertion ne dépend de l'heure réelle : les instants passés au service sont posés (2027) et les dates stockées sont
//! fixées très loin du présent (2020).
//!
//! | Sujet | Test(s) |
//! |---|---|
//! | Moyen D : sentinelle par instruction, offre du dossier par jointure, **fonction réelle**, deux offres | `s7_sentinelle_multi_lignes_sans_offre_id_deux_offres` |
//! | Moyen C : un rejeu en échec ne laisse pas la sentinelle absente | `s7_rejeu_en_echec_restaure_la_sentinelle` |
//! | Moyen A : vérification de santé (`tgenabled <> 'A'`) | `s7_sante_sentinelle_signale_un_declencheur_non_always` |
//! | Faible 2 : repli de l'alerte par le droit `InscriptionsGerer` | `s7_alerte_repli_par_le_droit_et_non_par_le_role` |

use ent_types::inscriptions::*;
use uuid::Uuid;

use super::tests_inscriptions::{dir, labo};
use super::tests_inscriptions_i5::{campagne_cap, corps, dossier, maintenant};
use super::tests_inscriptions_i5_suites6::{appels, membre};
use super::*;

const SENTINELLE: &str = "ent_inscription_verrou_offre_manquant";

fn code(r: &Result<(), sqlx::Error>) -> String {
    match r {
        Err(sqlx::Error::Database(d)) => d.code().map(|c| c.to_string()).unwrap_or_default(),
        Err(autre) => format!("autre erreur : {autre}"),
        Ok(_) => "aucune erreur".to_string(),
    }
}

fn refus(r: &Result<(), sqlx::Error>, quoi: &str) {
    assert_eq!(code(r), "55000", "{quoi} : refus attendu de la sentinelle");
    let msg = r.as_ref().unwrap_err().to_string();
    assert!(
        msg.contains(SENTINELLE),
        "{quoi} : l'erreur doit être nommée ({msg})"
    );
}

/// Un dossier en liste d'attente de `offre` (`offre_attente_id`), sans vœu retenu : son offre se lit par la jointure.
async fn dossier_en_liste(ent: &PgEnt, etab: Uuid, camp: Uuid, offre: Uuid, nom: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO ent_inscription_dossier
            (id, etablissement_id, campagne_id, nature, eleve_nom, eleve_prenom, etat, ordre_attente, offre_attente_id, reference)
         VALUES (gen_random_uuid(), $1, $2, 'admission', $4, 'Liste', 'liste_attente', 1, $3, 'S7-' || gen_random_uuid()::text)
         RETURNING id",
    )
    .bind(etab)
    .bind(camp)
    .bind(offre)
    .bind(nom)
    .fetch_one(ent.pool_verif())
    .await
    .expect("dossier en liste d'attente")
}

// ───────────────────────────── Moyen D : sentinelle par instruction, fonction réelle ─────────────────────────────

/// **Moyen D** : des lignes dont `offre_id` est NULL (l'offre est celle de la liste du dossier) sur **deux offres**, écrites
/// par une seule instruction. Les déclencheurs sont ceux de la **fonction réelle** `ent_inscription_sentinelle_instruction`
/// (posés sur une table temporaire de mêmes colonnes) : le verrou est exigé pour **chaque** offre (aucun, l'un, l'autre : refus ;
/// les deux : accepté), pour une mise à jour (ancienne **et** nouvelle offre) comme pour une suppression. Compte d'appels :
/// `ent_inscription_offre_du_dossier` n'est **jamais** appelée (jointure), et la vérification n'a lieu qu'**une fois par offre
/// distincte** ; **preuve rouge** : le déclencheur par ligne de 316 (`ent_inscription_sentinelle_ligne`) l'appelle pour chaque ligne.
#[tokio::test]
async fn s7_sentinelle_multi_lignes_sans_offre_id_deux_offres() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp1, o1) = campagne_cap(&ent, &c, Some(3), "2027-2028").await;
    let (camp2, o2) = campagne_cap(&ent, &c, Some(3), "2028-2029").await;
    let d1 = dossier_en_liste(&ent, etab, camp1, o1, "SeptUn").await;
    let d2 = dossier_en_liste(&ent, etab, camp2, o2, "SeptDeux").await;
    let garde = crate::migrate::garde_rejeu_partagee(ent.pool_verif()).await;

    let mut conn = ent.pool_verif().acquire().await.unwrap().detach();
    for ordre in [
        "CREATE TEMP TABLE s7_lignes (id int GENERATED ALWAYS AS IDENTITY, etablissement_id uuid NOT NULL,
            dossier_id uuid NOT NULL, offre_id uuid, v int NOT NULL DEFAULT 0)",
        "CREATE TRIGGER s7_ins AFTER INSERT ON s7_lignes REFERENCING NEW TABLE AS nouv
            FOR EACH STATEMENT EXECUTE FUNCTION public.ent_inscription_sentinelle_instruction()",
        "CREATE TRIGGER s7_upd AFTER UPDATE ON s7_lignes REFERENCING OLD TABLE AS anc NEW TABLE AS nouv
            FOR EACH STATEMENT EXECUTE FUNCTION public.ent_inscription_sentinelle_instruction()",
        "CREATE TRIGGER s7_del AFTER DELETE ON s7_lignes REFERENCING OLD TABLE AS anc
            FOR EACH STATEMENT EXECUTE FUNCTION public.ent_inscription_sentinelle_instruction()",
    ] {
        sqlx::query(ordre).execute(&mut conn).await.unwrap_or_else(|e| panic!("{ordre} : {e}"));
    }

    // Insertion de 400 lignes par dossier (800 lignes, deux offres), toutes sans `offre_id`.
    let inserer = "INSERT INTO s7_lignes (etablissement_id, dossier_id)
                   SELECT $1, d FROM unnest($2::uuid[]) d, generate_series(1, 400)";
    for (verrous, ok, quoi) in [
        (vec![], false, "aucun verrou"),
        (vec![o1], false, "le verrou de la première offre seulement"),
        (vec![o2], false, "le verrou de la seconde offre seulement"),
        (vec![o1, o2], true, "les deux verrous"),
    ] {
        let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
        for o in &verrous {
            sqlx::query("SELECT pg_advisory_xact_lock(ent_inscription_offre_cle($1))")
                .bind(o)
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        let r = sqlx::query(inserer)
            .bind(etab)
            .bind(vec![d1, d2])
            .execute(&mut *tx)
            .await
            .map(|_| ());
        tx.rollback().await.unwrap();
        if ok {
            r.unwrap_or_else(|e| panic!("{quoi} : doit passer ({e})"));
        } else {
            refus(&r, quoi);
        }
    }

    // Lignes en place (insérées sous les deux verrous, validées), puis mise à jour, suppression et compte d'appels.
    let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
    for o in [o1, o2] {
        sqlx::query("SELECT pg_advisory_xact_lock(ent_inscription_offre_cle($1))")
            .bind(o)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    sqlx::query(inserer)
        .bind(etab)
        .bind(vec![d1, d2])
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    for (verrous, ok, quoi) in [
        (vec![o1], false, "mise à jour : l'offre 2 manque"),
        (vec![o2], false, "mise à jour : l'offre 1 manque"),
        (vec![o1, o2], true, "mise à jour : les deux verrous"),
    ] {
        let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
        for o in &verrous {
            sqlx::query("SELECT pg_advisory_xact_lock(ent_inscription_offre_cle($1))")
                .bind(o)
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        let r = sqlx::query("UPDATE s7_lignes SET v = v + 1")
            .execute(&mut *tx)
            .await
            .map(|_| ());
        tx.rollback().await.unwrap();
        if ok {
            r.unwrap_or_else(|e| panic!("{quoi} : {e}"));
        } else {
            refus(&r, quoi);
        }
    }
    // Une ligne qui change de dossier (donc d'offre) exige l'ancienne ET la nouvelle offre.
    for (verrous, ok, quoi) in [
        (
            vec![o1],
            false,
            "changement de dossier : l'offre d'arrivée manque",
        ),
        (
            vec![o2],
            false,
            "changement de dossier : l'offre de départ manque",
        ),
        (
            vec![o1, o2],
            true,
            "changement de dossier : les deux verrous",
        ),
    ] {
        let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
        for o in &verrous {
            sqlx::query("SELECT pg_advisory_xact_lock(ent_inscription_offre_cle($1))")
                .bind(o)
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        let r = sqlx::query("UPDATE s7_lignes SET dossier_id = $1 WHERE dossier_id = $2 AND id = (SELECT min(id) FROM s7_lignes WHERE dossier_id = $2)")
            .bind(d1)
            .bind(d2)
            .execute(&mut *tx)
            .await
            .map(|_| ());
        tx.rollback().await.unwrap();
        if ok {
            r.unwrap_or_else(|e| panic!("{quoi} : {e}"));
        } else {
            refus(&r, quoi);
        }
    }
    for (verrous, ok, quoi) in [
        (vec![o1], false, "suppression : l'offre 2 manque"),
        (vec![o1, o2], true, "suppression : les deux verrous"),
    ] {
        let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
        for o in &verrous {
            sqlx::query("SELECT pg_advisory_xact_lock(ent_inscription_offre_cle($1))")
                .bind(o)
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        let r = sqlx::query("DELETE FROM s7_lignes")
            .execute(&mut *tx)
            .await
            .map(|_| ());
        tx.rollback().await.unwrap();
        if ok {
            r.unwrap_or_else(|e| panic!("{quoi} : {e}"));
        } else {
            refus(&r, quoi);
        }
    }

    // Compte d'appels : 800 lignes mises à jour sous les deux verrous.
    let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
    sqlx::query("SET LOCAL track_functions = 'all'")
        .execute(&mut *tx)
        .await
        .unwrap();
    for o in [o1, o2] {
        sqlx::query("SELECT pg_advisory_xact_lock(ent_inscription_offre_cle($1))")
            .bind(o)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE s7_lignes SET v = v + 1")
        .execute(&mut *tx)
        .await
        .unwrap();
    assert_eq!(
        appels(&mut tx, "ent_inscription_offre_du_dossier").await,
        0,
        "l'offre du dossier se lit par jointure : aucun appel par ligne"
    );
    // La vérification se fait par offre distincte : 2 offres d'après + 2 d'avant.
    assert_eq!(
        appels(&mut tx, "ent_inscription_exiger_verrou_offre").await,
        4,
        "une vérification par offre distincte (deux offres, ancienne et nouvelle)"
    );
    tx.rollback().await.unwrap();

    // Preuve rouge : le déclencheur par ligne de 316 appelle la fonction pour chaque ligne sans `offre_id`.
    sqlx::query("DROP TRIGGER s7_ins ON s7_lignes")
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("DROP TRIGGER s7_upd ON s7_lignes")
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("DROP TRIGGER s7_del ON s7_lignes")
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER s7_ligne AFTER UPDATE ON s7_lignes FOR EACH ROW EXECUTE FUNCTION public.ent_inscription_sentinelle_ligne()")
        .execute(&mut conn)
        .await
        .unwrap();
    let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
    sqlx::query("SET LOCAL track_functions = 'all'")
        .execute(&mut *tx)
        .await
        .unwrap();
    for o in [o1, o2] {
        sqlx::query("SELECT pg_advisory_xact_lock(ent_inscription_offre_cle($1))")
            .bind(o)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE s7_lignes SET v = v + 1")
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(
        appels(&mut tx, "ent_inscription_offre_du_dossier").await >= 800,
        "preuve rouge : par ligne, l'offre du dossier est appelée pour chacune des 800 lignes"
    );
    tx.rollback().await.unwrap();
    drop(conn);
    garde.liberer().await;
}

// ───────────────────────────── Moyen C : un rejeu en échec ─────────────────────────────

async fn etats_sentinelle(ent: &PgEnt) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT tgname::text, tgenabled::text FROM pg_trigger
          WHERE tgname LIKE 'ent\\_inscription\\_sentinelle\\_%' AND NOT tgisinternal ORDER BY tgname",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap()
}

/// **Moyen C** : la garde de 304 retire la sentinelle des décisions ; si un fichier **plus loin** dans le même rejeu échoue,
/// la sentinelle ne reste pas absente : elle est recréée (316, 317, 318), l'erreur d'origine est rendue, et une écriture sans
/// verrou est de nouveau refusée.
#[tokio::test]
async fn s7_rejeu_en_echec_restaure_la_sentinelle() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (_camp, offre) = campagne_cap(&ent, &c, Some(3), "2027-2028").await;
    // Rejeu partiel : le seul fichier à garde (304) puis un fichier qui échoue. Pas le préfixe entier du tableau : un rejeu
    // interrompu laisserait les objets des fichiers rejoués dans leur état d'alors pour les tests suivants de la base.
    // Contre-revue du pilote (passe 7, T-3) : la garde de rejeu se déclenche par le NOM du fichier ; son SQL est neutre, la base
    // n'est jamais remise dans un état ancien (ni rejeu de 304, ni rejeu complet de remise en état).
    let liste: Vec<(&str, &str)> = vec![
        ("304_inscription_demarche_dossier", "SELECT 1"),
        ("9999_echec_voulu_du_rejeu", "SELECT 1 / 0"),
    ];

    let r = crate::migrate::rejouer_liste(ent.pool_verif(), &liste).await;
    let e = r.expect_err("le rejeu doit échouer sur le fichier qui échoue");
    assert!(
        e.to_string().contains("division"),
        "l'erreur d'origine est rendue : {e}"
    );

    let garde = crate::migrate::garde_rejeu_partagee(ent.pool_verif()).await;
    let etats = etats_sentinelle(&ent).await;
    assert_eq!(
        etats.len(),
        7,
        "la sentinelle est de retour malgré l'échec : {etats:?}"
    );
    assert!(
        etats.iter().all(|(_, e)| e == "A"),
        "tous en ENABLE ALWAYS : {etats:?}"
    );
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let r =
        sqlx::query("UPDATE ent_inscription_offre SET liberations = liberations + 1 WHERE id = $1")
            .bind(offre)
            .execute(&mut *tx)
            .await
            .map(|_| ());
    tx.rollback().await.unwrap();
    refus(&r, "écriture sans verrou après un rejeu en échec");
    garde.liberer().await;
}

// ───────────────────────────── Moyen A : vérification de santé ─────────────────────────────

/// **Moyen A** : la vérification de santé du démarrage rend (et journalise en erreur) les déclencheurs de la sentinelle qui ne sont
/// pas `ENABLE ALWAYS` : aucun en état normal ; un déclencheur remis à « origine » (ce que fait `pg_restore --disable-triggers`)
/// ou suspendu est signalé ; le rétablissement la vide de nouveau. Elle ne bloque rien.
#[tokio::test]
async fn s7_sante_sentinelle_signale_un_declencheur_non_always() {
    let Some((ent, _c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let garde = crate::migrate::garde_rejeu_partagee(ent.pool_verif()).await;
    // La suspension et le rétablissement sont **dans une transaction annulée** : aucune autre session ne voit l'état suspendu ;
    // la vérification, elle, passe par la même transaction (connexion unique).
    let mut tx = ent.pool_verif().begin().await.unwrap();
    // T3 : aucun `ALTER TABLE` hors `lock_timeout` — si un test voisin tient un verrou sur ces tables, celui-ci échoue vite
    // (et lisiblement) au lieu d'attendre indéfiniment, et il ne bloque pas non plus les sessions des autres tests plus de
    // 15 s (`SET LOCAL` : valable pour cette seule transaction, annulée en fin de test ; 15 s depuis la contre-revue T3, M2,
    // commentaire mis à jour au palier des faibles, lot 4).
    sqlx::query("SET LOCAL lock_timeout = '15s'").execute(&mut *tx).await.unwrap();
    sqlx::query("ALTER TABLE ent_inscription_decision ENABLE TRIGGER ent_inscription_sentinelle_decision_ins_trg")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE ent_inscription_offre DISABLE TRIGGER ent_inscription_sentinelle_offre_upd_trg")
        .execute(&mut *tx)
        .await
        .unwrap();
    // Contre-revue du pilote (passe 7) : la FONCTION du démarrage elle-même, sur la transaction ; un déclencheur absent est
    // signalé aussi.
    sqlx::query("DROP TRIGGER ent_inscription_sentinelle_proposition_del_trg ON ent_inscription_proposition_place")
        .execute(&mut *tx)
        .await
        .unwrap();
    let anormaux = crate::migrate::sentinelles_non_toujours_actives(&mut *tx).await.unwrap();
    assert_eq!(
        anormaux,
        vec![
            "ent_inscription_sentinelle_decision_ins_trg".to_string(),
            "ent_inscription_sentinelle_offre_upd_trg".to_string(),
            "ent_inscription_sentinelle_proposition_del_trg".to_string(),
        ],
        "état « origine » (O), suspendu (D) et absent"
    );
    tx.rollback().await.unwrap();

    // État normal, hors transaction : la fonction du démarrage ne rend rien.
    let sains = crate::migrate::sentinelles_non_toujours_actives(ent.pool_verif())
        .await
        .unwrap();
    assert!(
        sains.is_empty(),
        "en état normal, aucun déclencheur à signaler : {sains:?}"
    );
    garde.liberer().await;
}

// ───────────────────────────── Faible 2 : repli de l'alerte par le droit ─────────────────────────────

/// **Faible 2** : le repli de l'alerte de décision caduque suit le **droit** `InscriptionsGerer`, pas un nom de rôle. Un CPE
/// (qui n'a pas ce droit) n'est pas alerté ; un autre CPE à qui une **attribution de rôle** donne le droit l'est ; un compte dont
/// l'attribution est échue (2020) ne l'est pas. T3 : un compte **famille** à qui une attribution de rôle donne le même droit
/// n'est pas alerté non plus (garde-fou « rôle de base personnel » : le repli n'alerte que le personnel).
#[tokio::test]
async fn s7_alerte_repli_par_le_droit_et_non_par_le_role() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, _) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let dos = dossier(&ent, &c, camp, "RepliDroit", None).await;
    let mut b = corps("ajourne", None, "Ajournement programmé");
    b.publier_le = Some(maintenant() + chrono::Duration::days(7));
    let d = ent
        .decider_complete_for_actor(etab, u, r, dos.id, &b, false, maintenant())
        .await
        .unwrap();
    ent.transition_for_actor(
        etab,
        u,
        r,
        dos.id,
        &TransitionBody {
            action: "annuler".into(),
            motif: "Annulation demandée".into(),
        },
    )
    .await
    .unwrap();

    let sans_droit = membre(&ent, etab, "cpe").await;
    let avec_droit = membre(&ent, etab, "cpe").await;
    let echu = membre(&ent, etab, "cpe").await;
    let famille = membre(&ent, etab, "parent").await;
    for (user, fin) in [(avec_droit, None), (echu, Some("2020-12-31")), (famille, None)] {
        let role = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_role (id, etablissement_id, nom) VALUES ($1, $2, $3)")
            .bind(role)
            .bind(etab)
            .bind(format!("instructeur-{}", role.simple()))
            .execute(ent.pool_verif())
            .await
            .unwrap();
        sqlx::query("INSERT INTO ent_role_droit (role_id, etablissement_id, action, portee) VALUES ($1, $2, 'inscriptions.gerer', 'etablissement')")
            .bind(role)
            .bind(etab)
            .execute(ent.pool_verif())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO ent_role_attribution (role_id, etablissement_id, user_id, debut, fin) VALUES ($1, $2, $3, '2020-01-01', $4::date)",
        )
        .bind(role)
        .bind(etab)
        .bind(user)
        .bind(fin)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    }
    // Direction et auteur partis ; les trois CPE ont un accès ouvert.
    // La garde « dernier administrateur » est levée **dans la transaction** (fixture, comme `fermer_acces`).
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SET LOCAL ent.purge_etablissement = 'on'")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_membership SET date_fin = '2020-01-01' WHERE etablissement_id = $1 AND NOT (user_id = ANY($2))")
        .bind(etab)
        .bind(vec![sans_droit, avec_droit, echu, famille])
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    ent.publier_decisions_echues_tache(maintenant() + chrono::Duration::days(7))
        .await
        .unwrap();
    let nb = |user: Uuid| {
        let pool = ent.pool_verif().clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM ent_notification WHERE source_event_id = $1 AND destinataire_user_id = $2
                    AND kind = 'inscription_decision_caduque'",
            )
            .bind(d.id)
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    assert_eq!(
        nb(avec_droit).await,
        1,
        "le détenteur du droit (par attribution) est alerté"
    );
    assert_eq!(
        nb(sans_droit).await,
        0,
        "un CPE sans le droit n'est pas alerté"
    );
    assert_eq!(
        nb(echu).await,
        0,
        "une attribution échue ne donne plus le droit"
    );
    assert_eq!(
        nb(famille).await,
        0,
        "une famille ne reçoit jamais cette alerte, même avec une attribution qui porte le droit (repli réservé au personnel)"
    );
}
