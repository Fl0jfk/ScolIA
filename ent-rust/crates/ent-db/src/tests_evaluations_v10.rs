//! Socle 21, lot V10 « pilotage et statistiques » : tests sur base (feature `labo`).
//! Aucune horloge réelle : jour fixe 2031-10-20 (monde V1 : année 2031-2032), dates d'évaluation fixes.

use super::*;
use crate::tests_evaluations::labo;
use crate::tests_evaluations_v1::{
    d, evaluation, exec, monde_v1, note, statut, t, u, DIR, PROF, W,
};
use chrono::NaiveDate;
use domain_eleve::evaluation_moteur::{Comptage, StatutCalcul};
use ent_types::evaluations_v10::*;
use uuid::Uuid;

fn jour() -> NaiveDate {
    d(2031, 10, 20)
}

async fn ecrire(ent: &PgEnt, w: &W, prof: Uuid, ev: Uuid, eleve: Uuid, valeur_m: i64) {
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, prof, PROF, &note(ev, eleve, valeur_m))
        .await
        .unwrap();
}

async fn eval20(ent: &PgEnt, w: &W, cours: Uuid, date: NaiveDate, titre: &str) -> Uuid {
    evaluation(ent, w, cours, date, titre, 20.0, 1.0, Comptage::Normal, false).await
}

/// Le second professeur du labo (celui du cours de français de la classe A).
fn autre_prof(cast: &crate::seed::LaboCast) -> Uuid {
    cast.profs.iter().map(|p| p.user_id).find(|x| *x != cast.prof.user_id).expect("second professeur")
}

/// Un cours de maths pour la classe D, tenu par `prof`.
async fn cours_maths_d(ent: &PgEnt, w: &W, prof: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    exec(
        ent,
        "INSERT INTO ent_service_enseignement (id, etablissement_id, annee_id, prof_user_id, matiere_id, matiere_libelle, classe_id)
         VALUES ($1, $2, $3, $4, $5, '', $6)",
        vec![u(id), u(w.etab), u(w.annee), u(prof), u(w.maths), u(w.classe_d)],
    )
    .await;
    sqlx::query_scalar("SELECT cours_id FROM ent_service_enseignement WHERE etablissement_id = $1 AND id = $2")
        .bind(w.etab)
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

async fn consultations(ent: &PgEnt, w: &W, acteur: Uuid, vue: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM eva_consultation_pilotage WHERE etablissement_id = $1 AND acteur_id = $2 AND vue = $3")
        .bind(w.etab)
        .bind(acteur)
        .bind(vue)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

#[tokio::test]
async fn v10_evaluation_commune_regles_de_rattachement_et_comparaison_r_v44() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let autre = autre_prof(&cast);
    let cours_d = cours_maths_d(&ent, &w, autre).await;
    let jour_examen = d(2031, 10, 14);
    let ex_a = eval20(&ent, &w, w.cours_maths, jour_examen, "Examen commun A").await;
    let ex_d = eval20(&ent, &w, cours_d, jour_examen, "Examen commun D").await;
    let mauvaise_date = eval20(&ent, &w, cours_d, d(2031, 10, 15), "Autre jour").await;
    let sur_dix = evaluation(&ent, &w, cours_d, jour_examen, "Sur 10", 10.0, 1.0, Comptage::Normal, false).await;
    let doublon = eval20(&ent, &w, w.cours_maths, jour_examen, "Deuxième du même cours").await;
    let notes_a = [8_000, 12_000, 16_000];
    for (e, v) in w.eleves_a.iter().zip(notes_a) {
        ecrire(&ent, &w, w.prof, ex_a, *e, v).await;
    }
    ecrire(&ent, &w, autre, ex_d, w.eleves_d[0], 10_000).await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, autre, PROF, &statut(ex_d, w.eleves_d[1], StatutCalcul::Absent))
        .await
        .unwrap();

    // Un professeur ne crée pas d'évaluation commune.
    let corps = CreerCommuneBody {
        libelle: "Examen commun de maths".into(),
        date_eval: jour_examen,
        bareme_m: 20_000,
        examen_blanc: true,
        evaluations: vec![ex_a, ex_d],
    };
    assert!(matches!(
        ent.eva_v10_creer_commune_for_actor(w.etab, w.prof, PROF, &corps).await,
        Err(EntDbError::Forbidden)
    ));
    // Date, barème, une par cours : refusés, tout ou rien.
    for mauvais in [mauvaise_date, sur_dix, doublon] {
        let r = ent
            .eva_v10_creer_commune_for_actor(
                w.etab,
                w.direction,
                DIR,
                &CreerCommuneBody { evaluations: vec![ex_a, mauvais], ..corps.clone() },
            )
            .await;
        assert!(r.is_err(), "{mauvais} devait être refusée");
    }
    let rattachees: i64 = sqlx::query_scalar("SELECT count(*) FROM eva_evaluation WHERE etablissement_id = $1 AND id = ANY($2) AND commune_id IS NOT NULL")
        .bind(w.etab)
        .bind(vec![ex_a, ex_d, mauvaise_date, sur_dix, doublon])
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(rattachees, 0, "un refus n'a rien rattaché");

    let c = ent.eva_v10_creer_commune_for_actor(w.etab, w.direction, DIR, &corps).await.unwrap();
    assert_eq!(c.evaluations.len(), 2);
    assert!(c.examen_blanc);
    // Une évaluation d'une commune ne passe pas dans une autre.
    let autre_commune = ent
        .eva_v10_creer_commune_for_actor(
            w.etab,
            w.direction,
            DIR,
            &CreerCommuneBody { libelle: "Autre".into(), evaluations: vec![], examen_blanc: false, ..corps.clone() },
        )
        .await
        .unwrap();
    assert!(ent
        .eva_v10_rattacher_for_actor(w.etab, w.direction, DIR, autre_commune.id, &EvaluationsBody { evaluations: vec![ex_a] })
        .await
        .is_err());

    // Direction : tous les groupes, l'ensemble ; rangés par libellé.
    let vue = ent.eva_v10_comparer_commune_for_actor(w.etab, w.direction, DIR, jour(), c.id).await.unwrap();
    assert_eq!(vue.vue, "direction");
    assert_eq!(vue.groupes.len(), 2);
    let ensemble = vue.ensemble.unwrap();
    assert_eq!(ensemble.effectif, 4, "l'absent ne compte pas");
    assert_eq!(ensemble.moyenne_m, 11_500);
    let g_d = vue.groupes.iter().find(|g| g.evaluation_id == ex_d).unwrap();
    assert_eq!(g_d.non_comptes, 1);
    assert_eq!(g_d.stats.unwrap().moyenne_m, 10_000);
    assert!(vue.autres_groupes.is_none());

    // Professeur concerné : son groupe, et les autres sans détail ; sous le seuil, pas même la moyenne.
    let vue_p = ent.eva_v10_comparer_commune_for_actor(w.etab, w.prof, PROF, jour(), c.id).await.unwrap();
    assert_eq!(vue_p.vue, "professeur");
    assert!(vue_p.ensemble.is_none());
    assert_eq!(vue_p.groupes.len(), 1);
    assert_eq!(vue_p.groupes[0].evaluation_id, ex_a);
    assert_eq!(vue_p.groupes[0].stats.unwrap().moyenne_m, 12_000);
    let autres = vue_p.autres_groupes.unwrap();
    assert_eq!((autres.groupes, autres.effectif, autres.moyenne_m), (1, 1, None));
    // Les deux lectures sont journalisées.
    assert_eq!(consultations(&ent, &w, w.direction, "commune").await, 1);
    assert_eq!(consultations(&ent, &w, w.prof, "commune").await, 1);

    // Un professeur sans groupe dans la commune ne la voit pas ; la liste du professeur ne la montre qu'à lui.
    let prof_fr = sqlx::query_scalar::<_, Uuid>(
        "SELECT prof_user_id FROM ent_service_enseignement WHERE etablissement_id = $1 AND cours_id = $2",
    )
    .bind(w.etab)
    .bind(w.cours_fr)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(prof_fr, autre);
    let liste = ent.eva_v10_communes_for_actor(w.etab, w.prof, PROF, jour()).await.unwrap();
    let vue_liste = liste.iter().find(|x| x.id == c.id).expect("commune visible du professeur concerné");
    assert_eq!(vue_liste.evaluations.iter().filter(|e| e.mon_cours).count(), 1);
    assert!(liste.iter().all(|x| x.id != autre_commune.id), "commune sans son groupe : absente");

    // Détacher : l'évaluation redevient ordinaire.
    let apres = ent
        .eva_v10_detacher_for_actor(w.etab, w.direction, DIR, c.id, &EvaluationsBody { evaluations: vec![ex_d] })
        .await
        .unwrap();
    assert_eq!(apres.evaluations.len(), 1);
    assert!(ent
        .eva_v10_detacher_for_actor(w.etab, w.direction, DIR, c.id, &EvaluationsBody { evaluations: vec![ex_d] })
        .await
        .is_err());
    // Le journal ne se réécrit pas.
    assert!(sqlx::query("DELETE FROM eva_consultation_pilotage WHERE etablissement_id = $1")
        .bind(w.etab)
        .execute(ent.pool_verif())
        .await
        .is_err());
}

#[tokio::test]
async fn v10_ecarts_entre_examen_blanc_et_controle_continu() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let examen = eval20(&ent, &w, w.cours_maths, d(2031, 10, 14), "Bac blanc").await;
    let cc1 = eval20(&ent, &w, w.cours_maths, d(2031, 9, 20), "DS 1").await;
    let cc2 = eval20(&ent, &w, w.cours_maths, d(2031, 10, 5), "DS 2").await;
    let [a, b, c] = [w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]];
    // a : examen 10, contrôle continu 14 ; b : examen 12, contrôle continu 12 ; c : examen seul (pas comparé).
    for (e, v) in [(a, 10_000), (b, 12_000), (c, 18_000)] {
        ecrire(&ent, &w, w.prof, examen, e, v).await;
    }
    for (ev, e, v) in [(cc1, a, 12_000), (cc2, a, 16_000), (cc1, b, 12_000)] {
        ecrire(&ent, &w, w.prof, ev, e, v).await;
    }
    let pas_blanc = ent
        .eva_v10_creer_commune_for_actor(
            w.etab,
            w.direction,
            DIR,
            &CreerCommuneBody {
                libelle: "Commune ordinaire".into(),
                date_eval: d(2031, 10, 14),
                bareme_m: 20_000,
                examen_blanc: false,
                evaluations: vec![],
            },
        )
        .await
        .unwrap();
    assert!(ent.eva_v10_ecarts_examen_for_actor(w.etab, w.direction, DIR, pas_blanc.id).await.is_err());
    let blanc = ent
        .eva_v10_creer_commune_for_actor(
            w.etab,
            w.direction,
            DIR,
            &CreerCommuneBody {
                libelle: "Bac blanc".into(),
                date_eval: d(2031, 10, 14),
                bareme_m: 20_000,
                examen_blanc: true,
                evaluations: vec![examen],
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        ent.eva_v10_ecarts_examen_for_actor(w.etab, w.prof, PROF, blanc.id).await,
        Err(EntDbError::Forbidden)
    ));
    let e = ent.eva_v10_ecarts_examen_for_actor(w.etab, w.direction, DIR, blanc.id).await.unwrap();
    assert_eq!(e.groupes.len(), 1);
    let g = &e.groupes[0];
    assert_eq!(g.effectif, 2, "l'élève sans contrôle continu n'est pas comparé");
    assert_eq!(g.moyenne_examen_m, Some(11_000));
    assert_eq!(g.moyenne_controle_continu_m, Some(13_000));
    assert_eq!(g.ecart_m, Some(-2_000));
    assert_eq!(e.ensemble.ecart_m, Some(-2_000));
    assert_eq!(consultations(&ent, &w, w.direction, "ecarts_examen").await, 1);
}

#[tokio::test]
async fn v10_statistiques_de_periode_natures_harmonisation_et_droits() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let autre = autre_prof(&cast);
    let cours_d = cours_maths_d(&ent, &w, autre).await;
    // Niveau commun aux classes A et D.
    let niveau = Uuid::new_v4();
    exec(
        &ent,
        "INSERT INTO ent_niveau (id, etablissement_id, code, libelle) VALUES ($1, $2, $3, 'Seconde')",
        vec![u(niveau), u(w.etab), t(&format!("2NDE{}", &niveau.simple().to_string()[..6]))],
    )
    .await;
    exec(
        &ent,
        "UPDATE ent_classe SET niveau_id = $1 WHERE etablissement_id = $2 AND id = ANY(ARRAY[$3, $4]::uuid[])",
        vec![u(niveau), u(w.etab), u(w.classe_a), u(w.classe_d)],
    )
    .await;
    let nature = Uuid::new_v4();
    exec(
        &ent,
        "INSERT INTO eva_nature (id, etablissement_id, code, libelle) VALUES ($1, $2, $3, 'Devoir surveillé')",
        vec![u(nature), u(w.etab), t(&format!("ds_{}", &nature.simple().to_string()[..6]))],
    )
    .await;
    let m_a = eval20(&ent, &w, w.cours_maths, d(2031, 10, 6), "DS maths A").await;
    exec(&ent, "UPDATE eva_evaluation SET nature_id = $1 WHERE id = $2", vec![u(nature), u(m_a)]).await;
    let f_a = eval20(&ent, &w, w.cours_fr, d(2031, 10, 7), "Rédaction").await;
    let m_d = eval20(&ent, &w, cours_d, d(2031, 10, 8), "DS maths D").await;
    // Hors période (T2) : ne compte pas pour T1.
    let hors = eval20(&ent, &w, w.cours_maths, d(2032, 1, 10), "DS de janvier").await;
    for (e, v) in w.eleves_a.iter().zip([10_000, 12_000, 14_000]) {
        ecrire(&ent, &w, w.prof, m_a, *e, v).await;
        ecrire(&ent, &w, autre, f_a, *e, v + 2_000).await;
    }
    for (e, v) in w.eleves_d.iter().zip([8_000, 10_000]) {
        ecrire(&ent, &w, autre, m_d, *e, v).await;
    }
    let _ = hors;

    // Direction, par matière : maths (5 moyennes) et français (3).
    let s = ent
        .eva_v10_statistiques_for_actor(w.etab, w.direction, DIR, jour(), w.t1, AxeStatistiques::Matiere)
        .await
        .unwrap();
    assert_eq!(s.lignes.len(), 2);
    let maths = s.lignes.iter().find(|l| l.cle == Some(w.maths)).unwrap();
    assert_eq!((maths.cours, maths.evaluations), (2, 2));
    assert_eq!(maths.stats.unwrap().effectif, 5);
    assert_eq!(maths.stats.unwrap().moyenne_m, 10_800);
    let fr = s.lignes.iter().find(|l| l.cle == Some(w.francais)).unwrap();
    assert_eq!(fr.stats.unwrap().moyenne_m, 14_000);
    // Rangées par libellé, jamais par valeur.
    assert_eq!(s.lignes[0].libelle, "Français");

    // Par classe, par professeur.
    let s = ent
        .eva_v10_statistiques_for_actor(w.etab, w.direction, DIR, jour(), w.t1, AxeStatistiques::Classe)
        .await
        .unwrap();
    let a = s.lignes.iter().find(|l| l.cle == Some(w.classe_a)).unwrap();
    assert_eq!(a.stats.unwrap().effectif, 6);
    let s = ent
        .eva_v10_statistiques_for_actor(w.etab, w.direction, DIR, jour(), w.t1, AxeStatistiques::Professeur)
        .await
        .unwrap();
    let l_autre = s.lignes.iter().find(|l| l.cle == Some(autre)).unwrap();
    assert_eq!(l_autre.cours, 2);
    assert_eq!(l_autre.stats.unwrap().effectif, 5);

    // Le professeur : ses cours seulement, quel que soit l'axe demandé.
    let s = ent
        .eva_v10_statistiques_for_actor(w.etab, w.prof, PROF, jour(), w.t1, AxeStatistiques::Professeur)
        .await
        .unwrap();
    assert_eq!(s.axe, AxeStatistiques::Cours);
    assert_eq!(s.lignes.len(), 1);
    assert_eq!(s.lignes[0].cle, Some(w.cours_maths));
    assert_eq!(s.lignes[0].stats.unwrap().moyenne_m, 12_000);
    assert_eq!(consultations(&ent, &w, w.direction, "statistiques").await, 3);
    assert_eq!(consultations(&ent, &w, w.prof, "statistiques").await, 1);

    // Natures : direction seulement.
    assert!(matches!(
        ent.eva_v10_natures_for_actor(w.etab, w.prof, PROF, w.t1).await,
        Err(EntDbError::Forbidden)
    ));
    let n = ent.eva_v10_natures_for_actor(w.etab, w.direction, DIR, w.t1).await.unwrap();
    let ds = n.lignes.iter().find(|l| l.nature_id == Some(nature)).unwrap();
    assert_eq!((ds.evaluations, ds.notes, ds.coefficients_m, ds.part_coefficients_m), (1, 3, 1_000, 333));
    let sans = n.lignes.iter().find(|l| l.nature_id.is_none()).unwrap();
    assert_eq!((sans.evaluations, sans.notes), (2, 5));
    assert_eq!(sans.moyennes_evaluations.unwrap().effectif, 2);

    // Harmonisation du niveau : maths, A à 12, D à 9, niveau à 10,8.
    assert!(matches!(
        ent.eva_v10_harmonisation_for_actor(w.etab, w.prof, PROF, w.t1, niveau).await,
        Err(EntDbError::Forbidden)
    ));
    let h = ent.eva_v10_harmonisation_for_actor(w.etab, w.direction, DIR, w.t1, niveau).await.unwrap();
    assert_eq!(h.niveau_libelle, "Seconde");
    let hm = h.matieres.iter().find(|m| m.matiere_id == Some(w.maths)).unwrap();
    assert_eq!(hm.niveau.unwrap().moyenne_m, 10_800);
    let ca = hm.classes.iter().find(|c| c.classe_id == w.classe_a).unwrap();
    let cd = hm.classes.iter().find(|c| c.classe_id == w.classe_d).unwrap();
    assert_eq!(ca.ecart_m, Some(1_200));
    assert_eq!(cd.ecart_m, Some(-1_800));
    assert_eq!(consultations(&ent, &w, w.direction, "harmonisation").await, 1);
}

#[tokio::test]
async fn v10_charge_de_classe_et_evaluations_non_publiees() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let lundi = d(2031, 10, 13);
    let e1 = eval20(&ent, &w, w.cours_maths, lundi, "DS maths").await;
    let e2 = eval20(&ent, &w, w.cours_fr, lundi, "Dictée").await;
    let e3 = eval20(&ent, &w, w.cours_fr, d(2031, 10, 22), "Rédaction").await;
    let vieille = eval20(&ent, &w, w.cours_maths, d(2031, 9, 15), "Interro de septembre").await;
    ecrire(&ent, &w, w.prof, vieille, w.eleves_a[0], 12_000).await;
    let publiee = evaluation(&ent, &w, w.cours_maths, d(2031, 9, 16), "Publiée", 20.0, 1.0, Comptage::Normal, true).await;

    // Charge : le professeur de la classe la lit, sans note.
    let ch = ent
        .eva_v10_charge_for_actor(w.etab, w.prof, PROF, jour(), w.classe_a, d(2031, 10, 13), d(2031, 10, 26))
        .await
        .unwrap();
    let ids: Vec<Uuid> = ch.evaluations.iter().map(|e| e.evaluation_id).collect();
    assert!(ids.contains(&e1) && ids.contains(&e2) && ids.contains(&e3));
    assert_eq!(ch.semaines.len(), 2);
    assert_eq!(ch.semaines[0].lundi, lundi);
    assert_eq!(ch.semaines[0].jours_charges, vec![lundi]);
    // Classe D : aucune évaluation ; fenêtre trop longue refusée.
    let ch_d = ent
        .eva_v10_charge_for_actor(w.etab, w.direction, DIR, jour(), w.classe_d, d(2031, 10, 13), d(2031, 10, 26))
        .await
        .unwrap();
    assert!(ch_d.evaluations.is_empty());
    assert!(ent
        .eva_v10_charge_for_actor(w.etab, w.direction, DIR, jour(), w.classe_a, d(2031, 9, 1), d(2032, 3, 1))
        .await
        .is_err());
    // Un professeur sans cours dans la classe D ne lit pas sa charge.
    assert!(matches!(
        ent.eva_v10_charge_for_actor(w.etab, w.prof, PROF, jour(), w.classe_d, d(2031, 10, 13), d(2031, 10, 26)).await,
        Err(EntDbError::Forbidden)
    ));

    // Non publiées depuis plus de 15 jours (défaut) : celle de septembre seulement.
    assert!(matches!(
        ent.eva_v10_non_publiees_for_actor(w.etab, w.prof, PROF, jour(), None).await,
        Err(EntDbError::Forbidden)
    ));
    let np = ent.eva_v10_non_publiees_for_actor(w.etab, w.direction, DIR, jour(), None).await.unwrap();
    assert_eq!(np.delai_jours, NON_PUBLIEE_JOURS_DEFAUT);
    let v = np.evaluations.iter().find(|e| e.evaluation_id == vieille).expect("évaluation de septembre signalée");
    assert_eq!((v.jours, v.resultats_saisis), (35, 1));
    assert_eq!(v.professeurs, vec![w.prof]);
    assert!(np.evaluations.iter().all(|e| e.evaluation_id != publiee && e.evaluation_id != e1));
    // Délai explicite : 5 jours fait apparaître celle du 13 octobre ; hors bornes refusé.
    let np5 = ent.eva_v10_non_publiees_for_actor(w.etab, w.direction, DIR, jour(), Some(5)).await.unwrap();
    assert!(np5.evaluations.iter().any(|e| e.evaluation_id == e1));
    assert!(ent.eva_v10_non_publiees_for_actor(w.etab, w.direction, DIR, jour(), Some(0)).await.is_err());

    // Journal : la direction le lit, pas le professeur.
    assert!(matches!(
        ent.eva_v10_consultations_for_actor(w.etab, w.prof, PROF, None).await,
        Err(EntDbError::Forbidden)
    ));
    ent.eva_v10_consultations_for_actor(w.etab, w.direction, DIR, Some(w.prof)).await.unwrap();
}
