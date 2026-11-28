//! Socle 21, lot V6 « appréciations » : tests sur base (feature `labo`). Aucune horloge réelle : jour fixe 2031-10-20
//! (monde V1 : année 2031-2032, T1 ouverte), dates d'évaluation fixes.

use super::*;
use crate::tests_evaluations::labo;
use crate::tests_evaluations_v1::{arreter, d, monde_v1, W, DIR, PROF};
use chrono::{NaiveDate, TimeZone, Utc};
use domain_eleve::DomainError;
use ent_types::evaluations_v6::*;
use uuid::Uuid;

fn jour() -> NaiveDate {
    d(2031, 10, 20)
}

fn corps(
    w: &W,
    eleve: Uuid,
    type_appreciation: &str,
    cours: Option<Uuid>,
    texte: &str,
    valider: bool,
    version: Option<i32>,
) -> SaisirAppreciationBody {
    SaisirAppreciationBody {
        periode_id: w.t1,
        eleve_id: eleve,
        type_appreciation: type_appreciation.into(),
        cours_id: cours,
        texte: texte.into(),
        valider,
        version_attendue: version,
        motif_correction: None,
    }
}

fn autre_prof(cast: &LaboCast, w: &W) -> Uuid {
    cast.profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)
        .expect("second professeur")
}

async fn etat(ent: &PgEnt, w: &W, cible: &str) {
    ent.eva_v1_changer_etat_periode_for_actor(
        w.etab,
        w.direction,
        DIR,
        w.t1,
        w.classe_a,
        cible,
        None,
        None,
    )
    .await
    .unwrap_or_else(|e| panic!("{cible} : {e:?}"));
}

#[track_caller]
fn est_interdit<T: std::fmt::Debug>(r: Result<T, EntDbError>) {
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
}

#[track_caller]
fn est_conflit<T: std::fmt::Debug>(r: Result<T, EntDbError>) {
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
}

#[track_caller]
fn est_refus<T: std::fmt::Debug>(r: Result<T, EntDbError>) {
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
}

/// EVA-194, §12.2 : versions, brouillon privé, validation, conflits ; fenêtre de chaque type selon l'état de la période
/// de la classe ; correction motivée de la direction ; générale du PP ; vie scolaire de la CPE ; vue famille.
#[tokio::test]
async fn saisie_versions_brouillons_fenetres_et_corrections() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let w = monde_v1(&ent, &cast).await;
    let e = w.eleves_a[0];
    let maths = Some(w.cours_maths);
    let autre = autre_prof(&cast, &w);

    // Brouillon : version 1, lisible de son seul auteur.
    let b = ent
        .eva_v6_saisir_for_actor(w.etab, w.prof, PROF, jour(), &corps(&w, e, "cours", maths, "Bon début.", false, None))
        .await
        .unwrap()
        .appreciation;
    assert_eq!((b.etat.as_str(), b.version, b.de_moi, b.limite), ("brouillon", 1, true, 400));
    let rel = ent
        .eva_v6_appreciations_eleve_for_actor(w.etab, w.direction, DIR, e, w.t1, jour())
        .await
        .unwrap();
    assert!(rel.appreciations.is_empty(), "le brouillon d'un autre n'est jamais montré");
    assert_eq!(rel.etat_periode.as_deref(), Some("ouverte"));
    // Pour la direction (`notes.corriger`), le brouillon d'un autre est « introuvable » : il n'existe pas pour elle.
    assert!(matches!(
        ent.eva_v6_historique_for_actor(w.etab, w.direction, DIR, b.id, jour()).await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));

    // Un professeur qui n'enseigne pas le cours, un élève hors du cours, la famille : refusés.
    est_interdit(
        ent.eva_v6_saisir_for_actor(w.etab, autre, PROF, jour(), &corps(&w, e, "cours", maths, "Texte.", true, None))
            .await,
    );
    est_interdit(
        ent.eva_v6_saisir_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &corps(&w, w.eleves_d[0], "cours", maths, "Texte.", true, None),
        )
        .await,
    );
    est_interdit(
        ent.eva_v6_saisir_for_actor(
            w.etab,
            cast.parent_lea.user_id,
            "parent",
            jour(),
            &corps(&w, e, "cours", maths, "Texte.", true, None),
        )
        .await,
    );
    // Un cours inconnu : aucun oracle pour un professeur.
    est_interdit(
        ent.eva_v6_saisir_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &corps(&w, e, "cours", Some(Uuid::new_v4()), "Texte.", true, None),
        )
        .await,
    );
    // Cours absent pour une appréciation de cours, type inconnu, texte trop long : refusés, jamais tronqués.
    est_refus(
        ent.eva_v6_saisir_for_actor(w.etab, w.prof, PROF, jour(), &corps(&w, e, "cours", None, "Texte.", true, None))
            .await,
    );
    est_refus(
        ent.eva_v6_saisir_for_actor(w.etab, w.prof, PROF, jour(), &corps(&w, e, "conseil", maths, "Texte.", true, None))
            .await,
    );
    est_refus(
        ent.eva_v6_saisir_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &corps(&w, e, "cours", maths, &"a".repeat(401), true, None),
        )
        .await,
    );

    // Conflits de version, puis validation : version 2.
    est_conflit(
        ent.eva_v6_saisir_for_actor(w.etab, w.prof, PROF, jour(), &corps(&w, e, "cours", maths, "Bien.", true, None))
            .await,
    );
    est_conflit(
        ent.eva_v6_saisir_for_actor(w.etab, w.prof, PROF, jour(), &corps(&w, e, "cours", maths, "Bien.", true, Some(7)))
            .await,
    );
    let v = ent
        .eva_v6_saisir_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &corps(&w, e, "cours", maths, "Bon trimestre, travail régulier.", true, Some(1)),
        )
        .await
        .unwrap()
        .appreciation;
    assert_eq!((v.id, v.etat.as_str(), v.version), (b.id, "validee", 2));
    // Réécrire le même texte : aucune nouvelle version.
    let same = ent
        .eva_v6_saisir_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &corps(&w, e, "cours", maths, "Bon trimestre, travail régulier.", true, Some(2)),
        )
        .await
        .unwrap()
        .appreciation;
    assert_eq!(same.version, 2);
    est_refus(
        ent.eva_v6_saisir_for_actor(w.etab, w.prof, PROF, jour(), &corps(&w, e, "cours", maths, "Autre.", false, Some(2)))
            .await,
    );
    let h = ent
        .eva_v6_historique_for_actor(w.etab, w.prof, PROF, v.id, jour())
        .await
        .unwrap();
    assert_eq!(h.iter().map(|x| x.version).collect::<Vec<_>>(), [1, 2], "l'auteur voit ses brouillons");
    let h = ent
        .eva_v6_historique_for_actor(w.etab, w.direction, DIR, v.id, jour())
        .await
        .unwrap();
    assert_eq!(h.iter().map(|x| x.version).collect::<Vec<_>>(), [2], "la direction ne lit que les versions validées");
    assert!(!h[0].de_moi);
    est_interdit(ent.eva_v6_historique_for_actor(w.etab, autre, PROF, v.id, jour()).await);

    // Intermédiaire : période ouverte seulement.
    let i = ent
        .eva_v6_saisir_for_actor(w.etab, w.prof, PROF, jour(), &corps(&w, e, "intermediaire", maths, "Point d'étape.", true, None))
        .await
        .unwrap()
        .appreciation;
    assert_eq!(i.type_appreciation, "intermediaire");

    // Notes arrêtées : le cours reste ouvert aux appréciations, l'intermédiaire non.
    arreter(&ent, &w, w.t1, w.classe_a).await;
    let v3 = ent
        .eva_v6_saisir_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &corps(&w, e, "cours", maths, "Bon trimestre, travail sérieux.", true, Some(2)),
        )
        .await
        .unwrap()
        .appreciation;
    assert_eq!(v3.version, 3);
    est_conflit(
        ent.eva_v6_saisir_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &corps(&w, e, "intermediaire", maths, "Autre point.", true, Some(i.version)),
        )
        .await,
    );

    // Appréciations closes : l'auteur ne peut plus ; la direction corrige avec un motif journalisé.
    etat(&ent, &w, "appreciations_closes").await;
    est_conflit(
        ent.eva_v6_saisir_for_actor(w.etab, w.prof, PROF, jour(), &corps(&w, e, "cours", maths, "Tardif.", true, Some(3)))
            .await,
    );
    est_refus(
        ent.eva_v6_saisir_for_actor(w.etab, w.direction, DIR, jour(), &corps(&w, e, "cours", maths, "Corrigé.", true, Some(3)))
            .await,
    );
    let mut c = corps(&w, e, "cours", maths, "Bon trimestre, travail sérieux et régulier.", true, Some(3));
    c.motif_correction = Some("Faute de frappe signalée par la famille".into());
    let v4 = ent
        .eva_v6_saisir_for_actor(w.etab, w.direction, DIR, jour(), &c)
        .await
        .unwrap()
        .appreciation;
    assert_eq!(v4.version, 4);
    let h = ent
        .eva_v6_historique_for_actor(w.etab, w.prof, PROF, v.id, jour())
        .await
        .unwrap();
    let derniere = h.last().unwrap();
    assert_eq!(derniere.version, 4);
    assert_eq!(derniere.motif_correction.as_deref(), Some("Faute de frappe signalée par la famille"));
    assert!(!derniere.de_moi);
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM eva_appreciation_correction WHERE etablissement_id = $1 AND appreciation_id = $2",
    )
    .bind(w.etab)
    .bind(v.id)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 1);
    // La vue famille (468) montre l'appréciation validée une fois les appréciations closes ; l'intermédiaire jamais.
    let vue: Vec<String> = sqlx::query_scalar(
        "SELECT texte FROM ent_note_appreciation_v WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(w.etab)
    .bind(e)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(vue, ["Bon trimestre, travail sérieux et régulier."]);

    // Générale : le professeur non principal est refusé ; le PP de la classe écrit jusqu'au conseil.
    est_interdit(
        ent.eva_v6_saisir_for_actor(w.etab, w.prof, PROF, jour(), &corps(&w, e, "generale", None, "Bilan.", true, None))
            .await,
    );
    sqlx::query(
        "UPDATE ent_service_enseignement SET professeur_principal = true
          WHERE etablissement_id = $1 AND prof_user_id = $2 AND classe_id = $3",
    )
    .bind(w.etab)
    .bind(autre)
    .bind(w.classe_a)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let g = ent
        .eva_v6_saisir_for_actor(w.etab, autre, PROF, jour(), &corps(&w, e, "generale", None, "Bilan satisfaisant.", true, None))
        .await
        .unwrap()
        .appreciation;
    assert_eq!(g.limite, 600);
    // Vie scolaire : la CPE ; un professeur non.
    ent.eva_v6_saisir_for_actor(
        w.etab,
        cast.cpe.user_id,
        "cpe",
        jour(),
        &corps(&w, e, "vie_scolaire", None, "Ponctuel et assidu.", true, None),
    )
    .await
    .unwrap();
    est_interdit(
        ent.eva_v6_saisir_for_actor(w.etab, w.prof, PROF, jour(), &corps(&w, e, "vie_scolaire", None, "Assidu.", true, None))
            .await,
    );
    // Relecture du PP : cours, intermédiaire, vie scolaire, générale (dans cet ordre).
    let rel = ent
        .eva_v6_appreciations_eleve_for_actor(w.etab, autre, PROF, e, w.t1, jour())
        .await
        .unwrap();
    assert_eq!(
        rel.appreciations.iter().map(|a| a.type_appreciation.as_str()).collect::<Vec<_>>(),
        ["cours", "vie_scolaire", "generale", "intermediaire"]
    );
    est_interdit(
        ent.eva_v6_appreciations_eleve_for_actor(w.etab, w.prof, PROF, e, w.t1, jour()).await,
    );

    // Conseil tenu : la générale est fermée au PP.
    etat(&ent, &w, "conseil_tenu").await;
    est_conflit(
        ent.eva_v6_saisir_for_actor(
            w.etab,
            autre,
            PROF,
            jour(),
            &corps(&w, e, "generale", None, "Bilan modifié.", true, Some(g.version)),
        )
        .await,
    );
}

/// EVA-195, EVA-196, EVA-200, EVA-204, S-117 : saisie en série, contrôles, mots interdits, banque, remarques, jeter.
#[tokio::test]
async fn serie_controles_banque_mots_interdits_et_remarques() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let w = monde_v1(&ent, &cast).await;
    let maths = Some(w.cours_maths);
    let autre = autre_prof(&cast, &w);
    let maintenant = Utc.with_ymd_and_hms(2031, 10, 20, 9, 0, 0).unwrap();

    // Mots interdits : la direction seule les règle ; un doublon (casse ignorée) rend le même mot.
    est_interdit(
        ent.eva_v6_ajouter_mot_interdit_for_actor(w.etab, w.prof, PROF, &AjouterMotInterditBody { mot: "paresseux".into() })
            .await,
    );
    let m = ent
        .eva_v6_ajouter_mot_interdit_for_actor(w.etab, w.direction, DIR, &AjouterMotInterditBody { mot: "paresseux".into() })
        .await
        .unwrap();
    let m2 = ent
        .eva_v6_ajouter_mot_interdit_for_actor(w.etab, w.direction, DIR, &AjouterMotInterditBody { mot: " Paresseux ".into() })
        .await
        .unwrap();
    assert_eq!(m.id, m2.id);
    assert_eq!(ent.eva_v6_mots_interdits_for_actor(w.etab, PROF).await.unwrap().len(), 1);

    // Même texte pour les trois élèves (sous le seuil de 5) avec un mot interdit ; puis un texte de santé.
    for (i, e) in w.eleves_a.iter().enumerate() {
        // Le troisième reste un brouillon du professeur.
        let r = ent
            .eva_v6_saisir_for_actor(
                w.etab,
                w.prof,
                PROF,
                jour(),
                &corps(&w, *e, "cours", maths, "Travail sérieux, parfois paresseux.", i < 2, None),
            )
            .await
            .unwrap();
        assert_eq!(r.controles.iter().map(|c| c.code.as_str()).collect::<Vec<_>>(), ["mot_interdit"]);
    }
    let s = ent
        .eva_v6_saisir_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &corps(&w, w.eleves_a[1], "cours", maths, "Élève dyslexique, courageux.", true, Some(1)),
        )
        .await
        .unwrap();
    assert_eq!(s.controles.iter().map(|c| c.code.as_str()).collect::<Vec<_>>(), ["sante"]);

    // Série du professeur : les trois élèves de la classe A, avec leur appréciation et ses contrôles.
    let serie = ent
        .eva_v6_serie_cours_for_actor(w.etab, w.prof, PROF, w.cours_maths, w.t1, jour(), maintenant)
        .await
        .unwrap();
    assert!(serie.peut_saisir);
    assert_eq!(serie.eleves.len(), 3);
    for l in &serie.eleves {
        assert!(l.saisie_ouverte && !l.brouillon_d_un_autre, "{l:?}");
        assert!(l.appreciation.is_some(), "validée, ou brouillon de l'acteur");
        assert_eq!(l.etat_periode.as_deref(), Some("ouverte"));
    }
    // La direction ne voit pas le brouillon du professeur : elle sait seulement qu'il existe.
    let serie_dir = ent
        .eva_v6_serie_cours_for_actor(w.etab, w.direction, DIR, w.cours_maths, w.t1, jour(), maintenant)
        .await
        .unwrap();
    let l2 = serie_dir.eleves.iter().find(|l| l.eleve_id == w.eleves_a[2]).unwrap();
    assert!(l2.appreciation.is_none() && l2.brouillon_d_un_autre && !l2.saisie_ouverte, "{l2:?}");
    est_conflit(
        ent.eva_v6_saisir_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            &corps(&w, w.eleves_a[2], "cours", maths, "Texte de la direction.", true, None),
        )
        .await,
    );
    let l1 = serie.eleves.iter().find(|l| l.eleve_id == w.eleves_a[1]).unwrap();
    assert_eq!(l1.controles.iter().map(|c| c.code.as_str()).collect::<Vec<_>>(), ["sante"]);
    // Un professeur sans le cours ne lit pas la série.
    assert!(ent
        .eva_v6_serie_cours_for_actor(w.etab, autre, PROF, w.cours_maths, w.t1, jour(), maintenant)
        .await
        .is_err());

    // Remarque de la direction à l'auteur sur la version lue ; le destinataire la traite, une fois.
    let app0 = ent
        .eva_v6_appreciations_eleve_for_actor(w.etab, w.direction, DIR, w.eleves_a[0], w.t1, jour())
        .await
        .unwrap()
        .appreciations
        .remove(0);
    est_conflit(
        ent.eva_v6_remarquer_for_actor(
            w.etab,
            w.direction,
            DIR,
            app0.id,
            jour(),
            &RemarqueBody { texte: "Retirez « paresseux ».".into(), version: 9 },
        )
        .await,
    );
    est_interdit(
        ent.eva_v6_remarquer_for_actor(
            w.etab,
            autre,
            PROF,
            app0.id,
            jour(),
            &RemarqueBody { texte: "Retirez « paresseux ».".into(), version: app0.version },
        )
        .await,
    );
    let rq = ent
        .eva_v6_remarquer_for_actor(
            w.etab,
            w.direction,
            DIR,
            app0.id,
            jour(),
            &RemarqueBody { texte: "Retirez « paresseux ».".into(), version: app0.version },
        )
        .await
        .unwrap();
    let recues = ent
        .eva_v6_mes_remarques_for_actor(w.etab, w.prof, PROF)
        .await
        .unwrap();
    assert_eq!(recues.len(), 1);
    assert!(!recues[0].de_moi && !recues[0].traitee);
    assert_eq!(recues[0].eleve_id, w.eleves_a[0]);
    est_interdit(
        ent.eva_v6_traiter_remarque_for_actor(w.etab, w.direction, DIR, rq.id)
            .await,
    );
    ent.eva_v6_traiter_remarque_for_actor(w.etab, w.prof, PROF, rq.id)
        .await
        .unwrap();
    est_conflit(
        ent.eva_v6_traiter_remarque_for_actor(w.etab, w.prof, PROF, rq.id)
            .await,
    );
    assert!(sqlx::query("UPDATE eva_appreciation_remarque SET texte = 'x' WHERE id = $1")
        .bind(rq.id)
        .execute(ent.pool_verif())
        .await
        .is_err(), "une remarque ne se modifie pas");

    // Banque de phrases : personnelle, sans doublon ; la phrase d'un autre ne se retire pas.
    let p = ent
        .eva_v6_ajouter_phrase_for_actor(w.etab, w.prof, PROF, &AjouterPhraseBody { texte: "Peut mieux faire.".into(), etiquettes: Some("relance".into()) })
        .await
        .unwrap();
    let p2 = ent
        .eva_v6_ajouter_phrase_for_actor(w.etab, w.prof, PROF, &AjouterPhraseBody { texte: " Peut mieux faire. ".into(), etiquettes: None })
        .await
        .unwrap();
    assert_eq!(p.id, p2.id);
    assert_eq!(ent.eva_v6_banque_for_actor(w.etab, w.prof, PROF).await.unwrap().len(), 1);
    assert!(ent.eva_v6_banque_for_actor(w.etab, autre, PROF).await.unwrap().is_empty());
    est_interdit(ent.eva_v6_retirer_phrase_for_actor(w.etab, autre, PROF, p.id).await);
    ent.eva_v6_retirer_phrase_for_actor(w.etab, w.prof, PROF, p.id)
        .await
        .unwrap();

    // Dispositif : réservé à la direction ; brouillon de la direction, jeté puis réécrit.
    est_interdit(
        ent.eva_v6_saisir_for_actor(w.etab, w.prof, PROF, jour(), &corps(&w, w.eleves_a[2], "dispositif", None, "ULIS.", true, None))
            .await,
    );
    let br = ent
        .eva_v6_saisir_for_actor(w.etab, w.direction, DIR, jour(), &corps(&w, w.eleves_a[2], "dispositif", None, "Suivi ULIS.", false, None))
        .await
        .unwrap()
        .appreciation;
    // Le professeur ne jette pas le brouillon d'un autre ; l'auteur le jette ; la clé se réécrit ensuite.
    est_interdit(
        ent.eva_v6_jeter_for_actor(w.etab, w.prof, PROF, br.id, &JeterBrouillonBody { version_attendue: br.version })
            .await,
    );
    est_conflit(
        ent.eva_v6_jeter_for_actor(w.etab, w.direction, DIR, br.id, &JeterBrouillonBody { version_attendue: 9 })
            .await,
    );
    ent.eva_v6_jeter_for_actor(w.etab, w.direction, DIR, br.id, &JeterBrouillonBody { version_attendue: br.version })
        .await
        .unwrap();
    let neuf = ent
        .eva_v6_saisir_for_actor(w.etab, w.direction, DIR, jour(), &corps(&w, w.eleves_a[2], "dispositif", None, "Suivi ULIS régulier.", true, None))
        .await
        .unwrap()
        .appreciation;
    assert_ne!(neuf.id, br.id);
    est_interdit(
        ent.eva_v6_historique_for_actor(w.etab, w.prof, PROF, br.id, jour())
            .await,
    );

    // Le professeur valide son brouillon : version 2.
    let v = ent
        .eva_v6_saisir_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &corps(&w, w.eleves_a[2], "cours", maths, "Travail sérieux.", true, Some(1)),
        )
        .await
        .unwrap()
        .appreciation;
    assert_eq!((v.version, v.etat.as_str()), (2, "validee"));

    // Mot interdit retiré : les contrôles ne le citent plus.
    ent.eva_v6_retirer_mot_interdit_for_actor(w.etab, w.direction, DIR, m.id)
        .await
        .unwrap();
    assert!(matches!(
        ent.eva_v6_retirer_mot_interdit_for_actor(w.etab, w.direction, DIR, m.id).await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));
    let serie = ent
        .eva_v6_serie_cours_for_actor(w.etab, w.direction, DIR, w.cours_maths, w.t1, jour(), maintenant)
        .await
        .unwrap();
    let l0 = serie.eleves.iter().find(|l| l.eleve_id == w.eleves_a[0]).unwrap();
    assert!(l0.controles.is_empty(), "{:?}", l0.controles);
}
