//! Socle 21, lot V3c « revue de sécurité de V3b » : tests sur base (feature `labo`). Aucune horloge réelle : jour fixe
//! 2031-10-20, dates d'évaluation fixes (monde V1 : année 2031-2032).

use super::*;
use crate::evaluations::{Aff, CalculClasse, PeriodeRow, StatsAff};
use crate::evaluations_v1::EcrireResultat;
use crate::tests_evaluations::labo;
use crate::tests_evaluations_v1::{
    ancien_monde, arreter, d, evaluation, exec, monde_v1, note, statut as statut_res, u, DIR, PROF,
};
use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::evaluation_moteur::{Comptage, Rat, ReglesMoteur, Statistiques, StatutCalcul};
use domain_eleve::NotePeriodeStatut;
use ent_types::evaluations_v3::*;
use std::collections::HashMap;
use uuid::Uuid;

fn jour() -> NaiveDate {
    d(2031, 10, 20)
}

fn maintenant() -> DateTime<Utc> {
    jour().and_hms_opt(12, 0, 0).unwrap().and_utc()
}

fn corps(
    cours: Uuid,
    titre: &str,
    date: NaiveDate,
    annoncee: bool,
    restreinte_a: Vec<Uuid>,
) -> CreerEvaluationV3Body {
    CreerEvaluationV3Body {
        cours_id: cours,
        titre: titre.into(),
        date_eval: date,
        bareme_m: 20_000,
        coef_m: 1_000,
        comptage: None,
        hors_bareme_autorise: false,
        sous_matiere_id: None,
        restreinte_a,
        publication: None,
        publication_prevue_le: None,
        annoncee,
    }
}

// ───────────────────────── M1 : auteur et attestation exigés par la base ─────────────────────────

#[tokio::test]
async fn v3c_m1_la_base_exige_un_auteur_et_l_attestation_apres_l_arret() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 10, 10),
        "DS",
        20.0,
        1.0,
        Comptage::Normal,
        false,
    )
    .await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 10_000))
        .await
        .unwrap();
    // INSERT sans auteur : refusé (avant : accepté par un déclencheur qui ne comparait que des NULL).
    let r = sqlx::query(
        "INSERT INTO eva_resultat (id, etablissement_id, evaluation_id, eleve_id, statut, valeur, saisi_par)
         VALUES ($1, $2, $3, $4, 'note', 12, NULL)",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(ev)
    .bind(b)
    .execute(ent.pool_verif())
    .await;
    let e = r
        .expect_err("un résultat sans auteur est refusé")
        .to_string();
    assert!(e.contains("auteur obligatoire"), "{e}");
    // UPDATE qui efface l'auteur : refusé aussi (période ouverte).
    let r = sqlx::query(
        "UPDATE eva_resultat SET valeur = 11, saisi_par = NULL WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(w.etab)
    .bind(ev)
    .bind(a)
    .execute(ent.pool_verif())
    .await;
    assert!(r
        .expect_err("auteur effacé")
        .to_string()
        .contains("auteur obligatoire"));

    // Après l'arrêt : un motif seul ne suffit pas ; l'auteur NULL non plus ; l'attestation de `notes.corriger` oui.
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let maj = |auteur: Option<Uuid>| {
        let (etab, ev, a) = (w.etab, ev, a);
        let pool = ent.pool_verif().clone();
        async move {
            sqlx::query(
                "UPDATE eva_resultat SET valeur = 13, motif_correction = 'correction de test', saisi_par = $4
                  WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
            )
            .bind(etab)
            .bind(ev)
            .bind(a)
            .bind(auteur)
            .execute(&pool)
            .await
        }
    };
    let e = maj(None)
        .await
        .expect_err("auteur NULL après l'arrêt")
        .to_string();
    assert!(e.contains("auteur obligatoire"), "{e}");
    let e = maj(Some(w.direction))
        .await
        .expect_err("sans attestation")
        .to_string();
    assert!(e.contains("attestation absente"), "{e}");
    // Attestation posée dans la transaction (ce que fait l'application après contrôle du droit) : acceptée.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('app.eva_correcteur', $1, true)")
        .bind(w.direction.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE eva_resultat SET valeur = 13, motif_correction = 'correction de test', saisi_par = $4
          WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(w.etab)
    .bind(ev)
    .bind(a)
    .bind(w.direction)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    // Une attestation au nom d'un autre auteur ne couvre pas cette écriture.
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('app.eva_correcteur', $1, true)")
        .bind(w.prof.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let r = sqlx::query(
        "UPDATE eva_resultat SET valeur = 14, motif_correction = 'correction de test', saisi_par = $4
          WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(w.etab)
    .bind(ev)
    .bind(a)
    .bind(w.direction)
    .execute(&mut *tx)
    .await;
    assert!(r.is_err(), "attestation d'un autre auteur");
}

// ───────────────────────── M3 : période ouverte = professeur, même par l'ancienne voie ─────────────────────────

#[tokio::test]
async fn v3c_m3_la_direction_ne_saisit_pas_a_la_place_du_professeur_en_periode_ouverte() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 10, 10),
        "DS",
        20.0,
        1.0,
        Comptage::Normal,
        false,
    )
    .await;
    let a = w.eleves_a[0];
    // L'écriture commune (celle que toutes les voies empruntent) refuse la direction : la règle ne vit plus seulement
    // dans l'écran de grille.
    let r = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &note(ev, a, 10_000))
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 10_000))
        .await
        .unwrap();
    // La direction ne remplace pas non plus une note existante tant que la période est ouverte.
    let r = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &note(ev, a, 15_000))
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Après l'arrêt, elle corrige avec un motif (R-V10) : la règle n'a pas supprimé le droit de correction.
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let corr = EcrireResultat {
        motif: Some("correction après l'arrêt".into()),
        ..note(ev, a, 15_000)
    };
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.direction, DIR, &corr)
        .await
        .unwrap();
}

// ───────────────────────── M2 : la vue de compatibilité ne rend plus 0 ─────────────────────────

#[tokio::test]
async fn v3c_m2_la_vue_ent_note_ne_rend_jamais_zero_pour_un_statut() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = evaluation(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 10, 10),
        "DS",
        20.0,
        1.0,
        Comptage::Normal,
        true,
    )
    .await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 12_000))
        .await
        .unwrap();
    ent.eva_v1_ecrire_resultat_for_actor(
        w.etab,
        w.prof,
        PROF,
        &statut_res(ev, b, StatutCalcul::Absent),
    )
    .await
    .unwrap();
    ent.eva_v1_ecrire_resultat_for_actor(
        w.etab,
        w.prof,
        PROF,
        &statut_res(ev, c, StatutCalcul::Dispense),
    )
    .await
    .unwrap();
    let lignes: Vec<(Uuid, Option<f64>, String)> = sqlx::query_as(
        "SELECT eleve_id, valeur, statut FROM ent_note WHERE etablissement_id = $1 AND devoir_id = $2 ORDER BY eleve_id",
    )
    .bind(w.etab)
    .bind(ev)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    let ligne = |x: Uuid| lignes.iter().find(|l| l.0 == x).unwrap().clone();
    assert_eq!((ligne(a).1, ligne(a).2.as_str()), (Some(12.0), "note"));
    assert_eq!(
        (ligne(b).1, ligne(b).2.as_str()),
        (None, "absent"),
        "absent : pas de 0 de substitution"
    );
    assert_eq!(
        (ligne(c).1, ligne(c).2.as_str()),
        (None, "dispense"),
        "dispensé : pas de 0 de substitution"
    );
}

// ───────────────────────── M3 (agenda) : annoncée et public restreint ─────────────────────────

#[tokio::test]
async fn v3c_m3_l_agenda_ne_montre_que_les_evaluations_annoncees_et_respecte_le_public_restreint() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    let creer = |titre: &str, date: NaiveDate, annoncee: bool, restreinte: Vec<Uuid>| {
        let c = corps(w.cours_maths, titre, date, annoncee, restreinte);
        let (ent, etab, prof) = (&ent, w.etab, w.prof);
        async move {
            ent.eva_v3_creer_evaluation_for_actor(etab, prof, PROF, jour(), maintenant(), &c)
                .await
                .unwrap()
        }
    };
    creer("Secrète", d(2031, 10, 25), false, vec![]).await;
    creer("Annoncée", d(2031, 10, 27), true, vec![]).await;
    creer("Demi-groupe", d(2031, 10, 29), true, vec![b]).await;
    let mut conn = ent.pool_verif().acquire().await.unwrap();
    let rows = PgEnt::eva_calendrier(
        &mut conn,
        w.etab,
        &[a, b],
        d(2031, 10, 21),
        d(2031, 11, 30),
        jour(),
    )
    .await
    .unwrap();
    let titres = |e: Uuid| -> Vec<String> {
        rows.iter()
            .filter(|r| r.eleve_id == e)
            .map(|r| r.evaluation.titre.clone())
            .collect()
    };
    assert_eq!(
        titres(a),
        vec!["Annoncée".to_string()],
        "ni la non annoncée, ni celle d'un demi-groupe"
    );
    assert_eq!(
        titres(b),
        vec!["Annoncée".to_string(), "Demi-groupe".to_string()]
    );
}

// ───────────────────────── M4 : statistiques de classe du bulletin sous le seuil ─────────────────────────

#[test]
fn v3c_m4_les_statistiques_de_classe_du_bulletin_respectent_le_seuil_des_familles() {
    let matiere = Uuid::new_v4();
    let stats = |effectif: usize| StatsAff {
        moyenne: Aff(12_000),
        min: Aff(8_000),
        max: Aff(16_000),
        effectif,
        brut: Statistiques {
            effectif,
            moyenne: Rat::new(12, 1).unwrap(),
            min: Rat::new(8, 1).unwrap(),
            max: Rat::new(16, 1).unwrap(),
            mediane: Rat::new(12, 1).unwrap(),
        },
    };
    let calc = |effectif: usize, seuil: usize| CalculClasse {
        classe_id: Uuid::new_v4(),
        classe_code: "4A".into(),
        periode: PeriodeRow {
            id: Uuid::new_v4(),
            code: "T1".into(),
            libelle: "T1".into(),
            date_debut: d(2031, 9, 1),
            date_fin: d(2031, 12, 20),
            statut: NotePeriodeStatut::Fermee,
        },
        arrondi: "centieme".into(),
        regles: ReglesMoteur {
            seuil_effectif_stats: seuil,
            ..ReglesMoteur::default()
        },
        eleves: vec![],
        matieres: vec![],
        moyennes: HashMap::new(),
        generales: HashMap::new(),
        stats_matiere: HashMap::from([(matiere, stats(effectif))]),
        stats_generale: Some(stats(effectif)),
    };
    // Seuil par défaut (5) : 4 élèves notés → rien ; 5 → montré.
    assert!(calc(4, 5).stats_matiere_montrables(&matiere).is_none());
    assert!(calc(4, 5).stats_generale_montrables().is_none());
    assert!(calc(5, 5).stats_matiere_montrables(&matiere).is_some());
    assert!(calc(5, 5).stats_generale_montrables().is_some());
    // Plancher : un seuil réglé à 3 laisse passer 3 élèves, jamais 2 (une moyenne de deux élèves identifie l'autre).
    assert!(calc(3, 3).stats_matiere_montrables(&matiere).is_some());
    assert!(calc(2, 3).stats_matiere_montrables(&matiere).is_none());
    assert!(calc(2, 3).stats_generale_montrables().is_none());
    // V3d (M-3) : réglage `stats_familles = false` → ni moyenne ni min ni max, même largement au-dessus du seuil.
    let mut coupe = calc(30, 5);
    coupe.regles.stats_familles = false;
    assert!(coupe.stats_matiere_montrables(&matiere).is_none());
    assert!(coupe.stats_generale_montrables().is_none());
}

// ───────────────────────── Faibles : lot de cases sans interblocage, ordre de réponse conservé ─────────────────────────

#[tokio::test]
async fn v3c_deux_lots_croises_ne_s_interbloquent_pas_et_la_reponse_suit_l_ordre_de_la_requete() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let mut evs = Vec::new();
    for (i, titre) in ["DS 1", "DS 2"].iter().enumerate() {
        let c = corps(
            w.cours_maths,
            titre,
            d(2031, 10, 10 + i as u32),
            false,
            vec![],
        );
        evs.push(
            ent.eva_v3_creer_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), &c)
                .await
                .unwrap()
                .id,
        );
    }
    let mut cases: Vec<(Uuid, Uuid)> = Vec::new();
    for ev in &evs {
        for e in &w.eleves_a {
            cases.push((*ev, *e));
        }
    }
    let lot = |ordre: Vec<(Uuid, Uuid)>, texte: &'static str| {
        let body = SaisieLotBody {
            cours_id: w.cours_maths,
            cellules: ordre
                .iter()
                .map(|(ev, e)| CelluleSaisieDto {
                    evaluation_id: *ev,
                    eleve_id: *e,
                    texte: texte.into(),
                    version_attendue: None,
                    motif: None,
                })
                .collect(),
            origine: None,
        };
        let (ent, etab, prof) = (&ent, w.etab, w.prof);
        async move {
            (
                ordre,
                ent.eva_v3_saisir_lot_for_actor(etab, prof, PROF, jour(), &body)
                    .await,
            )
        }
    };
    let mut inverse = cases.clone();
    inverse.reverse();
    // Deux lots simultanés, mêmes cases, ordres opposés : aucun n'échoue (pas d'interblocage), et chacun reçoit sa réponse
    // dans l'ordre de SA requête.
    let ((o1, r1), (o2, r2)) = tokio::join!(lot(cases.clone(), "12"), lot(inverse.clone(), "13"));
    for (ordre, retour) in [(o1, r1), (o2, r2)] {
        let retour = retour.expect("aucun interblocage");
        assert_eq!(retour.cellules.len(), ordre.len());
        for (c, (ev, e)) in retour.cellules.iter().zip(ordre.iter()) {
            assert_eq!(
                (c.evaluation_id, c.eleve_id),
                (*ev, *e),
                "réponse dans l'ordre de la requête"
            );
        }
    }
    // Chaque case a été écrite une fois (l'autre lot a vu un conflit, jamais un écrasement silencieux).
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM eva_resultat WHERE etablissement_id = $1 AND evaluation_id = ANY($2)",
    )
    .bind(w.etab)
    .bind(&evs)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, cases.len() as i64);
}

// ───────────────────────── Faibles : garde de visibilité sur le relevé ─────────────────────────

#[tokio::test]
async fn v3c_le_releve_applique_la_garde_des_eleves_proteges() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let anc = ancien_monde(&ent, &w).await;
    let protege = w.eleves_a[2];
    exec(
        &ent,
        "INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, niveau, motif_scelle, du, decide_par)
         VALUES ($1, $2, $3, 'renforcee', 'enc:v1:mesure', '2020-01-01', $4)",
        vec![u(Uuid::new_v4()), u(w.etab), u(protege), u(w.direction)],
    )
    .await;
    // Le secrétariat lit les relevés (`notes.lire` à portée établissement) mais ne voit pas un élève protégé : « introuvable »,
    // comme partout ailleurs (garde du socle 2).
    let adm = cast.administratif.user_id;
    let r = ent
        .releve_eleve_for_actor(w.etab, adm, "administratif", protege, anc.note_periode)
        .await;
    assert!(
        matches!(
            r,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
        ),
        "{r:?}"
    );
    // Un élève ordinaire reste lisible pour le même acteur ; la direction (accès aux protégés) lit aussi le protégé.
    ent.releve_eleve_for_actor(
        w.etab,
        adm,
        "administratif",
        w.eleves_a[0],
        anc.note_periode,
    )
    .await
    .unwrap();
    ent.releve_eleve_for_actor(w.etab, w.direction, DIR, protege, anc.note_periode)
        .await
        .unwrap();
}

// ───────────────────────── Faibles : pas d'avis « nouvelle note » pour un élève qui n'a pas de note ─────────────────────────

#[tokio::test]
async fn v3c_la_publication_ne_previent_que_les_eleves_qui_ont_une_note() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    // Lea (parent de a), Tom (parent de b) voient les notes ; c n'a ni compte ni lien.
    for (user, eleve) in [(cast.parent_lea.user_id, a), (cast.parent_tom.user_id, b)] {
        ent.link_parent(crate::foyer_repo::CreateLien {
            etablissement_id: w.etab,
            eleve_id: eleve,
            user_id: user,
            foyer_id: None,
            lien: "responsable".into(),
            peut_voir_notes: true,
            peut_voir_vs: true,
            peut_justifier_absence: false,
            peut_autoriser_sorties: false,
            est_payeur: false,
            part_paiement: None,
        })
        .await
        .unwrap();
    }
    let ev = ent
        .eva_v3_creer_evaluation_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            maintenant(),
            &corps(w.cours_maths, "DS 1", d(2031, 10, 10), false, vec![]),
        )
        .await
        .unwrap()
        .id;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, a, 12_000))
        .await
        .unwrap();
    ent.eva_v1_ecrire_resultat_for_actor(
        w.etab,
        w.prof,
        PROF,
        &statut_res(ev, b, StatutCalcul::Absent),
    )
    .await
    .unwrap();
    ent.eva_v1_ecrire_resultat_for_actor(
        w.etab,
        w.prof,
        PROF,
        &statut_res(ev, c, StatutCalcul::Dispense),
    )
    .await
    .unwrap();
    ent.eva_v3_publier_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        maintenant(),
        ev,
        &PublierV3Body {
            action: "maintenant".into(),
            prevue_le: None,
            prevue_locale: None,
            motif: None,
        },
    )
    .await
    .unwrap();
    ent.distributeur(crate::outbox::ConfigDistribution {
        taille_lot: 200,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(w.etab),
    })
    .distribuer_tout()
    .await
    .unwrap();
    let nb = |user: Uuid| {
        let (ent, etab) = (&ent, w.etab);
        async move {
            ent.mes_notifications(etab, user, 200)
                .await
                .unwrap()
                .into_iter()
                .filter(|n| n.kind == "evaluation_publiee")
                .count()
        }
    };
    assert_eq!(
        nb(cast.parent_lea.user_id).await,
        1,
        "a a une note : sa famille est prévenue"
    );
    assert_eq!(
        nb(cast.parent_tom.user_id).await,
        0,
        "b est absent : pas d'avis « nouvelle note »"
    );
}
