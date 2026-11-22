//! Socle 21, lot V4 « absences, dispenses, aménagements, rattrapages » : tests sur base (feature `labo`), CE-25 à CE-38.
//! Aucune horloge réelle : jour fixe 2031-10-20 (monde V1 : année 2031-2032), dates d'évaluation fixes.

use super::*;
use crate::evaluations_v4::types_v4;
use crate::tests_evaluations::labo;
use crate::tests_evaluations_v1::{arreter, d, monde_v1, DIR, PROF, W};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use domain_eleve::DomainError;
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

/// Une évaluation **annoncée** aux élèves (seules celles-là existent pour un AESH, V3d F-8).
async fn creer_annoncee(ent: &PgEnt, w: &W, titre: &str, date: NaiveDate) -> Uuid {
    let ev = creer(ent, w, titre, date).await;
    sqlx::query(
        "UPDATE eva_evaluation SET annoncee = true WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.etab)
    .bind(ev)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    ev
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
    assert!(matches!(
        ent.eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![c], d(2031, 10, 27))
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![a], d(2031, 10, 19))
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![Uuid::new_v4()], d(2031, 10, 27))
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![], d(2031, 10, 27))
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Un élève arrivé après / hors du public du cours à la date (ici : un élève de l'autre classe) n'est pas convocable.
    assert!(matches!(
        ent.eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![w.eleves_d[0]], d(2031, 10, 27))
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
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
        motif: None,
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
                motif: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(ra3.etat, "passe");
    assert_eq!(resultat(&ent, &w, ev, a).await.unwrap().1, Some(12.0));
    // Hors barème : refusé ; une convocation terminée ne change plus.
    assert!(matches!(
        ent.eva_v4_noter_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            rb.id,
            &NoterRattrapageBody {
                texte: Some("23".into()),
                absence: None,
                version_attendue: rb.version,
                motif: None,
            },
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
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
                version_attendue: ra3.version,
                motif: None,
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
                motif: None,
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
    assert!(matches!(
        ent.eva_v4_attribuer_zero_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            rb.id,
            &AttribuerZeroBody {
                confirme: true,
                version_attendue: rb2.version,
                motif: None,
            },
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
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
    assert!(matches!(
        ent.eva_v4_annuler_rattrapage_for_actor(
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
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
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
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
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
                motif: None,
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
                    motif: None,
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
                motif: None,
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
                motif: None,
            },
        )
        .await
        .unwrap();
    assert!(rb2.zero_possible);
    // Tant que la période est ouverte, le zéro revient au professeur du cours (V3d, M-4) ; la direction l'attribue
    // après l'arrêt, avec confirmation de la référence. Le professeur, lui, n'attribue plus rien après l'arrêt.
    arreter(&ent, &w, w.t1, w.classe_a).await;
    for motif in [None, Some("Absent sans justificatif".to_string())] {
        let r = ent
            .eva_v4_attribuer_zero_for_actor(
                w.etab,
                w.prof,
                PROF,
                jour(),
                rb.id,
                &AttribuerZeroBody {
                    confirme: true,
                    version_attendue: rb2.version,
                    motif,
                },
            )
            .await;
        assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    }
    assert!(
        matches!(
            zero(rb.id, rb2.version, false).await,
            Err(EntDbError::SaisieInvalide(_))
        ),
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
    assert!(matches!(
        ent.eva_v4_declarer_dispense_for_actor(
            w.etab,
            w.direction,
            DIR,
            &DeclarerDispenseBody {
                nature: "grippe".into(),
                ..corps.clone()
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.eva_v4_declarer_dispense_for_actor(
            w.etab,
            w.direction,
            DIR,
            &DeclarerDispenseBody {
                au: Some(d(2031, 9, 1)),
                ..corps.clone()
            }
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
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
    ent.activer_module_sante_test(w.etab).await;
    let ev_non_annoncee = creer(&ent, &w, "DS non annoncé", d(2031, 10, 24)).await;
    let ev = creer_annoncee(&ent, &w, "DS 1", d(2031, 10, 24)).await;
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
    // F-8 : l'AESH ne connaît pas une évaluation que le professeur n'a pas annoncée.
    assert!(matches!(
        ent.eva_v4_amenagements_evaluation_for_actor(w.etab, aesh, "cpe", ev_non_annoncee, jour())
            .await,
        Err(EntDbError::Forbidden)
    ));
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
    assert!(
        matches!(r, Err(EntDbError::Domain(DomainError::NotFound))),
        "{r:?}"
    );
    let r = ent2
        .eva_v4_planifier_rattrapage_for_actor(
            w2.etab,
            w2.prof,
            PROF,
            jour(),
            &plan(ev, vec![w.eleves_a[0]], d(2031, 10, 27)),
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::Domain(DomainError::NotFound))),
        "{r:?}"
    );
    let r = ent2
        .eva_v4_rattrapages_evaluation_for_actor(w2.etab, w2.prof, PROF, jour(), ev)
        .await;
    assert!(
        matches!(r, Err(EntDbError::Domain(DomainError::NotFound))),
        "{r:?}"
    );
    // Ce que le premier a n'est pas modifié par la tentative : aucune convocation n'a été créée.
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM eva_rattrapage WHERE etablissement_id = $1 AND evaluation_id = $2",
    )
    .bind(w.etab)
    .bind(ev)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 0);
}

// ───────────────────────── V3d : revue de sécurité de V4 ─────────────────────────

/// Écrit une case **avec** la version lue (corrige un statut existant) et rend l'issue (« ecrit », « conflit »…).
async fn saisir_v(
    ent: &PgEnt,
    w: &W,
    ev: Uuid,
    eleve: Uuid,
    texte: &str,
    version: Option<i32>,
) -> String {
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
                    version_attendue: version,
                    motif: None,
                }],
                origine: None,
            },
        )
        .await
        .unwrap();
    r.cellules[0].issue.clone()
}

async fn version_case(ent: &PgEnt, w: &W, ev: Uuid, eleve: Uuid) -> i32 {
    sqlx::query_scalar(
        "SELECT version FROM eva_resultat WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(w.etab)
    .bind(ev)
    .bind(eleve)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap()
}

fn note_texte(texte: &str, version: i32) -> NoterRattrapageBody {
    NoterRattrapageBody {
        texte: Some(texte.into()),
        absence: None,
        version_attendue: version,
        motif: None,
    }
}

fn absence_corps(qualif: &str, version: i32) -> NoterRattrapageBody {
    NoterRattrapageBody {
        texte: None,
        absence: Some(qualif.into()),
        version_attendue: version,
        motif: None,
    }
}

/// M-1 : la note d'un rattrapage n'écrase ni une note tardive, ni une dispense saisies entre-temps ; une saisie concurrente
/// et la note du rattrapage ne réussissent jamais toutes les deux.
#[tokio::test]
async fn v3d_m1_la_note_d_un_rattrapage_n_ecrase_ni_une_note_tardive_ni_une_dispense() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let ev = creer(&ent, &w, "DS 1", d(2031, 10, 10)).await;
    for e in [a, b, c] {
        saisir(&ent, &w, ev, e, "abs").await;
    }
    let rs = ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![a, b, c], d(2031, 10, 27)),
        )
        .await
        .unwrap();
    let rat = |e: Uuid| rs.iter().find(|r| r.eleve_id == e).unwrap().clone();
    let noter = |e: Uuid, texte: &str| {
        let r = rat(e);
        let (ent, etab, prof) = (&ent, w.etab, w.prof);
        let texte = texte.to_string();
        async move {
            ent.eva_v4_noter_rattrapage_for_actor(
                etab,
                prof,
                PROF,
                jour(),
                r.id,
                &note_texte(&texte, r.version),
            )
            .await
        }
    };
    // a : une note tardive (15) arrive avant le rattrapage → conflit, la note tardive reste.
    let va = version_case(&ent, &w, ev, a).await;
    assert_eq!(saisir_v(&ent, &w, ev, a, "15", Some(va)).await, "ecrit");
    let r = noter(a, "12").await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    let (st, v, _, _) = resultat(&ent, &w, ev, a).await.unwrap();
    assert_eq!((st.as_str(), v), ("note", Some(15.0)));
    // b : une dispense arrive avant le rattrapage → conflit, la dispense reste.
    let vb = version_case(&ent, &w, ev, b).await;
    assert_eq!(saisir_v(&ent, &w, ev, b, "disp", Some(vb)).await, "ecrit");
    let r = noter(b, "12").await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    assert_eq!(resultat(&ent, &w, ev, b).await.unwrap().0, "dispense");
    // La convocation n'a pas été consommée par l'échec : elle est toujours « convoque ».
    let en_cours: String = sqlx::query_scalar(
        "SELECT etat FROM eva_rattrapage WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.etab)
    .bind(rat(b).id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(en_cours, "convoque");
    // c : case restée « à rattraper » → la note du rattrapage s'écrit (cas nominal).
    let rc = noter(c, "12").await.unwrap();
    assert_eq!(rc.etat, "passe");
    assert_eq!(resultat(&ent, &w, ev, c).await.unwrap().1, Some(12.0));
    // Concurrence : la note du rattrapage et une saisie de la grille sur la même case, en parallèle. Un seul gagne.
    let ev2 = creer(&ent, &w, "DS 2", d(2031, 10, 13)).await;
    saisir(&ent, &w, ev2, a, "abs").await;
    let r2 = ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev2, vec![a], d(2031, 10, 27)),
        )
        .await
        .unwrap()
        .remove(0);
    let v2 = version_case(&ent, &w, ev2, a).await;
    let corps_rattrapage = note_texte("12", r2.version);
    let (n, g) = tokio::join!(
        ent.eva_v4_noter_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            r2.id,
            &corps_rattrapage
        ),
        saisir_v(&ent, &w, ev2, a, "15", Some(v2))
    );
    let (n_ok, g_ok) = (n.is_ok(), g == "ecrit");
    assert!(
        n_ok ^ g_ok,
        "un seul des deux écrit : rattrapage {n:?}, grille {g}"
    );
    let (st, v, _, _) = resultat(&ent, &w, ev2, a).await.unwrap();
    assert_eq!(st, "note");
    assert_eq!(v, Some(if n_ok { 12.0 } else { 15.0 }));
}

/// M-1 (zéro) et M-4 / M-5 : le zéro n'écrase ni une note ni une dispense ; en période ouverte le professeur du cours
/// l'attribue (confirmation, motif, absence non justifiée constatée par l'assiduité) ; une justification arrivée après le
/// constat le bloque.
#[tokio::test]
async fn v3d_m4_m5_le_professeur_attribue_le_zero_en_periode_ouverte_et_l_assiduite_est_relue() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let jr = d(2031, 10, 24);
    let rempl = creer(&ent, &w, "Évaluation de remplacement", jr).await;
    let creneau = seance(&ent, &w, jr).await;
    // a et c : absents au remplacement, absence NON justifiée au socle 12 ; b : aucune absence consignée.
    absence(&ent, &w, creneau, a, jr, "non_justifiee").await;
    absence(&ent, &w, creneau, c, jr, "non_justifiee").await;
    let rs = ent
        .eva_v4_convoquer_remplacement_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            &plan(rempl, vec![a, b, c], jr),
        )
        .await
        .unwrap();
    let mut conv = std::collections::HashMap::new();
    for r in &rs {
        let r2 = ent
            .eva_v4_noter_rattrapage_for_actor(
                w.etab,
                w.prof,
                PROF,
                jour(),
                r.id,
                &absence_corps("non_justifiee", r.version),
            )
            .await
            .unwrap();
        conv.insert(r.eleve_id, r2);
    }
    let zero = |acteur: Uuid, role: &'static str, e: Uuid, confirme: bool, motif: Option<&str>| {
        let (ent, etab) = (&ent, w.etab);
        let r = conv[&e].clone();
        let motif = motif.map(str::to_string);
        async move {
            ent.eva_v4_attribuer_zero_for_actor(
                etab,
                acteur,
                role,
                jour(),
                r.id,
                &AttribuerZeroBody {
                    confirme,
                    version_attendue: r.version,
                    motif,
                },
            )
            .await
        }
    };
    // Sans confirmation, sans motif : refusés (le professeur n'a pas `notes.corriger`).
    let r = zero(w.prof, PROF, a, false, Some("Absent sans justificatif")).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    let r = zero(w.prof, PROF, a, true, None).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    // La direction, en période OUVERTE, ne saisit pas à la place du professeur : message exact.
    let r = zero(w.direction, DIR, a, true, None).await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("période de l'élève ouverte")),
        "{r:?}"
    );
    // b : l'assiduité ne constate aucune absence → le professeur n'attribue pas le zéro.
    let r = zero(w.prof, PROF, b, true, Some("Absent sans justificatif")).await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("constatée par l'assiduité")),
        "{r:?}"
    );
    assert!(resultat(&ent, &w, rempl, b).await.is_none());
    // c : absence justifiée APRÈS le constat → le zéro est bloqué (M-5) ; puis « en attente de qualification » aussi.
    sqlx::query(
        "UPDATE ent_asd_absence SET qualification = 'excuse_valable' WHERE etablissement_id = $1 AND eleve_id = $2 AND jour_debut = $3",
    )
    .bind(w.etab)
    .bind(c)
    .bind(jr)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let r = zero(w.prof, PROF, c, true, Some("Absent sans justificatif")).await;
    assert!(
        matches!(&r, Err(EntDbError::ConflitDossier(m)) if m.contains("justifiée depuis")),
        "{r:?}"
    );
    let r = zero(w.direction, DIR, c, true, Some("Absent sans justificatif")).await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    sqlx::query(
        "UPDATE ent_asd_absence SET etat = 'a_qualifier', qualification = 'en_attente' WHERE etablissement_id = $1 AND eleve_id = $2 AND jour_debut = $3",
    )
    .bind(w.etab)
    .bind(c)
    .bind(jr)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let r = zero(w.prof, PROF, c, true, Some("Absent sans justificatif")).await;
    assert!(
        matches!(&r, Err(EntDbError::ConflitDossier(m)) if m.contains("pas encore qualifiée")),
        "{r:?}"
    );
    assert!(
        resultat(&ent, &w, rempl, c).await.is_none(),
        "rien n'est écrit"
    );
    // a : le professeur du cours attribue le zéro (confirmation + motif + absence non justifiée constatée).
    let z = zero(w.prof, PROF, a, true, Some("Absent sans justificatif"))
        .await
        .unwrap();
    assert!(z.rattrapage.zero_attribue && !z.rattrapage.zero_possible);
    let (st, v, _, _) = resultat(&ent, &w, rempl, a).await.unwrap();
    assert_eq!((st.as_str(), v), ("note", Some(0.0)));
    let (auteur, motif): (Uuid, Option<String>) = sqlx::query_as(
        "SELECT h.auteur, h.motif FROM eva_resultat_historique h JOIN eva_resultat r ON r.id = h.resultat_id
          WHERE r.etablissement_id = $1 AND r.evaluation_id = $2 AND r.eleve_id = $3 ORDER BY h.version DESC LIMIT 1",
    )
    .bind(w.etab)
    .bind(rempl)
    .bind(a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(auteur, w.prof);
    assert!(
        motif
            .as_deref()
            .is_some_and(|m| m.contains("Absent sans justificatif") && m.contains("R-01")),
        "le motif de l'acteur voyage dans l'historique"
    );
    // Un autre professeur (sans lien avec le cours), la famille, l'élève : refusés.
    let autre = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)
        .unwrap();
    for (u, role) in [
        (autre, PROF),
        (cast.parent_lea.user_id, "parent"),
        (cast.parent_lea.user_id, "eleve"),
    ] {
        let r = zero(u, role, a, true, Some("Absent sans justificatif")).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
}

/// M-1 (zéro) : une note ou une dispense saisie après le constat n'est jamais écrasée par le zéro.
#[tokio::test]
async fn v3d_m1_le_zero_n_ecrase_ni_une_note_tardive_ni_une_dispense() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let jr = d(2031, 10, 24);
    let rempl = creer(&ent, &w, "Évaluation de remplacement", jr).await;
    let rs = ent
        .eva_v4_convoquer_remplacement_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            &plan(rempl, vec![a, b, c], jr),
        )
        .await
        .unwrap();
    let mut conv = std::collections::HashMap::new();
    for r in &rs {
        let r2 = ent
            .eva_v4_noter_rattrapage_for_actor(
                w.etab,
                w.prof,
                PROF,
                jour(),
                r.id,
                &absence_corps("non_justifiee", r.version),
            )
            .await
            .unwrap();
        conv.insert(r.eleve_id, r2);
    }
    // Entre le constat et le zéro : une note tardive pour a, une dispense pour b ; rien pour c.
    assert_eq!(saisir_v(&ent, &w, rempl, a, "14", None).await, "ecrit");
    assert_eq!(saisir_v(&ent, &w, rempl, b, "disp", None).await, "ecrit");
    arreter(&ent, &w, w.t1, w.classe_a).await;
    for e in [a, b] {
        let r = ent
            .eva_v4_attribuer_zero_for_actor(
                w.etab,
                w.direction,
                DIR,
                jour(),
                conv[&e].id,
                &AttribuerZeroBody {
                    confirme: true,
                    version_attendue: conv[&e].version,
                    motif: None,
                },
            )
            .await;
        assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    }
    let (st, v, _, _) = resultat(&ent, &w, rempl, a).await.unwrap();
    assert_eq!((st.as_str(), v), ("note", Some(14.0)));
    assert_eq!(resultat(&ent, &w, rempl, b).await.unwrap().0, "dispense");
    // Les zéros refusés n'ont rien marqué sur les convocations.
    let marques: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM eva_rattrapage WHERE etablissement_id = $1 AND evaluation_id = $2 AND zero_attribue_le IS NOT NULL",
    )
    .bind(w.etab)
    .bind(rempl)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(marques, 0);
    // c (case vide) : le zéro s'écrit (aucune assiduité consignée : le constat de la convocation suffit à la direction).
    let z = ent
        .eva_v4_attribuer_zero_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            conv[&c].id,
            &AttribuerZeroBody {
                confirme: true,
                version_attendue: conv[&c].version,
                motif: None,
            },
        )
        .await
        .unwrap();
    assert!(z.rattrapage.zero_attribue);
}

/// Branche le professeur remplaçant du labo comme suppléant en saisie du service de maths dès le 2031-10-15.
async fn suppleant_en_saisie(ent: &PgEnt, w: &W, cast: &crate::seed::LaboCast) -> Option<Uuid> {
    let remplacant = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)?;
    let service: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_service_enseignement WHERE etablissement_id = $1 AND cours_id = $2",
    )
    .bind(w.etab)
    .bind(w.cours_maths)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    let agent: Option<Uuid> = sqlx::query_scalar(
        "SELECT ag.id FROM ent_agent ag JOIN ent_personne pe ON pe.id = ag.personne_id AND pe.etablissement_id = ag.etablissement_id
          WHERE ag.etablissement_id = $1 AND pe.user_id = $2 AND ag.fusionne_dans_agent_id IS NULL",
    )
    .bind(w.etab)
    .bind(remplacant)
    .fetch_optional(ent.pool_verif())
    .await
    .unwrap();
    let agent = agent?;
    sqlx::query(
        "INSERT INTO ent_service_intervenant (id, etablissement_id, service_id, agent_id, nature, du, saisie, origine)
         VALUES ($1, $2, $3, $4, 'suppleant', '2031-10-15', true, 'saisie')",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(service)
    .bind(agent)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    Some(remplacant)
}

/// M-2 (CE-44) : le suppléant ne déplace ni n'annule la convocation du titulaire ; il gère les siennes ; la direction et le
/// titulaire gardent la main.
#[tokio::test]
async fn v3d_m2_le_suppleant_ne_deplace_ni_n_annule_la_convocation_du_titulaire() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    let ev = creer(&ent, &w, "DS 1", d(2031, 10, 10)).await;
    saisir(&ent, &w, ev, a, "abs").await;
    saisir(&ent, &w, ev, b, "abs").await;
    // Le titulaire convoque a ; PUIS un suppléant prend le service en saisie (dès le 15 octobre).
    let rt = ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![a], d(2031, 10, 27)),
        )
        .await
        .unwrap()
        .remove(0);
    let Some(remplacant) = suppleant_en_saisie(&ent, &w, &cast).await else {
        return eprintln!("skip: pas d'agent pour le second professeur du labo");
    };
    let deplacer = |acteur: Uuid, role: &'static str, id: Uuid, v: i32| {
        let ent = &ent;
        let etab = w.etab;
        async move {
            ent.eva_v4_deplacer_rattrapage_for_actor(
                etab,
                acteur,
                role,
                jour(),
                id,
                &DeplacerRattrapageBody {
                    prevu_le: d(2031, 10, 29),
                    heure: None,
                    lieu: None,
                    modalite: None,
                    echeance: None,
                    version_attendue: v,
                    motif: None,
                },
            )
            .await
        }
    };
    // Le suppléant ne touche pas la convocation du titulaire (ni déplacement, ni annulation).
    let r = deplacer(remplacant, PROF, rt.id, rt.version).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let r = ent
        .eva_v4_annuler_rattrapage_for_actor(
            w.etab,
            remplacant,
            PROF,
            jour(),
            rt.id,
            &AnnulerRattrapageBody {
                motif: "essai du suppléant".into(),
                version_attendue: rt.version,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Il gère SA convocation (b), saisie en son nom.
    let rs = ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            remplacant,
            PROF,
            jour(),
            &plan(ev, vec![b], d(2031, 10, 27)),
        )
        .await
        .unwrap()
        .remove(0);
    let rs = deplacer(remplacant, PROF, rs.id, rs.version).await.unwrap();
    assert_eq!(rs.prevu_le, d(2031, 10, 29));
    // La direction garde la main sur la convocation du titulaire (droit de correction).
    let rt = deplacer(w.direction, DIR, rt.id, rt.version).await.unwrap();
    assert_eq!(rt.prevu_le, d(2031, 10, 29));
    // Le titulaire de retour (fin de la suppléance) retrouve la main.
    sqlx::query("UPDATE ent_service_intervenant SET au = '2031-10-20' WHERE etablissement_id = $1 AND nature = 'suppleant'")
        .bind(w.etab)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let retour = ent
        .eva_v4_deplacer_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            d(2031, 10, 21),
            rt.id,
            &DeplacerRattrapageBody {
                prevu_le: d(2031, 10, 30),
                heure: None,
                lieu: None,
                modalite: None,
                echeance: None,
                version_attendue: rt.version,
                motif: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(retour.prevu_le, d(2031, 10, 30));
}

/// M-6 : élève protégé, élève d'un autre établissement, convocation d'un autre établissement.
#[tokio::test]
async fn v3d_m6_eleve_protege_et_convocation_d_un_autre_etablissement_sont_introuvables() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    let protege = w.eleves_a[2];
    let adm = cast.administratif.user_id;
    let ev = creer(&ent, &w, "DS 1", d(2031, 10, 10)).await;
    for e in [a, protege] {
        saisir(&ent, &w, ev, e, "abs").await;
    }
    let rs = ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![a, protege], d(2031, 10, 27)),
        )
        .await
        .unwrap();
    assert_eq!(rs.len(), 2);
    // Une mesure de confidentialité renforcée est posée ensuite sur l'un des deux élèves.
    sqlx::query(
        "INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, niveau, motif_scelle, du, decide_par)
         VALUES ($1, $2, $3, 'renforcee', 'enc:v1:mesure', '2020-01-01', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(protege)
    .bind(w.direction)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    // Le secrétariat (`notes.lire`) ne voit pas un élève protégé : « introuvable » (dispenses), absent des convocations.
    let r = ent
        .eva_v4_dispenses_eleve_for_actor(w.etab, adm, "administratif", protege)
        .await;
    assert!(
        matches!(r, Err(EntDbError::Domain(DomainError::NotFound))),
        "{r:?}"
    );
    assert!(ent
        .eva_v4_dispenses_eleve_for_actor(w.etab, adm, "administratif", a)
        .await
        .unwrap()
        .is_empty());
    let vus: Vec<Uuid> = ent
        .eva_v4_rattrapages_evaluation_for_actor(w.etab, adm, "administratif", jour(), ev)
        .await
        .unwrap()
        .iter()
        .map(|r| r.eleve_id)
        .collect();
    assert_eq!(
        vus,
        vec![a],
        "l'élève protégé n'apparaît pas pour le secrétariat"
    );
    let tous = ent
        .eva_v4_rattrapages_evaluation_for_actor(w.etab, w.direction, DIR, jour(), ev)
        .await
        .unwrap();
    assert_eq!(
        tous.len(),
        2,
        "la direction (accès aux protégés) les voit tous"
    );
    // Une convocation d'un autre établissement est introuvable, sous toutes ses formes ; la sienne n'est pas touchée.
    let Some((ent2, cast2)) = labo().await else {
        return;
    };
    let w2 = monde_v1(&ent2, &cast2).await;
    let rc = rs.iter().find(|r| r.eleve_id == a).unwrap().clone();
    // V3e (item 6, règle 5) : sans `notes.corriger`, une convocation d'un autre établissement répond comme un droit refusé
    // (aucun oracle d'existence) ; seule la direction (`notes.corriger`) apprend qu'elle est « introuvable ».
    fn refusee<T: std::fmt::Debug>(r: Result<T, EntDbError>) {
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    }
    fn introuvable<T: std::fmt::Debug>(r: Result<T, EntDbError>) {
        assert!(
            matches!(r, Err(EntDbError::Domain(DomainError::NotFound))),
            "{r:?}"
        )
    }
    refusee(
        ent2.eva_v4_deplacer_rattrapage_for_actor(
            w2.etab,
            w2.prof,
            PROF,
            jour(),
            rc.id,
            &DeplacerRattrapageBody {
                prevu_le: d(2031, 10, 29),
                heure: None,
                lieu: None,
                modalite: None,
                echeance: None,
                version_attendue: rc.version,
                motif: None,
            },
        )
        .await,
    );
    refusee(
        ent2.eva_v4_annuler_rattrapage_for_actor(
            w2.etab,
            w2.prof,
            PROF,
            jour(),
            rc.id,
            &AnnulerRattrapageBody {
                motif: "tentative extérieure".into(),
                version_attendue: rc.version,
            },
        )
        .await,
    );
    refusee(
        ent2.eva_v4_noter_rattrapage_for_actor(
            w2.etab,
            w2.prof,
            PROF,
            jour(),
            rc.id,
            &note_texte("12", rc.version),
        )
        .await,
    );
    introuvable(
        ent2.eva_v4_reconvoquer_for_actor(
            w2.etab,
            w2.direction,
            DIR,
            jour(),
            rc.id,
            &ReconvoquerBody {
                prevu_le: d(2031, 10, 31),
                heure: None,
                lieu: None,
                modalite: None,
                echeance: None,
            },
        )
        .await,
    );
    introuvable(
        ent2.eva_v4_attribuer_zero_for_actor(
            w2.etab,
            w2.direction,
            DIR,
            jour(),
            rc.id,
            &AttribuerZeroBody {
                confirme: true,
                version_attendue: rc.version,
                motif: None,
            },
        )
        .await,
    );
    let _ = b;
    let intacte: String = sqlx::query_scalar(
        "SELECT etat FROM eva_rattrapage WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.etab)
    .bind(rc.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(intacte, "convoque");
}

/// M-6 / F-8 : module santé, journal de lecture, AESH limité à ses élèves et aux évaluations annoncées.
#[tokio::test]
async fn v3d_f8_amenagements_module_sante_journal_et_aesh_limite_a_ses_eleves() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let d1 = w.eleves_d[0];
    let aesh = cast.cpe.user_id;
    amenagement(&ent, &w, a, "tiers_temps", "examen").await;
    amenagement(&ent, &w, d1, "lecteur", "examen").await;
    let ev = creer_annoncee(&ent, &w, "DS 1", d(2031, 10, 24)).await;
    // Module santé inactif : fermé par défaut, même pour le professeur du cours.
    let r = ent
        .eva_v4_amenagements_evaluation_for_actor(w.etab, w.prof, PROF, ev, jour())
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    ent.activer_module_sante_test(w.etab).await;
    let v = ent
        .eva_v4_amenagements_evaluation_for_actor(w.etab, w.prof, PROF, ev, jour())
        .await
        .unwrap();
    assert_eq!(
        v.eleves.iter().map(|e| e.eleve_id).collect::<Vec<_>>(),
        vec![a]
    );
    // La lecture est journalisée (un enregistrement par élève lu), sans donnée de santé dans le libellé.
    let (n, libelle): (i64, String) = sqlx::query_as(
        "SELECT count(*), max(libelle) FROM ent_journal_modif
          WHERE etablissement_id = $1 AND domaine = 'evaluations' AND action = 'consultation_amenagements' AND eleve_id = $2",
    )
    .bind(w.etab)
    .bind(a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1);
    assert!(!libelle.to_lowercase().contains("tiers"), "{libelle}");
    // Un AESH affecté à un élève d'une AUTRE classe ne voit aucun élève de cette évaluation (aucune fuite des autres).
    sqlx::query(
        "INSERT INTO ent_aesh_affectation (id, etablissement_id, eleve_id, aesh_nom, aesh_user_id, date_debut, affecte_par_user_id)
         VALUES ($1, $2, $3, 'AESH test', $4, '2031-09-01', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(d1)
    .bind(aesh)
    .bind(w.direction)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let v = ent
        .eva_v4_amenagements_evaluation_for_actor(w.etab, aesh, "cpe", ev, jour())
        .await
        .unwrap();
    assert!(v.eleves.is_empty(), "{v:?}");
    // Un code hors du catalogue fermé n'est jamais montré (si la base l'accepte).
    let inconnu = sqlx::query(
        "INSERT INTO ent_amenagement (id, etablissement_id, eleve_id, code, portee, date_debut, defini_par_user_id)
         VALUES ($1, $2, $3, 'code_hors_catalogue', 'examen', '2031-09-01', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(a)
    .bind(w.direction)
    .execute(ent.pool_verif())
    .await;
    if inconnu.is_ok() {
        let v = ent
            .eva_v4_amenagements_evaluation_for_actor(w.etab, w.prof, PROF, ev, jour())
            .await
            .unwrap();
        let json = serde_json::to_string(&v).unwrap();
        assert!(!json.contains("code_hors_catalogue"), "{json}");
    }
}

/// F-4 : le rattrapage d'une période arrêtée est refusé à la direction aussi tant qu'il n'est pas autorisé ; après son
/// échéance, seule la direction note, avec un motif.
#[tokio::test]
async fn v3d_f4_periode_arretee_sans_autorisation_et_echeance_depassee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, "DS 1", d(2031, 10, 10)).await;
    saisir(&ent, &w, ev, a, "abs").await;
    arreter(&ent, &w, w.t1, w.classe_a).await;
    // La direction (qui ne saisit pas) est refusée comme le professeur : rien n'est créé.
    let r = ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            &plan(ev, vec![a], d(2031, 12, 10)),
        )
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("période arrêtée")),
        "{r:?}"
    );
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM eva_rattrapage WHERE etablissement_id = $1")
            .bind(w.etab)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(n, 0);
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
            w.direction,
            DIR,
            jour(),
            &plan(ev, vec![a], d(2031, 12, 10)),
        )
        .await
        .unwrap()
        .remove(0);
    // Après l'échéance (2031-12-10) : le professeur ne note plus ; la direction note avec un motif.
    let apres = d(2031, 12, 12);
    let noter = |acteur: Uuid, role: &'static str, motif: Option<&str>| {
        let (ent, etab) = (&ent, w.etab);
        let corps = NoterRattrapageBody {
            motif: motif.map(str::to_string),
            ..note_texte("11", rs.version)
        };
        let id = rs.id;
        async move {
            ent.eva_v4_noter_rattrapage_for_actor(etab, acteur, role, apres, id, &corps)
                .await
        }
    };
    let r = noter(w.prof, PROF, Some("trop tard")).await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("échéance")),
        "{r:?}"
    );
    let r = noter(w.direction, DIR, None).await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("motif")),
        "{r:?}"
    );
    let ok = noter(
        w.direction,
        DIR,
        Some("Convocation reportée par la direction"),
    )
    .await
    .unwrap();
    assert_eq!(ok.etat, "passe");
    assert_eq!(resultat(&ent, &w, ev, a).await.unwrap().1, Some(11.0));
    // V3e (item 1) : le motif de la note après l'échéance reste lisible au journal (avant : effacé par le déclencheur).
    let (motif, correction): (Option<String>, bool) = sqlx::query_as(
        "SELECT h.motif, h.correction FROM eva_resultat_historique h JOIN eva_resultat r ON r.id = h.resultat_id
          WHERE r.etablissement_id = $1 AND r.evaluation_id = $2 AND r.eleve_id = $3 ORDER BY h.version DESC LIMIT 1",
    )
    .bind(w.etab)
    .bind(ev)
    .bind(a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        motif.as_deref(),
        Some("Convocation reportée par la direction")
    );
    assert!(!correction, "rattrapage autorisé : ce n'est pas une correction après l'arrêt, mais son motif est conservé");
}

/// F-5 : une évaluation hors séance ne compte pas comme « absence sans justification » quand l'élève était absent ce jour-là
/// avec un justificatif valable (demi-journée du socle 12).
#[tokio::test]
async fn v3d_f5_ce27_evaluation_hors_seance_avec_demi_journee_justifiee_n_est_pas_un_faux_positif()
{
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    for (i, jd) in [d(2031, 10, 6), d(2031, 10, 8), d(2031, 10, 13)]
        .into_iter()
        .enumerate()
    {
        let ev = creer(&ent, &w, &format!("DM {i}"), jd).await; // aucune séance : hors séance
        saisir(&ent, &w, ev, a, "abs").await;
        saisir(&ent, &w, ev, b, "abs").await;
    }
    // Sans justificatif : a et b sont signalés (3 absences).
    let nb = |v: &[AbsenceRepeteeDto]| v.iter().map(|x| x.eleve_id).collect::<Vec<_>>();
    let avant = ent
        .eva_v4_absences_repetees_for_actor(w.etab, w.direction, DIR, jour(), w.t1, None)
        .await
        .unwrap();
    assert_eq!(nb(&avant), {
        let mut v = vec![a, b];
        v.sort();
        v
    });
    // a : absent toute la matinée du 8 octobre, avec un motif valable → 2 absences sans justification seulement.
    let abs = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_asd_absence (id, etablissement_id, eleve_id, jour_debut, jour_fin, etat, qualification)
         VALUES ($1, $2, $3, '2031-10-08', '2031-10-08', 'qualifie', 'excuse_valable')",
    )
    .bind(abs)
    .bind(w.etab)
    .bind(a)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_asd_absence_seance (id, etablissement_id, absence_id, eleve_id, jour, kind, periode, source)
         VALUES ($1, $2, $3, $4, '2031-10-08', 'demi_journee', 'matin', 'vie_scolaire')",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(abs)
    .bind(a)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let apres = ent
        .eva_v4_absences_repetees_for_actor(w.etab, w.direction, DIR, jour(), w.t1, None)
        .await
        .unwrap();
    assert_eq!(
        nb(&apres),
        vec![b],
        "a a un justificatif pour l'une des trois : plus de faux positif"
    );
}

/// F-1, F-9 : gardes de la base (TRUNCATE, ajout seul), dispense qui suit la fusion de fiches.
#[tokio::test]
async fn v3d_f1_f9_gardes_de_la_base_sur_les_ports_et_dispense_qui_suit_la_fusion() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    let disp = ent
        .eva_v4_declarer_dispense_for_actor(
            w.etab,
            w.direction,
            DIR,
            &DeclarerDispenseBody {
                eleve_id: a,
                matiere_id: Some(w.maths),
                nature: "dispense".into(),
                du: d(2031, 10, 1),
                au: None,
            },
        )
        .await
        .unwrap();
    let indispo = Uuid::new_v4();
    sqlx::query("INSERT INTO eva_aesh_indisponibilite_port (id, etablissement_id, aesh_user_id, du, au, saisi_par) VALUES ($1, $2, $3, '2031-10-24', '2031-10-24', $4)")
        .bind(indispo)
        .bind(w.etab)
        .bind(cast.cpe.user_id)
        .bind(w.direction)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    // TRUNCATE (dispenses, indisponibilités AESH), UPDATE et DELETE (indisponibilités) : refusés par la base.
    for ordre in [
        "TRUNCATE eva_dispense_port",
        "TRUNCATE eva_aesh_indisponibilite_port",
        "DELETE FROM eva_aesh_indisponibilite_port",
        "UPDATE eva_aesh_indisponibilite_port SET au = '2031-10-30'",
        "DELETE FROM eva_dispense_port",
    ] {
        let r = sqlx::query(ordre).execute(ent.pool_verif()).await;
        assert!(r.is_err(), "{ordre} doit être refusé");
    }
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM eva_dispense_port WHERE id = $1")
        .bind(disp.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(n, 1);
    // Fusion de fiches : seul le déplacement de l'élève, dans la fusion, est admis ; sinon la garde de 475 reste entière.
    let sans_fusion = sqlx::query("UPDATE eva_dispense_port SET eleve_id = $2 WHERE id = $1")
        .bind(disp.id)
        .bind(b)
        .execute(ent.pool_verif())
        .await;
    assert!(
        sans_fusion.is_err(),
        "hors fusion, l'élève d'une dispense ne change pas"
    );
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.fusion_eleve', 'on', true)")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE eva_dispense_port SET eleve_id = $2 WHERE id = $1")
        .bind(disp.id)
        .bind(b)
        .execute(&mut *tx)
        .await
        .expect("pendant la fusion, la dispense suit l'élève conservé");
    let autre =
        sqlx::query("UPDATE eva_dispense_port SET nature = 'inaptitude_totale' WHERE id = $1")
            .bind(disp.id)
            .execute(&mut *tx)
            .await;
    assert!(
        autre.is_err(),
        "même pendant la fusion, rien d'autre ne change"
    );
    tx.rollback().await.unwrap();
    assert_eq!(
        domain_eleve::table_eleve("eva_dispense_port")
            .unwrap()
            .strategie,
        domain_eleve::StrategieFusion::Deplacer
    );
}

/// F-3 : la confirmation des propositions est bornée.
#[tokio::test]
async fn v3d_f3_la_confirmation_des_propositions_est_bornee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = creer(&ent, &w, "DS 1", d(2031, 10, 10)).await;
    let r = ent
        .eva_v4_appliquer_propositions_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &AppliquerPropositionsBody {
                cours_id: w.cours_maths,
                evaluation_id: ev,
                eleves: Some((0..301).map(|_| Uuid::new_v4()).collect()),
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
}

/// Test web non déterministe (CI 463) et tests « moyenne vide » : la période d'un élève un jour donné ne dépend plus du plan
/// d'exécution, et s'accorde avec la période de classe d'une période de notes. Deux périodes de l'historique se recouvrent
/// (cas réel du labo et des tests : la projection `W…` d'une période de notes recouvre `T1`) ; règle commune : à recouvrement
/// égal, la plus précise (la plus courte), puis la plus ancienne.
#[tokio::test]
async fn v3d_item7_la_periode_de_l_eleve_est_deterministe_et_s_accorde_avec_celle_de_la_periode_de_notes(
) {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    // Une période de notes de l'ancien modèle (projetée dans la périodisation historique), contenue dans T1.
    let np_a = ent
        .creer_note_periode(domain_eleve::CreerNotePeriode {
            etablissement_id: w.etab,
            code: "NA".into(),
            libelle: "Contenue dans T1".into(),
            date_debut: d(2031, 10, 15),
            date_fin: d(2031, 11, 14),
            actor_user_id: None,
        })
        .await
        .unwrap()
        .id;
    // Une autre, qui DÉPASSE la fin de T1 (20 décembre) : elle recouvre davantage sa propre projection que le trimestre.
    let np_b = ent
        .creer_note_periode(domain_eleve::CreerNotePeriode {
            etablissement_id: w.etab,
            code: "NB".into(),
            libelle: "Dépasse T1".into(),
            date_debut: d(2031, 12, 1),
            date_fin: d(2031, 12, 30),
            actor_user_id: None,
        })
        .await
        .unwrap()
        .id;
    let mut conn = ent.pool_verif().acquire().await.unwrap();
    // Plusieurs plans d'exécution : le résultat ne change pas (avant 476, il changeait « une fois sur plusieurs »).
    for reglage in [
        "",
        "SET enable_hashjoin = off",
        "SET enable_nestloop = off",
        "SET enable_mergejoin = off",
        "SET enable_indexscan = off; SET enable_bitmapscan = off",
        "SET enable_seqscan = off",
    ] {
        sqlx::raw_sql("RESET ALL")
            .execute(&mut *conn)
            .await
            .unwrap();
        if !reglage.is_empty() {
            sqlx::raw_sql(reglage).execute(&mut *conn).await.unwrap();
        }
        for (np, jour_eval) in [(np_a, d(2031, 10, 20)), (np_b, d(2031, 12, 10))] {
            let de_l_eleve: Uuid =
                sqlx::query_scalar("SELECT periode_id FROM eva_periode_eleve_jour($1, $2, $3)")
                    .bind(w.etab)
                    .bind(a)
                    .bind(jour_eval)
                    .fetch_one(&mut *conn)
                    .await
                    .unwrap();
            let de_la_periode_de_notes: Uuid =
                sqlx::query_scalar("SELECT ent_periode_de_note_pour_classe($1, $2, $3)")
                    .bind(w.etab)
                    .bind(np)
                    .bind(w.classe_a)
                    .fetch_one(&mut *conn)
                    .await
                    .unwrap();
            assert_eq!(
                de_l_eleve, de_la_periode_de_notes,
                "plan « {reglage} », jour {jour_eval} : la période de l'élève et celle de la période de notes sont la même"
            );
            assert_ne!(
                de_l_eleve, w.t1,
                "la période la plus précise (la projection), pas le trimestre"
            );
        }
    }
    sqlx::raw_sql("RESET ALL")
        .execute(&mut *conn)
        .await
        .unwrap();
}

// ───────────────────────── V3e : revue de V3d ─────────────────────────

/// V3e item 2 (M-5 hors séance) : sans séance du cours ce jour-là, la demi-journée justifiée arrivée après le constat
/// bloque le zéro, comme une absence justifiée à la séance.
#[tokio::test]
async fn v3e_item2_le_zero_hors_seance_relit_la_demi_journee_justifiee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    let jr = d(2031, 10, 24);
    // Évaluation de remplacement HORS séance : aucune séance du cours ce jour-là.
    let rempl = creer(&ent, &w, "Remplacement hors séance", jr).await;
    let rs = ent
        .eva_v4_convoquer_remplacement_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            &plan(rempl, vec![a, b], jr),
        )
        .await
        .unwrap();
    let mut conv = std::collections::HashMap::new();
    for r in &rs {
        let r2 = ent
            .eva_v4_noter_rattrapage_for_actor(
                w.etab,
                w.prof,
                PROF,
                jour(),
                r.id,
                &absence_corps("non_justifiee", r.version),
            )
            .await
            .unwrap();
        conv.insert(r.eleve_id, r2);
    }
    // a : une demi-journée justifiée du socle 12 arrive APRÈS le constat.
    let abs = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_asd_absence (id, etablissement_id, eleve_id, jour_debut, jour_fin, etat, qualification)
         VALUES ($1, $2, $3, $4, $4, 'qualifie', 'excuse_valable')",
    )
    .bind(abs)
    .bind(w.etab)
    .bind(a)
    .bind(jr)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_asd_absence_seance (id, etablissement_id, absence_id, eleve_id, jour, kind, periode, source)
         VALUES ($1, $2, $3, $4, $5, 'demi_journee', 'matin', 'vie_scolaire')",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(abs)
    .bind(a)
    .bind(jr)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let zero = |e: Uuid| {
        let r = conv[&e].clone();
        let (ent, etab, direction) = (&ent, w.etab, w.direction);
        async move {
            ent.eva_v4_attribuer_zero_for_actor(
                etab,
                direction,
                DIR,
                jour(),
                r.id,
                &AttribuerZeroBody {
                    confirme: true,
                    version_attendue: r.version,
                    motif: None,
                },
            )
            .await
        }
    };
    let r = zero(a).await;
    assert!(
        matches!(&r, Err(EntDbError::ConflitDossier(m)) if m.contains("justifiée depuis")),
        "{r:?}"
    );
    assert!(
        resultat(&ent, &w, rempl, a).await.is_none(),
        "rien n'est écrit pour a"
    );
    // b : aucune justification : la direction attribue le zéro (période arrêtée).
    zero(b).await.unwrap();
    assert_eq!(resultat(&ent, &w, rempl, b).await.unwrap().1, Some(0.0));
}

/// V3e item 4 (F-4) : une échéance dépassée en période OUVERTE ne bloque plus personne : la direction la prolonge avec un
/// motif, la prolongation est tracée (échéances avant et après, motif), puis le professeur note.
#[tokio::test]
async fn v3e_item4_prolongation_d_echeance_tracee_en_periode_ouverte() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, "DS 1", d(2031, 10, 10)).await;
    saisir(&ent, &w, ev, a, "abs").await;
    let rs = ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![a], d(2031, 10, 22)),
        )
        .await
        .unwrap()
        .remove(0);
    let apres = d(2031, 10, 25); // après l'échéance du 22
    let corps = note_texte("11", rs.version);
    // Impasse d'avant V3e : ni le professeur ni la direction ne notent.
    let r = ent
        .eva_v4_noter_rattrapage_for_actor(w.etab, w.prof, PROF, apres, rs.id, &corps)
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("échéance")),
        "{r:?}"
    );
    let r = ent
        .eva_v4_noter_rattrapage_for_actor(
            w.etab,
            w.direction,
            DIR,
            apres,
            rs.id,
            &NoterRattrapageBody {
                motif: Some("Direction".into()),
                ..corps.clone()
            },
        )
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(_))),
        "période ouverte : la direction ne note pas à la place : {r:?}"
    );
    // La voie : prolonger. Le professeur ne le peut pas ; la direction sans motif non plus.
    let dep = |motif: Option<&str>| DeplacerRattrapageBody {
        prevu_le: d(2031, 10, 26),
        heure: None,
        lieu: None,
        modalite: None,
        echeance: Some(d(2031, 11, 5)),
        version_attendue: rs.version,
        motif: motif.map(str::to_string),
    };
    let r = ent
        .eva_v4_deplacer_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            apres,
            rs.id,
            &dep(Some("Je prolonge")),
        )
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("seule la direction")),
        "{r:?}"
    );
    let r = ent
        .eva_v4_deplacer_rattrapage_for_actor(w.etab, w.direction, DIR, apres, rs.id, &dep(None))
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("motif")),
        "{r:?}"
    );
    let r = ent
        .eva_v4_deplacer_rattrapage_for_actor(
            w.etab,
            w.direction,
            DIR,
            apres,
            rs.id,
            &dep(Some("ab")),
        )
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(_))),
        "motif trop court : {r:?}"
    );
    let prolongee = ent
        .eva_v4_deplacer_rattrapage_for_actor(
            w.etab,
            w.direction,
            DIR,
            apres,
            rs.id,
            &dep(Some("Élève malade, prolongation accordée")),
        )
        .await
        .unwrap();
    let (av, ap, motif, par): (Option<NaiveDate>, Option<NaiveDate>, Option<String>, Uuid) =
        sqlx::query_as(
            "SELECT echeance_avant, echeance_apres, motif, par FROM eva_rattrapage_historique
          WHERE etablissement_id = $1 AND rattrapage_id = $2 ORDER BY version DESC LIMIT 1",
        )
        .bind(w.etab)
        .bind(rs.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!((av, ap), (Some(d(2031, 10, 22)), Some(d(2031, 11, 5))));
    assert_eq!(
        motif.as_deref(),
        Some("Élève malade, prolongation accordée")
    );
    assert_eq!(par, w.direction);
    // Le motif d'une prolongation ne reste pas périmé : un déplacement sans changement d'échéance n'en porte aucun.
    let dep2 = DeplacerRattrapageBody {
        prevu_le: d(2031, 10, 27),
        heure: None,
        lieu: None,
        modalite: None,
        echeance: Some(d(2031, 11, 5)),
        version_attendue: prolongee.version,
        motif: None,
    };
    let r2 = ent
        .eva_v4_deplacer_rattrapage_for_actor(w.etab, w.prof, PROF, apres, rs.id, &dep2)
        .await
        .unwrap();
    let motif2: Option<String> = sqlx::query_scalar(
        "SELECT motif FROM eva_rattrapage_historique WHERE etablissement_id = $1 AND rattrapage_id = $2 ORDER BY version DESC LIMIT 1",
    )
    .bind(w.etab)
    .bind(rs.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(motif2, None);
    // Le professeur note, dans l'échéance prolongée.
    let ok = ent
        .eva_v4_noter_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            d(2031, 10, 28),
            rs.id,
            &note_texte("11", r2.version),
        )
        .await
        .unwrap();
    assert_eq!(ok.etat, "passe");
    assert_eq!(resultat(&ent, &w, ev, a).await.unwrap().1, Some(11.0));
}

/// V3e item 6 : « introuvable » ne se distingue pas de « pas votre droit » pour qui ne porte pas `notes.corriger`
/// (une convocation inconnue répond comme une convocation d'un autre cours).
#[tokio::test]
async fn v3e_item6_une_convocation_inconnue_ne_se_distingue_pas_d_un_droit_refuse() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = creer(&ent, &w, "DS 1", d(2031, 10, 10)).await;
    saisir(&ent, &w, ev, a, "abs").await;
    let rs = ent
        .eva_v4_planifier_rattrapage_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &plan(ev, vec![a], d(2031, 10, 22)),
        )
        .await
        .unwrap()
        .remove(0);
    let autre = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)
        .unwrap();
    let dep = |version: i32| DeplacerRattrapageBody {
        prevu_le: d(2031, 10, 30),
        heure: None,
        lieu: None,
        modalite: None,
        echeance: None,
        version_attendue: version,
        motif: None,
    };
    let existante = ent
        .eva_v4_deplacer_rattrapage_for_actor(w.etab, autre, PROF, jour(), rs.id, &dep(rs.version))
        .await;
    let inconnue = ent
        .eva_v4_deplacer_rattrapage_for_actor(w.etab, autre, PROF, jour(), Uuid::new_v4(), &dep(1))
        .await;
    assert!(
        matches!(existante, Err(EntDbError::Forbidden)),
        "{existante:?}"
    );
    assert!(
        matches!(inconnue, Err(EntDbError::Forbidden)),
        "{inconnue:?}"
    );
    // La direction (`notes.corriger`) voit la différence : une ligne qui n'existe pas est « introuvable ».
    let r = ent
        .eva_v4_deplacer_rattrapage_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            Uuid::new_v4(),
            &dep(1),
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::Domain(DomainError::NotFound))),
        "{r:?}"
    );
}

/// V3e item 5 : UNE fonction pour « la période d'un jour », totale : deux périodes qui se recouvrent dans la périodisation
/// historique, un jour appartient à UNE seule d'entre elles (jamais deux, jamais aucune), et les trois lecteurs s'accordent.
#[tokio::test]
async fn v3e_item5_la_periode_d_un_jour_est_unique_et_les_lecteurs_s_accordent() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let pz: Uuid = sqlx::query_scalar(
        "SELECT periodisation_id FROM ent_periode WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.etab)
    .bind(w.t1)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    // Une période plus courte, qui recouvre T1 du 16 novembre au 11 décembre (origine reprise : non protégée).
    let courte = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_periode (id, etablissement_id, annee_id, periodisation_id, code, libelle, kind, ordre,
                                  date_debut, date_fin, date_arret_notes, origine)
         VALUES ($1, $2, $3, $4, 'W', 'Période de notes', 'trimestre', 9, '2031-11-16', '2031-12-11', '2031-12-05', 'reprise')",
    )
    .bind(courte)
    .bind(w.etab)
    .bind(w.annee)
    .bind(pz)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    for (jour_test, attendue) in [
        (d(2031, 10, 10), w.t1),
        (d(2031, 11, 20), courte),
        (d(2031, 12, 15), w.t1),
    ] {
        let choisie: Option<Uuid> =
            sqlx::query_scalar("SELECT ent_periode_choisir($1, $2, $3, $3, $4)")
                .bind(w.etab)
                .bind(pz)
                .bind(jour_test)
                .bind(w.annee)
                .fetch_one(ent.pool_verif())
                .await
                .unwrap();
        assert_eq!(choisie, Some(attendue), "le {jour_test}");
        let eleve: Option<Uuid> =
            sqlx::query_scalar("SELECT periode_id FROM eva_periode_eleve_jour($1, $2, $3)")
                .bind(w.etab)
                .bind(a)
                .bind(jour_test)
                .fetch_optional(ent.pool_verif())
                .await
                .unwrap();
        assert_eq!(eleve, Some(attendue), "période de l'élève le {jour_test}");
        for (p, doit) in [(w.t1, attendue == w.t1), (courte, attendue == courte)] {
            let dans: bool = sqlx::query_scalar("SELECT eva_periode_dans($1, $2, $3)")
                .bind(w.etab)
                .bind(p)
                .bind(jour_test)
                .fetch_one(ent.pool_verif())
                .await
                .unwrap();
            assert_eq!(dans, doit, "le {jour_test} appartient à une seule période");
        }
    }
}
