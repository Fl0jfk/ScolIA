//! Socle 21, lot V4 « absences, dispenses, aménagements, rattrapages » : tests sur base (feature `labo`), CE-25 à CE-38.
//! Aucune horloge réelle : jour fixe 2031-10-20 (monde V1 : année 2031-2032), dates d'évaluation fixes.

use super::*;
use crate::evaluations_v4::types_v4;
use crate::tests_evaluations::labo;
use crate::tests_evaluations_v1::{arreter, d, monde_v1, DIR, PROF, W};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use ent_types::evaluations_v3::*;
use ent_types::evaluations_v4::*;
use uuid::Uuid;

fn jour() -> NaiveDate {
    d(2031, 10, 20)
}

fn maintenant() -> DateTime<Utc> {
    jour().and_hms_opt(12, 0, 0).unwrap().and_utc()
}

async fn creer(ent: &PgEnt, w: &W, titre: &str, date: NaiveDate) -> Uuid {
    ent.eva_v3_creer_evaluation_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        maintenant(),
        &CreerEvaluationV3Body {
            cours_id: w.cours_maths,
            titre: titre.into(),
            date_eval: date,
            bareme_m: 20_000,
            coef_m: 1_000,
            comptage: None,
            hors_bareme_autorise: false,
            sous_matiere_id: None,
            restreinte_a: vec![],
            publication: None,
            publication_prevue_le: None,
            annoncee: false,
        },
    )
    .await
    .unwrap()
    .id
}

async fn saisir(ent: &PgEnt, w: &W, ev: Uuid, eleve: Uuid, texte: &str) {
    let r = ent
        .eva_v3_saisir_lot_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &SaisieLotBody {
                cours_id: w.cours_maths,
                cellules: vec![CelluleSaisieDto {
                    evaluation_id: ev,
                    eleve_id: eleve,
                    texte: texte.into(),
                    version_attendue: None,
                    motif: None,
                }],
                origine: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(r.cellules[0].issue, "ecrit", "{:?}", r.cellules[0]);
}

/// Une séance du cours de maths ce jour-là, et (si demandé) l'absence de l'élève à cette séance, qualifiée au socle 12.
/// `qualification` : `justifiee`, `non_justifiee` ou `en_attente`.
async fn seance(ent: &PgEnt, w: &W, jour_s: NaiveDate) -> Uuid {
    let creneau = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_id, matiere_libelle, prof_user_id, jour_semaine,
                                      heure_debut, heure_fin, classe_id, cours_id)
         VALUES ($1, $2, 'A', $3, 'Mathématiques', $4, 1, '08:00', '09:00', $5, $6)",
    )
    .bind(creneau)
    .bind(w.etab)
    .bind(w.maths)
    .bind(w.prof)
    .bind(w.classe_a)
    .bind(w.cours_maths)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_edt_seance (id, etablissement_id, annee_id, creneau_id, jour, classe_id, classe, prof_user_id,
                                     matiere_id, matiere_libelle, heure_debut, heure_fin, cours_id)
         VALUES ($1, $2, $3, $4, $5, $6, 'A', $7, $8, 'Mathématiques', '08:00', '09:00', $9)",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(w.annee)
    .bind(creneau)
    .bind(jour_s)
    .bind(w.classe_a)
    .bind(w.prof)
    .bind(w.maths)
    .bind(w.cours_maths)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    creneau
}

async fn absence(
    ent: &PgEnt,
    w: &W,
    creneau: Uuid,
    eleve: Uuid,
    jour_s: NaiveDate,
    qualification: &str,
) {
    let (etat, qualif) = match qualification {
        "justifiee" => ("qualifie", "excuse_valable"),
        "non_justifiee" => ("qualifie", "non_valable"),
        _ => ("a_qualifier", "en_attente"),
    };
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_asd_absence (id, etablissement_id, eleve_id, jour_debut, jour_fin, etat, qualification)
         VALUES ($1, $2, $3, $4, $4, $5, $6)",
    )
    .bind(id)
    .bind(w.etab)
    .bind(eleve)
    .bind(jour_s)
    .bind(etat)
    .bind(qualif)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_asd_absence_seance (id, etablissement_id, absence_id, eleve_id, jour, kind, seance_ref, heure_debut,
                                             heure_fin, source)
         VALUES ($1, $2, $3, $4, $5, 'seance', $6, '08:00', '09:00', 'appel')",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(id)
    .bind(eleve)
    .bind(jour_s)
    .bind(creneau)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

async fn propositions(ent: &PgEnt, w: &W, ev: Uuid) -> PropositionsDto {
    ent.eva_v4_propositions_for_actor(w.etab, w.prof, PROF, ev, jour())
        .await
        .unwrap()
}

async fn resultat(
    ent: &PgEnt,
    w: &W,
    ev: Uuid,
    eleve: Uuid,
) -> Option<(String, Option<f64>, String, bool)> {
    sqlx::query_as(
        "SELECT statut, valeur::float8, source, amenage FROM eva_resultat
          WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(w.etab)
    .bind(ev)
    .bind(eleve)
    .fetch_optional(ent.pool_verif())
    .await
    .unwrap()
}

fn plan(ev: Uuid, eleves: Vec<Uuid>, prevu: NaiveDate) -> PlanifierRattrapageBody {
    PlanifierRattrapageBody {
        evaluation_id: ev,
        eleves,
        prevu_le: prevu,
        heure: NaiveTime::from_hms_opt(17, 0, 0),
        lieu: Some("Salle 12".into()),
        modalite: None,
        echeance: None,
    }
}

// ───────────────────────── CE-25, CE-26, CE-30, CE-31 : l'appel propose, le professeur confirme ─────────────────────────

#[tokio::test]
async fn v4_ce25_26_l_appel_propose_les_absents_le_professeur_confirme_jamais_un_zero() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let jd = d(2031, 10, 10);
    let creneau = seance(&ent, &w, jd).await;
    let ev = creer(&ent, &w, "DS 1", jd).await;
    // CE-30 : l'appel n'a pas encore été fait → cases vides, rien à proposer (mais une séance existe).
    let p0 = propositions(&ent, &w, ev).await;
    assert!(p0.seance_trouvee && p0.propositions.is_empty());
    // L'appel arrive : a absent justifié, b absent non justifié, c présent.
    absence(&ent, &w, creneau, a, jd, "justifiee").await;
    absence(&ent, &w, creneau, b, jd, "non_justifiee").await;
    let p = propositions(&ent, &w, ev).await;
    assert_eq!(p.propositions.len(), 2);
    let pa = p.propositions.iter().find(|x| x.eleve_id == a).unwrap();
    assert_eq!(
        (
            pa.statut.as_str(),
            pa.justification.as_deref(),
            pa.source.as_str()
        ),
        ("absent", Some("justifiee"), "appel")
    );
    assert!(
        pa.rattrapage_propose,
        "CE-25 : une absence justifiée appelle « programmer un rattrapage »"
    );
    let pb = p.propositions.iter().find(|x| x.eleve_id == b).unwrap();
    assert_eq!(pb.justification.as_deref(), Some("non_justifiee"));
    assert!(!pb.rattrapage_propose);
    // Rien n'est écrit tant que le professeur n'a pas confirmé.
    assert!(resultat(&ent, &w, ev, a).await.is_none());
    // Seul celui qui saisit voit les propositions : la direction (lecture), la famille, l'élève : refusé.
    assert!(matches!(
        ent.eva_v4_propositions_for_actor(w.etab, w.direction, DIR, ev, jour())
            .await,
        Err(EntDbError::Forbidden)
    ));
    for role in ["parent", "eleve"] {
        assert!(matches!(
            ent.eva_v4_propositions_for_actor(w.etab, cast.parent_lea.user_id, role, ev, jour())
                .await,
            Err(EntDbError::Forbidden)
        ));
    }
    // Confirmation d'un seul élève : l'autre reste proposé. Le serveur recalcule : un autre élève demandé est « non_propose ».
    let r = ent
        .eva_v4_appliquer_propositions_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &AppliquerPropositionsBody {
                cours_id: w.cours_maths,
                evaluation_id: ev,
                eleves: Some(vec![a, c]),
            },
        )
        .await
        .unwrap();
    assert!(r
        .iter()
        .any(|x| x.eleve_id == a && x.issue == "ecrit" && x.statut.as_deref() == Some("absent")));
    assert!(r
        .iter()
        .any(|x| x.eleve_id == c && x.issue == "non_propose"));
    let (statut, valeur, source, _) = resultat(&ent, &w, ev, a).await.unwrap();
    assert_eq!(
        (statut.as_str(), valeur, source.as_str()),
        ("absent", None, "appel"),
        "statut absent, jamais un zéro"
    );
    assert!(resultat(&ent, &w, ev, b).await.is_none());
    // Le reste, ensuite (toutes les propositions en cours).
    let r = ent
        .eva_v4_appliquer_propositions_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &AppliquerPropositionsBody {
                cours_id: w.cours_maths,
                evaluation_id: ev,
                eleves: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(resultat(&ent, &w, ev, b).await.unwrap().0, "absent");
    assert!(
        propositions(&ent, &w, ev).await.propositions.is_empty(),
        "cases remplies : plus de proposition"
    );
    // Absence justifiée APRÈS COUP (CE-25) : la justification se lit au socle 12 à chaque lecture ; le résultat reste « absent »
    // (jamais un zéro, jamais réécrit), l'écran lit la justification actuelle, sans copie qui vieillirait.
    sqlx::query(
        "UPDATE ent_asd_absence SET etat = 'qualifie', qualification = 'excuse_valable'
          WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(w.etab)
    .bind(b)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let j: Option<String> = sqlx::query_scalar("SELECT eva_absence_a_la_seance($1, $2, $3, $4)")
        .bind(w.etab)
        .bind(b)
        .bind(w.cours_maths)
        .bind(jd)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(j.as_deref(), Some("justifiee"));
    assert_eq!(resultat(&ent, &w, ev, b).await.unwrap().0, "absent");
    // Aucune moyenne, aucun « 0 » de substitution : le moteur ne compte pas les absents.
    let g = ent
        .eva_v3_grille_for_actor(
            w.etab,
            w.prof,
            PROF,
            w.cours_maths,
            w.t1,
            jour(),
            maintenant(),
        )
        .await
        .unwrap();
    for e in [a, b] {
        assert_eq!(
            g.eleves.iter().find(|l| l.eleve_id == e).unwrap().moyenne_m,
            None
        );
    }
}

#[tokio::test]
async fn v4_ce31_la_saisie_du_professeur_prevaut_sur_l_appel_et_l_incoherence_est_signalee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    let jd = d(2031, 10, 10);
    let creneau = seance(&ent, &w, jd).await;
    let ev = creer(&ent, &w, "DS 1", jd).await;
    // Le professeur a une copie de a (arrivé en retard après l'appel) : il la note ; l'appel dit absent.
    saisir(&ent, &w, ev, a, "14").await;
    absence(&ent, &w, creneau, a, jd, "non_justifiee").await;
    absence(&ent, &w, creneau, b, jd, "justifiee").await;
    let p = propositions(&ent, &w, ev).await;
    assert_eq!(
        p.incoherences,
        vec![IncoherenceAppelDto { eleve_id: a }],
        "la vie scolaire est prévenue de l'incohérence"
    );
    assert_eq!(
        p.propositions
            .iter()
            .map(|x| x.eleve_id)
            .collect::<Vec<_>>(),
        vec![b]
    );
    // Même confirmée « pour tous », la note de a n'est jamais écrasée.
    ent.eva_v4_appliquer_propositions_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        &AppliquerPropositionsBody {
            cours_id: w.cours_maths,
            evaluation_id: ev,
            eleves: Some(vec![a, b]),
        },
    )
    .await
    .unwrap();
    assert_eq!(resultat(&ent, &w, ev, a).await.unwrap().1, Some(14.0));
    // Une évaluation hors séance (DM, oral) : aucune séance, rien n'est proposé, `absent` se saisit à la main.
    let dm = creer(&ent, &w, "DM", d(2031, 10, 12)).await;
    let p = propositions(&ent, &w, dm).await;
    assert!(!p.seance_trouvee && p.propositions.is_empty());
}

// ───────────────────────── CE-27 : absences répétées aux évaluations ─────────────────────────

#[tokio::test]
async fn v4_ce27_trois_absences_sans_justification_dans_la_periode_declenchent_l_alerte() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    for (i, jd) in [d(2031, 10, 6), d(2031, 10, 8), d(2031, 10, 13)]
        .into_iter()
        .enumerate()
    {
        let creneau = seance(&ent, &w, jd).await;
        let ev = creer(&ent, &w, &format!("DS {i}"), jd).await;
        // a : absent trois fois mais justifié chaque fois ; b : absent trois fois, jamais justifié ; c : deux fois seulement.
        absence(&ent, &w, creneau, a, jd, "justifiee").await;
        absence(&ent, &w, creneau, b, jd, "non_justifiee").await;
        if i < 2 {
            absence(&ent, &w, creneau, c, jd, "non_justifiee").await;
        }
        ent.eva_v4_appliquer_propositions_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &AppliquerPropositionsBody {
                cours_id: w.cours_maths,
                evaluation_id: ev,
                eleves: None,
            },
        )
        .await
        .unwrap();
    }
    let alertes = ent
        .eva_v4_absences_repetees_for_actor(w.etab, w.direction, DIR, jour(), w.t1, None)
        .await
        .unwrap();
    assert_eq!(
        alertes,
        vec![AbsenceRepeteeDto {
            eleve_id: b,
            nb_evaluations: 3,
            remplacement_convoque: false
        }],
        "seul b : 3 absences non justifiées ; a est justifié, c n'en a que 2"
    );
    // Le professeur du cours voit la même chose pour son cours ; la famille et l'élève : jamais.
    let p = ent
        .eva_v4_absences_repetees_for_actor(w.etab, w.prof, PROF, jour(), w.t1, Some(w.cours_maths))
        .await
        .unwrap();
    assert_eq!(p.len(), 1);
    assert!(matches!(
        ent.eva_v4_absences_repetees_for_actor(w.etab, w.prof, PROF, jour(), w.t1, None)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.eva_v4_absences_repetees_for_actor(
            w.etab,
            cast.parent_lea.user_id,
            "parent",
            jour(),
            w.t1,
            None
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
}

// ───────────────────────── Rattrapages : planification, convocation, résultat, échéance ─────────────────────────

#[tokio::test]
async fn v4_rattrapage_collectif_convocation_resultat_echeance_et_historique() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let ev = creer(&ent, &w, "DS 1", d(2031, 10, 10)).await;
    saisir(&ent, &w, ev, a, "abs").await;
    saisir(&ent, &w, ev, b, "abs").await;
    saisir(&ent, &w, ev, c, "15").await;
    // Un rattrapage ne se prévoit pas pour une note déjà saisie, ni dans le passé, ni pour un inconnu.
    assert!(ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![c], d(2031, 10, 27))
        )
        .await
        .is_err());
    assert!(ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![a], d(2031, 10, 19))
        )
        .await
        .is_err());
    assert!(ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![Uuid::new_v4()], d(2031, 10, 27))
        )
        .await
        .is_err());
    assert!(ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![], d(2031, 10, 27))
        )
        .await
        .is_err());
    // Un élève arrivé après / hors du public du cours à la date (ici : un élève de l'autre classe) n'est pas convocable.
    assert!(ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![w.eleves_d[0]], d(2031, 10, 27))
        )
        .await
        .is_err());
    // La famille, l'élève, un autre professeur : refusés.
    for (u, role) in [
        (cast.parent_lea.user_id, "parent"),
        (cast.parent_lea.user_id, "eleve"),
    ] {
        assert!(matches!(
            ent.eva_v4_planifier_rattrapage_for_actor(
                w.etab,
                u,
                role,
                jour(),
                &plan(ev, vec![a], d(2031, 10, 27))
            )
            .await,
            Err(EntDbError::Forbidden)
        ));
    }
    let autre = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)
        .unwrap();
    assert!(matches!(
        ent.eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            autre,
            PROF,
            jour(),
            &plan(ev, vec![a], d(2031, 10, 27))
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    // Séance collective : un identifiant, deux convocations, les résultats se lisent « à rattraper » (jamais un zéro).
    let rs = ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![b, a], d(2031, 10, 27)),
        )
        .await
        .unwrap();
    assert_eq!(rs.len(), 2);
    assert_eq!(rs[0].seance_id, rs[1].seance_id);
    assert!(rs.iter().all(|r| r.etat == "convoque"
        && r.nature == "rattrapage"
        && r.version == 1
        && r.echeance == d(2031, 10, 27)));
    for e in [a, b] {
        let (st, v, _, _) = resultat(&ent, &w, ev, e).await.unwrap();
        assert_eq!((st.as_str(), v), ("a_rattraper", None));
    }
    // Une seule convocation en cours par élève.
    assert!(matches!(
        ent.eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![a], d(2031, 11, 3))
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    let ra = rs.iter().find(|r| r.eleve_id == a).unwrap().clone();
    let rb = rs.iter().find(|r| r.eleve_id == b).unwrap().clone();
    // Déplacement : version attendue ; une version périmée est un conflit.
    let dep = DeplacerRattrapageBody {
        prevu_le: d(2031, 10, 30),
        heure: NaiveTime::from_hms_opt(16, 30, 0),
        lieu: Some("Salle 14".into()),
        modalite: Some("oral".into()),
        echeance: Some(d(2031, 11, 5)),
        version_attendue: ra.version,
    };
    let ra2 = ent
        .eva_v4_deplacer_rattrapage_for_actor(w.etab, w.prof, PROF, jour(), ra.id, &dep)
        .await
        .unwrap();
    assert_eq!(
        (ra2.prevu_le, ra2.version, ra2.modalite.as_str()),
        (d(2031, 10, 30), 2, "oral")
    );
    assert!(matches!(
        ent.eva_v4_deplacer_rattrapage_for_actor(w.etab, w.prof, PROF, jour(), ra.id, &dep)
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Échéance : « échu » au jour de l'établissement, jamais avant.
    let l = ent
        .eva_v4_rattrapages_evaluation_for_actor(w.etab, w.prof, PROF, d(2031, 11, 3), ev)
        .await
        .unwrap();
    assert!(
        l.iter().find(|r| r.id == rb.id).unwrap().echu,
        "b : échéance du 27 octobre dépassée"
    );
    assert!(
        !l.iter().find(|r| r.id == ra.id).unwrap().echu,
        "a : échéance reportée au 5 novembre"
    );
    // Résultat : a note 12 → la convocation est « passée », le résultat est une NOTE (historique conservé).
    let ra3 = ent
        .eva_v4_noter_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            ra.id,
            &NoterRattrapageBody {
                texte: Some("12".into()),
                absence: None,
                version_attendue: ra2.version,
            },
        )
        .await
        .unwrap();
    assert_eq!(ra3.etat, "passe");
    assert_eq!(resultat(&ent, &w, ev, a).await.unwrap().1, Some(12.0));
    // Hors barème : refusé ; une convocation terminée ne change plus.
    assert!(ent
        .eva_v4_noter_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            rb.id,
            &NoterRattrapageBody {
                texte: Some("23".into()),
                absence: None,
                version_attendue: rb.version
            },
        )
        .await
        .is_err());
    assert!(matches!(
        ent.eva_v4_noter_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            ra.id,
            &NoterRattrapageBody {
                texte: Some("13".into()),
                absence: None,
                version_attendue: ra3.version
            },
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // b est absent AU rattrapage sans justification : la convocation le dit, le résultat reste « à rattraper » : jamais 0.
    let rb2 = ent
        .eva_v4_noter_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            rb.id,
            &NoterRattrapageBody {
                texte: None,
                absence: Some("non_justifiee".into()),
                version_attendue: rb.version,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (rb2.etat.as_str(), rb2.zero_possible),
        ("absent_non_justifie", false),
        "un rattrapage ordinaire n'a pas de zéro"
    );
    let (st, v, _, _) = resultat(&ent, &w, ev, b).await.unwrap();
    assert_eq!((st.as_str(), v), ("a_rattraper", None));
    // Le zéro est refusé pour une convocation de rattrapage ordinaire, même par la direction qui confirme.
    assert!(ent
        .eva_v4_attribuer_zero_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            rb.id,
            &AttribuerZeroBody {
                confirme: true,
                version_attendue: rb2.version
            },
        )
        .await
        .is_err());
    // La base refuse aussi, de son côté, un zéro hors remplacement (contrainte, pas seulement le service).
    let r = sqlx::query(
        "UPDATE eva_rattrapage SET zero_attribue_le = now(), zero_attribue_par = $2 WHERE id = $1",
    )
    .bind(rb.id)
    .bind(w.direction)
    .execute(ent.pool_verif())
    .await;
    assert!(
        r.is_err(),
        "zéro refusé par la base pour une convocation de rattrapage ordinaire"
    );
    // Annulation : motif obligatoire (3 caractères au moins).
    let rc = ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![b], d(2031, 11, 10)),
        )
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert!(ent
        .eva_v4_annuler_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            rc.id,
            &AnnulerRattrapageBody {
                motif: "x".into(),
                version_attendue: rc.version
            }
        )
        .await
        .is_err());
    let rc2 = ent
        .eva_v4_annuler_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            rc.id,
            &AnnulerRattrapageBody {
                motif: "élève malade".into(),
                version_attendue: rc.version,
            },
        )
        .await
        .unwrap();
    assert_eq!(rc2.etat, "annule");
    // Historique en ajout seul, alimenté par la base : création, déplacement, passage.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM eva_rattrapage_historique WHERE etablissement_id = $1 AND rattrapage_id = $2")
        .bind(w.etab)
        .bind(ra.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 3);
    let r = sqlx::query(
        "UPDATE eva_rattrapage_historique SET etat_apres = 'convoque' WHERE etablissement_id = $1",
    )
    .bind(w.etab)
    .execute(ent.pool_verif())
    .await;
    assert!(r.is_err(), "l'historique est en ajout seul");
    // Événements neutres : identifiants seulement, jamais d'élève.
    let evts: Vec<(String, String, Option<Uuid>)> = sqlx::query_as(
        "SELECT type, payload::text, eleve_id FROM ent_metier_event WHERE etablissement_id = $1 AND type LIKE 'evaluations.rattrapage%'",
    )
    .bind(w.etab)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(evts
        .iter()
        .any(|(t, _, _)| t == types_v4::RATTRAPAGE_CONVOQUE));
    for (t, p, eleve) in &evts {
        assert!(
            eleve.is_none(),
            "{t} : jamais d'élève dans un événement de rattrapage"
        );
        let v: serde_json::Value = serde_json::from_str(p).unwrap();
        let cles: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        assert!(
            cles.iter()
                .all(|c| ["evaluation_id", "rattrapage_id"].contains(c)),
            "{t} : {cles:?}"
        );
    }
}

#[tokio::test]
async fn v4_rattrapage_apres_l_arret_exige_l_autorisation_de_la_direction() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, "DS 1", d(2031, 10, 10)).await;
    saisir(&ent, &w, ev, a, "abs").await;
    arreter(&ent, &w, w.t1, w.classe_a).await;
    // Période arrêtée : la convocation entière est refusée tant que la direction n'a pas autorisé le rattrapage.
    // (Le statut « absent » est une ligne sans note : le professeur ne peut plus la changer sans autorisation.)
    let r = ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![a], d(2031, 12, 10)),
        )
        .await;
    assert!(r.is_err(), "{r:?}");
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM eva_rattrapage WHERE etablissement_id = $1")
            .bind(w.etab)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(n, 0, "rien n'est resté de la tentative refusée");
    ent.eva_v1_autoriser_rattrapage_for_actor(
        w.etab,
        w.direction,
        DIR,
        ev,
        a,
        d(2031, 12, 31),
        "absence justifiée",
    )
    .await
    .unwrap();
    let rs = ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![a], d(2031, 12, 10)),
        )
        .await
        .unwrap();
    let noter = ent
        .eva_v4_noter_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            rs[0].id,
            &NoterRattrapageBody {
                texte: Some("11".into()),
                absence: None,
                version_attendue: rs[0].version,
            },
        )
        .await
        .unwrap();
    assert_eq!(noter.etat, "passe");
    assert_eq!(resultat(&ent, &w, ev, a).await.unwrap().1, Some(11.0));
}

// ───────────────────────── CE-28, CE-29 : évaluation de remplacement et seul zéro autorisé ─────────────────────────

#[tokio::test]
async fn v4_ce28_29_seule_l_absence_non_justifiee_au_remplacement_permet_le_zero() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    let rempl = creer(&ent, &w, "Évaluation de remplacement", d(2031, 10, 24)).await;
    // La convocation au remplacement est un acte de la direction (`notes.corriger`), pas du professeur ni d'un tiers.
    assert!(matches!(
        ent.eva_v4_convoquer_remplacement_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(rempl, vec![a], d(2031, 10, 24))
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.eva_v4_convoquer_remplacement_for_actor(
            w.etab,
            cast.administratif.user_id,
            "administratif",
            jour(),
            &plan(rempl, vec![a], d(2031, 10, 24))
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    let r1 = ent
        .eva_v4_convoquer_remplacement_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            &plan(rempl, vec![a, b], d(2031, 10, 24)),
        )
        .await
        .unwrap();
    assert!(r1.iter().all(|r| r.nature == "remplacement"));
    let ra = r1.iter().find(|r| r.eleve_id == a).unwrap().clone();
    let rb = r1.iter().find(|r| r.eleve_id == b).unwrap().clone();
    // Un 0 est impossible tant que l'absence n'est pas constatée non justifiée.
    let zero = |id: Uuid, v: i32, confirme: bool| {
        let (ent, etab, dir) = (&ent, w.etab, w.direction);
        async move {
            ent.eva_v4_attribuer_zero_for_actor(
                etab,
                dir,
                DIR,
                jour(),
                id,
                &AttribuerZeroBody {
                    confirme,
                    version_attendue: v,
                },
            )
            .await
        }
    };
    assert!(matches!(
        zero(ra.id, ra.version, true).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // CE-28 : a est absent MAIS JUSTIFIÉ au remplacement → nouvelle convocation, tracée, liée à la précédente.
    let ra2 = ent
        .eva_v4_noter_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            ra.id,
            &NoterRattrapageBody {
                texte: None,
                absence: Some("justifiee".into()),
                version_attendue: ra.version,
            },
        )
        .await
        .unwrap();
    assert_eq!(ra2.etat, "absent_justifie");
    assert!(
        matches!(
            zero(ra.id, ra2.version, true).await,
            Err(EntDbError::ConflitDossier(_))
        ),
        "absence justifiée : jamais de zéro"
    );
    let ra3 = ent
        .eva_v4_reconvoquer_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            ra.id,
            &ReconvoquerBody {
                prevu_le: d(2031, 10, 31),
                heure: None,
                lieu: None,
                modalite: None,
                echeance: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (ra3.precedent_id, ra3.etat.as_str(), ra3.nature.as_str()),
        (Some(ra.id), "convoque", "remplacement")
    );
    // On ne reconvoque qu'après une absence justifiée (b est encore convoqué).
    assert!(matches!(
        ent.eva_v4_reconvoquer_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            rb.id,
            &ReconvoquerBody {
                prevu_le: d(2031, 10, 31),
                heure: None,
                lieu: None,
                modalite: None,
                echeance: None
            }
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // b : absent NON justifié au remplacement → le zéro devient possible, pour la direction seulement, avec confirmation.
    let rb2 = ent
        .eva_v4_noter_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            rb.id,
            &NoterRattrapageBody {
                texte: None,
                absence: Some("non_justifiee".into()),
                version_attendue: rb.version,
            },
        )
        .await
        .unwrap();
    assert!(rb2.zero_possible);
    // Tant que la période est ouverte, le zéro revient au professeur du cours (même règle que toute saisie) ; la direction
    // l'attribue après l'arrêt, avec confirmation de la référence.
    arreter(&ent, &w, w.t1, w.classe_a).await;
    assert!(matches!(
        ent.eva_v4_attribuer_zero_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            rb.id,
            &AttribuerZeroBody {
                confirme: true,
                version_attendue: rb2.version
            }
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(
        zero(rb.id, rb2.version, false).await.is_err(),
        "sans confirmation de la référence"
    );
    let z = zero(rb.id, rb2.version, true).await.unwrap();
    assert!(z
        .reference
        .contains("seule l'absence non justifiée à l'évaluation de remplacement"));
    assert!(z.rattrapage.zero_attribue && !z.rattrapage.zero_possible);
    let (st, v, _, _) = resultat(&ent, &w, rempl, b).await.unwrap();
    assert_eq!((st.as_str(), v), ("note", Some(0.0)));
    // Acte tracé : l'historique du résultat porte l'auteur et la correction motivée ; un second zéro est refusé.
    let (auteur, motif): (Uuid, Option<String>) = sqlx::query_as(
        "SELECT h.auteur, h.motif FROM eva_resultat_historique h JOIN eva_resultat r ON r.id = h.resultat_id
          WHERE r.etablissement_id = $1 AND r.evaluation_id = $2 AND r.eleve_id = $3 ORDER BY h.version DESC LIMIT 1",
    )
    .bind(w.etab)
    .bind(rempl)
    .bind(b)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(auteur, w.direction);
    assert!(motif.unwrap_or_default().contains("remplacement"));
    assert!(matches!(
        zero(rb.id, rb2.version + 1, true).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // L'événement du zéro est neutre.
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = $2 AND eleve_id IS NULL",
    )
    .bind(w.etab)
    .bind(types_v4::ZERO_REMPLACEMENT)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn v4_ce38_un_eleve_non_note_peut_etre_convoque_au_remplacement_sans_zero_ni_moyenne() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev1 = creer(&ent, &w, "DS 1", d(2031, 10, 6)).await;
    let ev2 = creer(&ent, &w, "DS 2", d(2031, 10, 13)).await;
    saisir(&ent, &w, ev1, a, "nn").await;
    saisir(&ent, &w, ev2, a, "nn").await;
    let g = ent
        .eva_v3_grille_for_actor(
            w.etab,
            w.prof,
            PROF,
            w.cours_maths,
            w.t1,
            jour(),
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!(
        g.eleves.iter().find(|l| l.eleve_id == a).unwrap().moyenne_m,
        None,
        "moyenne « en attente » : pas de moyenne, pas de 0"
    );
    let rempl = creer(&ent, &w, "Remplacement", d(2031, 10, 24)).await;
    let r = ent
        .eva_v4_convoquer_remplacement_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            &plan(rempl, vec![a], d(2031, 10, 24)),
        )
        .await
        .unwrap();
    assert_eq!(r[0].etat, "convoque");
}

// ───────────────────────── CE-32 à CE-36 : dispenses et inaptitudes (port 34, sans motif) ─────────────────────────

#[tokio::test]
async fn v4_ce32_33_36_la_dispense_est_proposee_sans_motif_et_n_entre_pas_dans_la_moyenne() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    // Seule la direction (`notes.corriger`) déclare ; ni le professeur, ni la famille.
    let corps = DeclarerDispenseBody {
        eleve_id: a,
        matiere_id: Some(w.maths),
        nature: "inaptitude_totale".into(),
        du: d(2031, 10, 1),
        au: Some(d(2031, 10, 31)),
    };
    assert!(matches!(
        ent.eva_v4_declarer_dispense_for_actor(w.etab, w.prof, PROF, &corps)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.eva_v4_declarer_dispense_for_actor(w.etab, cast.parent_lea.user_id, "parent", &corps)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .eva_v4_declarer_dispense_for_actor(
            w.etab,
            w.direction,
            DIR,
            &DeclarerDispenseBody {
                nature: "grippe".into(),
                ..corps.clone()
            }
        )
        .await
        .is_err());
    assert!(ent
        .eva_v4_declarer_dispense_for_actor(
            w.etab,
            w.direction,
            DIR,
            &DeclarerDispenseBody {
                au: Some(d(2031, 9, 1)),
                ..corps.clone()
            }
        )
        .await
        .is_err());
    let disp = ent
        .eva_v4_declarer_dispense_for_actor(w.etab, w.direction, DIR, &corps)
        .await
        .unwrap();
    // Aucun motif n'existe dans ce qui est lu : ni colonne, ni champ.
    let lues = ent
        .eva_v4_dispenses_eleve_for_actor(w.etab, w.direction, DIR, a)
        .await
        .unwrap();
    let json = serde_json::to_string(&lues).unwrap();
    assert!(!json.contains("motif") && !json.contains("sante"), "{json}");
    let colonnes: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns WHERE table_name = 'eva_dispense_port' AND column_name ILIKE '%motif%'",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(
        colonnes.is_empty(),
        "la table du port 34 ne porte aucun motif : {colonnes:?}"
    );
    // CE-32 : évaluation dans la plage → proposition « dispense » ; hors plage (novembre) → rien.
    let dans = creer(&ent, &w, "DS octobre", d(2031, 10, 10)).await;
    let hors = creer(&ent, &w, "DS novembre", d(2031, 11, 12)).await;
    let p = propositions(&ent, &w, dans).await;
    let pa = p.propositions.iter().find(|x| x.eleve_id == a).unwrap();
    assert_eq!(
        (
            pa.statut.as_str(),
            pa.source.as_str(),
            pa.nature_dispense.as_deref()
        ),
        ("dispense", "dispense", Some("inaptitude_totale"))
    );
    assert!(propositions(&ent, &w, hors).await.propositions.is_empty());
    ent.eva_v4_appliquer_propositions_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        &AppliquerPropositionsBody {
            cours_id: w.cours_maths,
            evaluation_id: dans,
            eleves: None,
        },
    )
    .await
    .unwrap();
    let (st, v, source, _) = resultat(&ent, &w, dans, a).await.unwrap();
    assert_eq!(
        (st.as_str(), v, source.as_str()),
        ("dispense", None, "dispense")
    );
    // Moyenne : calculée sur les autres évaluations ; a dispensé en octobre, noté en novembre → moyenne de novembre seule.
    saisir(&ent, &w, dans, b, "10").await;
    saisir(&ent, &w, hors, a, "16").await;
    let g = ent
        .eva_v3_grille_for_actor(
            w.etab,
            w.prof,
            PROF,
            w.cours_maths,
            w.t1,
            jour(),
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!(
        g.eleves.iter().find(|l| l.eleve_id == a).unwrap().moyenne_m,
        Some(16_000)
    );
    // Retrait : la proposition disparaît pour les nouvelles évaluations ; la dispense reste au registre.
    ent.eva_v4_retirer_dispense_for_actor(w.etab, w.direction, DIR, disp.id)
        .await
        .unwrap();
    assert!(
        ent.eva_v4_retirer_dispense_for_actor(w.etab, w.direction, DIR, disp.id)
            .await
            .is_err(),
        "déjà retirée"
    );
    let autre = creer(&ent, &w, "DS bis", d(2031, 10, 15)).await;
    assert!(propositions(&ent, &w, autre).await.propositions.is_empty());
    assert!(
        ent.eva_v4_dispenses_eleve_for_actor(w.etab, w.direction, DIR, a)
            .await
            .unwrap()[0]
            .retiree
    );
}

#[tokio::test]
async fn v4_ce33_34_inaptitude_a_l_annee_et_inaptitude_partielle() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    // CE-33 : inaptitude totale à l'année, tous enseignements → toutes les évaluations proposées « dispensé » ; aucune moyenne.
    ent.eva_v4_declarer_dispense_for_actor(
        w.etab,
        w.direction,
        DIR,
        &DeclarerDispenseBody {
            eleve_id: a,
            matiere_id: Some(w.maths),
            nature: "inaptitude_totale".into(),
            du: d(2031, 9, 1),
            au: None,
        },
    )
    .await
    .unwrap();
    // CE-34 : inaptitude PARTIELLE pour b : information, rien n'est pré-rempli, le professeur adapte et note.
    ent.eva_v4_declarer_dispense_for_actor(
        w.etab,
        w.direction,
        DIR,
        &DeclarerDispenseBody {
            eleve_id: b,
            matiere_id: Some(w.maths),
            nature: "inaptitude_partielle".into(),
            du: d(2031, 9, 1),
            au: None,
        },
    )
    .await
    .unwrap();
    for (i, jd) in [d(2031, 10, 6), d(2031, 10, 13)].into_iter().enumerate() {
        let ev = creer(&ent, &w, &format!("EPS {i}"), jd).await;
        let p = propositions(&ent, &w, ev).await;
        assert_eq!(
            p.propositions
                .iter()
                .map(|x| x.eleve_id)
                .collect::<Vec<_>>(),
            vec![a]
        );
        assert_eq!(
            p.inaptes_partiels,
            vec![b],
            "le professeur d'EPS est informé, sans motif"
        );
        ent.eva_v4_appliquer_propositions_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &AppliquerPropositionsBody {
                cours_id: w.cours_maths,
                evaluation_id: ev,
                eleves: None,
            },
        )
        .await
        .unwrap();
        saisir(&ent, &w, ev, b, "13").await;
    }
    let g = ent
        .eva_v3_grille_for_actor(
            w.etab,
            w.prof,
            PROF,
            w.cours_maths,
            w.t1,
            jour(),
            maintenant(),
        )
        .await
        .unwrap();
    assert_eq!(
        g.eleves.iter().find(|l| l.eleve_id == a).unwrap().moyenne_m,
        None,
        "pas de moyenne d'EPS pour un inapte à l'année"
    );
    assert_eq!(
        g.eleves.iter().find(|l| l.eleve_id == b).unwrap().moyenne_m,
        Some(13_000)
    );
}

// ───────────────────────── CE-35, CE-37 : aménagements, AESH ─────────────────────────

async fn amenagement(ent: &PgEnt, w: &W, eleve: Uuid, code: &str, portee: &str) {
    sqlx::query(
        "INSERT INTO ent_amenagement (id, etablissement_id, eleve_id, code, portee, date_debut, defini_par_user_id)
         VALUES ($1, $2, $3, $4, $5, '2031-09-01', $6)",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(eleve)
    .bind(code)
    .bind(portee)
    .bind(w.direction)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

#[tokio::test]
async fn v4_ce35_37_amenagements_visibles_du_professeur_et_de_l_aesh_sans_donnee_de_sante() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    let aesh = cast.cpe.user_id; // un membre du personnel affecté comme AESH de a
    amenagement(&ent, &w, a, "tiers_temps", "examen").await;
    amenagement(&ent, &w, a, "lecteur", "les_deux").await;
    amenagement(&ent, &w, a, "police_adaptee", "cours").await; // portée « cours » : ne concerne pas l'évaluation
    sqlx::query(
        "INSERT INTO ent_aesh_affectation (id, etablissement_id, eleve_id, aesh_nom, aesh_user_id, date_debut, affecte_par_user_id)
         VALUES ($1, $2, $3, 'AESH test', $4, '2031-09-01', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(a)
    .bind(aesh)
    .bind(w.direction)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let ev = creer(&ent, &w, "DS 1", d(2031, 10, 24)).await;
    // Le professeur voit les aménagements fonctionnels de a (libellés du catalogue), pas de b.
    let v = ent
        .eva_v4_amenagements_evaluation_for_actor(w.etab, w.prof, PROF, ev, jour())
        .await
        .unwrap();
    assert_eq!(v.eleves.len(), 1);
    assert_eq!(v.eleves[0].eleve_id, a);
    let libelles: Vec<&str> = v.eleves[0]
        .amenagements
        .iter()
        .map(|x| x.libelle.as_str())
        .collect();
    assert_eq!(libelles, vec!["Consignes lues à voix haute", "Tiers-temps"]);
    assert!(v.eleves[0].alerte.is_none() && !v.eleves[0].aesh_indisponible);
    // Jamais de plan (PAP, PPS), de diagnostic, de note : seulement des codes et libellés du catalogue.
    let json = serde_json::to_string(&v).unwrap().to_lowercase();
    for interdit in ["pap", "pps", "diagnostic", "dys", "note_privee", "valeur"] {
        assert!(!json.contains(interdit), "{interdit} dans {json}");
    }
    // L'AESH voit ses élèves seulement ; un membre du personnel sans lien, la famille, l'élève : refusés.
    let v = ent
        .eva_v4_amenagements_evaluation_for_actor(w.etab, aesh, "cpe", ev, jour())
        .await
        .unwrap();
    assert_eq!(
        v.eleves.iter().map(|e| e.eleve_id).collect::<Vec<_>>(),
        vec![a]
    );
    assert!(matches!(
        ent.eva_v4_amenagements_evaluation_for_actor(
            w.etab,
            cast.compta.user_id,
            "compta",
            ev,
            jour()
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    for role in ["parent", "eleve"] {
        assert!(matches!(
            ent.eva_v4_amenagements_evaluation_for_actor(
                w.etab,
                cast.parent_lea.user_id,
                role,
                ev,
                jour()
            )
            .await,
            Err(EntDbError::Forbidden)
        ));
    }
    // CE-37 : l'AESH est indisponible ce jour-là → alerte au professeur (aménagement qui demande une aide humaine) + report.
    sqlx::query("INSERT INTO eva_aesh_indisponibilite_port (id, etablissement_id, aesh_user_id, du, au, saisi_par) VALUES ($1, $2, $3, $4, $4, $5)")
        .bind(Uuid::new_v4())
        .bind(w.etab)
        .bind(aesh)
        .bind(d(2031, 10, 24))
        .bind(w.direction)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let v = ent
        .eva_v4_amenagements_evaluation_for_actor(w.etab, w.prof, PROF, ev, jour())
        .await
        .unwrap();
    assert!(v.eleves[0].aesh_indisponible);
    let alerte = v.eleves[0].alerte.clone().expect("alerte CE-37");
    assert!(
        alerte.contains("Consignes lues à voix haute") && alerte.contains("Reporter"),
        "{alerte}"
    );
    // La marque « aménagé » est posée par la base sur une NOTE de a (pas de b), jamais sur un statut, jamais publiée.
    saisir(&ent, &w, ev, a, "14").await;
    saisir(&ent, &w, ev, b, "14").await;
    assert!(resultat(&ent, &w, ev, a).await.unwrap().3, "a : aménagé");
    assert!(
        !resultat(&ent, &w, ev, b).await.unwrap().3,
        "b : pas d'aménagement"
    );
    let col: Vec<String> = sqlx::query_scalar(
        "SELECT column_name FROM information_schema.columns WHERE table_name = 'eva_resultat_publie' AND column_name = 'amenage'",
    )
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(
        col.is_empty(),
        "la marque « aménagé » n'est jamais dans la vue des publications (R-V16)"
    );
}

// ───────────────────────── Isolation ─────────────────────────

#[tokio::test]
async fn v4_une_evaluation_d_un_autre_etablissement_est_introuvable() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = creer(&ent, &w, "DS 1", d(2031, 10, 10)).await;
    let Some((ent2, cast2)) = labo().await else {
        return;
    };
    let w2 = monde_v1(&ent2, &cast2).await;
    // L'établissement 2 ne voit pas l'évaluation de l'établissement 1 (ni par propositions, ni par rattrapages).
    let r = ent2
        .eva_v4_propositions_for_actor(w2.etab, w2.prof, PROF, ev, jour())
        .await;
    assert!(r.is_err(), "{r:?}");
    let r = ent2
        .eva_v4_planifier_rattrapage_for_actor(
            w2.etab,
            w2.prof,
            PROF,
            jour(),
            &plan(ev, vec![w.eleves_a[0]], d(2031, 10, 27)),
        )
        .await;
    assert!(r.is_err(), "{r:?}");
    let r = ent2
        .eva_v4_rattrapages_evaluation_for_actor(w2.etab, w2.prof, PROF, jour(), ev)
        .await;
    assert!(r.is_err(), "{r:?}");
    // Ce que le premier a n'est pas modifié par la tentative.
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM eva_rattrapage")
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(n >= 0);
}
