//! Socle 21, lot V5 « parcours » : tests sur base (feature `labo`), CE-01 à CE-24 (ceux qui relèvent du socle 21).
//! Aucune horloge réelle : jour fixe 2031-10-20 (monde V1 : année 2031-2032), dates d'évaluation fixes.

use super::*;
use crate::tests_evaluations::labo;
use crate::tests_evaluations_v1::{
    affecter, arreter, creer_eleve, d, evaluation, exec, monde_v1, note, statut, t, u, DIR, PROF, W,
};
use chrono::NaiveDate;
use domain_eleve::evaluation_moteur::{Comptage, StatutCalcul};
use domain_eleve::DomainError;
use ent_types::evaluations_v5::*;
use uuid::Uuid;

fn jour() -> NaiveDate {
    d(2031, 10, 20)
}

/// Moyenne affichée (millièmes) d'un élève dans une matière pour une période (`ent_periode`).
async fn moyenne(
    ent: &PgEnt,
    w: &W,
    eleve: Uuid,
    matiere: Uuid,
    periode: Uuid,
    publiees: bool,
) -> Option<i64> {
    let m = ent
        .eva_v1_moyennes_eleve_periode(w.etab, eleve, periode, publiees)
        .await
        .unwrap();
    m.matieres
        .iter()
        .find(|x| x.matiere_id == matiere)
        .and_then(|x| x.moyenne.affichee_m)
}

async fn ecrire(ent: &PgEnt, w: &W, ev: Uuid, eleve: Uuid, valeur_m: i64) {
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, eleve, valeur_m))
        .await
        .unwrap();
}

async fn eval(
    ent: &PgEnt,
    w: &W,
    cours: Uuid,
    date: NaiveDate,
    titre: &str,
    publiee: bool,
) -> Uuid {
    evaluation(
        ent,
        w,
        cours,
        date,
        titre,
        20.0,
        1.0,
        Comptage::Normal,
        publiee,
    )
    .await
}

/// Un cours d'une matière pour la classe ou le groupe donné, tenu par `prof` ; rend l'identifiant du cours.
async fn cours_de(
    ent: &PgEnt,
    w: &W,
    prof: Uuid,
    matiere: Uuid,
    classe: Option<Uuid>,
    groupe: Option<Uuid>,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_service_enseignement (id, etablissement_id, annee_id, prof_user_id, matiere_id, matiere_libelle, classe_id, groupe_id)
         VALUES ($1, $2, $3, $4, $5, '', $6, $7)",
    )
    .bind(id)
    .bind(w.etab)
    .bind(w.annee)
    .bind(prof)
    .bind(matiere)
    .bind(classe)
    .bind(groupe)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query_scalar(
        "SELECT cours_id FROM ent_service_enseignement WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.etab)
    .bind(id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap()
}

async fn groupe(ent: &PgEnt, w: &W, code: &str) -> Uuid {
    let g = Uuid::new_v4();
    exec(
        ent,
        "INSERT INTO ent_groupe (id, etablissement_id, annee_id, code, libelle) VALUES ($1, $2, $3, $4, $4)",
        vec![u(g), u(w.etab), u(w.annee), t(&format!("{code}{}", &g.simple().to_string()[..5]))],
    )
    .await;
    g
}

/// Affectation de groupe (sans classe), datée, avec fin éventuelle.
async fn dans_le_groupe(
    ent: &PgEnt,
    w: &W,
    eleve: Uuid,
    g: Uuid,
    du: NaiveDate,
    au: Option<NaiveDate>,
) {
    sqlx::query(
        "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut, date_fin, source)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'saisie')",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(w.annee)
    .bind(eleve)
    .bind(g)
    .bind(du)
    .bind(au)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

/// Fait passer un élève de la classe `de` à la classe `vers` : fin la veille, début le jour du changement.
async fn changer_de_classe(ent: &PgEnt, w: &W, eleve: Uuid, de: Uuid, vers: Uuid, le: NaiveDate) {
    sqlx::query(
        "UPDATE ent_affectation SET date_fin = $4 WHERE etablissement_id = $1 AND eleve_id = $2 AND classe_id = $3 AND date_fin IS NULL",
    )
    .bind(w.etab)
    .bind(eleve)
    .bind(de)
    .bind(le.pred_opt().unwrap())
    .execute(ent.pool_verif())
    .await
    .unwrap();
    affecter(ent, w.etab, w.annee, eleve, vers, le).await;
}

async fn matiere(ent: &PgEnt, w: &W, code: &str, libelle: &str) -> Uuid {
    let m = Uuid::new_v4();
    exec(
        ent,
        "INSERT INTO ent_matiere (id, etablissement_id, code, libelle) VALUES ($1, $2, $3, $4)",
        vec![
            u(m),
            u(w.etab),
            t(&format!("{code}{}", &m.simple().to_string()[..5])),
            t(libelle),
        ],
    )
    .await;
    m
}

fn autre_prof(cast: &LaboCast, w: &W) -> Uuid {
    cast.profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)
        .expect("second professeur")
}

/// Période de notes historique (`ent_note_periode`) rattachée à `periode`, pour les lecteurs de classe.
async fn note_periode(ent: &PgEnt, w: &W, periode: Uuid, du: NaiveDate, au: NaiveDate) -> Uuid {
    let np = Uuid::new_v4();
    exec(
        ent,
        "INSERT INTO ent_note_periode (id, etablissement_id, code, libelle, date_debut, date_fin, statut, periode_id)
         VALUES ($1, $2, $3, 'Période', $4, $5, 'ouverte', $6)",
        vec![u(np), u(w.etab), t(&format!("P-{}", &np.simple().to_string()[..6])), dt(du), dt(au), u(periode)],
    )
    .await;
    np
}

use crate::tests_evaluations_v1::dt;

fn externe(
    eleve: Uuid,
    matiere: Uuid,
    valeur_m: i64,
    date: NaiveDate,
) -> DeclarerResultatExterneBody {
    DeclarerResultatExterneBody {
        eleve_id: eleve,
        matiere_id: matiere,
        origine: "cned".into(),
        libelle: Some("Devoir CNED".into()),
        date_resultat: Some(date),
        periode_id: None,
        valeur_m,
        bareme_m: 20_000,
        coef_m: None,
        document_id: None,
    }
}

// ───────────────────────── CE-01, CE-04, CE-05 : arrivée tardive, une seule note, absent toute la période ─────────

#[tokio::test]
async fn v5_ce01_ce04_ce05_arrivee_tardive_une_seule_note_absent_toute_la_periode() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let nouveau = creer_eleve(&ent, w.etab, "Nouveau").await;
    affecter(&ent, w.etab, w.annee, nouveau, w.classe_a, d(2031, 11, 4)).await;
    let avant = eval(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 10, 10),
        "DS d'octobre",
        true,
    )
    .await;
    let apres = eval(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 11, 10),
        "DS de novembre",
        true,
    )
    .await;
    let (a, b, absent) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    for e in [a, b] {
        ecrire(&ent, &w, avant, e, 10_000).await;
    }
    // CE-05 : absent à tout (statut, jamais un zéro) : aucune moyenne.
    for ev in [avant, apres] {
        ent.eva_v1_ecrire_resultat_for_actor(
            w.etab,
            w.prof,
            PROF,
            &statut(ev, absent, StatutCalcul::Absent),
        )
        .await
        .unwrap();
    }
    // CE-01 : le nouveau n'était pas concerné par l'évaluation d'octobre : rien ne s'écrit, ni note ni absence.
    for essai in [
        note(avant, nouveau, 12_000),
        statut(avant, nouveau, StatutCalcul::Absent),
    ] {
        let r = ent
            .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &essai)
            .await;
        assert!(r.is_err(), "élève non concerné à la date : {r:?}");
    }
    let lignes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM eva_resultat WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(w.etab)
    .bind(nouveau)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(lignes, 0);
    ecrire(&ent, &w, apres, nouveau, 12_000).await;
    // CE-04 : une seule note, la moyenne se calcule sur elle ; la moyenne de l'ancien n'est pas touchée.
    assert_eq!(
        moyenne(&ent, &w, nouveau, w.maths, w.t1, false).await,
        Some(12_000)
    );
    assert_eq!(
        moyenne(&ent, &w, a, w.maths, w.t1, false).await,
        Some(10_000)
    );
    // CE-05 : absent toute la période : pas de moyenne.
    assert_eq!(moyenne(&ent, &w, absent, w.maths, w.t1, false).await, None);
    let g = ent
        .eva_v1_moyennes_eleve_periode(w.etab, absent, w.t1, false)
        .await
        .unwrap()
        .generale;
    assert!(g.exacte.is_none(), "aucune moyenne générale sans note");
    // Le parcours dit la même chose : « arrivé le 4 novembre », octobre non concerné, novembre concerné.
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), nouveau, w.t1)
        .await
        .unwrap();
    assert_eq!(p.arrive_le, Some(d(2031, 11, 4)));
    let ligne = |ev: Uuid| {
        p.evaluations
            .iter()
            .find(|x| x.evaluation_id == ev)
            .unwrap()
    };
    assert!(!ligne(avant).concerne && ligne(avant).statut.is_none());
    assert!(ligne(apres).concerne && ligne(apres).statut.as_deref() == Some("note"));
    assert_eq!(p.classe_rattachement_id, Some(w.classe_a));
    // Règle 1 : rien n'est publié pour la famille côté moyenne calculée sur les seules évaluations publiées ?
    // (Les deux évaluations sont publiées ici ; la règle est éprouvée plus bas pour les résultats externes.)
    assert_eq!(
        moyenne(&ent, &w, nouveau, w.maths, w.t1, true).await,
        Some(12_000)
    );
}

// ───────────────────────── CE-02, CE-06, CE-07 : résultats externes ─────────────────────────

#[tokio::test]
async fn v5_ce02_ce07_un_resultat_externe_ne_compte_pas_par_defaut_decision_tracee_publication_retrait(
) {
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
    let ev = eval(&ent, &w, w.cours_maths, d(2031, 10, 10), "DS 1", true).await;
    ecrire(&ent, &w, ev, a, 10_000).await;
    let prof_fr = autre_prof(&cast, &w);
    // Déclaration : naît non comptée, non publiée.
    let x = ent
        .eva_v5_declarer_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &externe(a, w.maths, 18_000, d(2031, 10, 15)),
        )
        .await
        .unwrap();
    assert!(x.compte_dans_cours_id.is_none() && !x.publiee && !x.retiree);
    assert_eq!(
        moyenne(&ent, &w, a, w.maths, w.t1, false).await,
        Some(10_000),
        "ne compte pas par défaut (CE-02)"
    );
    // Validations de la déclaration.
    let mut mauvais = externe(a, w.maths, 25_000, d(2031, 10, 15));
    assert!(matches!(
        ent.eva_v5_declarer_externe_for_actor(w.etab, w.prof, PROF, jour(), &mauvais)
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    mauvais = externe(a, w.maths, 10_000, d(2031, 12, 1));
    assert!(
        matches!(
            ent.eva_v5_declarer_externe_for_actor(w.etab, w.prof, PROF, jour(), &mauvais)
                .await,
            Err(EntDbError::SaisieInvalide(_))
        ),
        "date future"
    );
    // La famille et l'élève ne déclarent rien.
    for (u_, role) in [
        (cast.parent_lea.user_id, "parent"),
        (cast.parent_lea.user_id, "eleve"),
    ] {
        let r = ent
            .eva_v5_declarer_externe_for_actor(
                w.etab,
                u_,
                role,
                jour(),
                &externe(a, w.maths, 10_000, d(2031, 10, 15)),
            )
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
    // Compter : seul le professeur du cours (ou la direction) ; l'autre professeur est refusé, le cours doit enseigner la matière.
    let corps = CompterExterneBody {
        cours_id: w.cours_maths,
        motif: "Relevé CNED vérifié".into(),
    };
    let r = ent
        .eva_v5_compter_externe_for_actor(w.etab, prof_fr, PROF, jour(), x.id, &corps)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let r = ent
        .eva_v5_compter_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            x.id,
            &CompterExterneBody {
                cours_id: w.cours_fr,
                motif: "Essai".into(),
            },
        )
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("matière")),
        "{r:?}"
    );
    let r = ent
        .eva_v5_compter_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            x.id,
            &CompterExterneBody {
                cours_id: w.cours_maths,
                motif: "x".into(),
            },
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::SaisieInvalide(_))),
        "motif obligatoire : {r:?}"
    );
    let compte = ent
        .eva_v5_compter_externe_for_actor(w.etab, w.prof, PROF, jour(), x.id, &corps)
        .await
        .unwrap();
    assert_eq!(compte.compte_dans_cours_id, Some(w.cours_maths));
    assert_eq!(compte.compte_motif.as_deref(), Some("Relevé CNED vérifié"));
    let par: (Option<Uuid>, Option<chrono::DateTime<chrono::Utc>>) = sqlx::query_as(
        "SELECT compte_decide_par, compte_decide_le FROM eva_resultat_externe WHERE id = $1",
    )
    .bind(x.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(par.0, Some(w.prof));
    assert!(par.1.is_some(), "la décision est attribuée et datée");
    // Personnel : (10 + 18) / 2 = 14. Famille : le résultat externe n'est pas publié, il n'existe pas pour elle (règle 1).
    assert_eq!(
        moyenne(&ent, &w, a, w.maths, w.t1, false).await,
        Some(14_000)
    );
    assert_eq!(
        moyenne(&ent, &w, a, w.maths, w.t1, true).await,
        Some(10_000)
    );
    let fam = ent
        .eva_v5_externes_famille_for_actor(w.etab, cast.parent_lea.user_id, "parent", a)
        .await
        .unwrap();
    assert!(fam.is_empty());
    // La décision ne se rejoue pas.
    let r = ent
        .eva_v5_compter_externe_for_actor(w.etab, w.prof, PROF, jour(), x.id, &corps)
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    // Publier : second geste ; la famille voit une mention neutre, sans auteur ni motif ; l'autre parent (sans droit) rien.
    let pub_ = ent
        .eva_v5_publier_externe_for_actor(w.etab, w.prof, PROF, jour(), x.id)
        .await
        .unwrap();
    assert!(pub_.publiee);
    assert_eq!(
        moyenne(&ent, &w, a, w.maths, w.t1, true).await,
        Some(14_000)
    );
    let fam = ent
        .eva_v5_externes_famille_for_actor(w.etab, cast.parent_lea.user_id, "parent", a)
        .await
        .unwrap();
    assert_eq!(fam.len(), 1);
    assert_eq!((fam[0].valeur_m, fam[0].bareme_m), (18_000, 20_000));
    assert_eq!(fam[0].mention, MENTION_AUTRE_ORGANISME);
    let r = ent
        .eva_v5_externes_famille_for_actor(w.etab, cast.parent_tom.user_id, "parent", a)
        .await;
    assert!(
        matches!(r, Err(EntDbError::Forbidden)),
        "lien sans droit sur les notes : {r:?}"
    );
    // Rien ne se modifie ni ne se supprime en base.
    assert!(
        sqlx::query("UPDATE eva_resultat_externe SET valeur = 1 WHERE id = $1")
            .bind(x.id)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM eva_resultat_externe WHERE id = $1")
            .bind(x.id)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    // Retrait motivé : la moyenne revient, la famille ne voit plus rien ; une seule fois.
    let r = ent
        .eva_v5_retirer_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            x.id,
            &RetirerExterneBody { motif: "".into() },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    let ret = ent
        .eva_v5_retirer_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            x.id,
            &RetirerExterneBody {
                motif: "Relevé erroné".into(),
            },
        )
        .await
        .unwrap();
    assert!(ret.retiree);
    assert_eq!(
        moyenne(&ent, &w, a, w.maths, w.t1, false).await,
        Some(10_000)
    );
    assert_eq!(
        moyenne(&ent, &w, a, w.maths, w.t1, true).await,
        Some(10_000)
    );
    assert!(ent
        .eva_v5_externes_famille_for_actor(w.etab, cast.parent_lea.user_id, "parent", a)
        .await
        .unwrap()
        .is_empty());
    let r = ent
        .eva_v5_retirer_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            x.id,
            &RetirerExterneBody {
                motif: "Encore".into(),
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    // Une ligne inconnue : droit refusé pour le professeur, « introuvable » seulement pour la direction.
    let r = ent
        .eva_v5_publier_externe_for_actor(w.etab, w.prof, PROF, jour(), Uuid::new_v4())
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let r = ent
        .eva_v5_publier_externe_for_actor(w.etab, w.direction, DIR, jour(), Uuid::new_v4())
        .await;
    assert!(
        matches!(r, Err(EntDbError::Domain(DomainError::NotFound))),
        "{r:?}"
    );
    // Le personnel voit tout, retrait compris, avec le motif.
    let tous = ent
        .eva_v5_externes_eleve_for_actor(w.etab, w.direction, DIR, jour(), a)
        .await
        .unwrap();
    assert_eq!(tous.len(), 1);
    assert_eq!(tous[0].motif_retrait.as_deref(), Some("Relevé erroné"));
}

#[tokio::test]
async fn v5_ce02_apres_l_arret_seule_la_direction_decide_de_compter_un_resultat_externe() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = eval(&ent, &w, w.cours_maths, d(2031, 10, 10), "DS 1", false).await;
    ecrire(&ent, &w, ev, a, 10_000).await;
    let x = ent
        .eva_v5_declarer_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &externe(a, w.maths, 16_000, d(2031, 10, 12)),
        )
        .await
        .unwrap();
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let corps = CompterExterneBody {
        cours_id: w.cours_maths,
        motif: "Relevé du lycée d'origine".into(),
    };
    let r = ent
        .eva_v5_compter_externe_for_actor(w.etab, w.prof, PROF, jour(), x.id, &corps)
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("arrêtée")),
        "{r:?}"
    );
    ent.eva_v5_compter_externe_for_actor(w.etab, w.direction, DIR, jour(), x.id, &corps)
        .await
        .unwrap();
    assert_eq!(
        moyenne(&ent, &w, a, w.maths, w.t1, false).await,
        Some(13_000)
    );
}

// ───────────────────────── CE-06 : enseignement suivi ailleurs, moyenne externe ─────────────────────────

#[tokio::test]
async fn v5_ce06_une_moyenne_externe_vaut_moyenne_de_matiere_sans_double_compte() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let decl = |m: Uuid, compte: bool| DeclarerEnseignementExterneBody {
        eleve_id: a,
        matiere_id: m,
        organisme: "cned".into(),
        organisme_libelle: Some("CNED".into()),
        du: d(2031, 9, 1),
        au: None,
        referent: None,
        compte_moyenne_generale: compte,
    };
    let saisie = |valeur: Option<i64>, st: &str| SaisirMoyenneExterneBody {
        periode_id: w.t1,
        statut: st.into(),
        valeur_m: valeur,
        bareme_m: None,
        appreciation_transcrite: None,
        document_id: None,
    };
    // Le professeur ne déclare pas : direction seulement.
    let r = ent
        .eva_v5_declarer_enseignement_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            &decl(w.francais, true),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let fr = ent
        .eva_v5_declarer_enseignement_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            &decl(w.francais, true),
        )
        .await
        .unwrap();
    let ma = ent
        .eva_v5_declarer_enseignement_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            &decl(w.maths, false),
        )
        .await
        .unwrap();
    // Incohérences refusées.
    for (v, st) in [
        (Some(1_000), "en_attente"),
        (None, "moyenne"),
        (Some(25_000), "moyenne"),
        (None, "autre"),
    ] {
        let r = ent
            .eva_v5_saisir_moyenne_externe_for_actor(
                w.etab,
                w.direction,
                DIR,
                fr.id,
                &saisie(v, st),
            )
            .await;
        assert!(
            matches!(r, Err(EntDbError::SaisieInvalide(_))),
            "{st} {v:?} : {r:?}"
        );
    }
    let r = ent
        .eva_v5_saisir_moyenne_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            fr.id,
            &saisie(Some(15_000), "moyenne"),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    ent.eva_v5_saisir_moyenne_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        fr.id,
        &saisie(Some(15_000), "moyenne"),
    )
    .await
    .unwrap();
    ent.eva_v5_saisir_moyenne_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        ma.id,
        &saisie(Some(5_000), "moyenne"),
    )
    .await
    .unwrap();
    // Français : moyenne externe comptée (CE-06) ; maths : déclarée « ne compte pas » : aucune moyenne.
    assert_eq!(
        moyenne(&ent, &w, a, w.francais, w.t1, false).await,
        Some(15_000)
    );
    assert_eq!(moyenne(&ent, &w, a, w.maths, w.t1, false).await, None);
    // Correction : journalisée en ajout seul (avant, après, auteur).
    ent.eva_v5_saisir_moyenne_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        fr.id,
        &saisie(Some(16_000), "moyenne"),
    )
    .await
    .unwrap();
    assert_eq!(
        moyenne(&ent, &w, a, w.francais, w.t1, false).await,
        Some(16_000)
    );
    let histo: Vec<(Option<i64>, Option<i64>, Uuid)> = sqlx::query_as(
        "SELECT (valeur_avant * 1000)::bigint, (valeur_apres * 1000)::bigint, par FROM eva_moyenne_externe_historique
          WHERE etablissement_id = $1 AND enseignement_externe_id = $2 ORDER BY le, id",
    )
    .bind(w.etab)
    .bind(fr.id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(histo.len(), 2);
    assert_eq!((histo[0].0, histo[0].1), (None, Some(15_000)));
    assert_eq!((histo[1].0, histo[1].1), (Some(15_000), Some(16_000)));
    assert!(histo.iter().all(|h| h.2 == w.direction));
    assert!(sqlx::query("DELETE FROM eva_moyenne_externe_historique")
        .execute(ent.pool_verif())
        .await
        .is_err());
    // Jamais de double compte : une note de l'établissement dans la même matière l'emporte, avec avertissement.
    let ev = eval(&ent, &w, w.cours_fr, d(2031, 10, 10), "Dictée", false).await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, autre_prof(&cast, &w), PROF, &note(ev, a, 12_000))
        .await
        .unwrap();
    assert_eq!(
        moyenne(&ent, &w, a, w.francais, w.t1, false).await,
        Some(12_000)
    );
    let g = ent
        .eva_v1_moyennes_eleve_periode(w.etab, a, w.t1, false)
        .await
        .unwrap()
        .generale;
    assert!(
        g.avertissements
            .iter()
            .any(|m| m.contains("moyenne externe")),
        "{:?}",
        g.avertissements
    );
    // Le détail personnel porte les deux moyennes ; rien d'autre n'est servi à la famille.
    let liste = ent
        .eva_v5_enseignements_externes_for_actor(w.etab, w.direction, DIR, jour(), a)
        .await
        .unwrap();
    assert_eq!(liste.len(), 2);
}

// ───────────────────────── CE-03 : élève parti ─────────────────────────

#[tokio::test]
async fn v5_ce03_un_eleve_parti_garde_sa_moyenne_et_sort_des_statistiques() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, parti) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let ev = eval(&ent, &w, w.cours_maths, d(2031, 10, 10), "DS 1", false).await;
    for (e, v) in [(a, 10_000), (b, 14_000), (parti, 2_000)] {
        ecrire(&ent, &w, ev, e, v).await;
    }
    let np = note_periode(&ent, &w, w.t1, d(2031, 9, 1), d(2031, 12, 20)).await;
    let avant = ent
        .eva_calculer_classe(w.etab, w.classe_a, np, false)
        .await
        .unwrap();
    assert_eq!(avant.stats_matiere[&w.maths].effectif, 3);
    // Départ le 20 novembre : fin d'affectation et date de sortie.
    exec(
        &ent,
        "UPDATE ent_affectation SET date_fin = '2031-11-20' WHERE etablissement_id = $1 AND eleve_id = $2",
        vec![u(w.etab), u(parti)],
    )
    .await;
    exec(
        &ent,
        "UPDATE ent_eleve SET date_sortie = '2031-11-20' WHERE etablissement_id = $1 AND id = $2",
        vec![u(w.etab), u(parti)],
    )
    .await;
    let apres = ent
        .eva_calculer_classe(w.etab, w.classe_a, np, false)
        .await
        .unwrap();
    assert!(
        apres.moyennes.contains_key(&(parti, w.maths)),
        "il garde sa moyenne (bulletin de départ sur demande)"
    );
    assert!(apres.eleves.iter().any(|(e, _, _)| *e == parti));
    assert_eq!(
        apres.stats_matiere[&w.maths].effectif, 2,
        "il sort des statistiques de la classe"
    );
    assert_eq!(
        moyenne(&ent, &w, parti, w.maths, w.t1, false).await,
        Some(2_000)
    );
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), parti, w.t1)
        .await
        .unwrap();
    assert_eq!(p.parti_le, Some(d(2031, 11, 20)));
}

// ───────────────────────── CE-09, CE-10, D15 : changement de classe ─────────────────────────

#[tokio::test]
async fn v5_ce09_ce10_d15_changement_de_classe_la_note_suit_l_eleve_un_seul_bulletin() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let s = creer_eleve(&ent, w.etab, "Mobile").await;
    affecter(&ent, w.etab, w.annee, s, w.classe_a, d(2031, 9, 1)).await;
    let cours_d = cours_de(&ent, &w, w.prof, w.maths, Some(w.classe_d), None).await;
    // Notes : 2 dans la classe A (dont la veille du changement), 1 dans la classe D.
    let e1 = eval(&ent, &w, w.cours_maths, d(2031, 10, 20), "DS A1", false).await;
    let veille = eval(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 11, 19),
        "DS A2 (veille)",
        false,
    )
    .await;
    let apres_dans_a = eval(
        &ent,
        &w,
        w.cours_maths,
        d(2031, 11, 21),
        "DS A3 (après)",
        false,
    )
    .await;
    let e3 = eval(&ent, &w, cours_d, d(2031, 11, 25), "DS D1", false).await;
    ecrire(&ent, &w, e1, s, 14_000).await;
    ecrire(&ent, &w, veille, s, 10_000).await;
    changer_de_classe(&ent, &w, s, w.classe_a, w.classe_d, d(2031, 11, 20)).await;
    // L'évaluation de la classe A du 21 novembre ne le concerne plus (rien ne s'écrit) ; celle de la classe D, si.
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(apres_dans_a, s, 5_000))
        .await
        .is_err());
    ecrire(&ent, &w, e3, s, 12_000).await;
    // Les trois notes comptent, une fois : (14 + 10 + 12) / 3 = 12.
    assert_eq!(
        moyenne(&ent, &w, s, w.maths, w.t1, false).await,
        Some(12_000)
    );
    // Un seul bulletin : il est dans la classe de rattachement (D), pas dans l'ancienne (A).
    let eff = |classe: Uuid| {
        let ent = &ent;
        let etab = w.etab;
        async move {
            ent.eva_effectif(etab, Some(classe), None, d(2031, 9, 1), d(2031, 12, 20))
                .await
                .unwrap()
        }
    };
    assert!(
        !eff(w.classe_a).await.iter().any(|(e, _, _)| *e == s),
        "l'ancienne classe ne produit pas de bulletin pour lui"
    );
    assert!(eff(w.classe_d).await.iter().any(|(e, _, _)| *e == s));
    let np = note_periode(&ent, &w, w.t1, d(2031, 9, 1), d(2031, 12, 20)).await;
    let ca = ent
        .eva_calculer_classe(w.etab, w.classe_a, np, false)
        .await
        .unwrap();
    let cd = ent
        .eva_calculer_classe(w.etab, w.classe_d, np, false)
        .await
        .unwrap();
    assert!(!ca.moyennes.contains_key(&(s, w.maths)) && !ca.generales.contains_key(&s));
    assert!(cd.moyennes.contains_key(&(s, w.maths)));
    assert_eq!(
        cd.moyennes[&(s, w.maths)].0,
        12_000,
        "et il porte les notes des deux classes"
    );
    // Le parcours montre les deux classes et l'origine de chaque évaluation.
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), s, w.t1)
        .await
        .unwrap();
    assert_eq!(p.classe_rattachement_id, Some(w.classe_d));
    assert_eq!(p.segments.len(), 2);
    let classe_a_code: String = sqlx::query_scalar("SELECT code FROM ent_classe WHERE id = $1")
        .bind(w.classe_a)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let ligne = |ev: Uuid| {
        p.evaluations
            .iter()
            .find(|x| x.evaluation_id == ev)
            .unwrap()
            .clone()
    };
    assert_eq!(
        ligne(e1).classe_code.as_deref(),
        Some(classe_a_code.as_str())
    );
    assert!(ligne(veille).concerne && ligne(e3).concerne && !ligne(apres_dans_a).concerne);
}

/// CE-09 avec une classe en trimestres (A) et une classe en semestres (D) : la note de l'ancienne classe ne disparaît pas du
/// bulletin du semestre, et n'est comptée qu'une fois par périodisation.
#[tokio::test]
async fn v5_ce09_changement_de_periodisation_la_note_n_est_ni_perdue_ni_comptee_deux_fois() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let pz = Uuid::new_v4();
    exec(
        &ent,
        "INSERT INTO ent_periodisation (id, etablissement_id, annee_id, code, libelle, usage) VALUES ($1, $2, $3, 'SEM', 'Semestres', 'evaluation')",
        vec![u(pz), u(w.etab), u(w.annee)],
    )
    .await;
    exec(
        &ent,
        "INSERT INTO ent_periodisation_cible (id, etablissement_id, annee_id, usage, periodisation_id, classe_id)
         VALUES ($1, $2, $3, 'evaluation', $4, $5)",
        vec![u(Uuid::new_v4()), u(w.etab), u(w.annee), u(pz), u(w.classe_d)],
    )
    .await;
    let s1 = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_periode (id, etablissement_id, annee_id, periodisation_id, code, libelle, kind, ordre,
                                  date_debut, date_fin, date_arret_notes, origine)
         VALUES ($1, $2, $3, $4, 'S1', 'Semestre 1', 'semestre', 1, '2031-09-01', '2032-01-31', '2032-01-20', 'service')",
    )
    .bind(s1)
    .bind(w.etab)
    .bind(w.annee)
    .bind(pz)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let s = creer_eleve(&ent, w.etab, "Mobile").await;
    affecter(&ent, w.etab, w.annee, s, w.classe_a, d(2031, 9, 1)).await;
    let cours_d = cours_de(&ent, &w, w.prof, w.maths, Some(w.classe_d), None).await;
    let ea = eval(&ent, &w, w.cours_maths, d(2031, 10, 20), "DS A", false).await;
    ecrire(&ent, &w, ea, s, 14_000).await;
    changer_de_classe(&ent, &w, s, w.classe_a, w.classe_d, d(2031, 11, 20)).await;
    let ed = eval(&ent, &w, cours_d, d(2032, 1, 10), "DS D", false).await;
    ecrire(&ent, &w, ed, s, 10_000).await;
    // Bulletin du semestre (classe D) : les DEUX notes (la note de la classe A n'est pas perdue).
    assert_eq!(moyenne(&ent, &w, s, w.maths, s1, false).await, Some(12_000));
    // Dans la périodisation en trimestres : le 20 octobre est au T1, le 10 janvier au T2 : une seule fois chacune.
    assert_eq!(
        moyenne(&ent, &w, s, w.maths, w.t1, false).await,
        Some(14_000)
    );
    assert_eq!(
        moyenne(&ent, &w, s, w.maths, w.t2, false).await,
        Some(10_000)
    );
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), s, s1)
        .await
        .unwrap();
    assert_eq!(p.classe_rattachement_id, Some(w.classe_d));
    assert_eq!(p.evaluations.len(), 2);
}

// ───────────────────────── CE-11, CE-12 : changement de groupe de même matière ─────────────────────────

#[tokio::test]
async fn v5_ce11_ce12_changement_de_groupe_les_deux_cours_alimentent_la_moyenne_de_matiere() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let prof2 = autre_prof(&cast, &w);
    let (g1, g2) = (groupe(&ent, &w, "G1").await, groupe(&ent, &w, "G2").await);
    let c1 = cours_de(&ent, &w, w.prof, w.maths, None, Some(g1)).await;
    let c2 = cours_de(&ent, &w, prof2, w.maths, None, Some(g2)).await;
    let s = w.eleves_a[0];
    dans_le_groupe(&ent, &w, s, g1, d(2031, 9, 1), Some(d(2031, 11, 2))).await;
    dans_le_groupe(&ent, &w, s, g2, d(2031, 11, 3), None).await;
    let e1 = eval(&ent, &w, c1, d(2031, 10, 20), "DS groupe 1", false).await;
    let e2 = eval(&ent, &w, c2, d(2031, 11, 10), "DS groupe 2", false).await;
    ecrire(&ent, &w, e1, s, 16_000).await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, prof2, PROF, &note(e2, s, 12_000))
        .await
        .unwrap();
    assert_eq!(
        moyenne(&ent, &w, s, w.maths, w.t1, false).await,
        Some(14_000),
        "les deux cours comptent"
    );
    // Chacun voit ses seules évaluations : ni l'un ni l'autre n'écrit dans le cours de l'autre.
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, prof2, PROF, &note(e1, s, 1_000))
        .await
        .is_err());
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(e2, s, 1_000))
        .await
        .is_err());
    // À la date de l'évaluation du groupe 1 le 10 novembre, l'élève n'y est plus : non concerné.
    let e1_tard = eval(&ent, &w, c1, d(2031, 11, 10), "DS groupe 1 tardif", false).await;
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(e1_tard, s, 1_000))
        .await
        .is_err());
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), s, w.t1)
        .await
        .unwrap();
    assert!(
        !p.evaluations
            .iter()
            .find(|x| x.evaluation_id == e1_tard)
            .unwrap()
            .concerne
    );
    assert_eq!(
        p.enseignements
            .iter()
            .filter(
                |e| e.matiere_libelle == "Mathématiques" && (e.cours_id == c1 || e.cours_id == c2)
            )
            .count(),
        2
    );
}

// ───────────────────────── CE-13, CE-14, CE-15 : option abandonnée ─────────────────────────

#[tokio::test]
async fn v5_ce13_ce15_option_abandonnee_la_moyenne_reste_sur_la_periode_suivie() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let latin = matiere(&ent, &w, "LAT", "Latin").await;
    let g = groupe(&ent, &w, "LAT").await;
    let c = cours_de(&ent, &w, w.prof, latin, None, Some(g)).await;
    let s = w.eleves_a[0];
    dans_le_groupe(&ent, &w, s, g, d(2031, 9, 1), Some(d(2031, 11, 2))).await;
    let avant = eval(&ent, &w, c, d(2031, 10, 15), "Version latine", false).await;
    ecrire(&ent, &w, avant, s, 15_000).await;
    let apres = eval(&ent, &w, c, d(2031, 11, 20), "Thème latin", false).await;
    // Après l'abandon : non concerné, ni note ni absence ; la moyenne de l'option reste celle de la période suivie.
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(apres, s, 3_000))
        .await
        .is_err());
    assert!(ent
        .eva_v1_ecrire_resultat_for_actor(
            w.etab,
            w.prof,
            PROF,
            &statut(apres, s, StatutCalcul::Absent)
        )
        .await
        .is_err());
    assert_eq!(moyenne(&ent, &w, s, latin, w.t1, false).await, Some(15_000));
    assert_eq!(
        moyenne(&ent, &w, s, latin, w.t2, false).await,
        None,
        "absente des périodes suivantes"
    );
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), s, w.t1)
        .await
        .unwrap();
    let ens = p
        .enseignements
        .iter()
        .find(|e| e.cours_id == c)
        .expect("l'option figure au parcours");
    assert_eq!(
        ens.abandonne_le,
        Some(d(2031, 11, 2)),
        "« abandonné le 2 novembre »"
    );
    assert!(
        !p.evaluations
            .iter()
            .find(|x| x.evaluation_id == apres)
            .unwrap()
            .concerne
    );
    // Un élève qui quitte l'établissement n'a pas « abandonné » l'option : le départ n'est pas un abandon.
    let q = w.eleves_a[1];
    dans_le_groupe(&ent, &w, q, g, d(2031, 9, 1), Some(d(2031, 11, 2))).await;
    exec(&ent, "UPDATE ent_affectation SET date_fin = '2031-11-02' WHERE etablissement_id = $1 AND eleve_id = $2 AND classe_id IS NOT NULL", vec![u(w.etab), u(q)]).await;
    exec(
        &ent,
        "UPDATE ent_eleve SET date_sortie = '2031-11-02' WHERE etablissement_id = $1 AND id = $2",
        vec![u(w.etab), u(q)],
    )
    .await;
    let pq = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), q, w.t1)
        .await
        .unwrap();
    assert_eq!(pq.parti_le, Some(d(2031, 11, 2)));
    assert!(pq.enseignements.iter().all(|e| e.abandonne_le.is_none()));
}

// ───────────────────────── CE-16, CE-17, CE-08 : double cursus, élève accueilli ─────────────────────────

#[tokio::test]
async fn v5_ce16_ce17_double_cursus_inclusion_dans_un_cours_la_classe_de_reference_produit_le_bulletin(
) {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    // Un élève d'ULIS (classe D, sa classe de référence) inclus dans le cours de maths de la classe A.
    let ulis = w.eleves_d[0];
    sqlx::query(
        "INSERT INTO ent_cours_membre (id, etablissement_id, cours_id, eleve_id, nature, motif_code, du)
         VALUES ($1, $2, $3, $4, 'inclusion', 'ulis', '2031-09-01')",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(w.cours_maths)
    .bind(ulis)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let ev = eval(&ent, &w, w.cours_maths, d(2031, 10, 10), "DS 1", false).await;
    ecrire(&ent, &w, ev, ulis, 13_000).await;
    assert_eq!(
        moyenne(&ent, &w, ulis, w.maths, w.t1, false).await,
        Some(13_000)
    );
    // Le bulletin est celui de sa classe de RÉFÉRENCE (D), pas de la classe A qui l'accueille pour la matière.
    let eff = |classe: Uuid| {
        let (ent, etab) = (&ent, w.etab);
        async move {
            ent.eva_effectif(etab, Some(classe), None, d(2031, 9, 1), d(2031, 12, 20))
                .await
                .unwrap()
        }
    };
    assert!(eff(w.classe_d).await.iter().any(|(e, _, _)| *e == ulis));
    assert!(!eff(w.classe_a).await.iter().any(|(e, _, _)| *e == ulis));
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), ulis, w.t1)
        .await
        .unwrap();
    assert_eq!(p.classe_rattachement_id, Some(w.classe_d));
    assert!(
        p.enseignements.iter().any(|e| e.cours_id == w.cours_maths),
        "l'enseignement d'inclusion figure au parcours"
    );
    assert!(p
        .evaluations
        .iter()
        .any(|x| x.evaluation_id == ev && x.concerne));
}

#[tokio::test]
async fn v5_ce08_un_eleve_accueilli_sans_classe_n_a_ni_bulletin_ni_statistique() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let hote = creer_eleve(&ent, w.etab, "Correspondant").await;
    sqlx::query(
        "INSERT INTO ent_cours_membre (id, etablissement_id, cours_id, eleve_id, nature, motif_code, du)
         VALUES ($1, $2, $3, $4, 'inclusion', 'accueil_temporaire', '2031-09-01')",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(w.cours_maths)
    .bind(hote)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let ev = eval(&ent, &w, w.cours_maths, d(2031, 10, 10), "DS 1", false).await;
    // Sans classe, l'élève n'a pas de période : il est verrouillé (M8) ; seule la direction écrit, avec un motif.
    let r = ent
        .eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(ev, hote, 9_000))
        .await;
    assert!(
        r.is_err(),
        "le professeur n'écrit pas pour un élève sans période : {r:?}"
    );
    // Aucune statistique, aucun bulletin : il n'est dans l'effectif d'aucune classe.
    for classe in [w.classe_a, w.classe_d] {
        let eff = ent
            .eva_effectif(w.etab, Some(classe), None, d(2031, 9, 1), d(2031, 12, 20))
            .await
            .unwrap();
        assert!(!eff.iter().any(|(e, _, _)| *e == hote));
    }
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), hote, w.t1)
        .await
        .unwrap();
    assert_eq!(p.classe_rattachement_id, None);
    assert!(
        p.evaluations
            .iter()
            .any(|x| x.evaluation_id == ev && x.concerne),
        "il est dans le public du cours"
    );
}

// ───────────────────────── CE-07, CE-19, CE-20 : exclusion temporaire d'un cours ─────────────────────────

#[tokio::test]
async fn v5_ce07_ce19_exclusion_temporaire_du_cours_non_concerne_puis_retour_sans_double_compte() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let s = w.eleves_a[0];
    // Échange à l'étranger / PFMP : exclusion du cours de maths du 3 au 28 novembre.
    sqlx::query(
        "INSERT INTO ent_cours_membre (id, etablissement_id, cours_id, eleve_id, nature, motif_code, du, au)
         VALUES ($1, $2, $3, $4, 'exclusion', 'echange', '2031-11-03', '2031-11-28')",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(w.cours_maths)
    .bind(s)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let avant = eval(&ent, &w, w.cours_maths, d(2031, 10, 20), "Avant", false).await;
    let pendant = eval(&ent, &w, w.cours_maths, d(2031, 11, 12), "Pendant", false).await;
    let apres = eval(&ent, &w, w.cours_maths, d(2031, 12, 3), "Après", false).await;
    ecrire(&ent, &w, avant, s, 12_000).await;
    assert!(
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(pendant, s, 0))
            .await
            .is_err(),
        "non concerné : jamais un zéro"
    );
    ecrire(&ent, &w, apres, s, 16_000).await;
    assert_eq!(
        moyenne(&ent, &w, s, w.maths, w.t1, false).await,
        Some(14_000)
    );
    // Les résultats de l'établissement d'accueil se versent en externes, sans toucher au calcul tant qu'ils ne sont pas comptés.
    let x = ent
        .eva_v5_declarer_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            &DeclarerResultatExterneBody {
                origine: "etranger".into(),
                ..externe(s, w.maths, 8_000, d(2031, 10, 12))
            },
        )
        .await
        .unwrap();
    assert_eq!(
        moyenne(&ent, &w, s, w.maths, w.t1, false).await,
        Some(14_000)
    );
    assert_eq!(x.origine, "etranger");
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), s, w.t1)
        .await
        .unwrap();
    assert!(
        !p.evaluations
            .iter()
            .find(|e| e.evaluation_id == pendant)
            .unwrap()
            .concerne
    );
    assert_eq!(p.resultats_externes.len(), 1);
}

// ───────────────────────── CE-24, règle 3 : famille, droits et visibilité ─────────────────────────

#[tokio::test]
async fn v5_ce24_apres_le_retrait_du_lien_la_famille_ne_lit_plus_ni_moyennes_ni_resultats_externes()
{
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    ent.link_parent(crate::foyer_repo::CreateLien {
        etablissement_id: w.etab,
        eleve_id: a,
        user_id: cast.parent_lea.user_id,
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
    let ev = eval(&ent, &w, w.cours_maths, d(2031, 10, 10), "DS 1", true).await;
    ecrire(&ent, &w, ev, a, 10_000).await;
    let x = ent
        .eva_v5_declarer_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            &externe(a, w.maths, 18_000, d(2031, 10, 15)),
        )
        .await
        .unwrap();
    ent.eva_v5_compter_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        jour(),
        x.id,
        &CompterExterneBody {
            cours_id: w.cours_maths,
            motif: "Relevé vérifié".into(),
        },
    )
    .await
    .unwrap();
    ent.eva_v5_publier_externe_for_actor(w.etab, w.direction, DIR, jour(), x.id)
        .await
        .unwrap();
    let lu = ent
        .eva_v1_moyennes_famille_for_actor(w.etab, cast.parent_lea.user_id, "parent", a, w.t1)
        .await
        .unwrap();
    assert!(lu.matieres.iter().any(|m| m.matiere_id == w.maths));
    assert_eq!(
        ent.eva_v5_externes_famille_for_actor(w.etab, cast.parent_lea.user_id, "parent", a)
            .await
            .unwrap()
            .len(),
        1
    );
    // L'élève majeur retire l'accès de ses parents (socle 4) : plus de notes à partir de là.
    exec(
        &ent,
        "UPDATE ent_eleve_lien SET peut_voir_notes = false WHERE etablissement_id = $1 AND eleve_id = $2",
        vec![u(w.etab), u(a)],
    )
    .await;
    let r = ent
        .eva_v1_moyennes_famille_for_actor(w.etab, cast.parent_lea.user_id, "parent", a, w.t1)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let r = ent
        .eva_v5_externes_famille_for_actor(w.etab, cast.parent_lea.user_id, "parent", a)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // La famille n'a de toute façon aucun accès au parcours ni aux résultats externes du personnel.
    let r = ent
        .eva_v5_parcours_for_actor(w.etab, cast.parent_lea.user_id, "parent", jour(), a, w.t1)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let r = ent
        .eva_v5_externes_eleve_for_actor(w.etab, cast.parent_lea.user_id, "parent", jour(), a)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
}

// ───────────────────────── Cas limite dérivé : quitte l'établissement puis revient ─────────────────────────

/// Un élève qui quitte l'établissement le 2 novembre puis revient le 1ᵉʳ décembre : absent du public entre les deux (non
/// concerné, jamais un zéro), ses notes d'avant et d'après comptent **une seule fois**, il n'est « parti » qu'à la date
/// de sortie encore en vigueur, et il reste dans l'effectif de sa classe.
#[tokio::test]
async fn v5_cas_limite_un_eleve_qui_quitte_puis_revient_ne_perd_ni_ne_double_aucune_note() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let s = creer_eleve(&ent, w.etab, "Revenant").await;
    affecter(&ent, w.etab, w.annee, s, w.classe_a, d(2031, 9, 1)).await;
    let avant = eval(&ent, &w, w.cours_maths, d(2031, 10, 20), "Avant", false).await;
    let pendant = eval(&ent, &w, w.cours_maths, d(2031, 11, 15), "Pendant", false).await;
    let apres = eval(&ent, &w, w.cours_maths, d(2031, 12, 5), "Après", false).await;
    ecrire(&ent, &w, avant, s, 14_000).await;
    // Départ le 2 novembre : fin d'affectation, date de sortie.
    exec(
        &ent,
        "UPDATE ent_affectation SET date_fin = '2031-11-02' WHERE etablissement_id = $1 AND eleve_id = $2",
        vec![u(w.etab), u(s)],
    )
    .await;
    exec(
        &ent,
        "UPDATE ent_eleve SET date_sortie = '2031-11-02' WHERE etablissement_id = $1 AND id = $2",
        vec![u(w.etab), u(s)],
    )
    .await;
    assert!(
        ent.eva_v1_ecrire_resultat_for_actor(w.etab, w.prof, PROF, &note(pendant, s, 0))
            .await
            .is_err(),
        "parti : non concerné, jamais un zéro"
    );
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), s, w.t1)
        .await
        .unwrap();
    assert_eq!(p.parti_le, Some(d(2031, 11, 2)));
    // Retour le 1ᵉʳ décembre : nouvelle affectation, sortie levée.
    affecter(&ent, w.etab, w.annee, s, w.classe_a, d(2031, 12, 1)).await;
    exec(
        &ent,
        "UPDATE ent_eleve SET date_sortie = NULL WHERE etablissement_id = $1 AND id = $2",
        vec![u(w.etab), u(s)],
    )
    .await;
    ecrire(&ent, &w, apres, s, 10_000).await;
    assert_eq!(
        moyenne(&ent, &w, s, w.maths, w.t1, false).await,
        Some(12_000),
        "avant et après comptent, une fois chacune"
    );
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), s, w.t1)
        .await
        .unwrap();
    assert_eq!(
        p.parti_le, None,
        "revenu : plus de date de sortie en vigueur"
    );
    assert_eq!(p.segments.len(), 2, "deux passages dans la même classe");
    assert_eq!(p.classe_rattachement_id, Some(w.classe_a));
    let ligne = |ev: Uuid| {
        p.evaluations
            .iter()
            .find(|x| x.evaluation_id == ev)
            .unwrap()
    };
    assert!(ligne(avant).concerne && !ligne(pendant).concerne && ligne(apres).concerne);
    let eff = ent
        .eva_effectif(
            w.etab,
            Some(w.classe_a),
            None,
            d(2031, 9, 1),
            d(2031, 12, 20),
        )
        .await
        .unwrap();
    assert_eq!(
        eff.iter().filter(|(e, _, _)| *e == s).count(),
        1,
        "une seule fois dans l'effectif"
    );
}
