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
use ent_auth::Action;
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
        motif: None,
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
    // Revue de V5 (M3) : une correction porte un motif.
    let r = ent
        .eva_v5_saisir_moyenne_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            fr.id,
            &saisie(Some(16_000), "moyenne"),
        )
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("motif")),
        "{r:?}"
    );
    ent.eva_v5_saisir_moyenne_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        fr.id,
        &SaisirMoyenneExterneBody {
            motif: Some("Rectification du relevé CNED".into()),
            ..saisie(Some(16_000), "moyenne")
        },
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

// ───────────────────────── Revue de V5 : correctifs E1, M1 à M5, F3, F4, H1, H2 ─────────────────────────

async fn lier_parent(ent: &PgEnt, w: &W, eleve: Uuid, user: Uuid, voit_notes: bool) {
    ent.link_parent(crate::foyer_repo::CreateLien {
        etablissement_id: w.etab,
        eleve_id: eleve,
        user_id: user,
        foyer_id: None,
        lien: "responsable".into(),
        peut_voir_notes: voit_notes,
        peut_voir_vs: true,
        peut_justifier_absence: false,
        peut_autoriser_sorties: false,
        est_payeur: false,
        part_paiement: None,
    })
    .await
    .unwrap();
}

/// Une pièce du socle 7 déposée pour `eleve` (document minimal, sans version).
async fn piece(ent: &PgEnt, w: &W, eleve: Uuid) -> Uuid {
    let entite = Uuid::new_v4();
    exec(
        ent,
        "INSERT INTO ent_doc_entite (id, etablissement_id, type, raison_sociale) VALUES ($1, $2, 'ensemble', 'Entité de test')",
        vec![u(entite), u(w.etab)],
    )
    .await;
    let doc = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_doc_document (id, etablissement_id, nature_code, entite_id, sujet_type, sujet_id, eleve_id, origine,
                                       sensibilite, domaine, titre, titre_neutre, categorie_conservation)
         VALUES ($1, $2, 'scolarite.releve', $3, 'eleve', $4, $4, 'depose', 'C1', 'scolarite', 'Relevé externe',
                 'Document au dossier', 'dossier_scolaire')",
    )
    .bind(doc)
    .bind(w.etab)
    .bind(entite)
    .bind(eleve)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    doc
}

async fn enseignement(
    ent: &PgEnt,
    w: &W,
    eleve: Uuid,
    matiere: Uuid,
    compte: bool,
) -> EnseignementExterneDto {
    ent.eva_v5_declarer_enseignement_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        &DeclarerEnseignementExterneBody {
            eleve_id: eleve,
            matiere_id: matiere,
            organisme: "cned".into(),
            organisme_libelle: Some("CNED".into()),
            du: d(2031, 9, 1),
            au: None,
            referent: None,
            compte_moyenne_generale: compte,
        },
    )
    .await
    .unwrap()
}

fn moy(
    periode: Uuid,
    statut: &str,
    valeur: Option<i64>,
    motif: Option<&str>,
    document: Option<Uuid>,
) -> SaisirMoyenneExterneBody {
    SaisirMoyenneExterneBody {
        periode_id: periode,
        statut: statut.into(),
        valeur_m: valeur,
        bareme_m: None,
        appreciation_transcrite: None,
        document_id: document,
        motif: motif.map(str::to_string),
    }
}

/// E1, M3, M5 : une moyenne externe n'existe pour la famille qu'après publication ; sa correction porte un motif ; le journal
/// ne trace que ce qui change (événement, motif, pièce, auteur exact).
#[tokio::test]
async fn revue_e1_m3_m5_une_moyenne_externe_se_publie_se_corrige_avec_motif_et_se_journalise() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    lier_parent(&ent, &w, a, cast.parent_lea.user_id, true).await;
    let fr = enseignement(&ent, &w, a, w.francais, true).await;
    let saisi = ent
        .eva_v5_saisir_moyenne_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            fr.id,
            &moy(w.t1, "moyenne", Some(15_000), None, None),
        )
        .await
        .unwrap();
    assert!(!saisi.moyennes[0].publiee);
    // E1 : personnel = 15 ; famille = rien tant que la moyenne n'est pas publiée (une note non publiée n'existe pas).
    assert_eq!(
        moyenne(&ent, &w, a, w.francais, w.t1, false).await,
        Some(15_000)
    );
    assert_eq!(moyenne(&ent, &w, a, w.francais, w.t1, true).await, None);
    let famille = ent
        .eva_v1_moyennes_famille_for_actor(w.etab, cast.parent_lea.user_id, "parent", a, w.t1)
        .await
        .unwrap();
    assert!(famille.matieres.iter().all(|m| m.matiere_id != w.francais));
    // Publier : direction seule ; une seule fois.
    let moyenne_id = saisi.moyennes[0].id;
    let r = ent
        .eva_v5_publier_moyenne_externe_for_actor(w.etab, w.prof, PROF, moyenne_id)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let r = ent
        .eva_v5_publier_moyenne_externe_for_actor(w.etab, w.prof, PROF, Uuid::new_v4())
        .await;
    assert!(
        matches!(r, Err(EntDbError::Forbidden)),
        "ligne inconnue : droit refusé : {r:?}"
    );
    let r = ent
        .eva_v5_publier_moyenne_externe_for_actor(w.etab, w.direction, DIR, Uuid::new_v4())
        .await;
    assert!(
        matches!(r, Err(EntDbError::Domain(DomainError::NotFound))),
        "{r:?}"
    );
    let publiee = ent
        .eva_v5_publier_moyenne_externe_for_actor(w.etab, w.direction, DIR, moyenne_id)
        .await
        .unwrap();
    assert!(publiee.moyennes[0].publiee);
    let r = ent
        .eva_v5_publier_moyenne_externe_for_actor(w.etab, w.direction, DIR, moyenne_id)
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    assert_eq!(
        moyenne(&ent, &w, a, w.francais, w.t1, true).await,
        Some(15_000)
    );
    let famille = ent
        .eva_v1_moyennes_famille_for_actor(w.etab, cast.parent_lea.user_id, "parent", a, w.t1)
        .await
        .unwrap();
    assert!(famille.matieres.iter().any(|m| m.matiere_id == w.francais));
    // M3 : correction sans motif, ou avec un motif trop court / trop long : refusée.
    for motif in [None, Some("ab"), Some("x".repeat(501).as_str())] {
        let r = ent
            .eva_v5_saisir_moyenne_externe_for_actor(
                w.etab,
                w.direction,
                DIR,
                fr.id,
                &moy(w.t1, "moyenne", Some(16_000), motif, None),
            )
            .await;
        assert!(
            matches!(r, Err(EntDbError::SaisieInvalide(_))),
            "{motif:?} : {r:?}"
        );
    }
    // Rien ne change : aucune écriture, aucun journal, aucun motif exigé (idempotent).
    ent.eva_v5_saisir_moyenne_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        fr.id,
        &moy(w.t1, "moyenne", Some(15_000), None, None),
    )
    .await
    .unwrap();
    let nb = |ent: &PgEnt| {
        let ent = ent.pool_verif().clone();
        let (etab, id) = (w.etab, fr.id);
        async move {
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM eva_moyenne_externe_historique WHERE etablissement_id = $1 AND enseignement_externe_id = $2",
            )
            .bind(etab)
            .bind(id)
            .fetch_one(&ent)
            .await
            .unwrap()
        }
    };
    assert_eq!(
        nb(&ent).await,
        2,
        "saisie + publication, pas de ligne pour un geste sans effet"
    );
    // Correction motivée (D-069, M-A) : la moyenne REDEVIENT non publiée (la famille ne voit plus rien jusqu'à la nouvelle
    // publication), le journal garde l'ancienne valeur.
    ent.eva_v5_saisir_moyenne_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        fr.id,
        &moy(
            w.t1,
            "moyenne",
            Some(16_000),
            Some("Rectification du relevé CNED"),
            None,
        ),
    )
    .await
    .unwrap();
    assert_eq!(moyenne(&ent, &w, a, w.francais, w.t1, true).await, None);
    assert_eq!(
        moyenne(&ent, &w, a, w.francais, w.t1, false).await,
        Some(16_000)
    );
    // M5 : la pièce d'un autre élève, ou inconnue, est refusée ; la pièce de l'élève est journalisée avec le motif.
    let autre = piece(&ent, &w, b).await;
    for faux in [autre, Uuid::new_v4()] {
        let r = ent
            .eva_v5_saisir_moyenne_externe_for_actor(
                w.etab,
                w.direction,
                DIR,
                fr.id,
                &moy(
                    w.t1,
                    "moyenne",
                    Some(16_000),
                    Some("Ajout de la pièce"),
                    Some(faux),
                ),
            )
            .await;
        assert!(
            matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("pièce")),
            "{r:?}"
        );
    }
    let mienne = piece(&ent, &w, a).await;
    ent.eva_v5_saisir_moyenne_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        fr.id,
        &moy(
            w.t1,
            "moyenne",
            Some(16_000),
            Some("Ajout de la pièce"),
            Some(mienne),
        ),
    )
    .await
    .unwrap();
    type Ligne = (
        String,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<Uuid>,
        Option<Uuid>,
        Uuid,
    );
    let histo: Vec<Ligne> = sqlx::query_as(
        "SELECT evenement, (valeur_avant * 1000)::bigint, (valeur_apres * 1000)::bigint, motif, document_avant, document_apres, par
           FROM eva_moyenne_externe_historique WHERE etablissement_id = $1 AND enseignement_externe_id = $2 ORDER BY le, id",
    )
    .bind(w.etab)
    .bind(fr.id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    let evenements: Vec<&str> = histo.iter().map(|h| h.0.as_str()).collect();
    assert_eq!(
        evenements,
        ["saisie", "publication", "correction", "correction"]
    );
    assert_eq!((histo[2].1, histo[2].2), (Some(15_000), Some(16_000)));
    assert_eq!(histo[2].3.as_deref(), Some("Rectification du relevé CNED"));
    assert_eq!((histo[3].4, histo[3].5), (None, Some(mienne)));
    assert_eq!(histo[3].3.as_deref(), Some("Ajout de la pièce"));
    assert!(histo.iter().all(|h| h.6 == w.direction), "auteur exact");
    // Même garde en base : une pièce de l'élève absente, un résultat externe avec la pièce d'un autre.
    let mut ext = externe(a, w.maths, 10_000, d(2031, 10, 15));
    ext.document_id = Some(autre);
    let r = ent
        .eva_v5_declarer_externe_for_actor(w.etab, w.direction, DIR, jour(), &ext)
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("pièce")),
        "{r:?}"
    );
    ext.document_id = Some(mienne);
    ent.eva_v5_declarer_externe_for_actor(w.etab, w.direction, DIR, jour(), &ext)
        .await
        .unwrap();
    // Défense en profondeur : la base refuse aussi un INSERT direct avec la pièce d'un autre élève.
    let r = sqlx::query(
        "INSERT INTO eva_resultat_externe (id, etablissement_id, eleve_id, matiere_id, origine, valeur, bareme, coef, document_id,
                                           saisi_par, date_resultat)
         VALUES ($1, $2, $3, $4, 'cned', 10, 20, 1, $5, $6, '2031-10-15')",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(a)
    .bind(w.maths)
    .bind(autre)
    .bind(w.direction)
    .execute(ent.pool_verif())
    .await;
    assert!(r.is_err(), "pièce d'un autre élève refusée par la base");
}

/// F3, F4 : la période d'une moyenne externe est de l'année et de la périodisation de l'élève, dans les dates de
/// l'enseignement ; deux saisies concurrentes de la même période : un conflit, jamais une erreur serveur.
#[tokio::test]
async fn revue_f3_f4_periode_de_la_moyenne_externe_et_saisies_concurrentes() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    // Une périodisation de semestres que la classe de a (classe A) n'utilise pas : cible la classe D seulement.
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
    // Enseignement suivi du 1er septembre au 30 novembre : le T2 (21 décembre → 31 mars) est hors de ses dates.
    let ens = ent
        .eva_v5_declarer_enseignement_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            &DeclarerEnseignementExterneBody {
                eleve_id: a,
                matiere_id: w.francais,
                organisme: "cned".into(),
                organisme_libelle: None,
                du: d(2031, 9, 1),
                au: Some(d(2031, 11, 30)),
                referent: None,
                compte_moyenne_generale: true,
            },
        )
        .await
        .unwrap();
    for (periode, pourquoi) in [
        (s1, "périodisation que l'élève n'utilise pas"),
        (w.t2, "hors des dates de l'enseignement"),
        (Uuid::new_v4(), "période inconnue"),
    ] {
        let r = ent
            .eva_v5_saisir_moyenne_externe_for_actor(
                w.etab,
                w.direction,
                DIR,
                ens.id,
                &moy(periode, "moyenne", Some(12_000), None, None),
            )
            .await;
        assert!(
            matches!(r, Err(EntDbError::SaisieInvalide(_))),
            "{pourquoi} : {r:?}"
        );
    }
    // F4 : deux saisies simultanées de la même période (valeurs différentes) : une réussit, l'autre est un conflit (ou
    // une correction sans motif refusée si elle passe après) ; jamais d'erreur de base, une seule ligne.
    let (c1, c2) = (
        moy(w.t1, "moyenne", Some(11_000), None, None),
        moy(w.t1, "moyenne", Some(13_000), None, None),
    );
    let (r1, r2) = tokio::join!(
        ent.eva_v5_saisir_moyenne_externe_for_actor(w.etab, w.direction, DIR, ens.id, &c1,),
        ent.eva_v5_saisir_moyenne_externe_for_actor(w.etab, w.direction, DIR, ens.id, &c2,),
    );
    assert!(r1.is_ok() ^ r2.is_ok(), "{r1:?} / {r2:?}");
    for r in [r1.err(), r2.err()].into_iter().flatten() {
        assert!(
            matches!(
                r,
                EntDbError::ConflitDossier(_) | EntDbError::SaisieInvalide(_)
            ),
            "{r:?}"
        );
    }
    let lignes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM eva_moyenne_externe WHERE etablissement_id = $1 AND enseignement_externe_id = $2",
    )
    .bind(w.etab)
    .bind(ens.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(lignes, 1);
}

/// M1 : publier ou retirer un résultat COMPTÉ après l'arrêt de la période de l'élève : direction seule, comme « compter ».
#[tokio::test]
async fn revue_m1_publier_et_retirer_apres_l_arret_seule_la_direction() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let mut ids = Vec::new();
    for _ in 0..2 {
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
        ent.eva_v5_compter_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            x.id,
            &CompterExterneBody {
                cours_id: w.cours_maths,
                motif: "Relevé du lycée d'origine".into(),
            },
        )
        .await
        .unwrap();
        ids.push(x.id);
    }
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let r = ent
        .eva_v5_publier_externe_for_actor(w.etab, w.prof, PROF, jour(), ids[0])
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("arrêtée")),
        "{r:?}"
    );
    let r = ent
        .eva_v5_retirer_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            ids[0],
            &RetirerExterneBody {
                motif: "Erreur de saisie".into(),
            },
        )
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("arrêtée")),
        "{r:?}"
    );
    // La direction publie et retire.
    assert!(
        ent.eva_v5_publier_externe_for_actor(w.etab, w.direction, DIR, jour(), ids[0])
            .await
            .unwrap()
            .publiee
    );
    assert!(
        ent.eva_v5_retirer_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            ids[1],
            &RetirerExterneBody {
                motif: "Erreur de saisie".into(),
            },
        )
        .await
        .unwrap()
        .retiree
    );
}

/// M2 : un professeur ne lit que les cours et les matières qu'il enseigne ; jamais `dispense` ni les motifs hors direction.
#[tokio::test]
async fn revue_m2_un_professeur_ne_voit_que_ses_matieres_ni_dispense_ni_motifs() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let prof_fr = autre_prof(&cast, &w);
    // Une évaluation dans chaque cours.
    let ev_m = eval(&ent, &w, w.cours_maths, d(2031, 10, 10), "DS maths", false).await;
    let ev_f = eval(&ent, &w, w.cours_fr, d(2031, 10, 11), "Dictée", false).await;
    ecrire(&ent, &w, ev_m, a, 10_000).await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, prof_fr, PROF, &note(ev_f, a, 12_000))
        .await
        .unwrap();
    // Un résultat externe en maths (compté, motif), un en français (non compté) ; un enseignement externe de français
    // « dispensé » (statut sensible) et un de maths.
    let x_m = ent
        .eva_v5_declarer_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &externe(a, w.maths, 18_000, d(2031, 10, 15)),
        )
        .await
        .unwrap();
    ent.eva_v5_compter_externe_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        x_m.id,
        &CompterExterneBody {
            cours_id: w.cours_maths,
            motif: "Motif réservé à la direction".into(),
        },
    )
    .await
    .unwrap();
    ent.eva_v5_declarer_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        jour(),
        &externe(a, w.francais, 9_000, d(2031, 10, 15)),
    )
    .await
    .unwrap();
    let ens_fr = enseignement(&ent, &w, a, w.francais, false).await;
    ent.eva_v5_saisir_moyenne_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        ens_fr.id,
        &moy(w.t1, "dispense", None, None, None),
    )
    .await
    .unwrap();
    let ens_ma = enseignement(&ent, &w, a, w.maths, false).await;
    ent.eva_v5_saisir_moyenne_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        ens_ma.id,
        &moy(w.t1, "dispense", None, None, None),
    )
    .await
    .unwrap();
    // Le professeur de maths : ses seules matières, aucun motif, aucune dispense.
    let ext = ent
        .eva_v5_externes_eleve_for_actor(w.etab, w.prof, PROF, jour(), a)
        .await
        .unwrap();
    assert_eq!(ext.len(), 1);
    assert_eq!(ext[0].matiere_id, w.maths);
    assert!(
        ext[0].compte_motif.is_none() && ext[0].motif_retrait.is_none(),
        "{:?}",
        ext[0]
    );
    let ens = ent
        .eva_v5_enseignements_externes_for_actor(w.etab, w.prof, PROF, jour(), a)
        .await
        .unwrap();
    assert_eq!(
        ens.len(),
        1,
        "l'enseignement de français n'existe pas pour lui"
    );
    assert_eq!(ens[0].matiere_id, w.maths);
    assert!(
        ens[0].moyennes.iter().all(|m| m.statut != "dispense"),
        "{:?}",
        ens[0].moyennes
    );
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, w.prof, PROF, jour(), a, w.t1)
        .await
        .unwrap();
    assert_eq!(p.enseignements.len(), 1);
    assert_eq!(p.enseignements[0].cours_id, w.cours_maths);
    assert_eq!(p.evaluations.len(), 1);
    assert_eq!(p.evaluations[0].evaluation_id, ev_m);
    assert!(p
        .resultats_externes
        .iter()
        .all(|x| x.matiere_id == w.maths && x.compte_motif.is_none()));
    // Le professeur de français voit l'inverse (jamais la matière de l'autre), sans le motif de maths.
    let ext = ent
        .eva_v5_externes_eleve_for_actor(w.etab, prof_fr, PROF, jour(), a)
        .await
        .unwrap();
    assert_eq!(ext.len(), 1);
    assert_eq!(ext[0].matiere_id, w.francais);
    // La direction voit tout, avec les motifs et le statut `dispense`.
    let tous = ent
        .eva_v5_externes_eleve_for_actor(w.etab, w.direction, DIR, jour(), a)
        .await
        .unwrap();
    assert_eq!(tous.len(), 2);
    assert!(tous
        .iter()
        .any(|x| x.compte_motif.as_deref() == Some("Motif réservé à la direction")));
    let ens = ent
        .eva_v5_enseignements_externes_for_actor(w.etab, w.direction, DIR, jour(), a)
        .await
        .unwrap();
    assert_eq!(ens.len(), 2);
    assert!(ens
        .iter()
        .flat_map(|e| &e.moyennes)
        .any(|m| m.statut == "dispense"));
    // Déclarer : un professeur ne déclare que dans une matière qu'il enseigne.
    let r = ent
        .eva_v5_declarer_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &externe(a, w.francais, 10_000, d(2031, 10, 15)),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // F1 : un cours inconnu ne se distingue pas d'un cours qu'on ne peut pas décider (droit refusé pour le professeur).
    let r = ent
        .eva_v5_compter_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            x_m.id,
            &CompterExterneBody {
                cours_id: Uuid::new_v4(),
                motif: "Cours inconnu".into(),
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
}

/// M4 : une moyenne externe ne compte jamais avec des notes de l'établissement dans la matière ; deux moyennes externes
/// d'une même matière sur la période : aucune n'est comptée, avertissement exact (jamais le message des notes de
/// l'établissement).
#[tokio::test]
async fn revue_m4_plusieurs_moyennes_externes_d_une_matiere_et_presence_de_notes_de_l_etablissement(
) {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    // a : deux déclarations en français, une moyenne chacune sur T1, aucune note de l'établissement.
    for v in [12_000, 16_000] {
        let e = enseignement(&ent, &w, a, w.francais, true).await;
        ent.eva_v5_saisir_moyenne_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            e.id,
            &moy(w.t1, "moyenne", Some(v), None, None),
        )
        .await
        .unwrap();
    }
    assert_eq!(moyenne(&ent, &w, a, w.francais, w.t1, false).await, None);
    let g = ent
        .eva_v1_moyennes_eleve_periode(w.etab, a, w.t1, false)
        .await
        .unwrap()
        .generale;
    assert!(
        g.avertissements
            .iter()
            .any(|m| m.contains("2 moyennes externes") && m.contains("aucune n'est comptée")),
        "{:?}",
        g.avertissements
    );
    assert!(
        !g.avertissements.iter().any(|m| m.contains("l'emportent")),
        "un autre message que celui des notes de l'établissement : {:?}",
        g.avertissements
    );
    // b : une seule moyenne externe, sans note de l'établissement : elle compte, sans avertissement.
    let e = enseignement(&ent, &w, b, w.francais, true).await;
    ent.eva_v5_saisir_moyenne_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        e.id,
        &moy(w.t1, "moyenne", Some(14_000), None, None),
    )
    .await
    .unwrap();
    assert_eq!(
        moyenne(&ent, &w, b, w.francais, w.t1, false).await,
        Some(14_000)
    );
    let gb = ent
        .eva_v1_moyennes_eleve_periode(w.etab, b, w.t1, false)
        .await
        .unwrap()
        .generale;
    assert!(
        gb.avertissements.iter().all(|m| !m.contains("moyenne")),
        "{:?}",
        gb.avertissements
    );
    // b a ensuite une note de l'établissement en français : elle l'emporte, avertissement nommant la matière.
    let ev = eval(&ent, &w, w.cours_fr, d(2031, 10, 10), "Dictée", false).await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, autre_prof(&cast, &w), PROF, &note(ev, b, 8_000))
        .await
        .unwrap();
    assert_eq!(
        moyenne(&ent, &w, b, w.francais, w.t1, false).await,
        Some(8_000)
    );
    let gb = ent
        .eva_v1_moyennes_eleve_periode(w.etab, b, w.t1, false)
        .await
        .unwrap()
        .generale;
    assert!(
        gb.avertissements.iter().any(|m| m.contains("l'emportent")),
        "{:?}",
        gb.avertissements
    );
}

/// H1 : une période annuelle (nature « periode ») qui englobe ses trimestres dans la même périodisation porte leurs notes
/// (avant : aucune note) ; chaque note compte une fois dans son trimestre et une fois dans l'année.
#[tokio::test]
async fn revue_h1_une_periode_annuelle_chevauchante_porte_les_notes_de_ses_trimestres() {
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
    let annee = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_periode (id, etablissement_id, annee_id, periodisation_id, code, libelle, kind, ordre,
                                  date_debut, date_fin, date_arret_notes, origine)
         VALUES ($1, $2, $3, $4, 'AN', 'Année', 'periode', 9, '2031-09-01', '2032-06-30', '2032-06-20', 'reprise')",
    )
    .bind(annee)
    .bind(w.etab)
    .bind(w.annee)
    .bind(pz)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let e1 = eval(&ent, &w, w.cours_maths, d(2031, 10, 10), "DS 1", false).await;
    let e2 = eval(&ent, &w, w.cours_maths, d(2031, 10, 13), "DS 2", false).await;
    ecrire(&ent, &w, e1, a, 10_000).await;
    ecrire(&ent, &w, e2, a, 14_000).await;
    // T1 : deux notes comptées UNE fois chacune ; année : les mêmes (avant : aucune note, la période était « sans notes »).
    assert_eq!(
        moyenne(&ent, &w, a, w.maths, w.t1, false).await,
        Some(12_000)
    );
    assert_eq!(
        moyenne(&ent, &w, a, w.maths, annee, false).await,
        Some(12_000)
    );
    // La note du 10 octobre est dans l'année ET dans le T1 ; jamais deux fois dans la même période.
    let (dans_t1, dans_an): (bool, bool) = sqlx::query_as(
        "SELECT eva_periode_dans($1, $2, '2031-10-10'), eva_periode_dans($1, $3, '2031-10-10')",
    )
    .bind(w.etab)
    .bind(w.t1)
    .bind(annee)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(dans_t1 && dans_an);
    let t2_dans_t1: bool = sqlx::query_scalar("SELECT eva_periode_dans($1, $2, '2032-01-10')")
        .bind(w.etab)
        .bind(w.t1)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(!t2_dans_t1, "une note de janvier n'est pas dans le T1");
    let t2_dans_an: bool = sqlx::query_scalar("SELECT eva_periode_dans($1, $2, '2032-01-10')")
        .bind(w.etab)
        .bind(annee)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(t2_dans_an, "mais elle est dans l'année");
}

/// H2 : après la fusion de deux fiches, les résultats externes COMPTÉS de la fiche absorbée entrent dans la moyenne de
/// l'élève conservé (ils restent lisibles et comptés, jamais perdus).
#[tokio::test]
async fn revue_h2_une_fusion_de_fiches_ne_fait_pas_disparaitre_un_resultat_externe_compte() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (absorbe, garde) = (w.eleves_a[0], w.eleves_a[1]);
    let ev = eval(&ent, &w, w.cours_maths, d(2031, 10, 10), "DS 1", false).await;
    ecrire(&ent, &w, ev, garde, 10_000).await;
    let x = ent
        .eva_v5_declarer_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &externe(absorbe, w.maths, 18_000, d(2031, 10, 15)),
        )
        .await
        .unwrap();
    ent.eva_v5_compter_externe_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        x.id,
        &CompterExterneBody {
            cours_id: w.cours_maths,
            motif: "Relevé vérifié".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        moyenne(&ent, &w, garde, w.maths, w.t1, false).await,
        Some(10_000)
    );
    // Fusion de la fiche `absorbe` dans `garde` (marque du socle 2).
    exec(
        &ent,
        "UPDATE ent_eleve SET fusionne_dans = $3 WHERE etablissement_id = $1 AND id = $2",
        vec![u(w.etab), u(absorbe), u(garde)],
    )
    .await;
    assert_eq!(
        moyenne(&ent, &w, garde, w.maths, w.t1, false).await,
        Some(14_000),
        "le résultat externe compté de la fiche absorbée compte pour l'élève conservé"
    );
    let lus = ent
        .eva_v5_externes_eleve_for_actor(w.etab, w.direction, DIR, jour(), garde)
        .await
        .unwrap();
    assert!(lus.iter().any(|y| y.id == x.id));
}

/// M-A (D-069) : une correction d'une moyenne externe PUBLIÉE la remet en « non publiée » (la famille ne voit plus rien
/// jusqu'à la nouvelle publication) et le journal dit qu'elle était visible ; une publication se RETIRE (direction, motif,
/// journal) ; F-3 : la base refuse de publier une ligne qui n'est pas une moyenne.
#[tokio::test]
async fn contre_revue_ma_une_correction_remet_la_moyenne_en_non_publiee_et_le_retrait_est_trace() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    lier_parent(&ent, &w, a, cast.parent_lea.user_id, true).await;
    let fr = enseignement(&ent, &w, a, w.francais, true).await;
    let saisi = ent
        .eva_v5_saisir_moyenne_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            fr.id,
            &moy(w.t1, "moyenne", Some(15_000), None, None),
        )
        .await
        .unwrap();
    let mid = saisi.moyennes[0].id;
    ent.eva_v5_publier_moyenne_externe_for_actor(w.etab, w.direction, DIR, mid)
        .await
        .unwrap();
    assert_eq!(
        moyenne(&ent, &w, a, w.francais, w.t1, true).await,
        Some(15_000)
    );
    // Correction : plus visible pour la famille, visible du personnel, l'état de publication est remis à zéro.
    let corrigee = ent
        .eva_v5_saisir_moyenne_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            fr.id,
            &moy(w.t1, "moyenne", Some(9_000), Some("Relevé corrigé"), None),
        )
        .await
        .unwrap();
    assert!(!corrigee.moyennes[0].publiee, "{corrigee:?}");
    assert_eq!(moyenne(&ent, &w, a, w.francais, w.t1, true).await, None);
    assert_eq!(
        moyenne(&ent, &w, a, w.francais, w.t1, false).await,
        Some(9_000)
    );
    let famille = ent
        .eva_v1_moyennes_famille_for_actor(w.etab, cast.parent_lea.user_id, "parent", a, w.t1)
        .await
        .unwrap();
    assert!(famille.matieres.iter().all(|m| m.matiere_id != w.francais));
    // Nouvelle publication, puis retrait : professeur refusé, motif obligatoire, ligne inconnue, double retrait.
    ent.eva_v5_publier_moyenne_externe_for_actor(w.etab, w.direction, DIR, mid)
        .await
        .unwrap();
    assert_eq!(
        moyenne(&ent, &w, a, w.francais, w.t1, true).await,
        Some(9_000)
    );
    let retrait = |motif: &str| RetirerExterneBody {
        motif: motif.into(),
    };
    let r = ent
        .eva_v5_retirer_publication_moyenne_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            mid,
            &retrait("Erreur de saisie"),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let r = ent
        .eva_v5_retirer_publication_moyenne_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            mid,
            &retrait(""),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    let r = ent
        .eva_v5_retirer_publication_moyenne_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            Uuid::new_v4(),
            &retrait("Erreur de saisie"),
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::Domain(DomainError::NotFound))),
        "{r:?}"
    );
    let retiree = ent
        .eva_v5_retirer_publication_moyenne_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            mid,
            &retrait("Publiée avant vérification"),
        )
        .await
        .unwrap();
    assert!(!retiree.moyennes[0].publiee);
    assert_eq!(moyenne(&ent, &w, a, w.francais, w.t1, true).await, None);
    let r = ent
        .eva_v5_retirer_publication_moyenne_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            mid,
            &retrait("Une seconde fois"),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    // Journal : correction (elle était visible), retrait (motif, auteur), publications.
    let histo: Vec<(String, Option<String>, bool, Uuid)> = sqlx::query_as(
        "SELECT evenement, motif, etait_publiee, par FROM eva_moyenne_externe_historique
          WHERE etablissement_id = $1 AND moyenne_id = $2 ORDER BY le, id",
    )
    .bind(w.etab)
    .bind(mid)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    let evenements: Vec<&str> = histo.iter().map(|h| h.0.as_str()).collect();
    assert_eq!(
        evenements,
        ["saisie", "publication", "correction", "publication", "retrait"]
    );
    assert!(!histo[0].2 && histo[2].2 && histo[4].2, "{histo:?}");
    assert_eq!(histo[2].1.as_deref(), Some("Relevé corrigé"));
    assert_eq!(histo[4].1.as_deref(), Some("Publiée avant vérification"));
    assert!(histo.iter().all(|h| h.3 == w.direction));
    // La famille ne lit pas non plus de motif ni de trace : elle ne voit rien.
    // F-3 en base : une ligne « en attente » ne se publie pas, même par SQL direct.
    let maths = enseignement(&ent, &w, a, w.maths, true).await;
    let attente = ent
        .eva_v5_saisir_moyenne_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            maths.id,
            &moy(w.t1, "en_attente", None, None, None),
        )
        .await
        .unwrap();
    let r = sqlx::query(
        "UPDATE eva_moyenne_externe SET publiee_le = now(), publiee_par = $3 WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(w.etab)
    .bind(attente.moyennes[0].id)
    .bind(w.direction)
    .execute(ent.pool_verif())
    .await;
    assert!(r.is_err(), "seule une moyenne se publie, en base aussi");
}

/// M-B : un seul moteur quelle que soit l'audience : « notes d'établissement présentes » et « plusieurs moyennes
/// externes » se décident sur TOUTES les lignes, publiées ou non ; seul l'affichage est filtré.
#[tokio::test]
async fn contre_revue_mb_le_moteur_decide_sur_toutes_les_lignes_seul_l_affichage_est_filtre() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    // a : deux moyennes externes en français, une seule publiée : aucune n'est comptée, ni pour le personnel ni pour la
    // famille (avant : la famille voyait l'unique moyenne publiée comme « la » moyenne).
    let mut ids = Vec::new();
    for v in [12_000, 16_000] {
        let e = enseignement(&ent, &w, a, w.francais, true).await;
        let m = ent
            .eva_v5_saisir_moyenne_externe_for_actor(
                w.etab,
                w.direction,
                DIR,
                e.id,
                &moy(w.t1, "moyenne", Some(v), None, None),
            )
            .await
            .unwrap();
        ids.push(m.moyennes[0].id);
    }
    ent.eva_v5_publier_moyenne_externe_for_actor(w.etab, w.direction, DIR, ids[0])
        .await
        .unwrap();
    assert_eq!(moyenne(&ent, &w, a, w.francais, w.t1, false).await, None);
    assert_eq!(moyenne(&ent, &w, a, w.francais, w.t1, true).await, None);
    let perso = ent
        .eva_v1_moyennes_eleve_periode(w.etab, a, w.t1, false)
        .await
        .unwrap()
        .generale;
    assert!(
        perso.avertissements.iter().any(|m| m.contains("2 moyennes externes")),
        "{:?}",
        perso.avertissements
    );
    let famille = ent
        .eva_v1_moyennes_eleve_periode(w.etab, a, w.t1, true)
        .await
        .unwrap()
        .generale;
    assert!(
        famille.avertissements.iter().all(|m| !m.contains("2 moyennes") && !m.contains("l'emportent")),
        "la famille n'apprend ni le nombre ni l'existence de ce qui n'est pas publié : {:?}",
        famille.avertissements
    );
    // Les deux publiées : même décision, même résultat.
    ent.eva_v5_publier_moyenne_externe_for_actor(w.etab, w.direction, DIR, ids[1])
        .await
        .unwrap();
    assert_eq!(moyenne(&ent, &w, a, w.francais, w.t1, true).await, None);

    // b : des notes d'établissement NON publiées en français et une moyenne externe publiée : les notes de l'établissement
    // l'emportent pour tous ; la famille ne voit ni la note (non publiée) ni la moyenne externe (qui ne compte pas).
    let e = enseignement(&ent, &w, b, w.francais, true).await;
    let m = ent
        .eva_v5_saisir_moyenne_externe_for_actor(
            w.etab,
            w.direction,
            DIR,
            e.id,
            &moy(w.t1, "moyenne", Some(14_000), None, None),
        )
        .await
        .unwrap();
    ent.eva_v5_publier_moyenne_externe_for_actor(w.etab, w.direction, DIR, m.moyennes[0].id)
        .await
        .unwrap();
    assert_eq!(
        moyenne(&ent, &w, b, w.francais, w.t1, true).await,
        Some(14_000),
        "sans aucune note d'établissement, la moyenne externe publiée compte"
    );
    let ev = eval(&ent, &w, w.cours_fr, d(2031, 10, 10), "Dictée", false).await;
    ent.eva_v1_ecrire_resultat_for_actor(w.etab, autre_prof(&cast, &w), PROF, &note(ev, b, 8_000))
        .await
        .unwrap();
    assert_eq!(
        moyenne(&ent, &w, b, w.francais, w.t1, false).await,
        Some(8_000)
    );
    assert_eq!(
        moyenne(&ent, &w, b, w.francais, w.t1, true).await,
        None,
        "la note non publiée n'existe pas pour la famille, et elle empêche la moyenne externe de compter"
    );
}

/// M-C : après une fusion de fiches, une note ORDINAIRE de la fiche absorbée suit l'élève conservé ; si la fiche
/// conservée a déjà un résultat pour la même évaluation, il l'emporte (jamais deux fois).
#[tokio::test]
async fn contre_revue_mc_une_fusion_de_fiches_rattache_les_notes_ordinaires_a_l_eleve_conserve() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (absorbe, garde) = (w.eleves_a[0], w.eleves_a[1]);
    let ev1 = eval(&ent, &w, w.cours_maths, d(2031, 10, 10), "DS 1", true).await;
    let ev2 = eval(&ent, &w, w.cours_maths, d(2031, 10, 12), "DS 2", true).await;
    ecrire(&ent, &w, ev1, absorbe, 18_000).await;
    ecrire(&ent, &w, ev2, absorbe, 4_000).await;
    ecrire(&ent, &w, ev2, garde, 10_000).await;
    assert_eq!(
        moyenne(&ent, &w, garde, w.maths, w.t1, false).await,
        Some(10_000)
    );
    exec(
        &ent,
        "UPDATE ent_eleve SET fusionne_dans = $3 WHERE etablissement_id = $1 AND id = $2",
        vec![u(w.etab), u(absorbe), u(garde)],
    )
    .await;
    // DS 1 (fiche absorbée) suit l'élève conservé ; DS 2 : le résultat de la fiche conservée (10) l'emporte, le 4 de la
    // fiche absorbée ne compte pas : (18 + 10) / 2 = 14.
    assert_eq!(
        moyenne(&ent, &w, garde, w.maths, w.t1, false).await,
        Some(14_000)
    );
    assert_eq!(
        moyenne(&ent, &w, garde, w.maths, w.t1, true).await,
        Some(14_000)
    );
}

/// F-1 : le professeur ne voit du parcours que ce qui concerne SES cours : segments de classe, rattachement, arrivée et
/// départ limités aux classes où il enseigne et à la période de ses cours ; la direction voit tout.
#[tokio::test]
async fn contre_revue_f1_le_professeur_ne_voit_du_parcours_que_ce_qui_concerne_ses_cours() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    // Un élève de la classe D qui rejoint la classe A le 1er octobre ; le professeur de maths n'enseigne qu'en A.
    let e = w.eleves_d[0];
    changer_de_classe(&ent, &w, e, w.classe_d, w.classe_a, d(2031, 10, 1)).await;
    let dir = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), e, w.t1)
        .await
        .unwrap();
    assert_eq!(dir.segments.len(), 2, "la direction voit les deux classes");
    let prof = ent
        .eva_v5_parcours_for_actor(w.etab, w.prof, PROF, jour(), e, w.t1)
        .await
        .unwrap();
    assert_eq!(prof.enseignements.len(), 1);
    assert_eq!(prof.segments.len(), 1, "{:?}", prof.segments);
    assert_eq!(prof.segments[0].classe_id, w.classe_a);
    assert!(
        prof.segments[0].du >= d(2031, 10, 1),
        "jamais avant le début de ses cours : {:?}",
        prof.segments
    );
    assert_eq!(prof.classe_rattachement_id, Some(w.classe_a));
    // Un élève de la classe D, sans aucun cours du professeur : rien du tout (réponse vide, aucun indice).
    let sans = ent
        .eva_v5_parcours_for_actor(w.etab, w.prof, PROF, jour(), w.eleves_d[1], w.t1)
        .await;
    match sans {
        Err(EntDbError::Forbidden) => {}
        Ok(p) => assert!(
            p.segments.is_empty()
                && p.classe_rattachement_id.is_none()
                && p.arrive_le.is_none()
                && p.parti_le.is_none()
                && p.enseignements.is_empty(),
            "{p:?}"
        ),
        autre => panic!("{autre:?}"),
    }
}

/// F-2 : un assistant ou un intervenant (lecture seule) ne déclare pas de résultat externe, mais lit ; un professeur ne
/// déclare pas pour une année précédente (la direction, si).
#[tokio::test]
async fn contre_revue_f2_un_assistant_ne_declare_pas_et_pas_d_annee_precedente_pour_le_professeur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let assistant = autre_prof(&cast, &w);
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
    .bind(assistant)
    .fetch_optional(ent.pool_verif())
    .await
    .unwrap();
    let Some(agent) = agent else {
        return eprintln!("skip: pas d'agent pour le second professeur du labo");
    };
    sqlx::query(
        "INSERT INTO ent_service_intervenant (id, etablissement_id, service_id, agent_id, nature, du, saisie, origine)
         VALUES ($1, $2, $3, $4, 'assistant', '2031-09-01', false, 'saisie')",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(service)
    .bind(agent)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let r = ent
        .eva_v5_declarer_externe_for_actor(
            w.etab,
            assistant,
            PROF,
            jour(),
            &externe(a, w.maths, 12_000, d(2031, 10, 15)),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Lecture seule : il lit le parcours du cours.
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, assistant, PROF, jour(), a, w.t1)
        .await
        .unwrap();
    // Il tient déjà le français ; le cours de maths s'y ajoute en lecture seule.
    assert!(
        p.enseignements.iter().any(|e| e.cours_id == w.cours_maths),
        "{:?}",
        p.enseignements
    );
    // Le titulaire déclare dans l'année en cours, pas pour une année précédente ; la direction, si.
    ent.eva_v5_declarer_externe_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        &externe(a, w.maths, 12_000, d(2031, 10, 15)),
    )
    .await
    .unwrap();
    let r = ent
        .eva_v5_declarer_externe_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &externe(a, w.maths, 12_000, d(2030, 10, 15)),
        )
        .await;
    assert!(
        matches!(&r, Err(EntDbError::SaisieInvalide(m)) if m.contains("année précédente")),
        "{r:?}"
    );
    ent.eva_v5_declarer_externe_for_actor(
        w.etab,
        w.direction,
        DIR,
        jour(),
        &externe(a, w.maths, 12_000, d(2030, 10, 15)),
    )
    .await
    .unwrap();
}

/// M-D : un résultat « dispense » (donnée de santé) n'apparaît dans le parcours qu'à la direction ; un personnel à portée
/// DÉSIGNÉE sans cours (CPE de niveau, vie scolaire) est autorisé mais ne reçoit AUCUN contenu pédagogique (D-069).
#[tokio::test]
async fn contre_revue_md_dispense_et_portee_designee_sans_cours_dans_le_parcours() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let ev = eval(&ent, &w, w.cours_maths, d(2031, 10, 10), "DS maths", false).await;
    ent.eva_v1_ecrire_resultat_for_actor(
        w.etab,
        w.prof,
        PROF,
        &statut(ev, a, StatutCalcul::Dispense),
    )
    .await
    .unwrap();
    let prof = ent
        .eva_v5_parcours_for_actor(w.etab, w.prof, PROF, jour(), a, w.t1)
        .await
        .unwrap();
    assert_eq!(prof.evaluations.len(), 1);
    assert_eq!(prof.evaluations[0].statut, None, "jamais `dispense` au professeur");
    let dir = ent
        .eva_v5_parcours_for_actor(w.etab, w.direction, DIR, jour(), a, w.t1)
        .await
        .unwrap();
    assert_eq!(dir.evaluations[0].statut.as_deref(), Some("dispense"));
    // Personnel à portée désignée (classe A) sans aucun cours : un rôle « lecture des notes » désigné, sur un compte de
    // comptabilité (qui n'a aucun droit de notes par défaut). L'élève doit être dans la classe désignée AUJOURD'HUI (le
    // droit désigné se calcule à la date réelle de l'établissement) : son affectation démarre en 2020, ouverte.
    exec(
        &ent,
        "UPDATE ent_affectation SET date_debut = '2020-09-01' WHERE etablissement_id = $1 AND eleve_id = $2",
        vec![u(w.etab), u(a)],
    )
    .await;
    let role = ent
        .creer_role_for_actor(
            w.etab,
            cast.admin.user_id,
            "admin",
            crate::droits_repo::RoleSaisie {
                nom: format!("Vie scolaire désignée {}", &Uuid::new_v4().simple().to_string()[..4]),
                description: None,
                modele: None,
                droits: vec![crate::droits_repo::Droit {
                    action: Action::NotesLire,
                    portee: ent_auth::Portee::Designes,
                    cibles: vec![w.classe_a],
                }],
            },
        )
        .await
        .unwrap();
    let vie_scolaire = cast.compta.user_id;
    ent.attribuer_role_for_actor(w.etab, cast.admin.user_id, "admin", role.id, vie_scolaire, None)
        .await
        .unwrap();
    let p = ent
        .eva_v5_parcours_for_actor(w.etab, vie_scolaire, "compta", jour(), a, w.t1)
        .await
        .unwrap();
    assert!(
        p.enseignements.is_empty()
            && p.evaluations.is_empty()
            && p.resultats_externes.is_empty()
            && p.segments.is_empty()
            && p.classe_rattachement_id.is_none(),
        "{p:?}"
    );
    // Hors de sa portée : droit refusé.
    let r = ent
        .eva_v5_parcours_for_actor(w.etab, vie_scolaire, "compta", jour(), w.eleves_d[0], w.t1)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
}
