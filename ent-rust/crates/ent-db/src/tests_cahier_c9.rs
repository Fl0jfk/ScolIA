//! Socle 20, lot C9 : progressions et reconduction (`20_CAHIER_DE_TEXTES.md` §16), contre la base locale de test (feature
//! `labo`). Sous-module de `tests_cahier` (fixture `Fx`, un établissement neuf par test). Les jours sont ceux de l'établissement.

use super::*;
use chrono::Duration;
use ent_types::cahier_c2::*;
use ent_types::cahier_c9::*;

async fn jour(fx: &Fx) -> NaiveDate {
    fx.ent.jour_etab(fx.cast.etablissement_id).await.unwrap()
}

async fn cours_de(fx: &Fx, service: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT cours_id FROM ent_service_enseignement WHERE id = $1")
        .bind(service)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
}

/// Premier jour ouvré (non fermé) à partir de `d`.
async fn ouvre(fx: &Fx, d: NaiveDate) -> NaiveDate {
    sqlx::query_scalar("SELECT x::date FROM generate_series($2::date, $2::date + 60, interval '1 day') AS x WHERE NOT ent_cdt_jour_ferme($1, x::date) ORDER BY 1 LIMIT 1")
        .bind(fx.cast.etablissement_id)
        .bind(d)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap()
}

/// `n` séances consécutives (jours ouvrés distincts) du cours de `service`, à partir de `debut` ; retourne leurs jours.
async fn seances(
    fx: &Fx,
    classe: &str,
    prof: Uuid,
    service: Uuid,
    debut: NaiveDate,
    n: usize,
) -> Vec<(Uuid, NaiveDate)> {
    let mut out = Vec::new();
    let mut j = debut;
    for _ in 0..n {
        j = ouvre(fx, j).await;
        out.push((fx.seance(classe, prof, Some(service), j).await, j));
        j += Duration::days(1);
    }
    out
}

fn etape(titre: &str, duree: i16) -> EtapeBody {
    EtapeBody {
        id: None,
        titre: titre.into(),
        notions: "Définition ; exemples".into(),
        duree_seances: duree,
    }
}

fn progression(titre: &str, etapes: Vec<EtapeBody>) -> ProgressionBody {
    ProgressionBody {
        titre: titre.into(),
        matiere_id: None,
        niveau_id: None,
        annee_id: None,
        etapes,
    }
}

async fn service(fx: &Fx, prof: Uuid, classe: Uuid, matiere: Option<Uuid>) -> Uuid {
    fx.ent
        .referentiel_creer_service(
            Role::Direction.as_str(),
            fx.cast.etablissement_id,
            CreerService {
                prof_user_id: prof,
                matiere_id: matiere,
                classe_id: Some(classe),
                groupe_id: None,
                professeur_principal: false,
            },
        )
        .await
        .unwrap()
        .id
}

async fn matiere(fx: &Fx, code: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_matiere (id, etablissement_id, code, libelle) VALUES ($1, $2, $3, $3)",
    )
    .bind(id)
    .bind(fx.cast.etablissement_id)
    .bind(code)
    .execute(fx.ent.pool())
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn c9_une_progression_est_privee_son_auteur_seul_la_lit_et_la_modifie() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let p = fx
        .ent
        .cdt_c9_progression_creer(
            &fx.prof(),
            progression(
                "Fractions",
                vec![etape("Définitions", 2), etape("Opérations", 3)],
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        (p.resume.nb_etapes, p.resume.nb_seances, p.resume.mienne),
        (2, 5, true)
    );
    assert_eq!(p.etapes[0].ordre, 1);
    // Personne d'autre ne la lit : ni l'autre professeur, ni la direction (le plan est privé), ni la vie scolaire, ni élèves et familles.
    assert!(fx
        .ent
        .cdt_c9_progression_lire(&fx.prof2(), p.resume.id)
        .await
        .is_err_and(|e| est_introuvable(&e)));
    assert!(fx
        .ent
        .cdt_c9_progression_lire(&fx.direction(), p.resume.id)
        .await
        .is_err_and(|e| est_introuvable(&e)));
    for a in [fx.cpe(), fx.compta(), fx.eleve(&fx.x), fx.parent(&fx.x)] {
        assert!(
            fx.ent
                .cdt_c9_progression_lire(&a, p.resume.id)
                .await
                .is_err_and(|e| est_interdit(&e)),
            "{}",
            a.role
        );
        assert!(fx
            .ent
            .cdt_c9_progressions(&a)
            .await
            .is_err_and(|e| est_interdit(&e)));
        assert!(fx
            .ent
            .cdt_c9_progression_creer(&a, progression("x", vec![etape("e", 1)]))
            .await
            .is_err_and(|e| est_interdit(&e)));
    }
    assert!(fx
        .ent
        .cdt_c9_progressions(&fx.prof2())
        .await
        .unwrap()
        .mes
        .is_empty());
    assert!(fx
        .ent
        .cdt_c9_progressions(&fx.direction())
        .await
        .unwrap()
        .partagees
        .is_empty());
    let l = fx.ent.cdt_c9_progressions(&fx.prof()).await.unwrap();
    assert_eq!(l.mes.len(), 1);
    assert!(l.partagees.is_empty());
    // Modification : réservée à l'auteur ; validations (HTML refusé, durée, vide).
    let mut m = progression(
        "Fractions v2",
        vec![
            EtapeBody {
                id: Some(p.etapes[0].id),
                ..etape("Définitions revues", 1)
            },
            etape("Problèmes", 2),
        ],
    );
    assert!(fx
        .ent
        .cdt_c9_progression_modifier(&fx.prof2(), p.resume.id, m.clone())
        .await
        .is_err_and(|e| est_introuvable(&e)));
    let q = fx
        .ent
        .cdt_c9_progression_modifier(&fx.prof(), p.resume.id, m.clone())
        .await
        .unwrap();
    assert_eq!(q.etapes.len(), 2);
    assert_eq!(
        q.etapes[0].id, p.etapes[0].id,
        "l'étape conservée garde son identifiant"
    );
    assert_eq!(q.resume.titre, "Fractions v2");
    m.etapes[0].titre = "<b>gras</b>".into();
    assert!(fx
        .ent
        .cdt_c9_progression_modifier(&fx.prof(), p.resume.id, m.clone())
        .await
        .is_err());
    m.etapes[0].titre = "ok".into();
    m.etapes[0].duree_seances = 31;
    assert!(fx
        .ent
        .cdt_c9_progression_modifier(&fx.prof(), p.resume.id, m.clone())
        .await
        .is_err());
    assert!(fx
        .ent
        .cdt_c9_progression_creer(&fx.prof(), progression("vide", vec![]))
        .await
        .is_err());
    assert!(fx
        .ent
        .cdt_c9_progression_creer(&fx.prof(), progression("  ", vec![etape("e", 1)]))
        .await
        .is_err());
    // Archivée : disparaît des listes, ne se modifie plus, mais n'est pas supprimée.
    fx.ent
        .cdt_c9_progression_archiver(&fx.prof2(), p.resume.id)
        .await
        .expect_err("pas l'auteur");
    fx.ent
        .cdt_c9_progression_archiver(&fx.prof(), p.resume.id)
        .await
        .unwrap();
    assert!(fx
        .ent
        .cdt_c9_progressions(&fx.prof())
        .await
        .unwrap()
        .mes
        .is_empty());
    assert!(fx
        .ent
        .cdt_c9_progression_modifier(
            &fx.prof(),
            p.resume.id,
            progression("x", vec![etape("e", 1)])
        )
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_cdt_progression WHERE id = $1")
        .bind(p.resume.id)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn c9_partage_avec_un_cours_ou_une_discipline_sans_ouvrir_le_cahier_d_un_autre_cours() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (p0, p1) = (fx.cast.profs[0].user_id, fx.cast.profs[1].user_id);
    let maths = matiere(&fx, "MATHS-C9").await;
    let svc_b = service(&fx, p0, fx.classe_b, Some(maths)).await;
    let svc_c = service(&fx, p1, fx.classe_a, Some(maths)).await;
    let cours_b = cours_de(&fx, svc_b).await;
    let cours_c = cours_de(&fx, svc_c).await;
    let cours_a = cours_de(&fx, fx.service_a).await;
    let p = fx
        .ent
        .cdt_c9_progression_creer(
            &fx.prof(),
            progression("Géométrie", vec![etape("Angles", 2)]),
        )
        .await
        .unwrap();
    let id = p.resume.id;
    // Partager n'est possible que pour son auteur, vers un cours où il intervient ou une discipline qu'il enseigne.
    let vers_c = PartageBody {
        cours_id: Some(cours_c),
        matiere_id: None,
    };
    assert!(
        fx.ent
            .cdt_c9_partager(&fx.prof(), id, vers_c.clone())
            .await
            .is_err_and(|e| est_interdit(&e)),
        "il n'intervient pas dans le cours de son collègue"
    );
    assert!(
        fx.ent
            .cdt_c9_partager(
                &fx.prof2(),
                id,
                PartageBody {
                    cours_id: Some(cours_c),
                    matiere_id: None
                }
            )
            .await
            .is_err_and(|e| est_introuvable(&e)),
        "ce n'est pas sa progression"
    );
    let autre_matiere = matiere(&fx, "LATIN-C9").await;
    assert!(fx
        .ent
        .cdt_c9_partager(
            &fx.prof(),
            id,
            PartageBody {
                cours_id: None,
                matiere_id: Some(autre_matiere)
            }
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c9_partager(
            &fx.prof(),
            id,
            PartageBody {
                cours_id: None,
                matiere_id: None
            }
        )
        .await
        .is_err());
    assert!(fx
        .ent
        .cdt_c9_partager(
            &fx.prof(),
            id,
            PartageBody {
                cours_id: Some(cours_a),
                matiere_id: Some(maths)
            }
        )
        .await
        .is_err());
    assert!(fx
        .ent
        .cdt_c9_partager(
            &fx.prof(),
            id,
            PartageBody {
                cours_id: Some(Uuid::new_v4()),
                matiere_id: None
            }
        )
        .await
        .is_err_and(|e| est_introuvable(&e)));
    // Partage avec la discipline : le collègue de la même matière la voit, en lecture, sans les partages de l'auteur.
    let p2 = fx
        .ent
        .cdt_c9_partager(
            &fx.prof(),
            id,
            PartageBody {
                cours_id: None,
                matiere_id: Some(maths),
            },
        )
        .await
        .unwrap();
    assert_eq!(p2.partages.len(), 1);
    assert!(fx
        .ent
        .cdt_c9_partager(
            &fx.prof(),
            id,
            PartageBody {
                cours_id: None,
                matiere_id: Some(maths)
            }
        )
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
    let vue = fx
        .ent
        .cdt_c9_progression_lire(&fx.prof2(), id)
        .await
        .unwrap();
    assert!(!vue.resume.mienne);
    assert!(
        vue.partages.is_empty(),
        "les partages sont lus de l'auteur seulement"
    );
    assert_eq!(
        fx.ent
            .cdt_c9_progressions(&fx.prof2())
            .await
            .unwrap()
            .partagees
            .len(),
        1
    );
    // Le collègue ne peut ni la modifier, ni l'archiver, ni la repartager.
    assert!(fx
        .ent
        .cdt_c9_progression_modifier(&fx.prof2(), id, progression("x", vec![etape("e", 1)]))
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c9_progression_archiver(&fx.prof2(), id)
        .await
        .is_err_and(|e| est_interdit(&e)));
    assert!(fx
        .ent
        .cdt_c9_partager(
            &fx.prof2(),
            id,
            PartageBody {
                cours_id: Some(cours_c),
                matiere_id: None
            }
        )
        .await
        .is_err_and(|e| est_interdit(&e)));
    // La direction et le reste de l'établissement ne la voient toujours pas.
    assert!(fx
        .ent
        .cdt_c9_progression_lire(&fx.direction(), id)
        .await
        .is_err_and(|e| est_introuvable(&e)));
    // Retrait du partage : le collègue n'y accède plus.
    let sid = p2.partages[0].id;
    assert!(fx
        .ent
        .cdt_c9_partage_retirer(&fx.prof2(), id, sid)
        .await
        .is_err());
    assert!(fx
        .ent
        .cdt_c9_partage_retirer(&fx.prof(), id, Uuid::new_v4())
        .await
        .is_err_and(|e| est_introuvable(&e)));
    fx.ent
        .cdt_c9_partage_retirer(&fx.prof(), id, sid)
        .await
        .unwrap();
    assert!(fx
        .ent
        .cdt_c9_progression_lire(&fx.prof2(), id)
        .await
        .is_err_and(|e| est_introuvable(&e)));
    // Partage avec un cours où l'auteur intervient : les intervenants de ce cours la voient.
    fx.ent
        .cdt_c9_partager(
            &fx.prof(),
            id,
            PartageBody {
                cours_id: Some(cours_b),
                matiere_id: None,
            },
        )
        .await
        .unwrap();
    assert!(
        fx.ent
            .cdt_c9_progression_lire(&fx.prof2(), id)
            .await
            .is_err_and(|e| est_introuvable(&e)),
        "prof2 n'intervient pas dans le cours B"
    );
    // Partager une progression ne donne aucun accès au cahier d'un autre cours : le collègue n'écrit pas dans mon cours A.
    let j = ouvre(&fx, jour(&fx).await + Duration::days(1)).await;
    let _ = fx.seance("CDT-A", p0, Some(fx.service_a), j).await;
    assert!(fx
        .ent
        .cdt_c9_planifier(
            &fx.prof2(),
            id,
            PlanifierBody {
                cours_id: cours_a,
                a_partir_du: None,
                jusqu_au: None,
                apercu: false
            }
        )
        .await
        .is_err_and(|e| est_introuvable(&e) || est_interdit(&e)));
}

#[tokio::test]
async fn c9_planifier_cree_des_brouillons_sur_les_seances_libres_sans_rien_publier_ni_dupliquer() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cours = cours_de(&fx, fx.service_a).await;
    let auj = jour(&fx).await;
    let p0 = fx.cast.profs[0].user_id;
    // Une séance passée, jamais planifiée ; quatre séances à venir.
    let passee = fx
        .seance("CDT-A", p0, Some(fx.service_a), auj - Duration::days(3))
        .await;
    let futures = seances(&fx, "CDT-A", p0, fx.service_a, auj + Duration::days(1), 4).await;
    let p = fx
        .ent
        .cdt_c9_progression_creer(
            &fx.prof(),
            progression(
                "Chapitre",
                vec![etape("Cours", 2), etape("Exercices", 1), etape("Bilan", 2)],
            ),
        )
        .await
        .unwrap();
    let id = p.resume.id;
    let plan = |apercu: bool| PlanifierBody {
        cours_id: cours,
        a_partir_du: None,
        jusqu_au: None,
        apercu,
    };
    // Droits : seul l'intervenant en saisie planifie ; l'autre professeur, la vie scolaire, les élèves non.
    assert!(fx
        .ent
        .cdt_c9_planifier(&fx.prof2(), id, plan(false))
        .await
        .is_err());
    for a in [fx.cpe(), fx.eleve(&fx.x), fx.parent(&fx.x)] {
        assert!(
            fx.ent
                .cdt_c9_planifier(&a, id, plan(false))
                .await
                .is_err_and(|e| est_interdit(&e)),
            "{}",
            a.role
        );
    }
    assert!(fx
        .ent
        .cdt_c9_planifier(&fx.prof(), Uuid::new_v4(), plan(false))
        .await
        .is_err_and(|e| est_introuvable(&e)));
    assert!(fx
        .ent
        .cdt_c9_planifier(
            &fx.prof(),
            id,
            PlanifierBody {
                cours_id: Uuid::new_v4(),
                a_partir_du: None,
                jusqu_au: None,
                apercu: false
            }
        )
        .await
        .is_err_and(|e| est_introuvable(&e)));
    // Aperçu : rien n'est écrit.
    let a = fx
        .ent
        .cdt_c9_planifier(&fx.prof(), id, plan(true))
        .await
        .unwrap();
    assert!(a.apercu && a.creees == 0);
    assert_eq!(a.affectations.len(), 4, "quatre séances libres");
    assert_eq!(
        a.sans_seance.len(),
        1,
        "5 séances voulues, 4 disponibles : la fin du bilan manque"
    );
    assert!(
        a.affectations.iter().all(|x| x.seance_id != passee),
        "jamais une séance passée"
    );
    assert_eq!(
        a.affectations
            .iter()
            .map(|x| x.seance_id)
            .collect::<Vec<_>>(),
        futures.iter().map(|f| f.0).collect::<Vec<_>>()
    );
    let (n0,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_cdt_entree WHERE cours_id = $1")
        .bind(cours)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(n0, 0);
    // Planification réelle : 4 brouillons, rattachés à leur étape, auteur = l'appelant, jamais visibles.
    let r = fx
        .ent
        .cdt_c9_planifier(&fx.prof(), id, plan(false))
        .await
        .unwrap();
    assert_eq!(r.creees, 4);
    let lignes: Vec<(String, String, Option<Uuid>, Option<Uuid>)> =
        sqlx::query_as("SELECT etat, COALESCE(titre, ''), progression_etape_id, auteur_user_id FROM ent_cdt_entree WHERE cours_id = $1 ORDER BY titre")
            .bind(cours)
            .fetch_all(fx.ent.pool())
            .await
            .unwrap();
    assert_eq!(lignes.len(), 4);
    assert!(lignes
        .iter()
        .all(|l| l.0 == "brouillon" && l.2.is_some() && l.3 == Some(p0)));
    assert!(
        lignes.iter().any(|l| l.1 == "Cours (1/2)") && lignes.iter().any(|l| l.1 == "Exercices")
    );
    // Les élèves ne voient rien d'une séance qui n'a qu'un brouillon (préparer n'est pas publier).
    let (du, au) = (auj, auj + Duration::days(20));
    for e in [&fx.x, &fx.z] {
        assert!(
            fx.ent
                .cahier_seances_eleve(&fx.eleve(e), None, du, au)
                .await
                .unwrap()
                .iter()
                .all(|s| s.contenu.is_none()),
            "un brouillon n'est visible d'aucun élève"
        );
    }
    // Idempotent : replanifier ne duplique rien ; une séance de plus ne complète que la fin du bilan.
    let r2 = fx
        .ent
        .cdt_c9_planifier(&fx.prof(), id, plan(false))
        .await
        .unwrap();
    assert_eq!(r2.creees, 0);
    assert_eq!(r2.deja_planifiees, 2);
    let nouvelle = seances(&fx, "CDT-A", p0, fx.service_a, auj + Duration::days(12), 1).await;
    let r3 = fx
        .ent
        .cdt_c9_planifier(&fx.prof(), id, plan(false))
        .await
        .unwrap();
    assert_eq!((r3.creees, r3.sans_seance.len()), (1, 0));
    assert_eq!(r3.affectations[0].seance_id, nouvelle[0].0);
    assert_eq!(r3.affectations[0].rang, 2);
    // Fenêtre bornée : « jusqu'au » avant le début refusé.
    assert!(fx
        .ent
        .cdt_c9_planifier(
            &fx.prof(),
            id,
            PlanifierBody {
                cours_id: cours,
                a_partir_du: Some(auj + Duration::days(5)),
                jusqu_au: Some(auj),
                apercu: true
            }
        )
        .await
        .is_err());
    // Une étape déjà planifiée ne se supprime pas de la progression.
    let mut b = progression("Chapitre", vec![]);
    b.etapes = vec![EtapeBody {
        id: Some(p.etapes[0].id),
        ..etape("Cours", 2)
    }];
    assert!(fx
        .ent
        .cdt_c9_progression_modifier(&fx.prof(), id, b)
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
    // Une séance annulée n'est jamais planifiée.
    let annulee = seances(&fx, "CDT-A", p0, fx.service_a, auj + Duration::days(20), 1).await;
    sqlx::query("UPDATE ent_edt_seance SET statut = 'annulee' WHERE id = $1")
        .bind(annulee[0].0)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    let p4 = fx
        .ent
        .cdt_c9_progression_creer(&fx.prof(), progression("Autre", vec![etape("Seule", 1)]))
        .await
        .unwrap();
    let r4 = fx
        .ent
        .cdt_c9_planifier(
            &fx.prof(),
            p4.resume.id,
            PlanifierBody {
                cours_id: cours,
                a_partir_du: Some(auj + Duration::days(20)),
                jusqu_au: None,
                apercu: false,
            },
        )
        .await
        .unwrap();
    assert_eq!((r4.creees, r4.sans_seance.len()), (0, 1));
    // Cahier figé (clôture) : plus de planification.
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(fx.cast.etablissement_id)
        .await
        .unwrap();
    fx.ent
        .cdt_c8_cloturer(
            &fx.direction(),
            ent_types::cahier_c8::ClotureBody {
                annee_id: annee.id,
                confirmer: true,
            },
        )
        .await
        .unwrap();
    assert!(fx
        .ent
        .cdt_c9_planifier(&fx.prof(), p4.resume.id, plan(false))
        .await
        .is_err_and(|e| est_refus(&e, RefusMetier::Conflit)));
}

#[tokio::test]
async fn c9_reconduire_copie_sans_dates_sans_partage_et_sans_donnee_d_eleve() {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let annee = fx
        .ent
        .referentiel_annee_courante_assuree(fx.cast.etablissement_id)
        .await
        .unwrap();
    let p = fx
        .ent
        .cdt_c9_progression_creer(
            &fx.prof(),
            progression("Année N", vec![etape("A", 2), etape("B", 1)]),
        )
        .await
        .unwrap();
    // Une progression d'autrui, privée, ne se reconduit pas.
    assert!(fx
        .ent
        .cdt_c9_reconduire(&fx.prof2(), p.resume.id, ReconduireBody { annee_id: None })
        .await
        .is_err_and(|e| est_introuvable(&e)));
    assert!(fx
        .ent
        .cdt_c9_reconduire(
            &fx.direction(),
            p.resume.id,
            ReconduireBody { annee_id: None }
        )
        .await
        .is_err_and(|e| est_introuvable(&e)));
    for a in [fx.cpe(), fx.eleve(&fx.x), fx.parent(&fx.x)] {
        assert!(fx
            .ent
            .cdt_c9_reconduire(&a, p.resume.id, ReconduireBody { annee_id: None })
            .await
            .is_err_and(|e| est_interdit(&e)));
    }
    // Reconduction : copie privée, neuve, sans partage, liée à l'originale ; l'année demandée doit exister.
    fx.ent
        .cdt_c9_partager(
            &fx.prof(),
            p.resume.id,
            PartageBody {
                cours_id: Some(cours_de(&fx, fx.service_a).await),
                matiere_id: None,
            },
        )
        .await
        .unwrap();
    assert!(fx
        .ent
        .cdt_c9_reconduire(
            &fx.prof(),
            p.resume.id,
            ReconduireBody {
                annee_id: Some(Uuid::new_v4())
            }
        )
        .await
        .is_err_and(|e| est_introuvable(&e)));
    let c = fx
        .ent
        .cdt_c9_reconduire(
            &fx.prof(),
            p.resume.id,
            ReconduireBody {
                annee_id: Some(annee.id),
            },
        )
        .await
        .unwrap();
    assert_ne!(c.resume.id, p.resume.id);
    assert_eq!(c.resume.copie_de_id, Some(p.resume.id));
    assert_eq!(c.resume.annee_id, Some(annee.id));
    assert!(c.partages.is_empty(), "le partage n'est pas reconduit");
    assert_eq!(c.etapes.len(), 2);
    assert!(
        c.etapes
            .iter()
            .all(|e| !p.etapes.iter().any(|o| o.id == e.id)),
        "étapes neuves : aucun lien avec les entrées de l'ancien cahier"
    );
    assert_eq!(
        c.etapes
            .iter()
            .map(|e| (e.ordre, e.titre.as_str(), e.duree_seances))
            .collect::<Vec<_>>(),
        vec![(1, "A", 2), (2, "B", 1)]
    );
    // Modifier la copie ne touche pas l'originale.
    fx.ent
        .cdt_c9_progression_modifier(
            &fx.prof(),
            c.resume.id,
            progression("Année N+1", vec![etape("Z", 1)]),
        )
        .await
        .unwrap();
    assert_eq!(
        fx.ent
            .cdt_c9_progression_lire(&fx.prof(), p.resume.id)
            .await
            .unwrap()
            .etapes
            .len(),
        2
    );
}

#[tokio::test]
async fn c9_copier_les_entrees_d_un_cours_a_l_autre_en_brouillon_sans_donnee_d_eleve_ni_entree_d_autrui(
) {
    let Some(fx) = fixture().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = fx.cast.etablissement_id;
    let (p0, p1) = (fx.cast.profs[0].user_id, fx.cast.profs[1].user_id);
    let src = cours_de(&fx, fx.service_a).await;
    let svc_b = service(&fx, p0, fx.classe_b, None).await;
    let cible = cours_de(&fx, svc_b).await;
    let auj = jour(&fx).await;
    // Source : trois entrées de prof0 (une publiée, un brouillon, une retirée) et une entrée écrite par prof2 (co-enseignant fictif).
    let s1 = fx
        .seance("CDT-A", p0, Some(fx.service_a), auj - Duration::days(6))
        .await;
    let s2 = fx
        .seance("CDT-A", p0, Some(fx.service_a), auj - Duration::days(5))
        .await;
    let s3 = fx
        .seance("CDT-A", p0, Some(fx.service_a), auj - Duration::days(4))
        .await;
    let s4 = fx
        .seance("CDT-A", p0, Some(fx.service_a), auj - Duration::days(3))
        .await;
    let ent = |s, c: &'static str, p: &'static str| {
        let fx = &fx;
        async move {
            fx.ent
                .cdt_entree_enregistrer(
                    &fx.prof(),
                    src,
                    s,
                    EntreeBody {
                        titre: Some(c.into()),
                        contenu: format!("Contenu {c}"),
                        notions: vec!["Notion".into()],
                        publication: p.into(),
                        publie_a: None,
                        publie_jour: None,
                        publie_heure: None,
                        sans_avis: false,
                        motif: None,
                    },
                )
                .await
                .unwrap()
        }
    };
    let e1 = ent(s1, "Un", "maintenant").await;
    ent(s2, "Deux", "brouillon").await;
    let e3 = ent(s3, "Trois", "maintenant").await;
    fx.ent
        .cdt_entree_retirer(&fx.prof(), e3.id, "erreur de saisie")
        .await
        .unwrap();
    ent(s4, "Quatre", "maintenant").await;
    // Entrée d'un autre auteur (insertion directe : prof2 n'enseigne pas ce cours).
    let autre_seance = fx
        .seance("CDT-A", p0, Some(fx.service_a), auj - Duration::days(2))
        .await;
    sqlx::query("INSERT INTO ent_cdt_entree (id, etablissement_id, cours_id, seance_id, annee_id, contenu, etat, auteur_user_id) SELECT $1, $2, $3, $4, annee_id, 'Entrée d''un collègue', 'publiee', $5 FROM ent_cours WHERE id = $3")
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(src)
        .bind(autre_seance)
        .bind(p1)
        .execute(fx.ent.pool())
        .await
        .unwrap();
    // Un travail et une contribution du cours source ne sont jamais copiés.
    let (nt0,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM ent_cdt_travail WHERE etablissement_id = $1")
            .bind(etab)
            .fetch_one(fx.ent.pool())
            .await
            .unwrap();
    // Cible : séances à venir en nombre suffisant pour 3 entrées (Un, Deux, Quatre), puis insuffisant dans un second essai.
    let futures = seances(&fx, "CDT-B", p0, svc_b, auj + Duration::days(1), 2).await;
    let corps = |apercu: bool| CopierCoursBody {
        source_cours_id: src,
        cible_cours_id: cible,
        a_partir_du: None,
        apercu,
    };
    // Droits : il faut avoir enseigné le cours source ET écrire dans le cours cible.
    assert!(
        fx.ent
            .cdt_c9_copier_cours(&fx.prof2(), corps(false))
            .await
            .is_err_and(|e| est_interdit(&e)),
        "prof2 n'a jamais enseigné le cours source"
    );
    for a in [fx.cpe(), fx.eleve(&fx.x), fx.parent(&fx.x)] {
        assert!(
            fx.ent
                .cdt_c9_copier_cours(&a, corps(false))
                .await
                .is_err_and(|e| est_interdit(&e)),
            "{}",
            a.role
        );
    }
    assert!(fx
        .ent
        .cdt_c9_copier_cours(
            &fx.prof(),
            CopierCoursBody {
                source_cours_id: src,
                cible_cours_id: src,
                a_partir_du: None,
                apercu: false
            }
        )
        .await
        .is_err());
    assert!(fx
        .ent
        .cdt_c9_copier_cours(
            &fx.prof(),
            CopierCoursBody {
                source_cours_id: Uuid::new_v4(),
                cible_cours_id: cible,
                a_partir_du: None,
                apercu: false
            }
        )
        .await
        .is_err_and(|e| est_introuvable(&e)));
    assert!(fx
        .ent
        .cdt_c9_copier_cours(
            &fx.prof(),
            CopierCoursBody {
                source_cours_id: src,
                cible_cours_id: Uuid::new_v4(),
                a_partir_du: None,
                apercu: false
            }
        )
        .await
        .is_err_and(|e| est_introuvable(&e)));
    // Aperçu : rien n'est écrit ; 3 entrées à copier pour 2 séances libres.
    let a = fx
        .ent
        .cdt_c9_copier_cours(&fx.prof(), corps(true))
        .await
        .unwrap();
    assert_eq!(
        (a.copiees.len(), a.sans_seance, a.ignorees_autrui),
        (2, 1, 1)
    );
    let (n0,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_cdt_entree WHERE cours_id = $1")
        .bind(cible)
        .fetch_one(fx.ent.pool())
        .await
        .unwrap();
    assert_eq!(n0, 0);
    let r = fx
        .ent
        .cdt_c9_copier_cours(&fx.prof(), corps(false))
        .await
        .unwrap();
    assert_eq!(r.copiees.len(), 2);
    assert_eq!(
        r.copiees
            .iter()
            .map(|c| c.seance_cible_id)
            .collect::<Vec<_>>(),
        futures.iter().map(|f| f.0).collect::<Vec<_>>()
    );
    let l: Vec<(String, Option<String>, String, Option<Uuid>, Option<Uuid>)> =
        sqlx::query_as("SELECT etat, titre, contenu, auteur_user_id, copie_de_id FROM ent_cdt_entree WHERE cours_id = $1 ORDER BY (SELECT jour FROM ent_edt_seance WHERE id = seance_id)")
            .bind(cible)
            .fetch_all(fx.ent.pool())
            .await
            .unwrap();
    assert_eq!(l.len(), 2);
    assert!(
        l.iter().all(|x| x.0 == "brouillon" && x.3 == Some(p0)),
        "brouillons, au nom de l'appelant"
    );
    assert_eq!(l[0].2, "Contenu Un");
    assert_eq!(l[0].4, Some(e1.id));
    assert!(
        l.iter().all(|x| !x.2.contains("collègue")),
        "l'entrée d'autrui n'est jamais copiée"
    );
    assert!(
        l.iter().all(|x| !x.2.contains("Trois")),
        "une entrée retirée n'est pas copiée"
    );
    let (nt1,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM ent_cdt_travail WHERE etablissement_id = $1")
            .bind(etab)
            .fetch_one(fx.ent.pool())
            .await
            .unwrap();
    assert_eq!(nt1, nt0, "aucun travail copié");
    let (cn,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_cdt_entree_notion n JOIN ent_cdt_entree e ON e.id = n.entree_id WHERE e.cours_id = $1").bind(cible).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(cn, 2, "les notions suivent");
    let (liens,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_cdt_remise r JOIN ent_cdt_travail t ON t.id = r.travail_id WHERE t.cours_id = $1").bind(cible).fetch_one(fx.ent.pool()).await.unwrap();
    assert_eq!(liens, 0, "aucune remise, aucune donnée d'élève");
    // Les séances de destination sont maintenant occupées : une seconde copie n'a plus de place.
    let r2 = fx
        .ent
        .cdt_c9_copier_cours(&fx.prof(), corps(false))
        .await
        .unwrap();
    assert_eq!((r2.copiees.len(), r2.sans_seance), (0, 3));
}
