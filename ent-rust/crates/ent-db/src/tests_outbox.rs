//! Socle 2, lot S2-0 : outbox transactionnelle, distribution et premières
//! réactions, contre la base locale de test (feature `labo`).
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use crate::appel_repo::LigneEtat;
use crate::outbox::{
    ConfigDistribution, Consommateur, Distributeur, EvenementDistribue, FutureTraitement, Publier,
};
use crate::reactions::CONSOMMATEUR_NOTIFICATIONS;
use domain_eleve::today_utc;
use ent_auth::Role;
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::PgConnection;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

/// Base migrée et établissement vierge (tests sans jeu de données labo).
async fn base() -> Option<(PgEnt, Uuid)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some((ent, Uuid::new_v4()))
}

/// Réglages de test : distribution limitée à l'établissement du test, reprises
/// immédiates (le temps est simulé en SQL quand un délai est vérifié).
fn config_test(etab: Uuid) -> ConfigDistribution {
    ConfigDistribution {
        taille_lot: 50,
        max_tentatives: 3,
        delai_base: Duration::ZERO,
        delai_max: Duration::ZERO,
        intervalle: Duration::from_millis(20),
        etablissement: Some(etab),
    }
}

fn brouillon(etab: Uuid, type_name: &str, aggregate_id: Uuid, eleve_id: Option<Uuid>) -> MetierEventDraft {
    MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.into(),
        aggregate: AggregateKind::Eleve,
        aggregate_id,
        eleve_id,
        payload: json!({}),
        actor_user_id: None,
    }
}

async fn publier_seul(ent: &PgEnt, d: MetierEventDraft) -> ent_events::MetierEvent {
    let mut tx = ent.pool().begin().await.unwrap();
    let e = tx.publier(d).await.unwrap();
    tx.commit().await.unwrap();
    e
}

async fn compter(ent: &PgEnt, sql: &str, id: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql).bind(id).fetch_one(ent.pool()).await.unwrap();
    n
}

async fn notifications(ent: &PgEnt, etab: Uuid, user: Uuid, kind: &str, eleve: Uuid) -> usize {
    ent.mes_notifications(etab, user, 200)
        .await
        .unwrap()
        .into_iter()
        .filter(|n| n.kind == kind && n.eleve_id == Some(eleve))
        .count()
}

/// Ouvre l'appel Rogue/4B du jour, pointe les états donnés, renvoie l'appel.
async fn appel_4b(ent: &PgEnt, cast: &LaboCast, etats: &[(Uuid, LigneEtat)]) -> crate::appel_repo::Appel {
    let (appel, _) = ent
        .ouvrir_appel(
            cast.etablissement_id,
            cast.creneau_rogue_4b,
            today_utc(),
            cast.prof.user_id,
            Role::Prof.as_str(),
        )
        .await
        .expect("ouvrir");
    for (eleve, etat) in etats {
        ent.marquer_appel_ligne(
            cast.etablissement_id,
            appel.id,
            cast.prof.user_id,
            Role::Prof.as_str(),
            *eleve,
            etat.clone(),
        )
        .await
        .expect("marquer");
    }
    appel
}

fn eleve_nomme<'a>(cast: &'a LaboCast, prenom: &str) -> &'a LaboEleve {
    cast.eleves.iter().find(|e| e.prenom == prenom).expect("élève du cast")
}

/// Consommateur de test : journalise l'ordre, peut échouer sur commande, et
/// vérifie que la transaction est bornée à l'établissement de l'événement.
struct Sonde {
    nom: &'static str,
    types: &'static [&'static str],
    vus: Mutex<Vec<(Uuid, i64)>>,
    appels: AtomicU32,
    echecs_restants: AtomicU32,
    echoue_toujours: AtomicBool,
    hors_etab: AtomicU32,
    rejouable: bool,
}

impl Sonde {
    fn new(nom: &'static str, types: &'static [&'static str]) -> Self {
        Self {
            nom,
            types,
            vus: Mutex::new(Vec::new()),
            appels: AtomicU32::new(0),
            echecs_restants: AtomicU32::new(0),
            echoue_toujours: AtomicBool::new(false),
            hors_etab: AtomicU32::new(0),
            rejouable: true,
        }
    }
}

impl Consommateur for Sonde {
    fn nom(&self) -> &'static str {
        self.nom
    }
    fn types(&self) -> &'static [&'static str] {
        self.types
    }
    fn rejouable(&self) -> bool {
        self.rejouable
    }
    fn traiter<'a>(&'a self, tx: &'a mut PgConnection, evt: &'a EvenementDistribue) -> FutureTraitement<'a> {
        Box::pin(async move {
            let (etab,): (String,) = sqlx::query_as("SELECT current_setting('app.etablissement_id', true)")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            if etab != evt.etablissement_id().to_string() {
                self.hors_etab.fetch_add(1, Ordering::SeqCst);
            }
            if self.echoue_toujours.load(Ordering::SeqCst) {
                return Err("panne simulée".into());
            }
            if self
                .echecs_restants
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                .is_ok()
            {
                return Err("panne passagère".into());
            }
            self.appels.fetch_add(1, Ordering::SeqCst);
            self.vus.lock().unwrap().push((evt.event().aggregate_id, evt.seq_agregat));
            Ok(())
        })
    }
}

// ─── Atomicité ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn transaction_annulee_aucun_evenement() {
    let Some((ent, etab)) = base().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let mut tx = ent.pool().begin().await.unwrap();
    let e = tx.publier(brouillon(etab, "test.annule", Uuid::new_v4(), None)).await.unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_outbox WHERE event_id = $1", e.id).await, 0);
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_metier_event WHERE id = $1", e.id).await, 0);

    // Établissement obligatoire, type non vide.
    let mut tx = ent.pool().begin().await.unwrap();
    assert!(tx.publier(brouillon(Uuid::nil(), "test.x", Uuid::new_v4(), None)).await.is_err());
    assert!(tx.publier(brouillon(etab, "  ", Uuid::new_v4(), None)).await.is_err());
}

/// Panne simulée entre le fait et l'événement : l'écriture dans l'outbox échoue
/// au milieu de la clôture d'appel → rien n'est validé (appel encore ouvert,
/// aucune absence, aucun événement) ; après réparation, la clôture passe.
#[tokio::test]
async fn panne_outbox_annule_la_cloture_d_appel() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let jour = today_utc();
    let appel = appel_4b(&ent, &cast, &[(cast.eleve_lea_id, LigneEtat::Absent)]).await;
    // Le jeu labo peut déjà contenir des absences du jour : on compare avant/après.
    let absences_harry = "SELECT count(*) FROM ent_absence WHERE eleve_id = $1";
    let avant = compter(&ent, absences_harry, cast.eleve_lea_id).await;

    let suffixe = etab.simple().to_string();
    let fonction = format!("ent_test_panne_outbox_{suffixe}");
    sqlx::query(&format!(
        "CREATE OR REPLACE FUNCTION {fonction}() RETURNS trigger LANGUAGE plpgsql AS $$ \
         BEGIN IF NEW.etablissement_id = '{etab}'::uuid AND NEW.type = 'eleve.absence_declared' \
         THEN RAISE EXCEPTION 'panne simulée'; END IF; RETURN NEW; END $$"
    ))
    .execute(ent.pool())
    .await
    .unwrap();
    sqlx::query(&format!(
        "CREATE TRIGGER {fonction} BEFORE INSERT ON ent_outbox FOR EACH ROW EXECUTE FUNCTION {fonction}()"
    ))
    .execute(ent.pool())
    .await
    .unwrap();

    let r = ent
        .clore_appel(etab, appel.id, cast.prof.user_id, Role::Prof.as_str(), None)
        .await;

    sqlx::query(&format!("DROP TRIGGER {fonction} ON ent_outbox"))
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query(&format!("DROP FUNCTION {fonction}()"))
        .execute(ent.pool())
        .await
        .unwrap();

    assert!(r.is_err(), "la clôture doit échouer avec l'outbox");
    assert_eq!(compter(&ent, absences_harry, cast.eleve_lea_id).await, avant, "aucune absence");
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_metier_event WHERE aggregate_id = $1 AND type = 'appel.closed'", appel.id).await,
        0,
        "aucun appel.closed pour un fait annulé"
    );
    let (statut,): (String,) = sqlx::query_as("SELECT statut FROM ent_appel WHERE id = $1")
        .bind(appel.id)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(statut, "ouvert");

    // Réparé : la même clôture aboutit, fait + événements ensemble.
    ent.clore_appel(etab, appel.id, cast.prof.user_id, Role::Prof.as_str(), None)
        .await
        .expect("clore après réparation");
    assert!(ent.eleve_est_absent(etab, cast.eleve_lea_id, jour).await.unwrap());
    assert_eq!(compter(&ent, absences_harry, cast.eleve_lea_id).await, avant + 1);
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_outbox WHERE aggregate_id = $1 AND type = 'appel.closed'", appel.id).await,
        1
    );
}

// ─── Au moins une fois + idempotence ─────────────────────────────────────────

#[tokio::test]
async fn double_distribution_un_seul_effet() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let appel = appel_4b(&ent, &cast, &[(cast.eleve_lea_id, LigneEtat::Absent)]).await;
    ent.clore_appel(etab, appel.id, cast.prof.user_id, Role::Prof.as_str(), None)
        .await
        .unwrap();

    let d = ent.distributeur(config_test(etab));
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifications(&ent, etab, cast.parent_lea.user_id, "absence_constatee", cast.eleve_lea_id).await, 1);

    // Redistribution forcée (ex. panne après l'effet, avant la marque « distribué »).
    sqlx::query("UPDATE ent_outbox SET statut = 'a_distribuer' WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
    let r = d.distribuer_tout().await.unwrap();
    assert!(r.distribues >= 1);
    assert!(d.metriques().doublons >= 1, "la seconde livraison est reconnue comme doublon");
    assert_eq!(
        notifications(&ent, etab, cast.parent_lea.user_id, "absence_constatee", cast.eleve_lea_id).await,
        1,
        "un seul effet malgré deux livraisons"
    );

    // Deux distributeurs concurrents : aucun effet en double.
    sqlx::query("UPDATE ent_outbox SET statut = 'a_distribuer' WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("DELETE FROM ent_event_traitement WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
    let d2 = ent.distributeur(config_test(etab));
    let (a, b) = tokio::join!(d.distribuer_tout(), d2.distribuer_tout());
    a.unwrap();
    b.unwrap();
    assert_eq!(
        notifications(&ent, etab, cast.parent_lea.user_id, "absence_constatee", cast.eleve_lea_id).await,
        1
    );
    let etat = d.etat(etab).await.unwrap();
    assert_eq!((etat.a_distribuer, etat.echecs), (0, 0));
}

// ─── Ordre par agrégat ───────────────────────────────────────────────────────

#[tokio::test]
async fn ordre_garanti_par_agregat_meme_avec_echec() {
    let Some((ent, etab)) = base().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    for agg in [a, b, a, b, a] {
        publier_seul(&ent, brouillon(etab, "test.ordre", agg, None)).await;
    }
    let mut d = Distributeur::new(ent.pool().clone(), config_test(etab));
    let sonde = Arc::new(Sonde::new("test.ordre", &["test.ordre"]));
    // Le premier événement de A échoue une fois : B avance, A attend.
    sonde.echecs_restants.store(1, Ordering::SeqCst);
    d.enregistrer(sonde.clone()).unwrap();
    assert!(d.enregistrer(sonde.clone()).is_err(), "nom en double refusé");

    let r1 = d.distribuer_un_lot().await.unwrap();
    assert_eq!(r1.lus, 2, "un seul événement par agrégat et par lot");
    assert_eq!(r1.reprises, 1);
    d.distribuer_tout().await.unwrap();

    let vus = sonde.vus.lock().unwrap().clone();
    let ordre_a: Vec<i64> = vus.iter().filter(|(g, _)| *g == a).map(|(_, s)| *s).collect();
    let ordre_b: Vec<i64> = vus.iter().filter(|(g, _)| *g == b).map(|(_, s)| *s).collect();
    assert_eq!(ordre_a, vec![1, 2, 3]);
    assert_eq!(ordre_b, vec![1, 2]);
    assert_eq!(sonde.hors_etab.load(Ordering::SeqCst), 0);
}

// ─── Échec, reprise, file des échecs, rejeu ──────────────────────────────────

#[tokio::test]
async fn echec_reprise_puis_file_des_echecs_et_rejeu() {
    let Some((ent, etab)) = base().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let agg = Uuid::new_v4();
    let e1 = publier_seul(&ent, brouillon(etab, "test.reprise", agg, None)).await;
    let e2 = publier_seul(&ent, brouillon(etab, "test.reprise", agg, None)).await;

    // Délai croissant réel pour vérifier la planification.
    let mut cfg = config_test(etab);
    cfg.delai_base = Duration::from_secs(60);
    cfg.delai_max = Duration::from_secs(3600);
    let mut d = Distributeur::new(ent.pool().clone(), cfg);
    let ok = Arc::new(Sonde::new("test.reprise.ok", &["test.reprise"]));
    let ko = Arc::new(Sonde::new("test.reprise.ko", &["test.reprise"]));
    ko.echoue_toujours.store(true, Ordering::SeqCst);
    d.enregistrer(ok.clone()).unwrap();
    d.enregistrer(ko.clone()).unwrap();

    let r = d.distribuer_tout().await.unwrap();
    assert_eq!((r.lus, r.reprises), (1, 1));
    let (attente,): (bool,) = sqlx::query_as(
        "SELECT prochaine_tentative > now() + interval '50 seconds' FROM ent_outbox WHERE event_id = $1",
    )
    .bind(e1.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(attente, "reprise planifiée avec délai");

    // Temps simulé : deux reprises de plus → plafond (3) → file des échecs.
    for _ in 0..2 {
        sqlx::query("UPDATE ent_outbox SET prochaine_tentative = now() WHERE event_id = $1")
            .bind(e1.id)
            .execute(ent.pool())
            .await
            .unwrap();
        d.distribuer_tout().await.unwrap();
    }
    let echecs = d.file_des_echecs(etab, 10).await.unwrap();
    assert_eq!(echecs.len(), 1);
    assert_eq!(echecs[0].event_id, e1.id);
    assert_eq!(echecs[0].tentatives, 3);
    assert!(echecs[0].derniere_erreur.as_deref().unwrap_or("").contains("test.reprise.ko"));
    // Le successeur du même agrégat reste bloqué ; le consommateur réussi n'a
    // agi qu'une fois malgré les trois tentatives.
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_outbox WHERE event_id = $1 AND statut = 'a_distribuer'", e2.id).await, 1);
    assert_eq!(ok.appels.load(Ordering::SeqCst), 1);
    assert!(d.metriques().echecs >= 1);

    // Correction puis rejeu depuis la file des échecs : tout s'écoule, dans l'ordre.
    ko.echoue_toujours.store(false, Ordering::SeqCst);
    assert!(d.rejouer_echec(etab, e1.id).await.unwrap());
    assert!(!d.rejouer_echec(Uuid::new_v4(), e1.id).await.unwrap(), "rejeu borné à l'établissement");
    d.distribuer_tout().await.unwrap();
    assert_eq!(ok.appels.load(Ordering::SeqCst), 2);
    assert_eq!(ko.appels.load(Ordering::SeqCst), 2);
    assert_eq!(ko.vus.lock().unwrap().iter().map(|(_, s)| *s).collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(d.etat(etab).await.unwrap().echecs, 0);
}

#[tokio::test]
async fn rejeu_d_un_consommateur_sans_rejouer_les_effets_externes() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let appel = appel_4b(&ent, &cast, &[(cast.eleve_lea_id, LigneEtat::Absent)]).await;
    ent.clore_appel(etab, appel.id, cast.prof.user_id, Role::Prof.as_str(), None)
        .await
        .unwrap();

    let mut d = ent.distributeur(config_test(etab));
    let mut envoi = Sonde::new("test.envoi_externe", &[types::ELEVE_ABSENCE_DECLARED]);
    envoi.rejouable = false;
    let envoi = Arc::new(envoi);
    d.enregistrer(envoi.clone()).unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(envoi.appels.load(Ordering::SeqCst), 1);

    // Projection perdue : on la reconstruit par rejeu.
    sqlx::query("DELETE FROM ent_notification WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
    let remis = d.rejouer_consommateur(CONSOMMATEUR_NOTIFICATIONS, etab, 0).await.unwrap();
    assert!(remis >= 1);
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifications(&ent, etab, cast.parent_lea.user_id, "absence_constatee", cast.eleve_lea_id).await, 1);
    assert_eq!(envoi.appels.load(Ordering::SeqCst), 1, "l'envoi externe n'est jamais rejoué");
    assert!(matches!(
        d.rejouer_consommateur("test.envoi_externe", etab, 0).await,
        Err(crate::outbox::ErreurRejeu::NonRejouable)
    ));
    assert!(matches!(
        d.rejouer_consommateur("inconnu", etab, 0).await,
        Err(crate::outbox::ErreurRejeu::ConsommateurInconnu)
    ));
}

// ─── Isolation entre établissements ──────────────────────────────────────────

#[tokio::test]
async fn consommateur_borne_a_l_etablissement_de_l_evenement() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab_a = cast.etablissement_id;
    let etab_b = Uuid::new_v4();
    // Événement forgé dans B qui cite un élève de A : aucune notification.
    let mut forge = brouillon(etab_b, types::ELEVE_ABSENCE_DECLARED, Uuid::new_v4(), Some(cast.eleve_lea_id));
    forge.aggregate = AggregateKind::Absence;
    forge.payload = json!({"debut": "2026-11-05", "source": "appel"});
    let e_b = publier_seul(&ent, forge).await;

    let mut d = ent.distributeur(config_test(etab_b));
    let sonde = Arc::new(Sonde::new("test.isolation", &[types::ELEVE_ABSENCE_DECLARED]));
    d.enregistrer(sonde.clone()).unwrap();
    let r = d.distribuer_tout().await.unwrap();
    assert_eq!(r.distribues, 1);
    assert_eq!(sonde.hors_etab.load(Ordering::SeqCst), 0, "transaction bornée à l'établissement");
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_notification WHERE source_event_id = $1", e_b.id).await,
        0
    );
    assert!(ent.mes_notifications(etab_b, cast.parent_lea.user_id, 50).await.unwrap().is_empty());
    // Un distributeur restreint à B ne touche pas aux événements de A.
    let e_a = publier_seul(&ent, brouillon(etab_a, "test.isolation", Uuid::new_v4(), None)).await;
    d.distribuer_tout().await.unwrap();
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_outbox WHERE event_id = $1 AND statut = 'a_distribuer'", e_a.id).await,
        1
    );
}

// ─── Réactions de bout en bout ───────────────────────────────────────────────

/// Appel clos → absences + retards (même transaction) → notifications des
/// responsables autorisés ; puis décision sur justificatif → notification.
#[tokio::test]
async fn reactions_appel_absence_retard_justificatif() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let jour = today_utc();
    let hermione = eleve_nomme(&cast, "Hermione").clone();
    let ginny = eleve_nomme(&cast, "Ginny").clone();
    assert!(!ginny.peut_voir_vs, "Ginny : lien sans droit vie scolaire");
    let appel = appel_4b(
        &ent,
        &cast,
        &[
            (cast.eleve_lea_id, LigneEtat::Absent),
            (hermione.id, LigneEtat::Absent),
            (ginny.id, LigneEtat::Absent),
        ],
    )
    .await;
    let clos = ent
        .clore_appel(etab, appel.id, cast.prof.user_id, Role::Prof.as_str(), Some(cast.prof.user_id.to_string()))
        .await
        .unwrap();
    assert_eq!(clos.statut, crate::appel_repo::AppelStatut::Clos);
    assert!(ent.brain().unwrap().journal.types().contains(&"appel.closed".to_string()));

    // Avant distribution : le fait est là, la réaction pas encore.
    assert_eq!(notifications(&ent, etab, cast.parent_lea.user_id, "absence_constatee", cast.eleve_lea_id).await, 0);
    // Chaîne de causalité : les absences sont causées par la clôture.
    let (causes,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_outbox o JOIN ent_outbox c ON c.event_id = o.causation_id \
         WHERE c.aggregate_id = $1 AND c.type = 'appel.closed' AND o.type = 'eleve.absence_declared' \
         AND o.profondeur = 1 AND o.correlation_id = c.event_id",
    )
    .bind(appel.id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(causes, 3);

    let d = ent.distributeur(config_test(etab));
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifications(&ent, etab, cast.parent_lea.user_id, "absence_constatee", cast.eleve_lea_id).await, 1);
    assert_eq!(notifications(&ent, etab, hermione.parent.user_id, "absence_constatee", hermione.id).await, 1);
    assert_eq!(
        notifications(&ent, etab, ginny.parent.user_id, "absence_constatee", ginny.id).await,
        0,
        "responsable sans droit vie scolaire : pas de notification"
    );
    // Aucun texte libre de l'événement recopié, pas d'information d'un autre enfant.
    let n = ent.mes_notifications(etab, cast.parent_lea.user_id, 50).await.unwrap();
    assert!(n.iter().all(|x| x.eleve_id == Some(cast.eleve_lea_id)));
    assert!(n.iter().any(|x| x.titre.contains("Harry")));

    // Justificatifs : accepté pour Harry, refusé pour Hermione.
    let file = ent.list_absences_a_traiter(etab, jour).await.unwrap();
    let abs_harry = file.iter().find(|a| a.eleve_id == cast.eleve_lea_id).unwrap().id;
    let abs_hermione = file.iter().find(|a| a.eleve_id == hermione.id).unwrap().id;
    ent.accepter_justificatif_for_actor(etab, cast.cpe.user_id, Role::Cpe.as_str(), abs_harry, Some("note interne".into()))
        .await
        .unwrap();
    ent.refuser_justificatif_for_actor(etab, cast.cpe.user_id, Role::Cpe.as_str(), abs_hermione, None)
        .await
        .unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifications(&ent, etab, cast.parent_lea.user_id, "absence_justifiee", cast.eleve_lea_id).await, 1);
    assert_eq!(notifications(&ent, etab, hermione.parent.user_id, "absence_refusee", hermione.id).await, 1);
    let n = ent.mes_notifications(etab, cast.parent_lea.user_id, 50).await.unwrap();
    assert!(n.iter().all(|x| !x.corps.contains("note interne")), "note de décision jamais recopiée");

    // Droits relus au traitement : lien retiré avant distribution → rien.
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_vs = false WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(cast.eleve_lea_id)
        .execute(ent.pool())
        .await
        .unwrap();
    let mut evt = brouillon(etab, types::RETARD_DECLARE, Uuid::new_v4(), Some(cast.eleve_lea_id));
    evt.aggregate = AggregateKind::Retard;
    evt.payload = json!({"jour": jour.to_string()});
    publier_seul(&ent, evt).await;
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifications(&ent, etab, cast.parent_lea.user_id, "retard", cast.eleve_lea_id).await, 0);
}

/// Retard pointé à l'appel → fait canonique + événement en transaction →
/// notification du responsable autorisé.
#[tokio::test]
async fn reaction_retard_a_l_appel() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let appel = appel_4b(&ent, &cast, &[(cast.eleve_lea_id, LigneEtat::Retard)]).await;
    ent.clore_appel(etab, appel.id, cast.prof.user_id, Role::Prof.as_str(), None)
        .await
        .unwrap();
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_retard WHERE source_appel_id = $1", appel.id).await,
        1
    );
    let d = ent.distributeur(config_test(etab));
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifications(&ent, etab, cast.parent_lea.user_id, "retard", cast.eleve_lea_id).await, 1);
    assert_eq!(notifications(&ent, etab, cast.parent_lea.user_id, "absence_constatee", cast.eleve_lea_id).await, 0);
}

/// Absence déclarée par la famille (hors appel) : pas de notification à la famille.
#[tokio::test]
async fn absence_hors_appel_non_notifiee() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let mut evt = brouillon(etab, types::ELEVE_ABSENCE_DECLARED, Uuid::new_v4(), Some(cast.eleve_lea_id));
    evt.aggregate = AggregateKind::Absence;
    evt.payload = json!({"debut": "2026-11-05", "source": "famille"});
    publier_seul(&ent, evt).await;
    let d = ent.distributeur(config_test(etab));
    d.distribuer_tout().await.unwrap();
    assert_eq!(notifications(&ent, etab, cast.parent_lea.user_id, "absence_constatee", cast.eleve_lea_id).await, 0);
}

/// La boucle tokio distribue d'elle-même puis s'arrête sur demande.
#[tokio::test]
async fn boucle_de_distribution_en_tache_de_fond() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let appel = appel_4b(&ent, &cast, &[(cast.eleve_lea_id, LigneEtat::Absent)]).await;
    ent.clore_appel(etab, appel.id, cast.prof.user_id, Role::Prof.as_str(), None)
        .await
        .unwrap();
    let d = Arc::new(ent.distributeur(config_test(etab)));
    let tache = d.clone().lancer();
    let mut ok = false;
    for _ in 0..200 {
        if notifications(&ent, etab, cast.parent_lea.user_id, "absence_constatee", cast.eleve_lea_id).await == 1 {
            ok = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    d.arreter();
    tokio::time::timeout(Duration::from_secs(5), tache).await.unwrap().unwrap();
    assert!(ok, "notification produite par la boucle de fond");
    assert!(d.metriques().lots >= 1);
}
