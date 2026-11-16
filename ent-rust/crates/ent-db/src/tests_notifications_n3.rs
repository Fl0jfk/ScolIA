//! Socle 8, lot N3 (préférences et temps) : heures calmes à l'heure de l'établissement (Réunion, changements
//! d'heure de Paris), urgences, résumé, budget de bruit, pause, fratrie, reprise de l'ancien réglage. Les
//! instants sont fixés dans chaque test (appels directs au service) : aucun ne dépend de l'heure réelle.
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use crate::notifications::Preferences;
use crate::ntf::catalogue::{ABSENCE_CONSTATEE, ABSENCE_JUSTIFIEE, PASSAGE_INFIRMERIE, RETARD_CONSTATE};
use crate::ntf::contrat::*;
use crate::ntf::reglages::{definir_reglage, lire_reglage, ReglagePersonne};
use crate::ntf::service::{aviser, DemandeAvis};
use crate::ntf::temps::{Calme, ModeResume, Plage};
use crate::tenant_tx::TenantTx;
use crate::tests_notifications_n1::labo;
use chrono::{DateTime, Utc};
use uuid::Uuid;

fn instant(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

async fn fuseau(ent: &PgEnt, etab: Uuid, tz: &str) {
    sqlx::query(
        "INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1, $2)
         ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = $2",
    )
    .bind(etab)
    .bind(tz)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

/// Paramètres de l'établissement : heures calmes (ou aucune si `None`), jours calmes, budget de bruit.
async fn parametres(ent: &PgEnt, etab: Uuid, calme: Option<(&str, &str)>, jours: i16, budget: i16) {
    let (d, f) = calme.unwrap_or(("00:00", "00:00"));
    sqlx::query(
        "INSERT INTO ent_ntf_parametre (etablissement_id, calme_debut, calme_fin, jours_calmes_masque, budget_bruit_u0)
         VALUES ($1, $2::time, $3::time, $4, $5)
         ON CONFLICT (etablissement_id) DO UPDATE SET calme_debut = $2::time, calme_fin = $3::time,
            jours_calmes_masque = $4, budget_bruit_u0 = $5",
    )
    .bind(etab)
    .bind(d)
    .bind(f)
    .bind(jours)
    .bind(budget)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

fn demande(etab: Uuid, nature: &'static NatureAvisDecl, eleve: Uuid, cle: &str, effet: &str, plus: Option<Urgence>) -> DemandeAvis {
    DemandeAvis {
        etablissement_id: etab,
        nature,
        eleve_id: Some(eleve),
        cle: format!("{cle}-{}", Uuid::new_v4().simple()),
        parametres: vec![],
        emetteur_brique: None,
        source_type: "test".into(),
        source_id: None,
        cause_event_id: None,
        effet_le: instant(effet),
        urgence_plus: plus,
        delai_min: None,
    }
}

async fn creer(ent: &PgEnt, d: DemandeAvis) -> Uuid {
    let etab = d.etablissement_id;
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let a = aviser(&mut tx, d).await.expect("avis");
    tx.commit().await.unwrap();
    a.avis_id
}

/// (canal, statut, motif de planification, instant) des remises d'un avis pour un compte.
async fn remises(ent: &PgEnt, avis: Uuid, compte: Uuid) -> Vec<(String, String, String, DateTime<Utc>)> {
    sqlx::query_as(
        "SELECT r.canal, r.statut, r.motif_planification, r.planifiee_pour
           FROM ent_ntf_remise r JOIN ent_ntf_destinataire d ON d.id = r.destinataire_id
          WHERE d.avis_id = $1 AND d.compte_id = $2 ORDER BY r.canal",
    )
    .bind(avis)
    .bind(compte)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap()
}

fn externe(v: &[(String, String, String, DateTime<Utc>)]) -> &(String, String, String, DateTime<Utc>) {
    v.iter().find(|r| r.0 != "inapp").expect("une remise externe")
}

async fn personne_du_compte(ent: &PgEnt, etab: Uuid, compte: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2")
        .bind(etab)
        .bind(compte)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

/// Heures calmes de l'établissement dans son fuseau (Réunion, UTC+4) : U1 de 21 h 30 locale attend 7 h locale ;
/// la même heure en U2 attend aussi (nuit) ; à 14 h locale, tout part ; l'ENT n'attend jamais.
#[tokio::test]
async fn n3_heures_calmes_a_l_heure_de_la_reunion() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    fuseau(&ent, etab, "Indian/Reunion").await;
    parametres(&ent, etab, Some(("20:30", "07:00")), 1 << 6, 5).await;
    // Mercredi 4 novembre 2026, 21 h 30 locale = 17 h 30 UTC.
    let nuit = creer(&ent, demande(etab, &PASSAGE_INFIRMERIE, eleve, "n3-nuit", "2026-11-04T17:30:00Z", None)).await;
    let r = remises(&ent, nuit, parent).await;
    assert_eq!(r.iter().find(|x| x.0 == "inapp").unwrap().3, instant("2026-11-04T17:30:00Z"), "l'ENT n'attend jamais");
    let e = externe(&r);
    assert_eq!((e.2.as_str(), e.3), ("heures_calmes", instant("2026-11-05T03:00:00Z")), "7 h locale = 3 h UTC");
    // U2 (absence, délai 20 min) de nuit : attend aussi la fin de la nuit.
    let u2 = creer(&ent, demande(etab, &ABSENCE_CONSTATEE, eleve, "n3-u2nuit", "2026-11-04T17:30:00Z", None)).await;
    assert_eq!(externe(&remises(&ent, u2, parent).await).3, instant("2026-11-05T03:00:00Z"));
    // 14 h locale : immédiat (après le délai de confirmation pour l'absence).
    let jour = creer(&ent, demande(etab, &PASSAGE_INFIRMERIE, eleve, "n3-jour", "2026-11-04T10:00:00Z", None)).await;
    assert_eq!(externe(&remises(&ent, jour, parent).await).3, instant("2026-11-04T10:00:00Z"));
    let abs = creer(&ent, demande(etab, &ABSENCE_CONSTATEE, eleve, "n3-abs", "2026-11-04T10:00:00Z", None)).await;
    let e = remises(&ent, abs, parent).await;
    assert_eq!(externe(&e).3, instant("2026-11-04T10:20:00Z"), "délai de confirmation, pas d'heure calme");
}

/// Changements d'heure de Paris : la fin des heures calmes (7 h locale) tombe sur 06 h UTC l'hiver et 05 h UTC l'été.
#[tokio::test]
async fn n3_changements_d_heure_de_paris() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    fuseau(&ent, etab, "Europe/Paris").await;
    // Dimanche calme : samedi soir → lundi 7 h locale.
    parametres(&ent, etab, Some(("20:30", "07:00")), 1 << 6, 5).await;
    // Samedi 24 octobre 2026 22 h CEST (20 h UTC) ; retour à l'heure d'hiver le 25 : lundi 26 à 7 h CET = 6 h UTC.
    let a = creer(&ent, demande(etab, &PASSAGE_INFIRMERIE, eleve, "n3-hiver", "2026-10-24T20:00:00Z", None)).await;
    assert_eq!(externe(&remises(&ent, a, parent).await).3, instant("2026-10-26T06:00:00Z"));
    // Samedi 28 mars 2026 22 h CET (21 h UTC) ; passage à l'heure d'été le 29 : lundi 30 à 7 h CEST = 5 h UTC.
    let b = creer(&ent, demande(etab, &PASSAGE_INFIRMERIE, eleve, "n3-ete", "2026-03-28T21:00:00Z", None)).await;
    assert_eq!(externe(&remises(&ent, b, parent).await).3, instant("2026-03-30T05:00:00Z"));
}

/// Trois étages : heures calmes de l'établissement, puis de la personne (U1 attend, U2 non), pause (U1 attend,
/// U2 non, U3 jamais). U3 (hausse d'urgence permise à une nature de santé) part tout de suite en pleine nuit.
#[tokio::test]
async fn n3_etages_urgences_pause_et_u3() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    fuseau(&ent, etab, "Indian/Reunion").await;
    parametres(&ent, etab, Some(("20:30", "07:00")), 1 << 6, 5).await;
    let personne = personne_du_compte(&ent, etab, parent).await;
    // Heures calmes de la personne : 13 h–15 h (locale). Mercredi 14 h locale = 10 h UTC.
    let mut r = ReglagePersonne::default();
    r.calme = Calme { plage: Some(Plage { debut: 13 * 60, fin: 15 * 60 }), jours: 0 };
    let t0 = instant("2026-11-04T09:00:00Z");
    {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        definir_reglage(&mut tx, etab, personne, &r, t0).await.unwrap();
        tx.commit().await.unwrap();
    }
    let u1 = creer(&ent, demande(etab, &PASSAGE_INFIRMERIE, eleve, "n3-p1", "2026-11-04T10:00:00Z", None)).await;
    assert_eq!(externe(&remises(&ent, u1, parent).await).3, instant("2026-11-04T11:00:00Z"), "U1 attend 15 h locale");
    let u2 = creer(&ent, demande(etab, &ABSENCE_CONSTATEE, eleve, "n3-p2", "2026-11-04T10:00:00Z", None)).await;
    assert_eq!(externe(&remises(&ent, u2, parent).await).3, instant("2026-11-04T10:20:00Z"), "U2 ignore les heures de jour de la personne");
    // Pause jusqu'au 9 novembre 12 h UTC.
    r.calme = Calme::default();
    r.pause_jusqua = Some(instant("2026-11-09T12:00:00Z"));
    {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        definir_reglage(&mut tx, etab, personne, &r, t0).await.unwrap();
        tx.commit().await.unwrap();
    }
    let u1 = creer(&ent, demande(etab, &PASSAGE_INFIRMERIE, eleve, "n3-p3", "2026-11-04T10:00:00Z", None)).await;
    // 9 novembre 12 h UTC = 16 h locale (un lundi) : hors nuit, hors jour calme.
    assert_eq!(externe(&remises(&ent, u1, parent).await).3, instant("2026-11-09T12:00:00Z"), "U1 attend la fin de la pause");
    let u2 = creer(&ent, demande(etab, &ABSENCE_CONSTATEE, eleve, "n3-p4", "2026-11-04T10:00:00Z", None)).await;
    assert_eq!(externe(&remises(&ent, u2, parent).await).3, instant("2026-11-04T10:20:00Z"), "la pause ne retient pas U2");
    // U3 : en pleine nuit locale (22 h) et en pause, part tout de suite.
    let u3 = creer(&ent, demande(etab, &PASSAGE_INFIRMERIE, eleve, "n3-p5", "2026-11-04T18:00:00Z", Some(Urgence::U3))).await;
    let e = remises(&ent, u3, parent).await;
    assert_eq!((externe(&e).2.as_str(), externe(&e).3), ("immediat", instant("2026-11-04T18:00:00Z")));
    // Une nature qui n'est pas de santé ne peut pas monter à U3.
    let abs = creer(&ent, demande(etab, &RETARD_CONSTATE, eleve, "n3-p6", "2026-11-04T10:00:00Z", Some(Urgence::U3))).await;
    let a: String = sqlx::query_scalar("SELECT urgence FROM ent_ntf_avis WHERE id = $1").bind(abs).fetch_one(ent.pool_verif()).await.unwrap();
    assert_eq!(a, "u1", "la hausse à U3 est refusée hors briques autorisées : l'urgence reste celle de la nature");
}

/// Résumé à l'heure choisie, et budget de bruit (CN-49) : le sixième U0 du jour passe au résumé.
#[tokio::test]
async fn n3_resume_et_budget_de_bruit() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    fuseau(&ent, etab, "Indian/Reunion").await;
    parametres(&ent, etab, Some(("20:30", "07:00")), 1 << 6, 2).await;
    // Budget de 2 : les deux premiers U0 du jour partent, le troisième va au résumé de 18 h locale (14 h UTC).
    let mut cles = Vec::new();
    for i in 0..3 {
        cles.push(creer(&ent, demande(etab, &ABSENCE_JUSTIFIEE, eleve, &format!("n3-b{i}"), "2026-11-04T05:00:00Z", None)).await);
    }
    let e0 = remises(&ent, cles[0], parent).await;
    let e1 = remises(&ent, cles[1], parent).await;
    assert_eq!(externe(&e0).3, instant("2026-11-04T05:00:00Z"));
    assert_eq!(externe(&e1).3, instant("2026-11-04T05:00:00Z"));
    let e2 = remises(&ent, cles[2], parent).await;
    assert_eq!((externe(&e2).2.as_str(), externe(&e2).3), ("budget_bruit", instant("2026-11-04T14:00:00Z")));
    // Résumé quotidien choisi à 12 h locale (8 h UTC) pour un retard (U1 regroupable) du matin.
    let personne = personne_du_compte(&ent, etab, parent).await;
    let mut r = ReglagePersonne::default();
    r.mode_resume = ModeResume::Quotidien;
    r.heure_resume = 12 * 60;
    {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        definir_reglage(&mut tx, etab, personne, &r, instant("2026-11-04T00:00:00Z")).await.unwrap();
        tx.commit().await.unwrap();
    }
    let retard = creer(&ent, demande(etab, &RETARD_CONSTATE, eleve, "n3-res", "2026-11-05T05:00:00Z", None)).await;
    let e = remises(&ent, retard, parent).await;
    assert_eq!((externe(&e).2.as_str(), externe(&e).3), ("resume_quotidien", instant("2026-11-05T08:00:00Z")));
    // Après l'heure du résumé, c'est le lendemain.
    let tard = creer(&ent, demande(etab, &RETARD_CONSTATE, eleve, "n3-res2", "2026-11-05T09:00:00Z", None)).await;
    assert_eq!(externe(&remises(&ent, tard, parent).await).3, instant("2026-11-06T08:00:00Z"));
    // L'absence constatée (U2) n'est jamais mise au résumé.
    let abs = creer(&ent, demande(etab, &ABSENCE_CONSTATEE, eleve, "n3-res3", "2026-11-05T05:00:00Z", None)).await;
    assert_eq!(externe(&remises(&ent, abs, parent).await).2, "delai_confirmation");
}

/// Fratrie : avec le réglage, un seul avis externe par foyer et demi-journée ; l'ENT garde chaque enfant.
#[tokio::test]
async fn n3_fratrie_un_seul_avis_externe_par_demi_journee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    fuseau(&ent, etab, "Indian/Reunion").await;
    parametres(&ent, etab, None, 0, 5).await;
    // Un second enfant du même parent.
    let frere = ent
        .create_eleve(domain_eleve::CreateEleve {
            etablissement_id: etab,
            nom: format!("Fratrie{}", Uuid::new_v4().simple()),
            prenom: "Tom".into(),
            ine: None,
            sexe: Some("M".into()),
            date_naissance: chrono::NaiveDate::from_ymd_opt(2012, 5, 2),
            numero_badge: None,
            redoublant: false,
            classe: None,
            regime: None,
            mef: None,
            site: None,
            date_entree: Some(domain_eleve::horloge::jour_paris() - chrono::Duration::days(50)),
            date_sortie: None,
            motif_sortie: None,
            actor_user_id: None,
        })
        .await
        .expect("frère")
        .id;
    let personne = personne_du_compte(&ent, etab, parent).await;
    sqlx::query("INSERT INTO ent_eleve_lien (id, etablissement_id, eleve_id, personne_id, lien) VALUES ($1, $2, $3, $4, 'pere')")
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(frere)
        .bind(personne)
        .execute(ent.pool_verif())
        .await
        .expect("lien du frère");
    let mut r = ReglagePersonne::default();
    r.regrouper_fratrie = true;
    {
        let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
        definir_reglage(&mut tx, etab, personne, &r, instant("2026-11-04T00:00:00Z")).await.unwrap();
        tx.commit().await.unwrap();
    }
    let a1 = creer(&ent, demande(etab, &ABSENCE_CONSTATEE, eleve, "n3-f1", "2026-11-04T04:00:00Z", None)).await;
    let a2 = creer(&ent, demande(etab, &ABSENCE_CONSTATEE, frere, "n3-f2", "2026-11-04T04:05:00Z", None)).await;
    let r1 = remises(&ent, a1, parent).await;
    let r2 = remises(&ent, a2, parent).await;
    assert_eq!(externe(&r1).1, "planifiee");
    let e2 = r2.iter().find(|x| x.0 != "inapp").unwrap();
    assert_eq!(e2.1, "ignoree", "{r2:?}");
    let motif: Option<String> = sqlx::query_scalar(
        "SELECT r.motif FROM ent_ntf_remise r JOIN ent_ntf_destinataire d ON d.id = r.destinataire_id
          WHERE d.avis_id = $1 AND d.compte_id = $2 AND r.canal <> 'inapp'",
    )
    .bind(a2)
    .bind(parent)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(motif.as_deref(), Some("regroupe_fratrie"));
    assert_eq!(r2.iter().find(|x| x.0 == "inapp").unwrap().1, "planifiee", "l'ENT garde chaque enfant");
}

/// Reprise sans perte de `ent_preference_notification` (migration 3803 rejouée) et reflet de l'ancien réglage.
#[tokio::test]
async fn n3_reprise_et_reflet_de_l_ancien_reglage() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, parent) = (cast.etablissement_id, cast.parent_lea.user_id);
    let personne = personne_du_compte(&ent, etab, parent).await;
    sqlx::query(
        "INSERT INTO ent_preference_notification
            (etablissement_id, user_id, canal_email, canal_sms, canal_push, resume, calme_debut, calme_fin)
         VALUES ($1, $2, false, true, true, 'quotidien', 1260, 420)",
    )
    .bind(etab)
    .bind(parent)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    // Rejeu de la migration (rejouable) : la reprise copie la ligne de l'ancien moteur.
    let sql = include_str!("../../../migrations/3803_ntf_preferences.sql");
    sqlx::raw_sql(sql).execute(ent.pool_verif()).await.expect("migration rejouée");
    let (mode, heure, cd, cf): (String, String, String, String) = sqlx::query_as(
        "SELECT resume_mode, to_char(resume_heure, 'HH24:MI'), to_char(calme_debut, 'HH24:MI'), to_char(calme_fin, 'HH24:MI')
           FROM ent_ntf_reglage_personne WHERE etablissement_id = $1 AND personne_id = $2",
    )
    .bind(etab)
    .bind(personne)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((mode.as_str(), heure.as_str(), cd.as_str(), cf.as_str()), ("quotidien", "07:30", "21:00", "07:00"));
    let (n, push, email, sms): (i64, bool, bool, bool) = sqlx::query_as(
        "SELECT count(*), bool_and(push), bool_and(email), bool_and(sms) FROM ent_ntf_preference
          WHERE etablissement_id = $1 AND personne_id = $2",
    )
    .bind(etab)
    .bind(personne)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((n, push, email, sms), (10, true, false, true), "dix catégories, canaux repris");
    // Reflet : une écriture par l'API de l'ancien moteur met à jour les nouvelles tables.
    ent.definir_preferences_notification(
        etab,
        parent,
        Preferences { email: true, sms: false, push: false, resume: "immediat".into(), calme_debut: Some(22 * 60), calme_fin: Some(6 * 60) },
    )
    .await
    .expect("préférences");
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let r = lire_reglage(&mut tx, etab, personne).await.unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(r.mode_resume, ModeResume::Immediat);
    assert_eq!(r.calme.plage, Some(Plage { debut: 22 * 60, fin: 6 * 60 }));
    let (push, email): (bool, bool) = sqlx::query_as(
        "SELECT bool_and(push), bool_and(email) FROM ent_ntf_preference WHERE etablissement_id = $1 AND personne_id = $2",
    )
    .bind(etab)
    .bind(personne)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((push, email), (false, true));
}
