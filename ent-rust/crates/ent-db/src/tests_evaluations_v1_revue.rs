//! Socle 21, lots V1/V2 — tests des **correctifs de la revue indépendante** (E1 à E4, M1 à M10, F1 à F13).
//! Aucune horloge réelle : année fictive 2031-2032, dates fixes. Les autorisations de rattrapage utilisent des
//! dates très éloignées (2020 : périmée ; 2099 : en vigueur) pour ne dépendre d'aucun « aujourd'hui ».

use super::*;
use crate::tests_evaluations::labo;
use crate::tests_evaluations_v1::*;
use domain_eleve::evaluation_moteur::{Comptage, ReglesMoteur, StatutCalcul, TraitementNonRendu};
use sqlx::Row;
use uuid::Uuid;

fn code(e: &sqlx::Error) -> Option<String> {
    e.as_database_error()
        .and_then(|d| d.code())
        .map(|c| c.to_string())
}

async fn evaluation_maths(ent: &PgEnt, w: &W, jour: chrono::NaiveDate, publiee: bool) -> Uuid {
    evaluation(
        ent,
        w,
        w.cours_maths,
        jour,
        "DS",
        20.0,
        1.0,
        Comptage::Normal,
        publiee,
    )
    .await
}

// ───────────────────────── E1 : TRUNCATE ─────────────────────────

#[tokio::test]
async fn e1_truncate_est_refuse_sur_toutes_les_tables_protegees() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation_maths(&ent, &w, d(2031, 10, 10), true).await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, w.eleves_a[0], 12_000))
        .await
        .unwrap();
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let protegees = [
        "eva_resultat_historique",
        "eva_periode_transition",
        "eva_reglage_historique",
        "eva_appreciation_historique",
        "eva_consultation",
        "eva_instantane",
        "eva_instantane_ligne",
        "eva_reprise_ligne",
        "eva_resultat",
        "eva_rattrapage_autorise",
        "eva_evaluation_historique",
        "eva_evaluation",
        "eva_reglage",
        "eva_periode_etat",
        "eva_appreciation",
        "eva_coefficient_matiere",
        "eva_coefficient_sous_matiere",
    ];
    for table in protegees {
        // Seule la protection est visée : on vide « en cascade » pour aussi couvrir le vidage par une table mère.
        let e = sqlx::query(&format!("TRUNCATE {table} CASCADE"))
            .execute(ent.pool_verif())
            .await
            .expect_err(&format!("{table} : TRUNCATE refusé"));
        assert_eq!(
            code(&e).as_deref(),
            Some("23514"),
            "{table} : refus par le déclencheur ({e})"
        );
    }
    // Un TRUNCATE d'une table parente ne vide rien non plus (la cascade atteint une table protégée).
    assert!(sqlx::query("TRUNCATE ent_eleve CASCADE")
        .execute(ent.pool_verif())
        .await
        .is_err());
    let (n,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM eva_resultat WHERE etablissement_id = $1 AND evaluation_id = $2")
            .bind(w.etab)
            .bind(ev)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(n, 1, "rien n'a été vidé");
}

// ───────────────────────── E2 : ni SET, ni source ─────────────────────────

#[tokio::test]
async fn e2_aucune_variable_de_session_ne_contourne_l_historique_ni_les_gardes() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation_maths(&ent, &w, d(2031, 10, 10), true).await;
    let (a, dd) = (w.eleves_a[0], w.eleves_a[2]);
    let r = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 12_000))
        .await
        .unwrap();
    // Les variables « historiques » de la reprise et du motif ne font plus rien : l'historique s'écrit quand même.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.eva_reprise', 'oui', true), set_config('ent.eva_motif', 'forgé pour le test', true)")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE eva_resultat SET valeur = 13 WHERE id = $1")
        .bind(r.resultat_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    let (n,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM eva_resultat_historique WHERE resultat_id = $1")
            .bind(r.resultat_id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(n, 2, "SET ent.eva_reprise = 'oui' n'éteint pas le journal");
    tx.commit().await.unwrap();
    // La source `reprise` n'est écrite ni à l'INSERT ni à l'UPDATE par un écrivain ordinaire, quelles que soient les variables.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.eva_reprise', 'oui', true)")
        .execute(&mut *tx)
        .await
        .unwrap();
    let e = sqlx::query(
        "INSERT INTO eva_resultat (id, etablissement_id, evaluation_id, eleve_id, statut, source, saisi_par)
         VALUES ($1, $2, $3, $4, 'absent', 'reprise', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(ev)
    .bind(w.eleves_a[1])
    .bind(w.prof)
    .execute(&mut *tx)
    .await
    .expect_err("source reprise refusée à l'INSERT");
    assert_eq!(code(&e).as_deref(), Some("23514"));
    drop(tx);
    let e = sqlx::query("UPDATE eva_resultat SET source = 'reprise' WHERE id = $1")
        .bind(r.resultat_id)
        .execute(ent.pool_verif())
        .await
        .expect_err("source reprise refusée à l'UPDATE");
    assert_eq!(code(&e).as_deref(), Some("23514"));
    let e = sqlx::query("UPDATE eva_resultat SET source = 'import' WHERE id = $1")
        .bind(r.resultat_id)
        .execute(ent.pool_verif())
        .await
        .expect_err("la source ne change pas");
    assert_eq!(code(&e).as_deref(), Some("23514"));
    // Après l'arrêt, un motif « forgé » par variable de session n'ouvre rien : seul `motif_correction` de la ligne vaut.
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, dd, 9_000))
        .await
        .unwrap();
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.eva_reprise', 'oui', true), set_config('ent.eva_motif', 'forgé pour le test', true)")
        .execute(&mut *tx)
        .await
        .unwrap();
    let e = sqlx::query(
        "UPDATE eva_resultat SET valeur = 10 WHERE evaluation_id = $1 AND eleve_id = $2",
    )
    .bind(ev)
    .bind(dd)
    .execute(&mut *tx)
    .await
    .expect_err("notes arrêtées : la variable de session ne tient pas lieu de motif");
    assert_eq!(code(&e).as_deref(), Some("23514"));
}

// ───────────────────────── E3 : gel des règles ─────────────────────────

#[tokio::test]
async fn e3_le_premier_arret_fige_les_regles_meme_sans_jeu_pose_et_m3_tout_est_historise() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    // Plancher de 3 sur le seuil des statistiques : refusé par la couche et par la base.
    let trop_bas = ReglesMoteur {
        seuil_effectif_stats: 2,
        ..ReglesMoteur::default()
    };
    assert!(matches!(
        ent.eva_v1_definir_regles_for_actor(w.etab, w.direction, DIR, w.annee, &trop_bas, 10)
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Aucun jeu de règles posé : un établissement « par défaut » arrête T1.
    let (avant,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM eva_reglage WHERE etablissement_id = $1 AND annee_id = $2")
            .bind(w.etab)
            .bind(w.annee)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(avant, 0);
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let (lignes, figees): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(fige_le) FROM eva_reglage WHERE etablissement_id = $1 AND annee_id = $2",
    )
    .bind(w.etab)
    .bind(w.annee)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (lignes, figees),
        (1, 1),
        "la ligne par défaut est créée, figée"
    );
    // Changer l'échelle, ou n'importe quelle règle de calcul, est refusé.
    let courantes = ent.eva_v1_regles(w.etab, w.annee).await.unwrap();
    assert!(courantes.figees);
    let mut dix = courantes.regles;
    dix.echelle_m = 10_000;
    assert!(ent
        .eva_v1_definir_regles_for_actor(w.etab, w.direction, DIR, w.annee, &dix, 10)
        .await
        .is_err());
    let mut zero = courantes.regles;
    zero.non_rendu = TraitementNonRendu::Zero;
    assert!(ent
        .eva_v1_definir_regles_for_actor(w.etab, w.direction, DIR, w.annee, &zero, 10)
        .await
        .is_err());
    let e = sqlx::query(
        "UPDATE eva_reglage SET arrondi_matiere_mode = 'superieur' WHERE etablissement_id = $1",
    )
    .bind(w.etab)
    .execute(ent.pool_verif())
    .await
    .expect_err("un arrondi d'étape est figé aussi");
    assert_eq!(code(&e).as_deref(), Some("23514"));
    let e =
        sqlx::query("UPDATE eva_reglage SET seuil_effectif_stats = 2 WHERE etablissement_id = $1")
            .bind(w.etab)
            .execute(ent.pool_verif())
            .await
            .expect_err("plancher dur de 3 en base");
    assert_eq!(code(&e).as_deref(), Some("23514"));
    // Ce qui ne change aucun calcul reste modifiable (seuil des statistiques) et le gel ne se retire pas.
    let mut six = courantes.regles;
    six.seuil_effectif_stats = 6;
    six.mediane_familles = true;
    ent.eva_v1_definir_regles_for_actor(w.etab, w.direction, DIR, w.annee, &six, 10)
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE eva_reglage SET fige_le = NULL WHERE etablissement_id = $1")
            .bind(w.etab)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    // M3 : l'historique porte tout (modes d'arrondi de chaque étape, seuil du bonus, plafond, statistiques).
    let h = sqlx::query(
        "SELECT h.arrondi_note_ramenee_mode, h.arrondi_sous_matiere_mode, h.arrondi_matiere_mode, h.arrondi_periode_mode,
                h.plafonner_echelle, h.stats_familles, h.mediane_familles, h.seuil_effectif_stats, h.figee, h.version
           FROM eva_reglage_historique h JOIN eva_reglage r ON r.id = h.reglage_id
          WHERE r.etablissement_id = $1 AND r.annee_id = $2 ORDER BY h.version DESC LIMIT 1",
    )
    .bind(w.etab)
    .bind(w.annee)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(h.get::<bool, _>("mediane_familles") && h.get::<bool, _>("figee"));
    assert_eq!(h.get::<i16, _>("seuil_effectif_stats"), 6);
    assert_eq!(h.get::<String, _>("arrondi_matiere_mode"), "demi_haut");
    assert!(h.get::<bool, _>("plafonner_echelle") && h.get::<bool, _>("stats_familles"));
    assert!(
        h.get::<i32, _>("version") >= 2,
        "création figée, puis la pose du seuil"
    );
}

// ───────────────────────── E4 : rattrapage ─────────────────────────

#[tokio::test]
async fn e4_le_rattrapage_expire_et_ne_peut_pas_ecraser_une_note() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation_maths(&ent, &w, d(2031, 10, 10), true).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 14_000))
        .await
        .unwrap();
    ent.eva_v1_ecrire_resultat_for_actor(
        w.etab,
        w.prof,
        PROF,
        &statut(ev, b, StatutCalcul::Absent),
    )
    .await
    .unwrap();
    arreter(&ent, &w, w.t1, w.classe_a).await;
    // Autorisation PÉRIMÉE (posée à la main : la couche refuse de la poser) : aucun effet.
    exec(
        &ent,
        "INSERT INTO eva_rattrapage_autorise (id, evaluation_id, eleve_id, etablissement_id, jusqu_au, motif, par)
         VALUES ($1, $2, $3, $4, '2020-01-01', 'périmée', $5)",
        vec![u(Uuid::new_v4()), u(ev), u(b), u(w.etab), u(w.direction)],
    )
    .await;
    assert!(matches!(
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, b, 11_000))
            .await,
        Err(EntDbError::Forbidden)
    ));
    // La couche refuse aussi de poser une fin déjà passée, et un motif trop court.
    assert!(matches!(
        ent.eva_v1_autoriser_rattrapage_for_actor(
            w.etab,
            w.direction,
            DIR,
            ev,
            b,
            d(2020, 1, 1),
            "trop tard"
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.eva_v1_autoriser_rattrapage_for_actor(
            w.etab,
            w.direction,
            DIR,
            ev,
            b,
            d(2099, 1, 1),
            "x"
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Autorisation en vigueur : le professeur note l'absent, sans correction.
    ent.eva_v1_autoriser_rattrapage_for_actor(
        w.etab,
        w.direction,
        DIR,
        ev,
        b,
        d(2099, 12, 31),
        "absent justifié",
    )
    .await
    .unwrap();
    let ok = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, b, 11_000))
        .await
        .unwrap();
    assert!(!ok.correction);
    // Une note DÉJÀ saisie ne s'écrase pas par ce biais, même avec une autorisation en vigueur.
    ent.eva_v1_autoriser_rattrapage_for_actor(
        w.etab,
        w.direction,
        DIR,
        ev,
        a,
        d(2099, 12, 31),
        "rattrapage",
    )
    .await
    .unwrap();
    assert!(matches!(
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 19_000))
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &note(ev, a, 19_000))
        .await
        .is_err());
    // La base le tient aussi, sans passer par la couche : pas de motif, pas d'écrasement.
    let e = sqlx::query(
        "UPDATE eva_resultat SET valeur = 19 WHERE evaluation_id = $1 AND eleve_id = $2",
    )
    .bind(ev)
    .bind(a)
    .execute(ent.pool_verif())
    .await
    .expect_err("note déjà saisie");
    assert_eq!(code(&e).as_deref(), Some("23514"));
    // Journalisé : une nouvelle autorisation est une nouvelle version ; l'ancienne ne se modifie ni ne se supprime.
    ent.eva_v1_autoriser_rattrapage_for_actor(
        w.etab,
        w.direction,
        DIR,
        ev,
        b,
        d(2099, 12, 31),
        "prolongé",
    )
    .await
    .unwrap();
    let (n, max): (i64, i32) = sqlx::query_as(
        "SELECT count(*), max(version) FROM eva_rattrapage_autorise WHERE evaluation_id = $1 AND eleve_id = $2",
    )
    .bind(ev)
    .bind(b)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((n, max), (3, 3));
    for sql in [
        "UPDATE eva_rattrapage_autorise SET jusqu_au = '2099-01-01'",
        "DELETE FROM eva_rattrapage_autorise",
    ] {
        let e = sqlx::query(sql)
            .execute(ent.pool_verif())
            .await
            .expect_err("ajout seul");
        assert_eq!(code(&e).as_deref(), Some("23514"));
    }
}

// ───────────────────────── M1 : moyennes par acteur ─────────────────────────

#[tokio::test]
async fn m1_les_moyennes_passent_par_les_fonctions_d_acteur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    for (user, voit) in [
        (cast.parent_lea.user_id, true),
        (cast.parent_tom.user_id, false),
    ] {
        ent.link_parent(crate::foyer_repo::CreateLien {
            etablissement_id: w.etab,
            eleve_id: a,
            user_id: user,
            foyer_id: None,
            lien: "responsable".into(),
            peut_voir_notes: voit,
            peut_voir_vs: true,
            peut_justifier_absence: false,
            peut_autoriser_sorties: false,
            est_payeur: false,
            part_paiement: None,
        })
        .await
        .unwrap();
    }
    let publie = evaluation_maths(&ent, &w, d(2031, 10, 10), true).await;
    let prive = evaluation_maths(&ent, &w, d(2031, 10, 17), false).await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(publie, a, 10_000))
        .await
        .unwrap();
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(prive, a, 20_000))
        .await
        .unwrap();
    // Famille : seule l'évaluation publiée entre dans le calcul (le paramètre n'existe pas : il est forcé).
    let fam = ent
        .eva_v1_moyennes_famille_for_actor(w.etab, cast.parent_lea.user_id, "parent", a, w.t1)
        .await
        .unwrap();
    assert_eq!(fam.generale.affichee_m, Some(10_000));
    // Personnel (direction : notes.lire à portée établissement) : tout.
    let pers = ent
        .eva_v1_moyennes_personnel_for_actor(w.etab, w.direction, DIR, a, w.t1)
        .await
        .unwrap();
    assert_eq!(pers.generale.affichee_m, Some(15_000));
    // Négatifs : parent sans droit aux notes, parent sans lien, personnel sans notes.lire, rôle famille côté personnel.
    assert!(matches!(
        ent.eva_v1_moyennes_famille_for_actor(w.etab, cast.parent_tom.user_id, "parent", a, w.t1)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.eva_v1_moyennes_famille_for_actor(
            w.etab,
            cast.parent_lea.user_id,
            "parent",
            w.eleves_a[1],
            w.t1
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.eva_v1_moyennes_personnel_for_actor(w.etab, cast.compta.user_id, "compta", a, w.t1)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.eva_v1_moyennes_personnel_for_actor(w.etab, w.prof, PROF, a, w.t1)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .eva_v1_moyennes_personnel_for_actor(w.etab, cast.parent_lea.user_id, "parent", a, w.t1)
        .await
        .is_err());
    // Élève protégé : le personnel qui a notes.lire mais pas la garde du socle 2 le trouve « introuvable ».
    exec(
        &ent,
        "INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, niveau, motif_scelle, du, decide_par)
         VALUES ($1, $2, $3, 'renforcee', 'enc:v1:mesure', '2020-01-01', $4)",
        vec![u(Uuid::new_v4()), u(w.etab), u(a), u(w.direction)],
    )
    .await;
    assert!(matches!(
        ent.eva_v1_moyennes_personnel_for_actor(
            w.etab,
            cast.administratif.user_id,
            "administratif",
            a,
            w.t1
        )
        .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    assert!(ent
        .eva_v1_moyennes_personnel_for_actor(w.etab, w.direction, DIR, a, w.t1)
        .await
        .is_ok());
}

// ───────────────────────── M2 : coefficient d'un élève sorti ─────────────────────────

#[tokio::test]
async fn m2_un_eleve_sorti_avant_la_fin_de_la_periode_garde_les_coefficients_de_sa_classe() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let sorti = w.eleves_a[0];
    // Coefficients de la classe A : maths 3, français 1 (valables dès la rentrée, posés avant tout résultat).
    for (matiere, coef) in [(w.maths, 3.0f64), (w.francais, 1.0)] {
        sqlx::query(
            "INSERT INTO eva_coefficient_matiere (id, etablissement_id, annee_id, matiere_id, classe_id, coef, valable_du)
             VALUES ($1, $2, $3, $4, $5, $6::float8::numeric, '2031-09-01')",
        )
        .bind(Uuid::new_v4())
        .bind(w.etab)
        .bind(w.annee)
        .bind(matiere)
        .bind(w.classe_a)
        .bind(coef)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    }
    let ev_m = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 10, 10),
        "DS maths",
        20.0,
        1.0,
        Comptage::Normal,
        true,
    )
    .await;
    let ev_f = evaluation(
        &ent,
        &w,
        w.cours_fr,
        d(2031, 10, 12),
        "DS français",
        20.0,
        1.0,
        Comptage::Normal,
        true,
    )
    .await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &note(ev_m, sorti, 12_000))
        .await
        .unwrap();
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &note(ev_f, sorti, 8_000))
        .await
        .unwrap();
    // L'élève quitte l'établissement le 15/12, avant la fin de T1 (20/12).
    exec(
        &ent,
        "UPDATE ent_affectation SET date_fin = $2 WHERE eleve_id = $1",
        vec![u(sorti), dt(d(2031, 12, 15))],
    )
    .await;
    let m = ent
        .eva_v1_moyennes_personnel_for_actor(w.etab, w.direction, DIR, sorti, w.t1)
        .await
        .unwrap();
    // (12 × 3 + 8 × 1) / 4 = 11 : la classe est celle de sa dernière affectation de la période, pas « aucune ».
    assert_eq!(m.generale.affichee_m, Some(11_000));
    assert!(
        m.generale.avertissements.is_empty(),
        "coefficients trouvés : rien à signaler"
    );
    let coefs: Vec<i64> = m.matieres.iter().map(|x| x.coef_m).collect();
    assert!(
        coefs.contains(&3_000) && coefs.contains(&1_000),
        "{coefs:?}"
    );
}

#[tokio::test]
async fn m2_sans_coefficient_trouve_le_calcul_le_dit_au_lieu_de_supposer_un() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev_m = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 10, 10),
        "DS maths",
        20.0,
        1.0,
        Comptage::Normal,
        true,
    )
    .await;
    let ev_f = evaluation(
        &ent,
        &w,
        w.cours_fr,
        d(2031, 10, 12),
        "DS français",
        20.0,
        1.0,
        Comptage::Normal,
        true,
    )
    .await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &note(ev_m, a, 12_000))
        .await
        .unwrap();
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &note(ev_f, a, 8_000))
        .await
        .unwrap();
    let m = ent
        .eva_v1_moyennes_personnel_for_actor(w.etab, w.direction, DIR, a, w.t1)
        .await
        .unwrap();
    assert_eq!(m.generale.affichee_m, Some(10_000));
    assert_eq!(
        m.generale.avertissements.len(),
        2,
        "une alerte par matière sans coefficient"
    );
    let texte = m.generale.explication_fr(&m.regles).join("\n");
    assert!(
        texte.contains("Coefficient de la matière introuvable"),
        "{texte}"
    );
}

// ───────────────────────── M3 / M4 : coefficients datés, évaluation figée ─────────────────────────

#[tokio::test]
async fn m3_un_coefficient_n_est_jamais_retroactif_et_garde_son_historique() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation_maths(&ent, &w, d(2031, 10, 10), true).await;
    // Sans résultat, on peut poser un coefficient daté d'avant.
    let poser = |matiere: Uuid, du: &'static str, coef: f64| {
        let (pool, etab, annee) = (ent.pool_verif().clone(), w.etab, w.annee);
        async move {
            sqlx::query(
                "INSERT INTO eva_coefficient_sous_matiere (id, etablissement_id, annee_id, matiere_id, coef, valable_du)
                 VALUES ($1, $2, $3, $4, $5::float8::numeric, $6::date)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(annee)
            .bind(matiere)
            .bind(coef)
            .bind(du)
            .execute(&pool)
            .await
        }
    };
    poser(w.francais, "2031-09-01", 2.0)
        .await
        .expect("sans résultat : possible");
    // Même sous-matière : chevauchement refusé (une seule valeur à une date).
    assert!(poser(w.francais, "2031-10-01", 3.0).await.is_err());
    // Matière de coefficient : même garde, la ligne est datée et ne se modifie plus.
    exec(
        &ent,
        "INSERT INTO eva_coefficient_matiere (id, etablissement_id, annee_id, matiere_id, classe_id, coef, valable_du)
         VALUES ($1, $2, $3, $4, $5, 2, '2031-09-01')",
        vec![u(Uuid::new_v4()), u(w.etab), u(w.annee), u(w.maths), u(w.classe_a)],
    )
    .await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, w.eleves_a[0], 12_000))
        .await
        .unwrap();
    for sql in [
        "UPDATE eva_coefficient_matiere SET coef = 5 WHERE etablissement_id = $1",
        "DELETE FROM eva_coefficient_matiere WHERE etablissement_id = $1",
        "UPDATE eva_coefficient_sous_matiere SET coef = 5 WHERE etablissement_id = $1",
        "DELETE FROM eva_coefficient_sous_matiere WHERE etablissement_id = $1",
    ] {
        let e = sqlx::query(sql)
            .bind(w.etab)
            .execute(ent.pool_verif())
            .await
            .expect_err(sql);
        assert_eq!(code(&e).as_deref(), Some("23514"), "{sql} ({e})");
    }
    // Rétroactif : un coefficient de maths « par défaut » daté d'avant le DS déjà noté est refusé ; daté après
    // tous les résultats, il est accepté. (Critère de données : aucune horloge.)
    let defaut = |du: &'static str| {
        let (pool, etab, annee, maths) = (ent.pool_verif().clone(), w.etab, w.annee, w.maths);
        async move {
            sqlx::query(
                "INSERT INTO eva_coefficient_matiere (id, etablissement_id, annee_id, matiere_id, defaut, coef, valable_du)
                 VALUES ($1, $2, $3, $4, true, 4, $5::date)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(annee)
            .bind(maths)
            .bind(du)
            .execute(&pool)
            .await
        }
    };
    let e = defaut("2031-10-01")
        .await
        .expect_err("rétroactif : des résultats existent depuis cette date");
    assert_eq!(code(&e).as_deref(), Some("23514"));
    defaut("2032-01-01")
        .await
        .expect("après le dernier résultat : possible");
    // Clore une ligne avant des résultats existants est refusé aussi ; la clore après est possible.
    let e = sqlx::query(
        "UPDATE eva_coefficient_matiere SET valable_au = '2031-09-30' WHERE classe_id IS NOT NULL",
    )
    .execute(ent.pool_verif())
    .await
    .expect_err("fin avant des résultats");
    assert_eq!(code(&e).as_deref(), Some("23514"));
    sqlx::query(
        "UPDATE eva_coefficient_matiere SET valable_au = '2031-12-31' WHERE classe_id IS NOT NULL",
    )
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

#[tokio::test]
async fn m4_une_evaluation_qui_porte_des_resultats_ne_change_plus_de_bareme_ni_de_coefficient_sans_motif(
) {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let libre = evaluation_maths(&ent, &w, d(2031, 10, 10), true).await;
    let notee = evaluation_maths(&ent, &w, d(2031, 10, 17), true).await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(notee, w.eleves_a[0], 12_000))
        .await
        .unwrap();
    // Sans résultat ni arrêt : modifiable (et journalisé, sans motif exigé).
    exec(
        &ent,
        "UPDATE eva_evaluation SET coef = 2 WHERE id = $1",
        vec![u(libre)],
    )
    .await;
    let (n, corr): (i64, bool) = sqlx::query_as(
        "SELECT count(*), bool_or(correction) FROM eva_evaluation_historique WHERE evaluation_id = $1",
    )
    .bind(libre)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((n, corr), (1, false));
    // Avec un résultat : chaque champ du calcul est refusé sans motif.
    for (champ, valeur) in [
        ("bareme", "10"),
        ("coef", "3"),
        ("comptage", "'bonus'"),
        ("mode", "'positionnement'"),
        ("hors_bareme_autorise", "true"),
        ("date_eval", "'2031-10-18'"),
    ] {
        let e = sqlx::query(&format!(
            "UPDATE eva_evaluation SET {champ} = {valeur} WHERE id = $1"
        ))
        .bind(notee)
        .execute(ent.pool_verif())
        .await
        .expect_err(champ);
        assert_eq!(code(&e).as_deref(), Some("23514"), "{champ} ({e})");
    }
    // Par une correction motivée : acceptée, journalisée avec auteur et motif ; le motif ne survit pas.
    exec(
        &ent,
        "UPDATE eva_evaluation SET coef = 3, motif_correction = 'coefficient annoncé erroné', modifie_par = $2 WHERE id = $1",
        vec![u(notee), u(w.direction)],
    )
    .await;
    let h = sqlx::query(
        "SELECT correction, motif, par, coef_avant::text AS avant, coef_apres::text AS apres
           FROM eva_evaluation_historique WHERE evaluation_id = $1",
    )
    .bind(notee)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(h.get::<bool, _>("correction"));
    assert_eq!(
        h.get::<Option<String>, _>("motif").as_deref(),
        Some("coefficient annoncé erroné")
    );
    assert_eq!(h.get::<Option<Uuid>, _>("par"), Some(w.direction));
    assert_eq!(
        (
            h.get::<String, _>("avant").as_str(),
            h.get::<String, _>("apres").as_str()
        ),
        ("1.000", "3.000")
    );
    let (reste,): (Option<String>,) =
        sqlx::query_as("SELECT motif_correction FROM eva_evaluation WHERE id = $1")
            .bind(notee)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert!(reste.is_none());
    // Période arrêtée, sans résultat : le barème ne change pas non plus sans motif.
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let e = sqlx::query("UPDATE eva_evaluation SET bareme = 40 WHERE id = $1")
        .bind(libre)
        .execute(ent.pool_verif())
        .await
        .expect_err("période arrêtée");
    assert_eq!(code(&e).as_deref(), Some("23514"));
    // Le journal de l'évaluation est en ajout seul.
    let e = sqlx::query("DELETE FROM eva_evaluation_historique")
        .execute(ent.pool_verif())
        .await
        .expect_err("ajout seul");
    assert_eq!(code(&e).as_deref(), Some("23514"));
}

// ───────────────────────── M5 : vue de période, F3 appréciations, F6 réouverture, F11 sous-matière ─────────────────────────

#[tokio::test]
async fn m5_le_statut_de_periode_compte_toutes_les_classes_dont_celles_sans_etat() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let np = Uuid::new_v4();
    exec(
        &ent,
        "INSERT INTO ent_note_periode (id, etablissement_id, code, libelle, date_debut, date_fin, statut, periode_id)
         VALUES ($1, $2, 'T1-REVUE', 'T1', '2031-09-01', '2031-12-20', 'ouverte', $3)",
        vec![u(np), u(w.etab), u(w.t1)],
    )
    .await;
    // Classe A arrêtée, classe D sans ligne d'état : la période reste « ouverte » pour l'ancien lecteur.
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let (statut, closes): (String, bool) =
        sqlx::query_as("SELECT statut, appreciations_closes FROM ent_note_periode_v WHERE id = $1")
            .bind(np)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!((statut.as_str(), closes), ("ouverte", false));
    arreter(&ent, &w, w.t1, w.classe_d).await;
    let (statut, closes): (String, bool) =
        sqlx::query_as("SELECT statut, appreciations_closes FROM ent_note_periode_v WHERE id = $1")
            .bind(np)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(
        (statut.as_str(), closes),
        ("partielle_appreciations", false)
    );
}

#[tokio::test]
async fn f3_f6_f11_appreciation_jetee_reouverture_et_sous_matiere() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    // F3 : le journal garde l'éditeur de CHAQUE version ; un brouillon se jette (état `jete`), jamais par DELETE.
    let ap = Uuid::new_v4();
    exec(
        &ent,
        "INSERT INTO eva_appreciation (id, etablissement_id, periode_id, eleve_id, type, texte, etat, auteur, modifie_par)
         VALUES ($1, $2, $3, $4, 'generale', 'Premier jet', 'brouillon', $5, $5)",
        vec![u(ap), u(w.etab), u(w.t1), u(a), u(w.prof)],
    )
    .await;
    exec(
        &ent,
        "UPDATE eva_appreciation SET texte = 'Repris par la direction', modifie_par = $2 WHERE id = $1",
        vec![u(ap), u(w.direction)],
    )
    .await;
    let auteurs: Vec<(i32, Uuid)> = sqlx::query_as(
        "SELECT version, auteur FROM eva_appreciation_historique WHERE appreciation_id = $1 ORDER BY version",
    )
    .bind(ap)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(auteurs, vec![(1, w.prof), (2, w.direction)]);
    let e = sqlx::query("DELETE FROM eva_appreciation WHERE id = $1")
        .bind(ap)
        .execute(ent.pool_verif())
        .await
        .expect_err("plus de DELETE");
    assert_eq!(code(&e).as_deref(), Some("23514"));
    exec(
        &ent,
        "UPDATE eva_appreciation SET etat = 'jete', modifie_par = $2 WHERE id = $1",
        vec![u(ap), u(w.prof)],
    )
    .await;
    // Le brouillon jeté libère la place : un nouveau brouillon du même type est possible ; un jeté ne revient pas.
    exec(
        &ent,
        "INSERT INTO eva_appreciation (id, etablissement_id, periode_id, eleve_id, type, texte, etat, auteur)
         VALUES ($1, $2, $3, $4, 'generale', 'Nouveau', 'brouillon', $5)",
        vec![u(Uuid::new_v4()), u(w.etab), u(w.t1), u(a), u(w.prof)],
    )
    .await;
    assert!(
        sqlx::query("UPDATE eva_appreciation SET etat = 'brouillon' WHERE id = $1")
            .bind(ap)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    // F6 : la réouverture vide la date d'arrêt effective de la classe.
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let (date,): (Option<chrono::NaiveDate>,) = sqlx::query_as(
        "SELECT date_arret_effective FROM eva_periode_etat WHERE periode_id = $1 AND classe_id = $2",
    )
    .bind(w.t1)
    .bind(w.classe_a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(date, Some(d(2031, 12, 5)));
    ent.eva_v1_changer_etat_periode_for_actor(
        w.etab,
        w.direction,
        DIR,
        w.t1,
        w.classe_a,
        "ouverte",
        Some("erreur d'arrêt"),
        None,
    )
    .await
    .unwrap();
    let (date, etat): (Option<chrono::NaiveDate>, String) = sqlx::query_as(
        "SELECT date_arret_effective, etat FROM eva_periode_etat WHERE periode_id = $1 AND classe_id = $2",
    )
    .bind(w.t1)
    .bind(w.classe_a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((date, etat.as_str()), (None, "ouverte"));
    // F11 : l'évaluation ne vise qu'une sous-matière de la matière de son cours.
    let sous = Uuid::new_v4();
    exec(
        &ent,
        "INSERT INTO ent_matiere (id, etablissement_id, code, libelle, parent_id) VALUES ($1, $2, $3, 'Algèbre', $4)",
        vec![u(sous), u(w.etab), t(&format!("ALG{}", &Uuid::new_v4().simple().to_string()[..5])), u(w.maths)],
    )
    .await;
    let ev = evaluation_maths(&ent, &w, d(2031, 10, 10), true).await;
    exec(
        &ent,
        "UPDATE eva_evaluation SET matiere_id = $2 WHERE id = $1",
        vec![u(ev), u(sous)],
    )
    .await;
    for (cible, nom) in [
        (w.francais, "une autre matière"),
        (w.maths, "la matière elle-même"),
    ] {
        let e = sqlx::query("UPDATE eva_evaluation SET matiere_id = $2 WHERE id = $1")
            .bind(ev)
            .bind(cible)
            .execute(ent.pool_verif())
            .await
            .expect_err(nom);
        assert_eq!(code(&e).as_deref(), Some("23514"), "{nom}");
    }
}

// ───────────────────────── M8 : verrouillage sans période, arrêt automatique ─────────────────────────

#[tokio::test]
async fn m8_un_eleve_sans_periode_a_la_date_est_verrouille_sauf_correction() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    // 15/04/2032 : dans l'année (jusqu'au 30/06) mais après T2 (31/03) : aucune période, donc verrouillé.
    let ev = evaluation_maths(&ent, &w, d(2032, 4, 15), true).await;
    let a = w.eleves_a[0];
    assert!(ent
        .eva_v1_periode_eleve(w.etab, ev, a)
        .await
        .unwrap()
        .is_none());
    assert!(matches!(
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 12_000))
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &note(ev, a, 12_000))
        .await
        .is_err());
    let mut corr = note(ev, a, 12_000);
    corr.motif = Some("évaluation hors période".into());
    let c = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &corr)
        .await
        .unwrap();
    assert!(c.correction);
    // La base le tient aussi : écriture directe sans motif refusée.
    let e = sqlx::query("UPDATE eva_resultat SET valeur = 13 WHERE evaluation_id = $1")
        .bind(ev)
        .execute(ent.pool_verif())
        .await
        .expect_err("sans période : verrouillé");
    assert_eq!(code(&e).as_deref(), Some("23514"));
    // La grille ne propose pas la saisie à cet élève.
    let grille = ent
        .eva_v1_grille_eleves_for_actor(w.etab, w.prof, PROF, ev)
        .await
        .unwrap();
    assert!(grille.iter().all(|c| !c.saisie_ouverte));
}

#[tokio::test]
async fn m8_arret_automatique_a_la_date_d_arret_idempotent_et_jamais_apres_une_reouverture() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    // La veille de la date d'arrêt (05/12/2031) : rien.
    assert_eq!(
        ent.eva_v1_arret_auto_au(w.etab, Some(d(2031, 12, 4)))
            .await
            .unwrap(),
        0
    );
    // Le jour même : les deux classes de la périodisation par défaut (T1) passent en `notes_arretees`, origine `auto`.
    assert_eq!(
        ent.eva_v1_arret_auto_au(w.etab, Some(d(2031, 12, 5)))
            .await
            .unwrap(),
        2
    );
    let etats: Vec<(String, String, Option<chrono::NaiveDate>)> = sqlx::query_as(
        "SELECT etat, origine, date_arret_effective FROM eva_periode_etat WHERE etablissement_id = $1 AND periode_id = $2",
    )
    .bind(w.etab)
    .bind(w.t1)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(etats.len(), 2);
    assert!(etats
        .iter()
        .all(|(e, o, dt)| e == "notes_arretees" && o == "auto" && *dt == Some(d(2031, 12, 5))));
    let (transitions,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM eva_periode_transition WHERE etablissement_id = $1 AND origine = 'auto'",
    )
    .bind(w.etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(transitions, 2);
    // Idempotent ; le gel des règles a eu lieu.
    assert_eq!(
        ent.eva_v1_arret_auto_au(w.etab, Some(d(2031, 12, 5)))
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        ent.eva_v1_arret_auto_au(w.etab, Some(d(2032, 1, 20)))
            .await
            .unwrap(),
        0
    );
    assert!(ent.eva_v1_regles(w.etab, w.annee).await.unwrap().figees);
    // Rouvert par la direction : jamais ré-arrêté par le planificateur.
    ent.eva_v1_changer_etat_periode_for_actor(
        w.etab,
        w.direction,
        DIR,
        w.t1,
        w.classe_a,
        "ouverte",
        Some("saisie à compléter"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        ent.eva_v1_arret_auto_au(w.etab, Some(d(2031, 12, 10)))
            .await
            .unwrap(),
        0
    );
    // Passage quotidien : l'étape est branchée sur le planificateur (T2 le 20/03/2032) et compte ses classes.
    let b = ent
        .passages_quotidiens_etablissement(w.etab, Some(d(2032, 3, 20)))
        .await;
    assert_eq!((b.periodes_arretees, b.erreurs), (2, 0), "{b:?}");
    let b = ent
        .passages_quotidiens_etablissement(w.etab, Some(d(2032, 3, 20)))
        .await;
    assert_eq!(b.periodes_arretees, 0, "idempotent : {b:?}");
}

#[tokio::test]
async fn m8_le_reglage_arret_auto_desactive_l_arret_automatique() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    ent.eva_v1_definir_regles_for_actor(
        w.etab,
        w.direction,
        DIR,
        w.annee,
        &ReglesMoteur::default(),
        10,
    )
    .await
    .unwrap();
    exec(
        &ent,
        "UPDATE eva_reglage SET arret_auto = false WHERE etablissement_id = $1",
        vec![u(w.etab)],
    )
    .await;
    assert_eq!(
        ent.eva_v1_arret_auto_au(w.etab, Some(d(2031, 12, 10)))
            .await
            .unwrap(),
        0
    );
}

// ───────────────────────── M9 : la grille est celle du titulaire ou de notes.lire ─────────────────────────

#[tokio::test]
async fn m9_la_grille_n_est_lisible_que_du_titulaire_ou_d_un_detenteur_de_notes_lire() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation_maths(&ent, &w, d(2031, 10, 10), false).await;
    // Titulaire du cours de maths : oui.
    assert_eq!(
        ent.eva_v1_grille_eleves_for_actor(w.etab, w.prof, PROF, ev)
            .await
            .unwrap()
            .len(),
        3
    );
    // Le professeur de FRANÇAIS (autre professeur, sans notes.lire à portée établissement) : non.
    let autre_prof = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)
        .unwrap();
    assert!(matches!(
        ent.eva_v1_grille_eleves_for_actor(w.etab, autre_prof, PROF, ev)
            .await,
        Err(EntDbError::Forbidden)
    ));
    // La comptabilité (ni titulaire, ni notes.lire) : non.
    assert!(matches!(
        ent.eva_v1_grille_eleves_for_actor(w.etab, cast.compta.user_id, "compta", ev)
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Direction et gestion : notes.lire à portée établissement (modèle par défaut : direction, administratif, CPE).
    for (user, role) in [
        (w.direction, DIR),
        (cast.administratif.user_id, "administratif"),
    ] {
        assert_eq!(
            ent.eva_v1_grille_eleves_for_actor(w.etab, user, role, ev)
                .await
                .unwrap()
                .len(),
            3,
            "{role}"
        );
    }
    // Évaluation inconnue : introuvable, pas « droit refusé » (aucune fuite d'existence).
    assert!(matches!(
        ent.eva_v1_grille_eleves_for_actor(w.etab, w.direction, DIR, Uuid::new_v4())
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
}

// ───────────────────────── F4 : une écriture sans effet ne publie rien ─────────────────────────

#[tokio::test]
async fn f4_f5_une_ecriture_sans_effet_ne_publie_rien_et_le_journal_garde_le_reste_de_la_ligne() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation_maths(&ent, &w, d(2031, 10, 10), true).await;
    let a = w.eleves_a[0];
    let compter = || async {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type LIKE 'evaluations.%'",
        )
        .bind(w.etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        n
    };
    let r1 = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 12_000))
        .await
        .unwrap();
    let apres_premiere = compter().await;
    let r2 = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 12_000))
        .await
        .unwrap();
    assert_eq!(r2.version, r1.version, "écriture sans effet : même version");
    assert_eq!(
        compter().await,
        apres_premiere,
        "écriture sans effet : aucun événement"
    );
    let (histo,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM eva_resultat_historique WHERE resultat_id = $1")
            .bind(r1.resultat_id)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(
        histo, 1,
        "aucune ligne de journal pour une écriture sans effet"
    );
    // F5 : un changement de commentaire, d'aménagement, de rattrapage ou de note privée (même valeur de note)
    // est journalisé, avec l'avant et l'après.
    exec(
        &ent,
        "UPDATE eva_resultat SET commentaire_eleve = 'Bien', amenage = true, note_privee = 'à suivre' WHERE id = $1",
        vec![u(r1.resultat_id)],
    )
    .await;
    let h = sqlx::query(
        "SELECT commentaire_avant, commentaire_apres, amenage_avant, amenage_apres, note_privee_apres
           FROM eva_resultat_historique WHERE resultat_id = $1 AND version = 2",
    )
    .bind(r1.resultat_id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(h.get::<Option<String>, _>("commentaire_avant"), None);
    assert_eq!(
        h.get::<Option<String>, _>("commentaire_apres").as_deref(),
        Some("Bien")
    );
    assert_eq!(
        (
            h.get::<Option<bool>, _>("amenage_avant"),
            h.get::<Option<bool>, _>("amenage_apres")
        ),
        (Some(false), Some(true))
    );
    assert_eq!(
        h.get::<Option<String>, _>("note_privee_apres").as_deref(),
        Some("à suivre")
    );
}

// ───────────────────────── F9 : les cours « de reprise » hors des publics et de l'EDT ─────────────────────────

#[tokio::test]
async fn f9_un_cours_de_reprise_n_apparait_ni_dans_les_publics_ni_dans_l_edt() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let reprise = Uuid::new_v4();
    exec(
        &ent,
        "INSERT INTO ent_cours (id, etablissement_id, annee_id, libelle, matiere_id, classe_id, ouvert_le, source, a_verifier)
         VALUES ($1, $2, $3, 'Reprise', $4, $5, '2031-09-01', 'reprise', true)",
        vec![u(reprise), u(w.etab), u(w.annee), u(w.francais), u(w.classe_d)],
    )
    .await;
    let eleve = w.eleves_d[0];
    // Publics : l'élève ne « suit » pas ce cours dans les listes ; l'appel explicite par identifiant fonctionne
    // toujours (le contrôle des résultats d'une évaluation reprise en a besoin).
    let (dans_la_liste,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_public_cours_jour($1, '2031-10-10', NULL, NULL, $2) WHERE cours_id = $3",
    )
    .bind(w.etab)
    .bind(eleve)
    .bind(reprise)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(dans_la_liste, 0);
    let (explicite,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_public_cours_jour($1, '2031-10-10', $2) WHERE eleve_id = $3",
    )
    .bind(w.etab)
    .bind(reprise)
    .bind(eleve)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(explicite, 1);
    // EDT : aucune séance ni aucun plan ne nomme ce cours (une séance ne naît que d'un plan qui désigne le cours).
    for (table, colonne) in [
        ("ent_edt_placement", "cours_id"),
        ("ent_edt_seance", "cours_id"),
    ] {
        let (n,): (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM {table} WHERE {colonne} = $1"
        ))
        .bind(reprise)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
        assert_eq!(
            n, 0,
            "{table} : aucun enregistrement pour un cours de reprise"
        );
    }
}
