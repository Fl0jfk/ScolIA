//! Socle 8, lot N1 (brique et contrat) : bascule par nature, délai de confirmation, annulation par événement,
//! droit relu à la remise, idempotence, refus du service. Chaque test a son propre établissement (semis du labo
//! neuf) et fixe ses instants à partir de ceux des événements : aucun ne dépend de l'heure réelle.
//! Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use crate::ntf::catalogue::{ABSENCE_CONSTATEE, PASSAGE_INFIRMERIE};
use crate::ntf::consommateur::CONSOMMATEUR_AVIS;
use crate::ntf::contrat::*;
use crate::ntf::expedition::expedier_avis_dus;
use crate::ntf::service::{annuler_avis, aviser, etat, Annulation, DemandeAvis, RefusAvis};
use crate::outbox::{ConfigDistribution, Publier};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use ent_events::{types, AggregateKind, MetierEvent, MetierEventDraft};
use serde_json::json;
use uuid::Uuid;

pub(crate) async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    ent.install_mail_dry_run();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    Some((ent, cast))
}

pub(crate) fn config(etab: Uuid) -> ConfigDistribution {
    ConfigDistribution {
        taille_lot: 100,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    }
}

pub(crate) async fn distribuer(ent: &PgEnt, etab: Uuid) {
    ent.distributeur(config(etab)).distribuer_tout().await.unwrap();
}

pub(crate) async fn publier(ent: &PgEnt, d: MetierEventDraft) -> MetierEvent {
    let mut tx = ent.pool().begin().await.unwrap();
    let e = tx.publier(d).await.unwrap();
    tx.commit().await.unwrap();
    e
}

pub(crate) fn evenement(etab: Uuid, type_name: &str, agregat: AggregateKind, eleve: Uuid, payload: serde_json::Value) -> MetierEventDraft {
    MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.into(),
        aggregate: agregat,
        aggregate_id: Uuid::new_v4(),
        eleve_id: Some(eleve),
        payload,
        actor_user_id: None,
    }
}

pub(crate) fn absence_appel(etab: Uuid, eleve: Uuid) -> MetierEventDraft {
    evenement(etab, types::ELEVE_ABSENCE_DECLARED, AggregateKind::Absence, eleve, json!({"debut": "2026-11-05", "source": "appel"}))
}

pub(crate) async fn basculer(ent: &PgEnt, etab: Uuid, nature: &str) {
    sqlx::query(
        "INSERT INTO ent_ntf_nature_etab (etablissement_id, nature, via_brique) VALUES ($1, $2, true)
         ON CONFLICT (etablissement_id, nature) DO UPDATE SET via_brique = true",
    )
    .bind(etab)
    .bind(nature)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    // Déterminisme : aucune heure calme (deux heures égales = pas de plage), aucun jour calme, quelle que soit
    // l'heure réelle d'exécution du test. Les tests du lot N3 posent leurs propres paramètres.
    sqlx::query(
        "INSERT INTO ent_ntf_parametre (etablissement_id, calme_debut, calme_fin, jours_calmes_masque)
         VALUES ($1, TIME '00:00', TIME '00:00', 0) ON CONFLICT (etablissement_id) DO NOTHING",
    )
    .bind(etab)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

pub(crate) async fn compter(ent: &PgEnt, sql: &str, etab: Uuid, id: Uuid) -> i64 {
    sqlx::query_scalar(sql).bind(etab).bind(id).fetch_one(ent.pool_verif()).await.unwrap()
}

pub(crate) async fn notifications_legacy(ent: &PgEnt, etab: Uuid, user: Uuid, kind: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = $3",
    )
    .bind(etab)
    .bind(user)
    .bind(kind)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap()
}

#[derive(sqlx::FromRow)]
pub(crate) struct LigneAvis {
    pub(crate) id: Uuid,
    pub(crate) statut: String,
    pub(crate) effet_le: DateTime<Utc>,
    pub(crate) confirme_apres: DateTime<Utc>,
    pub(crate) annule_motif: Option<String>,
}

pub(crate) async fn avis_de(ent: &PgEnt, etab: Uuid, cause: Uuid) -> Vec<LigneAvis> {
    sqlx::query_as::<_, LigneAvis>(
        "SELECT id, statut, effet_le, confirme_apres, annule_motif FROM ent_ntf_avis
          WHERE etablissement_id = $1 AND cause_event_id = $2 ORDER BY created_at",
    )
    .bind(etab)
    .bind(cause)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap()
}

/// Absence constatée, bascule faite : l'ancien chemin se tait, l'avis attend 20 minutes, puis l'ENT est remis
/// une seule fois (rejeu compris).
#[tokio::test]
async fn n1_absence_de_bout_en_bout() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let evt = publier(&ent, absence_appel(etab, eleve)).await;
    distribuer(&ent, etab).await;
    // L'ancien chemin se tait pour cette nature et cet établissement.
    assert_eq!(notifications_legacy(&ent, etab, parent, "absence_constatee").await, 0, "ancien chemin non silencieux");
    let avis = avis_de(&ent, etab, evt.id).await;
    assert_eq!(avis.len(), 1, "un avis par événement");
    let a = &avis[0];
    assert_eq!(a.statut, "planifie");
    assert_eq!(a.confirme_apres - a.effet_le, Duration::minutes(20), "délai de confirmation de la nature (CN-01)");
    let dest: i64 = compter(
        &ent,
        "SELECT count(*) FROM ent_ntf_destinataire WHERE etablissement_id = $1 AND avis_id = $2 AND retenu AND compte_id IS NOT NULL",
        etab,
        a.id,
    )
    .await;
    assert!(dest >= 1, "le responsable de l'élève est destinataire");
    let remises: i64 = compter(
        &ent,
        "SELECT count(*) FROM ent_ntf_remise r JOIN ent_ntf_destinataire d ON d.id = r.destinataire_id
          WHERE r.etablissement_id = $1 AND d.avis_id = $2 AND r.statut = 'planifiee'",
        etab,
        a.id,
    )
    .await;
    assert_eq!(remises, dest * 2, "ENT et push planifiés par destinataire");

    // Avant la fin du délai : rien ne part.
    let b = expedier_avis_dus(ent.pool(), etab, a.effet_le + Duration::minutes(19)).await.unwrap();
    assert_eq!(b.avis, 0);
    assert_eq!(notifications_legacy(&ent, etab, parent, "absence_constatee").await, 0);
    // Après : l'ENT est remis, le push passe à l'ancien moteur, l'avis est terminé.
    let apres = a.effet_le + Duration::minutes(21);
    let b = expedier_avis_dus(ent.pool(), etab, apres).await.unwrap();
    assert_eq!(b.avis, 1);
    assert_eq!(b.remises_inapp as i64, dest);
    assert_eq!(notifications_legacy(&ent, etab, parent, "absence_constatee").await, 1);
    let titre: String = sqlx::query_scalar(
        "SELECT titre FROM ent_notification WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND kind = 'absence_constatee'",
    )
    .bind(etab)
    .bind(parent)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(titre.starts_with("Absence de "), "{titre}");
    let (statut_avis, statut_dest): (String, String) = sqlx::query_as(
        "SELECT a.statut, d.statut FROM ent_ntf_avis a JOIN ent_ntf_destinataire d ON d.avis_id = a.id
          WHERE a.etablissement_id = $1 AND a.id = $2 AND d.compte_id = $3",
    )
    .bind(etab)
    .bind(a.id)
    .bind(parent)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((statut_avis.as_str(), statut_dest.as_str()), ("termine", "remis"));
    // Une seconde expédition ne remet rien de plus.
    let b = expedier_avis_dus(ent.pool(), etab, apres + Duration::minutes(5)).await.unwrap();
    assert_eq!(b.avis, 0);
    // Rejeu du consommateur : toujours un seul avis, une seule notification.
    let d = ent.distributeur(config(etab));
    d.rejouer_consommateur(CONSOMMATEUR_AVIS, etab, 0).await.unwrap();
    d.distribuer_tout().await.unwrap();
    assert_eq!(avis_de(&ent, etab, evt.id).await.len(), 1, "idempotence du rejeu");
    expedier_avis_dus(ent.pool(), etab, apres + Duration::minutes(10)).await.unwrap();
    assert_eq!(notifications_legacy(&ent, etab, parent, "absence_constatee").await, 1);
}

/// Sans bascule : l'ancien chemin prévient seul, la brique ne crée aucun avis.
#[tokio::test]
async fn n1_sans_bascule_l_ancien_chemin_seul() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    let evt = publier(&ent, absence_appel(etab, eleve)).await;
    distribuer(&ent, etab).await;
    assert_eq!(notifications_legacy(&ent, etab, parent, "absence_constatee").await, 1);
    assert!(avis_de(&ent, etab, evt.id).await.is_empty());
}

/// Un retard enregistré dans le délai de confirmation annule l'avis d'absence : rien n'est parti (CN-01).
#[tokio::test]
async fn n1_retard_dans_le_delai_annule_l_absence() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let evt = publier(&ent, absence_appel(etab, eleve)).await;
    distribuer(&ent, etab).await;
    // Un retard un autre jour ne touche pas l'avis.
    publier(&ent, evenement(etab, types::RETARD_DECLARE, AggregateKind::Eleve, eleve, json!({"jour": "2026-11-06"}))).await;
    distribuer(&ent, etab).await;
    assert_eq!(avis_de(&ent, etab, evt.id).await[0].statut, "planifie", "retard d'un autre jour");
    // Le retard du même jour l'annule.
    publier(&ent, evenement(etab, types::RETARD_DECLARE, AggregateKind::Eleve, eleve, json!({"jour": "2026-11-05"}))).await;
    distribuer(&ent, etab).await;
    let a = &avis_de(&ent, etab, evt.id).await[0];
    assert_eq!((a.statut.as_str(), a.annule_motif.as_deref()), ("annule", Some("annulee_par_evenement")));
    let restantes: i64 = compter(
        &ent,
        "SELECT count(*) FROM ent_ntf_remise r JOIN ent_ntf_destinataire d ON d.id = r.destinataire_id
          WHERE r.etablissement_id = $1 AND d.avis_id = $2 AND r.statut <> 'annulee'",
        etab,
        a.id,
    )
    .await;
    assert_eq!(restantes, 0);
    expedier_avis_dus(ent.pool(), etab, a.effet_le + Duration::hours(2)).await.unwrap();
    assert_eq!(notifications_legacy(&ent, etab, parent, "absence_constatee").await, 0, "aucune absence notifiée");
}

/// Le droit du lien est relu à la remise : retiré entre l'avis et la remise, rien ne part.
#[tokio::test]
async fn n1_droit_retire_avant_la_remise() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let evt = publier(&ent, absence_appel(etab, eleve)).await;
    distribuer(&ent, etab).await;
    let a = &avis_de(&ent, etab, evt.id).await[0];
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_vs = false WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab)
        .bind(eleve)
        .bind(parent)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let b = expedier_avis_dus(ent.pool(), etab, a.confirme_apres + Duration::minutes(1)).await.unwrap();
    assert!(b.annulees >= 1);
    assert_eq!(notifications_legacy(&ent, etab, parent, "absence_constatee").await, 0);
    let (st, motif): (String, Option<String>) = sqlx::query_as(
        "SELECT r.statut, r.motif FROM ent_ntf_remise r JOIN ent_ntf_destinataire d ON d.id = r.destinataire_id
          WHERE r.etablissement_id = $1 AND d.avis_id = $2 AND d.compte_id = $3 AND r.canal = 'inapp'",
    )
    .bind(etab)
    .bind(a.id)
    .bind(parent)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!((st.as_str(), motif.as_deref()), ("annulee", Some("droit_retire")));
}

/// Avis d'infirmerie (droit = lien) : un responsable sans `peut_voir_vs` est destinataire, sans délai.
#[tokio::test]
async fn n1_infirmerie_droit_lien_sans_delai() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    basculer(&ent, etab, PASSAGE_INFIRMERIE.code).await;
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_vs = false WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab)
        .bind(eleve)
        .bind(parent)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let evt = publier(
        &ent,
        evenement(etab, "infirmerie.famille_informee", AggregateKind::Eleve, eleve, json!({"jour": "2026-11-05"})),
    )
    .await;
    distribuer(&ent, etab).await;
    let a = &avis_de(&ent, etab, evt.id).await[0];
    assert_eq!(a.confirme_apres, a.effet_le, "aucun délai de confirmation");
    expedier_avis_dus(ent.pool(), etab, a.effet_le).await.unwrap();
    assert_eq!(notifications_legacy(&ent, etab, parent, "infirmerie_passage").await, 1);
}

/// Refus du service : brique étrangère, nature désactivée, paramètre trop long, sujet manquant ; idempotence ;
/// état ; annulation avant et après la remise.
#[tokio::test]
async fn n1_service_refus_idempotence_etat_et_annulation() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve, parent) = (cast.etablissement_id, cast.eleve_lea_id, cast.parent_lea.user_id);
    let instant = DateTime::parse_from_rfc3339("2026-11-05T09:00:00Z").unwrap().with_timezone(&Utc);
    let demande = |cle: &str, param: Vec<Param>, brique: Option<&'static str>, eleve: Option<Uuid>| DemandeAvis {
        etablissement_id: etab,
        nature: &ABSENCE_CONSTATEE,
        eleve_id: eleve,
        cle: cle.into(),
        parametres: param,
        emetteur_brique: brique,
        source_type: "test".into(),
        source_id: None,
        cause_event_id: None,
        effet_le: instant,
        urgence_plus: None,
    };
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert!(matches!(
        aviser(&mut tx, demande("n1-a", vec![], Some("finances"), Some(eleve))).await,
        Err(RefusAvis::BriqueEtrangere)
    ));
    assert!(matches!(aviser(&mut tx, demande("n1-b", vec![], None, None)).await, Err(RefusAvis::SujetManquant)));
    let trop_long = ValeurParam::Texte("x".repeat(PARAM_TEXTE_MAX + 1));
    assert!(matches!(
        aviser(&mut tx, demande("n1-c", vec![("motif", trop_long)], None, Some(eleve))).await,
        Err(RefusAvis::ParametreTropLong("motif"))
    ));
    tx.rollback().await.unwrap();

    let cle = format!("n1-service-{}", Uuid::new_v4().simple());
    let jour = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let a = aviser(&mut tx, demande(&cle, vec![("jour", ValeurParam::Jour(jour))], None, Some(eleve))).await.unwrap();
    assert!(a.cree && !a.retenus.is_empty());
    let b = aviser(&mut tx, demande(&cle, vec![("jour", ValeurParam::Jour(jour))], None, Some(eleve))).await.unwrap();
    assert!(!b.cree && b.avis_id == a.avis_id, "même clé : même avis");
    let e = etat(&mut tx, etab, a.avis_id).await.unwrap().unwrap();
    assert_eq!(e.statut, "planifie");
    assert!(e.destinataires.iter().any(|d| d.compte_id == Some(parent) && d.statut == "a_remettre"));
    // Annulation avant remise : rien n'est parti.
    assert_eq!(annuler_avis(&mut tx, etab, a.avis_id, "erreur_de_saisie", instant).await.unwrap(), Annulation::Annule);
    assert_eq!(
        annuler_avis(&mut tx, etab, a.avis_id, "erreur_de_saisie", instant).await.unwrap(),
        Annulation::Introuvable,
        "déjà annulé"
    );
    tx.commit().await.unwrap();

    // Nature désactivée par l'établissement : refusée.
    sqlx::query(
        "INSERT INTO ent_ntf_nature_etab (etablissement_id, nature, desactivee) VALUES ($1, $2, true)
         ON CONFLICT (etablissement_id, nature) DO UPDATE SET desactivee = true",
    )
    .bind(etab)
    .bind(ABSENCE_CONSTATEE.code)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert!(matches!(
        aviser(&mut tx, demande("n1-d", vec![], None, Some(eleve))).await,
        Err(RefusAvis::NatureDesactivee)
    ));
    tx.rollback().await.unwrap();
}

/// Un avis déjà remis ne s'annule pas : la correction est due (traitée au lot N8).
#[tokio::test]
async fn n1_avis_remis_correction_due() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, eleve) = (cast.etablissement_id, cast.eleve_lea_id);
    basculer(&ent, etab, ABSENCE_CONSTATEE.code).await;
    let evt = publier(&ent, absence_appel(etab, eleve)).await;
    distribuer(&ent, etab).await;
    let a = &avis_de(&ent, etab, evt.id).await[0];
    expedier_avis_dus(ent.pool(), etab, a.confirme_apres).await.unwrap();
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    assert_eq!(
        annuler_avis(&mut tx, etab, a.id, "erreur", a.confirme_apres).await.unwrap(),
        Annulation::DejaRemis { correction_due: true }
    );
    tx.rollback().await.unwrap();
}
