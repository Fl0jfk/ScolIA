//! Socle 3, lot I5 — **suites de la passe 4** (« passe 5 »), contre la base locale de test (feature `labo`).
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés en local.
//!
//! Aucun test ne dépend de l'heure réelle ni du jour UTC : instants posés (2027), dates fixes (2099) ; les accès de la direction
//! sont fermés par leur **état** ou par une date de fin fixée dans le passé lointain.
//!
//! | Sujet | Test(s) |
//! |---|---|
//! | M-2 sentinelle en base : écriture hors verrou refusée, verrou de l'offre acceptée | `s5_sentinelle_refuse_l_ecriture_hors_verrou`, `s5_sentinelle_ne_se_falsifie_pas` |
//! | M-2 sentinelle : tout chemin du service passe (cycle complet, sans erreur de sentinelle) | `s5_cycle_complet_sous_sentinelle` |
//! | M-3 `repondre_accord` : offres → dossier → demande, contre une décision de séance | `s5_repondre_accord_dans_l_ordre_contre_une_decision` |
//! | F-a/F-b écritures de masse (prolongation, fermeture) sous le verrou d'offre | `s5_prolongation_sous_le_verrou_d_offre`, `s5_fermeture_sous_le_verrou_d_offre` |
//! | F-c formule de rang unique | `s5_rang_une_seule_formule`, `s5_garde_statique_formule_de_rang` |
//! | F-e directeurs partis non alertés | `s5_directeurs_partis_non_alertes` |

use std::sync::Arc;

use chrono::NaiveDate;
use ent_types::inscriptions::*;
use uuid::Uuid;

use super::tests_inscriptions::{dir, labo};
use super::tests_inscriptions_i3_i4::{dernier_code, lien_accord};
use super::tests_inscriptions_i5::{campagne_cap, corps, decider, dossier, expedier, maintenant};
use super::tests_inscriptions_i5_passe4::{
    attendre_attentes, est_technique, issue, positions, tenir, tous_libres, SOURCES_INSCRIPTION,
};
use super::tests_inscriptions_i5_revue::sqlstate;
use super::*;

const SENTINELLE: &str = "ent_inscription_verrou_offre_manquant";

fn jour(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

/// Exécute `sql` (une écriture, `$1` = identifiant donné) dans une transaction, après avoir pris — ou non — le verrou
/// consultatif de transaction de `verrou` (la clé de l'offre, par la **même** fonction SQL que le code).
async fn ecrire(ent: &PgEnt, verrou: Option<Uuid>, sql: &str, id: Uuid) -> Result<(), sqlx::Error> {
    let mut tx = ent.pool_verif().begin().await?;
    if let Some(o) = verrou {
        sqlx::query("SELECT pg_advisory_xact_lock(ent_inscription_offre_cle($1))")
            .bind(o)
            .execute(&mut *tx)
            .await?;
    }
    let r = sqlx::query(sql).bind(id).execute(&mut *tx).await;
    match r {
        Ok(_) => tx.commit().await,
        Err(e) => {
            tx.rollback().await?;
            Err(e)
        }
    }
}

/// Refus nommé de la sentinelle : SQLSTATE `55000` **et** nom de l'erreur dans le message.
fn refus_sentinelle(r: &Result<(), sqlx::Error>, quoi: &str) {
    assert_eq!(
        sqlstate(r),
        "55000",
        "{quoi} : refus attendu de la sentinelle"
    );
    let msg = r.as_ref().unwrap_err().to_string();
    assert!(
        msg.contains(SENTINELLE),
        "{quoi} : l'erreur doit être nommée ({msg})"
    );
}

// ───────────────────────────── M-2 : sentinelle en base ─────────────────────────────

/// **M-2** : une écriture des places (offre), des propositions ou des décisions **sans** le verrou consultatif de l'offre
/// concernée est refusée par la base, quel que soit le chemin (ici du SQL direct, que la garde statique ne voit pas) ; avec le
/// verrou de **cette** offre elle passe. Une écriture qui ne touche pas les places n'est pas gênée.
#[tokio::test]
async fn s5_sentinelle_refuse_l_ecriture_hors_verrou() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, Some(3), "2027-2028").await;
    let d = dossier(&ent, &c, camp, "Sentinelle", None).await;
    let dec = decider(&ent, &c, &d, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();

    // Places de l'offre : capacité, places rendues.
    let liberer = "UPDATE ent_inscription_offre SET liberations = liberations + 1 WHERE id = $1";
    refus_sentinelle(
        &ecrire(&ent, None, liberer, offre).await,
        "places rendues sans verrou",
    );
    let capacite =
        "UPDATE ent_inscription_offre SET capacite = COALESCE(capacite, 0) + 1 WHERE id = $1";
    refus_sentinelle(
        &ecrire(&ent, None, capacite, offre).await,
        "capacité sans verrou",
    );
    // Une colonne qui ne touche pas aux places n'est pas soumise ; une capacité inchangée non plus.
    ecrire(
        &ent,
        None,
        "UPDATE ent_inscription_offre SET modifie_le = now() WHERE id = $1",
        offre,
    )
    .await
    .unwrap();
    ecrire(
        &ent,
        None,
        "UPDATE ent_inscription_offre SET capacite = capacite WHERE id = $1",
        offre,
    )
    .await
    .unwrap();
    ecrire(&ent, Some(offre), liberer, offre).await.unwrap();
    ecrire(&ent, Some(offre), capacite, offre).await.unwrap();

    // Décisions.
    let reporter = "UPDATE ent_inscription_decision SET publication_prevue_le = NULL WHERE id = $1";
    refus_sentinelle(
        &ecrire(&ent, None, reporter, dec.id).await,
        "décision sans verrou",
    );
    ecrire(&ent, Some(offre), reporter, dec.id).await.unwrap();

    // Propositions : insertion (offre donnée), insertion (offre déduite du dossier), mise à jour, suppression.
    let inserer = format!(
        "INSERT INTO ent_inscription_proposition_place (id, etablissement_id, dossier_id, offre_id, origine, statut)
         VALUES (gen_random_uuid(), '{etab}', $1, '{offre}', 'liste_attente', 'annulee')"
    );
    refus_sentinelle(
        &ecrire(&ent, None, &inserer, d.id).await,
        "proposition sans verrou",
    );
    let sans_offre = format!(
        "INSERT INTO ent_inscription_proposition_place (id, etablissement_id, dossier_id, origine, statut)
         VALUES (gen_random_uuid(), '{etab}', $1, 'liste_attente', 'annulee')"
    );
    refus_sentinelle(
        &ecrire(&ent, None, &sans_offre, d.id).await,
        "proposition sans offre propre : l'offre du dossier est celle qui compte",
    );
    ecrire(&ent, Some(offre), &inserer, d.id).await.unwrap();
    ecrire(&ent, Some(offre), &sans_offre, d.id).await.unwrap();
    let majeure = "UPDATE ent_inscription_proposition_place SET motif = 'x' WHERE dossier_id = $1";
    refus_sentinelle(
        &ecrire(&ent, None, majeure, d.id).await,
        "mise à jour de proposition sans verrou",
    );
    ecrire(&ent, Some(offre), majeure, d.id).await.unwrap();
    let supprimer = "DELETE FROM ent_inscription_proposition_place WHERE dossier_id = $1";
    refus_sentinelle(
        &ecrire(&ent, None, supprimer, d.id).await,
        "suppression de proposition sans verrou",
    );
    ecrire(&ent, Some(offre), supprimer, d.id).await.unwrap();
}

/// Contre-revue des suites (M1) : une proposition dont l'offre était **déduite du dossier** (A) et qui reçoit une offre propre
/// (B) exige les **deux** verrous : B seul est refusé, A seul aussi, A et B passent.
#[tokio::test]
async fn s5_sentinelle_changement_d_offre_exige_l_ancienne_et_la_nouvelle() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre_a) = campagne_cap(&ent, &c, Some(3), "2027-2028").await;
    let (_, offre_b) = campagne_cap(&ent, &c, Some(3), "2028-2029").await;
    let d = dossier(&ent, &c, camp, "Changement", None).await;
    decider(&ent, &c, &d, &corps("admis", Some(offre_a), "Admis"))
        .await
        .unwrap();
    let sans_offre = format!(
        "INSERT INTO ent_inscription_proposition_place (id, etablissement_id, dossier_id, origine, statut)
         VALUES (gen_random_uuid(), '{etab}', $1, 'liste_attente', 'annulee')"
    );
    ecrire(&ent, Some(offre_a), &sans_offre, d.id).await.unwrap();
    let vers_b = format!(
        "UPDATE ent_inscription_proposition_place SET offre_id = '{offre_b}' WHERE dossier_id = $1 AND offre_id IS NULL"
    );
    refus_sentinelle(
        &ecrire(&ent, Some(offre_b), &vers_b, d.id).await,
        "nouvelle offre seule : l'ancienne (déduite du dossier) manque",
    );
    refus_sentinelle(
        &ecrire(&ent, Some(offre_a), &vers_b, d.id).await,
        "ancienne offre seule : la nouvelle manque",
    );
    let mut tx = ent.pool_verif().begin().await.unwrap();
    for o in [offre_a.min(offre_b), offre_a.max(offre_b)] {
        sqlx::query("SELECT pg_advisory_xact_lock(ent_inscription_offre_cle($1))")
            .bind(o)
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    let n = sqlx::query(&vers_b).bind(d.id).execute(&mut *tx).await.expect("les deux verrous suffisent");
    assert_eq!(n.rows_affected(), 1);
    tx.rollback().await.unwrap();
}

/// La sentinelle ne se falsifie pas : le verrou d'**une autre offre**, le verrou **d'une autre session**, un réglage de session
/// ou une table temporaire du même nom ne la satisfont pas. Seul le verrou consultatif réellement accordé à cette session, pour
/// cette offre, la satisfait.
#[tokio::test]
async fn s5_sentinelle_ne_se_falsifie_pas() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (_, offre_a) = campagne_cap(&ent, &c, Some(3), "2027-2028").await;
    let (_, offre_b) = campagne_cap(&ent, &c, Some(3), "2028-2029").await;
    let liberer = "UPDATE ent_inscription_offre SET liberations = liberations + 1 WHERE id = $1";

    // Le verrou d'une autre offre ne couvre pas celle-ci.
    refus_sentinelle(
        &ecrire(&ent, Some(offre_b), liberer, offre_a).await,
        "verrou d'une autre offre",
    );

    // Le verrou **d'une autre session** ne couvre pas cette session (filtre `pid = pg_backend_pid()`).
    let externe = tenir(&ent, &[offre_a]).await;
    refus_sentinelle(
        &ecrire(&ent, None, liberer, offre_a).await,
        "verrou tenu par une autre session",
    );
    externe.relacher().await;

    // Un réglage de session ne remplace pas le verrou : aucune garde ne lit `set_config`.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.verrou_offre', $1, true), set_config('application_name', 'verrou', true)")
        .bind(offre_a.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let r = sqlx::query(liberer)
        .bind(offre_a)
        .execute(&mut *tx)
        .await
        .map(|_| ());
    refus_sentinelle(&r, "réglage de session");
    tx.rollback().await.unwrap();

    // Une table temporaire qui masquerait `pg_locks` n'est pas lue : la fonction fixe son `search_path`.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("CREATE TEMP TABLE pg_locks (locktype text, pid int, granted bool, objsubid smallint, database oid, classid oid, objid oid) ON COMMIT DROP")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO pg_temp.pg_locks VALUES ('advisory', pg_backend_pid(), true, 1, 0, 0, 0)",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let r = sqlx::query(liberer)
        .bind(offre_a)
        .execute(&mut *tx)
        .await
        .map(|_| ());
    refus_sentinelle(&r, "table temporaire masquant pg_locks");
    tx.rollback().await.unwrap();

    // Le verrou de session de cette connexion (non transactionnel) vaut aussi : la base ne voit que le verrou accordé.
    let mut conn = ent.pool_verif().acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock(ent_inscription_offre_cle($1))")
        .bind(offre_a)
        .execute(&mut *conn)
        .await
        .unwrap();
    let r = sqlx::query(liberer)
        .bind(offre_a)
        .execute(&mut *conn)
        .await
        .map(|_| ());
    sqlx::query("SELECT pg_advisory_unlock(ent_inscription_offre_cle($1))")
        .bind(offre_a)
        .execute(&mut *conn)
        .await
        .unwrap();
    r.expect("un verrou réellement tenu par cette session satisfait la sentinelle");
}

/// **M-2** : le service complet (dépôt, décisions, liste d'attente, désistement, désignation, proposition, acceptation,
/// modification de capacité, prolongation, fermeture) tourne **sous** la sentinelle : aucune écriture du service n'est refusée.
/// Chaque écriture de places, de propositions ou de décisions des fichiers `inscriptions*.rs` passe donc par le verrou de l'offre.
#[tokio::test]
async fn s5_cycle_complet_sous_sentinelle() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let a = dossier(&ent, &c, camp, "CycleA", None).await;
    let b = dossier(&ent, &c, camp, "CycleB", None).await;
    let x = dossier(&ent, &c, camp, "CycleC", None).await;
    decider(&ent, &c, &a, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    decider(
        &ent,
        &c,
        &b,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    decider(
        &ent,
        &c,
        &x,
        &corps("liste_attente", Some(offre), "Complet"),
    )
    .await
    .unwrap();
    // A se désiste : la place est rendue, B est désigné (propositions, places).
    ent.desister_famille(etab, &a.jeton, a.id).await.unwrap();
    let l = ent
        .liste_attente_for_actor(etab, u, r, offre)
        .await
        .unwrap();
    assert!(
        l.lignes.iter().any(|x| x.proposition_id.is_some()),
        "une place libérée est désignée au suivant"
    );
    // Un départ de la liste, puis une capacité en hausse : une nouvelle place pour le dernier.
    ent.retirer_candidature_famille(etab, &x.jeton, x.id)
        .await
        .unwrap();
    ent.modifier_offre_for_actor(
        etab,
        u,
        r,
        offre,
        &OffreBody {
            niveau_code: "6EME".into(),
            libelle_public: "6ᵉ".into(),
            capacite: Some(3),
            motif: Some("Ouverture de places".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let propositions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_inscription_proposition_place WHERE offre_id = $1",
    )
    .bind(offre)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(
        propositions >= 1,
        "des propositions ont été écrites sous la sentinelle"
    );
}

// ───────────────────────────── M-3 : `repondre_accord` dans l'ordre ─────────────────────────────

/// Campagne d'admission où **tous les titulaires** doivent donner leur accord, avec son offre.
async fn campagne_accord(ent: &PgEnt, c: &LaboCast, annee: &str) -> (Uuid, Uuid) {
    let (d, dr) = dir(c);
    let mut regles = crate::inscriptions_modele::regles_par_defaut("admission");
    regles.accord_responsables = "tous_les_titulaires".into();
    let mut b = super::tests_inscriptions::campagne("admission", Some(5));
    b.pieces = vec![];
    b.annee_cible = annee.into();
    b.regles = Some(regles);
    let camp = ent
        .creer_campagne_for_actor(c.etablissement_id, d, dr, &b)
        .await
        .unwrap();
    ent.ouvrir_campagne_for_actor(c.etablissement_id, d, dr, camp.id)
        .await
        .unwrap();
    let offre: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_inscription_offre WHERE etablissement_id = $1 AND campagne_id = $2 AND archivee_le IS NULL",
    )
    .bind(c.etablissement_id)
    .bind(camp.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    (camp.id, offre)
}

/// **M-3** : `repondre_accord` prend les **offres, puis le dossier**, la demande en dernier. Une connexion externe tient le verrou
/// de l'offre (comme une décision de séance en cours) : la réponse **attend l'offre** sans tenir la demande ni le dossier. On
/// vérifie qu'une transaction qui tient l'offre peut prendre la demande (`FOR UPDATE NOWAIT`) — l'ancien code, qui verrouillait la
/// demande avant l'offre, formait ici le cycle —, puis qu'une décision de séance concurrente et la réponse aboutissent toutes
/// deux, sans interblocage.
#[tokio::test]
async fn s5_repondre_accord_dans_l_ordre_contre_une_decision() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = Arc::new(ent);
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_accord(&ent, &c, "2027-2028").await;
    let pere = super::tests_inscriptions_i5::email("pere");
    let avec_accord = dossier(&ent, &c, camp, "AvecAccord", Some(&pere)).await;
    let autre = dossier(&ent, &c, camp, "AutreDossier", None).await;
    // Le dossier est rattaché à l'offre (vœu retenu) par une décision d'attente : l'admission attendrait l'accord du père.
    decider(
        &ent,
        &c,
        &avec_accord,
        &corps("liste_attente", Some(offre), "Rattaché à l'offre"),
    )
    .await
    .unwrap();
    let sur_offre: Option<Uuid> =
        sqlx::query_scalar("SELECT voeu_retenu_id FROM ent_inscription_dossier WHERE id = $1")
            .bind(avec_accord.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert!(
        sur_offre.is_some(),
        "le dossier du test porte un vœu sur l'offre"
    );
    expedier(&ent).await;
    let (demande, jeton) = lien_accord(&ent, &pere);
    ent.envoyer_code_accord(etab, demande, &jeton)
        .await
        .unwrap();
    expedier(&ent).await;
    let code = dernier_code(&ent, &pere);

    let verrou = tenir(&ent, &[offre]).await;
    let reponse = {
        let (e, j, k) = (ent.clone(), jeton.clone(), code.clone());
        tokio::spawn(async move { e.repondre_accord(etab, demande, &j, &k, "donne").await })
    };
    let decision = {
        let (e, id) = (ent.clone(), autre.id);
        let b = corps("admis", Some(offre), "Décision de séance concurrente");
        tokio::spawn(async move {
            e.decider_complete_for_actor(etab, u, r, id, &b, false, maintenant())
                .await
                .map(|_| ())
        })
    };
    attendre_attentes(&ent, offre, 2, "réponse d'accord + décision").await;
    // L'ordre : ni le dossier, ni la demande ne sont tenus par la réponse qui attend l'offre.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    let demande_libre = sqlx::query(
        "SELECT id FROM ent_inscription_accord_demande WHERE id = $1 FOR UPDATE NOWAIT",
    )
    .bind(demande)
    .execute(&mut *tx)
    .await
    .is_ok();
    tx.rollback().await.unwrap();
    assert!(
        demande_libre,
        "repondre_accord tient la demande alors qu'il attend l'offre (ordre inversé)"
    );
    tous_libres(&ent, &[&avec_accord, &autre], "réponse d'accord + décision").await;
    verrou.relacher().await;
    let (rep, dec) = (
        issue(reponse, "réponse d'accord").await,
        issue(decision, "décision de séance").await,
    );
    rep.expect("la réponse aboutit");
    if let Err(e) = &dec {
        assert!(!est_technique(e), "décision : erreur technique {e:?}");
    }
    let reponse_enregistree: String = sqlx::query_scalar(
        "SELECT a.reponse FROM ent_inscription_accord a
           JOIN ent_inscription_accord_demande dm ON dm.etablissement_id = a.etablissement_id AND dm.accord_id = a.id
          WHERE dm.id = $1",
    )
    .bind(demande)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(reponse_enregistree, "donne");
}

// ───────────────────────────── F-a / F-b : écritures de masse sous le verrou d'offre ─────────────────────────────

/// **F-b** : la prolongation des échéances de dossiers (écriture de masse) attend le verrou de l'offre de ses dossiers, ne tient
/// aucun dossier pendant l'attente, puis aboutit. Sans verrou d'offre, la sentinelle ne la voyait pas (elle ne touche que des
/// dossiers) : c'est l'ordre unique qui le garantit, prouvé ici par l'attente.
#[tokio::test]
async fn s5_prolongation_sous_le_verrou_d_offre() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = Arc::new(ent);
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(5), "2027-2028").await;
    let d1 = dossier(&ent, &c, camp, "ProlongeUn", None).await;
    let d2 = dossier(&ent, &c, camp, "ProlongeDeux", None).await;
    for d in [&d1, &d2] {
        decider(
            &ent,
            &c,
            d,
            &corps("liste_attente", Some(offre), "Rattaché à l'offre"),
        )
        .await
        .unwrap();
    }
    for d in [&d1, &d2] {
        sqlx::query("UPDATE ent_inscription_dossier SET echeance_famille = $2 WHERE id = $1")
            .bind(d.id)
            .bind(jour("2099-01-10"))
            .execute(ent.pool_verif())
            .await
            .unwrap();
    }
    let verrou = tenir(&ent, &[offre]).await;
    let prolongation = {
        let e = ent.clone();
        let b = ProlongationBody {
            objet: "echeance_dossiers".into(),
            nouvelle_date: jour("2099-06-16"),
            motif_public: "Prolongation de test".into(),
            dossiers: vec![],
        };
        tokio::spawn(async move { e.prolonger_for_actor(etab, u, r, camp, &b).await })
    };
    attendre_attentes(&ent, offre, 1, "prolongation des échéances").await;
    tous_libres(&ent, &[&d1, &d2], "prolongation des échéances").await;
    verrou.relacher().await;
    let p = issue(prolongation, "prolongation")
        .await
        .expect("la prolongation aboutit");
    assert_eq!(p.nb_dossiers, 2);
    for d in [&d1, &d2] {
        let e: Option<NaiveDate> = sqlx::query_scalar(
            "SELECT echeance_famille FROM ent_inscription_dossier WHERE id = $1",
        )
        .bind(d.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert_eq!(e, Some(jour("2099-06-16")));
    }
}

/// **F-b** : la déclaration de fermeture (écriture de masse sur les dossiers dont l'échéance tombe dans la période) attend les
/// offres de ces dossiers, sans tenir de dossier.
#[tokio::test]
async fn s5_fermeture_sous_le_verrou_d_offre() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let ent = Arc::new(ent);
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let (camp, offre) = campagne_cap(&ent, &c, Some(5), "2027-2028").await;
    let d1 = dossier(&ent, &c, camp, "FermeUn", None).await;
    decider(
        &ent,
        &c,
        &d1,
        &corps("liste_attente", Some(offre), "Rattaché à l'offre"),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE ent_inscription_dossier SET echeance_famille = $2 WHERE id = $1")
        .bind(d1.id)
        .bind(jour("2099-02-10"))
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let verrou = tenir(&ent, &[offre]).await;
    let fermeture = {
        let e = ent.clone();
        let b = FermetureBody {
            date_debut: jour("2099-02-01"),
            date_fin: jour("2099-02-20"),
            report_jours: Some(3),
            motif_public: "Fermeture de test".into(),
            contact_urgence: None,
        };
        tokio::spawn(async move { e.declarer_fermeture_for_actor(etab, u, r, &b).await })
    };
    attendre_attentes(&ent, offre, 1, "déclaration de fermeture").await;
    tous_libres(&ent, &[&d1], "déclaration de fermeture").await;
    verrou.relacher().await;
    issue(fermeture, "fermeture")
        .await
        .expect("la fermeture aboutit");
    let e: Option<NaiveDate> =
        sqlx::query_scalar("SELECT echeance_famille FROM ent_inscription_dossier WHERE id = $1")
            .bind(d1.id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(
        e,
        Some(jour("2099-02-24")),
        "20 février + 1 + 3 jours de report"
    );
}

// ───────────────────────────── F-c : une seule formule du rang ─────────────────────────────

/// **F-c** : la liste d'attente affichée, le rang d'un dossier et l'ordre de désignation lisent la **même** liste
/// (`ent_inscription_liste_attente`) : les positions coïncident après un départ au milieu, et le suivant désigné est celui de
/// la position 1.
#[tokio::test]
async fn s5_rang_une_seule_formule() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (camp, offre) = campagne_cap(&ent, &c, Some(1), "2027-2028").await;
    let occupant = dossier(&ent, &c, camp, "RangOccupant", None).await;
    decider(&ent, &c, &occupant, &corps("admis", Some(offre), "Admis"))
        .await
        .unwrap();
    let mut file = Vec::new();
    for i in 0..5 {
        let d = dossier(&ent, &c, camp, &format!("RangFile{i}"), None).await;
        decider(
            &ent,
            &c,
            &d,
            &corps("liste_attente", Some(offre), "Complet"),
        )
        .await
        .unwrap();
        file.push(d);
    }
    ent.retirer_candidature_famille(etab, &file[1].jeton, file[1].id)
        .await
        .unwrap();

    let affichee = positions(&ent, &c, offre).await;
    assert_eq!(affichee.len(), 4);
    // Le rang d'un dossier (fonction SQL) = sa position affichée, = sa position dans la liste SQL ordonnée.
    for (id, pos) in &affichee {
        let rang: Option<i64> =
            sqlx::query_scalar("SELECT ent_inscription_rang_attente($1, $2, false)")
                .bind(etab)
                .bind(id)
                .fetch_one(ent.pool_verif())
                .await
                .unwrap();
        assert_eq!(rang, Some(*pos), "rang SQL et position affichée");
        let liste: Option<i64> = sqlx::query_scalar(
            "SELECT pos FROM ent_inscription_liste_attente($1, $2, false) WHERE dossier_id = $3",
        )
        .bind(etab)
        .bind(offre)
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert_eq!(liste, Some(*pos), "liste SQL et position affichée");
    }
    assert_eq!(
        affichee.iter().map(|p| p.1).collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    // Un dossier hors de la liste (l'occupant, le partant) n'a pas de rang.
    for hors in [occupant.id, file[1].id] {
        let rang: Option<i64> =
            sqlx::query_scalar("SELECT ent_inscription_rang_attente($1, $2, false)")
                .bind(etab)
                .bind(hors)
                .fetch_one(ent.pool_verif())
                .await
                .unwrap();
        assert_eq!(rang, None);
    }
    // La désignation prend la position 1 : le premier de la file reçoit la place rendue par le retrait de l'occupant.
    ent.desister_famille(etab, &occupant.jeton, occupant.id)
        .await
        .unwrap();
    let designes: Vec<Uuid> = sqlx::query_scalar(
        "SELECT dossier_id FROM ent_inscription_proposition_place
          WHERE offre_id = $1 AND origine = 'liste_attente' AND statut = 'designee'",
    )
    .bind(offre)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        designes,
        vec![file[0].id],
        "le suivant désigné est celui de la position 1"
    );
}

/// **F-c** (garde statique) : la formule d'ordre de la liste (`ordre_attente`, repli `rang_attente`) n'apparaît dans **aucune**
/// requête des fichiers d'inscription : elle ne vit que dans `ent_inscription_liste_attente` (migration 316).
#[test]
fn s5_garde_statique_formule_de_rang() {
    let mut fautes = Vec::new();
    for (nom, src) in SOURCES_INSCRIPTION {
        for motif in ["ordre_attente", "row_number()"] {
            let mut debut = 0;
            while let Some(i) = src[debut..].find(motif) {
                let pos = debut + i;
                debut = pos + motif.len();
                let ligne_debut = src[..pos].rfind('\n').map_or(0, |i| i + 1);
                if src[ligne_debut..].trim_start().starts_with("//") {
                    continue;
                }
                let lit = super::tests_inscriptions_i5_passe4::litteral_autour(src, pos);
                // L'écriture de `ordre_attente` à l'entrée sur la liste (affectation) n'est pas une formule d'ordre.
                if lit.contains("ORDER BY")
                    || lit.contains("OVER (")
                    || lit.contains("COALESCE(d.ordre_attente")
                {
                    fautes.push(format!(
                        "{nom} : `{motif}` dans une requête d'ordre de liste ({})",
                        lit.lines().next().unwrap_or("")
                    ));
                }
            }
        }
    }
    assert!(
        fautes.is_empty(),
        "formule de rang dupliquée :\n{}",
        fautes.join("\n")
    );
}

// ───────────────────────────── F-e : directeurs partis ─────────────────────────────

/// Crée un compte de direction de l'établissement avec l'état d'accès et la date de fin donnés.
async fn directeur(ent: &PgEnt, etab: Uuid, etat: &str, date_fin: Option<NaiveDate>) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_user (id, email, password_hash, display_name) VALUES ($1, $2, 'x', 'Direction de test')")
        .bind(id)
        .bind(format!("dir-{}@exemple.test", id.simple()))
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_membership (user_id, etablissement_id, role, etat, date_fin) VALUES ($1, $2, 'direction', $3, $4)",
    )
    .bind(id)
    .bind(etab)
    .bind(etat)
    .bind(date_fin)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}

/// **F-e** : l'avis « décision programmée non publiée » ne va qu'aux accès **ouverts aujourd'hui** : un directeur dont l'accès est
/// fermé (fin de fonction), suspendu, ou dont la date de fin est dépassée, n'est pas alerté ; un directeur en poste et le
/// compte en poste qui a décidé le sont. Une seule fois chacun.
#[tokio::test]
async fn s5_directeurs_partis_non_alertes() {
    let Some((ent, c)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = c.etablissement_id;
    let (u, r) = dir(&c);
    let en_poste = directeur(&ent, etab, "ouvert", None).await;
    let fin_future = directeur(&ent, etab, "ouvert", Some(jour("2099-12-31"))).await;
    let parti_ferme = directeur(&ent, etab, "ferme", None).await;
    let suspendu = directeur(&ent, etab, "suspendu", None).await;
    let fin_depassee = directeur(&ent, etab, "ouvert", Some(jour("2020-01-01"))).await;

    let (camp, _) = campagne_cap(&ent, &c, None, "2027-2028").await;
    let annule = dossier(&ent, &c, camp, "AjourneAnnule", None).await;
    let mut b = corps("ajourne", None, "Ajournement programmé");
    b.publier_le = Some(maintenant() + chrono::Duration::days(7));
    let d = ent
        .decider_complete_for_actor(etab, u, r, annule.id, &b, false, maintenant())
        .await
        .unwrap();
    ent.transition_for_actor(
        etab,
        u,
        r,
        annule.id,
        &TransitionBody {
            action: "annuler".into(),
            motif: "Annulation demandée".into(),
        },
    )
    .await
    .unwrap();
    ent.publier_decisions_echues_tache(maintenant() + chrono::Duration::days(7))
        .await
        .unwrap();

    let recu = |user: Uuid| {
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
        recu(en_poste).await,
        1,
        "directeur en poste : alerté une fois"
    );
    assert_eq!(
        recu(fin_future).await,
        1,
        "directeur dont la fin est lointaine : alerté"
    );
    assert_eq!(
        recu(u).await,
        1,
        "l'auteur de la décision, accès ouvert : alerté"
    );
    for (id, nom) in [
        (parti_ferme, "accès fermé (fin de fonction)"),
        (suspendu, "accès suspendu"),
        (fin_depassee, "date de fin dépassée"),
    ] {
        assert_eq!(recu(id).await, 0, "{nom} : aucune alerte");
    }
}
