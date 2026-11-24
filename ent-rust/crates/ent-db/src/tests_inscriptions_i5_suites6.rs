//! Socle 3, lot I5 — **suites, passe 6** (migration 317), contre la base locale de test (feature `labo`).
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés en local.
//!
//! Les instants passés au **service** sont posés (2027) ; l'horloge de la base (`now()`, jour courant de `ent_acces_ouvert`) reste lue
//! par quelques écritures (horodatages de ligne, accès ouverts), mais aucune assertion ne la compare à un instant de test : les dates
//! qui en dépendent sont fixées très loin du présent (2020, 2099). Voir aussi la règle 33 de PILOTAGE. Les mesures de temps de
//! `s6_mesure_surcout_sentinelle` sont **imprimées**, jamais comparées.
//!
//! | Sujet | Test(s) |
//! |---|---|
//! | M2 rejeu de 304 sur une base qui contient un dossier admis sans décision | `s6_rejeu_de_304_dossier_admis_sans_decision` |
//! | F1 `ENABLE ALWAYS` + procédure de restauration (suspension explicite) | `s6_sentinelle_toujours_active_et_restauration` |
//! | M4 sentinelle par instruction : une vérification par offre distincte | `s6_sentinelle_par_instruction_exige_chaque_offre`, `s6_mesure_surcout_sentinelle` |
//! | F3 liste d'attente sur 10 000 dossiers : pas d'appel par dossier de l'établissement | `s6_liste_attente_borne_d_appels_sur_10000_dossiers` |
//! | F6 alerte sans destinataire : repli sur l'administration, puis trace | `s6_alerte_sans_destinataire_repli_puis_trace` |
//! | F7 archivage d'offre sous le verrou de l'offre | `s6_archivage_d_offre_attend_le_verrou`, `s6_archivage_lit_les_voeux_sous_le_verrou` |

use std::sync::Arc;
use std::time::Instant;

use ent_types::inscriptions::*;
use uuid::Uuid;

use super::tests_inscriptions::{dir, labo};
use super::tests_inscriptions_i5::{campagne_cap, corps, dossier, maintenant};
use super::tests_inscriptions_i5_passe4::{attendre_attentes, issue, tenir};
use super::*;

const SENTINELLE: &str = "ent_inscription_verrou_offre_manquant";

fn code(r: &Result<(), sqlx::Error>) -> String {
    match r {
        Err(sqlx::Error::Database(d)) => d.code().map(|c| c.to_string()).unwrap_or_default(),
        Err(autre) => format!("autre erreur : {autre}"),
        Ok(_) => "aucune erreur".to_string(),
    }
}

/// Refus nommé de la sentinelle : SQLSTATE `55000` **et** nom de l'erreur dans le message.
fn refus(r: &Result<(), sqlx::Error>, quoi: &str) {
    assert_eq!(code(r), "55000", "{quoi} : refus attendu de la sentinelle");
    let msg = r.as_ref().unwrap_err().to_string();
    assert!(
        msg.contains(SENTINELLE),
        "{quoi} : l'erreur doit être nommée ({msg})"
    );
}

// ───────────────────────────── M2 : rejeu de 304 ─────────────────────────────

/// **M2** : le rejeu de tous les fichiers (reprise d'anciennes données) sur une base qui contient un dossier **admis sans
/// décision** ne bute plus sur la sentinelle : la reprise de 304 (`INSERT INTO ent_inscription_decision … SELECT`) ne tient
/// aucun verrou d'offre. La garde de rejeu retire les déclencheurs de la sentinelle avant 304 ; 316 et 317 les recréent
/// ensuite : à la fin du rejeu, la reprise a posé la décision **et** la sentinelle est de nouveau active (`ENABLE ALWAYS`).
#[tokio::test]
async fn s6_rejeu_de_304_dossier_admis_sans_decision() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (camp, offre) = campagne_cap(&ent, &c, Some(3), "2027-2028").await;
    let d = dossier(&ent, &c, camp, "RejeuAdmis", None).await;
    // État hérité : un dossier admis (colonnes `statut`, `decide_le`, `decision_motif` d'avant les décisions) sans ligne de décision.
    let r = sqlx::query(
        "UPDATE ent_inscription_dossier
            SET statut = 'admis', decide_le = '2027-03-01T10:00:00Z', decision_motif = 'Admis avant la reprise'
          WHERE id = $1",
    )
    .bind(d.id)
    .execute(ent.pool_verif())
    .await;
    r.expect("le dossier hérité (admis, sans décision) se pose");
    let avant: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_inscription_decision WHERE dossier_id = $1")
            .bind(d.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(avant, 0, "fixture : aucun enregistrement de décision");

    crate::migrate::rejouer_tout(ent.pool_verif())
        .await
        .expect("le rejeu passe sur une base qui contient un dossier admis sans décision");

    let (n, sens): (i64, Option<String>) = sqlx::query_as(
        "SELECT count(*), min(sens) FROM ent_inscription_decision WHERE dossier_id = $1",
    )
    .bind(d.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (n, sens.as_deref()),
        (1, Some("admis")),
        "la reprise a posé la décision du dossier admis"
    );

    // La sentinelle est revenue : actifs `ALWAYS`, aucun déclencheur par ligne de 316 ne subsiste, écriture sans verrou refusée.
    let garde = crate::migrate::garde_rejeu_partagee(ent.pool_verif()).await;
    let etats: Vec<(String, String)> = sqlx::query_as(
        "SELECT tgname::text, tgenabled::text FROM pg_trigger
          WHERE tgname LIKE 'ent\\_inscription\\_sentinelle\\_%' AND NOT tgisinternal ORDER BY tgname",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        etats.len(),
        7,
        "sept déclencheurs de sentinelle : {etats:?}"
    );
    assert!(
        etats.iter().all(|(_, e)| e == "A"),
        "tous en ENABLE ALWAYS : {etats:?}"
    );
    assert!(
        etats.iter().all(|(n, _)| n.ends_with("_ins_trg")
            || n.ends_with("_upd_trg")
            || n.ends_with("_del_trg")),
        "plus de déclencheur par ligne : {etats:?}"
    );
    let r = ecrire_sans_verrou(&ent, offre).await;
    refus(&r, "écriture sans verrou après le rejeu");
    garde.liberer().await;
}

async fn ecrire_sans_verrou(ent: &PgEnt, offre: Uuid) -> Result<(), sqlx::Error> {
    let mut tx = ent.pool_verif().begin().await?;
    let r =
        sqlx::query("UPDATE ent_inscription_offre SET liberations = liberations + 1 WHERE id = $1")
            .bind(offre)
            .execute(&mut *tx)
            .await
            .map(|_| ());
    tx.rollback().await?;
    r
}

// ───────────────────────────── F1 : ENABLE ALWAYS, procédure de restauration ─────────────────────────────

/// **F1** : la sentinelle se coupe **explicitement** (`DISABLE TRIGGER`), pas par `session_replication_role = replica`. C'est un
/// filet contre l'oubli du verrou dans le code, pas un contrôle de sécurité : le propriétaire de la table peut la suspendre le
/// temps d'une restauration de données, et la procédure rétablit `ENABLE ALWAYS`.
#[tokio::test]
async fn s6_sentinelle_toujours_active_et_restauration() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (_camp, offre) = campagne_cap(&ent, &c, Some(3), "2027-2028").await;
    let garde = crate::migrate::garde_rejeu_partagee(ent.pool_verif()).await;

    // `ENABLE ALWAYS` sur les sept déclencheurs.
    let etats: Vec<(String, String)> = sqlx::query_as(
        "SELECT tgname::text, tgenabled::text FROM pg_trigger
          WHERE tgname LIKE 'ent\\_inscription\\_sentinelle\\_%' AND NOT tgisinternal ORDER BY tgname",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(etats.len(), 7, "{etats:?}");
    assert!(etats.iter().all(|(_, e)| e == "A"), "{etats:?}");

    // `session_replication_role = replica` ne la contourne plus.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    let r =
        sqlx::query("UPDATE ent_inscription_offre SET liberations = liberations + 1 WHERE id = $1")
            .bind(offre)
            .execute(&mut *tx)
            .await
            .map(|_| ());
    tx.rollback().await.unwrap();
    refus(&r, "écriture sous session_replication_role = replica");

    // Procédure de restauration : suspension explicite, reprise sans verrou, rétablissement `ALWAYS`, vérification.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("ALTER TABLE ent_inscription_offre DISABLE TRIGGER ent_inscription_sentinelle_offre_upd_trg")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_inscription_offre SET liberations = liberations + 1 WHERE id = $1")
        .bind(offre)
        .execute(&mut *tx)
        .await
        .expect("sentinelle suspendue explicitement : la reprise passe sans verrou");
    sqlx::query("ALTER TABLE ent_inscription_offre ENABLE ALWAYS TRIGGER ent_inscription_sentinelle_offre_upd_trg")
        .execute(&mut *tx)
        .await
        .unwrap();
    let etat: String = sqlx::query_scalar(
        "SELECT tgenabled::text FROM pg_trigger WHERE tgname = 'ent_inscription_sentinelle_offre_upd_trg'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(etat, "A", "la procédure rétablit ENABLE ALWAYS");
    let r =
        sqlx::query("UPDATE ent_inscription_offre SET liberations = liberations + 1 WHERE id = $1")
            .bind(offre)
            .execute(&mut *tx)
            .await
            .map(|_| ());
    tx.rollback().await.unwrap();
    refus(&r, "après le rétablissement");
    garde.liberer().await;
}

// ───────────────────────────── M4 : sentinelle par instruction ─────────────────────────────

/// **M4** : une instruction qui écrit les places de **deux offres** exige le verrou des deux (une vérification par offre
/// distincte, plus par ligne) : avec un seul, elle est refusée ; avec les deux, elle passe.
#[tokio::test]
async fn s6_sentinelle_par_instruction_exige_chaque_offre() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (_c1, o1) = campagne_cap(&ent, &c, Some(3), "2027-2028").await;
    let (_c2, o2) = campagne_cap(&ent, &c, Some(3), "2028-2029").await;
    let garde = crate::migrate::garde_rejeu_partagee(ent.pool_verif()).await;
    let masse = "UPDATE ent_inscription_offre SET liberations = liberations + 1 WHERE id = ANY($1)";
    let ids = vec![o1, o2];
    for (verrous, attendu_ok, quoi) in [
        (vec![], false, "aucun verrou"),
        (vec![o1], false, "le verrou de la première seulement"),
        (vec![o2], false, "le verrou de la seconde seulement"),
        (vec![o1, o2], true, "les deux verrous"),
    ] {
        let mut tx = ent.pool_verif().begin().await.unwrap();
        for o in &verrous {
            sqlx::query("SELECT pg_advisory_xact_lock(ent_inscription_offre_cle($1))")
                .bind(o)
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        let r = sqlx::query(masse)
            .bind(&ids)
            .execute(&mut *tx)
            .await
            .map(|_| ());
        tx.rollback().await.unwrap();
        if attendu_ok {
            r.unwrap_or_else(|e| panic!("{quoi} : l'instruction doit passer ({e})"));
        } else {
            refus(&r, quoi);
        }
    }
    garde.liberer().await;
}

/// **M4 (mesure reproductible)** : surcoût de la vérification, **par ligne** (déclencheur `FOR EACH ROW`, 316) contre **par
/// instruction** (déclencheur à table de transition, 317), sur une table temporaire qui appelle la **même** fonction
/// `ent_inscription_exiger_verrou_offre`. Variables : lignes par instruction (1, 100, 1000) et verrous consultatifs d'une
/// autre session (0, 500). Les temps (médiane de 5 passes) sont **imprimés** (`--nocapture`), jamais assertés ; l'assertion
/// porte sur la correction (les deux variantes refusent sans verrou, acceptent avec).
#[tokio::test]
async fn s6_mesure_surcout_sentinelle() {
    let Some((ent, _c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let offre = Uuid::new_v4();
    let cle: i64 = sqlx::query_scalar("SELECT ent_inscription_offre_cle($1)")
        .bind(offre)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    // Verrous d'une autre session : 0 puis 500 (clés sans rapport avec l'offre).
    let mut autre = ent.pool_verif().acquire().await.unwrap().detach();
    let mut conn = ent.pool_verif().acquire().await.unwrap().detach();
    for ordre in [
        "CREATE TEMP TABLE bench_sentinelle (id int GENERATED ALWAYS AS IDENTITY, offre_id uuid NOT NULL, v int NOT NULL DEFAULT 0)",
        "CREATE FUNCTION pg_temp.bench_ligne() RETURNS trigger LANGUAGE plpgsql
            SET search_path = pg_catalog, public, pg_temp AS $$
            BEGIN PERFORM public.ent_inscription_exiger_verrou_offre(NEW.offre_id, 'bench'); RETURN NEW; END $$",
        "CREATE FUNCTION pg_temp.bench_instruction() RETURNS trigger LANGUAGE plpgsql
            SET search_path = pg_catalog, public, pg_temp AS $$
            DECLARE o uuid;
            BEGIN
                FOR o IN SELECT DISTINCT n.offre_id FROM nouv n ORDER BY 1 LOOP
                    PERFORM public.ent_inscription_exiger_verrou_offre(o, 'bench');
                END LOOP;
                RETURN NULL;
            END $$",
        "CREATE TRIGGER bench_ligne_trg AFTER UPDATE ON bench_sentinelle FOR EACH ROW EXECUTE FUNCTION pg_temp.bench_ligne()",
        "CREATE TRIGGER bench_instruction_trg AFTER UPDATE ON bench_sentinelle REFERENCING NEW TABLE AS nouv
            FOR EACH STATEMENT EXECUTE FUNCTION pg_temp.bench_instruction()",
    ] {
        sqlx::query(ordre).execute(&mut conn).await.unwrap_or_else(|e| panic!("{ordre} : {e}"));
    }
    sqlx::query("INSERT INTO bench_sentinelle (offre_id) SELECT $1 FROM generate_series(1, 1000)")
        .bind(offre)
        .execute(&mut conn)
        .await
        .unwrap();

    // Correction : sans verrou, les deux variantes refusent ; avec le verrou, elles acceptent.
    for variante in ["bench_ligne_trg", "bench_instruction_trg"] {
        sqlx::query("ALTER TABLE bench_sentinelle DISABLE TRIGGER bench_ligne_trg")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("ALTER TABLE bench_sentinelle DISABLE TRIGGER bench_instruction_trg")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query(&format!(
            "ALTER TABLE bench_sentinelle ENABLE TRIGGER {variante}"
        ))
        .execute(&mut conn)
        .await
        .unwrap();
        let r = sqlx::query("UPDATE bench_sentinelle SET v = v + 1 WHERE id <= 10")
            .execute(&mut conn)
            .await
            .map(|_| ());
        refus(&r, &format!("{variante} sans verrou"));
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(cle)
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("UPDATE bench_sentinelle SET v = v + 1 WHERE id <= 10")
            .execute(&mut conn)
            .await
            .unwrap_or_else(|e| panic!("{variante} avec le verrou : {e}"));
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(cle)
            .execute(&mut conn)
            .await
            .unwrap();
    }

    // Mesure : verrou de l'offre tenu (session), N lignes par instruction, médiane de 5 passes (µs par instruction).
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(cle)
        .execute(&mut conn)
        .await
        .unwrap();
    eprintln!("M4 mesure (µs par instruction ; médiane de 5) — variante / lignes par instruction / verrous d'une autre session");
    for autres in [0i64, 500] {
        if autres > 0 {
            sqlx::query("SELECT count(pg_advisory_lock(g)) FROM generate_series(1, $1) g")
                .bind(autres)
                .execute(&mut autre)
                .await
                .unwrap();
        }
        for lignes in [1i64, 100, 1000] {
            for variante in ["bench_ligne_trg", "bench_instruction_trg"] {
                sqlx::query("ALTER TABLE bench_sentinelle DISABLE TRIGGER bench_ligne_trg")
                    .execute(&mut conn)
                    .await
                    .unwrap();
                sqlx::query("ALTER TABLE bench_sentinelle DISABLE TRIGGER bench_instruction_trg")
                    .execute(&mut conn)
                    .await
                    .unwrap();
                sqlx::query(&format!(
                    "ALTER TABLE bench_sentinelle ENABLE TRIGGER {variante}"
                ))
                .execute(&mut conn)
                .await
                .unwrap();
                let mut temps = Vec::new();
                for _ in 0..5 {
                    let t = Instant::now();
                    sqlx::query("UPDATE bench_sentinelle SET v = v + 1 WHERE id <= $1")
                        .bind(i32::try_from(lignes).unwrap())
                        .execute(&mut conn)
                        .await
                        .unwrap();
                    temps.push(t.elapsed().as_micros());
                }
                temps.sort_unstable();
                eprintln!(
                    "M4 {variante:<22} {lignes:>5} lignes  {autres:>3} verrous : {:>8} µs",
                    temps[2]
                );
            }
        }
    }
    drop(autre);
}

// ───────────────────────────── F3 : liste d'attente sur 10 000 dossiers ─────────────────────────────

/// Appels d'une fonction SQL dans la transaction courante (`pg_stat_xact_user_functions`, `track_functions = 'all'`) : un
/// **compte d'appels**, déterministe, jamais une durée.
async fn appels(tx: &mut sqlx::PgConnection, fonction: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(sum(calls), 0)::bigint FROM pg_stat_xact_user_functions WHERE funcname = $1",
    )
    .bind(fonction)
    .fetch_one(tx)
    .await
    .unwrap()
}

/// **F3** : sur un établissement de **10 000 dossiers en liste d'attente** répartis sur deux offres (50 sur l'offre lue),
/// `ent_inscription_liste_attente` n'appelle **aucune** fonction par dossier de l'établissement : zéro appel de
/// `ent_inscription_offre_du_dossier` (l'offre se lit par jointure), et la fonction par dossier de la famille
/// (`ent_inscription_en_liste_visible`) ne s'exécute que sur les **50 dossiers de l'offre**. Le même énoncé avec la formule de 316
/// (preuve rouge, rejouée ici) faisait 10 000 appels de l'offre du dossier. Aucune assertion de durée.
#[tokio::test]
async fn s6_liste_attente_borne_d_appels_sur_10000_dossiers() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, Some(3), "2027-2028").await;
    let (_camp2, autre) = campagne_cap(&ent, &c, Some(3), "2028-2029").await;
    for (cible, nb) in [(offre, 50i32), (autre, 9950)] {
        sqlx::query(
            "INSERT INTO ent_inscription_dossier
                (id, etablissement_id, campagne_id, nature, eleve_nom, eleve_prenom, etat, ordre_attente, offre_attente_id, reference)
             SELECT gen_random_uuid(), $1, $2, 'admission', 'Attente', 'Dossier', 'liste_attente', g, $3, 'F3-' || $3::text || '-' || g
               FROM generate_series(1, $4) g",
        )
        .bind(etab)
        .bind(camp)
        .bind(cible)
        .bind(nb)
        .execute(ent.pool_verif())
        .await
        .expect("10 000 dossiers en liste d'attente");
    }
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_inscription_dossier WHERE etablissement_id = $1 AND etat = 'liste_attente'")
        .bind(etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(total, 10_000);

    for famille in [false, true] {
        let mut tx = ent.pool_verif().begin().await.unwrap();
        sqlx::query("SET LOCAL track_functions = 'all'")
            .execute(&mut *tx)
            .await
            .unwrap();
        let liste: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT dossier_id, pos FROM ent_inscription_liste_attente($1, $2, $3) ORDER BY pos",
        )
        .bind(etab)
        .bind(offre)
        .bind(famille)
        .fetch_all(&mut *tx)
        .await
        .unwrap();
        assert_eq!(
            liste.len(),
            50,
            "la liste de l'offre compte ses 50 dossiers (famille = {famille})"
        );
        assert_eq!(
            liste.iter().map(|(_, p)| *p).collect::<Vec<_>>(),
            (1..=50).collect::<Vec<_>>(),
            "positions 1 à 50"
        );
        let par_dossier = appels(&mut tx, "ent_inscription_offre_du_dossier").await;
        let visibles = appels(&mut tx, "ent_inscription_en_liste_visible").await;
        assert_eq!(par_dossier, 0, "aucun appel de l'offre du dossier par dossier de l'établissement (famille = {famille})");
        assert!(
            visibles <= 50,
            "la fonction par dossier de la famille ne s'exécute que sur la liste de l'offre : {visibles} appels (famille = {famille})"
        );
        if famille {
            assert_eq!(visibles, 50, "un appel par dossier de l'offre côté famille");
        }
        tx.rollback().await.unwrap();
    }

    // Preuve rouge : la formule de 316 (offre du dossier appelée pour chaque dossier de l'établissement).
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SET LOCAL track_functions = 'all'")
        .execute(&mut *tx)
        .await
        .unwrap();
    let ancienne: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM (
            SELECT x.id, row_number() OVER (ORDER BY COALESCE(x.ordre_attente, x.rang_attente::bigint) NULLS LAST, x.id)
              FROM ent_inscription_dossier x
             WHERE x.etablissement_id = $1
               AND ent_inscription_offre_du_dossier(x.etablissement_id, x.id) = $2
               AND x.etat = 'liste_attente') l",
    )
    .bind(etab)
    .bind(offre)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(ancienne, 50);
    assert!(
        appels(&mut tx, "ent_inscription_offre_du_dossier").await >= 10_000,
        "la formule de 316 appelait l'offre du dossier pour chacun des 10 000 dossiers"
    );
    tx.rollback().await.unwrap();
}

// ───────────────────────────── F6 : alerte sans destinataire ─────────────────────────────

async fn membre(ent: &PgEnt, etab: Uuid, role: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_user (id, email, password_hash, display_name) VALUES ($1, $2, 'x', 'Membre de test')")
        .bind(id)
        .bind(format!("m-{}@exemple.test", id.simple()))
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, etat) VALUES ($1, $2, $3, 'ouvert')")
        .bind(id)
        .bind(etab)
        .bind(role)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    id
}

/// Fin de fonction : la date de fin des accès de l'établissement (sauf `sauf`) passe en 2020. La garde « dernier administrateur »
/// est levée **dans la transaction** (réglage de purge, comme la suppression d'un établissement) : c'est une fixture.
async fn fermer_acces(ent: &PgEnt, etab: Uuid, sauf: Option<Uuid>) {
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SET LOCAL ent.purge_etablissement = 'on'")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_membership SET date_fin = '2020-01-01' WHERE etablissement_id = $1 AND user_id IS DISTINCT FROM $2")
        .bind(etab)
        .bind(sauf)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

/// **F6** : une décision programmée devenue caduque alors qu'**aucun** accès de direction (ni l'auteur) n'est ouvert : repli
/// prudent sur les accès ouverts de l'administration (`administratif`) ; s'il n'y en a pas non plus, l'alerte est **tracée**
/// dans le suivi interne du dossier (invisible de la famille), jamais perdue en silence.
#[tokio::test]
async fn s6_alerte_sans_destinataire_repli_puis_trace() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let secretariat = membre(&ent, etab, "administratif").await;
    let (camp, _) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let mut ids = Vec::new();
    for (prenom, jours) in [("RepliUn", 7), ("RepliDeux", 8)] {
        let dos = dossier(&ent, &c, camp, prenom, None).await;
        let mut b = corps("ajourne", None, "Ajournement programmé");
        b.publier_le = Some(maintenant() + chrono::Duration::days(jours));
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
        ids.push((dos.id, d.id));
    }
    // Plus aucune direction ni auteur à accès ouvert (ils sont « partis ») ; seul le secrétariat reste.
    fermer_acces(&ent, etab, Some(secretariat)).await;
    let nb = |decision: Uuid, user: Uuid| {
        let pool = ent.pool_verif().clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM ent_notification WHERE source_event_id = $1 AND destinataire_user_id = $2
                    AND kind = 'inscription_decision_caduque'",
            )
            .bind(decision)
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    let traces = |dossier: Uuid| {
        let pool = ent.pool_verif().clone();
        async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM ent_inscription_suivi_dossier
                  WHERE dossier_id = $1 AND texte LIKE 'Alerte de décision caduque sans destinataire%' AND NOT visible_famille",
            )
            .bind(dossier)
            .fetch_one(&pool)
            .await
            .unwrap()
        }
    };
    // 1. Repli : seul le secrétariat est ouvert.
    ent.publier_decisions_echues_tache(maintenant() + chrono::Duration::days(7))
        .await
        .unwrap();
    assert_eq!(
        nb(ids[0].1, secretariat).await,
        1,
        "repli : le secrétariat est alerté"
    );
    assert_eq!(
        nb(ids[0].1, u).await,
        0,
        "l'auteur, dont la date de fin est dépassée, n'est pas alerté"
    );
    assert_eq!(
        traces(ids[0].0).await,
        0,
        "avec un destinataire, pas de trace « sans destinataire »"
    );
    // 2. Plus personne : trace interne.
    fermer_acces(&ent, etab, None).await;
    ent.publier_decisions_echues_tache(maintenant() + chrono::Duration::days(8))
        .await
        .unwrap();
    assert_eq!(
        nb(ids[1].1, secretariat).await,
        0,
        "date de fin dépassée : aucune alerte"
    );
    assert_eq!(
        traces(ids[1].0).await,
        1,
        "sans destinataire : l'alerte est tracée dans le suivi interne"
    );
}

// ───────────────────────────── F7 : archivage d'offre ─────────────────────────────

/// **F7** : l'archivage d'une offre **attend** le verrou de l'offre (comme toute écriture sur une offre), puis aboutit.
#[tokio::test]
async fn s6_archivage_d_offre_attend_le_verrou() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = Arc::new(ent);
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (_camp, offre) = campagne_cap(&ent, &c, Some(3), "2027-2028").await;
    let verrou = tenir(&ent, &[offre]).await;
    let archivage = {
        let e = ent.clone();
        tokio::spawn(async move { e.archiver_offre_for_actor(etab, u, r, offre).await })
    };
    attendre_attentes(&ent, offre, 1, "archivage de l'offre").await;
    let archivee: bool = sqlx::query_scalar(
        "SELECT archivee_le IS NOT NULL FROM ent_inscription_offre WHERE id = $1",
    )
    .bind(offre)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(!archivee, "pendant l'attente, l'offre n'est pas archivée");
    verrou.relacher().await;
    issue(archivage, "archivage")
        .await
        .expect("l'archivage aboutit une fois le verrou rendu");
    let archivee: bool = sqlx::query_scalar(
        "SELECT archivee_le IS NOT NULL FROM ent_inscription_offre WHERE id = $1",
    )
    .bind(offre)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(archivee, "l'offre est archivée");
}

/// **F7** : le décompte des vœux est lu **sous** le verrou : un vœu posé sur l'offre (par un chemin qui tient ce verrou) pendant
/// l'attente de l'archivage est vu, et l'archivage est refusé (conflit), l'offre reste en place.
#[tokio::test]
async fn s6_archivage_lit_les_voeux_sous_le_verrou() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = Arc::new(ent);
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(3), "2027-2028").await;
    let dos = dossier(&ent, &c, camp, "VoeuTardif", None).await;
    let verrou = tenir(&ent, &[offre]).await;
    let archivage = {
        let e = ent.clone();
        tokio::spawn(async move { e.archiver_offre_for_actor(etab, u, r, offre).await })
    };
    attendre_attentes(&ent, offre, 1, "archivage de l'offre").await;
    // Pendant l'attente, un chemin qui tient le verrou pose un vœu sur l'offre (ici la connexion qui le tient).
    sqlx::query(
        "INSERT INTO ent_inscription_voeu (id, etablissement_id, dossier_id, rang, offre_id) VALUES ($1,$2,$3,1,$4)
         ON CONFLICT DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(dos.id)
    .bind(offre)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    verrou.relacher().await;
    let res = issue(archivage, "archivage").await;
    assert!(
        res.is_err(),
        "un vœu s'est posé pendant l'attente : l'archivage est refusé"
    );
    let archivee: bool = sqlx::query_scalar(
        "SELECT archivee_le IS NOT NULL FROM ent_inscription_offre WHERE id = $1",
    )
    .bind(offre)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(!archivee, "l'offre reste en place");
}
