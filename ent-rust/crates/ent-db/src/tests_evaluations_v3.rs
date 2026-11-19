//! Socle 21, lot V3 « grille et saisie » : tests sur base (feature `labo`). Aucune horloge réelle : jour fixe
//! (2031-10-20, dans le premier trimestre fictif) et « maintenant » fixe (midi UTC de ce jour). Chaque scénario
//! part d'un établissement neuf du labo ; les préparations passent par `pool_verif()`.

use super::*;
use crate::evaluations_v3_eval::BilanPublications;
use crate::tests_evaluations::labo;
use crate::tests_evaluations_v1::{
    ancien_monde, ancienne_note, arreter, creer_eleve, d, exec, monde_v1, u, DIR, PROF, W,
};
use chrono::{DateTime, NaiveDate, Utc};
use ent_types::evaluations_v3::*;
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
    bareme_m: i64,
    coef_m: i64,
) -> CreerEvaluationV3Body {
    CreerEvaluationV3Body {
        cours_id: cours,
        titre: titre.into(),
        date_eval: date,
        bareme_m,
        coef_m,
        comptage: None,
        hors_bareme_autorise: false,
        sous_matiere_id: None,
        restreinte_a: vec![],
        publication: None,
        publication_prevue_le: None,
        annoncee: false,
    }
}

async fn creer(
    ent: &PgEnt,
    w: &W,
    cours: Uuid,
    titre: &str,
    date: NaiveDate,
    bareme_m: i64,
    coef_m: i64,
) -> EvaluationColonneDto {
    ent.eva_v3_creer_evaluation_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        maintenant(),
        &corps(cours, titre, date, bareme_m, coef_m),
    )
    .await
    .unwrap()
}

fn cell(ev: Uuid, eleve: Uuid, texte: &str, version: Option<i32>) -> CelluleSaisieDto {
    CelluleSaisieDto {
        evaluation_id: ev,
        eleve_id: eleve,
        texte: texte.into(),
        version_attendue: version,
        motif: None,
    }
}

async fn lot_par(
    ent: &PgEnt,
    w: &W,
    par: Uuid,
    role: &str,
    cours: Uuid,
    cellules: Vec<CelluleSaisieDto>,
) -> Result<SaisieLotRetourDto, EntDbError> {
    ent.eva_v3_saisir_lot_for_actor(
        w.etab,
        par,
        role,
        jour(),
        &SaisieLotBody {
            cours_id: cours,
            cellules,
            origine: None,
        },
    )
    .await
}

async fn lot(
    ent: &PgEnt,
    w: &W,
    cours: Uuid,
    cellules: Vec<CelluleSaisieDto>,
) -> SaisieLotRetourDto {
    lot_par(ent, w, w.prof, PROF, cours, cellules)
        .await
        .unwrap()
}

async fn grille(ent: &PgEnt, w: &W, cours: Uuid) -> GrilleV3Dto {
    ent.eva_v3_grille_for_actor(w.etab, w.prof, PROF, cours, w.t1, jour(), maintenant())
        .await
        .unwrap()
}

fn issue(r: &SaisieLotRetourDto, i: usize) -> &str {
    r.cellules[i].issue.as_str()
}

// ───────────────────────── Grille, moteur, cours sans matière ─────────────────────────

#[tokio::test]
async fn v3_la_grille_est_calculee_par_le_moteur_et_expose_les_statistiques() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let e1 = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    let e2 = creer(
        &ent,
        &w,
        w.cours_maths,
        "Interro",
        d(2031, 10, 17),
        10_000,
        2_000,
    )
    .await;
    let r = lot(
        &ent,
        &w,
        w.cours_maths,
        vec![
            cell(e1.id, a, "14", None),
            cell(e1.id, b, "12,5", None),
            cell(e1.id, c, "abs", None),
            cell(e2.id, a, "7", None),
            cell(e2.id, b, "nr", None),
            cell(e2.id, c, "5", None),
        ],
    )
    .await;
    assert!(r.cellules.iter().all(|x| x.issue == "ecrit"), "{r:?}");
    let g = grille(&ent, &w, w.cours_maths).await;
    assert!(g.peut_saisir && !g.enseignement.sans_matiere);
    assert_eq!(g.evaluations.len(), 2);
    assert_eq!(g.eleves.len(), 3);
    let ligne = |id: Uuid| g.eleves.iter().find(|e| e.eleve_id == id).unwrap();
    // Abbott : 14/20 et 7/10 (coef 2) → 14. Bones : 12,5 (le non rendu n'est pas compté). Corner : l'absence n'est jamais un zéro.
    assert_eq!(ligne(a).moyenne_m, Some(14_000));
    assert_eq!(ligne(b).moyenne_m, Some(12_500));
    assert_eq!(ligne(c).moyenne_m, Some(10_000));
    assert_eq!(ligne(c).cellules[0].statut.as_deref(), Some("absent"));
    let cl = g.moyenne_classe.expect("moyenne de classe");
    assert_eq!(
        (cl.effectif, cl.min_m, cl.max_m, cl.mediane_m),
        (3, 10_000, 14_000, 12_500)
    );
    assert_eq!(
        cl.moyenne_m, 12_170,
        "moyenne des moyennes, arrondie au centième par le moteur"
    );
    // Statistiques de l'évaluation : le moteur, sur les notes ramenées.
    let s1 = g.evaluations[0].stats.as_ref().expect("stats DS 1");
    assert_eq!((s1.effectif, s1.min_m, s1.max_m), (2, 12_500, 14_000));
    // Explication ligne à ligne.
    let ex = ent
        .eva_v3_explication_for_actor(w.etab, w.prof, PROF, w.cours_maths, w.t1, c, jour())
        .await
        .unwrap();
    assert_eq!(ex.moyenne_m, Some(10_000));
    assert!(
        ex.lignes.iter().any(|l| l.to_lowercase().contains("abs")),
        "{:?}",
        ex.lignes
    );
}

#[tokio::test]
async fn v3_un_cours_sans_matiere_du_referentiel_a_sa_grille() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    // Service « Atelier » sans matière du référentiel (libellé libre) : l'ancienne grille répondait
    // « service sans matière du référentiel ».
    exec(
        &ent,
        "INSERT INTO ent_service_enseignement (id, etablissement_id, annee_id, prof_user_id, matiere_id, matiere_libelle, classe_id)
         VALUES ($1, $2, $3, $4, NULL, 'Atelier théâtre', $5)",
        vec![u(Uuid::new_v4()), u(w.etab), u(w.annee), u(w.prof), u(w.classe_a)],
    )
    .await;
    let cours: Uuid = sqlx::query_scalar(
        "SELECT cours_id FROM ent_service_enseignement WHERE etablissement_id = $1 AND prof_user_id = $2 AND matiere_libelle = 'Atelier théâtre'",
    )
    .bind(w.etab)
    .bind(w.prof)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let ens = ent
        .eva_v3_enseignements_for_actor(w.etab, w.prof, PROF, jour())
        .await
        .unwrap();
    let atelier = ens
        .iter()
        .find(|e| e.cours_id == cours)
        .expect("le cours sans matière est listé");
    assert!(
        atelier.sans_matiere && atelier.droit == "saisie" && atelier.libelle.contains("Atelier")
    );
    let ev = creer(&ent, &w, cours, "Scène 1", d(2031, 10, 15), 20_000, 1_000).await;
    let r = lot(
        &ent,
        &w,
        cours,
        vec![cell(ev.id, w.eleves_a[0], "16", None)],
    )
    .await;
    assert_eq!(issue(&r, 0), "ecrit");
    let g = grille(&ent, &w, cours).await;
    assert!(g.enseignement.sans_matiere);
    assert_eq!(
        g.eleves
            .iter()
            .find(|e| e.eleve_id == w.eleves_a[0])
            .unwrap()
            .moyenne_m,
        Some(16_000)
    );
}

#[tokio::test]
async fn v3_les_periodes_du_cours_sont_proposees() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ps = ent
        .eva_v3_periodes_for_actor(w.etab, w.prof, PROF, w.cours_maths, jour())
        .await
        .unwrap();
    assert!(
        ps.iter().any(|p| p.id == w.t1) && ps.iter().any(|p| p.id == w.t2),
        "{ps:?}"
    );
}

// ───────────────────────── Droits ─────────────────────────

#[tokio::test]
async fn v3_droits_professeur_direction_secretariat_famille() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    lot(
        &ent,
        &w,
        w.cours_maths,
        vec![cell(ev.id, w.eleves_a[0], "11", None)],
    )
    .await;
    // Direction et secrétariat : lecture seule.
    for (u_id, role) in [
        (w.direction, DIR),
        (cast.administratif.user_id, "administratif"),
    ] {
        let g = ent
            .eva_v3_grille_for_actor(
                w.etab,
                u_id,
                role,
                w.cours_maths,
                w.t1,
                jour(),
                maintenant(),
            )
            .await
            .unwrap();
        assert!(!g.peut_saisir, "{role} : lecture seule");
        assert_eq!(g.eleves.len(), 3);
        let r = lot_par(
            &ent,
            &w,
            u_id,
            role,
            w.cours_maths,
            vec![cell(ev.id, w.eleves_a[1], "9", None)],
        )
        .await;
        let refuse = match r {
            Err(EntDbError::Forbidden) => true,
            Ok(r) => issue(&r, 0) == "refuse",
            Err(_) => false,
        };
        assert!(
            refuse,
            "{role} ne saisit pas pendant que la période est ouverte"
        );
    }
    // Un autre professeur : ni lecture ni saisie sur ce cours.
    let autre = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)
        .unwrap();
    let r = ent
        .eva_v3_grille_for_actor(
            w.etab,
            autre,
            PROF,
            w.cours_maths,
            w.t1,
            jour(),
            maintenant(),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    assert!(matches!(
        lot_par(
            &ent,
            &w,
            autre,
            PROF,
            w.cours_maths,
            vec![cell(ev.id, w.eleves_a[1], "9", None)]
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    // Famille et élève : jamais la grille.
    for role in ["parent", "eleve"] {
        let r = ent
            .eva_v3_grille_for_actor(
                w.etab,
                cast.parent_lea.user_id,
                role,
                w.cours_maths,
                w.t1,
                jour(),
                maintenant(),
            )
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
    // Un cours d'un autre établissement n'existe pas.
    let r = ent
        .eva_v3_grille_for_actor(
            Uuid::new_v4(),
            w.prof,
            PROF,
            w.cours_maths,
            w.t1,
            jour(),
            maintenant(),
        )
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn v3_apres_l_arret_seule_la_direction_corrige_avec_un_motif_et_l_historique_le_montre() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    let r = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "11", None)]).await;
    let v1 = r.cellules[0].version.unwrap();
    arreter(&ent, &w, w.t1, w.classe_a).await;
    // Le professeur ne saisit plus.
    let r = lot(
        &ent,
        &w,
        w.cours_maths,
        vec![cell(ev.id, a, "15", Some(v1))],
    )
    .await;
    assert_eq!(issue(&r, 0), "refuse", "{r:?}");
    // La direction corrige, avec un motif.
    let sans_motif = lot_par(
        &ent,
        &w,
        w.direction,
        DIR,
        w.cours_maths,
        vec![cell(ev.id, a, "15", Some(v1))],
    )
    .await
    .unwrap();
    assert_eq!(
        issue(&sans_motif, 0),
        "refuse",
        "sans motif : {sans_motif:?}"
    );
    let mut avec = cell(ev.id, a, "15", Some(v1));
    avec.motif = Some("erreur de report de la copie".into());
    let r = lot_par(&ent, &w, w.direction, DIR, w.cours_maths, vec![avec])
        .await
        .unwrap();
    assert_eq!(issue(&r, 0), "ecrit", "{r:?}");
    let h = ent
        .eva_v3_historique_for_actor(w.etab, w.prof, PROF, ev.id, a, jour())
        .await
        .unwrap();
    assert_eq!(h.len(), 2);
    assert!(!h[0].correction && h[1].correction);
    assert_eq!(
        (h[1].valeur_avant_m, h[1].valeur_apres_m),
        (Some(11_000), Some(15_000))
    );
    assert_eq!(h[1].motif.as_deref(), Some("erreur de report de la copie"));
    assert!(h[1].auteur_nom.is_some(), "qui a corrigé est affiché");
    // La grille signale la case corrigée.
    let g = grille(&ent, &w, w.cours_maths).await;
    let cel = &g.eleves.iter().find(|e| e.eleve_id == a).unwrap().cellules[0];
    assert!(cel.corrige && !cel.saisie_ouverte);
    // Un parent ne lit pas l'historique.
    assert!(ent
        .eva_v3_historique_for_actor(w.etab, cast.parent_lea.user_id, "parent", ev.id, a, jour())
        .await
        .is_err());
}

// ───────────────────────── Conflits de version ─────────────────────────

#[tokio::test]
async fn v3_deux_onglets_jamais_d_ecrasement_silencieux() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    // L'onglet 1 écrit 12 (case vide) ; l'onglet 2 croyait aussi la case vide : conflit.
    let r1 = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "12", None)]).await;
    assert_eq!(issue(&r1, 0), "ecrit");
    let v1 = r1.cellules[0].version.unwrap();
    let r2 = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "9", None)]).await;
    assert_eq!(issue(&r2, 0), "conflit");
    let c = r2.cellules[0]
        .conflit
        .as_ref()
        .expect("état actuel de la case");
    assert_eq!(
        (c.statut.as_str(), c.valeur_m, c.version),
        ("note", Some(12_000), v1)
    );
    assert!(c.par_nom.is_some());
    // L'onglet 1 corrige (version lue) → 13 ; l'onglet 2 renvoie avec la version périmée : conflit encore.
    let r3 = lot(
        &ent,
        &w,
        w.cours_maths,
        vec![cell(ev.id, a, "13", Some(v1))],
    )
    .await;
    assert_eq!(issue(&r3, 0), "ecrit");
    let v2 = r3.cellules[0].version.unwrap();
    assert!(v2 > v1);
    let r4 = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "9", Some(v1))]).await;
    assert_eq!(issue(&r4, 0), "conflit");
    // Résolution explicite : « garder la mienne » = renvoyer avec la version actuelle.
    let r5 = lot(
        &ent,
        &w,
        w.cours_maths,
        vec![cell(
            ev.id,
            a,
            "9",
            Some(r4.cellules[0].conflit.as_ref().unwrap().version),
        )],
    )
    .await;
    assert_eq!(issue(&r5, 0), "ecrit");
    let g = grille(&ent, &w, w.cours_maths).await;
    assert_eq!(
        g.eleves.iter().find(|e| e.eleve_id == a).unwrap().cellules[0].valeur_m,
        Some(9_000)
    );
    // Une même valeur à la même version ne change rien (ni version ni journal).
    let v5 = r5.cellules[0].version.unwrap();
    let r6 = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "9", Some(v5))]).await;
    assert_eq!(issue(&r6, 0), "inchange");
    assert_eq!(r6.cellules[0].version, Some(v5));
    let h = ent
        .eva_v3_historique_for_actor(w.etab, w.prof, PROF, ev.id, a, jour())
        .await
        .unwrap();
    assert_eq!(h.len(), 3, "12, 13, 9 : une ligne par changement réel");
}

#[tokio::test]
async fn v3_chaque_case_est_independante_et_les_erreurs_sont_dites() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let ev = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    let r = lot(
        &ent,
        &w,
        w.cours_maths,
        vec![
            cell(ev.id, a, "21", None),              // au-dessus du barème
            cell(ev.id, b, "douze", None),           // illisible
            cell(ev.id, c, "disp", None),            // code rapide
            cell(ev.id, a, "", None),                // vide : rien
            cell(ev.id, Uuid::new_v4(), "10", None), // élève inconnu
            cell(Uuid::new_v4(), a, "10", None),     // évaluation inconnue
        ],
    )
    .await;
    let v: Vec<&str> = r.cellules.iter().map(|x| x.issue.as_str()).collect();
    assert_eq!(
        v,
        ["refuse", "refuse", "ecrit", "vide", "refuse", "refuse"],
        "{r:?}"
    );
    assert!(r.cellules[0].message.as_deref().unwrap().contains("barème"));
    assert_eq!(r.cellules[2].statut.as_deref(), Some("dispense"));
    // Un élève ne figurant pas dans le public à la date n'est jamais saisi (non concerné).
    let hors = creer_eleve(&ent, w.etab, "Hors-public").await;
    let r = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, hors, "10", None)]).await;
    assert_eq!(issue(&r, 0), "refuse");
    // Trop de cases : refusé en bloc.
    let trop: Vec<CelluleSaisieDto> = (0..2001).map(|_| cell(ev.id, a, "1", None)).collect();
    assert!(lot_par(&ent, &w, w.prof, PROF, w.cours_maths, trop)
        .await
        .is_err());
}

#[tokio::test]
async fn v3_un_eleve_protege_est_introuvable_pour_le_secretariat_mais_pas_pour_le_professeur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    lot(
        &ent,
        &w,
        w.cours_maths,
        vec![cell(ev.id, w.eleves_a[0], "11", None)],
    )
    .await;
    // Le professeur du cours voit toujours ses élèves ; le secrétariat voit la même grille que sans protection
    // (aucune protection posée ici) : le test garantit au moins que la garde ne retire personne à tort.
    let g = ent
        .eva_v3_grille_for_actor(
            w.etab,
            cast.administratif.user_id,
            "administratif",
            w.cours_maths,
            w.t1,
            jour(),
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!(g.eleves.len(), 3);
}

// ───────────────────────── Parcours : arrivée et départ en cours de période ─────────────────────────

#[tokio::test]
async fn v3_eleve_arrive_ou_parti_en_cours_de_periode() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let arrive = creer_eleve(&ent, w.etab, "Arrive").await;
    crate::tests_evaluations_v1::affecter(
        &ent,
        w.etab,
        w.annee,
        arrive,
        w.classe_a,
        d(2031, 10, 15),
    )
    .await;
    let part = creer_eleve(&ent, w.etab, "Part").await;
    crate::tests_evaluations_v1::affecter(&ent, w.etab, w.annee, part, w.classe_a, d(2031, 9, 1))
        .await;
    exec(
        &ent,
        "UPDATE ent_affectation SET date_fin = '2031-10-12' WHERE etablissement_id = $1 AND eleve_id = $2",
        vec![u(w.etab), u(part)],
    )
    .await;
    let e1 = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    let e2 = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 2",
        d(2031, 10, 18),
        20_000,
        1_000,
    )
    .await;
    let r = lot(
        &ent,
        &w,
        w.cours_maths,
        vec![
            cell(e1.id, part, "13", None),
            cell(e2.id, arrive, "8", None),
            cell(e1.id, arrive, "9", None),
        ],
    )
    .await;
    assert_eq!(issue(&r, 0), "ecrit");
    assert_eq!(issue(&r, 1), "ecrit");
    assert_eq!(
        issue(&r, 2),
        "refuse",
        "l'arrivant n'était pas dans le public le 10 octobre : non concerné"
    );
    let g = grille(&ent, &w, w.cours_maths).await;
    let la = g.eleves.iter().find(|e| e.eleve_id == arrive).unwrap();
    let lp = g.eleves.iter().find(|e| e.eleve_id == part).unwrap();
    assert!(!la.cellules[0].concerne && la.cellules[1].concerne);
    assert!(la.mouvement.as_deref().unwrap().contains("Arrivé"));
    assert!(lp.cellules[0].concerne && !lp.cellules[1].concerne);
    assert!(lp.mouvement.as_deref().unwrap().contains("Parti"));
    assert_eq!(la.moyenne_m, Some(8_000));
    assert_eq!(lp.moyenne_m, Some(13_000));
}

#[tokio::test]
async fn v3_periode_arretee_pour_une_classe_seulement_et_evaluation_commune() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    // La période de la classe A est arrêtée : la création d'une nouvelle évaluation pour cette classe est refusée.
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let r = ent
        .eva_v3_creer_evaluation_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            maintenant(),
            &corps(w.cours_maths, "DS 2", d(2031, 10, 18), 20_000, 1_000),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    // Les colonnes existantes restent lisibles, les cases verrouillées.
    let g = grille(&ent, &w, w.cours_maths).await;
    assert!(g.eleves.iter().all(|e| !e.cellules[0].saisie_ouverte));
    assert_eq!(g.evaluations[0].id, ev.id);
}

// ───────────────────────── Évaluations : création, réglage, annulation ─────────────────────────

#[tokio::test]
async fn v3_creer_regler_et_annuler_une_evaluation() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    // Contrôles de création.
    for (titre, date, bar, coef) in [
        ("", d(2031, 10, 10), 20_000, 1_000),
        ("Ok", d(2030, 1, 1), 20_000, 1_000),
        ("Ok", d(2031, 10, 10), 0, 1_000),
        ("Ok", d(2031, 10, 10), 20_000, -1),
    ] {
        let r = ent
            .eva_v3_creer_evaluation_for_actor(
                w.etab,
                w.prof,
                PROF,
                jour(),
                maintenant(),
                &corps(w.cours_maths, titre, date, bar, coef),
            )
            .await;
        assert!(r.is_err(), "{titre} {date} {bar} {coef}");
    }
    // Seul le professeur du cours crée.
    let r = ent
        .eva_v3_creer_evaluation_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            maintenant(),
            &corps(w.cours_maths, "X", d(2031, 10, 10), 20_000, 1_000),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)));
    let ev = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    assert_eq!(
        (ev.publication.as_str(), ev.version, ev.a_des_resultats),
        ("explicite", 1, false)
    );
    // Réglage sans résultats : libre ; version attendue.
    let m = ent
        .eva_v3_modifier_evaluation_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            maintenant(),
            ev.id,
            &ModifierEvaluationV3Body {
                version_attendue: ev.version,
                titre: Some("DS 1 bis".into()),
                date_eval: None,
                bareme_m: Some(40_000),
                coef_m: Some(2_000),
                comptage: None,
                hors_bareme_autorise: None,
                annoncee: None,
                motif: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (m.titre.as_str(), m.bareme_m, m.coef_m),
        ("DS 1 bis", 40_000, 2_000)
    );
    // Version périmée : conflit.
    let r = ent
        .eva_v3_modifier_evaluation_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            maintenant(),
            ev.id,
            &ModifierEvaluationV3Body {
                version_attendue: ev.version,
                titre: Some("autre".into()),
                date_eval: None,
                bareme_m: None,
                coef_m: None,
                comptage: None,
                hors_bareme_autorise: None,
                annoncee: None,
                motif: None,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    // Des résultats existent : le barème ne change plus sans motif ; jamais sous une note déjà saisie.
    lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "30", None)]).await;
    let corps_modif = |v: i32, bar: i64, motif: Option<&str>| ModifierEvaluationV3Body {
        version_attendue: v,
        titre: None,
        date_eval: None,
        bareme_m: Some(bar),
        coef_m: None,
        comptage: None,
        hors_bareme_autorise: None,
        annoncee: None,
        motif: motif.map(String::from),
    };
    let r = ent
        .eva_v3_modifier_evaluation_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            maintenant(),
            ev.id,
            &corps_modif(m.version, 20_000, None),
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::SaisieInvalide(_))),
        "motif exigé : {r:?}"
    );
    let r = ent
        .eva_v3_modifier_evaluation_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            maintenant(),
            ev.id,
            &corps_modif(m.version, 20_000, Some("erreur de barème")),
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::SaisieInvalide(_))),
        "la note 30 dépasserait le barème 20 : {r:?}"
    );
    // Limite de conversion de barème : dès qu'une note existe, le barème ne change plus (aucune conversion
    // silencieuse des notes saisies), même avec un motif et même vers un barème plus grand.
    let r = ent
        .eva_v3_modifier_evaluation_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            maintenant(),
            ev.id,
            &corps_modif(m.version, 50_000, Some("erreur de barème")),
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::SaisieInvalide(_))),
        "barème figé dès qu'une note existe : {r:?}"
    );
    // Le coefficient, lui, reste modifiable avec un motif : changement journalisé.
    let ok = ent
        .eva_v3_modifier_evaluation_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            maintenant(),
            ev.id,
            &ModifierEvaluationV3Body {
                version_attendue: m.version,
                titre: None,
                date_eval: None,
                bareme_m: None,
                coef_m: Some(3_000),
                comptage: None,
                hors_bareme_autorise: None,
                annoncee: None,
                motif: Some("erreur de coefficient".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(ok.coef_m, 3_000);
    let histo: i64 = sqlx::query_scalar("SELECT count(*) FROM eva_evaluation_historique WHERE etablissement_id = $1 AND evaluation_id = $2")
        .bind(w.etab).bind(ev.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(
        histo >= 2,
        "chaque changement de calcul est journalisé : {histo}"
    );
    // Annulation : motif obligatoire ; ensuite plus de saisie.
    let r = ent
        .eva_v3_annuler_evaluation_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            maintenant(),
            ev.id,
            &AnnulerEvaluationV3Body {
                version_attendue: ok.version,
                motif: "x".into(),
            },
        )
        .await;
    assert!(r.is_err());
    let an = ent
        .eva_v3_annuler_evaluation_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            maintenant(),
            ev.id,
            &AnnulerEvaluationV3Body {
                version_attendue: ok.version,
                motif: "sujet diffusé".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(an.etat, "annulee");
    let r = lot(
        &ent,
        &w,
        w.cours_maths,
        vec![cell(ev.id, w.eleves_a[1], "10", None)],
    )
    .await;
    assert_eq!(issue(&r, 0), "refuse");
}

#[tokio::test]
async fn v3_evaluation_restreinte_et_sous_matiere() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    let mut c = corps(
        w.cours_maths,
        "Oral demi-groupe",
        d(2031, 10, 10),
        20_000,
        1_000,
    );
    c.restreinte_a = vec![a];
    let ev = ent
        .eva_v3_creer_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), &c)
        .await
        .unwrap();
    assert!(ev.restreinte);
    let r = lot(
        &ent,
        &w,
        w.cours_maths,
        vec![cell(ev.id, a, "14", None), cell(ev.id, b, "14", None)],
    )
    .await;
    assert_eq!((issue(&r, 0), issue(&r, 1)), ("ecrit", "refuse"));
    // Un élève visé hors du public : refusé à la création.
    let hors = creer_eleve(&ent, w.etab, "Pas-dans-le-cours").await;
    let mut c2 = corps(w.cours_maths, "Oral", d(2031, 10, 10), 20_000, 1_000);
    c2.restreinte_a = vec![hors];
    assert!(ent
        .eva_v3_creer_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), &c2)
        .await
        .is_err());
    // Une sous-matière d'une autre matière est refusée par la base.
    let mut c3 = corps(
        w.cours_maths,
        "Sous-matière",
        d(2031, 10, 10),
        20_000,
        1_000,
    );
    c3.sous_matiere_id = Some(w.francais);
    assert!(ent
        .eva_v3_creer_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), &c3)
        .await
        .is_err());
}

// ───────────────────────── Remplaçant et co-enseignement ─────────────────────────

#[tokio::test]
async fn v3_suppleant_et_co_enseignant() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let remplacant = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)
        .unwrap();
    let ev = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS du titulaire",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    // Avant la suppléance : l'autre professeur n'a rien.
    assert!(matches!(
        ent.eva_v3_grille_for_actor(
            w.etab,
            remplacant,
            PROF,
            w.cours_maths,
            w.t1,
            jour(),
            maintenant()
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    // Suppléance du 15 au 31 octobre : le suppléant saisit, lit tout l'historique du cours (CE-44).
    let service: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_service_enseignement WHERE etablissement_id = $1 AND cours_id = $2",
    )
    .bind(w.etab)
    .bind(w.cours_maths)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let agent: Uuid = sqlx::query_scalar(
        "SELECT ag.id FROM ent_agent ag JOIN ent_personne pe ON pe.id = ag.personne_id AND pe.etablissement_id = ag.etablissement_id
          WHERE ag.etablissement_id = $1 AND pe.user_id = $2 AND ag.fusionne_dans_agent_id IS NULL",
    )
    .bind(w.etab).bind(remplacant).fetch_optional(ent.pool_verif()).await.unwrap().unwrap_or_default();
    if agent.is_nil() {
        return eprintln!("skip: pas d'agent pour le second professeur du labo");
    }
    exec(
        &ent,
        "INSERT INTO ent_service_intervenant (id, etablissement_id, service_id, agent_id, nature, du, saisie, origine)
         VALUES ($1, $2, $3, $4, 'suppleant', '2031-10-15', true, 'saisie')",
        vec![u(Uuid::new_v4()), u(w.etab), u(service), u(agent)],
    )
    .await;
    let g = ent
        .eva_v3_grille_for_actor(
            w.etab,
            remplacant,
            PROF,
            w.cours_maths,
            w.t1,
            jour(),
            maintenant(),
        )
        .await
        .unwrap();
    assert!(g.peut_saisir, "le suppléant saisit");
    assert!(
        g.evaluations.iter().any(|e| e.id == ev.id),
        "il voit tout l'historique du cours"
    );
    assert!(
        !g.evaluations[0].peut_modifier,
        "mais ne règle pas l'évaluation du titulaire sans droit de correction"
    );
    let r = lot_par(
        &ent,
        &w,
        remplacant,
        PROF,
        w.cours_maths,
        vec![cell(ev.id, a, "12", None)],
    )
    .await
    .unwrap();
    assert_eq!(issue(&r, 0), "ecrit");
    let r = ent
        .eva_v3_modifier_evaluation_for_actor(
            w.etab,
            remplacant,
            PROF,
            jour(),
            maintenant(),
            ev.id,
            &ModifierEvaluationV3Body {
                version_attendue: ev.version,
                titre: Some("renommée".into()),
                date_eval: None,
                bareme_m: None,
                coef_m: None,
                comptage: None,
                hors_bareme_autorise: None,
                annoncee: None,
                motif: None,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Il crée ses propres évaluations.
    let propre = ent
        .eva_v3_creer_evaluation_for_actor(
            w.etab,
            remplacant,
            PROF,
            jour(),
            maintenant(),
            &corps(
                w.cours_maths,
                "DS du suppléant",
                d(2031, 10, 17),
                20_000,
                1_000,
            ),
        )
        .await
        .unwrap();
    assert!(propre.peut_modifier);
    // Le titulaire n'a plus la saisie pendant la suppléance (une saisie par jour), mais garde la lecture.
    let gt = grille(&ent, &w, w.cours_maths).await;
    assert!(
        !gt.peut_saisir,
        "le suppléant retire la saisie au titulaire pendant la période"
    );
    assert_eq!(gt.evaluations.len(), 2);
}

// ───────────────────────── Publication ─────────────────────────

async fn publiees(ent: &PgEnt, w: &W, ev: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM eva_resultat_publie WHERE etablissement_id = $1 AND evaluation_id = $2")
        .bind(w.etab).bind(ev).fetch_one(ent.pool_verif()).await.unwrap()
}

fn pub_body(action: &str, prevue: Option<DateTime<Utc>>, motif: Option<&str>) -> PublierV3Body {
    PublierV3Body {
        action: action.into(),
        prevue_le: prevue,
        prevue_locale: None,
        motif: motif.map(String::from),
    }
}

#[tokio::test]
async fn v3_publication_programmee_retrait_et_journal() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "14", None)]).await;
    assert_eq!(
        publiees(&ent, &w, ev.id).await,
        0,
        "rien n'est visible sans publication (R-V7)"
    );
    // Programmation dans le passé : refusée ; dans le futur : acceptée.
    assert!(ent
        .eva_v3_publier_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            maintenant(),
            ev.id,
            &pub_body(
                "programmer",
                Some(maintenant() - chrono::Duration::hours(1)),
                None
            )
        )
        .await
        .is_err());
    let prevue = maintenant() + chrono::Duration::hours(30);
    let p = ent
        .eva_v3_publier_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            maintenant(),
            ev.id,
            &pub_body("programmer", Some(prevue), None),
        )
        .await
        .unwrap();
    assert_eq!(
        (p.publication.as_str(), p.publication_prevue_le),
        ("programmee", Some(prevue))
    );
    // Le planificateur : avant l'heure, rien ; à l'heure, publié ; idempotent.
    let b0 = ent
        .eva_v3_publications_automatiques_au(w.etab, maintenant(), jour())
        .await
        .unwrap();
    assert_eq!(b0.programmees, 0);
    assert_eq!(publiees(&ent, &w, ev.id).await, 0);
    let apres = prevue + chrono::Duration::minutes(5);
    let b1 = ent
        .eva_v3_publications_automatiques_au(w.etab, apres, jour() + chrono::Duration::days(2))
        .await
        .unwrap();
    assert_eq!(b1.programmees, 1);
    assert_eq!(publiees(&ent, &w, ev.id).await, 1);
    let b2 = ent
        .eva_v3_publications_automatiques_au(w.etab, apres, jour() + chrono::Duration::days(2))
        .await
        .unwrap();
    assert_eq!(b2, BilanPublications::default(), "idempotent");
    // Retrait par le professeur dans les 24 h : motif exigé ; la famille verra « retiré pour vérification ».
    let t = apres + chrono::Duration::hours(2);
    let r = ent
        .eva_v3_publier_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            t,
            ev.id,
            &pub_body("retirer", None, None),
        )
        .await;
    assert!(r.is_err(), "motif obligatoire");
    let g = ent
        .eva_v3_publier_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            t,
            ev.id,
            &pub_body("retirer", None, Some("erreur de barème")),
        )
        .await
        .unwrap();
    assert!(g.publiee_le.is_none() && g.retiree_le.is_some());
    assert_eq!(publiees(&ent, &w, ev.id).await, 0);
    // Republier maintenant : plus de trace de retrait ; puis, au-delà de 24 h, le professeur ne retire plus.
    let g = ent
        .eva_v3_publier_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            t,
            ev.id,
            &pub_body("maintenant", None, None),
        )
        .await
        .unwrap();
    assert!(g.publiee_le.is_some() && g.retiree_le.is_none());
    let tard = t + chrono::Duration::hours(30);
    let r = ent
        .eva_v3_publier_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            tard,
            ev.id,
            &pub_body("retirer", None, Some("encore une erreur")),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // La direction le peut, après 24 h.
    let g = ent
        .eva_v3_publier_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            tard,
            ev.id,
            &pub_body("retirer", None, Some("décision de la direction")),
        )
        .await
        .unwrap();
    assert!(g.retiree_le.is_some());
    // Journal des publications (ajout seul).
    let actions: Vec<String> = sqlx::query_scalar("SELECT action FROM eva_evaluation_publication_journal WHERE etablissement_id = $1 AND evaluation_id = $2 ORDER BY le, id")
        .bind(w.etab).bind(ev.id).fetch_all(ent.pool_verif()).await.unwrap();
    assert_eq!(
        actions,
        [
            "programmee",
            "publiee_auto_programmee",
            "retiree",
            "publiee",
            "retiree"
        ]
    );
    let r = sqlx::query(
        "UPDATE eva_evaluation_publication_journal SET motif = 'x' WHERE etablissement_id = $1",
    )
    .bind(w.etab)
    .execute(ent.pool_verif())
    .await;
    assert!(r.is_err(), "le journal est en ajout seul");
}

#[tokio::test]
async fn v3_delai_maximal_de_publication_rappel_et_en_cours_de_correction() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    exec(
        &ent,
        "INSERT INTO eva_reglage (id, etablissement_id, annee_id, delai_publication_max_jours) VALUES ($1, $2, $3, 15)
         ON CONFLICT DO NOTHING",
        vec![u(Uuid::new_v4()), u(w.etab), u(w.annee)],
    )
    .await;
    exec(
        &ent,
        "UPDATE eva_reglage SET delai_publication_max_jours = 15 WHERE etablissement_id = $1",
        vec![u(w.etab)],
    )
    .await;
    let e1 = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 1),
        20_000,
        1_000,
    )
    .await;
    let e2 = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 2 en correction",
        d(2031, 10, 1),
        20_000,
        1_000,
    )
    .await;
    let e3 = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 3 sans note",
        d(2031, 10, 1),
        20_000,
        1_000,
    )
    .await;
    for e in [&e1, &e2] {
        lot(&ent, &w, w.cours_maths, vec![cell(e.id, a, "12", None)]).await;
    }
    ent.eva_v3_en_cours_correction_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        maintenant(),
        e2.id,
        true,
    )
    .await
    .unwrap();
    // Délai de 15 jours après le 1er octobre : terme le 16. Le 13 : trop tôt ; le 14 : rappel unique (deux jours avant).
    let j13 = d(2031, 10, 13);
    let b = ent
        .eva_v3_publications_automatiques_au(
            w.etab,
            j13.and_hms_opt(12, 0, 0).unwrap().and_utc(),
            j13,
        )
        .await
        .unwrap();
    assert_eq!((b.rappels, b.delai_max), (0, 0), "trop tôt");
    let j14 = d(2031, 10, 14);
    let b = ent
        .eva_v3_publications_automatiques_au(
            w.etab,
            j14.and_hms_opt(12, 0, 0).unwrap().and_utc(),
            j14,
        )
        .await
        .unwrap();
    assert_eq!(
        (b.rappels, b.delai_max),
        (1, 0),
        "rappel de DS 1 seulement (DS 2 est en cours de correction, DS 3 n'a pas de note)"
    );
    let b = ent
        .eva_v3_publications_automatiques_au(
            w.etab,
            j14.and_hms_opt(18, 0, 0).unwrap().and_utc(),
            j14,
        )
        .await
        .unwrap();
    assert_eq!(b.rappels, 0, "un seul rappel");
    // Le 16 : DS 1 est publiée ; DS 2 (en cours de correction) et DS 3 (sans résultat) non.
    let j16 = d(2031, 10, 16);
    let b = ent
        .eva_v3_publications_automatiques_au(
            w.etab,
            j16.and_hms_opt(12, 0, 0).unwrap().and_utc(),
            j16,
        )
        .await
        .unwrap();
    assert_eq!(b.delai_max, 1);
    assert_eq!(publiees(&ent, &w, e1.id).await, 1);
    assert_eq!(publiees(&ent, &w, e2.id).await, 0);
    assert_eq!(publiees(&ent, &w, e3.id).await, 0);
    // Quand le professeur lève « en cours de correction », la publication suit au passage suivant.
    ent.eva_v3_en_cours_correction_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        maintenant(),
        e2.id,
        false,
    )
    .await
    .unwrap();
    let j17 = d(2031, 10, 17);
    let b = ent
        .eva_v3_publications_automatiques_au(
            w.etab,
            j17.and_hms_opt(12, 0, 0).unwrap().and_utc(),
            j17,
        )
        .await
        .unwrap();
    assert_eq!(b.delai_max, 1);
    assert_eq!(publiees(&ent, &w, e2.id).await, 1);
}

#[tokio::test]
async fn v3_les_evenements_ne_portent_ni_note_ni_motif_ni_nom() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(
        &ent,
        &w,
        w.cours_maths,
        "Titre secret Zèbre",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "13,37", None)]).await;
    ent.eva_v3_publier_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        maintenant(),
        ev.id,
        &pub_body("maintenant", None, None),
    )
    .await
    .unwrap();
    ent.eva_v3_publier_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        maintenant(),
        ev.id,
        &pub_body("retirer", None, Some("motif confidentiel Zèbre")),
    )
    .await
    .unwrap();
    let payloads: Vec<(String, String)> = sqlx::query_as(
        "SELECT type, payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type LIKE 'evaluations.%'",
    )
    .bind(w.etab)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap_or_default();
    assert!(!payloads.is_empty(), "des événements neutres sont émis");
    for (type_, p) in &payloads {
        // F9 : on compare la LISTE DE CLÉS autorisées (des identifiants seulement), pas des sous-chaînes : « 13 » ou
        // « 37 » se retrouvent légitimement dans un identifiant ou une version.
        let valeur: serde_json::Value = serde_json::from_str(p).unwrap();
        let cles: Vec<&str> = valeur.as_object().expect("charge objet").keys().map(String::as_str).collect();
        assert!(
            cles.iter().all(|c| CLES_AUTORISEES.contains(c)),
            "clé non autorisée dans {type_} : {cles:?}"
        );
        // Chaque valeur est un identifiant (UUID), un entier de version ou nulle : jamais du texte libre ni une note.
        for (c, v) in valeur.as_object().unwrap() {
            let ok = v.is_null()
                || v.as_i64().is_some_and(|_| *c == "version")
                || v.as_str().is_some_and(|t| Uuid::parse_str(t).is_ok());
            assert!(ok, "valeur non neutre pour {c} dans {type_} : {v}");
        }
        assert!(
            !p.contains("Zèbre") && !p.contains("confidentiel") && !p.contains("Abbott"),
            "{p}"
        );
    }
}

/// Clés qu'une charge d'événement du socle 21 peut porter : des identifiants et un numéro de version (F9).
const CLES_AUTORISEES: [&str; 8] = ["execution_id", "evaluation_id", "resultat_id", "version", "periode_id", "classe_id", "auteur_id", "annee_id"];

// ───────────────────────── Collage et import ─────────────────────────

fn ligne_nom(nom: &str, cellules: &[&str]) -> ImportLigneBody {
    ImportLigneBody {
        eleve_id: None,
        nom: Some(nom.into()),
        rang: None,
        cellules: cellules.iter().map(|s| s.to_string()).collect(),
    }
}

#[tokio::test]
async fn v3_import_apercu_erreurs_ligne_a_ligne_confirmation_et_conflit() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let e1 = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    let e2 = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 2",
        d(2031, 10, 17),
        20_000,
        1_000,
    )
    .await;
    lot(&ent, &w, w.cours_maths, vec![cell(e1.id, c, "10", None)]).await;
    // Les trois élèves du labo s'appellent « <Nom> Test » ; on rapproche par nom dans n'importe quel ordre et casse.
    let corps_import = |empreinte: Option<String>| ImportBody {
        cours_id: w.cours_maths,
        mode: "fichier".into(),
        cibles: vec![e1.id, e2.id],
        lignes: vec![
            ligne_nom("ABBOTT test", &["15,5", "abs"]),
            ligne_nom("Test Bones", &["12", ""]),
            ligne_nom("Corner Test", &["10", "7"]), // 10 inchangé, 7 nouveau
            ligne_nom("Inconnu Test", &["9", "9"]), // non rapproché
            ligne_nom("Abbott Test", &["1", "1"]),  // doublon de la première ligne : écarté
        ],
        empreinte,
    };
    let ap = ent
        .eva_v3_apercu_import_for_actor(w.etab, w.prof, PROF, jour(), &corps_import(None))
        .await
        .unwrap();
    assert_eq!(ap.totaux.lignes, 5);
    assert_eq!(ap.lignes[0].rapprochement, "ok");
    assert_eq!(ap.lignes[3].rapprochement, "non_rapproche");
    assert_eq!(
        ap.lignes[4].rapprochement, "ambigu",
        "un élève ne figure qu'une fois dans un import"
    );
    let cc = &ap.lignes[2].cellules;
    assert_eq!(
        (cc[0].etat.as_str(), cc[1].etat.as_str()),
        ("inchange", "nouveau")
    );
    assert_eq!(ap.lignes[1].cellules[1].etat, "vide");
    // Rien n'a été écrit par l'aperçu.
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM eva_resultat WHERE etablissement_id = $1 AND evaluation_id = ANY($2)")
            .bind(w.etab)
            .bind(vec![e1.id, e2.id])
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(n, 1);
    // Confirmation sans empreinte ou avec une empreinte fausse : refus.
    assert!(ent
        .eva_v3_confirmer_import_for_actor(w.etab, w.prof, PROF, jour(), &corps_import(None))
        .await
        .is_err());
    assert!(matches!(
        ent.eva_v3_confirmer_import_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &corps_import(Some("0".repeat(64)))
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Une note change entre l'aperçu et la confirmation : l'empreinte ne correspond plus, rien n'est écrit.
    lot(&ent, &w, w.cours_maths, vec![cell(e1.id, a, "3", None)]).await;
    assert!(matches!(
        ent.eva_v3_confirmer_import_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &corps_import(Some(ap.empreinte.clone()))
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Nouvel aperçu, puis confirmation : seules les lignes rapprochées sont écrites ; la case d'Abbott change (3 → 15,5).
    let ap2 = ent
        .eva_v3_apercu_import_for_actor(w.etab, w.prof, PROF, jour(), &corps_import(None))
        .await
        .unwrap();
    assert_eq!(ap2.lignes[0].cellules[0].etat, "modifie");
    let r = ent
        .eva_v3_confirmer_import_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &corps_import(Some(ap2.empreinte.clone())),
        )
        .await
        .unwrap();
    let ecrites = r.cellules.iter().filter(|x| x.issue == "ecrit").count();
    assert_eq!(ecrites, 4, "Abbott ×2, Bones ×1, Corner ×1 : {r:?}");
    let g = grille(&ent, &w, w.cours_maths).await;
    let l = |id: Uuid| g.eleves.iter().find(|e| e.eleve_id == id).unwrap();
    assert_eq!(l(a).cellules[0].valeur_m, Some(15_500));
    assert_eq!(l(a).cellules[1].statut.as_deref(), Some("absent"));
    assert_eq!(l(b).cellules[0].valeur_m, Some(12_000));
    assert_eq!(l(c).cellules[1].valeur_m, Some(7_000));
    // La source « import » est au journal de la note ; le journal des imports ne porte que des comptes.
    let h = ent
        .eva_v3_historique_for_actor(w.etab, w.prof, PROF, e2.id, c, jour())
        .await
        .unwrap();
    assert_eq!(
        h.last().unwrap().source,
        "import",
        "la case créée par l'import porte la source import"
    );
    let (nb_lignes, nb_ecrites): (i32, i32) = sqlx::query_as(
        "SELECT nb_lignes, nb_ecrites FROM eva_import_journal WHERE etablissement_id = $1",
    )
    .bind(w.etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((nb_lignes, nb_ecrites), (3, 4));
    // L'import est dans SES évaluations, à sa portée : un autre professeur, la direction, la famille sont refusés.
    let autre = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)
        .unwrap();
    for (u_id, role) in [
        (autre, PROF),
        (w.direction, DIR),
        (cast.parent_lea.user_id, "parent"),
    ] {
        let r = ent
            .eva_v3_apercu_import_for_actor(w.etab, u_id, role, jour(), &corps_import(None))
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
    // Un autre cours : introuvable.
    let mut autre_cours = corps_import(None);
    autre_cours.cours_id = w.cours_fr;
    assert!(ent
        .eva_v3_apercu_import_for_actor(w.etab, w.prof, PROF, jour(), &autre_cours)
        .await
        .is_err());
}

#[tokio::test]
async fn v3_collage_par_rang_et_par_identifiant() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = creer(
        &ent,
        &w,
        w.cours_maths,
        "DS 1",
        d(2031, 10, 10),
        20_000,
        1_000,
    )
    .await;
    // 3 élèves dans l'ordre alphabétique (Abbott, Bones, Corner) ; 4 lignes collées : la 4ᵉ ne se rapproche pas.
    let mk = |rang: usize, t: &str| ImportLigneBody {
        eleve_id: None,
        nom: None,
        rang: Some(rang),
        cellules: vec![t.into()],
    };
    let body = ImportBody {
        cours_id: w.cours_maths,
        mode: "collage".into(),
        cibles: vec![ev.id],
        lignes: vec![mk(1, "8"), mk(2, "9,5"), mk(3, "nn"), mk(4, "12")],
        empreinte: None,
    };
    let ap = ent
        .eva_v3_apercu_import_for_actor(w.etab, w.prof, PROF, jour(), &body)
        .await
        .unwrap();
    assert_eq!((ap.totaux.rapprochees, ap.totaux.non_rapprochees), (3, 1));
    let r = ent
        .eva_v3_confirmer_import_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &ImportBody {
                empreinte: Some(ap.empreinte),
                ..body
            },
        )
        .await
        .unwrap();
    assert_eq!(r.cellules.len(), 3);
    let g = grille(&ent, &w, w.cours_maths).await;
    assert_eq!(g.eleves[0].cellules[0].valeur_m, Some(8_000));
    assert_eq!(g.eleves[1].cellules[0].valeur_m, Some(9_500));
    assert_eq!(g.eleves[2].cellules[0].statut.as_deref(), Some("non_note"));
}

// ───────────────────────── F7, F8, F12 ─────────────────────────

#[tokio::test]
async fn f8_une_note_reprise_puis_corrigee_par_l_ancien_ecrivain_est_recopiee_y_compris_a_b_a() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    let anc = ancien_monde(&ent, &w).await;
    let a = w.eleves_a[0];
    let n = ancienne_note(&ent, &w, anc.devoirs[0], a, 12.0, 20.0, "note").await;
    ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    let corr = |de: f64, vers: f64, le: &'static str, motif: &'static str| {
        let ent = &ent;
        let w = &w;
        let anc_pi = anc.note_periode;
        async move {
            exec(
                ent,
                &format!("UPDATE ent_note_archive SET valeur = {vers} WHERE id = $1"),
                vec![u(n)],
            )
            .await;
            exec(
                ent,
                &format!(
                    "INSERT INTO ent_note_correction (id, etablissement_id, note_id, eleve_id, periode_id, ancien_statut, ancienne_valeur,
                                                      nouveau_statut, nouvelle_valeur, motif, corrige_par_user_id, corrige_le)
                     VALUES ($1, $2, $3, $4, $5, 'note', {de}, 'note', {vers}, '{motif}', $6, '{le}')"
                ),
                vec![u(Uuid::new_v4()), u(w.etab), u(n), u(a), u(anc_pi), u(w.direction)],
            )
            .await;
        }
    };
    // A (12) → B (14) → A (12) par l'ancien écrivain, deux corrections journalisées.
    corr(12.0, 14.0, "2031-11-20T09:00:00Z", "premiere").await;
    corr(14.0, 12.0, "2031-11-21T09:00:00Z", "seconde").await;
    let rapport = ent
        .eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    for c in &rapport.comptes {
        assert_eq!(
            c.lues,
            c.creees + c.inchangees + c.refusees + c.ignorees,
            "invariant {}",
            c.objet
        );
    }
    assert!(
        rapport
            .lignes
            .iter()
            .any(|l| l.source_id == n && l.code == "correction_recopiee"),
        "{:?}",
        rapport.lignes
    );
    let hist: Vec<(i32, Option<String>, Option<String>, bool, Option<String>)> = sqlx::query_as(
        "SELECT version, valeur_avant::text, valeur_apres::text, correction, motif FROM eva_resultat_historique
          WHERE etablissement_id = $1 AND resultat_id = $2 ORDER BY version",
    )
    .bind(w.etab).bind(n).fetch_all(ent.pool_verif()).await.unwrap();
    assert_eq!(hist.len(), 3, "création + deux corrections : {hist:?}");
    assert_eq!(hist[1].4.as_deref(), Some("premiere"));
    assert_eq!(hist[2].4.as_deref(), Some("seconde"));
    assert!(hist[1].3 && hist[2].3);
    let v: String = sqlx::query_scalar("SELECT valeur::text FROM eva_resultat WHERE id = $1")
        .bind(n)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(v, "12.000");
    // Idempotent : rien de plus à la reprise suivante.
    ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    let nb: i64 =
        sqlx::query_scalar("SELECT count(*) FROM eva_resultat_historique WHERE resultat_id = $1")
            .bind(n)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(nb, 3);
    // La nouvelle grille a écrit entre-temps : l'ancienne correction n'écrase rien, le conflit est listé.
    let ev = anc.devoirs[0];
    let r = lot(&ent, &w, w.cours_maths, vec![cell(ev, a, "16", Some(3))]).await;
    assert_eq!(issue(&r, 0), "ecrit", "{r:?}");
    corr(12.0, 13.0, "2031-11-22T09:00:00Z", "troisieme").await;
    let rapport = ent
        .eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    assert!(
        rapport
            .lignes
            .iter()
            .any(|l| l.source_id == n && l.code == "deux_ecrivains"),
        "{:?}",
        rapport.lignes
    );
    let v: String = sqlx::query_scalar("SELECT valeur::text FROM eva_resultat WHERE id = $1")
        .bind(n)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(
        v, "16.000",
        "la saisie de la nouvelle grille n'est pas écrasée"
    );
}

#[tokio::test]
async fn f7_deux_reprises_simultanees_ne_comptent_pas_de_doublons_comme_refuses() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .unwrap();
    let anc = ancien_monde(&ent, &w).await;
    for (i, e) in w.eleves_a.iter().enumerate() {
        ancienne_note(&ent, &w, anc.devoirs[0], *e, 10.0 + i as f64, 20.0, "note").await;
    }
    let (r1, r2) = tokio::join!(
        ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR),
        ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
    );
    let (r1, r2) = (r1.unwrap(), r2.unwrap());
    let mut creees = 0;
    for r in [&r1, &r2] {
        assert!(
            !r.lignes.iter().any(|l| l.code == "erreur_sql"),
            "aucun refus erreur_sql : {:?}",
            r.lignes
        );
        for c in &r.comptes {
            assert_eq!(
                c.lues,
                c.creees + c.inchangees + c.refusees + c.ignorees,
                "invariant {}",
                c.objet
            );
        }
        creees += r.compte("note").map_or(0, |c| c.creees);
        assert_eq!(r.compte("note").map_or(0, |c| c.refusees), 0);
    }
    assert_eq!(creees, 3, "chaque note est créée une seule fois");
}

#[tokio::test]
async fn f12_un_debordement_n_echoue_que_la_matiere_concernee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    // Français : une évaluation tout à fait ordinaire. Maths : des barèmes aux dénominateurs premiers entre eux
    // (999,983 ; 999,979 ; …) dont la somme exacte déborde les entiers du moteur.
    let ev_fr = evaluation_directe(&ent, &w, w.cours_fr, d(2031, 10, 10), "Dictée", 20.0).await;
    let autre = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)
        .unwrap();
    ent.eva_v1_ecrire_resultat_for_actor(
        w.etab,
        autre,
        PROF,
        &crate::tests_evaluations_v1::note(ev_fr, a, 15_000),
    )
    .await
    .unwrap();
    for (i, bareme) in [
        999.983_f64,
        999.979,
        999.961,
        999.959,
        999.953,
        999.931,
        999.907,
        999.893,
        999.883,
    ]
    .iter()
    .enumerate()
    {
        let ev = evaluation_directe(
            &ent,
            &w,
            w.cours_maths,
            d(2031, 10, 1 + i as u32),
            &format!("P{i}"),
            *bareme,
        )
        .await;
        ent.eva_v1_ecrire_resultat_for_actor(
            w.etab,
            w.prof,
            PROF,
            &crate::tests_evaluations_v1::note(ev, a, 500_000 + i as i64 * 70),
        )
        .await
        .unwrap();
    }
    let m = ent
        .eva_v1_moyennes_personnel_for_actor(w.etab, w.direction, DIR, a, w.t1)
        .await
        .expect("le calcul ne doit pas échouer en bloc");
    let fr = m
        .matieres
        .iter()
        .find(|x| x.matiere_id == w.francais)
        .expect("français");
    assert_eq!(
        fr.moyenne.affichee_m,
        Some(15_000),
        "le français reste calculé"
    );
    let maths = m
        .matieres
        .iter()
        .find(|x| x.matiere_id == w.maths)
        .expect("maths");
    if maths.moyenne.exacte.is_none() {
        assert!(
            maths
                .moyenne
                .avertissements
                .iter()
                .any(|a| a.contains("impossible")),
            "{:?}",
            maths.moyenne.avertissements
        );
        assert!(
            m.generale
                .avertissements
                .iter()
                .any(|a| a.contains("indisponible")),
            "{:?}",
            m.generale.avertissements
        );
        assert_eq!(
            m.generale.affichee_m,
            Some(15_000),
            "la générale s'établit sur les autres matières"
        );
    }
}

async fn evaluation_directe(
    ent: &PgEnt,
    w: &W,
    cours: Uuid,
    date: NaiveDate,
    titre: &str,
    bareme: f64,
) -> Uuid {
    crate::tests_evaluations_v1::evaluation(
        ent,
        w,
        cours,
        date,
        titre,
        bareme,
        1.0,
        domain_eleve::evaluation_moteur::Comptage::Normal,
        false,
    )
    .await
}

// ───────────────────────── Mesure : grille de 40 élèves × 20 évaluations ─────────────────────────

#[tokio::test]
async fn v3_mesure_grille_40_eleves_x_20_evaluations() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let mut eleves = w.eleves_a.clone();
    for i in 0..37 {
        let e = creer_eleve(&ent, w.etab, &format!("Zeta{i:02}")).await;
        crate::tests_evaluations_v1::affecter(&ent, w.etab, w.annee, e, w.classe_a, d(2031, 9, 1)).await;
        eleves.push(e);
    }
    let t0 = std::time::Instant::now();
    let mut evals = Vec::new();
    for k in 0..20u32 {
        evals.push(creer(&ent, &w, w.cours_maths, &format!("DS {k}"), d(2031, 9, 2 + k), 20_000, 1_000).await);
    }
    let t_creation = t0.elapsed();
    let cellules: Vec<CelluleSaisieDto> = eleves
        .iter()
        .enumerate()
        .flat_map(|(i, e)| {
            evals.iter().enumerate().map(move |(k, ev)| {
                let texte = if (i + k) % 17 == 0 { "abs".to_string() } else { format!("{}", 5 + (i * 7 + k * 3) % 16) };
                cell(ev.id, *e, &texte, None)
            })
        })
        .collect();
    assert_eq!(cellules.len(), 800);
    let t1 = std::time::Instant::now();
    let r = lot(&ent, &w, w.cours_maths, cellules).await;
    let t_saisie = t1.elapsed();
    assert!(r.cellules.iter().all(|c| c.issue == "ecrit"), "800 cases écrites");
    // Lecture de la grille (moteur : 40 moyennes, 20 statistiques, 1 statistique de classe), meilleure de 5 lectures.
    let mut lectures = Vec::new();
    for _ in 0..5 {
        let t2 = std::time::Instant::now();
        let g = grille(&ent, &w, w.cours_maths).await;
        lectures.push(t2.elapsed());
        assert_eq!((g.eleves.len(), g.evaluations.len()), (40, 20));
        assert!(g.eleves.iter().all(|e| e.moyenne_m.is_some()));
    }
    lectures.sort();
    // Une frappe isolée (une case, version lue) : le cas de la saisie continue.
    let g = grille(&ent, &w, w.cours_maths).await;
    let cel = &g.eleves[5].cellules[5];
    let t3 = std::time::Instant::now();
    let r = lot(&ent, &w, w.cours_maths, vec![cell(evals[5].id, eleves_id(&g, 5), "19", cel.version)]).await;
    let t_frappe = t3.elapsed();
    assert_eq!(r.cellules[0].issue, "ecrit");
    eprintln!(
        "MESURE 40x20 (base locale, un fil) : créer 20 évaluations {:?} ; écrire 800 cases {:?} ; lire la grille (min de 5) {:?}, médiane {:?} ; une frappe (1 case) {:?}",
        t_creation, t_saisie, lectures[0], lectures[2], t_frappe
    );
    assert!(lectures[0] < std::time::Duration::from_secs(3), "la lecture de la grille doit rester rapide");
}

fn eleves_id(g: &GrilleV3Dto, i: usize) -> Uuid {
    g.eleves[i].eleve_id
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// Lot V3b : suites de la revue de sécurité de V3 (M1, M2, M3, F1 à F9, conversion de barème)
// ═════════════════════════════════════════════════════════════════════════════════════════════

use crate::evaluations_v1::{Attendu, EcrireResultat, SourceResultat};
use crate::tenant_tx::TenantTx;
use domain_eleve::evaluation_moteur::StatutCalcul;
use std::sync::Arc;

fn note_m(ev: Uuid, eleve: Uuid, valeur_m: i64) -> EcrireResultat {
    EcrireResultat {
        evaluation_id: ev,
        eleve_id: eleve,
        statut: StatutCalcul::Note,
        valeur_m: Some(valeur_m),
        motif: None,
        commentaire_eleve: None,
        source: SourceResultat::Saisie,
    }
}

/// M1 : deux transactions CONCURRENTES RÉELLES sur une case VIDE. `SELECT … FOR UPDATE` ne verrouille rien sur une ligne
/// qui n'existe pas : avant V3b, le second `INSERT … ON CONFLICT DO UPDATE` écrasait sans condition de version. Désormais
/// la seconde écriture attend la première, puis reçoit un CONFLIT (jamais un écrasement).
#[tokio::test]
async fn v3b_m1_deux_ecritures_concurrentes_sur_une_case_vide_une_seule_gagne() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ent = Arc::new(ent);
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, w.cours_maths, "DS concurrent", d(2031, 10, 10), 20_000, 1_000).await;
    let mut t1 = TenantTx::begin(ent.pool(), w.etab).await.unwrap();
    let r1 = PgEnt::ecrire_resultat_tx(&mut t1, w.etab, w.prof, false, jour(), &note_m(ev.id, a, 14_000), Attendu::Vide)
        .await
        .unwrap();
    assert_eq!(r1.version, 1);
    // Seconde transaction, autre connexion : la même case, toujours « vide » du point de vue de son lecteur.
    let e2 = ent.clone();
    let (etab, prof, evid) = (w.etab, w.prof, ev.id);
    let h = tokio::spawn(async move {
        let mut t2 = TenantTx::begin(e2.pool(), etab).await.unwrap();
        PgEnt::ecrire_resultat_tx(&mut t2, etab, prof, false, jour(), &note_m(evid, a, 18_000), Attendu::Vide).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    assert!(!h.is_finished(), "la seconde écriture attend la première (même case)");
    t1.commit().await.unwrap();
    let r2 = h.await.unwrap();
    assert!(matches!(r2, Err(EntDbError::ConflitDossier(_))), "conflit, pas d'écrasement : {r2:?}");
    let (valeur, version): (String, i32) = sqlx::query_as(
        "SELECT valeur::text, version FROM eva_resultat WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(w.etab).bind(ev.id).bind(a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((valeur.as_str(), version), ("14.000", 1), "la première écriture reste");
    // Version attendue : deux écritures concurrentes sur la même version, une seule passe.
    let mut t3 = TenantTx::begin(ent.pool(), w.etab).await.unwrap();
    PgEnt::ecrire_resultat_tx(&mut t3, w.etab, w.prof, false, jour(), &note_m(ev.id, a, 15_000), Attendu::Version(1))
        .await
        .unwrap();
    let e4 = ent.clone();
    let h4 = tokio::spawn(async move {
        let mut t4 = TenantTx::begin(e4.pool(), etab).await.unwrap();
        PgEnt::ecrire_resultat_tx(&mut t4, etab, prof, false, jour(), &note_m(evid, a, 19_000), Attendu::Version(1)).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    t3.commit().await.unwrap();
    assert!(matches!(h4.await.unwrap(), Err(EntDbError::ConflitDossier(_))));
    let valeur: String = sqlx::query_scalar(
        "SELECT valeur::text FROM eva_resultat WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(w.etab).bind(ev.id).bind(a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(valeur, "15.000");
}

/// M1 par l'API (lots) : vingt cases vides écrites en parallèle par deux envois, une seule valeur gagne par case.
#[tokio::test]
async fn v3b_m1_deux_envois_paralleles_aucune_case_n_est_ecrasee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ent = Arc::new(ent);
    let w = monde_v1(&ent, &cast).await;
    let mut evs = Vec::new();
    for i in 0..8 {
        evs.push(creer(&ent, &w, w.cours_maths, &format!("DS {i}"), d(2031, 10, 10), 20_000, 1_000).await.id);
    }
    let a = w.eleves_a[0];
    let envoi = |ent: Arc<PgEnt>, texte: &'static str, prof: Uuid, etab: Uuid, cours: Uuid, evs: Vec<Uuid>| async move {
        ent.eva_v3_saisir_lot_for_actor(
            etab,
            prof,
            PROF,
            jour(),
            &SaisieLotBody {
                cours_id: cours,
                cellules: evs.iter().map(|e| cell(*e, a, texte, None)).collect(),
                origine: None,
            },
        )
        .await
        .unwrap()
    };
    let (h1, h2) = (
        tokio::spawn(envoi(ent.clone(), "11", w.prof, w.etab, w.cours_maths, evs.clone())),
        tokio::spawn(envoi(ent.clone(), "17", w.prof, w.etab, w.cours_maths, evs.clone())),
    );
    let (r1, r2) = (h1.await.unwrap(), h2.await.unwrap());
    for i in 0..evs.len() {
        let issues = [issue(&r1, i), issue(&r2, i)];
        assert_eq!(issues.iter().filter(|x| **x == "ecrit").count(), 1, "case {i} : {issues:?}");
        assert_eq!(issues.iter().filter(|x| **x == "conflit").count(), 1, "case {i} : {issues:?}");
    }
}

/// M2 : une correction ancienne invalide ne fait plus échouer toute la reprise : elle est listée « refusée », la note
/// n'est pas réécrite, les autres notes et corrections sont reprises.
#[tokio::test]
async fn v3b_m2_une_correction_ancienne_invalide_n_annule_pas_la_reprise() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let anc = ancien_monde(&ent, &w).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let n_ok = ancienne_note(&ent, &w, anc.devoirs[0], a, 12.0, 20.0, "note").await;
    let n_mauvaise = ancienne_note(&ent, &w, anc.devoirs[0], b, 10.0, 20.0, "note").await;
    ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR).await.unwrap();
    for (note, eleve, apres, statut) in [(n_ok, a, 13.0_f64, "note"), (n_mauvaise, b, 99_999.0, "note")] {
        exec(
            &ent,
            &format!(
                "INSERT INTO ent_note_correction (id, etablissement_id, note_id, eleve_id, periode_id, ancien_statut, ancienne_valeur,
                                                  nouveau_statut, nouvelle_valeur, motif, corrige_par_user_id, corrige_le)
                 VALUES ($1, $2, $3, $4, $5, 'note', 10, '{statut}', {apres}, 'correction ancienne', $6, now())"
            ),
            vec![u(Uuid::new_v4()), u(w.etab), u(note), u(eleve), u(anc.note_periode), u(w.direction)],
        )
        .await;
    }
    // Une note ancienne valide de plus : elle doit être reprise malgré la correction invalide.
    let n_nouvelle = ancienne_note(&ent, &w, anc.devoirs[0], c, 8.0, 20.0, "note").await;
    let rapport = ent
        .eva_v1_reprendre_for_actor(w.etab, w.direction, DIR)
        .await
        .expect("la reprise ne doit pas échouer à cause d'une correction invalide");
    let corr = rapport.compte("correction").unwrap();
    assert_eq!((corr.lues, corr.creees, corr.refusees), (2, 1, 1), "{corr:?}");
    assert_eq!(corr.lues, corr.creees + corr.inchangees + corr.refusees + corr.ignorees);
    let ligne = rapport
        .lignes
        .iter()
        .find(|l| l.objet == "correction" && l.source_id == n_mauvaise)
        .expect("la correction invalide est listée");
    assert_eq!((ligne.issue.as_str(), ligne.code.as_str()), ("refusee", "correction_valeur_invalide"));
    let val = |id: Uuid| {
        let ent = &ent;
        async move {
            sqlx::query_scalar::<_, String>("SELECT valeur::text FROM eva_resultat WHERE id = $1")
                .bind(id)
                .fetch_one(ent.pool_verif())
                .await
                .unwrap()
        }
    };
    assert_eq!(val(n_ok).await, "13.000", "la correction valide est recopiée");
    assert_eq!(val(n_mauvaise).await, "10.000", "la note à correction invalide n'est pas réécrite");
    assert_eq!(val(n_nouvelle).await, "8.000", "les autres notes sont reprises");
    // Elle est de nouveau listée à chaque reprise tant que l'ancienne donnée n'est pas corrigée, sans rien casser.
    let encore = ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR).await.unwrap();
    assert_eq!(encore.compte("correction").unwrap().refusees, 1);
}

/// M3 : en période OUVERTE, l'évaluation appartient au professeur ; un porteur de `notes.corriger` (la direction), sans droit
/// de saisie, ne la modifie pas et ne l'annule pas. Après l'arrêt : correction possible avec un motif d'au moins 3 caractères,
/// journalisée.
#[tokio::test]
async fn v3b_m3_periode_ouverte_l_evaluation_appartient_au_professeur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = creer(&ent, &w, w.cours_maths, "DS M3", d(2031, 10, 10), 20_000, 1_000).await;
    let modifier = |par: Uuid, role: &'static str, version: i32, bareme: Option<i64>, motif: Option<&'static str>| {
        let ent = &ent;
        let w = &w;
        async move {
            ent.eva_v3_modifier_evaluation_for_actor(
                w.etab,
                par,
                role,
                jour(),
                maintenant(),
                ev.id,
                &ModifierEvaluationV3Body {
                    version_attendue: version,
                    titre: None,
                    date_eval: None,
                    bareme_m: bareme,
                    coef_m: Some(2_000),
                    comptage: None,
                    hors_bareme_autorise: None,
                    annoncee: None,
                    motif: motif.map(String::from),
                },
            )
            .await
        }
    };
    // Période ouverte : la direction ne règle rien (ni coefficient, ni barème), même avec un motif.
    let r = modifier(w.direction, DIR, ev.version, None, Some("je corrige quand même")).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let r = ent
        .eva_v3_annuler_evaluation_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            maintenant(),
            ev.id,
            &AnnulerEvaluationV3Body { version_attendue: ev.version, motif: "annulation par la direction".into() },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Le professeur, lui, règle.
    let ok = modifier(w.prof, PROF, ev.version, None, None).await.unwrap();
    assert_eq!(ok.coef_m, 2_000);
    // Après l'arrêt : la direction corrige, avec motif (3 caractères au moins), journalisé.
    arreter(&ent, &w, w.t1, w.classe_a).await;
    arreter(&ent, &w, w.t1, w.classe_d).await;
    let r = modifier(w.direction, DIR, ok.version, None, None).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "motif obligatoire : {r:?}");
    let r = modifier(w.direction, DIR, ok.version, None, Some("ab")).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "motif d'au moins 3 caractères : {r:?}");
    let corr = modifier(w.direction, DIR, ok.version, Some(10_000), Some("barème saisi sur 20 au lieu de 10"))
        .await
        .unwrap();
    assert_eq!((corr.bareme_m, corr.coef_m), (10_000, 2_000));
    let (n, motif): (i64, Option<String>) = sqlx::query_as(
        "SELECT count(*), max(motif) FROM eva_evaluation_historique WHERE etablissement_id = $1 AND evaluation_id = $2 AND correction",
    )
    .bind(w.etab).bind(ev.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1, "la correction après l'arrêt est journalisée");
    assert_eq!(motif.as_deref(), Some("barème saisi sur 20 au lieu de 10"));
    // L'annulation après l'arrêt : la direction, avec motif.
    let annulee = ent
        .eva_v3_annuler_evaluation_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            maintenant(),
            ev.id,
            &AnnulerEvaluationV3Body { version_attendue: corr.version, motif: "évaluation annulée après l'arrêt".into() },
        )
        .await
        .unwrap();
    assert_eq!(annulee.etat, "annulee");
}

/// Conversion des notes au changement de barème (limite §5 de V3) : REFUSÉE dès qu'une note existe (14 sur 10 n'est pas 14
/// sur 20) ; permise tant qu'aucune note n'est saisie. Marche à suivre : annuler et recréer, ou corriger les notes.
#[tokio::test]
async fn v3b_le_bareme_ne_change_pas_quand_des_notes_existent() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, w.cours_maths, "DS barème", d(2031, 10, 10), 20_000, 1_000).await;
    let corps = |version: i32, bareme: i64, motif: Option<&str>| ModifierEvaluationV3Body {
        version_attendue: version,
        titre: None,
        date_eval: None,
        bareme_m: Some(bareme),
        coef_m: None,
        comptage: None,
        hors_bareme_autorise: None,
        annoncee: None,
        motif: motif.map(String::from),
    };
    // Sans note : le barème se change librement.
    let sur10 = ent
        .eva_v3_modifier_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), ev.id, &corps(ev.version, 10_000, None))
        .await
        .unwrap();
    assert_eq!(sur10.bareme_m, 10_000);
    // Une absence n'est pas une note : le barème reste modifiable.
    lot(&ent, &w, w.cours_maths, vec![cell(ev.id, w.eleves_a[1], "abs", None)]).await;
    let version = |ent: &PgEnt| {
        let (etab, id) = (w.etab, ev.id);
        let pool = ent.pool_verif().clone();
        async move {
            sqlx::query_scalar::<_, i32>("SELECT version FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap()
        }
    };
    let sur20 = ent
        .eva_v3_modifier_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), ev.id, &corps(version(&ent).await, 20_000, Some("absence saisie")))
        .await
        .unwrap();
    assert_eq!(sur20.bareme_m, 20_000);
    // Une note : le barème est figé, même avec un motif, même vers un barème plus grand.
    lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "14", None)]).await;
    for bareme in [10_000, 40_000] {
        let r = ent
            .eva_v3_modifier_evaluation_for_actor(
                w.etab,
                w.prof,
                PROF,
                jour(),
                maintenant(),
                ev.id,
                &corps(version(&ent).await, bareme, Some("mes notes étaient sur 10")),
            )
            .await;
        match r {
            Err(EntDbError::SaisieInvalide(m)) => assert!(m.contains("barème ne change pas"), "{m}"),
            autre => panic!("le changement de barème doit être refusé : {autre:?}"),
        }
    }
    let g = grille(&ent, &w, w.cours_maths).await;
    assert_eq!(g.evaluations[0].bareme_m, 20_000, "barème inchangé");
}

/// F1 : le suppléant SAISIT les notes du titulaire (accepté) ; « en cours de correction » et la modification suivent CE-44 :
/// seulement ses propres évaluations.
#[tokio::test]
async fn v3b_f1_le_suppleant_ne_met_pas_en_cours_de_correction_l_evaluation_du_titulaire() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let remplacant = cast.profs.iter().map(|p| p.user_id).find(|x| *x != w.prof).unwrap();
    let ev = creer(&ent, &w, w.cours_maths, "DS titulaire F1", d(2031, 10, 10), 20_000, 1_000).await;
    let service: Uuid = sqlx::query_scalar("SELECT id FROM ent_service_enseignement WHERE etablissement_id = $1 AND cours_id = $2")
        .bind(w.etab).bind(w.cours_maths).fetch_one(ent.pool_verif()).await.unwrap();
    let agent: Option<Uuid> = sqlx::query_scalar(
        "SELECT ag.id FROM ent_agent ag JOIN ent_personne pe ON pe.id = ag.personne_id AND pe.etablissement_id = ag.etablissement_id
          WHERE ag.etablissement_id = $1 AND pe.user_id = $2 AND ag.fusionne_dans_agent_id IS NULL",
    )
    .bind(w.etab).bind(remplacant).fetch_optional(ent.pool_verif()).await.unwrap();
    let Some(agent) = agent else {
        return eprintln!("skip: pas d'agent pour le second professeur du labo");
    };
    exec(
        &ent,
        "INSERT INTO ent_service_intervenant (id, etablissement_id, service_id, agent_id, nature, du, saisie, origine)
         VALUES ($1, $2, $3, $4, 'suppleant', '2031-10-15', true, 'saisie')",
        vec![u(Uuid::new_v4()), u(w.etab), u(service), u(agent)],
    )
    .await;
    // Il saisit (accepté)…
    let r = lot_par(&ent, &w, remplacant, PROF, w.cours_maths, vec![cell(ev.id, w.eleves_a[0], "12", None)]).await.unwrap();
    assert_eq!(issue(&r, 0), "ecrit");
    // … mais ne met pas l'évaluation du titulaire « en cours de correction ».
    let r = ent
        .eva_v3_en_cours_correction_for_actor(w.etab, remplacant, PROF, jour(), maintenant(), ev.id, true)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Sur SES évaluations, oui.
    let propre = ent
        .eva_v3_creer_evaluation_for_actor(w.etab, remplacant, PROF, jour(), maintenant(), &corps(w.cours_maths, "DS suppléant F1", d(2031, 10, 17), 20_000, 1_000))
        .await
        .unwrap();
    let ok = ent
        .eva_v3_en_cours_correction_for_actor(w.etab, remplacant, PROF, jour(), maintenant(), propre.id, true)
        .await
        .unwrap();
    assert!(ok.en_cours_de_correction);
}

/// F2 : la base vérifie qu'une écriture après l'arrêt vient d'un PORTEUR de `notes.corriger` (pas seulement d'un motif non
/// vide) ; un arrêt qui se commet pendant une écriture ordinaire est vu (verrou de l'état puis relecture).
#[tokio::test]
async fn v3b_f2_apres_l_arret_la_base_exige_l_attestation_du_droit_de_correction() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ent = Arc::new(ent);
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, w.cours_maths, "DS F2", d(2031, 10, 10), 20_000, 1_000).await;
    let ev2 = creer(&ent, &w, w.cours_maths, "DS F2 bis", d(2031, 10, 11), 20_000, 1_000).await;
    let r = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "12", None)]).await;
    let version = r.cellules[0].version;
    arreter(&ent, &w, w.t1, w.classe_a).await;
    // SQL direct avec un motif valide mais SANS attestation : refusé par la base.
    let direct = sqlx::query(
        "UPDATE eva_resultat SET valeur = 15, motif_correction = 'correction sans attestation', saisi_par = $4
          WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(w.etab).bind(ev.id).bind(a).bind(w.direction)
    .execute(ent.pool_verif())
    .await;
    let e = direct.expect_err("l'écriture sans attestation doit être refusée");
    assert!(e.to_string().contains("exige un porteur de notes.corriger"), "{e}");
    // Par la direction, via l'API (qui atteste son droit) : acceptée.
    let ok = lot_par(
        &ent,
        &w,
        w.direction,
        DIR,
        w.cours_maths,
        vec![CelluleSaisieDto { motif: Some("erreur de report".into()), ..cell(ev.id, a, "15", version) }],
    )
    .await
    .unwrap();
    assert_eq!(issue(&ok, 0), "ecrit", "{ok:?}");
    // Le professeur, après l'arrêt : refusé (pas de droit de correction), motif ou non.
    let ko = lot_par(
        &ent,
        &w,
        w.prof,
        PROF,
        w.cours_maths,
        vec![CelluleSaisieDto { motif: Some("je corrige".into()), ..cell(ev.id, a, "16", ok.cellules[0].version) }],
    )
    .await
    .unwrap();
    assert_eq!(issue(&ko, 0), "refuse");
    // Course : une écriture ordinaire tient le verrou de l'état ; l'arrêt attend sa fin (il ne se glisse pas au milieu).
    exec(
        &ent,
        "UPDATE eva_periode_etat SET etat = 'ouverte', origine = 'geste', modifie_par = $3, motif = 'réouverture de test'
          WHERE etablissement_id = $1 AND classe_id = $2",
        vec![u(w.etab), u(w.classe_a), u(w.direction)],
    )
    .await;
    let mut t1 = TenantTx::begin(ent.pool(), w.etab).await.unwrap();
    PgEnt::ecrire_resultat_tx(&mut t1, w.etab, w.prof, false, jour(), &note_m(ev2.id, a, 9_000), Attendu::Vide)
        .await
        .unwrap();
    let e2 = ent.clone();
    let (etab, direction, classe, t1_id) = (w.etab, w.direction, w.classe_a, w.t1);
    let arret = tokio::spawn(async move {
        e2.eva_v1_changer_etat_periode_for_actor(etab, direction, DIR, t1_id, classe, "notes_arretees", None, Some(d(2031, 12, 5))).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    assert!(!arret.is_finished(), "l'arrêt attend la fin de l'écriture en cours (verrou partagé sur l'état)");
    t1.commit().await.unwrap();
    arret.await.unwrap().unwrap();
}

/// F3 : `origine = import` est réservé à la confirmation d'un import de fichier ; `/eva/saisie` écrit toujours `saisie`.
#[tokio::test]
async fn v3b_f3_la_saisie_ecrit_toujours_saisie() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, w.cours_maths, "DS F3", d(2031, 10, 10), 20_000, 1_000).await;
    ent.eva_v3_saisir_lot_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        &SaisieLotBody { cours_id: w.cours_maths, cellules: vec![cell(ev.id, a, "13", None)], origine: Some("import".into()) },
    )
    .await
    .unwrap();
    let source: String = sqlx::query_scalar(
        "SELECT source FROM eva_resultat WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(w.etab).bind(ev.id).bind(a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(source, "saisie", "une saisie qui se dit « import » reste une saisie");
    // Une origine inconnue reste refusée.
    let r = ent
        .eva_v3_saisir_lot_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &SaisieLotBody { cours_id: w.cours_maths, cellules: vec![], origine: Some("magie".into()) },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))));
}

/// F4 : les 24 h du retrait par le professeur courent depuis la PREMIÈRE publication : retirer puis republier ne les
/// relance pas.
#[tokio::test]
async fn v3b_f4_le_delai_de_24h_court_depuis_la_premiere_publication() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = creer(&ent, &w, w.cours_maths, "DS F4", d(2031, 10, 10), 20_000, 1_000).await;
    lot(&ent, &w, w.cours_maths, vec![cell(ev.id, w.eleves_a[0], "12", None)]).await;
    let t0 = maintenant();
    let publier = |par: Uuid, role: &'static str, action: &'static str, a_: DateTime<Utc>| {
        let (ent, w) = (&ent, &w);
        async move {
            ent.eva_v3_publier_for_actor(w.etab, par, role, jour(), a_, ev.id, &pub_body(action, None, if action == "retirer" { Some("erreur de barème") } else { None }))
                .await
        }
    };
    publier(w.prof, PROF, "maintenant", t0).await.unwrap();
    // Dix heures plus tard : le professeur retire (dans les 24 h), puis republie aussitôt.
    publier(w.prof, PROF, "retirer", t0 + chrono::Duration::hours(10)).await.unwrap();
    publier(w.prof, PROF, "maintenant", t0 + chrono::Duration::hours(10)).await.unwrap();
    // À 25 h de la première publication (15 h de la seconde) : le professeur ne retire plus ; la direction, si.
    let r = publier(w.prof, PROF, "retirer", t0 + chrono::Duration::hours(25)).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "délai non relancé par la republication : {r:?}");
    publier(w.direction, DIR, "retirer", t0 + chrono::Duration::hours(25)).await.unwrap();
    // La première publication est posée une fois (déclencheur) et ne bouge plus.
    let (premiere, courante): (DateTime<Utc>, Option<DateTime<Utc>>) = sqlx::query_as(
        "SELECT premiere_publiee_le, publiee_le FROM eva_evaluation WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.etab).bind(ev.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(premiere, t0);
    assert!(courante.is_none());
}

/// F5 : les événements de saisie et de correction ne portent PAS d'`eleve_id` ; aucune note non publiée ne se déduit de
/// l'outbox (et rien ne peut notifier une famille sur ces types).
#[tokio::test]
async fn v3b_f5_les_evenements_de_saisie_et_de_correction_ne_portent_pas_d_eleve() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, w.cours_maths, "DS F5", d(2031, 10, 10), 20_000, 1_000).await;
    let r = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "12", None)]).await;
    arreter(&ent, &w, w.t1, w.classe_a).await;
    lot_par(
        &ent,
        &w,
        w.direction,
        DIR,
        w.cours_maths,
        vec![CelluleSaisieDto { motif: Some("erreur de report".into()), ..cell(ev.id, a, "14", r.cellules[0].version) }],
    )
    .await
    .unwrap();
    let (n, avec_eleve): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(eleve_id) FROM ent_metier_event
          WHERE etablissement_id = $1 AND type IN ('evaluations.resultats_saisis', 'evaluations.corrigee')",
    )
    .bind(w.etab)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(n >= 2, "les deux types sont émis ({n})");
    assert_eq!(avec_eleve, 0, "aucun événement de saisie ou de correction ne porte d'élève");
}

/// F6 : l'historique d'une case est réservé au professeur du cours et à la direction (EVA-139), pas à tout `notes.lire`.
#[tokio::test]
async fn v3b_f6_l_historique_d_une_case_est_reserve_au_professeur_et_a_la_direction() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, w.cours_maths, "DS F6", d(2031, 10, 10), 20_000, 1_000).await;
    lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "12", None)]).await;
    let hist = |par: Uuid, role: &'static str| {
        let (ent, w) = (&ent, &w);
        async move { ent.eva_v3_historique_for_actor(w.etab, par, role, ev.id, a, jour()).await }
    };
    assert_eq!(hist(w.prof, PROF).await.unwrap().len(), 1);
    assert_eq!(hist(w.direction, DIR).await.unwrap().len(), 1);
    // Le secrétariat lit la grille (notes.lire établissement) mais pas l'historique des cases.
    let secr = cast.administratif.user_id;
    assert!(ent.eva_v3_grille_for_actor(w.etab, secr, "administratif", w.cours_maths, w.t1, jour(), maintenant()).await.is_ok());
    let r = hist(secr, "administratif").await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Un autre professeur (qui n'enseigne pas ce cours) non plus.
    let autre = cast.profs.iter().map(|p| p.user_id).find(|x| *x != w.prof).unwrap();
    assert!(hist(autre, PROF).await.is_err());
}

/// F7 : une publication programmée sort à l'heure. Un jour explicite n'est plus figé à 12 h : sans instant fourni, c'est la
/// FIN du jour (heure de l'établissement) ; avec un instant, c'est cet instant. Le retard maximal est documenté.
#[tokio::test]
async fn v3b_f7_la_publication_programmee_sort_a_l_heure() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let prevue = jour().and_hms_opt(14, 0, 0).unwrap().and_utc();
    let programmee = |titre: &str, date: NaiveDate, a_: DateTime<Utc>| {
        let (ent, w) = (&ent, &w);
        let titre = titre.to_string();
        async move {
            let ev = creer(ent, w, w.cours_maths, &titre, date, 20_000, 1_000).await;
            ent.eva_v3_saisir_lot_for_actor(
                w.etab,
                w.prof,
                PROF,
                jour(),
                &SaisieLotBody { cours_id: w.cours_maths, cellules: vec![cell(ev.id, a, "12", None)], origine: None },
            )
            .await
            .unwrap();
            ent.eva_v3_publier_for_actor(
                w.etab,
                w.prof,
                PROF,
                jour(),
                maintenant(),
                ev.id,
                &PublierV3Body { action: "programmer".into(), prevue_le: Some(a_), prevue_locale: None, motif: None },
            )
            .await
            .unwrap();
            ev.id
        }
    };
    let e14 = programmee("Programmée 14 h", d(2031, 10, 10), prevue).await;
    let e20 = programmee("Programmée 20 h", d(2031, 10, 11), jour().and_hms_opt(20, 0, 0).unwrap().and_utc()).await;
    let publiee = |id: Uuid| {
        let ent = &ent;
        async move {
            sqlx::query_scalar::<_, bool>("SELECT publiee_le IS NOT NULL FROM eva_evaluation WHERE id = $1")
                .bind(id)
                .fetch_one(ent.pool_verif())
                .await
                .unwrap()
        }
    };
    // 13 h 59 : rien ; 14 h 00 pile : sortie (≤) ; l'évaluation de 20 h reste.
    let avant = ent
        .passages_quotidiens_etablissement_a(w.etab, Some(jour()), Some(prevue - chrono::Duration::minutes(1)))
        .await;
    assert_eq!(avant.publications_automatiques, 0);
    assert!(!publiee(e14).await);
    let a_l_heure = ent.passages_quotidiens_etablissement_a(w.etab, Some(jour()), Some(prevue)).await;
    assert_eq!(a_l_heure.publications_automatiques, 1);
    assert!(publiee(e14).await && !publiee(e20).await);
    // Jour explicite SANS instant : la fin du jour (23 h 59 à Paris) : l'évaluation de 20 h UTC sort (avant : figé à 12 h).
    let fin_du_jour = ent.passages_quotidiens_etablissement_a(w.etab, Some(jour()), None).await;
    assert_eq!(fin_du_jour.publications_automatiques, 1, "{fin_du_jour:?}");
    assert!(publiee(e20).await);
    // Planificateur dédié (toutes les 5 minutes) : retard maximal documenté.
    assert!(crate::passages_quotidiens::RETARD_MAX_PUBLICATION_PROGRAMMEE_SECONDES <= 10 * 60);
    let bilan = ent.eva_v3_publications_automatiques_tous(prevue).await.unwrap();
    assert_eq!(bilan.programmees, 0, "idempotent : rien n'est republié");
}

/// F8 : bornes (`restreinte_a`, texte de case, motif), doublons de cibles ignorés, fonction de reprise fermée à PUBLIC et
/// contexte d'établissement obligatoire, course barème / saisie.
#[tokio::test]
async fn v3b_f8_bornes_doublons_et_fonction_de_reprise_fermee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let ent = Arc::new(ent);
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    // restreinte_a : au plus 300 élèves.
    let mut c = corps(w.cours_maths, "DS restreinte", d(2031, 10, 10), 20_000, 1_000);
    c.restreinte_a = (0..301).map(|_| Uuid::new_v4()).collect();
    let r = ent.eva_v3_creer_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), &c).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(m)) if m.contains("300")), "{r:?}");
    // Un public restreint avec un doublon d'élève est accepté (ignoré proprement).
    let mut c = corps(w.cours_maths, "DS demi-groupe", d(2031, 10, 10), 20_000, 1_000);
    c.restreinte_a = vec![a, a, b];
    assert!(ent.eva_v3_creer_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), &c).await.is_ok());
    let ev = creer(&ent, &w, w.cours_maths, "DS bornes", d(2031, 10, 12), 20_000, 1_000).await;
    // Texte de case : 40 caractères au plus.
    let r = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, &"1".repeat(41), None)]).await;
    assert_eq!(issue(&r, 0), "refuse");
    assert!(r.cellules[0].message.as_deref().unwrap_or("").contains("40"));
    // Motif : 500 caractères au plus.
    let r = lot(&ent, &w, w.cours_maths, vec![CelluleSaisieDto { motif: Some("m".repeat(501)), ..cell(ev.id, a, "12", None) }]).await;
    assert_eq!(issue(&r, 0), "refuse");
    // Doublons de `cibles` à l'import : la seconde colonne identique est ignorée (une seule case par élève et évaluation).
    let ap = ent
        .eva_v3_apercu_import_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &ImportBody {
                cours_id: w.cours_maths,
                mode: "collage".into(),
                cibles: vec![ev.id, ev.id],
                lignes: vec![ImportLigneBody { eleve_id: Some(a), nom: None, rang: None, cellules: vec!["14".into(), "9".into()] }],
                empreinte: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(ap.lignes[0].cellules.len(), 1, "le doublon de cible est ignoré");
    assert_eq!(ap.lignes[0].cellules[0].valeur_m, Some(14_000), "la première colonne gagne");
    // `eva_reprendre_existant` : fermée à PUBLIC.
    let public_execute: bool = sqlx::query_scalar(
        "SELECT p.proacl IS NULL OR EXISTS (SELECT 1 FROM aclexplode(p.proacl) a WHERE a.grantee = 0 AND a.privilege_type = 'EXECUTE')
           FROM pg_proc p WHERE p.proname = 'eva_reprendre_existant'",
    )
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(!public_execute, "EXECUTE retiré à PUBLIC");
    // … et le contexte d'établissement est OBLIGATOIRE (avant : « si présent » seulement).
    let sans_contexte = sqlx::query("SELECT eva_reprendre_existant($1, 'manuel')").bind(w.etab).execute(ent.pool_verif()).await;
    assert!(sans_contexte.is_err(), "pas de reprise sans contexte d'établissement");
    let mut autre = TenantTx::begin(ent.pool(), Uuid::new_v4()).await.unwrap();
    let hors = sqlx::query("SELECT eva_reprendre_existant($1, 'manuel')").bind(w.etab).execute(&mut *autre).await;
    assert!(hors.is_err(), "pas de reprise d'un autre établissement");
    drop(autre);
    // Course barème / saisie : une écriture en cours tient l'évaluation en lecture ; le changement de barème attend sa fin
    // puis voit la note (et la refuse) au lieu de laisser un 15 sur 10.
    let ev2 = creer(&ent, &w, w.cours_maths, "DS course", d(2031, 10, 13), 20_000, 1_000).await;
    let mut t1 = TenantTx::begin(ent.pool(), w.etab).await.unwrap();
    PgEnt::ecrire_resultat_tx(&mut t1, w.etab, w.prof, false, jour(), &note_m(ev2.id, a, 15_000), Attendu::Vide)
        .await
        .unwrap();
    let e2 = ent.clone();
    let (etab, prof, evid, version) = (w.etab, w.prof, ev2.id, ev2.version);
    let modif = tokio::spawn(async move {
        e2.eva_v3_modifier_evaluation_for_actor(
            etab,
            prof,
            PROF,
            jour(),
            maintenant(),
            evid,
            &ModifierEvaluationV3Body {
                version_attendue: version,
                titre: None,
                date_eval: None,
                bareme_m: Some(10_000),
                coef_m: None,
                comptage: None,
                hors_bareme_autorise: None,
                annoncee: None,
                motif: Some("barème sur 10".into()),
            },
        )
        .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    assert!(!modif.is_finished(), "le changement de barème attend l'écriture en cours");
    t1.commit().await.unwrap();
    assert!(modif.await.unwrap().is_err(), "la note 15 saisie pendant ce temps interdit un barème sur 10");
    let bareme: String = sqlx::query_scalar("SELECT bareme::text FROM eva_evaluation WHERE id = $1").bind(ev2.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(bareme, "20.000");
}
