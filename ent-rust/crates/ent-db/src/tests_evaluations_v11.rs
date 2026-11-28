//! Socle 21, lot V11 « alertes précoces », partie déterministe : tests sur base (feature `labo`). Aucune horloge réelle
//! dans le calcul : jour fixe 2031-10-20 (monde V1 : année 2031-2032, T1 puis T2), dates d'évaluation fixes.

use super::*;
use crate::tests_evaluations::labo;
use crate::tests_evaluations_v1::{
    affecter, d, evaluation, exec, monde_v1, note, statut, u, DIR, PROF, W,
};
use chrono::NaiveDate;
use domain_eleve::evaluation_moteur::{Comptage, ReglesMoteur, StatutCalcul};
use ent_types::evaluations_v11::*;
use uuid::Uuid;

const CPE: &str = "cpe";

fn jour() -> NaiveDate {
    d(2031, 10, 20)
}

struct Fx {
    w: W,
    autre_prof: Uuid,
    cpe: Uuid,
    parent: Uuid,
}

async fn ecrire(ent: &PgEnt, prof: Uuid, ev: Uuid, eleve: Uuid, valeur_m: i64) {
    ent.eva_v1_ecrire_resultat_for_actor_test(prof, ev, eleve, Some(valeur_m), StatutCalcul::Note)
        .await;
}

impl PgEnt {
    /// Écrit un résultat par le service (le professeur du cours), ou échoue le test.
    async fn eva_v1_ecrire_resultat_for_actor_test(
        &self,
        prof: Uuid,
        ev: Uuid,
        eleve: Uuid,
        valeur_m: Option<i64>,
        s: StatutCalcul,
    ) {
        let etab: Uuid =
            sqlx::query_scalar("SELECT etablissement_id FROM eva_evaluation WHERE id = $1")
                .bind(ev)
                .fetch_one(self.pool_verif())
                .await
                .unwrap();
        let corps = match valeur_m {
            Some(v) => note(ev, eleve, v),
            None => statut(ev, eleve, s),
        };
        self.eva_v1_ecrire_resultat_for_actor(etab, prof, PROF, &corps)
            .await
            .unwrap_or_else(|e| panic!("écriture du résultat : {e:?}"));
    }
}

async fn absent(ent: &PgEnt, prof: Uuid, ev: Uuid, eleve: Uuid) {
    ent.eva_v1_ecrire_resultat_for_actor_test(prof, ev, eleve, None, StatutCalcul::Absent)
        .await;
}

async fn eval(ent: &PgEnt, w: &W, cours: Uuid, date: NaiveDate, titre: &str) -> Uuid {
    evaluation(
        ent,
        w,
        cours,
        date,
        titre,
        20.0,
        1.0,
        Comptage::Normal,
        false,
    )
    .await
}

/// Le monde des alertes : classe A (Abbott, Bones, Corner), le professeur du labo y est professeur principal et
/// enseigne les maths ; un autre professeur y enseigne le français.
///
/// * Abbott : 16 puis 6 en maths, 14 puis 7 en français → moyenne générale 15 puis 6,5 (baisse de 8,5 points).
/// * Bones : 12 partout ; trois absences aux maths de T2, mais un aménagement de scolarité actif (CE-70).
/// * Corner : 12 partout ; trois absences aux maths de T2, sans aménagement.
async fn monde(ent: &PgEnt, cast: &LaboCast) -> Fx {
    let w = monde_v1(ent, cast).await;
    let autre_prof: Uuid = sqlx::query_scalar(
        "SELECT prof_user_id FROM ent_service_enseignement WHERE etablissement_id = $1 AND cours_id = $2",
    )
    .bind(w.etab)
    .bind(w.cours_fr)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    exec(
        ent,
        "UPDATE ent_service_enseignement SET professeur_principal = true
          WHERE etablissement_id = $1 AND cours_id = $2 AND prof_user_id = $3",
        vec![u(w.etab), u(w.cours_maths), u(w.prof)],
    )
    .await;
    ent.eva_v1_definir_regles_for_actor(
        w.etab,
        w.direction,
        DIR,
        w.annee,
        &ReglesMoteur::default(),
        500,
    )
    .await
    .unwrap();
    let [a, b, c] = [w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]];
    let m1 = eval(ent, &w, w.cours_maths, d(2031, 10, 10), "Maths T1").await;
    let f1 = eval(ent, &w, w.cours_fr, d(2031, 10, 11), "Français T1").await;
    let m2 = eval(ent, &w, w.cours_maths, d(2032, 1, 15), "Maths T2").await;
    let f2 = eval(ent, &w, w.cours_fr, d(2032, 1, 16), "Français T2").await;
    for (ev, prof, va, vb) in [
        (m1, w.prof, 16_000, 12_000),
        (f1, autre_prof, 14_000, 12_000),
        (m2, w.prof, 6_000, 12_000),
        (f2, autre_prof, 7_000, 12_000),
    ] {
        ecrire(ent, prof, ev, a, va).await;
        ecrire(ent, prof, ev, b, vb).await;
        ecrire(ent, prof, ev, c, vb).await;
    }
    for i in 0..3 {
        let ev = eval(
            ent,
            &w,
            w.cours_maths,
            d(2032, 2, 2 + i),
            &format!("Interrogation {i}"),
        )
        .await;
        absent(ent, w.prof, ev, b).await;
        absent(ent, w.prof, ev, c).await;
        ecrire(ent, w.prof, ev, a, 6_000).await;
    }
    exec(
        ent,
        "INSERT INTO ent_amenagement (id, etablissement_id, eleve_id, code, portee, date_debut, defini_par_user_id)
         VALUES ($1, $2, $3, 'emploi_du_temps_amenage', 'les_deux', '2031-09-01', $4)",
        vec![u(Uuid::new_v4()), u(w.etab), u(b), u(w.direction)],
    )
    .await;
    Fx {
        w,
        autre_prof,
        cpe: cast.cpe.user_id,
        parent: cast.parent_lea.user_id,
    }
}

fn regles(e: &EleveAlerteDto) -> Vec<RegleAlerte> {
    e.alertes.iter().map(|a| a.regle).collect()
}

async fn consultations(ent: &PgEnt, etab: Uuid, vue: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM eva_alerte_consultation WHERE etablissement_id = $1 AND vue = $2",
    )
    .bind(etab)
    .bind(vue)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap()
}

#[tokio::test]
async fn v11_alertes_de_classe_expliquees_ce70_reglage_et_journal() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let fx = monde(&ent, &cast).await;
    let w = &fx.w;
    let [a, c] = [w.eleves_a[0], w.eleves_a[2]];

    // Le professeur principal voit les alertes de sa classe (position normale par défaut).
    let v = ent
        .eva_v11_alertes_classe_for_actor(w.etab, w.prof, PROF, jour(), w.classe_a, w.t2)
        .await
        .unwrap();
    assert_eq!(v.seuils.position, PositionAlertes::Normal);
    assert_eq!(v.effectif, 3);
    assert_eq!(v.periode_precedente_id, Some(w.t1));
    let ids: Vec<Uuid> = v.eleves.iter().map(|e| e.eleve_id).collect();
    assert_eq!(
        ids,
        vec![a, c],
        "Bones n'a pas d'alerte (absences sous aménagement, CE-70) ; rangés par nom"
    );
    let abbott = &v.eleves[0];
    assert_eq!(
        regles(abbott),
        vec![RegleAlerte::BaisseGenerale],
        "deux matières sous 8 : sous le seuil de 3"
    );
    assert_eq!(abbott.moyenne_generale_precedente_m, Some(15_000));
    assert!(
        abbott.alertes[0].faits[0].contains("15") && abbott.alertes[0].faits[0].contains("6,5"),
        "{:?}",
        abbott.alertes[0].faits
    );
    assert_eq!(regles(&v.eleves[1]), vec![RegleAlerte::AbsencesEvaluations]);
    assert_eq!(
        v.eleves[1].alertes[0].faits[0],
        "3 absences aux évaluations de la période."
    );
    assert!(v.mention.contains("sans intelligence artificielle"));
    for e in &v.eleves {
        for x in &e.alertes {
            assert!(
                x.faits.iter().all(|t| !t.contains("aménagement")),
                "la nature d'un aménagement n'est jamais dite"
            );
        }
    }

    // T1 : pas de période précédente, aucune baisse ; avertissement explicite.
    let v1 = ent
        .eva_v11_alertes_classe_for_actor(w.etab, w.direction, DIR, jour(), w.classe_a, w.t1)
        .await
        .unwrap();
    assert!(v1.eleves.is_empty());
    assert!(v1
        .avertissements
        .iter()
        .any(|x| x.contains("Aucune période précédente")));

    // Réglage : le professeur le lit, ne le change pas ; la direction passe en strict → deux matières suffisent.
    let r = ent
        .eva_v11_reglage_for_actor(w.etab, w.prof, PROF, w.annee)
        .await
        .unwrap();
    assert_eq!(r.positions.len(), 3);
    let corps = DefinirReglageAlertesBody {
        annee_id: w.annee,
        position: PositionAlertes::Strict,
    };
    assert!(matches!(
        ent.eva_v11_definir_reglage_for_actor(w.etab, w.prof, PROF, &corps)
            .await,
        Err(EntDbError::Forbidden)
    ));
    let r = ent
        .eva_v11_definir_reglage_for_actor(w.etab, w.direction, DIR, &corps)
        .await
        .unwrap();
    assert_eq!(r.seuils.position, PositionAlertes::Strict);
    let v = ent
        .eva_v11_alertes_classe_for_actor(w.etab, w.prof, PROF, jour(), w.classe_a, w.t2)
        .await
        .unwrap();
    assert_eq!(
        regles(&v.eleves[0]),
        vec![RegleAlerte::BaisseGenerale, RegleAlerte::MatieresSousSeuil]
    );

    // Droits : un professeur qui n'est pas PP, un CPE sans mission, une famille : refusés.
    for (qui, role) in [(fx.autre_prof, PROF), (fx.cpe, CPE), (fx.parent, "parent")] {
        assert!(
            matches!(
                ent.eva_v11_alertes_classe_for_actor(w.etab, qui, role, jour(), w.classe_a, w.t2)
                    .await,
                Err(EntDbError::Forbidden)
            ),
            "{role}"
        );
    }
    // Le CPE référent du niveau de la classe (mission) voit les alertes.
    let niveau = Uuid::new_v4();
    exec(
        &ent,
        "INSERT INTO ent_niveau (id, etablissement_id, code, libelle) VALUES ($1, $2, $3, $3)",
        vec![
            u(niveau),
            u(w.etab),
            crate::tests_evaluations_v1::t(&format!("N{}", &niveau.simple().to_string()[..6])),
        ],
    )
    .await;
    exec(
        &ent,
        "UPDATE ent_classe SET niveau_id = $2 WHERE id = $1",
        vec![u(w.classe_a), u(niveau)],
    )
    .await;
    let agent: Uuid = sqlx::query_scalar(
        "SELECT a.id FROM ent_agent a JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND p.user_id = $2 AND a.fusionne_dans_agent_id IS NULL",
    )
    .bind(w.etab)
    .bind(fx.cpe)
    .fetch_one(ent.pool_verif())
    .await
    .expect("le CPE du labo a un agent");
    {
        let mission = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_mission (id, etablissement_id, type_id, agent_id, du)
             SELECT $1, $2, id, $3, '2031-09-01' FROM ent_mission_type WHERE code = 'cpe_referent' AND etablissement_id IS NULL",
        )
        .bind(mission)
        .bind(w.etab)
        .bind(agent)
        .execute(ent.pool_verif())
        .await
        .unwrap();
        exec(
            &ent,
            "INSERT INTO ent_mission_perimetre (id, etablissement_id, mission_id, nature, cible_id)
             VALUES (gen_random_uuid(), $1, $2, 'niveau', $3)",
            vec![u(w.etab), u(mission), u(niveau)],
        )
        .await;
        let v = ent
            .eva_v11_alertes_classe_for_actor(w.etab, fx.cpe, CPE, jour(), w.classe_a, w.t2)
            .await
            .unwrap();
        assert_eq!(v.eleves.len(), 2);
    }

    // Journal : chaque lecture laisse une ligne ; ajout seul ; la direction seule le lit.
    assert!(consultations(&ent, w.etab, "classe").await >= 3);
    let j = ent
        .eva_v11_consultations_for_actor(
            w.etab,
            w.direction,
            DIR,
            d(2099, 1, 1),
            Some(d(2026, 1, 1)),
            None,
        )
        .await;
    assert!(j.is_err(), "fenêtre de plus d'un an refusée");
    assert!(matches!(
        ent.eva_v11_consultations_for_actor(w.etab, w.prof, PROF, jour(), None, None)
            .await,
        Err(EntDbError::Forbidden)
    ));
    let maj =
        sqlx::query("UPDATE eva_alerte_consultation SET vue = 'cours' WHERE etablissement_id = $1")
            .bind(w.etab)
            .execute(ent.pool_verif())
            .await;
    assert!(maj.is_err(), "journal en ajout seul");
}

#[tokio::test]
async fn v11_alertes_du_cours_pour_son_professeur_seulement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let fx = monde(&ent, &cast).await;
    let w = &fx.w;
    let v = ent
        .eva_v11_alertes_cours_for_actor(w.etab, w.prof, PROF, jour(), w.cours_maths, w.t2)
        .await
        .unwrap();
    assert_eq!(v.effectif, 3);
    assert_eq!(v.sous_seuil, 1, "« 1 élève sous 8 dans ce groupe »");
    let ids: Vec<Uuid> = v.eleves.iter().map(|e| e.eleve_id).collect();
    assert_eq!(ids, vec![w.eleves_a[0], w.eleves_a[2]]);
    assert_eq!(v.eleves[0].moyenne_cours_m, Some(6_000));
    assert_eq!(v.eleves[0].alertes[0].regle, RegleAlerte::CoursSousSeuil);
    assert_eq!(
        v.eleves[1].alertes[0].regle,
        RegleAlerte::AbsencesEvaluations
    );
    // L'autre professeur ne lit pas le cours de maths ; la famille non plus.
    for (qui, role) in [(fx.autre_prof, PROF), (fx.parent, "parent")] {
        assert!(matches!(
            ent.eva_v11_alertes_cours_for_actor(w.etab, qui, role, jour(), w.cours_maths, w.t2)
                .await,
            Err(EntDbError::Forbidden)
        ));
    }
    // Il lit le sien : Abbott a 7 en français (sous 8).
    let f = ent
        .eva_v11_alertes_cours_for_actor(w.etab, fx.autre_prof, PROF, jour(), w.cours_fr, w.t2)
        .await
        .unwrap();
    assert_eq!(f.sous_seuil, 1);
    assert_eq!(consultations(&ent, w.etab, "cours").await, 2);
}

#[tokio::test]
async fn v11_ce_qui_a_change_et_suites_humaines() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let fx = monde(&ent, &cast).await;
    let w = &fx.w;
    let a = w.eleves_a[0];
    exec(
        &ent,
        "INSERT INTO eva_appreciation (id, etablissement_id, periode_id, eleve_id, type, texte, etat, auteur)
         VALUES ($1, $2, $3, $4, 'generale', 'Très bon trimestre, à poursuivre.', 'validee', $5),
                ($6, $2, $3, $4, 'vie_scolaire', 'Brouillon jamais montré', 'brouillon', $5)",
        vec![u(Uuid::new_v4()), u(w.etab), u(w.t1), u(a), u(w.prof), u(Uuid::new_v4())],
    )
    .await;
    let c = ent
        .eva_v11_changements_for_actor(w.etab, w.prof, PROF, jour(), a, w.t2)
        .await
        .unwrap();
    assert_eq!(c.periode_precedente_id, Some(w.t1));
    assert_eq!(c.generale.avant_m, Some(15_000));
    assert_eq!(c.generale.apres_m, Some(6_500));
    assert_eq!(c.generale.ecart_m, Some(-8_500));
    assert_eq!(c.matieres.len(), 2);
    assert!(c.matieres.iter().all(|m| m.ecart_m.is_some_and(|e| e < 0)));
    assert_eq!(
        c.alertes_nouvelles
            .iter()
            .map(|x| x.regle)
            .collect::<Vec<_>>(),
        vec![RegleAlerte::BaisseGenerale]
    );
    assert!(c.alertes_levees.is_empty() && c.alertes_maintenues.is_empty());
    assert_eq!(
        c.appreciations_precedentes.len(),
        1,
        "seule l'appréciation validée est rappelée"
    );
    assert!(c.suites.is_empty());

    // Suite humaine : posée par le PP, datée, visible dans la vue de classe ; jamais modifiable.
    let corps = PoserSuiteBody {
        periode_id: w.t2,
        nature: NatureSuite::EntretienPrevu,
        commentaire: Some("Entretien avec l'élève jeudi".into()),
    };
    let s = ent
        .eva_v11_poser_suite_for_actor(w.etab, w.prof, PROF, jour(), a, &corps)
        .await
        .unwrap();
    assert_eq!(s.nature, NatureSuite::EntretienPrevu);
    assert_eq!(s.auteur_id, w.prof);
    let v = ent
        .eva_v11_alertes_classe_for_actor(w.etab, w.prof, PROF, jour(), w.classe_a, w.t2)
        .await
        .unwrap();
    assert_eq!(
        v.eleves[0].derniere_suite.as_ref().map(|x| x.id),
        Some(s.id)
    );
    let l = ent
        .eva_v11_suites_for_actor(w.etab, w.direction, DIR, jour(), a, w.t2)
        .await
        .unwrap();
    assert_eq!(l.len(), 1);
    assert!(sqlx::query("DELETE FROM eva_alerte_suite WHERE id = $1")
        .bind(s.id)
        .execute(ent.pool_verif())
        .await
        .is_err());
    // Un professeur qui n'est pas PP ne pose ni ne lit de suite, ni les changements ; la famille non plus.
    for (qui, role) in [(fx.autre_prof, PROF), (fx.parent, "parent")] {
        assert!(matches!(
            ent.eva_v11_poser_suite_for_actor(w.etab, qui, role, jour(), a, &corps)
                .await,
            Err(EntDbError::Forbidden)
        ));
        assert!(matches!(
            ent.eva_v11_changements_for_actor(w.etab, qui, role, jour(), a, w.t2)
                .await,
            Err(EntDbError::Forbidden)
        ));
    }
    // Un commentaire trop long est refusé ; un élève d'une autre classe n'est pas suivi par ce PP.
    let long = PoserSuiteBody {
        commentaire: Some("x".repeat(501)),
        ..corps.clone()
    };
    assert!(ent
        .eva_v11_poser_suite_for_actor(w.etab, w.prof, PROF, jour(), a, &long)
        .await
        .is_err());
    assert!(matches!(
        ent.eva_v11_poser_suite_for_actor(w.etab, w.prof, PROF, jour(), w.eleves_d[0], &corps)
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Un élève arrivé en T2 n'a ni moyenne de T1, ni baisse : aucune alerte inventée.
    let nouveau = crate::tests_evaluations_v1::creer_eleve(&ent, w.etab, "Zabini").await;
    affecter(&ent, w.etab, w.annee, nouveau, w.classe_a, d(2032, 1, 5)).await;
    let c = ent
        .eva_v11_changements_for_actor(w.etab, w.prof, PROF, jour(), nouveau, w.t2)
        .await
        .unwrap();
    assert_eq!(c.generale.avant_m, None);
    assert!(c.alertes_nouvelles.is_empty());
    // Exclusions de l'IA : tout le personnel, pas la famille.
    assert!(ent.eva_v11_exclusions_ia_for_actor(PROF).unwrap().len() >= 4);
    assert!(matches!(
        ent.eva_v11_exclusions_ia_for_actor("parent"),
        Err(EntDbError::Forbidden)
    ));
}
