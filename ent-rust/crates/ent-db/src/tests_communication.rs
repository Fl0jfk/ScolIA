//! Socle 2, lot S2-8 : communication complète contre la base locale de test
//! (feature `labo`). Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.
//!
//! Couvre : préférences et regroupement, heures calmes, plafonds SMS,
//! idempotence des envois (y compris deux expéditeurs concurrents), droits relus
//! à l'envoi, campagnes (confirmation obligatoire, ciblage selon les droits,
//! aperçu périmé), rendez-vous, formulaires et consentements (appliqués au
//! trombinoscope), isolation entre établissements.
//!
//! Aucun appel réseau réel : e-mail, SMS et push sont en mode simulé.

use super::*;
use crate::campagnes::{ConfirmationLancement, NouvelleCampagne};
use crate::formulaires::{NouveauFormulaire, ReponseFormulaire};
use crate::notifications::{ConfigExpedition, Plafonds, Preferences};
use crate::outbox::{ConfigDistribution, Publier};
use crate::rencontres::{NouveauxCreneaux, NouvelleRencontre};
use chrono::{DateTime, Duration, Timelike, Utc};
use ent_auth::{Action, Portee, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::json;
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    // Mail, SMS et push en mode simulé : jamais de réseau.
    ent.install_mail_dry_run();
    // Le jeu de données du labo est semé une seule fois par exécution (coûteux) ; chaque
    // test se reconnecte avec son propre pool (un pool ne survit pas à son runtime de test).
    static CAST: std::sync::OnceLock<LaboCast> = std::sync::OnceLock::new();
    let cast = match CAST.get() {
        Some(c) => c.clone(),
        None => {
            let c = ent.seed_labo_cast().await.expect("seed cast");
            let _ = CAST.set(c.clone());
            c
        }
    };
    reset(&ent, cast.etablissement_id).await;
    Some((ent, cast))
}

/// Repart d'un état propre pour les tables du lot (la base de test est réutilisée).
async fn reset(ent: &PgEnt, etab: Uuid) {
    for t in [
        "ent_notification_envoi",
        "ent_campagne",
        "ent_creneau_rdv",
        "ent_rencontre",
        "ent_formulaire_reponse",
        "ent_formulaire",
        "ent_preference_notification",
        "ent_notification_plafond",
    ] {
        sqlx::query(&format!("DELETE FROM {t} WHERE etablissement_id = $1"))
            .bind(etab)
            .execute(ent.pool())
            .await
            .unwrap();
    }
    // Les liens suspendus par un test précédent sont rétablis.
    sqlx::query("UPDATE ent_eleve_lien SET suspendu_le = NULL, suspendu_par = NULL, suspension_motif = NULL, suspension_reference = NULL WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
}

fn dir(c: &LaboCast) -> (Uuid, &'static str) {
    (c.direction.user_id, Role::Direction.as_str())
}

fn expediteur(ent: &PgEnt, etab: Uuid) -> crate::notifications::Expediteur {
    ent.expediteur(ConfigExpedition {
        taille_lot: 500,
        etablissement: Some(etab),
        ..ConfigExpedition::default()
    })
}

fn config_distribution(etab: Uuid) -> ConfigDistribution {
    ConfigDistribution {
        taille_lot: 100,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    }
}

/// Donne un téléphone et un appareil push au parent (canaux joignables).
async fn rendre_joignable(ent: &PgEnt, etab: Uuid, user: Uuid, tel: &str) {
    sqlx::query("UPDATE ent_user SET telephone = $2 WHERE id = $1")
        .bind(user)
        .bind(tel)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_push_device (id, etablissement_id, user_id, platform, token)
         VALUES ($1,$2,$3,'android',$4) ON CONFLICT (etablissement_id, token) DO UPDATE SET revoked_at = NULL",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(user)
    .bind(format!("tok-s28-{user}"))
    .execute(ent.pool())
    .await
    .unwrap();
}

/// Publie un retard pour l'élève (déclenche la notification de la famille), puis distribue.
async fn retard(ent: &PgEnt, etab: Uuid, eleve: Uuid) -> Uuid {
    let mut tx = ent.pool().begin().await.unwrap();
    let e = tx
        .publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::RETARD_DECLARE.to_string(),
            aggregate: AggregateKind::Eleve,
            aggregate_id: eleve,
            eleve_id: Some(eleve),
            payload: json!({ "jour": "2026-11-06" }),
            actor_user_id: None,
        })
        .await
        .unwrap();
    tx.commit().await.unwrap();
    ent.distributeur(config_distribution(etab)).distribuer_tout().await.unwrap();
    e.id
}

async fn statuts(ent: &PgEnt, etab: Uuid, user: Uuid, canal: &str) -> Vec<(String, Option<String>)> {
    sqlx::query_as(
        "SELECT statut, motif FROM ent_notification_envoi
          WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND canal = $3
          ORDER BY created_at, id",
    )
    .bind(etab)
    .bind(user)
    .bind(canal)
    .fetch_all(ent.pool())
    .await
    .unwrap()
}

async fn nb_envois(ent: &PgEnt, etab: Uuid, user: Uuid, canal: &str, statut: &str) -> i64 {
    statuts(ent, etab, user, canal).await.iter().filter(|s| s.0 == statut).count() as i64
}

fn pref(f: impl FnOnce(&mut Preferences)) -> Preferences {
    let mut p = Preferences::default();
    f(&mut p);
    p
}

async fn classe_id(ent: &PgEnt, etab: Uuid, code: &str) -> Uuid {
    let (id,): (Uuid,) = sqlx::query_as(
        "SELECT c.id FROM ent_classe c
           JOIN ent_annee_scolaire y ON y.id = c.annee_id AND y.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.code = $2 AND c.archivee_le IS NULL
          ORDER BY y.courante DESC, y.date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .bind(code)
    .fetch_one(ent.pool())
    .await
    .expect("classe amorcée par le seed");
    id
}

/// Le professeur du cast a la classe en service (sinon on la lui donne).
async fn donner_service(ent: &PgEnt, etab: Uuid, prof: Uuid, classe: Uuid) {
    sqlx::query(
        "INSERT INTO ent_service_enseignement
            (id, etablissement_id, annee_id, prof_user_id, matiere_libelle, classe_id)
         SELECT $1, c.etablissement_id, c.annee_id, $3, 'Potions', c.id
           FROM ent_classe c
          WHERE c.etablissement_id = $2 AND c.id = $4
            AND NOT EXISTS (SELECT 1 FROM ent_service_enseignement s
                             WHERE s.etablissement_id = c.etablissement_id AND s.prof_user_id = $3
                               AND s.classe_id = c.id)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(prof)
    .bind(classe)
    .execute(ent.pool())
    .await
    .unwrap();
}

// ─── 1. Préférences ─────────────────────────────────────────────────────────

#[tokio::test]
async fn preferences_defauts_validation_et_cloisonnement() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    // Défauts : e-mail et push actifs, SMS désactivé (consentement explicite).
    let p = ent.mes_preferences_notification(etab, lea).await.unwrap();
    assert_eq!(p, Preferences::default());
    assert!(p.email && p.push && !p.sms);
    // Enregistrement et relecture.
    let voulu = pref(|p| {
        p.sms = true;
        p.resume = "quotidien".into();
        p.calme_debut = Some(21 * 60);
        p.calme_fin = Some(7 * 60);
    });
    ent.definir_preferences_notification(etab, lea, voulu.clone()).await.unwrap();
    assert_eq!(ent.mes_preferences_notification(etab, lea).await.unwrap(), voulu);
    // Les préférences d'une autre personne ne bougent pas.
    assert_eq!(
        ent.mes_preferences_notification(etab, cast.parent_tom.user_id).await.unwrap(),
        Preferences::default()
    );
    // Validation.
    assert!(ent.definir_preferences_notification(etab, lea, pref(|p| p.resume = "hebdo".into())).await.is_err());
    assert!(ent
        .definir_preferences_notification(etab, lea, pref(|p| { p.calme_debut = Some(10); p.calme_fin = None }))
        .await
        .is_err());
    // Une personne étrangère à l'établissement ne peut rien y enregistrer.
    assert!(matches!(
        ent.definir_preferences_notification(Uuid::new_v4(), lea, Preferences::default()).await,
        Err(EntDbError::Forbidden)
    ));
}

// ─── 2. Envois selon les préférences, traçabilité, dédoublonnage ─────────────

#[tokio::test]
async fn envois_selon_preferences_et_tracabilite() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    rendre_joignable(&ent, etab, lea, "06 12 34 56 78").await;
    let evt = retard(&ent, etab, cast.eleve_lea_id).await;
    // Défauts : e-mail et push planifiés, pas de SMS ; la notification dans l'ENT existe.
    assert_eq!(statuts(&ent, etab, lea, "email").await.len(), 1);
    assert_eq!(statuts(&ent, etab, lea, "push").await.len(), 1);
    assert!(statuts(&ent, etab, lea, "sms").await.is_empty(), "SMS désactivé par défaut");
    assert!(ent.mes_notifications(etab, lea, 50).await.unwrap().iter().any(|n| n.source_event_id == evt));
    // Expédition : simulée (aucun réseau), tracée.
    let b = expediteur(&ent, etab).expedier(Utc::now()).await.unwrap();
    assert_eq!(b.simules, 2, "{b:?}");
    assert_eq!(nb_envois(&ent, etab, lea, "email", "simule").await, 1);
    let (mails,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_mail_outbox m JOIN ent_notification_envoi e ON e.id = m.source_id
          WHERE e.etablissement_id = $1 AND e.destinataire_user_id = $2",
    )
    .bind(etab)
    .bind(lea)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(mails, 1, "l'e-mail est journalisé dans l'outbox existante");
    // Traçabilité côté personne : sans adresse ni numéro.
    let vus = ent.mes_envois_notification(etab, lea, 20).await.unwrap();
    assert!(vus.iter().all(|v| !format!("{v:?}").contains('@')));
    // Opt-in SMS : un nouvel événement part aussi par SMS.
    ent.definir_preferences_notification(etab, lea, pref(|p| p.sms = true)).await.unwrap();
    retard(&ent, etab, cast.eleve_lea_id).await;
    expediteur(&ent, etab).expedier(Utc::now()).await.unwrap();
    assert_eq!(nb_envois(&ent, etab, lea, "sms", "simule").await, 1);
    // Canal e-mail coupé : plus d'e-mail pour le suivant, l'ENT reste alimenté.
    ent.definir_preferences_notification(etab, lea, pref(|p| p.email = false)).await.unwrap();
    let avant = ent.mes_notifications(etab, lea, 100).await.unwrap().len();
    retard(&ent, etab, cast.eleve_lea_id).await;
    assert_eq!(statuts(&ent, etab, lea, "email").await.len(), 2, "pas de nouvel e-mail");
    assert_eq!(ent.mes_notifications(etab, lea, 100).await.unwrap().len(), avant + 1);
}

#[tokio::test]
async fn regroupement_en_resume_quotidien() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    ent.definir_preferences_notification(etab, lea, pref(|p| { p.resume = "quotidien".into(); p.push = false }))
        .await
        .unwrap();
    retard(&ent, etab, cast.eleve_lea_id).await;
    retard(&ent, etab, cast.eleve_lea_id).await;
    assert_eq!(statuts(&ent, etab, lea, "email").await.len(), 2);
    // Pas d'envoi immédiat : le résumé part au prochain créneau du matin.
    let b = expediteur(&ent, etab).expedier(Utc::now()).await.unwrap();
    assert_eq!(b.reclames, 0, "rien d'échu maintenant");
    // Le lendemain matin (+2 jours) : un seul e-mail pour les deux messages.
    let b = expediteur(&ent, etab).expedier(Utc::now() + Duration::days(2)).await.unwrap();
    assert_eq!(b.regroupes, 2, "{b:?}");
    let (nb, sujet): (i64, String) = sqlx::query_as(
        "SELECT count(*), max(m.subject) FROM ent_mail_outbox m
           JOIN ent_notification_envoi e ON e.id = m.source_id
          WHERE e.etablissement_id = $1 AND e.destinataire_user_id = $2",
    )
    .bind(etab)
    .bind(lea)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(nb, 1, "un seul e-mail pour le résumé");
    assert!(sujet.contains("Résumé") && sujet.contains('2'), "{sujet}");
    let (distincts,): (i64,) = sqlx::query_as(
        "SELECT count(DISTINCT resume_id) FROM ent_notification_envoi
          WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND canal = 'email' AND statut = 'regroupe'",
    )
    .bind(etab)
    .bind(lea)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(distincts, 1);
}

#[tokio::test]
async fn heures_calmes_reportent_l_envoi() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    // Heures calmes qui contiennent « maintenant » (heure de Paris).
    let local = heure_paris(Utc::now());
    let m = (local.hour() * 60 + local.minute()) as i64;
    let debut = ((m + 1440 - 30) % 1440) as u16;
    let fin = ((m + 60) % 1440) as u16;
    ent.definir_preferences_notification(
        etab,
        lea,
        pref(|p| { p.calme_debut = Some(debut); p.calme_fin = Some(fin); p.push = false }),
    )
    .await
    .unwrap();
    retard(&ent, etab, cast.eleve_lea_id).await;
    let b = expediteur(&ent, etab).expedier(Utc::now()).await.unwrap();
    assert_eq!(b.reclames, 0, "rien ne part pendant les heures calmes");
    assert_eq!(nb_envois(&ent, etab, lea, "email", "en_attente").await, 1);
    // Deux heures plus tard : hors des heures calmes.
    let b = expediteur(&ent, etab).expedier(Utc::now() + Duration::hours(2)).await.unwrap();
    assert_eq!(b.simules, 1, "{b:?}");
}

#[tokio::test]
async fn envois_idempotents_meme_en_concurrence() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    ent.definir_preferences_notification(etab, lea, pref(|p| p.push = false)).await.unwrap();
    let evt = retard(&ent, etab, cast.eleve_lea_id).await;
    // Rejeu de l'événement : le consommateur est rejouable, il ne crée aucun envoi de plus.
    sqlx::query("DELETE FROM ent_event_traitement WHERE etablissement_id = $1 AND event_id = $2")
        .bind(etab)
        .bind(evt)
        .execute(ent.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_outbox SET statut = 'a_distribuer', prochaine_tentative = now() WHERE etablissement_id = $1 AND event_id = $2")
        .bind(etab)
        .bind(evt)
        .execute(ent.pool())
        .await
        .unwrap();
    ent.distributeur(config_distribution(etab)).distribuer_tout().await.unwrap();
    assert_eq!(statuts(&ent, etab, lea, "email").await.len(), 1, "rejeu : pas de second envoi");
    // Deux expéditeurs simultanés : un seul e-mail.
    let a = expediteur(&ent, etab);
    let b = expediteur(&ent, etab);
    let (ra, rb) = tokio::join!(a.expedier(Utc::now()), b.expedier(Utc::now()));
    let total = ra.unwrap().simules + rb.unwrap().simules;
    assert_eq!(total, 1);
    // Un passage de plus ne renvoie rien.
    assert_eq!(expediteur(&ent, etab).expedier(Utc::now()).await.unwrap().reclames, 0);
    let (mails,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_mail_outbox m JOIN ent_notification_envoi e ON e.id = m.source_id
          WHERE e.etablissement_id = $1 AND e.destinataire_user_id = $2",
    )
    .bind(etab)
    .bind(lea)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(mails, 1);
}

// ─── 3. Plafonds SMS ─────────────────────────────────────────────────────────

#[tokio::test]
async fn plafonds_sms_par_personne_et_par_etablissement() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let (adm, role) = dir(&cast);
    let lea = cast.parent_lea.user_id;
    rendre_joignable(&ent, etab, lea, "0612345678").await;
    ent.definir_preferences_notification(etab, lea, pref(|p| { p.sms = true; p.email = false; p.push = false }))
        .await
        .unwrap();
    // Seule la direction règle les plafonds ; un professeur n'y accède pas.
    assert!(matches!(
        ent.definir_plafonds_notification(etab, cast.prof.user_id, "prof", Plafonds::default()).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .definir_plafonds_notification(etab, adm, role, Plafonds { sms_par_jour: -1, ..Plafonds::default() })
        .await
        .is_err());
    // Plafond par personne : 2 SMS sur 4 messages.
    ent.definir_plafonds_notification(etab, adm, role, Plafonds { sms_par_personne_jour: 2, ..Plafonds::default() })
        .await
        .unwrap();
    for _ in 0..4 {
        retard(&ent, etab, cast.eleve_lea_id).await;
    }
    let b = expediteur(&ent, etab).expedier(Utc::now()).await.unwrap();
    assert_eq!((b.simules, b.plafonnes), (2, 2), "{b:?}");
    let s = statuts(&ent, etab, lea, "sms").await;
    assert!(s.iter().filter(|x| x.0 == "plafonne").all(|x| x.1.as_deref() == Some("plafond_personne")));
    assert_eq!(ent.sms().receipts().len() >= 2, true);
    // Plafond par établissement : 1 SMS par jour pour tous.
    reset(&ent, etab).await;
    ent.definir_preferences_notification(etab, lea, pref(|p| { p.sms = true; p.email = false; p.push = false }))
        .await
        .unwrap();
    ent.definir_plafonds_notification(
        etab,
        adm,
        role,
        Plafonds { sms_par_jour: 1, sms_par_personne_jour: 10, ..Plafonds::default() },
    )
    .await
    .unwrap();
    for _ in 0..3 {
        retard(&ent, etab, cast.eleve_lea_id).await;
    }
    let b = expediteur(&ent, etab).expedier(Utc::now()).await.unwrap();
    assert_eq!((b.simules, b.plafonnes), (1, 2), "{b:?}");
    assert!(statuts(&ent, etab, lea, "sms")
        .await
        .iter()
        .any(|x| x.1.as_deref() == Some("plafond_etablissement")));
    // Les plafonds sont lus par la direction.
    let p = ent.plafonds_notification_for_actor(etab, adm, role).await.unwrap();
    assert_eq!(p.sms_par_jour, 1);
}

// ─── 4. Droits relus à l'envoi, isolation ────────────────────────────────────

#[tokio::test]
async fn droit_retire_avant_l_envoi_annule_l_envoi() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.parent_lea.user_id;
    ent.definir_preferences_notification(etab, lea, pref(|p| p.push = false)).await.unwrap();
    retard(&ent, etab, cast.eleve_lea_id).await;
    assert_eq!(nb_envois(&ent, etab, lea, "email", "en_attente").await, 1);
    // Le lien est suspendu (décision de justice) avant l'expédition.
    sqlx::query("UPDATE ent_eleve_lien SET suspendu_le = now(), suspension_motif = 'test', suspension_reference = 'ref' WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab)
        .bind(cast.eleve_lea_id)
        .bind(lea)
        .execute(ent.pool())
        .await
        .unwrap();
    let b = expediteur(&ent, etab).expedier(Utc::now()).await.unwrap();
    assert_eq!((b.ignores, b.simules), (1, 0), "{b:?}");
    let s = statuts(&ent, etab, lea, "email").await;
    assert_eq!(s[0], ("ignore".to_string(), Some("droit_retire".to_string())));
    reset(&ent, etab).await;
}

#[tokio::test]
async fn expediteur_limite_a_son_etablissement() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    ent.definir_preferences_notification(etab, cast.parent_lea.user_id, pref(|p| p.push = false)).await.unwrap();
    retard(&ent, etab, cast.eleve_lea_id).await;
    // Un expéditeur restreint à un autre établissement ne touche pas à celui-ci.
    let b = expediteur(&ent, Uuid::new_v4()).expedier(Utc::now()).await.unwrap();
    assert_eq!(b.reclames, 0);
    assert_eq!(nb_envois(&ent, etab, cast.parent_lea.user_id, "email", "en_attente").await, 1);
    // Un destinataire sans lien avec l'élève d'un autre établissement ne reçoit rien :
    // (événement forgé dans un autre établissement citant l'élève de celui-ci)
    let autre = Uuid::new_v4();
    let mut tx = ent.pool().begin().await.unwrap();
    tx.publier(MetierEventDraft {
        etablissement_id: autre,
        type_name: types::RETARD_DECLARE.to_string(),
        aggregate: AggregateKind::Eleve,
        aggregate_id: cast.eleve_lea_id,
        eleve_id: Some(cast.eleve_lea_id),
        payload: json!({ "jour": "2026-11-06" }),
        actor_user_id: None,
    })
    .await
    .unwrap();
    tx.commit().await.unwrap();
    ent.distributeur(config_distribution(autre)).distribuer_tout().await.unwrap();
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_notification_envoi WHERE etablissement_id = $1")
        .bind(autre)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 0);
    reset(&ent, etab).await;
}

// ─── 5. Campagnes ────────────────────────────────────────────────────────────

fn nouvelle(classe: Uuid, canaux: &[&str]) -> NouvelleCampagne {
    NouvelleCampagne {
        titre: "Réunion de rentrée".into(),
        corps: "La réunion aura lieu jeudi à 18 h.".into(),
        canaux: canaux.iter().map(|c| c.to_string()).collect(),
        audience: "classe".into(),
        cible_ids: vec![classe],
    }
}

#[tokio::test]
async fn campagne_confirmation_obligatoire_et_idempotence() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let (adm, role) = dir(&cast);
    let c4b = classe_id(&ent, etab, "4B").await;
    let lea = cast.parent_lea.user_id;
    let camp = ent.creer_campagne(etab, adm, role, nouvelle(c4b, &["email", "push"])).await.unwrap();
    assert_eq!(camp.statut, "brouillon");
    // Pas de lancement sans aperçu.
    let sans_apercu = ent
        .lancer_campagne(etab, adm, role, camp.id, ConfirmationLancement { empreinte: "x".into(), nb_destinataires: 1, confirme_sms: false })
        .await;
    assert!(matches!(sans_apercu, Err(EntDbError::SaisieInvalide(_))), "{sans_apercu:?}");
    // Aperçu : destinataires nommés, sans adresse ni numéro, comptage par canal.
    let ap = ent.apercu_campagne(etab, adm, role, camp.id).await.unwrap();
    assert!(ap.nb_destinataires >= 1 && ap.nb_eleves >= 1);
    assert!(ap.destinataires.iter().any(|d| d.nom.contains("Potter") || !d.nom.is_empty()));
    assert!(!format!("{ap:?}").contains('@'));
    assert!(ap.par_canal.get("email").copied().unwrap_or(0) >= 1);
    assert_eq!(ap.sms.is_none(), true);
    // Rien n'est parti avant la confirmation.
    let (avant,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_notification_envoi WHERE etablissement_id = $1 AND source_type = 'campagne'")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(avant, 0);
    // Mauvaise empreinte : refus.
    let mauvaise = ent
        .lancer_campagne(etab, adm, role, camp.id, ConfirmationLancement { empreinte: "0".repeat(64), nb_destinataires: ap.nb_destinataires, confirme_sms: false })
        .await;
    assert!(matches!(mauvaise, Err(EntDbError::ConflitDossier(_))), "{mauvaise:?}");
    // Un autre utilisateur (même établissement) ne peut ni voir ni lancer.
    assert!(ent.campagne_for_actor(etab, cast.cpe.user_id, "cpe", camp.id).await.is_err());
    // Confirmation correcte.
    let conf = ConfirmationLancement { empreinte: ap.empreinte.clone(), nb_destinataires: ap.nb_destinataires, confirme_sms: false };
    let lancee = ent.lancer_campagne(etab, adm, role, camp.id, conf.clone()).await.unwrap();
    assert_eq!(lancee.statut, "lancee");
    assert_eq!(lancee.nb_destinataires, Some(ap.nb_destinataires as i32));
    // Notification dans l'ENT + envois planifiés ; e-mail par défaut, pas de SMS.
    assert!(ent.mes_notifications(etab, lea, 50).await.unwrap().iter().any(|n| n.kind == "campagne"));
    assert_eq!(statuts(&ent, etab, lea, "email").await.len(), 1);
    assert!(statuts(&ent, etab, lea, "sms").await.is_empty());
    // Second lancement (double clic, rejeu) : refusé, aucun envoi de plus.
    let encore = ent.lancer_campagne(etab, adm, role, camp.id, conf).await;
    assert!(matches!(encore, Err(EntDbError::ConflitDossier(_))), "{encore:?}");
    assert_eq!(statuts(&ent, etab, lea, "email").await.len(), 1);
    // Événement publié, sans texte libre.
    let (evt,): (serde_json::Value,) = sqlx::query_as("SELECT m.payload FROM ent_metier_event m WHERE m.etablissement_id = $1 AND m.type = 'campagne.lancee' ORDER BY m.created_at DESC LIMIT 1")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(!evt.to_string().contains("réunion"), "{evt}");
    // Expédition tracée, bilan de la campagne.
    expediteur(&ent, etab).expedier(Utc::now()).await.unwrap();
    let bilan = ent.bilan_envois_source(etab, "campagne", camp.id).await.unwrap();
    assert!(bilan.iter().any(|(c, s, n)| c == "email" && s == "simule" && *n >= 1), "{bilan:?}");
    reset(&ent, etab).await;
}

#[tokio::test]
async fn campagne_sms_double_confirmation_et_plafonds() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let (adm, role) = dir(&cast);
    let c4b = classe_id(&ent, etab, "4B").await;
    let lea = cast.parent_lea.user_id;
    rendre_joignable(&ent, etab, lea, "0612345678").await;
    ent.definir_preferences_notification(etab, lea, pref(|p| p.sms = true)).await.unwrap();
    let camp = ent.creer_campagne(etab, adm, role, nouvelle(c4b, &["sms"])).await.unwrap();
    let ap = ent.apercu_campagne(etab, adm, role, camp.id).await.unwrap();
    let sms = ap.sms.clone().expect("estimation SMS");
    assert!(sms.destinataires >= 1 && sms.credits_estimes >= sms.destinataires && !sms.depasse);
    let conf = |ok| ConfirmationLancement { empreinte: ap.empreinte.clone(), nb_destinataires: ap.nb_destinataires, confirme_sms: ok };
    // Sans la seconde confirmation : refus.
    let r = ent.lancer_campagne(etab, adm, role, camp.id, conf(false)).await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    // Plafond SMS par campagne trop bas : refus.
    ent.definir_plafonds_notification(etab, adm, role, Plafonds { sms_par_campagne: 0, ..Plafonds::default() })
        .await
        .unwrap();
    let r = ent.lancer_campagne(etab, adm, role, camp.id, conf(true)).await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    // Plafond du jour trop bas : refus.
    ent.definir_plafonds_notification(etab, adm, role, Plafonds { sms_par_jour: 0, ..Plafonds::default() })
        .await
        .unwrap();
    assert!(matches!(ent.lancer_campagne(etab, adm, role, camp.id, conf(true)).await, Err(EntDbError::ConflitDossier(_))));
    // Plafonds normaux : lancement, puis SMS expédié (simulé).
    ent.definir_plafonds_notification(etab, adm, role, Plafonds::default()).await.unwrap();
    ent.lancer_campagne(etab, adm, role, camp.id, conf(true)).await.unwrap();
    expediteur(&ent, etab).expedier(Utc::now()).await.unwrap();
    assert_eq!(nb_envois(&ent, etab, lea, "sms", "simule").await, 1);
    // Plafond de campagnes par jour.
    ent.definir_plafonds_notification(etab, adm, role, Plafonds { campagnes_par_jour: 1, ..Plafonds::default() })
        .await
        .unwrap();
    let c2 = ent.creer_campagne(etab, adm, role, nouvelle(c4b, &["email"])).await.unwrap();
    let ap2 = ent.apercu_campagne(etab, adm, role, c2.id).await.unwrap();
    let r = ent
        .lancer_campagne(etab, adm, role, c2.id, ConfirmationLancement { empreinte: ap2.empreinte, nb_destinataires: ap2.nb_destinataires, confirme_sms: false })
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "{r:?}");
    reset(&ent, etab).await;
}

#[tokio::test]
async fn campagne_apercu_perime_si_un_lien_change() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let (adm, role) = dir(&cast);
    let c4b = classe_id(&ent, etab, "4B").await;
    let camp = ent.creer_campagne(etab, adm, role, nouvelle(c4b, &["email"])).await.unwrap();
    let ap = ent.apercu_campagne(etab, adm, role, camp.id).await.unwrap();
    // Entre l'aperçu et le lancement, le lien d'un responsable est suspendu.
    sqlx::query("UPDATE ent_eleve_lien SET suspendu_le = now(), suspension_motif = 'test', suspension_reference = 'ref' WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab)
        .bind(cast.eleve_lea_id)
        .bind(cast.parent_lea.user_id)
        .execute(ent.pool())
        .await
        .unwrap();
    let r = ent
        .lancer_campagne(etab, adm, role, camp.id, ConfirmationLancement { empreinte: ap.empreinte.clone(), nb_destinataires: ap.nb_destinataires, confirme_sms: false })
        .await;
    assert!(matches!(r, Err(EntDbError::ConflitDossier(_))), "aperçu périmé : {r:?}");
    // Nouvel aperçu : le responsable suspendu n'y figure plus.
    let ap2 = ent.apercu_campagne(etab, adm, role, camp.id).await.unwrap();
    assert!(ap2.nb_destinataires < ap.nb_destinataires || ap2.empreinte != ap.empreinte);
    reset(&ent, etab).await;
}

#[tokio::test]
async fn campagne_ciblage_limite_aux_droits() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let c4b = classe_id(&ent, etab, "4B").await;
    let c5a = classe_id(&ent, etab, "5A").await;
    // Un professeur n'a pas l'action par défaut.
    assert!(matches!(
        ent.creer_campagne(etab, cast.prof.user_id, "prof", nouvelle(c4b, &["email"])).await,
        Err(EntDbError::Forbidden)
    ));
    // Un rôle limité à la classe 4B (portée désignée) lui est attribué.
    let role = ent
        .creer_role_for_actor(
            etab,
            cast.admin.user_id,
            "admin",
            RoleSaisie {
                nom: format!("Campagnes 4B {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![Droit { action: Action::CampagnesLancer, portee: Portee::Designes, cibles: vec![c4b] }],
            },
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, cast.admin.user_id, "admin", role.id, cast.prof.user_id, None)
        .await
        .unwrap();
    let prof = (cast.prof.user_id, "prof");
    // Établissement entier : interdit avec une portée limitée.
    let tout = NouvelleCampagne { audience: "etablissement".into(), cible_ids: vec![], ..nouvelle(c4b, &["email"]) };
    assert!(matches!(ent.creer_campagne(etab, prof.0, prof.1, tout).await, Err(EntDbError::Forbidden)));
    // Classe désignée : aperçu non vide.
    let ok = ent.creer_campagne(etab, prof.0, prof.1, nouvelle(c4b, &["email"])).await.unwrap();
    let ap = ent.apercu_campagne(etab, prof.0, prof.1, ok.id).await.unwrap();
    assert!(ap.nb_destinataires >= 1 && ap.exclus_hors_droits == 0);
    // Autre classe : aucun destinataire, tous les élèves « hors droits », lancement impossible.
    let hors = ent.creer_campagne(etab, prof.0, prof.1, nouvelle(c5a, &["email"])).await.unwrap();
    let ap = ent.apercu_campagne(etab, prof.0, prof.1, hors.id).await.unwrap();
    assert_eq!(ap.nb_destinataires, 0);
    assert!(ap.exclus_hors_droits >= 1);
    let r = ent
        .lancer_campagne(etab, prof.0, prof.1, hors.id, ConfirmationLancement { empreinte: ap.empreinte, nb_destinataires: 0, confirme_sms: false })
        .await;
    assert!(matches!(r, Err(EntDbError::SaisieInvalide(_))), "{r:?}");
    // Un parent n'a aucun accès aux campagnes ; ni un autre établissement.
    assert!(matches!(
        ent.campagnes_for_actor(etab, cast.parent_lea.user_id, "parent").await,
        Err(EntDbError::Forbidden)
    ));
    // Autre établissement : aucune campagne visible, et celle-ci y est introuvable.
    assert!(ent
        .campagnes_for_actor(Uuid::new_v4(), cast.direction.user_id, "direction")
        .await
        .unwrap()
        .is_empty());
    assert!(ent
        .campagne_for_actor(Uuid::new_v4(), cast.direction.user_id, "direction", ok.id)
        .await
        .is_err());
    // Audience avec cible d'un autre établissement : refusée.
    let (adm, rl) = dir(&cast);
    let faux = NouvelleCampagne { cible_ids: vec![Uuid::new_v4()], ..nouvelle(c4b, &["email"]) };
    assert!(matches!(ent.creer_campagne(etab, adm, rl, faux).await, Err(EntDbError::SaisieInvalide(_))));
    // Retrait du rôle : droits retirés immédiatement.
    ent.supprimer_role_for_actor(etab, cast.admin.user_id, "admin", role.id).await.unwrap();
    assert!(matches!(
        ent.apercu_campagne(etab, prof.0, prof.1, ok.id).await,
        Err(EntDbError::Forbidden)
    ));
    reset(&ent, etab).await;
}

// ─── 6. Rendez-vous et rencontres ────────────────────────────────────────────

fn demain_9h() -> DateTime<Utc> {
    (Utc::now() + Duration::days(3)).date_naive().and_hms_opt(9, 0, 0).unwrap().and_utc()
}

#[tokio::test]
async fn rencontres_parents_professeurs_de_bout_en_bout() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let (adm, role) = dir(&cast);
    let c4b = classe_id(&ent, etab, "4B").await;
    donner_service(&ent, etab, cast.prof.user_id, c4b).await;
    let lea = cast.parent_lea.user_id;
    let tom = cast.parent_tom.user_id;
    // Droits : un parent ne crée pas de rencontre.
    assert!(matches!(
        ent.creer_rencontre(etab, lea, "parent", NouvelleRencontre { titre: "x".into(), classe_id: c4b }).await,
        Err(EntDbError::Forbidden)
    ));
    let r = ent
        .creer_rencontre(etab, adm, role, NouvelleRencontre { titre: "Rencontres 4B".into(), classe_id: c4b })
        .await
        .unwrap();
    assert_eq!(r.statut, "brouillon");
    // Ouvrir sans créneau : refusé.
    assert!(ent.ouvrir_rencontre(etab, adm, role, r.id).await.is_err());
    // Un enseignant sans la classe en service ne peut pas recevoir de créneaux.
    assert!(ent
        .ajouter_creneaux(etab, adm, role, r.id, NouveauxCreneaux { enseignant_user_id: cast.cpe.user_id, debut: demain_9h(), duree_minutes: 10, nombre: 2 })
        .await
        .is_err());
    let n = ent
        .ajouter_creneaux(etab, adm, role, r.id, NouveauxCreneaux { enseignant_user_id: cast.prof.user_id, debut: demain_9h(), duree_minutes: 10, nombre: 3 })
        .await
        .unwrap();
    assert_eq!(n, 3);
    // Un chevauchement pour le même enseignant est refusé.
    assert!(matches!(
        ent.ajouter_creneaux(etab, adm, role, r.id, NouveauxCreneaux { enseignant_user_id: cast.prof.user_id, debut: demain_9h() + Duration::minutes(5), duree_minutes: 10, nombre: 1 }).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Pas de réservation tant que la rencontre n'est pas ouverte.
    let ouvertes_avant = ent.rencontres_famille(etab, lea).await.unwrap();
    assert!(ouvertes_avant.is_empty());
    let r = ent.ouvrir_rencontre(etab, adm, role, r.id).await.unwrap();
    assert_eq!((r.statut.as_str(), r.nb_creneaux), ("ouverte", 3));
    // Les familles de la classe sont prévenues (dans l'ENT), pas celles des autres classes.
    assert!(ent.mes_notifications(etab, lea, 50).await.unwrap().iter().any(|n| n.kind == "rencontre_ouverte"));
    assert!(!ent.mes_notifications(etab, tom, 50).await.unwrap().iter().any(|n| n.kind == "rencontre_ouverte"));
    // Famille de 4B : voit la rencontre, les créneaux de l'enseignant.
    let vues = ent.rencontres_famille(etab, lea).await.unwrap();
    assert_eq!(vues.len(), 1);
    assert!(ent.rencontres_famille(etab, tom).await.unwrap().is_empty(), "famille d'une autre classe");
    let eleve = cast.eleve_lea_id;
    let profs = ent.creneaux_famille(etab, lea, r.id, eleve).await.unwrap();
    let mes_creneaux = &profs.iter().find(|p| p.enseignant_user_id == cast.prof.user_id).unwrap().creneaux;
    assert_eq!(mes_creneaux.len(), 3);
    // Une autre famille ne peut ni consulter ni réserver pour cet enfant.
    assert!(ent.creneaux_famille(etab, tom, r.id, eleve).await.is_err());
    assert!(ent.reserver_creneau(etab, tom, mes_creneaux[0].id, eleve).await.is_err());
    // Réservation.
    let pris = ent.reserver_creneau(etab, lea, mes_creneaux[0].id, eleve).await.unwrap();
    assert_eq!(pris.statut, "reserve");
    assert!(ent.mes_notifications(etab, lea, 50).await.unwrap().iter().any(|n| n.kind == "rdv_pris"));
    assert!(ent.mes_notifications(etab, cast.prof.user_id, 50).await.unwrap().iter().any(|n| n.kind == "rdv_pris"));
    // Idem : un seul rendez-vous par enseignant et par enfant.
    assert!(matches!(ent.reserver_creneau(etab, lea, mes_creneaux[1].id, eleve).await, Err(EntDbError::ConflitDossier(_))));
    // Un créneau pris n'est plus réservable (même par un responsable légitime de la classe).
    sqlx::query("UPDATE ent_eleve_lien SET autorite_parentale = true WHERE etablissement_id = $1")
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
    // Vue de l'enseignant et du planning.
    let siens = ent.mes_creneaux_enseignant(etab, cast.prof.user_id).await.unwrap();
    assert!(siens.iter().any(|l| l.creneau_id == pris.creneau_id && l.statut == "reserve"));
    let planning = ent.planning_rencontre(etab, adm, role, r.id).await.unwrap();
    assert_eq!(planning.iter().filter(|l| l.statut == "reserve").count(), 1);
    // Les autres familles voient ce créneau comme indisponible, sans identité.
    let profs_tom = ent.creneaux_famille(etab, lea, r.id, eleve).await.unwrap();
    let etats: Vec<&str> = profs_tom.iter().flat_map(|p| p.creneaux.iter()).map(|c| c.etat.as_str()).collect();
    assert!(etats.contains(&"reserve_par_moi") && etats.contains(&"libre"));
    // Annulation : seul le titulaire (ou l'organisateur).
    assert!(ent.annuler_reservation(etab, tom, "parent", pris.creneau_id).await.is_err());
    let libre = ent.annuler_reservation(etab, lea, "parent", pris.creneau_id).await.unwrap();
    assert_eq!(libre.statut, "libre");
    // Rappels : idempotents.
    ent.reserver_creneau(etab, lea, mes_creneaux[2].id, eleve).await.unwrap();
    let t = demain_9h() - Duration::hours(2) - Duration::days(0);
    let proche = ent.emettre_rappels_rdv(t, Duration::hours(24), Some(etab)).await.unwrap();
    let hors = ent.emettre_rappels_rdv(Utc::now(), Duration::hours(1), Some(etab)).await.unwrap();
    assert_eq!((proche, hors), (1, 0), "rappel dans les 24 h seulement");
    assert_eq!(ent.emettre_rappels_rdv(t, Duration::hours(24), Some(etab)).await.unwrap(), 0, "un seul rappel");
    assert!(ent.mes_notifications(etab, lea, 50).await.unwrap().iter().any(|n| n.kind == "rdv_rappel"));
    // Clôture : plus de réservation.
    ent.cloturer_rencontre(etab, adm, role, r.id).await.unwrap();
    assert!(ent.reserver_creneau(etab, lea, mes_creneaux[1].id, eleve).await.is_err());
    // Isolation : rien de tout cela n'est visible depuis un autre établissement.
    assert!(ent.planning_rencontre(Uuid::new_v4(), adm, role, r.id).await.is_err());
    assert!(ent.rencontres_famille(Uuid::new_v4(), lea).await.unwrap().is_empty());
    reset(&ent, etab).await;
}

#[tokio::test]
async fn rendez_vous_une_seule_famille_l_emporte() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let (adm, role) = dir(&cast);
    let c4b = classe_id(&ent, etab, "4B").await;
    donner_service(&ent, etab, cast.prof.user_id, c4b).await;
    // Deux familles de 4B : Harry (parent_lea) et Ginny / Ron (parent Weasley).
    let Some(wea) = cast.parent_weasley.clone() else { return eprintln!("skip: pas de parent Weasley") };
    let ginny = cast.eleve_ginny_id.unwrap();
    let r = ent.creer_rencontre(etab, adm, role, NouvelleRencontre { titre: "Concurrence".into(), classe_id: c4b }).await.unwrap();
    ent.ajouter_creneaux(etab, adm, role, r.id, NouveauxCreneaux { enseignant_user_id: cast.prof.user_id, debut: demain_9h(), duree_minutes: 10, nombre: 1 })
        .await
        .unwrap();
    ent.ouvrir_rencontre(etab, adm, role, r.id).await.unwrap();
    let profs = ent.creneaux_famille(etab, cast.parent_lea.user_id, r.id, cast.eleve_lea_id).await.unwrap();
    let k = profs.iter().find(|p| p.enseignant_user_id == cast.prof.user_id).unwrap().creneaux[0].id;
    let (a, b) = tokio::join!(
        ent.reserver_creneau(etab, cast.parent_lea.user_id, k, cast.eleve_lea_id),
        ent.reserver_creneau(etab, wea.user_id, k, ginny)
    );
    assert_eq!(a.is_ok() as u8 + b.is_ok() as u8, 1, "{a:?} / {b:?}");
    let perdant = if a.is_err() { a.unwrap_err() } else { b.unwrap_err() };
    assert!(matches!(perdant, EntDbError::ConflitDossier(_)), "{perdant:?}");
    reset(&ent, etab).await;
}

// ─── 7. Formulaires et consentements ─────────────────────────────────────────

async fn photo_trombi(ent: &PgEnt, etab: Uuid, eleve: Uuid, jour: chrono::NaiveDate) -> Option<String> {
    ent.list_trombi_classe_for_actor(etab, "direction", "4B", jour)
        .await
        .unwrap()
        .into_iter()
        .find(|e| e.eleve_id == eleve)
        .expect("élève au trombinoscope de 4B")
        .photo_key
}

fn consentement(classe: Uuid, texte: &str) -> NouveauFormulaire {
    NouveauFormulaire {
        type_formulaire: "consentement".into(),
        objet: Some("droit_image".into()),
        titre: "Droit à l'image".into(),
        texte: texte.into(),
        audience: "classe".into(),
        cible_ids: vec![classe],
    }
}

fn reponse(eleve: Uuid, decision: &str) -> ReponseFormulaire {
    ReponseFormulaire { eleve_id: eleve, decision: decision.into(), commentaire: None }
}

#[tokio::test]
async fn consentement_droit_a_l_image_applique_au_trombinoscope() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let (adm, role) = dir(&cast);
    let c4b = classe_id(&ent, etab, "4B").await;
    let lea = cast.parent_lea.user_id;
    let harry = cast.eleve_lea_id;
    // Photo posée pour Harry (clé conforme).
    let cle = format!("{etab}/trombi/{harry}.jpg");
    sqlx::query("UPDATE ent_eleve SET photo_key = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(harry)
        .bind(&cle)
        .execute(ent.pool())
        .await
        .unwrap();
    let jour = chrono::Utc::now().date_naive();
    assert_eq!(photo_trombi(&ent, etab, harry, jour).await, Some(cle.clone()), "sans décision : la photo reste affichée");
    // Droits : un parent ne crée pas de formulaire ; la direction oui.
    assert!(matches!(ent.creer_formulaire(etab, lea, "parent", consentement(c4b, "texte")).await, Err(EntDbError::Forbidden)));
    let f1 = ent.creer_formulaire(etab, adm, role, consentement(c4b, "Autorisez-vous la publication de la photo ?")).await.unwrap();
    assert_eq!((f1.version, f1.statut.as_str()), (1, "brouillon"));
    // Pas de réponse sur un brouillon.
    assert!(ent.repondre_formulaire(etab, lea, f1.id, reponse(harry, "refuse")).await.is_err());
    ent.publier_formulaire(etab, adm, role, f1.id).await.unwrap();
    // La famille voit le formulaire pour son enfant, à compléter.
    let mes = ent.formulaires_famille(etab, lea).await.unwrap();
    let m = mes.iter().find(|m| m.formulaire_id == f1.id && m.eleve_id == harry).expect("formulaire visible");
    assert!(m.a_repondre && m.ma_decision.is_none());
    assert!(ent.mes_notifications(etab, lea, 50).await.unwrap().iter().any(|n| n.kind == "formulaire"));
    // Une autre famille ne peut pas répondre pour cet enfant.
    assert!(ent.repondre_formulaire(etab, cast.parent_tom.user_id, f1.id, reponse(harry, "refuse")).await.is_err());
    // Refus : la photo est masquée.
    let etat = ent.repondre_formulaire(etab, lea, f1.id, reponse(harry, "refuse")).await.unwrap();
    assert_eq!(etat.decision, "refuse");
    assert_eq!(photo_trombi(&ent, etab, harry, jour).await, None, "refus du droit à l'image : photo masquée");
    assert!(ent.get_photo_eleve_bytes_for_actor(etab, adm, role, harry).await.unwrap().is_none());
    // Idempotence : répéter le refus n'écrit rien.
    ent.repondre_formulaire(etab, lea, f1.id, reponse(harry, "refuse")).await.unwrap();
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_formulaire_reponse WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(harry)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 1);
    // Un refus n'affecte pas les autres élèves de la classe.
    let autres = ent.list_trombi_classe_for_actor(etab, "direction", "4B", jour).await.unwrap();
    assert!(autres.iter().filter(|e| e.eleve_id != harry).all(|e| e.photo_key.is_none() || e.photo_key.is_some()));
    // Qui consent ? Lecture du seul objet, par classe, réservée aux habilités.
    let liste = ent.consentements_classe_for_actor(etab, adm, role, "droit_image", c4b).await.unwrap();
    assert_eq!(liste.iter().find(|c| c.eleve_id == harry).unwrap().decision, "refuse");
    assert!(liste.iter().any(|c| c.decision == "aucune"));
    assert!(matches!(
        ent.consentements_classe_for_actor(etab, cast.prof.user_id, "prof", "droit_image", c4b).await,
        Err(EntDbError::Forbidden)
    ));
    // Résultats agrégés, sans identité.
    let res = ent.resultats_formulaire(etab, adm, role, f1.id).await.unwrap();
    assert!(res.refuse == 1 && res.cibles >= 1);
    // Retour sur la décision : acceptation, la photo réapparaît (décision datée la plus récente).
    ent.repondre_formulaire(etab, lea, f1.id, reponse(harry, "accepte")).await.unwrap();
    assert_eq!(photo_trombi(&ent, etab, harry, jour).await, Some(cle.clone()));
    // Retrait : masquée de nouveau ; retirer sans acceptation préalable est refusé.
    ent.repondre_formulaire(etab, lea, f1.id, reponse(harry, "retire")).await.unwrap();
    assert_eq!(photo_trombi(&ent, etab, harry, jour).await, None);
    assert!(matches!(
        ent.repondre_formulaire(etab, lea, f1.id, reponse(harry, "retire")).await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // L'historique est conservé (jamais écrasé).
    let (hist,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_formulaire_reponse WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(harry)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(hist, 3);
    // Nouvelle version : la précédente est archivée ; le refus (retiré) reste valable.
    let f2 = ent.creer_formulaire(etab, adm, role, consentement(c4b, "Nouveau texte")).await.unwrap();
    assert_eq!(f2.version, 2);
    ent.publier_formulaire(etab, adm, role, f2.id).await.unwrap();
    assert!(ent.repondre_formulaire(etab, lea, f1.id, reponse(harry, "accepte")).await.is_err(), "v1 archivée");
    assert_eq!(photo_trombi(&ent, etab, harry, jour).await, None, "le refus d'une version précédente reste appliqué");
    ent.repondre_formulaire(etab, lea, f2.id, reponse(harry, "accepte")).await.unwrap();
    assert_eq!(photo_trombi(&ent, etab, harry, jour).await, Some(cle));
    // Événements : sans texte libre.
    let (evts,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type LIKE 'consentement.%'")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(evts >= 4);
    // Isolation : depuis un autre établissement, rien.
    assert!(ent.formulaires_gestion(Uuid::new_v4(), adm, role).await.unwrap().is_empty());
    assert!(ent.formulaires_famille(Uuid::new_v4(), lea).await.unwrap().is_empty());
    sqlx::query("UPDATE ent_eleve SET photo_key = NULL WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(harry)
        .execute(ent.pool())
        .await
        .unwrap();
    reset(&ent, etab).await;
}

#[tokio::test]
async fn eleve_majeur_signe_lui_meme_et_sondage() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let (adm, role) = dir(&cast);
    let c4b = classe_id(&ent, etab, "4B").await;
    let lea = cast.parent_lea.user_id;
    let harry = cast.eleve_lea_id;
    let f = ent.creer_formulaire(etab, adm, role, consentement(c4b, "Photo")).await.unwrap();
    ent.publier_formulaire(etab, adm, role, f.id).await.unwrap();
    ent.repondre_formulaire(etab, lea, f.id, reponse(harry, "refuse")).await.unwrap();
    assert_eq!(ent.eleves_photo_masquee(etab, &[harry]).await.unwrap().len(), 1);
    // L'élève devient majeur : les responsables ne signent plus, leur refus ne compte plus.
    sqlx::query("UPDATE ent_eleve SET date_naissance = current_date - INTERVAL '19 years' WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(harry)
        .execute(ent.pool())
        .await
        .unwrap();
    assert!(ent.repondre_formulaire(etab, lea, f.id, reponse(harry, "accepte")).await.is_err());
    assert!(ent.eleves_photo_masquee(etab, &[harry]).await.unwrap().is_empty(), "parents sans effet sur un majeur");
    // Son compte élève signe pour lui-même.
    let compte: Option<(Uuid,)> = sqlx::query_as("SELECT user_id FROM ent_eleve_compte WHERE etablissement_id = $1 AND eleve_id = $2")
        .bind(etab)
        .bind(harry)
        .fetch_optional(ent.pool())
        .await
        .unwrap();
    if let Some((eleve_user,)) = compte {
        ent.repondre_formulaire(etab, eleve_user, f.id, reponse(harry, "refuse")).await.unwrap();
        assert_eq!(ent.eleves_photo_masquee(etab, &[harry]).await.unwrap().len(), 1);
    }
    sqlx::query("UPDATE ent_eleve SET date_naissance = NULL WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(harry)
        .execute(ent.pool())
        .await
        .unwrap();
    // Sondage : pas d'objet, pas de retrait ; résultat agrégé.
    let s = ent
        .creer_formulaire(
            etab,
            adm,
            role,
            NouveauFormulaire {
                type_formulaire: "sondage".into(),
                objet: None,
                titre: "Horaires de la réunion".into(),
                texte: "18 h vous convient-il ?".into(),
                audience: "classe".into(),
                cible_ids: vec![c4b],
            },
        )
        .await
        .unwrap();
    assert!(s.objet.is_none());
    ent.publier_formulaire(etab, adm, role, s.id).await.unwrap();
    ent.repondre_formulaire(etab, lea, s.id, reponse(harry, "accepte")).await.unwrap();
    assert!(ent.repondre_formulaire(etab, lea, s.id, reponse(harry, "retire")).await.is_err());
    let res = ent.resultats_formulaire(etab, adm, role, s.id).await.unwrap();
    assert_eq!(res.accepte, 1);
    // Objet invalide à la création.
    let mauvais = NouveauFormulaire { objet: Some("Droit Image!".into()), ..consentement(c4b, "x") };
    assert!(ent.creer_formulaire(etab, adm, role, mauvais).await.is_err());
    reset(&ent, etab).await;
}
