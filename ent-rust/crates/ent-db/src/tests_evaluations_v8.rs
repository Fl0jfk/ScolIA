//! Socle 21, lot V8 « fin de période » : tests sur base (feature `labo`). Aucune horloge réelle : jours fixes de l'année
//! fictive du monde V1 (2031-2032 ; T1 du 2031-09-01 au 2031-12-20, arrêt des notes le 2031-12-05), instants fixes.

use super::*;
use crate::outbox::ConfigDistribution;
use crate::tests_evaluations::labo;
use crate::tests_evaluations_v1::{d, monde_v1, DIR, PROF, W};
use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_types::evaluations_v3::*;
use ent_types::evaluations_v8::*;
use uuid::Uuid;

fn jour() -> NaiveDate {
    d(2031, 11, 20)
}

fn a_midi(j: NaiveDate) -> DateTime<Utc> {
    j.and_hms_opt(12, 0, 0).unwrap().and_utc()
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

async fn creer(ent: &PgEnt, w: &W, prof: Uuid, cours: Uuid, titre: &str, date: NaiveDate) -> Uuid {
    ent.eva_v3_creer_evaluation_for_actor(
        w.etab,
        prof,
        PROF,
        jour(),
        a_midi(jour()),
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
            annoncee: false,
        },
    )
    .await
    .unwrap()
    .id
}

async fn saisir(ent: &PgEnt, w: &W, j: NaiveDate, ev: Uuid, eleve: Uuid, texte: &str) {
    let r = ent
        .eva_v3_saisir_lot_for_actor(
            w.etab,
            w.prof,
            PROF,
            j,
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

async fn version_case(ent: &PgEnt, w: &W, ev: Uuid, eleve: Uuid) -> (i32, Option<i64>) {
    sqlx::query_as(
        "SELECT version, (valeur * 1000)::bigint FROM eva_resultat WHERE etablissement_id = $1 AND evaluation_id = $2 AND eleve_id = $3",
    )
    .bind(w.etab)
    .bind(ev)
    .bind(eleve)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap()
}

async fn etat_classe(ent: &PgEnt, w: &W, periode: Uuid) -> String {
    sqlx::query_scalar("SELECT etat FROM eva_periode_etat WHERE etablissement_id = $1 AND periode_id = $2 AND classe_id = $3")
        .bind(w.etab)
        .bind(periode)
        .bind(w.classe_a)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

async fn autre_prof(ent: &PgEnt, w: &W) -> Uuid {
    sqlx::query_scalar("SELECT prof_user_id FROM ent_service_enseignement WHERE etablissement_id = $1 AND cours_id = $2")
        .bind(w.etab)
        .bind(w.cours_fr)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

async fn arreter(ent: &PgEnt, w: &W, j: NaiveDate) -> Vec<GesteClasseDto> {
    ent.eva_v8_arreter_for_actor(w.etab, w.direction, DIR, a_midi(j), w.t1, &ArreterBody { classes: vec![w.classe_a] })
        .await
        .unwrap()
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

async fn notifs(ent: &PgEnt, etab: Uuid, user: Uuid, kind: &str) -> usize {
    ent.mes_notifications(etab, user, 200).await.unwrap().into_iter().filter(|n| n.kind == kind).count()
}

/// Arrêt (aperçu, publication de tout, instantané), correction directe et son effet, versions et écarts, réouverture
/// bornée puis refermée par le passage quotidien.
#[tokio::test]
async fn arret_publication_instantane_correction_et_reouverture() {
    let Some((ent, cast)) = labo().await else { return };
    let w = monde_v1(&ent, &cast).await;
    let [a, b, c] = [w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]];
    let ev = creer(&ent, &w, w.prof, w.cours_maths, "Contrôle 1", d(2031, 10, 10)).await;
    saisir(&ent, &w, jour(), ev, a, "12").await;
    saisir(&ent, &w, jour(), ev, b, "8").await;

    // Aperçu : une évaluation à publier, une case vide ; le professeur (non principal) ne le lit pas.
    let ap = ent.eva_v8_apercu_arret_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a).await.unwrap();
    assert_eq!((ap.etat.as_str(), ap.eleves, ap.a_publier, ap.cases_vides), ("ouverte", 3, 1, 1), "{ap:?}");
    est_interdit(ent.eva_v8_apercu_arret_for_actor(w.etab, w.prof, PROF, jour(), w.t1, w.classe_a).await);
    // Classe d'une autre périodisation ou période inconnue.
    est_introuvable(ent.eva_v8_apercu_arret_for_actor(w.etab, w.direction, DIR, jour(), Uuid::new_v4(), w.classe_a).await);

    // Arrêt : le professeur ne peut pas ; la direction arrête, tout est publié, version 1 de l'instantané.
    est_interdit(
        ent.eva_v8_arreter_for_actor(w.etab, w.prof, PROF, a_midi(jour()), w.t1, &ArreterBody { classes: vec![w.classe_a] })
            .await,
    );
    est_refus(ent.eva_v8_arreter_for_actor(w.etab, w.direction, DIR, a_midi(jour()), w.t1, &ArreterBody { classes: vec![] }).await);
    let g = arreter(&ent, &w, jour()).await;
    assert_eq!((g[0].publiees, g[0].instantane_version), (1, Some(1)), "{g:?}");
    let (publiee, journal): (bool, i64) = sqlx::query_as(
        "SELECT e.publiee_le IS NOT NULL, (SELECT count(*) FROM eva_evaluation_publication_journal j
                                             WHERE j.evaluation_id = e.id AND j.action = 'publiee_auto_arret')
           FROM eva_evaluation e WHERE e.id = $1",
    )
    .bind(ev)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(publiee && journal == 1, "publication à l'arrêt (R-V9)");
    assert_eq!(etat_classe(&ent, &w, w.t1).await, "notes_arretees");
    est_conflit(
        ent.eva_v8_arreter_for_actor(w.etab, w.direction, DIR, a_midi(jour()), w.t1, &ArreterBody { classes: vec![w.classe_a] })
            .await,
    );

    // Instantané : 3 élèves ; celui sans note n'a qu'une ligne de moyenne générale (vide).
    let iv = ent.eva_v8_instantanes_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a).await.unwrap();
    assert_eq!(iv.versions.len(), 1);
    assert!(iv.a_jour, "{iv:?}");
    assert_eq!(iv.versions[0].lignes, 5);
    let v1 = ent.eva_v8_instantane_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a, 1, None).await.unwrap();
    let gen_b = v1.lignes.iter().find(|l| l.eleve_id == b && l.nature == "moyenne_generale").unwrap();
    assert_eq!(gen_b.valeur_m, Some(8_000));
    est_introuvable(ent.eva_v8_instantane_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a, 9, None).await);

    // Correction directe : effet affiché, seuil de 10 franchi, nouvelle version de l'instantané.
    let (vb, _) = version_case(&ent, &w, ev, b).await;
    est_refus(
        ent.eva_v8_corriger_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            &CorrigerBody { evaluation_id: ev, eleve_id: b, version_attendue: Some(vb), statut: "note".into(), valeur_m: None, motif: "erreur de report".into() },
        )
        .await,
    );
    est_interdit(
        ent.eva_v8_corriger_for_actor(
            w.etab,
            w.prof,
            PROF,
            jour(),
            &CorrigerBody { evaluation_id: ev, eleve_id: b, version_attendue: Some(vb), statut: "note".into(), valeur_m: Some(10_000), motif: "erreur de report".into() },
        )
        .await,
    );
    let f = ent
        .eva_v8_corriger_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            &CorrigerBody { evaluation_id: ev, eleve_id: b, version_attendue: Some(vb), statut: "note".into(), valeur_m: Some(10_000), motif: "erreur de report".into() },
        )
        .await
        .unwrap();
    assert_eq!((f.effet.matiere_avant_m, f.effet.matiere_apres_m), (Some(8_000), Some(10_000)), "{f:?}");
    assert_eq!(f.effet.seuils_franchis_m, vec![10_000]);
    assert_eq!(f.instantane_version, Some(2));
    // Version périmée : conflit, rien n'est écrit.
    est_conflit(
        ent.eva_v8_corriger_for_actor(
            w.etab,
            w.direction,
            DIR,
            jour(),
            &CorrigerBody { evaluation_id: ev, eleve_id: b, version_attendue: Some(vb), statut: "note".into(), valeur_m: Some(11_000), motif: "erreur de report".into() },
        )
        .await,
    );
    let v2 = ent.eva_v8_instantane_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a, 2, Some(1)).await.unwrap();
    assert_eq!(v2.ecarts.len(), 2, "matière et générale de l'élève corrigé : {:?}", v2.ecarts);
    assert!(v2.ecarts.iter().all(|e| e.eleve_id == b && e.avant_m == Some(8_000) && e.apres_m == Some(10_000)));
    // Reprise sans changement : aucune nouvelle version.
    let r = ent
        .eva_v8_reprendre_instantane_for_actor(w.etab, w.direction, DIR, w.t1, w.classe_a, &ReprendreInstantaneBody { motif: "vérification".into() })
        .await
        .unwrap();
    assert_eq!((r.version, r.nouvelle), (2, false));
    // Journal des corrections : la correction directe, avec son motif ; réservé à la direction.
    let j = ent.eva_v8_journal_corrections_for_actor(w.etab, w.direction, DIR, w.t1, w.classe_a).await.unwrap();
    assert_eq!(j.len(), 1, "{j:?}");
    assert_eq!((j[0].valeur_avant_m, j[0].valeur_apres_m, j[0].motif.as_deref()), (Some(8_000), Some(10_000), Some("erreur de report")));
    est_interdit(ent.eva_v8_journal_corrections_for_actor(w.etab, w.prof, PROF, w.t1, w.classe_a).await);

    // Réouverture : motif et durée contrôlés ; la classe rouverte se saisit de nouveau.
    let corps = |motif: &str, jusqu_au: NaiveDate| RouvrirBody { classes: vec![w.classe_a], motif: motif.into(), jusqu_au, vers: None };
    est_refus(ent.eva_v8_rouvrir_for_actor(w.etab, w.direction, DIR, jour(), w.t1, &corps("trop bref", jour() + chrono::Duration::days(3))).await);
    est_refus(ent.eva_v8_rouvrir_for_actor(w.etab, w.direction, DIR, jour(), w.t1, &corps("copies retrouvées au CDI", jour() + chrono::Duration::days(31))).await);
    est_interdit(ent.eva_v8_rouvrir_for_actor(w.etab, w.prof, PROF, jour(), w.t1, &corps("copies retrouvées au CDI", jour() + chrono::Duration::days(3))).await);
    let ro = ent
        .eva_v8_rouvrir_for_actor(w.etab, w.direction, DIR, jour(), w.t1, &corps("copies retrouvées au CDI", jour() + chrono::Duration::days(3)))
        .await
        .unwrap();
    assert_eq!((ro[0].etat_avant.as_str(), ro[0].etat_apres.as_str()), ("notes_arretees", "ouverte"));
    assert_eq!(etat_classe(&ent, &w, w.t1).await, "ouverte");
    est_conflit(ent.eva_v8_rouvrir_for_actor(w.etab, w.direction, DIR, jour(), w.t1, &corps("copies retrouvées au CDI", jour() + chrono::Duration::days(3))).await);
    saisir(&ent, &w, jour(), ev, c, "15").await;
    // Le passage avant le terme ne referme rien ; au lendemain du terme, la classe est de nouveau arrêtée et l'instantané
    // reprend une version (avec la note saisie pendant la réouverture).
    let b0 = ent.eva_v8_passage_au(w.etab, Some(jour() + chrono::Duration::days(3)), a_midi(jour())).await.unwrap();
    assert_eq!(b0.reouvertures_refermees, 0, "{b0:?}");
    let b1 = ent.eva_v8_passage_au(w.etab, Some(jour() + chrono::Duration::days(4)), a_midi(jour())).await.unwrap();
    assert_eq!((b1.reouvertures_refermees, b1.instantanes), (1, 1), "{b1:?}");
    assert_eq!(etat_classe(&ent, &w, w.t1).await, "notes_arretees");
    let b2 = ent.eva_v8_passage_au(w.etab, Some(jour() + chrono::Duration::days(4)), a_midi(jour())).await.unwrap();
    assert_eq!(b2.total(), 0, "idempotent : {b2:?}");
    let iv = ent.eva_v8_instantanes_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a).await.unwrap();
    assert_eq!(iv.versions.len(), 3);
    assert_eq!(iv.versions[2].motif.as_deref(), Some("nouvel arrêt des notes"));
    let journal = ent.eva_v8_reouvertures_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a).await.unwrap();
    assert_eq!((journal.len(), journal[0].refermee_origine.as_deref()), (1, Some("auto")));

    // Gardes de la base : instantané et journal en ajout seul.
    assert!(sqlx::query("DELETE FROM eva_reouverture WHERE etablissement_id = $1").bind(w.etab).execute(ent.pool_verif()).await.is_err());
    assert!(sqlx::query("UPDATE eva_instantane_appreciation SET version = 9 WHERE etablissement_id = $1")
        .bind(w.etab)
        .execute(ent.pool_verif())
        .await
        .map(|r| r.rows_affected() == 0)
        .unwrap_or(true));
}

/// Demandes de correction du professeur, effet vu par la direction sans écrire, refus motivé, acceptation, retrait,
/// gardes de la base.
#[tokio::test]
async fn demandes_de_correction_effet_et_decision() {
    let Some((ent, cast)) = labo().await else { return };
    let w = monde_v1(&ent, &cast).await;
    let [a, _, c] = [w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]];
    let ev = creer(&ent, &w, w.prof, w.cours_maths, "Contrôle 1", d(2031, 10, 10)).await;
    saisir(&ent, &w, jour(), ev, a, "12").await;
    let (va, _) = version_case(&ent, &w, ev, a).await;
    let demande = |version_vue: i32, eleve: Uuid| DemanderCorrectionBody {
        evaluation_id: ev,
        eleve_id: eleve,
        version_vue,
        statut: "note".into(),
        valeur_m: Some(14_000),
        motif: "exercice 3 oublié au barème".into(),
    };
    // Période ouverte : la note se corrige dans la grille.
    est_conflit(ent.eva_v8_demander_correction_for_actor(w.etab, w.prof, PROF, jour(), &demande(va, a)).await);
    arreter(&ent, &w, jour()).await;
    est_conflit(ent.eva_v8_demander_correction_for_actor(w.etab, w.prof, PROF, jour(), &demande(va + 1, a)).await);
    est_refus(ent.eva_v8_demander_correction_for_actor(w.etab, w.prof, PROF, jour(), &demande(1, c)).await);
    let autre = autre_prof(&ent, &w).await;
    est_interdit(ent.eva_v8_demander_correction_for_actor(w.etab, autre, PROF, jour(), &demande(va, a)).await);
    est_refus(
        ent.eva_v8_demander_correction_for_actor(w.etab, w.prof, PROF, jour(), &DemanderCorrectionBody { valeur_m: Some(21_000), ..demande(va, a) })
            .await,
    );
    let dm = ent.eva_v8_demander_correction_for_actor(w.etab, w.prof, PROF, jour(), &demande(va, a)).await.unwrap();
    assert_eq!((dm.etat.as_str(), dm.valeur_proposee_m, dm.de_moi), ("en_attente", Some(14_000), true));
    est_conflit(ent.eva_v8_demander_correction_for_actor(w.etab, w.prof, PROF, jour(), &demande(va, a)).await);
    assert_eq!(ent.eva_v8_demandes_for_actor(w.etab, w.prof, PROF, false).await.unwrap().len(), 1);
    assert!(ent.eva_v8_demandes_for_actor(w.etab, autre, PROF, false).await.unwrap().is_empty());
    assert_eq!(ent.eva_v8_demandes_for_actor(w.etab, w.direction, DIR, false).await.unwrap().len(), 1);

    // Effet : la direction voit 12 → 14, rien n'est écrit.
    est_interdit(ent.eva_v8_effet_demande_for_actor(w.etab, w.prof, PROF, jour(), dm.id).await);
    let e = ent.eva_v8_effet_demande_for_actor(w.etab, w.direction, DIR, jour(), dm.id).await.unwrap();
    assert_eq!((e.matiere_avant_m, e.matiere_apres_m, e.seuils_franchis_m.clone()), (Some(12_000), Some(14_000), vec![14_000]), "{e:?}");
    assert_eq!(version_case(&ent, &w, ev, a).await, (va, Some(12_000)), "simulation sans écriture");

    // Décision : un refus est motivé ; l'acceptation écrit, garde l'effet, reversionne l'instantané.
    est_refus(ent.eva_v8_decider_for_actor(w.etab, w.direction, DIR, jour(), dm.id, &DeciderCorrectionBody { accepter: false, motif: None }).await);
    est_interdit(ent.eva_v8_decider_for_actor(w.etab, w.prof, PROF, jour(), dm.id, &DeciderCorrectionBody { accepter: true, motif: None }).await);
    let ok = ent.eva_v8_decider_for_actor(w.etab, w.direction, DIR, jour(), dm.id, &DeciderCorrectionBody { accepter: true, motif: None }).await.unwrap();
    assert_eq!((ok.etat.as_str(), ok.generale_apres_m), ("acceptee", Some(14_000)), "{ok:?}");
    assert_eq!(version_case(&ent, &w, ev, a).await, (va + 1, Some(14_000)));
    est_conflit(ent.eva_v8_decider_for_actor(w.etab, w.direction, DIR, jour(), dm.id, &DeciderCorrectionBody { accepter: true, motif: None }).await);
    let iv = ent.eva_v8_instantanes_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a).await.unwrap();
    assert_eq!((iv.versions.len(), iv.a_jour), (2, true));
    assert_eq!(iv.versions[1].motif.as_deref(), Some("correction acceptée"));
    let j = ent.eva_v8_journal_corrections_for_actor(w.etab, w.direction, DIR, w.t1, w.classe_a).await.unwrap();
    assert!(j[0].motif.as_deref().unwrap_or("").contains("acceptée"), "{j:?}");

    // Une seconde demande, refusée ; une troisième, retirée par son auteur seulement.
    let d2 = ent.eva_v8_demander_correction_for_actor(w.etab, w.prof, PROF, jour(), &DemanderCorrectionBody { valeur_m: Some(15_000), ..demande(va + 1, a) }).await.unwrap();
    let refusee = ent
        .eva_v8_decider_for_actor(w.etab, w.direction, DIR, jour(), d2.id, &DeciderCorrectionBody { accepter: false, motif: Some("barème déjà appliqué".into()) })
        .await
        .unwrap();
    assert_eq!((refusee.etat.as_str(), refusee.decision_motif.as_deref()), ("refusee", Some("barème déjà appliqué")));
    let d3 = ent.eva_v8_demander_correction_for_actor(w.etab, w.prof, PROF, jour(), &demande(va + 1, a)).await.unwrap();
    est_introuvable(ent.eva_v8_retirer_demande_for_actor(w.etab, w.direction, DIR, d3.id).await);
    ent.eva_v8_retirer_demande_for_actor(w.etab, w.prof, PROF, d3.id).await.unwrap();
    est_conflit(ent.eva_v8_retirer_demande_for_actor(w.etab, w.prof, PROF, d3.id).await);
    assert_eq!(ent.eva_v8_demandes_for_actor(w.etab, w.prof, PROF, true).await.unwrap().len(), 3);
    assert!(ent.eva_v8_demandes_for_actor(w.etab, w.prof, PROF, false).await.unwrap().is_empty());

    // L'avis de décision va au demandeur.
    distribuer(&ent, w.etab).await;
    assert_eq!(notifs(&ent, w.etab, w.prof, "evaluations_correction_decidee").await, 2);

    // Gardes de la base : la demande ne se réécrit pas, une décision ne se reprend pas.
    assert!(sqlx::query("UPDATE eva_correction_demande SET motif = 'autre motif' WHERE id = $1").bind(dm.id).execute(ent.pool_verif()).await.is_err());
    assert!(sqlx::query("UPDATE eva_correction_demande SET etat = 'refusee', decision_motif = 'non' WHERE id = $1").bind(dm.id).execute(ent.pool_verif()).await.is_err());
    assert!(sqlx::query("DELETE FROM eva_correction_demande WHERE id = $1").bind(dm.id).execute(ent.pool_verif()).await.is_err());
}

/// État de saisie, relances, rappels J-7 / J-2 (avis aux professeurs), arrêt automatique finalisé, rattrapage borné à la
/// veille du conseil, clôture automatique des appréciations.
#[tokio::test]
async fn pilotage_relances_rappels_rattrapage_et_cloture_auto() {
    let Some((ent, cast)) = labo().await else { return };
    let w = monde_v1(&ent, &cast).await;
    let [a, b, c] = [w.eleves_a[0], w.eleves_a[1], w.eleves_a[2]];
    let autre = autre_prof(&ent, &w).await;
    // Le professeur de maths est professeur principal de la classe A ; conseil le 2031-12-10.
    sqlx::query("UPDATE ent_service_enseignement SET professeur_principal = true WHERE etablissement_id = $1 AND cours_id = $2")
        .bind(w.etab)
        .bind(w.cours_maths)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_periode SET date_conseil = '2031-12-10' WHERE id = $1").bind(w.t1).execute(ent.pool_verif()).await.unwrap();
    let ev = creer(&ent, &w, w.prof, w.cours_maths, "Contrôle 1", d(2031, 10, 10)).await;
    saisir(&ent, &w, jour(), ev, a, "12").await;
    saisir(&ent, &w, jour(), ev, b, "9").await;

    // État de saisie : le PP le lit, l'autre professeur non.
    est_interdit(ent.eva_v8_etat_saisie_for_actor(w.etab, autre, PROF, jour(), w.t1, w.classe_a).await);
    let es = ent.eva_v8_etat_saisie_for_actor(w.etab, w.prof, PROF, jour(), w.t1, w.classe_a).await.unwrap();
    assert_eq!((es.eleves, es.cours.len(), es.cours_sans_evaluation, es.jours_avant_arret), (3, 2, 1, Some(15)), "{es:?}");
    let pm = es.professeurs.iter().find(|p| p.prof_user_id == w.prof).unwrap();
    assert_eq!((pm.non_publiees, pm.cases_vides, pm.cours_sans_evaluation), (1, 1, 0), "{pm:?}");
    let pf = es.professeurs.iter().find(|p| p.prof_user_id == autre).unwrap();
    assert_eq!(pf.cours_sans_evaluation, 1);
    assert!(pf.message.contains("1 cours sans évaluation"), "{}", pf.message);

    // Relances : une par professeur et par jour ; un professeur hors de la classe est refusé.
    est_refus(
        ent.eva_v8_relancer_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a, &RelancerBody { professeurs: vec![Uuid::new_v4()], message: None })
            .await,
    );
    let r = ent.eva_v8_relancer_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a, &relance_a_tous()).await.unwrap();
    assert_eq!(r.len(), 2, "{r:?}");
    assert!(ent.eva_v8_relancer_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a, &relance_a_tous()).await.unwrap().is_empty());
    let es = ent.eva_v8_etat_saisie_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a).await.unwrap();
    assert!(es.professeurs.iter().all(|p| p.relance_aujourdhui));

    // Rappels J-7 puis J-2, une fois par échéance.
    let j7 = ent.eva_v8_passage_au(w.etab, Some(d(2031, 11, 28)), a_midi(d(2031, 11, 28))).await.unwrap();
    assert_eq!(j7.rappels, 2, "{j7:?}");
    assert_eq!(ent.eva_v8_passage_au(w.etab, Some(d(2031, 11, 29)), a_midi(d(2031, 11, 29))).await.unwrap().rappels, 0);
    assert_eq!(ent.eva_v8_passage_au(w.etab, Some(d(2031, 12, 3)), a_midi(d(2031, 12, 3))).await.unwrap().rappels, 2);
    let journal = ent.eva_v8_relances_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a).await.unwrap();
    assert_eq!(journal.iter().map(|r| r.nature.as_str()).collect::<Vec<_>>(), vec!["manuelle", "manuelle", "j7", "j7", "j2", "j2"]);
    distribuer(&ent, w.etab).await;
    assert_eq!(notifs(&ent, w.etab, w.prof, "evaluations_relance").await, 1);
    assert_eq!(notifs(&ent, w.etab, autre, "evaluations_rappel_saisie").await, 2);

    // Arrêt automatique (V1) puis finalisation par le passage V8 : publication et instantané.
    assert_eq!(ent.eva_v1_arret_auto_au(w.etab, Some(d(2031, 12, 6))).await.unwrap(), 2, "classes A et D");
    let fin = ent.eva_v8_passage_au(w.etab, Some(d(2031, 12, 6)), a_midi(d(2031, 12, 6))).await.unwrap();
    assert_eq!((fin.publiees, fin.instantanes), (1, 2), "{fin:?}");

    // Rattrapage : au plus la veille du conseil, élève visible, droit de correction.
    let rat = |jusqu_au: NaiveDate| AutoriserRattrapageBody { evaluation_id: ev, eleve_id: c, jusqu_au, motif: "absent justifié".into() };
    est_refus(ent.eva_v8_autoriser_rattrapage_for_actor(w.etab, w.direction, DIR, d(2031, 12, 6), &rat(d(2031, 12, 10))).await);
    est_refus(ent.eva_v8_autoriser_rattrapage_for_actor(w.etab, w.direction, DIR, d(2031, 12, 6), &rat(d(2031, 12, 5))).await);
    est_interdit(ent.eva_v8_autoriser_rattrapage_for_actor(w.etab, w.prof, PROF, d(2031, 12, 6), &rat(d(2031, 12, 9))).await);
    ent.eva_v8_autoriser_rattrapage_for_actor(w.etab, w.direction, DIR, d(2031, 12, 6), &rat(d(2031, 12, 9))).await.unwrap();

    // Clôture automatique des appréciations la veille du conseil.
    assert_eq!(ent.eva_v8_passage_au(w.etab, Some(d(2031, 12, 8)), a_midi(d(2031, 12, 8))).await.unwrap().appreciations_closes, 0);
    let veille = ent.eva_v8_passage_au(w.etab, Some(d(2031, 12, 9)), a_midi(d(2031, 12, 9))).await.unwrap();
    assert_eq!(veille.appreciations_closes, 2, "{veille:?}");
    assert_eq!(etat_classe(&ent, &w, w.t1).await, "appreciations_closes");
    // Après le conseil, plus de rattrapage.
    ent.eva_v1_changer_etat_periode_for_actor(w.etab, w.direction, DIR, w.t1, w.classe_a, "conseil_tenu", None, None).await.unwrap();
    est_conflit(ent.eva_v8_autoriser_rattrapage_for_actor(w.etab, w.direction, DIR, d(2031, 12, 9), &rat(d(2031, 12, 9))).await);
}

/// Contrôle de 3ᵉ : la dernière période d'une classe de 3ᵉ ne se clôt pas tant qu'un enseignement obligatoire n'a pas
/// de moyenne annuelle ; une dispense l'excuse ; hors 3ᵉ, rien ne bloque.
#[tokio::test]
async fn controle_3e_bloque_la_cloture_de_la_derniere_periode() {
    let Some((ent, cast)) = labo().await else { return };
    let w = monde_v1(&ent, &cast).await;
    let niveau = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_niveau (id, etablissement_id, code, libelle) VALUES ($1, $2, $3, '3e')")
        .bind(niveau)
        .bind(w.etab)
        .bind("3E")
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_classe SET niveau_id = $2 WHERE id = $1").bind(w.classe_a).bind(niveau).execute(ent.pool_verif()).await.unwrap();
    let ev = creer(&ent, &w, w.prof, w.cours_maths, "Contrôle 1", d(2031, 10, 10)).await;
    for e in &w.eleves_a {
        saisir(&ent, &w, jour(), ev, *e, "12").await;
    }
    // T1 n'est pas la dernière période : contrôle non applicable.
    let t1 = ent.eva_v8_controle_3e_for_actor(w.etab, w.direction, DIR, jour(), w.t1, w.classe_a).await.unwrap();
    assert!(!t1.applicable && t1.cloture_permise);
    // T2 : les maths ont une moyenne annuelle (T1), pas le français.
    let t2 = ent.eva_v8_controle_3e_for_actor(w.etab, w.direction, DIR, jour(), w.t2, w.classe_a).await.unwrap();
    assert!(t2.applicable && !t2.cloture_permise, "{t2:?}");
    assert_eq!(t2.manques.len(), 3, "{t2:?}");
    assert!(t2.manques.iter().all(|m| m.matiere_id == w.francais));
    for cible in ["notes_arretees", "appreciations_closes", "conseil_tenu"] {
        ent.eva_v1_changer_etat_periode_for_actor(w.etab, w.direction, DIR, w.t2, w.classe_a, cible, None, None).await.unwrap();
    }
    est_interdit(ent.eva_v8_clore_periode_for_actor(w.etab, w.prof, PROF, w.t2, w.classe_a).await);
    est_conflit(ent.eva_v8_clore_periode_for_actor(w.etab, w.direction, DIR, w.t2, w.classe_a).await);
    // Une dispense de français pour chacun l'excuse (R-03 : dispense réglementaire, aucune moyenne attendue).
    for e in &w.eleves_a {
        sqlx::query(
            "INSERT INTO eva_dispense_port (id, etablissement_id, eleve_id, matiere_id, nature, du, saisi_par)
             VALUES ($1, $2, $3, $4, 'dispense', '2031-09-01', $5)",
        )
        .bind(Uuid::new_v4())
        .bind(w.etab)
        .bind(e)
        .bind(w.francais)
        .bind(w.direction)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    }
    let g = ent.eva_v8_clore_periode_for_actor(w.etab, w.direction, DIR, w.t2, w.classe_a).await.unwrap();
    assert_eq!(g.etat, "close");
    est_conflit(ent.eva_v8_clore_periode_for_actor(w.etab, w.direction, DIR, w.t2, w.classe_a).await);
    // La classe D (sans niveau) n'est pas concernée.
    let dd = ent.eva_v8_controle_3e_for_actor(w.etab, w.direction, DIR, jour(), w.t2, w.classe_d).await.unwrap();
    assert!(!dd.applicable);
}

fn relance_a_tous() -> RelancerBody {
    RelancerBody { professeurs: vec![], message: None }
}
