//! Socle 21, lot V7 « famille et élève » : tests sur base (feature `labo`). Aucune horloge réelle : jour fixe 2031-10-20
//! (monde V1 : année 2031-2032, T1 ouverte, arrêt des notes le 2031-12-05), dates d'évaluation fixes.

use super::*;
use crate::evaluations_v7::types_v7;
use crate::outbox::ConfigDistribution;
use crate::tests_evaluations::labo;
use crate::tests_evaluations_v1::{affecter, creer_eleve, d, monde_v1, DIR, PROF, W};
use crate::tests_messagerie_m1::nouvel_utilisateur;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use domain_eleve::DomainError;
use ent_types::evaluations_v3::*;
use ent_types::evaluations_v4::PlanifierRattrapageBody;
use ent_types::evaluations_v7::*;
use uuid::Uuid;

fn jour() -> NaiveDate {
    d(2031, 10, 20)
}

fn maintenant() -> DateTime<Utc> {
    jour().and_hms_opt(12, 0, 0).unwrap().and_utc()
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

#[track_caller]
fn est_introuvable<T: std::fmt::Debug>(r: Result<T, EntDbError>) {
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::NotFound))), "{r:?}");
}

async fn creer(ent: &PgEnt, w: &W, cours: Uuid, titre: &str, date: NaiveDate, annoncee: bool) -> Uuid {
    ent.eva_v3_creer_evaluation_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        maintenant(),
        &CreerEvaluationV3Body {
            cours_id: cours,
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
            annoncee,
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

async fn publication(ent: &PgEnt, w: &W, ev: Uuid, action: &str, motif: Option<&str>) {
    ent.eva_v3_publier_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        maintenant(),
        ev,
        &PublierV3Body {
            action: action.into(),
            prevue_le: None,
            prevue_locale: None,
            motif: motif.map(String::from),
        },
    )
    .await
    .unwrap_or_else(|e| panic!("{action} : {e:?}"));
}

async fn lier_parent(ent: &PgEnt, w: &W, eleve: Uuid, voit_notes: bool) -> Uuid {
    let user = nouvel_utilisateur(ent, w.etab, "parent", "Parent").await;
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
    user
}

async fn compte_eleve(ent: &PgEnt, w: &W, eleve: Uuid) -> Uuid {
    let user = nouvel_utilisateur(ent, w.etab, "eleve", "Eleve").await;
    sqlx::query("INSERT INTO ent_eleve_compte (etablissement_id, eleve_id, user_id) VALUES ($1, $2, $3)")
        .bind(w.etab)
        .bind(eleve)
        .bind(user)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    user
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
        (cible == "notes_arretees").then(|| d(2031, 12, 5)),
    )
    .await
    .unwrap_or_else(|e| panic!("{cible} : {e:?}"));
}

/// Cinq élèves en classe A (seuil de confidentialité des statistiques atteint).
async fn classe_de_cinq(ent: &PgEnt, w: &W) -> Vec<Uuid> {
    let mut v = w.eleves_a.clone();
    for nom in ["Diggory", "Finnigan"] {
        let e = creer_eleve(ent, w.etab, nom).await;
        affecter(ent, w.etab, w.annee, e, w.classe_a, d(2031, 9, 1)).await;
        v.push(e);
    }
    v
}

async fn distribuer(ent: &PgEnt, etab: Uuid) {
    ent.distributeur(ConfigDistribution {
        taille_lot: 200,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    })
    .distribuer_tout()
    .await
    .unwrap();
}

async fn avis(ent: &PgEnt, etab: Uuid, user: Uuid, kind: &str) -> Vec<(String, String)> {
    ent.mes_notifications(etab, user, 200)
        .await
        .unwrap()
        .into_iter()
        .filter(|n| n.kind == kind)
        .map(|n| (n.titre, n.corps))
        .collect()
}

/// EVA-159 à EVA-161, EVA-163, EVA-269, §10.1 : le relevé de la famille et de l'élève ne montre que le publié, avec la
/// moyenne provisoire expliquée par le moteur, les statistiques sous le seuil de 5, l'évaluation retirée « pour
/// vérification », les dates de la période ; rien pour le personnel, un parent sans « voir les notes » ou l'autre foyer.
#[tokio::test]
async fn releve_publie_explique_statistiques_retrait_et_droits() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let w = monde_v1(&ent, &cast).await;
    let eleves = classe_de_cinq(&ent, &w).await;
    let a = eleves[0];
    let parent = lier_parent(&ent, &w, a, true).await;
    let parent_sans_notes = lier_parent(&ent, &w, a, false).await;
    let compte = compte_eleve(&ent, &w, a).await;
    let parent_autre = lier_parent(&ent, &w, w.eleves_d[0], true).await;

    // DS 1 publié : cinq notes (8, 10, 12, 14, 16) ; DS 2 non publié ; DS 3 publié puis retiré.
    let ds1 = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 1), false).await;
    for (e, n) in eleves.iter().zip(["8", "10", "12", "14", "16"]) {
        saisir(&ent, &w, ds1, *e, n).await;
    }
    publication(&ent, &w, ds1, "maintenant", None).await;
    let ds2 = creer(&ent, &w, w.cours_maths, "DS 2 non publié", d(2031, 10, 8), false).await;
    saisir(&ent, &w, ds2, a, "3").await;
    let ds3 = creer(&ent, &w, w.cours_maths, "DS 3 retiré", d(2031, 10, 9), false).await;
    saisir(&ent, &w, ds3, a, "19").await;
    publication(&ent, &w, ds3, "maintenant", None).await;
    publication(&ent, &w, ds3, "retirer", Some("erreur de barème")).await;

    let r = ent
        .eva_v7_releve_for_actor(w.etab, parent, "parent", a, w.t1, jour())
        .await
        .unwrap();
    assert_eq!(r.echelle_m, 20_000);
    assert!(r.provisoire, "T1 est ouverte");
    assert_eq!(r.etat_periode.as_deref(), Some("ouverte"));
    assert!(
        r.mention_provisoire.as_deref().is_some_and(|m| m.contains("5 décembre 2031")),
        "{:?}",
        r.mention_provisoire
    );
    assert_eq!(r.calendrier.date_arret, Some(d(2031, 12, 5)));
    let maths: Vec<&MatiereReleveDto> = r.matieres.iter().filter(|m| m.matiere_id == Some(w.maths)).collect();
    assert_eq!(maths.len(), 1, "{:?}", r.matieres);
    let m = maths[0];
    assert_eq!(m.matiere_libelle, "Mathématiques");
    assert_eq!(m.moyenne_m, Some(8_000), "seul DS 1 publié compte");
    assert_eq!(m.resultats.len(), 1, "ni le non publié, ni le retiré : {:?}", m.resultats);
    let res = &m.resultats[0];
    assert_eq!((res.evaluation_id, res.valeur_m, res.bareme_m), (ds1, Some(8_000), 20_000));
    assert!(res.revision_possible);
    let st = res.stats.expect("cinq élèves notés : statistiques montrées");
    assert_eq!((st.effectif, st.moyenne_m, st.min_m, st.max_m, st.mediane_m), (5, 12_000, 8_000, 16_000, None));
    assert!(m.explication.iter().any(|l| l.contains("DS 1")), "{:?}", m.explication);
    assert!(m.explication.iter().all(|l| !l.contains("DS 2")), "le non publié n'est pas expliqué");
    assert_eq!(r.moyenne_generale_m, Some(8_000));
    assert_eq!(r.retirees.len(), 1);
    assert_eq!((r.retirees[0].evaluation_id, r.retirees[0].mention.as_str()), (ds3, MENTION_RETIREE));
    let texte = format!("{r:?}");
    assert!(!texte.contains("erreur de barème") && !texte.contains("19000"), "ni motif ni note retirée : {texte}");

    // L'élève lit le même relevé ; une trace de consultation par lecture (R-V26).
    let r2 = ent
        .eva_v7_releve_for_actor(w.etab, compte, "eleve", a, w.t1, jour())
        .await
        .unwrap();
    assert_eq!(r2.matieres, r.matieres);
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM eva_consultation WHERE etablissement_id = $1 AND eleve_id = $2 AND objet = 'releve'",
    )
    .bind(w.etab)
    .bind(a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(n, 2);

    // Sous le seuil : un groupe de trois n'a pas de statistiques. L'établissement peut aussi les masquer (S-113).
    sqlx::query(
        "INSERT INTO eva_reglage (id, etablissement_id, annee_id, stats_familles) VALUES ($1, $2, $3, false)
         ON CONFLICT (etablissement_id, annee_id) DO UPDATE SET stats_familles = false",
    )
        .bind(Uuid::new_v4())
        .bind(w.etab)
        .bind(w.annee)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let masque = ent
        .eva_v7_releve_for_actor(w.etab, parent, "parent", a, w.t1, jour())
        .await
        .unwrap();
    assert!(
        masque.matieres.iter().flat_map(|m| &m.resultats).all(|x| x.stats.is_none()),
        "statistiques masquées par l'établissement"
    );

    // Droits : personnel, parent sans « voir les notes », parent d'un autre élève, élève d'un autre compte.
    est_interdit(ent.eva_v7_releve_for_actor(w.etab, w.prof, PROF, a, w.t1, jour()).await);
    est_interdit(ent.eva_v7_releve_for_actor(w.etab, w.direction, DIR, a, w.t1, jour()).await);
    est_interdit(ent.eva_v7_releve_for_actor(w.etab, parent_sans_notes, "parent", a, w.t1, jour()).await);
    est_interdit(ent.eva_v7_releve_for_actor(w.etab, parent_autre, "parent", a, w.t1, jour()).await);
    est_interdit(ent.eva_v7_releve_for_actor(w.etab, compte, "eleve", eleves[1], w.t1, jour()).await);
    est_introuvable(ent.eva_v7_releve_for_actor(w.etab, parent, "parent", a, Uuid::new_v4(), jour()).await);

    // À l'arrêt des notes, la moyenne n'est plus provisoire ; au conseil, plus de demande de révision.
    etat(&ent, &w, "notes_arretees").await;
    let arr = ent
        .eva_v7_releve_for_actor(w.etab, parent, "parent", a, w.t1, jour())
        .await
        .unwrap();
    assert!(!arr.provisoire && arr.mention_provisoire.is_none());
    assert_eq!(arr.etat_periode.as_deref(), Some("notes_arretees"));
    etat(&ent, &w, "appreciations_closes").await;
    etat(&ent, &w, "conseil_tenu").await;
    let fin = ent
        .eva_v7_releve_for_actor(w.etab, parent, "parent", a, w.t1, jour())
        .await
        .unwrap();
    assert!(fin.matieres.iter().flat_map(|m| &m.resultats).all(|x| !x.revision_possible));
    est_conflit(
        ent.eva_v7_demander_revision_for_actor(
            w.etab,
            parent,
            "parent",
            &DemanderRevisionBody { eleve_id: a, evaluation_id: ds1, message: "Un exercice oublié ?".into() },
        )
        .await,
    );
}

/// EVA-166, CE-59, R-V46 : demande tracée, une ouverte par résultat et demandeur, jamais sur le non publié ; le parent ne
/// voit que les siennes ; seul un professeur du cours (ou la direction) répond, une fois ; retrait ; avis neutres ;
/// la base garde la demande intacte.
#[tokio::test]
async fn revisions_demande_reponse_retrait_avis_et_garde() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let w = monde_v1(&ent, &cast).await;
    let a = w.eleves_a[0];
    let parent = lier_parent(&ent, &w, a, true).await;
    let parent2 = lier_parent(&ent, &w, a, true).await;
    let compte = compte_eleve(&ent, &w, a).await;
    let autre_prof = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|x| *x != w.prof)
        .expect("second professeur");

    let ds1 = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 1), false).await;
    saisir(&ent, &w, ds1, a, "9").await;
    publication(&ent, &w, ds1, "maintenant", None).await;
    let ds2 = creer(&ent, &w, w.cours_maths, "DS 2", d(2031, 10, 8), false).await;
    saisir(&ent, &w, ds2, a, "4").await;

    let corps = |ev: Uuid, msg: &str| DemanderRevisionBody { eleve_id: a, evaluation_id: ev, message: msg.into() };
    // Message trop court, évaluation non publiée ou inconnue, personnel : refusés.
    est_refus(ent.eva_v7_demander_revision_for_actor(w.etab, parent, "parent", &corps(ds1, "ok")).await);
    est_introuvable(
        ent.eva_v7_demander_revision_for_actor(w.etab, parent, "parent", &corps(ds2, "Pourquoi cette note ?")).await,
    );
    est_introuvable(
        ent.eva_v7_demander_revision_for_actor(w.etab, parent, "parent", &corps(Uuid::new_v4(), "Pourquoi ?"))
            .await,
    );
    est_interdit(ent.eva_v7_demander_revision_for_actor(w.etab, w.prof, PROF, &corps(ds1, "Pourquoi ?")).await);

    let d1 = ent
        .eva_v7_demander_revision_for_actor(
            w.etab,
            parent,
            "parent",
            &corps(ds1, "  L'exercice 3 ne semble pas compté.  "),
        )
        .await
        .unwrap();
    assert_eq!((d1.etat.as_str(), d1.message.as_str()), ("ouverte", "L'exercice 3 ne semble pas compté."));
    est_conflit(ent.eva_v7_demander_revision_for_actor(w.etab, parent, "parent", &corps(ds1, "Encore moi.")).await);
    // L'élève et l'autre parent ont chacun la leur ; personne ne voit celle de l'autre (R-V46).
    let d2 = ent
        .eva_v7_demander_revision_for_actor(w.etab, compte, "eleve", &corps(ds1, "Je pense avoir 11."))
        .await
        .unwrap();
    let d3 = ent
        .eva_v7_demander_revision_for_actor(w.etab, parent2, "parent", &corps(ds1, "Autre parent."))
        .await
        .unwrap();
    let miennes = ent.eva_v7_mes_revisions_for_actor(w.etab, parent, "parent", a).await.unwrap();
    assert_eq!(miennes.iter().map(|x| x.id).collect::<Vec<_>>(), vec![d1.id]);
    let siennes = ent.eva_v7_mes_revisions_for_actor(w.etab, parent2, "parent", a).await.unwrap();
    assert_eq!(siennes.iter().map(|x| x.id).collect::<Vec<_>>(), vec![d3.id]);
    let role: Option<String> = sqlx::query_scalar("SELECT demandeur_role FROM eva_revision_demande WHERE id = $1")
        .bind(d2.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(role.as_deref(), Some("eleve"));

    // Le personnel : le professeur du cours voit les trois ; un autre professeur rien ; la direction tout.
    let p = ent
        .eva_v7_revisions_personnel_for_actor(w.etab, w.prof, PROF, jour(), true)
        .await
        .unwrap();
    assert_eq!(p.len(), 3);
    assert!(p.iter().all(|x| x.evaluation_id == ds1 && x.eleve_id == a));
    assert!(ent
        .eva_v7_revisions_personnel_for_actor(w.etab, autre_prof, PROF, jour(), true)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        ent.eva_v7_revisions_personnel_for_actor(w.etab, w.direction, DIR, jour(), true).await.unwrap().len(),
        3
    );
    est_interdit(ent.eva_v7_revisions_personnel_for_actor(w.etab, parent, "parent", jour(), true).await);

    // Réponse : pas l'autre professeur, pas la famille ; le professeur du cours, une fois.
    let rep = RepondreRevisionBody { reponse: "Vérifié : l'exercice 3 est bien compté.".into() };
    est_interdit(ent.eva_v7_repondre_revision_for_actor(w.etab, autre_prof, PROF, jour(), d1.id, &rep).await);
    est_interdit(ent.eva_v7_repondre_revision_for_actor(w.etab, parent, "parent", jour(), d1.id, &rep).await);
    est_interdit(ent.eva_v7_repondre_revision_for_actor(w.etab, w.prof, PROF, jour(), Uuid::new_v4(), &rep).await);
    est_introuvable(
        ent.eva_v7_repondre_revision_for_actor(w.etab, w.direction, DIR, jour(), Uuid::new_v4(), &rep).await,
    );
    let r = ent
        .eva_v7_repondre_revision_for_actor(w.etab, w.prof, PROF, jour(), d1.id, &rep)
        .await
        .unwrap();
    assert_eq!((r.etat.as_str(), r.reponse.as_deref()), ("repondue", Some(rep.reponse.as_str())));
    est_conflit(ent.eva_v7_repondre_revision_for_actor(w.etab, w.prof, PROF, jour(), d1.id, &rep).await);
    let miennes = ent.eva_v7_mes_revisions_for_actor(w.etab, parent, "parent", a).await.unwrap();
    assert_eq!(miennes[0].reponse.as_deref(), Some(rep.reponse.as_str()));
    // La note n'a pas bougé.
    let valeur: Option<f64> = sqlx::query_scalar(
        "SELECT valeur::float8 FROM eva_resultat WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(w.etab)
    .bind(ds1)
    .bind(a)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(valeur, Some(9.0));

    // Retrait : par son auteur, tant qu'elle est ouverte ; jamais celle d'un autre.
    est_conflit(ent.eva_v7_retirer_revision_for_actor(w.etab, parent, "parent", d1.id).await);
    est_introuvable(ent.eva_v7_retirer_revision_for_actor(w.etab, parent, "parent", d2.id).await);
    est_interdit(ent.eva_v7_retirer_revision_for_actor(w.etab, w.prof, PROF, d2.id).await);
    ent.eva_v7_retirer_revision_for_actor(w.etab, compte, "eleve", d2.id).await.unwrap();
    est_conflit(ent.eva_v7_retirer_revision_for_actor(w.etab, compte, "eleve", d2.id).await);
    // Retirée : une nouvelle demande est de nouveau possible.
    ent.eva_v7_demander_revision_for_actor(w.etab, compte, "eleve", &corps(ds1, "Je reformule ma demande."))
        .await
        .unwrap();
    assert_eq!(
        ent.eva_v7_revisions_personnel_for_actor(w.etab, w.prof, PROF, jour(), true).await.unwrap().len(),
        2,
        "ouvertes : celle de l'autre parent et la nouvelle de l'élève"
    );
    assert_eq!(
        ent.eva_v7_revisions_personnel_for_actor(w.etab, w.prof, PROF, jour(), false).await.unwrap().len(),
        4
    );

    // Événements neutres, puis avis : le professeur du cours pour chaque demande, le seul parent auteur pour la réponse.
    let charges: Vec<(String, Option<Uuid>, String)> = sqlx::query_as(
        "SELECT type, eleve_id, payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type = ANY($2)",
    )
    .bind(w.etab)
    .bind(vec![types_v7::REVISION_DEMANDEE, types_v7::REVISION_REPONDUE])
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(charges.iter().filter(|c| c.0 == types_v7::REVISION_DEMANDEE).count(), 4);
    assert_eq!(charges.iter().filter(|c| c.0 == types_v7::REVISION_REPONDUE).count(), 1);
    for (_, eleve, payload) in &charges {
        assert!(eleve.is_none(), "charge neutre");
        assert!(!payload.contains("exercice") && !payload.contains("valeur") && !payload.contains("eleve"), "ni message, ni note, ni élève : {payload}");
    }
    distribuer(&ent, w.etab).await;
    let pour_prof = avis(&ent, w.etab, w.prof, "revision_demandee").await;
    // L'avis relit la demande au traitement : une demande déjà répondue ou retirée n'appelle plus le professeur (d1, d2).
    assert_eq!(pour_prof.len(), 2, "{pour_prof:?}");
    assert!(pour_prof.iter().all(|(t, c)| !c.contains("exercice") && t.contains("Mathématiques")));
    assert!(avis(&ent, w.etab, autre_prof, "revision_demandee").await.is_empty());
    assert_eq!(avis(&ent, w.etab, parent, "revision_repondue").await.len(), 1);
    assert!(avis(&ent, w.etab, parent2, "revision_repondue").await.is_empty(), "R-V46");
    assert!(avis(&ent, w.etab, compte, "revision_repondue").await.is_empty());

    // La base garde la demande : ni suppression, ni réécriture, ni retour à « ouverte ».
    for sql in [
        "DELETE FROM eva_revision_demande WHERE id = $1",
        "UPDATE eva_revision_demande SET message = 'réécrit' WHERE id = $1",
        "UPDATE eva_revision_demande SET etat = 'ouverte', reponse = NULL, repondue_par = NULL, repondue_le = NULL WHERE id = $1",
    ] {
        let e = sqlx::query(sql).bind(d1.id).execute(ent.pool_verif()).await;
        assert!(e.is_err(), "{sql}");
    }
    let e = sqlx::query("UPDATE eva_revision_demande SET etat = 'repondue', reponse = 'x' WHERE id = $1")
        .bind(d3.id)
        .execute(ent.pool_verif())
        .await;
    assert!(e.is_err(), "une réponse porte son auteur et sa date");
}

/// EVA-162 à EVA-164, EVA-270 : à venir (annoncées non publiées, rattrapages, dispenses sans motif), projet d'évaluation,
/// simulation de l'élève (aucune écriture), refus pour le parent et pour une matière non suivie.
#[tokio::test]
async fn a_venir_projet_et_simulation() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let w = monde_v1(&ent, &cast).await;
    let (a, b) = (w.eleves_a[0], w.eleves_a[1]);
    let parent = lier_parent(&ent, &w, a, true).await;
    let compte = compte_eleve(&ent, &w, a).await;

    // Annoncée dans deux semaines ; non annoncée ; annoncée au-delà de l'horizon ; publiée (n'est plus « à venir »).
    let annoncee = creer(&ent, &w, w.cours_maths, "Contrôle de géométrie", d(2031, 11, 3), true).await;
    creer(&ent, &w, w.cours_maths, "Surprise", d(2031, 11, 4), false).await;
    creer(&ent, &w, w.cours_maths, "Trop loin", d(2032, 2, 1), true).await;
    let ds1 = creer(&ent, &w, w.cours_maths, "DS 1", d(2031, 10, 1), false).await;
    saisir(&ent, &w, ds1, a, "10").await;
    saisir(&ent, &w, ds1, b, "abs").await;
    publication(&ent, &w, ds1, "maintenant", None).await;
    // Rattrapage de b (absent) ; dispense d'EPS de a (simulée par le port 34, sans motif).
    ent.eva_v4_planifier_rattrapage_for_actor(
        w.etab,
        w.prof,
        PROF,
        jour(),
        &PlanifierRattrapageBody {
            evaluation_id: ds1,
            eleves: vec![b],
            prevu_le: d(2031, 10, 27),
            heure: NaiveTime::from_hms_opt(17, 0, 0),
            lieu: Some("Salle 12".into()),
            modalite: None,
            echeance: None,
            motif: None,
        },
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO eva_dispense_port (id, etablissement_id, eleve_id, matiere_id, nature, du, au, saisi_par)
         VALUES ($1, $2, $3, $4, 'dispense', '2031-09-15', NULL, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(w.etab)
    .bind(a)
    .bind(w.francais)
    .bind(w.direction)
    .execute(ent.pool_verif())
    .await
    .unwrap();

    let v = ent.eva_v7_a_venir_for_actor(w.etab, parent, "parent", a, jour()).await.unwrap();
    assert_eq!(v.evaluations.iter().map(|x| x.evaluation_id).collect::<Vec<_>>(), vec![annoncee]);
    assert_eq!(v.evaluations[0].matiere_libelle, "Mathématiques");
    assert!(v.rattrapages.is_empty(), "le rattrapage est celui de b");
    assert_eq!(v.dispenses.len(), 1);
    assert_eq!((v.dispenses[0].matiere_libelle.as_str(), v.dispenses[0].nature.as_str()), ("Français", "dispense"));
    let vb = ent
        .eva_v7_a_venir_for_actor(w.etab, lier_parent(&ent, &w, b, true).await, "parent", b, jour())
        .await
        .unwrap();
    assert_eq!(vb.rattrapages.len(), 1);
    let ra = &vb.rattrapages[0];
    assert_eq!(
        (ra.evaluation_titre.as_str(), ra.prevu_le, ra.heure.as_deref(), ra.lieu.as_deref()),
        ("DS 1", d(2031, 10, 27), Some("17:00"), Some("Salle 12"))
    );
    est_interdit(ent.eva_v7_a_venir_for_actor(w.etab, w.prof, PROF, a, jour()).await);

    // Projet : les règles en phrases, les matières de la classe (maths, français) avec leur coefficient.
    let p = ent.eva_v7_projet_for_actor(w.etab, compte, "eleve", a, w.t1, jour()).await.unwrap();
    assert_eq!(p.echelle_m, 20_000);
    assert!(p.regles.iter().any(|x| x.contains("jamais zéro")), "{:?}", p.regles);
    let mut matieres: Vec<Uuid> = p.coefficients.iter().map(|c| c.matiere_id).collect();
    matieres.sort();
    let mut attendu = vec![w.maths, w.francais];
    attendu.sort();
    assert_eq!(matieres, attendu);

    // Simulation : 10 en maths ; « si j'ai 14 » → 12 ; rien n'est écrit.
    let avant: i64 = sqlx::query_scalar("SELECT count(*) FROM eva_resultat WHERE etablissement_id = $1")
        .bind(w.etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    let corps = |matiere: Uuid, valeur_m: i64| SimulationBody {
        periode_id: w.t1,
        matiere_id: matiere,
        valeur_m,
        bareme_m: None,
        coef_m: None,
    };
    let s = ent
        .eva_v7_simuler_for_actor(w.etab, compte, "eleve", a, jour(), &corps(w.maths, 14_000))
        .await
        .unwrap();
    assert_eq!((s.matiere_avant_m, s.matiere_apres_m), (Some(10_000), Some(12_000)));
    assert_eq!((s.generale_avant_m, s.generale_apres_m), (Some(10_000), Some(12_000)));
    assert_eq!(s.mention, MENTION_SIMULATION);
    // Une matière suivie mais sans note : la simulation donne sa première moyenne.
    let f = ent
        .eva_v7_simuler_for_actor(w.etab, compte, "eleve", a, jour(), &corps(w.francais, 16_000))
        .await
        .unwrap();
    assert_eq!((f.matiere_avant_m, f.matiere_apres_m), (None, Some(16_000)));
    let apres: i64 = sqlx::query_scalar("SELECT count(*) FROM eva_resultat WHERE etablissement_id = $1")
        .bind(w.etab)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(avant, apres, "la simulation n'écrit rien");
    est_interdit(ent.eva_v7_simuler_for_actor(w.etab, parent, "parent", a, jour(), &corps(w.maths, 14_000)).await);
    est_refus(ent.eva_v7_simuler_for_actor(w.etab, compte, "eleve", a, jour(), &corps(Uuid::new_v4(), 14_000)).await);
    est_refus(ent.eva_v7_simuler_for_actor(w.etab, compte, "eleve", a, jour(), &corps(w.maths, 25_000)).await);
}
