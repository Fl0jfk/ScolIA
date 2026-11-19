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

fn corps(cours: Uuid, titre: &str, date: NaiveDate, bareme_m: i64, coef_m: i64) -> CreerEvaluationV3Body {
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

async fn creer(ent: &PgEnt, w: &W, cours: Uuid, titre: &str, date: NaiveDate, bareme_m: i64, coef_m: i64) -> EvaluationColonneDto {
    ent.eva_v3_creer_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), &corps(cours, titre, date, bareme_m, coef_m))
        .await
        .unwrap()
}

fn cell(ev: Uuid, eleve: Uuid, texte: &str, version: Option<i32>) -> CelluleSaisieDto {
    CelluleSaisieDto { evaluation_id: ev, eleve_id: eleve, texte: texte.into(), version_attendue: version, motif: None }
}

async fn lot_par(ent: &PgEnt, w: &W, par: Uuid, role: &str, cours: Uuid, cellules: Vec<CelluleSaisieDto>) -> Result<SaisieLotRetourDto, EntDbError> {
    ent.eva_v3_saisir_lot_for_actor(w.etab, par, role, jour(), &SaisieLotBody { cours_id: cours, cellules, origine: None })
        .await
}

async fn lot(ent: &PgEnt, w: &W, cours: Uuid, cellules: Vec<CelluleSaisieDto>) -> SaisieLotRetourDto {
    lot_par(ent, w, w.prof, PROF, cours, cellules).await.unwrap()
}

async fn grille(ent: &PgEnt, w: &W, cours: Uuid) -> GrilleV3Dto {
    ent.eva_v3_grille_for_actor(w.etab, w.prof, PROF, cours, w.t1, jour(), maintenant()).await.unwrap()
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
    let e1 = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 10), 20_000, 1_000).await;
    let e2 = creer(&ent, &w, w.cours_maths, "Interro", d(2031, 10, 17), 10_000, 2_000).await;
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
    assert_eq!((cl.effectif, cl.min_m, cl.max_m, cl.mediane_m), (3, 10_000, 14_000, 12_500));
    assert_eq!(cl.moyenne_m, 12_170, "moyenne des moyennes, arrondie au centième par le moteur");
    // Statistiques de l'évaluation : le moteur, sur les notes ramenées.
    let s1 = g.evaluations[0].stats.as_ref().expect("stats DS 1");
    assert_eq!((s1.effectif, s1.min_m, s1.max_m), (2, 12_500, 14_000));
    // Explication ligne à ligne.
    let ex = ent
        .eva_v3_explication_for_actor(w.etab, w.prof, PROF, w.cours_maths, w.t1, c, jour())
        .await
        .unwrap();
    assert_eq!(ex.moyenne_m, Some(10_000));
    assert!(ex.lignes.iter().any(|l| l.to_lowercase().contains("abs")), "{:?}", ex.lignes);
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
    let ens = ent.eva_v3_enseignements_for_actor(w.etab, w.prof, PROF, jour()).await.unwrap();
    let atelier = ens.iter().find(|e| e.cours_id == cours).expect("le cours sans matière est listé");
    assert!(atelier.sans_matiere && atelier.droit == "saisie" && atelier.libelle.contains("Atelier"));
    let ev = creer(&ent, &w, cours, "Scène 1", d(2031, 10, 15), 20_000, 1_000).await;
    let r = lot(&ent, &w, cours, vec![cell(ev.id, w.eleves_a[0], "16", None)]).await;
    assert_eq!(issue(&r, 0), "ecrit");
    let g = grille(&ent, &w, cours).await;
    assert!(g.enseignement.sans_matiere);
    assert_eq!(g.eleves.iter().find(|e| e.eleve_id == w.eleves_a[0]).unwrap().moyenne_m, Some(16_000));
}

#[tokio::test]
async fn v3_les_periodes_du_cours_sont_proposees() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ps = ent.eva_v3_periodes_for_actor(w.etab, w.prof, PROF, w.cours_maths, jour()).await.unwrap();
    assert!(ps.iter().any(|p| p.id == w.t1) && ps.iter().any(|p| p.id == w.t2), "{ps:?}");
}

// ───────────────────────── Droits ─────────────────────────

#[tokio::test]
async fn v3_droits_professeur_direction_secretariat_famille() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 10), 20_000, 1_000).await;
    lot(&ent, &w, w.cours_maths, vec![cell(ev.id, w.eleves_a[0], "11", None)]).await;
    // Direction et secrétariat : lecture seule.
    for (u_id, role) in [(w.direction, DIR), (cast.administratif.user_id, "administratif")] {
        let g = ent.eva_v3_grille_for_actor(w.etab, u_id, role, w.cours_maths, w.t1, jour(), maintenant()).await.unwrap();
        assert!(!g.peut_saisir, "{role} : lecture seule");
        assert_eq!(g.eleves.len(), 3);
        let r = lot_par(&ent, &w, u_id, role, w.cours_maths, vec![cell(ev.id, w.eleves_a[1], "9", None)]).await;
        let refuse = match r {
            Err(EntDbError::Forbidden) => true,
            Ok(r) => issue(&r, 0) == "refuse",
            Err(_) => false,
        };
        assert!(refuse, "{role} ne saisit pas pendant que la période est ouverte");
    }
    // Un autre professeur : ni lecture ni saisie sur ce cours.
    let autre = cast.profs.iter().map(|p| p.user_id).find(|x| *x != w.prof).unwrap();
    let r = ent.eva_v3_grille_for_actor(w.etab, autre, PROF, w.cours_maths, w.t1, jour(), maintenant()).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    assert!(matches!(
        lot_par(&ent, &w, autre, PROF, w.cours_maths, vec![cell(ev.id, w.eleves_a[1], "9", None)]).await,
        Err(EntDbError::Forbidden)
    ));
    // Famille et élève : jamais la grille.
    for role in ["parent", "eleve"] {
        let r = ent.eva_v3_grille_for_actor(w.etab, cast.parent_lea.user_id, role, w.cours_maths, w.t1, jour(), maintenant()).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
    // Un cours d'un autre établissement n'existe pas.
    let r = ent.eva_v3_grille_for_actor(Uuid::new_v4(), w.prof, PROF, w.cours_maths, w.t1, jour(), maintenant()).await;
    assert!(r.is_err());
}

#[tokio::test]
async fn v3_apres_l_arret_seule_la_direction_corrige_avec_un_motif_et_l_historique_le_montre() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 10), 20_000, 1_000).await;
    let r = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "11", None)]).await;
    let v1 = r.cellules[0].version.unwrap();
    arreter(&ent, &w, w.t1, w.classe_a).await;
    // Le professeur ne saisit plus.
    let r = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "15", Some(v1))]).await;
    assert_eq!(issue(&r, 0), "refuse", "{r:?}");
    // La direction corrige, avec un motif.
    let sans_motif = lot_par(&ent, &w, w.direction, DIR, w.cours_maths, vec![cell(ev.id, a, "15", Some(v1))]).await.unwrap();
    assert_eq!(issue(&sans_motif, 0), "refuse", "sans motif : {sans_motif:?}");
    let mut avec = cell(ev.id, a, "15", Some(v1));
    avec.motif = Some("erreur de report de la copie".into());
    let r = lot_par(&ent, &w, w.direction, DIR, w.cours_maths, vec![avec]).await.unwrap();
    assert_eq!(issue(&r, 0), "ecrit", "{r:?}");
    let h = ent.eva_v3_historique_for_actor(w.etab, w.prof, PROF, ev.id, a, jour()).await.unwrap();
    assert_eq!(h.len(), 2);
    assert!(!h[0].correction && h[1].correction);
    assert_eq!((h[1].valeur_avant_m, h[1].valeur_apres_m), (Some(11_000), Some(15_000)));
    assert_eq!(h[1].motif.as_deref(), Some("erreur de report de la copie"));
    assert!(h[1].auteur_nom.is_some(), "qui a corrigé est affiché");
    // La grille signale la case corrigée.
    let g = grille(&ent, &w, w.cours_maths).await;
    let cel = &g.eleves.iter().find(|e| e.eleve_id == a).unwrap().cellules[0];
    assert!(cel.corrige && !cel.saisie_ouverte);
    // Un parent ne lit pas l'historique.
    assert!(ent.eva_v3_historique_for_actor(w.etab, cast.parent_lea.user_id, "parent", ev.id, a, jour()).await.is_err());
}

// ───────────────────────── Conflits de version ─────────────────────────

#[tokio::test]
async fn v3_deux_onglets_jamais_d_ecrasement_silencieux() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 10), 20_000, 1_000).await;
    // L'onglet 1 écrit 12 (case vide) ; l'onglet 2 croyait aussi la case vide : conflit.
    let r1 = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "12", None)]).await;
    assert_eq!(issue(&r1, 0), "ecrit");
    let v1 = r1.cellules[0].version.unwrap();
    let r2 = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "9", None)]).await;
    assert_eq!(issue(&r2, 0), "conflit");
    let c = r2.cellules[0].conflit.as_ref().expect("état actuel de la case");
    assert_eq!((c.statut.as_str(), c.valeur_m, c.version), ("note", Some(12_000), v1));
    assert!(c.par_nom.is_some());
    // L'onglet 1 corrige (version lue) → 13 ; l'onglet 2 renvoie avec la version périmée : conflit encore.
    let r3 = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "13", Some(v1))]).await;
    assert_eq!(issue(&r3, 0), "ecrit");
    let v2 = r3.cellules[0].version.unwrap();
    assert!(v2 > v1);
    let r4 = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "9", Some(v1))]).await;
    assert_eq!(issue(&r4, 0), "conflit");
    // Résolution explicite : « garder la mienne » = renvoyer avec la version actuelle.
    let r5 = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "9", Some(r4.cellules[0].conflit.as_ref().unwrap().version))]).await;
    assert_eq!(issue(&r5, 0), "ecrit");
    let g = grille(&ent, &w, w.cours_maths).await;
    assert_eq!(g.eleves.iter().find(|e| e.eleve_id == a).unwrap().cellules[0].valeur_m, Some(9_000));
    // Une même valeur à la même version ne change rien (ni version ni journal).
    let v5 = r5.cellules[0].version.unwrap();
    let r6 = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "9", Some(v5))]).await;
    assert_eq!(issue(&r6, 0), "inchange");
    assert_eq!(r6.cellules[0].version, Some(v5));
    let h = ent.eva_v3_historique_for_actor(w.etab, w.prof, PROF, ev.id, a, jour()).await.unwrap();
    assert_eq!(h.len(), 3, "12, 13, 9 : une ligne par changement réel");
}

#[tokio::test]
async fn v3_chaque_case_est_independante_et_les_erreurs_sont_dites() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let ev = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 10), 20_000, 1_000).await;
    let r = lot(
        &ent,
        &w,
        w.cours_maths,
        vec![
            cell(ev.id, a, "21", None),     // au-dessus du barème
            cell(ev.id, b, "douze", None),  // illisible
            cell(ev.id, c, "disp", None),   // code rapide
            cell(ev.id, a, "", None),       // vide : rien
            cell(ev.id, Uuid::new_v4(), "10", None), // élève inconnu
            cell(Uuid::new_v4(), a, "10", None),     // évaluation inconnue
        ],
    )
    .await;
    let v: Vec<&str> = r.cellules.iter().map(|x| x.issue.as_str()).collect();
    assert_eq!(v, ["refuse", "refuse", "ecrit", "vide", "refuse", "refuse"], "{r:?}");
    assert!(r.cellules[0].message.as_deref().unwrap().contains("barème"));
    assert_eq!(r.cellules[2].statut.as_deref(), Some("dispense"));
    // Un élève ne figurant pas dans le public à la date n'est jamais saisi (non concerné).
    let hors = creer_eleve(&ent, w.etab, "Hors-public").await;
    let r = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, hors, "10", None)]).await;
    assert_eq!(issue(&r, 0), "refuse");
    // Trop de cases : refusé en bloc.
    let trop: Vec<CelluleSaisieDto> = (0..2001).map(|_| cell(ev.id, a, "1", None)).collect();
    assert!(lot_par(&ent, &w, w.prof, PROF, w.cours_maths, trop).await.is_err());
}

#[tokio::test]
async fn v3_un_eleve_protege_est_introuvable_pour_le_secretariat_mais_pas_pour_le_professeur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 10), 20_000, 1_000).await;
    lot(&ent, &w, w.cours_maths, vec![cell(ev.id, w.eleves_a[0], "11", None)]).await;
    // Le professeur du cours voit toujours ses élèves ; le secrétariat voit la même grille que sans protection
    // (aucune protection posée ici) : le test garantit au moins que la garde ne retire personne à tort.
    let g = ent
        .eva_v3_grille_for_actor(w.etab, cast.administratif.user_id, "administratif", w.cours_maths, w.t1, jour(), maintenant())
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
    crate::tests_evaluations_v1::affecter(&ent, w.etab, w.annee, arrive, w.classe_a, d(2031, 10, 15)).await;
    let part = creer_eleve(&ent, w.etab, "Part").await;
    crate::tests_evaluations_v1::affecter(&ent, w.etab, w.annee, part, w.classe_a, d(2031, 9, 1)).await;
    exec(
        &ent,
        "UPDATE ent_affectation SET date_fin = '2031-10-12' WHERE etablissement_id = $1 AND eleve_id = $2",
        vec![u(w.etab), u(part)],
    )
    .await;
    let e1 = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 10), 20_000, 1_000).await;
    let e2 = creer(&ent, &w, w.cours_maths, "DS 2", d(2031, 10, 18), 20_000, 1_000).await;
    let r = lot(
        &ent,
        &w,
        w.cours_maths,
        vec![cell(e1.id, part, "13", None), cell(e2.id, arrive, "8", None), cell(e1.id, arrive, "9", None)],
    )
    .await;
    assert_eq!(issue(&r, 0), "ecrit");
    assert_eq!(issue(&r, 1), "ecrit");
    assert_eq!(issue(&r, 2), "refuse", "l'arrivant n'était pas dans le public le 10 octobre : non concerné");
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
    let ev = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 10), 20_000, 1_000).await;
    // La période de la classe A est arrêtée : la création d'une nouvelle évaluation pour cette classe est refusée.
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let r = ent
        .eva_v3_creer_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), &corps(w.cours_maths, "DS 2", d(2031, 10, 18), 20_000, 1_000))
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
            .eva_v3_creer_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), &corps(w.cours_maths, titre, date, bar, coef))
            .await;
        assert!(r.is_err(), "{titre} {date} {bar} {coef}");
    }
    // Seul le professeur du cours crée.
    let r = ent
        .eva_v3_creer_evaluation_for_actor(w.etab, w.direction, DIR, jour(), maintenant(), &corps(w.cours_maths, "X", d(2031, 10, 10), 20_000, 1_000))
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)));
    let ev = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 10), 20_000, 1_000).await;
    assert_eq!((ev.publication.as_str(), ev.version, ev.a_des_resultats), ("explicite", 1, false));
    // Réglage sans résultats : libre ; version attendue.
    let m = ent
        .eva_v3_modifier_evaluation_for_actor(
            w.etab, w.prof, PROF, jour(), maintenant(), ev.id,
            &ModifierEvaluationV3Body { version_attendue: ev.version, titre: Some("DS 1 bis".into()), date_eval: None, bareme_m: Some(40_000), coef_m: Some(2_000), comptage: None, hors_bareme_autorise: None, annoncee: None, motif: None },
        )
        .await
        .unwrap();
    assert_eq!((m.titre.as_str(), m.bareme_m, m.coef_m), ("DS 1 bis", 40_000, 2_000));
    // Version périmée : conflit.
    let r = ent
        .eva_v3_modifier_evaluation_for_actor(
            w.etab, w.prof, PROF, jour(), maintenant(), ev.id,
            &ModifierEvaluationV3Body { version_attendue: ev.version, titre: Some("autre".into()), date_eval: None, bareme_m: None, coef_m: None, comptage: None, hors_bareme_autorise: None, annoncee: None, motif: None },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    // Des résultats existent : le barème ne change plus sans motif ; jamais sous une note déjà saisie.
    lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "30", None)]).await;
    let corps_modif = |v: i32, bar: i64, motif: Option<&str>| ModifierEvaluationV3Body {
        version_attendue: v, titre: None, date_eval: None, bareme_m: Some(bar), coef_m: None, comptage: None,
        hors_bareme_autorise: None, annoncee: None, motif: motif.map(String::from),
    };
    let r = ent.eva_v3_modifier_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), ev.id, &corps_modif(m.version, 20_000, None)).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "motif exigé : {r:?}");
    let r = ent.eva_v3_modifier_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), ev.id, &corps_modif(m.version, 20_000, Some("erreur de barème"))).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "la note 30 dépasserait le barème 20 : {r:?}");
    let ok = ent.eva_v3_modifier_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), ev.id, &corps_modif(m.version, 50_000, Some("erreur de barème"))).await.unwrap();
    assert_eq!(ok.bareme_m, 50_000);
    let histo: i64 = sqlx::query_scalar("SELECT count(*) FROM eva_evaluation_historique WHERE etablissement_id = $1 AND evaluation_id = $2")
        .bind(w.etab).bind(ev.id).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(histo >= 2, "chaque changement de calcul est journalisé : {histo}");
    // Annulation : motif obligatoire ; ensuite plus de saisie.
    let r = ent.eva_v3_annuler_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), ev.id, &AnnulerEvaluationV3Body { version_attendue: ok.version, motif: "x".into() }).await;
    assert!(r.is_err());
    let an = ent.eva_v3_annuler_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), ev.id, &AnnulerEvaluationV3Body { version_attendue: ok.version, motif: "sujet diffusé".into() }).await.unwrap();
    assert_eq!(an.etat, "annulee");
    let r = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, w.eleves_a[1], "10", None)]).await;
    assert_eq!(issue(&r, 0), "refuse");
}

#[tokio::test]
async fn v3_evaluation_restreinte_et_sous_matiere() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    let mut c = corps(w.cours_maths, "Oral demi-groupe", d(2031, 10, 10), 20_000, 1_000);
    c.restreinte_a = vec![a];
    let ev = ent.eva_v3_creer_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), &c).await.unwrap();
    assert!(ev.restreinte);
    let r = lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "14", None), cell(ev.id, b, "14", None)]).await;
    assert_eq!((issue(&r, 0), issue(&r, 1)), ("ecrit", "refuse"));
    // Un élève visé hors du public : refusé à la création.
    let hors = creer_eleve(&ent, w.etab, "Pas-dans-le-cours").await;
    let mut c2 = corps(w.cours_maths, "Oral", d(2031, 10, 10), 20_000, 1_000);
    c2.restreinte_a = vec![hors];
    assert!(ent.eva_v3_creer_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), &c2).await.is_err());
    // Une sous-matière d'une autre matière est refusée par la base.
    let mut c3 = corps(w.cours_maths, "Sous-matière", d(2031, 10, 10), 20_000, 1_000);
    c3.sous_matiere_id = Some(w.francais);
    assert!(ent.eva_v3_creer_evaluation_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), &c3).await.is_err());
}

// ───────────────────────── Remplaçant et co-enseignement ─────────────────────────

#[tokio::test]
async fn v3_suppleant_et_co_enseignant() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let remplacant = cast.profs.iter().map(|p| p.user_id).find(|x| *x != w.prof).unwrap();
    let ev = creer(&ent, &w, w.cours_maths, "DS du titulaire", d(2031, 10, 10), 20_000, 1_000).await;
    // Avant la suppléance : l'autre professeur n'a rien.
    assert!(matches!(
        ent.eva_v3_grille_for_actor(w.etab, remplacant, PROF, w.cours_maths, w.t1, jour(), maintenant()).await,
        Err(EntDbError::Forbidden)
    ));
    // Suppléance du 15 au 31 octobre : le suppléant saisit, lit tout l'historique du cours (CE-44).
    let service: Uuid = sqlx::query_scalar("SELECT id FROM ent_service_enseignement WHERE etablissement_id = $1 AND cours_id = $2")
        .bind(w.etab).bind(w.cours_maths).fetch_one(ent.pool_verif()).await.unwrap();
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
    let g = ent.eva_v3_grille_for_actor(w.etab, remplacant, PROF, w.cours_maths, w.t1, jour(), maintenant()).await.unwrap();
    assert!(g.peut_saisir, "le suppléant saisit");
    assert!(g.evaluations.iter().any(|e| e.id == ev.id), "il voit tout l'historique du cours");
    assert!(!g.evaluations[0].peut_modifier, "mais ne règle pas l'évaluation du titulaire sans droit de correction");
    let r = lot_par(&ent, &w, remplacant, PROF, w.cours_maths, vec![cell(ev.id, a, "12", None)]).await.unwrap();
    assert_eq!(issue(&r, 0), "ecrit");
    let r = ent
        .eva_v3_modifier_evaluation_for_actor(
            w.etab, remplacant, PROF, jour(), maintenant(), ev.id,
            &ModifierEvaluationV3Body { version_attendue: ev.version, titre: Some("renommée".into()), date_eval: None, bareme_m: None, coef_m: None, comptage: None, hors_bareme_autorise: None, annoncee: None, motif: None },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Il crée ses propres évaluations.
    let propre = ent
        .eva_v3_creer_evaluation_for_actor(w.etab, remplacant, PROF, jour(), maintenant(), &corps(w.cours_maths, "DS du suppléant", d(2031, 10, 17), 20_000, 1_000))
        .await
        .unwrap();
    assert!(propre.peut_modifier);
    // Le titulaire n'a plus la saisie pendant la suppléance (une saisie par jour), mais garde la lecture.
    let gt = grille(&ent, &w, w.cours_maths).await;
    assert!(!gt.peut_saisir, "le suppléant retire la saisie au titulaire pendant la période");
    assert_eq!(gt.evaluations.len(), 2);
}

// ───────────────────────── Publication ─────────────────────────

async fn publiees(ent: &PgEnt, w: &W, ev: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM eva_resultat_publie WHERE etablissement_id = $1 AND evaluation_id = $2")
        .bind(w.etab).bind(ev).fetch_one(ent.pool_verif()).await.unwrap()
}

fn pub_body(action: &str, prevue: Option<DateTime<Utc>>, motif: Option<&str>) -> PublierV3Body {
    PublierV3Body { action: action.into(), prevue_le: prevue, prevue_locale: None, motif: motif.map(String::from) }
}

#[tokio::test]
async fn v3_publication_programmee_retrait_et_journal() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 10), 20_000, 1_000).await;
    lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "14", None)]).await;
    assert_eq!(publiees(&ent, &w, ev.id).await, 0, "rien n'est visible sans publication (R-V7)");
    // Programmation dans le passé : refusée ; dans le futur : acceptée.
    assert!(ent.eva_v3_publier_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), ev.id, &pub_body("programmer", Some(maintenant() - chrono::Duration::hours(1)), None)).await.is_err());
    let prevue = maintenant() + chrono::Duration::hours(30);
    let p = ent.eva_v3_publier_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), ev.id, &pub_body("programmer", Some(prevue), None)).await.unwrap();
    assert_eq!((p.publication.as_str(), p.publication_prevue_le), ("programmee", Some(prevue)));
    // Le planificateur : avant l'heure, rien ; à l'heure, publié ; idempotent.
    let b0 = ent.eva_v3_publications_automatiques_au(w.etab, maintenant(), jour()).await.unwrap();
    assert_eq!(b0.programmees, 0);
    assert_eq!(publiees(&ent, &w, ev.id).await, 0);
    let apres = prevue + chrono::Duration::minutes(5);
    let b1 = ent.eva_v3_publications_automatiques_au(w.etab, apres, jour() + chrono::Duration::days(2)).await.unwrap();
    assert_eq!(b1.programmees, 1);
    assert_eq!(publiees(&ent, &w, ev.id).await, 1);
    let b2 = ent.eva_v3_publications_automatiques_au(w.etab, apres, jour() + chrono::Duration::days(2)).await.unwrap();
    assert_eq!(b2, BilanPublications::default(), "idempotent");
    // Retrait par le professeur dans les 24 h : motif exigé ; la famille verra « retiré pour vérification ».
    let t = apres + chrono::Duration::hours(2);
    let r = ent.eva_v3_publier_for_actor(w.etab, w.prof, PROF, jour(), t, ev.id, &pub_body("retirer", None, None)).await;
    assert!(r.is_err(), "motif obligatoire");
    let g = ent.eva_v3_publier_for_actor(w.etab, w.prof, PROF, jour(), t, ev.id, &pub_body("retirer", None, Some("erreur de barème"))).await.unwrap();
    assert!(g.publiee_le.is_none() && g.retiree_le.is_some());
    assert_eq!(publiees(&ent, &w, ev.id).await, 0);
    // Republier maintenant : plus de trace de retrait ; puis, au-delà de 24 h, le professeur ne retire plus.
    let g = ent.eva_v3_publier_for_actor(w.etab, w.prof, PROF, jour(), t, ev.id, &pub_body("maintenant", None, None)).await.unwrap();
    assert!(g.publiee_le.is_some() && g.retiree_le.is_none());
    let tard = t + chrono::Duration::hours(30);
    let r = ent.eva_v3_publier_for_actor(w.etab, w.prof, PROF, jour(), tard, ev.id, &pub_body("retirer", None, Some("encore une erreur"))).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // La direction le peut, après 24 h.
    let g = ent.eva_v3_publier_for_actor(w.etab, w.direction, DIR, jour(), tard, ev.id, &pub_body("retirer", None, Some("décision de la direction"))).await.unwrap();
    assert!(g.retiree_le.is_some());
    // Journal des publications (ajout seul).
    let actions: Vec<String> = sqlx::query_scalar("SELECT action FROM eva_evaluation_publication_journal WHERE etablissement_id = $1 AND evaluation_id = $2 ORDER BY le, id")
        .bind(w.etab).bind(ev.id).fetch_all(ent.pool_verif()).await.unwrap();
    assert_eq!(actions, ["programmee", "publiee_auto_programmee", "retiree", "publiee", "retiree"]);
    let r = sqlx::query("UPDATE eva_evaluation_publication_journal SET motif = 'x' WHERE etablissement_id = $1").bind(w.etab).execute(ent.pool_verif()).await;
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
    exec(&ent, "UPDATE eva_reglage SET delai_publication_max_jours = 15 WHERE etablissement_id = $1", vec![u(w.etab)]).await;
    let e1 = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 1), 20_000, 1_000).await;
    let e2 = creer(&ent, &w, w.cours_maths, "DS 2 en correction", d(2031, 10, 1), 20_000, 1_000).await;
    let e3 = creer(&ent, &w, w.cours_maths, "DS 3 sans note", d(2031, 10, 1), 20_000, 1_000).await;
    for e in [&e1, &e2] {
        lot(&ent, &w, w.cours_maths, vec![cell(e.id, a, "12", None)]).await;
    }
    ent.eva_v3_en_cours_correction_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), e2.id, true).await.unwrap();
    // Délai de 15 jours après le 1er octobre : terme le 16. Le 13 : trop tôt ; le 14 : rappel unique (deux jours avant).
    let j13 = d(2031, 10, 13);
    let b = ent.eva_v3_publications_automatiques_au(w.etab, j13.and_hms_opt(12, 0, 0).unwrap().and_utc(), j13).await.unwrap();
    assert_eq!((b.rappels, b.delai_max), (0, 0), "trop tôt");
    let j14 = d(2031, 10, 14);
    let b = ent.eva_v3_publications_automatiques_au(w.etab, j14.and_hms_opt(12, 0, 0).unwrap().and_utc(), j14).await.unwrap();
    assert_eq!((b.rappels, b.delai_max), (1, 0), "rappel de DS 1 seulement (DS 2 est en cours de correction, DS 3 n'a pas de note)");
    let b = ent.eva_v3_publications_automatiques_au(w.etab, j14.and_hms_opt(18, 0, 0).unwrap().and_utc(), j14).await.unwrap();
    assert_eq!(b.rappels, 0, "un seul rappel");
    // Le 16 : DS 1 est publiée ; DS 2 (en cours de correction) et DS 3 (sans résultat) non.
    let j16 = d(2031, 10, 16);
    let b = ent.eva_v3_publications_automatiques_au(w.etab, j16.and_hms_opt(12, 0, 0).unwrap().and_utc(), j16).await.unwrap();
    assert_eq!(b.delai_max, 1);
    assert_eq!(publiees(&ent, &w, e1.id).await, 1);
    assert_eq!(publiees(&ent, &w, e2.id).await, 0);
    assert_eq!(publiees(&ent, &w, e3.id).await, 0);
    // Quand le professeur lève « en cours de correction », la publication suit au passage suivant.
    ent.eva_v3_en_cours_correction_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), e2.id, false).await.unwrap();
    let j17 = d(2031, 10, 17);
    let b = ent.eva_v3_publications_automatiques_au(w.etab, j17.and_hms_opt(12, 0, 0).unwrap().and_utc(), j17).await.unwrap();
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
    let ev = creer(&ent, &w, w.cours_maths, "Titre secret Zèbre", d(2031, 10, 10), 20_000, 1_000).await;
    lot(&ent, &w, w.cours_maths, vec![cell(ev.id, a, "13,37", None)]).await;
    ent.eva_v3_publier_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), ev.id, &pub_body("maintenant", None, None)).await.unwrap();
    ent.eva_v3_publier_for_actor(w.etab, w.prof, PROF, jour(), maintenant(), ev.id, &pub_body("retirer", None, Some("motif confidentiel Zèbre"))).await.unwrap();
    let payloads: Vec<String> = sqlx::query_scalar(
        "SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type LIKE 'evaluations.%'",
    )
    .bind(w.etab)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap_or_default();
    assert!(!payloads.is_empty(), "des événements neutres sont émis");
    for p in &payloads {
        assert!(!p.contains("13") || !p.contains("37"), "{p}");
        assert!(!p.contains("Zèbre") && !p.contains("confidentiel") && !p.contains("Abbott"), "{p}");
    }
}

// ───────────────────────── Collage et import ─────────────────────────

fn ligne_nom(nom: &str, cellules: &[&str]) -> ImportLigneBody {
    ImportLigneBody { eleve_id: None, nom: Some(nom.into()), rang: None, cellules: cellules.iter().map(|s| s.to_string()).collect() }
}

#[tokio::test]
async fn v3_import_apercu_erreurs_ligne_a_ligne_confirmation_et_conflit() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let e1 = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 10), 20_000, 1_000).await;
    let e2 = creer(&ent, &w, w.cours_maths, "DS 2", d(2031, 10, 17), 20_000, 1_000).await;
    lot(&ent, &w, w.cours_maths, vec![cell(e1.id, c, "10", None)]).await;
    // Les trois élèves du labo s'appellent « <Nom> Test » ; on rapproche par nom dans n'importe quel ordre et casse.
    let corps_import = |empreinte: Option<String>| ImportBody {
        cours_id: w.cours_maths,
        mode: "fichier".into(),
        cibles: vec![e1.id, e2.id],
        lignes: vec![
            ligne_nom("ABBOTT test", &["15,5", "abs"]),
            ligne_nom("Test Bones", &["12", ""]),
            ligne_nom("Corner Test", &["10", "7"]),   // 10 inchangé, 7 nouveau
            ligne_nom("Inconnu Test", &["9", "9"]),   // non rapproché
            ligne_nom("Abbott Test", &["1", "1"]),    // doublon de la première ligne : écarté
        ],
        empreinte,
    };
    let ap = ent.eva_v3_apercu_import_for_actor(w.etab, w.prof, PROF, jour(), &corps_import(None)).await.unwrap();
    assert_eq!(ap.totaux.lignes, 5);
    assert_eq!(ap.lignes[0].rapprochement, "ok");
    assert_eq!(ap.lignes[3].rapprochement, "non_rapproche");
    assert_eq!(ap.lignes[4].rapprochement, "ambigu", "un élève ne figure qu'une fois dans un import");
    let cc = &ap.lignes[2].cellules;
    assert_eq!((cc[0].etat.as_str(), cc[1].etat.as_str()), ("inchange", "nouveau"));
    assert_eq!(ap.lignes[1].cellules[1].etat, "vide");
    // Rien n'a été écrit par l'aperçu.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM eva_resultat WHERE etablissement_id = $1").bind(w.etab).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(n, 1);
    // Confirmation sans empreinte ou avec une empreinte fausse : refus.
    assert!(ent.eva_v3_confirmer_import_for_actor(w.etab, w.prof, PROF, jour(), &corps_import(None)).await.is_err());
    assert!(matches!(
        ent.eva_v3_confirmer_import_for_actor(w.etab, w.prof, PROF, jour(), &corps_import(Some("0".repeat(64)))).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Une note change entre l'aperçu et la confirmation : l'empreinte ne correspond plus, rien n'est écrit.
    lot(&ent, &w, w.cours_maths, vec![cell(e1.id, a, "3", None)]).await;
    assert!(matches!(
        ent.eva_v3_confirmer_import_for_actor(w.etab, w.prof, PROF, jour(), &corps_import(Some(ap.empreinte.clone()))).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Nouvel aperçu, puis confirmation : seules les lignes rapprochées sont écrites ; la case d'Abbott change (3 → 15,5).
    let ap2 = ent.eva_v3_apercu_import_for_actor(w.etab, w.prof, PROF, jour(), &corps_import(None)).await.unwrap();
    assert_eq!(ap2.lignes[0].cellules[0].etat, "modifie");
    let r = ent.eva_v3_confirmer_import_for_actor(w.etab, w.prof, PROF, jour(), &corps_import(Some(ap2.empreinte.clone()))).await.unwrap();
    let ecrites = r.cellules.iter().filter(|x| x.issue == "ecrit").count();
    assert_eq!(ecrites, 4, "Abbott ×2, Bones ×1, Corner ×1 : {r:?}");
    let g = grille(&ent, &w, w.cours_maths).await;
    let l = |id: Uuid| g.eleves.iter().find(|e| e.eleve_id == id).unwrap();
    assert_eq!(l(a).cellules[0].valeur_m, Some(15_500));
    assert_eq!(l(a).cellules[1].statut.as_deref(), Some("absent"));
    assert_eq!(l(b).cellules[0].valeur_m, Some(12_000));
    assert_eq!(l(c).cellules[1].valeur_m, Some(7_000));
    // La source « import » est au journal de la note ; le journal des imports ne porte que des comptes.
    let h = ent.eva_v3_historique_for_actor(w.etab, w.prof, PROF, e2.id, c, jour()).await.unwrap();
    assert_eq!(h.last().unwrap().source, "import", "la case créée par l'import porte la source import");
    let (nb_lignes, nb_ecrites): (i32, i32) = sqlx::query_as("SELECT nb_lignes, nb_ecrites FROM eva_import_journal WHERE etablissement_id = $1")
        .bind(w.etab).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!((nb_lignes, nb_ecrites), (3, 4));
    // L'import est dans SES évaluations, à sa portée : un autre professeur, la direction, la famille sont refusés.
    let autre = cast.profs.iter().map(|p| p.user_id).find(|x| *x != w.prof).unwrap();
    for (u_id, role) in [(autre, PROF), (w.direction, DIR), (cast.parent_lea.user_id, "parent")] {
        let r = ent.eva_v3_apercu_import_for_actor(w.etab, u_id, role, jour(), &corps_import(None)).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
    // Un autre cours : introuvable.
    let mut autre_cours = corps_import(None);
    autre_cours.cours_id = w.cours_fr;
    assert!(ent.eva_v3_apercu_import_for_actor(w.etab, w.prof, PROF, jour(), &autre_cours).await.is_err());
}

#[tokio::test]
async fn v3_collage_par_rang_et_par_identifiant() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 10), 20_000, 1_000).await;
    // 3 élèves dans l'ordre alphabétique (Abbott, Bones, Corner) ; 4 lignes collées : la 4ᵉ ne se rapproche pas.
    let mk = |rang: usize, t: &str| ImportLigneBody { eleve_id: None, nom: None, rang: Some(rang), cellules: vec![t.into()] };
    let body = ImportBody {
        cours_id: w.cours_maths,
        mode: "collage".into(),
        cibles: vec![ev.id],
        lignes: vec![mk(1, "8"), mk(2, "9,5"), mk(3, "nn"), mk(4, "12")],
        empreinte: None,
    };
    let ap = ent.eva_v3_apercu_import_for_actor(w.etab, w.prof, PROF, jour(), &body).await.unwrap();
    assert_eq!((ap.totaux.rapprochees, ap.totaux.non_rapprochees), (3, 1));
    let r = ent.eva_v3_confirmer_import_for_actor(w.etab, w.prof, PROF, jour(), &ImportBody { empreinte: Some(ap.empreinte), ..body }).await.unwrap();
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
    ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR).await.unwrap();
    let anc = ancien_monde(&ent, &w).await;
    let a = w.eleves_a[0];
    let n = ancienne_note(&ent, &w, anc.devoirs[0], a, 12.0, 20.0, "note").await;
    ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR).await.unwrap();
    let corr = |de: f64, vers: f64, le: &'static str, motif: &'static str| {
        let ent = &ent;
        let w = &w;
        let anc_pi = anc.note_periode;
        async move {
            exec(ent, &format!("UPDATE ent_note SET valeur = {vers} WHERE id = $1"), vec![u(n)]).await;
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
    let rapport = ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR).await.unwrap();
    for c in &rapport.comptes {
        assert_eq!(c.lues, c.creees + c.inchangees + c.refusees + c.ignorees, "invariant {}", c.objet);
    }
    assert!(rapport.lignes.iter().any(|l| l.source_id == n && l.code == "correction_recopiee"), "{:?}", rapport.lignes);
    let hist: Vec<(i32, Option<String>, Option<String>, bool, Option<String>)> = sqlx::query_as(
        "SELECT version, valeur_avant::text, valeur_apres::text, correction, motif FROM eva_resultat_historique
          WHERE etablissement_id = $1 AND resultat_id = $2 ORDER BY version",
    )
    .bind(w.etab).bind(n).fetch_all(ent.pool_verif()).await.unwrap();
    assert_eq!(hist.len(), 3, "création + deux corrections : {hist:?}");
    assert_eq!(hist[1].4.as_deref(), Some("premiere"));
    assert_eq!(hist[2].4.as_deref(), Some("seconde"));
    assert!(hist[1].3 && hist[2].3);
    let v: String = sqlx::query_scalar("SELECT valeur::text FROM eva_resultat WHERE id = $1").bind(n).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(v, "12.000");
    // Idempotent : rien de plus à la reprise suivante.
    ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR).await.unwrap();
    let nb: i64 = sqlx::query_scalar("SELECT count(*) FROM eva_resultat_historique WHERE resultat_id = $1").bind(n).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(nb, 3);
    // La nouvelle grille a écrit entre-temps : l'ancienne correction n'écrase rien, le conflit est listé.
    let ev = anc.devoirs[0];
    let r = lot(&ent, &w, w.cours_maths, vec![cell(ev, a, "16", Some(3))]).await;
    assert_eq!(issue(&r, 0), "ecrit", "{r:?}");
    corr(12.0, 13.0, "2031-11-22T09:00:00Z", "troisieme").await;
    let rapport = ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR).await.unwrap();
    assert!(rapport.lignes.iter().any(|l| l.source_id == n && l.code == "deux_ecrivains"), "{:?}", rapport.lignes);
    let v: String = sqlx::query_scalar("SELECT valeur::text FROM eva_resultat WHERE id = $1").bind(n).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(v, "16.000", "la saisie de la nouvelle grille n'est pas écrasée");
}

#[tokio::test]
async fn f7_deux_reprises_simultanees_ne_comptent_pas_de_doublons_comme_refuses() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    ent.eva_v1_reprendre_for_actor(w.etab, w.direction, DIR).await.unwrap();
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
        assert!(!r.lignes.iter().any(|l| l.code == "erreur_sql"), "aucun refus erreur_sql : {:?}", r.lignes);
        for c in &r.comptes {
            assert_eq!(c.lues, c.creees + c.inchangees + c.refusees + c.ignorees, "invariant {}", c.objet);
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
    let autre = cast.profs.iter().map(|p| p.user_id).find(|x| *x != w.prof).unwrap();
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, autre, PROF, &crate::tests_evaluations_v1::note(ev_fr, a, 15_000)).await.unwrap();
    for (i, bareme) in [999.983_f64, 999.979, 999.961, 999.959, 999.953, 999.931, 999.907, 999.893, 999.883].iter().enumerate() {
        let ev = evaluation_directe(&ent, &w, w.cours_maths, d(2031, 10, 1 + i as u32), &format!("P{i}"), *bareme).await;
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &crate::tests_evaluations_v1::note(ev, a, 500_000 + i as i64 * 70)).await.unwrap();
    }
    let m = ent
        .eva_v1_moyennes_personnel_for_actor(w.etab, w.direction, DIR, a, w.t1)
        .await
        .expect("le calcul ne doit pas échouer en bloc");
    let fr = m.matieres.iter().find(|x| x.matiere_id == w.francais).expect("français");
    assert_eq!(fr.moyenne.affichee_m, Some(15_000), "le français reste calculé");
    let maths = m.matieres.iter().find(|x| x.matiere_id == w.maths).expect("maths");
    if maths.moyenne.exacte.is_none() {
        assert!(maths.moyenne.avertissements.iter().any(|a| a.contains("impossible")), "{:?}", maths.moyenne.avertissements);
        assert!(m.generale.avertissements.iter().any(|a| a.contains("indisponible")), "{:?}", m.generale.avertissements);
        assert_eq!(m.generale.affichee_m, Some(15_000), "la générale s'établit sur les autres matières");
    }
}

async fn evaluation_directe(ent: &PgEnt, w: &W, cours: Uuid, date: NaiveDate, titre: &str, bareme: f64) -> Uuid {
    crate::tests_evaluations_v1::evaluation(ent, w, cours, date, titre, bareme, 1.0, domain_eleve::evaluation_moteur::Comptage::Normal, false).await
}
