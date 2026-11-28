//! Socle 21, lot V3f « barème détaillé, conversion des notes, duplication » : tests sur base (feature `labo`). Aucune
//! horloge réelle : jour fixe (2031-10-20, premier trimestre fictif) et « maintenant » fixe. Chaque scénario part d'un
//! établissement neuf du labo.

use super::*;
use crate::tests_evaluations::labo;
use crate::tests_evaluations_v1::{d, monde_v1, DIR, PROF, W};
use chrono::{DateTime, NaiveDate, Utc};
use ent_types::evaluations_v3::*;
use ent_types::evaluations_v3f::*;
use uuid::Uuid;

fn jour() -> NaiveDate {
    d(2031, 10, 20)
}

fn maintenant() -> DateTime<Utc> {
    jour().and_hms_opt(12, 0, 0).unwrap().and_utc()
}

async fn creer(ent: &PgEnt, w: &W, titre: &str, date: NaiveDate, bareme_m: i64) -> Uuid {
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
            bareme_m,
            coef_m: 2000,
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

async fn bareme(ent: &PgEnt, w: &W, ev: Uuid) -> BaremeDetailleDto {
    ent.eva_v3f_bareme_for_actor(w.etab, w.prof, PROF, jour(), ev)
        .await
        .unwrap()
}

async fn definir(
    ent: &PgEnt,
    w: &W,
    ev: Uuid,
    exercices: &[(&str, i64)],
) -> Result<BaremeDetailleDto, EntDbError> {
    let version = bareme(ent, w, ev).await.version;
    ent.eva_v3f_definir_bareme_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        ev,
        &DefinirBaremeBody {
            version_attendue: version,
            exercices: exercices
                .iter()
                .map(|(l, m)| ExerciceSaisiDto {
                    libelle: (*l).into(),
                    maximum_m: *m,
                })
                .collect(),
        },
    )
    .await
}

fn detail(
    eleve: Uuid,
    ex: &[ExerciceDto],
    points: &[i64],
    version: Option<i32>,
) -> DetailEleveSaisiDto {
    DetailEleveSaisiDto {
        eleve_id: eleve,
        points: ex
            .iter()
            .filter(|x| !x.annule)
            .zip(points)
            .map(|(x, p)| PointsExerciceDto {
                exercice_id: x.id,
                points_m: *p,
            })
            .collect(),
        version_attendue: version,
        motif: None,
    }
}

async fn saisir(
    ent: &PgEnt,
    w: &W,
    ev: Uuid,
    eleves: Vec<DetailEleveSaisiDto>,
) -> SaisirDetailsRetourDto {
    ent.eva_v3f_saisir_details_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        ev,
        &SaisirDetailsBody { eleves },
    )
    .await
    .unwrap()
}

async fn note(ent: &PgEnt, w: &W, ev: Uuid, eleve: Uuid) -> (Option<i64>, i32) {
    let (v, ver): (Option<i64>, i32) = sqlx::query_as(
        "SELECT (valeur * 1000)::bigint, version FROM eva_resultat WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(w.etab)
    .bind(ev)
    .bind(eleve)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    (v, ver)
}

async fn case_grille(
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

async fn publier(ent: &PgEnt, w: &W, ev: Uuid) {
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
}

fn autre_prof(cast: &crate::seed::LaboCast) -> Uuid {
    cast.profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != cast.prof.user_id)
        .expect("second professeur")
}

// ───────────────────────── Barème détaillé, détail, réussite ─────────────────────────

#[tokio::test]
async fn v3f_la_note_est_la_somme_des_points_et_la_reussite_ne_classe_personne() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let ev = creer(&ent, &w, "DS 1", d(2031, 10, 10), 20_000).await;
    // La somme des maximums doit égaler le barème.
    let r = definir(&ent, &w, ev, &[("Ex. 1", 8_000), ("Ex. 2", 8_000)]).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    let r = definir(&ent, &w, ev, &[("  ", 20_000)]).await;
    assert!(
        matches!(r, Err(EntDbError::SaisieInvalide(_))),
        "libellé vide : {r:?}"
    );
    let b0 = definir(
        &ent,
        &w,
        ev,
        &[("Ex. 1", 8_000), ("Ex. 2", 8_000), ("Ex. 3", 4_000)],
    )
    .await
    .unwrap();
    assert_eq!(b0.exercices.len(), 3);
    assert_eq!(b0.somme_maximums_m, 20_000);
    assert!(b0.redefinissable);
    // Version périmée : conflit.
    let r = ent
        .eva_v3f_definir_bareme_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            ev,
            &DefinirBaremeBody {
                version_attendue: b0.version - 1,
                exercices: vec![],
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    let ex = b0.exercices.clone();
    let r = saisir(
        &ent,
        &w,
        ev,
        vec![
            detail(a, &ex, &[6_000, 7_500, 2_000], None),
            detail(b, &ex, &[2_000, 3_000, 4_000], None),
            // Dépasse le maximum de l'exercice 3.
            detail(c, &ex, &[1_000, 1_000, 5_000], None),
        ],
    )
    .await;
    assert_eq!(r.eleves[0].issue, "ecrit", "{:?}", r.eleves[0]);
    assert_eq!(r.eleves[0].valeur_m, Some(15_500));
    assert_eq!(r.eleves[1].valeur_m, Some(9_000));
    assert_eq!(r.eleves[2].issue, "refuse");
    // Il manque un exercice : refusé, rien n'est écrit.
    let r = saisir(&ent, &w, ev, vec![detail(c, &ex, &[1_000, 1_000], None)]).await;
    assert_eq!(r.eleves[0].issue, "refuse");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM eva_resultat WHERE evaluation_id = $1 AND eleve_id = $2"
        )
        .bind(ev)
        .bind(c)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap(),
        0
    );
    assert_eq!(note(&ent, &w, ev, a).await.0, Some(15_500));
    // Réussite : moyenne et répartition, sans nom ni rang.
    let b1 = bareme(&ent, &w, ev).await;
    assert_eq!(b1.nb_details_a_jour, 2);
    assert!(!b1.redefinissable, "des points sont saisis");
    let r1 = &b1.reussite[0];
    assert_eq!((r1.effectif, r1.moyenne_m), (2, Some(4_000)));
    // A : 6/8 (trois quarts exacts) → dernière tranche ; B : 2/8 (quart exact) → deuxième tranche.
    assert_eq!(r1.repartition, [0, 1, 0, 1]);
    // Redéfinition refusée une fois des points saisis.
    let r = definir(&ent, &w, ev, &[("Tout", 20_000)]).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    // Une note changée à la main dans la grille rend le détail périmé.
    let v = note(&ent, &w, ev, b).await.1;
    assert_eq!(case_grille(&ent, &w, ev, b, "10", Some(v)).await, "ecrit");
    let b2 = bareme(&ent, &w, ev).await;
    assert_eq!((b2.nb_details_a_jour, b2.nb_details_perimes), (1, 1));
    let det = ent
        .eva_v3f_details_for_actor(w.etab, w.prof, PROF, jour(), ev)
        .await
        .unwrap();
    let lb = det.eleves.iter().find(|e| e.eleve_id == b).unwrap();
    assert!(!lb.a_jour);
    assert_eq!(lb.note_m, Some(10_000));
    // Ressaisir son détail le remet à jour (note recalculée).
    let v = note(&ent, &w, ev, b).await.1;
    let r = saisir(
        &ent,
        &w,
        ev,
        vec![detail(b, &ex, &[3_000, 3_000, 4_000], Some(v))],
    )
    .await;
    assert_eq!(r.eleves[0].issue, "ecrit");
    assert_eq!(bareme(&ent, &w, ev).await.nb_details_a_jour, 2);
    // Base : les points ne dépassent jamais le maximum, et aucun détail ne se supprime.
    let rid: Uuid = sqlx::query_scalar(
        "SELECT id FROM eva_resultat WHERE evaluation_id = $1 AND eleve_id = $2",
    )
    .bind(ev)
    .bind(a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(sqlx::query(
        "UPDATE eva_resultat_item SET valeur = 9 WHERE resultat_id = $1 AND item_id = $2"
    )
    .bind(rid)
    .bind(ex[0].id)
    .execute(ent.pool_verif())
    .await
    .is_err());
    assert!(
        sqlx::query("DELETE FROM eva_resultat_item WHERE resultat_id = $1")
            .bind(rid)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE eva_bareme_item SET maximum = 9 WHERE id = $1")
            .bind(ex[0].id)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn v3f_droits_proprietaire_lecture_direction_autre_professeur_famille() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let ev = creer(&ent, &w, "DS droits", d(2031, 10, 10), 20_000).await;
    let b0 = definir(&ent, &w, ev, &[("Ex. 1", 20_000)]).await.unwrap();
    // La direction lit (comme la grille), ne définit pas le barème d'un professeur.
    assert!(ent
        .eva_v3f_bareme_for_actor(w.etab, w.direction, DIR, jour(), ev)
        .await
        .is_ok());
    let r = ent
        .eva_v3f_definir_bareme_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            ev,
            &DefinirBaremeBody {
                version_attendue: b0.version,
                exercices: vec![],
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Un autre professeur : rien.
    let x = autre_prof(&cast);
    let r = ent
        .eva_v3f_bareme_for_actor(w.etab, x, PROF, jour(), ev)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let r = ent
        .eva_v3f_saisir_details_for_actor(
            w.etab,
            x,
            PROF,
            jour(),
            ev,
            &SaisirDetailsBody { eleves: vec![] },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Famille : jamais les routes du personnel.
    for role in ["parent", "eleve"] {
        let r = ent
            .eva_v3f_details_for_actor(w.etab, cast.parent_lea.user_id, role, jour(), ev)
            .await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
    // Un autre établissement n'en sait rien.
    let r = ent
        .eva_v3f_bareme_for_actor(Uuid::new_v4(), w.prof, PROF, jour(), ev)
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn v3f_la_famille_ne_voit_le_detail_que_publie_montre_et_a_jour() {
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
    let ev = creer(&ent, &w, "DS famille", d(2031, 10, 10), 20_000).await;
    let b0 = definir(&ent, &w, ev, &[("Calcul", 12_000), ("Problème", 8_000)])
        .await
        .unwrap();
    let r = saisir(
        &ent,
        &w,
        ev,
        vec![detail(a, &b0.exercices, &[10_000, 5_000], None)],
    )
    .await;
    assert_eq!(r.eleves[0].issue, "ecrit");
    let lire = |user: Uuid| {
        let ent = &ent;
        let w = &w;
        async move {
            ent.eva_v3f_detail_famille_for_actor(w.etab, user, "parent", a, ev)
                .await
        }
    };
    let lea = cast.parent_lea.user_id;
    // Non publiée : introuvable.
    assert!(matches!(
        lire(lea).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    publier(&ent, &w, ev).await;
    // Publiée mais détail non montré : introuvable.
    assert!(matches!(
        lire(lea).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    let v = bareme(&ent, &w, ev).await.version;
    ent.eva_v3f_visibilite_detail_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        ev,
        &VisibiliteDetailBody {
            version_attendue: v,
            visible: true,
        },
    )
    .await
    .unwrap();
    let dfam = lire(lea).await.unwrap();
    assert_eq!(dfam.note_m, 15_000);
    assert_eq!(dfam.exercices.len(), 2);
    assert_eq!(dfam.exercices[0].points_m, Some(10_000));
    // Le parent dont le lien ne donne pas les notes : refusé.
    assert!(matches!(
        lire(cast.parent_tom.user_id).await,
        Err(EntDbError::Forbidden)
    ));
    // Note changée à la main : détail périmé, plus montré.
    let ver = note(&ent, &w, ev, a).await.1;
    assert_eq!(case_grille(&ent, &w, ev, a, "16", Some(ver)).await, "ecrit");
    assert!(matches!(
        lire(lea).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
}

// ───────────────────────── Conversion (CE-39, CE-83) ─────────────────────────

#[tokio::test]
async fn v3f_ce39_changer_de_bareme_convertit_les_notes_apres_apercu_et_garde_l_historique() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    // Notes sur 10 saisies sur une évaluation créée sur 20 par erreur : on passe à 10.
    let ev = creer(&ent, &w, "Interro", d(2031, 10, 10), 10_000).await;
    assert_eq!(case_grille(&ent, &w, ev, a, "7", None).await, "ecrit");
    assert_eq!(case_grille(&ent, &w, ev, b, "4,5", None).await, "ecrit");
    assert_eq!(case_grille(&ent, &w, ev, c, "abs", None).await, "ecrit");
    let v = bareme(&ent, &w, ev).await.version;
    let demande = ConversionBody {
        version_attendue: v,
        nouveau_bareme_m: Some(20_000),
        annuler_exercice_id: None,
    };
    let ap = ent
        .eva_v3f_apercu_conversion_for_actor(w.etab, w.prof, PROF, jour(), ev, &demande)
        .await
        .unwrap();
    assert_eq!((ap.bareme_avant_m, ap.bareme_apres_m), (10_000, 20_000));
    assert_eq!(
        ap.lignes.len(),
        2,
        "l'absent n'est pas une note : il ne se convertit pas"
    );
    let la = ap.lignes.iter().find(|l| l.eleve_id == a).unwrap();
    assert_eq!(
        (la.avant_m, la.apres_m, la.source.as_str()),
        (7_000, 14_000, "proportion")
    );
    // L'aperçu n'écrit rien.
    assert_eq!(note(&ent, &w, ev, a).await.0, Some(7_000));
    let appliquer = |empreinte: String, motif: &str| AppliquerConversionBody {
        version_attendue: v,
        nouveau_bareme_m: Some(20_000),
        annuler_exercice_id: None,
        empreinte,
        motif: motif.into(),
    };
    // Motif obligatoire.
    let r = ent
        .eva_v3f_appliquer_conversion_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            ev,
            &appliquer(ap.empreinte.clone(), " "),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    // Une note change entre l'aperçu et la confirmation : l'empreinte ne correspond plus, rien n'est écrit.
    let vb = note(&ent, &w, ev, b).await.1;
    assert_eq!(case_grille(&ent, &w, ev, b, "5", Some(vb)).await, "ecrit");
    let r = ent
        .eva_v3f_appliquer_conversion_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            ev,
            &appliquer(ap.empreinte.clone(), "Barème saisi sur 10 au lieu de 20"),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    assert_eq!(note(&ent, &w, ev, a).await.0, Some(7_000));
    let ap = ent
        .eva_v3f_apercu_conversion_for_actor(w.etab, w.prof, PROF, jour(), ev, &demande)
        .await
        .unwrap();
    let ok = ent
        .eva_v3f_appliquer_conversion_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            ev,
            &appliquer(ap.empreinte.clone(), "Barème saisi sur 10 au lieu de 20"),
        )
        .await
        .unwrap();
    assert_eq!((ok.bareme_m, ok.nb_notes_converties), (20_000, 2));
    assert_eq!(note(&ent, &w, ev, a).await.0, Some(14_000));
    assert_eq!(note(&ent, &w, ev, b).await.0, Some(10_000));
    // Historique : l'ancienne valeur reste, avec le motif ; le barème est journalisé.
    let h = ent
        .eva_v3_historique_for_actor(w.etab, w.prof, PROF, ev, a, jour())
        .await
        .unwrap();
    assert!(
        h.iter()
            .any(|l| l.valeur_avant_m == Some(7_000) && l.valeur_apres_m == Some(14_000)),
        "{h:?}"
    );
    let (avant, apres, motif): (i64, i64, Option<String>) = sqlx::query_as(
        "SELECT (bareme_avant * 1000)::bigint, (bareme_apres * 1000)::bigint, motif FROM eva_evaluation_historique
          WHERE evaluation_id = $1 ORDER BY version DESC LIMIT 1",
    )
    .bind(ev)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((avant, apres), (10_000, 20_000));
    assert_eq!(motif.as_deref(), Some("Barème saisi sur 10 au lieu de 20"));
    // Le même aperçu ne se rejoue pas (version de l'évaluation avancée).
    let r = ent
        .eva_v3f_appliquer_conversion_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            ev,
            &appliquer(ap.empreinte, "Barème saisi sur 10 au lieu de 20"),
        )
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    // Un autre professeur ne convertit pas.
    let r = ent
        .eva_v3f_apercu_conversion_for_actor(
            w.etab,
            autre_prof(&cast),
            PROF,
            jour(),
            ev,
            &ConversionBody {
                version_attendue: ok.version,
                nouveau_bareme_m: Some(10_000),
                annuler_exercice_id: None,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
}

#[tokio::test]
async fn v3f_ce83_un_exercice_annule_reduit_le_bareme_et_ramene_les_notes() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b, c) = (w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]);
    let ev = creer(&ent, &w, "DS énoncé faux", d(2031, 10, 10), 20_000).await;
    let b0 = definir(
        &ent,
        &w,
        ev,
        &[("Ex. 1", 10_000), ("Ex. 2 (faux)", 4_000), ("Ex. 3", 6_000)],
    )
    .await
    .unwrap();
    let ex = b0.exercices.clone();
    let r = saisir(
        &ent,
        &w,
        ev,
        vec![
            detail(a, &ex, &[8_000, 3_000, 5_000], None),
            detail(b, &ex, &[5_000, 0, 2_000], None),
        ],
    )
    .await;
    assert!(r.eleves.iter().all(|e| e.issue == "ecrit"), "{r:?}");
    // L'élève C est noté directement dans la grille (sans détail) : 12 sur 20.
    assert_eq!(case_grille(&ent, &w, ev, c, "12", None).await, "ecrit");
    let v = bareme(&ent, &w, ev).await.version;
    let demande = ConversionBody {
        version_attendue: v,
        nouveau_bareme_m: None,
        annuler_exercice_id: Some(ex[1].id),
    };
    let ap = ent
        .eva_v3f_apercu_conversion_for_actor(w.etab, w.prof, PROF, jour(), ev, &demande)
        .await
        .unwrap();
    assert_eq!(ap.bareme_apres_m, 16_000);
    let ligne = |e: Uuid| ap.lignes.iter().find(|l| l.eleve_id == e).unwrap().clone();
    assert_eq!(
        (ligne(a).apres_m, ligne(a).source.as_str()),
        (13_000, "detail")
    );
    assert_eq!(
        (ligne(b).apres_m, ligne(b).source.as_str()),
        (7_000, "detail")
    );
    // 12 × 16 / 20 = 9,6 : au pas de saisie par défaut (0,01).
    assert_eq!(
        (ligne(c).apres_m, ligne(c).source.as_str()),
        (9_600, "proportion")
    );
    let ok = ent
        .eva_v3f_appliquer_conversion_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            ev,
            &AppliquerConversionBody {
                version_attendue: v,
                nouveau_bareme_m: None,
                annuler_exercice_id: Some(ex[1].id),
                empreinte: ap.empreinte,
                motif: "Énoncé de l'exercice 2 erroné".into(),
            },
        )
        .await
        .unwrap();
    // B avait 0 à l'exercice annulé : sa note ne change pas.
    assert_eq!((ok.bareme_m, ok.nb_notes_converties), (16_000, 2));
    assert_eq!(note(&ent, &w, ev, a).await.0, Some(13_000));
    assert_eq!(note(&ent, &w, ev, b).await.0, Some(7_000));
    assert_eq!(note(&ent, &w, ev, c).await.0, Some(9_600));
    let b1 = bareme(&ent, &w, ev).await;
    assert!(b1.exercices[1].annule);
    assert_eq!(b1.somme_maximums_m, 16_000);
    assert_eq!(
        b1.nb_details_a_jour, 2,
        "les détails suivent la note convertie"
    );
    assert_eq!(
        b1.reussite.len(),
        2,
        "l'exercice annulé n'a plus de réussite"
    );
    // L'exercice annulé le reste, en base.
    assert!(
        sqlx::query("UPDATE eva_bareme_item SET annule = false WHERE id = $1")
            .bind(ex[1].id)
            .execute(ent.pool_verif())
            .await
            .is_err()
    );
    // Saisie suivante : un point par exercice restant.
    let vb = note(&ent, &w, ev, b).await.1;
    let r = saisir(
        &ent,
        &w,
        ev,
        vec![detail(b, &b1.exercices, &[6_000, 2_000], Some(vb))],
    )
    .await;
    assert_eq!(
        (r.eleves[0].issue.as_str(), r.eleves[0].valeur_m),
        ("ecrit", Some(8_000))
    );
    // Le dernier exercice restant ne s'annule pas seul ; un changement de barème direct est refusé (barème détaillé).
    let r = ent
        .eva_v3f_apercu_conversion_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            ev,
            &ConversionBody {
                version_attendue: b1.version,
                nouveau_bareme_m: Some(20_000),
                annuler_exercice_id: None,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
}

// ───────────────────────── Duplication (EVA-125 multiple) ─────────────────────────

#[tokio::test]
async fn v3f_dupliquer_plusieurs_evaluations_sans_resultats_avec_leurs_exercices() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let e1 = creer(&ent, &w, "DS 1", d(2031, 10, 10), 20_000).await;
    let e2 = creer(&ent, &w, "Interro", d(2031, 10, 12), 10_000).await;
    let b1 = definir(&ent, &w, e1, &[("Ex. 1", 15_000), ("Ex. 2", 5_000)])
        .await
        .unwrap();
    saisir(
        &ent,
        &w,
        e1,
        vec![detail(a, &b1.exercices, &[10_000, 5_000], None)],
    )
    .await;
    let corps = DupliquerBody {
        cours_cible_id: w.cours_maths,
        evaluations: vec![
            DupliquerItemDto {
                evaluation_id: e1,
                date_eval: d(2031, 11, 14),
            },
            DupliquerItemDto {
                evaluation_id: e2,
                date_eval: d(2031, 11, 15),
            },
        ],
    };
    let r = ent
        .eva_v3f_dupliquer_for_actor(w.etab, w.prof, PROF, jour(), &corps)
        .await
        .unwrap();
    assert_eq!(r.creees.len(), 2);
    assert_eq!(r.creees[0].nb_exercices, 2);
    let copie = r.creees[0].evaluation_id;
    let (titre, bareme_m, coef_m, n_res): (String, i64, i64, i64) = sqlx::query_as(
        "SELECT titre, (bareme * 1000)::bigint, (coef * 1000)::bigint,
                (SELECT count(*) FROM eva_resultat r WHERE r.evaluation_id = e.id)
           FROM eva_evaluation e WHERE id = $1",
    )
    .bind(copie)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(
        (titre.as_str(), bareme_m, coef_m, n_res),
        ("DS 1", 20_000, 2_000, 0)
    );
    assert!(bareme(&ent, &w, copie).await.redefinissable);
    // Tout ou rien : une date hors de l'année refuse toute l'opération.
    let avant: i64 = sqlx::query_scalar("SELECT count(*) FROM eva_evaluation WHERE cours_id = $1")
        .bind(w.cours_maths)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let r = ent
        .eva_v3f_dupliquer_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &DupliquerBody {
                cours_cible_id: w.cours_maths,
                evaluations: vec![
                    DupliquerItemDto {
                        evaluation_id: e2,
                        date_eval: d(2031, 11, 20),
                    },
                    DupliquerItemDto {
                        evaluation_id: e1,
                        date_eval: d(2035, 1, 1),
                    },
                ],
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    let apres: i64 = sqlx::query_scalar("SELECT count(*) FROM eva_evaluation WHERE cours_id = $1")
        .bind(w.cours_maths)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(avant, apres);
    // Le professeur de français ne duplique pas les évaluations de maths (ni dans son cours, ni dans celui de maths).
    let x = autre_prof(&cast);
    let r = ent
        .eva_v3f_dupliquer_for_actor(
            w.etab,
            x,
            PROF,
            jour(),
            &DupliquerBody {
                cours_cible_id: w.cours_fr,
                evaluations: vec![DupliquerItemDto {
                    evaluation_id: e1,
                    date_eval: d(2031, 11, 14),
                }],
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    let r = ent
        .eva_v3f_dupliquer_for_actor(w.etab, x, PROF, jour(), &corps)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    // Une évaluation annulée ne se duplique pas.
    let v = bareme(&ent, &w, e2).await.version;
    ent.eva_v3_annuler_evaluation_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        maintenant(),
        e2,
        &AnnulerEvaluationV3Body {
            version_attendue: v,
            motif: "Sujet donné à la mauvaise classe".into(),
        },
    )
    .await
    .unwrap();
    let r = ent
        .eva_v3f_dupliquer_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &DupliquerBody {
                cours_cible_id: w.cours_maths,
                evaluations: vec![DupliquerItemDto {
                    evaluation_id: e2,
                    date_eval: d(2031, 11, 20),
                }],
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
}
