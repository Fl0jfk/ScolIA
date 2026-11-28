//! Socle 27, **palier G5, lot 1** : registre des violations (déclaration, horloge des 72 h, qualification, grille,
//! avis du DPO, décision du risque, notification CNIL, non-notification, clôture, alertes, port système).
//! Base exigée sous `CI` ou `ENT_EXIGER_BASE` (RG-D03).
//!
//! Aucun test ne dépend de l'heure à laquelle il tourne (règle 33) : les instants comparés sont fixés par le test
//! (`maintenant` passé aux variantes `_a`, prise de connaissance antérieure saisie par un gestionnaire) ; les seuls
//! instants lus sont ceux que le service vient d'écrire, et les assertions portent sur des écarts.

use super::*;
use crate::droits_repo::{Droit, RoleSaisie};
use crate::violations::{
    echeance_cnil, Declaration, FiltreRegistre, Qualification, MERCI_DECLARANT,
};
use chrono::{DateTime, Duration, SubsecRound, Utc};
use domain_eleve::DomainError;
use ent_auth::{Action, Portee};
use uuid::Uuid;

const DIR: &str = "direction";
const ADF: &str = "administratif";
const PROF: &str = "prof";

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| {
        e.install_brain_p0();
    })
    .await;
    Some((ent, cast))
}

fn decl(intitule: &str) -> Declaration<'_> {
    Declaration {
        intitule,
        description: Some("clé USB perdue dans le bus, contenait des listes"),
        constatee_le: None,
        incident_securite: false,
    }
}

/// Instant d'ancrage des scénarios, à la microseconde (précision de PostgreSQL) : sans cela, une échéance relue en
/// base tombe quelques nanosecondes avant l'heure pleine et les décomptes d'heures glissent d'une unité.
fn instant() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(6)
}

/// Une violation saisie par la direction, connue `il_y_a` avant `maintenant`.
async fn violation_connue_il_y_a(
    ent: &PgEnt,
    cast: &LaboCast,
    maintenant: DateTime<Utc>,
    il_y_a: Duration,
) -> Uuid {
    ent.violations_saisir_a(
        cast.etablissement_id,
        cast.direction.user_id,
        DIR,
        decl("liste de classe envoyée à toutes les familles du niveau"),
        Some(maintenant - il_y_a),
        maintenant,
    )
    .await
    .expect("saisie")
    .id
}

fn qualif<'a>(cats: &'a [&'a str], mineurs: bool) -> Qualification<'a> {
    Qualification {
        nature_incident: "violation",
        confidentialite: true,
        integrite: false,
        disponibilite: false,
        categories: cats,
        personnes_nombre: Some(120),
        personnes_estime: false,
        personnes_mineurs: mineurs,
        consequences: Some("noms et classes connus de familles d'autres classes"),
    }
}

async fn compter(ent: &PgEnt, sql: &str, id: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

// ─────────────────────────────── Déclaration et alerte ───────────────────────────────

/// RGP-240, RGP-242, CL27-45 : un professeur déclare en un geste ; il reçoit la référence et un merci, rien d'autre ;
/// la direction est prévenue tout de suite, sur tous ses canaux joignables, **sans attendre la fin d'une plage calme**
/// ; le déclarant ne reçoit pas d'alerte. Une famille ne déclare pas (elle le signale à l'établissement).
#[tokio::test]
async fn g5_declaration_en_un_geste_et_alerte_immediate() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let maintenant = instant();
    let m = ent
        .violations_declarer_a(
            etab,
            cast.prof.user_id,
            PROF,
            decl("clé USB perdue"),
            maintenant,
        )
        .await
        .expect("déclaration");
    assert!(m.reference.starts_with("V-"), "{}", m.reference);
    assert_eq!(m.avancement, "recue");
    assert_eq!(m.message, MERCI_DECLARANT);
    assert_eq!(
        m.connue_le.timestamp(),
        maintenant.timestamp(),
        "prise de connaissance = déclaration"
    );
    // Alerte : la direction la reçoit, le déclarant non.
    let direction = compter(
        &ent,
        &format!(
            "SELECT count(*) FROM ent_notification WHERE kind = 'rgpd.violation_declaree' AND destinataire_user_id = '{}'
               AND corps LIKE '%' || (SELECT 'V-' || annee || '-' || lpad(numero::text, 3, '0') FROM rgp_violation WHERE id = $1) || '%'",
            cast.direction.user_id
        ),
        m.id,
    )
    .await;
    assert_eq!(direction, 1, "la direction est prévenue une fois");
    let declarant = compter(
        &ent,
        &format!(
            "SELECT count(*) FROM ent_notification n JOIN rgp_violation v ON v.id = $1
              WHERE n.kind = 'rgpd.violation_declaree' AND n.destinataire_user_id = '{}' AND n.created_at >= v.cree_le",
            cast.prof.user_id
        ),
        m.id,
    )
    .await;
    assert_eq!(
        declarant, 0,
        "le déclarant n'est pas alerté de sa propre déclaration"
    );
    // Hors plage calme : chaque envoi externe planifié l'est pour l'instant de la déclaration, jamais plus tard.
    let tard = compter(
        &ent,
        "SELECT count(*) FROM ent_notification_envoi WHERE source_id = $1 AND kind = 'rgpd.violation_declaree'
           AND planifie_pour > (SELECT cree_le FROM rgp_violation WHERE id = $1) + interval '1 minute'",
        m.id,
    )
    .await;
    assert_eq!(tard, 0, "aucun envoi d'alerte différé par une plage calme");
    // La chronologie dit la déclaration et l'alerte, sans nom.
    let v = ent
        .violations_fiche_a(etab, cast.direction.user_id, DIR, m.id, maintenant)
        .await
        .unwrap();
    let etapes: Vec<&str> = v.chronologie.iter().map(|e| e.etape.as_str()).collect();
    assert_eq!(&etapes[..2], &["declaree", "alerte_envoyee"], "{etapes:?}");
    assert_eq!(v.declaree_par_compte_id, Some(cast.prof.user_id));
    // Une famille ne déclare pas ; un texte vide est refusé.
    assert!(matches!(
        ent.violations_declarer(etab, cast.parent_lea.user_id, "parent", decl("je signale"))
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.violations_declarer(etab, cast.prof.user_id, PROF, decl("  "))
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
}

/// R-RG32 : le déclarant suit l'avancement de SES déclarations ; il ne lit ni le registre ni la fiche.
#[tokio::test]
async fn g5_le_declarant_ne_voit_que_l_avancement() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let m = ent
        .violations_declarer(etab, cast.prof.user_id, PROF, decl("écran laissé ouvert"))
        .await
        .unwrap();
    let mes = ent
        .violations_mes_declarations(etab, cast.prof.user_id, PROF)
        .await
        .unwrap();
    assert!(mes.iter().any(|d| d.id == m.id));
    let cpe = ent
        .violations_mes_declarations(etab, cast.cpe.user_id, "cpe")
        .await
        .unwrap();
    assert!(
        !cpe.iter().any(|d| d.id == m.id),
        "un autre personnel ne voit pas la déclaration"
    );
    assert!(matches!(
        ent.violations_fiche(etab, cast.prof.user_id, PROF, m.id)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.violations_registre(etab, cast.prof.user_id, PROF, FiltreRegistre::default())
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Le secrétariat ne gère pas les violations par défaut.
    assert!(matches!(
        ent.violations_fiche(etab, cast.administratif.user_id, ADF, m.id)
            .await,
        Err(EntDbError::Forbidden)
    ));
}

/// Horloge : échéance = prise de connaissance + 72 h exactement ; numéros consécutifs sans trou ; une saisie refusée
/// ne consomme pas de numéro ; une prise de connaissance future ou de plus d'un an est refusée.
#[tokio::test]
async fn g5_horloge_72_h_et_numeros() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let maintenant = instant();
    let a = violation_connue_il_y_a(&ent, &cast, maintenant, Duration::hours(5)).await;
    let va = ent
        .violations_fiche_a(etab, dir, DIR, a, maintenant)
        .await
        .unwrap();
    assert_eq!(va.echeance_cnil_le, echeance_cnil(va.connue_le));
    assert_eq!((va.echeance_cnil_le - va.connue_le).num_minutes(), 72 * 60);
    assert_eq!(va.heures_restantes, 67);
    assert!(!va.en_retard);
    for (connue, quoi) in [
        (maintenant + Duration::minutes(5), "future"),
        (maintenant - Duration::days(400), "trop ancienne"),
    ] {
        assert!(
            matches!(
                ent.violations_saisir_a(etab, dir, DIR, decl("refusée"), Some(connue), maintenant)
                    .await,
                Err(EntDbError::SaisieInvalide(_))
            ),
            "prise de connaissance {quoi}"
        );
    }
    let b = violation_connue_il_y_a(&ent, &cast, maintenant, Duration::hours(1)).await;
    let vb = ent
        .violations_fiche_a(etab, dir, DIR, b, maintenant)
        .await
        .unwrap();
    // Numéros croissants dans l'année de l'établissement (d'autres tests déclarent en parallèle dans le même
    // établissement : on n'exige pas la contiguïté ; autour du 1er janvier, les deux années diffèrent).
    let (annee_a, na) = va.reference[2..].split_once('-').unwrap();
    let (annee_b, nb) = vb.reference[2..].split_once('-').unwrap();
    if annee_a == annee_b {
        assert!(
            nb.parse::<i32>().unwrap() > na.parse::<i32>().unwrap(),
            "{} puis {}",
            va.reference,
            vb.reference
        );
    }
    // Connue il y a 80 h, non notifiée : en retard.
    let c = violation_connue_il_y_a(&ent, &cast, maintenant, Duration::hours(80)).await;
    let vc = ent
        .violations_fiche_a(etab, dir, DIR, c, maintenant)
        .await
        .unwrap();
    assert!(vc.en_retard);
    assert_eq!(vc.heures_restantes, -8);
    let a_notifier = ent
        .violations_registre_a(
            etab,
            dir,
            DIR,
            FiltreRegistre {
                a_notifier: true,
                ..Default::default()
            },
            maintenant,
        )
        .await
        .unwrap();
    assert!(a_notifier.iter().any(|l| l.id == c && l.en_retard));
}

// ─────────────────────────── Qualification, grille, décision ───────────────────────────

/// RGP-243, RGP-245, R-RG34 : la grille propose, la direction décide après l'avis du DPO ; la décision peut s'écarter
/// de la proposition avec un motif ; un gestionnaire sans `violations.decider` ne décide pas.
#[tokio::test]
async fn g5_qualification_grille_puis_decision_apres_avis_dpo() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let maintenant = instant();
    let id = violation_connue_il_y_a(&ent, &cast, maintenant, Duration::hours(2)).await;
    // Catégorie inconnue : refusée ; atteinte non qualifiée : refusée.
    assert!(matches!(
        ent.violations_qualifier(etab, dir, DIR, id, qualif(&["inconnue.x"], false))
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let mut sans = qualif(&["scol.registre"], false);
    sans.confidentialite = false;
    assert!(matches!(
        ent.violations_qualifier(etab, dir, DIR, id, sans).await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Décider avant de qualifier : refusé.
    assert!(matches!(
        ent.violations_decider_risque(etab, dir, DIR, id, "faible", "liste nom et classe")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // CL27-41 : nom + classe → faible proposé.
    let v = ent
        .violations_qualifier(etab, dir, DIR, id, qualif(&["scol.registre"], false))
        .await
        .unwrap();
    assert_eq!(v.etat, "qualifiee");
    assert_eq!(v.risque_propose.as_deref(), Some("faible"));
    assert!(!v.raisons_proposition.is_empty());
    assert_eq!(v.categories, vec!["scol.registre".to_string()]);
    // Sans avis du DPO : refusé.
    assert!(matches!(
        ent.violations_decider_risque(etab, dir, DIR, id, "faible", "liste nom et classe")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.violations_avis_dpo(
        etab,
        dir,
        DIR,
        id,
        "DPO consulté : risque faible, pas de donnée sensible",
    )
    .await
    .unwrap();
    // Un gestionnaire DPO (rôle D-017 : `violations.gerer` seul) qualifie mais ne décide pas.
    let role_dpo = ent
        .creer_role_for_actor(
            etab,
            cast.admin.user_id,
            "admin",
            RoleSaisie {
                nom: format!("DPO {}", Uuid::new_v4()),
                description: Some("délégué à la protection des données".into()),
                modele: None,
                droits: vec![Droit {
                    action: Action::ViolationsGerer,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .expect("rôle DPO")
        .id;
    ent.attribuer_role_for_actor(
        etab,
        cast.admin.user_id,
        "admin",
        role_dpo,
        cast.administratif.user_id,
        None,
    )
    .await
    .expect("attribution");
    ent.violations_fiche(etab, cast.administratif.user_id, ADF, id)
        .await
        .expect("le DPO lit la fiche");
    assert!(matches!(
        ent.violations_decider_risque(etab, cast.administratif.user_id, ADF, id, "faible", "avis")
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Mineurs touchés : la proposition monte à « avéré » ; la direction décide « faible » avec motif (écart tracé).
    let v = ent
        .violations_qualifier(etab, dir, DIR, id, qualif(&["scol.registre"], true))
        .await
        .unwrap();
    assert_eq!(v.risque_propose.as_deref(), Some("avere"));
    let v = ent
        .violations_decider_risque(
            etab,
            dir,
            DIR,
            id,
            "faible",
            "nom et classe déjà connus de ces familles (même niveau)",
        )
        .await
        .unwrap();
    assert_eq!(v.risque.as_deref(), Some("faible"));
    let decide = v
        .chronologie
        .iter()
        .find(|e| e.etape == "risque_decide")
        .unwrap();
    assert!(
        decide.texte.as_deref().unwrap().contains("écart"),
        "{decide:?}"
    );
    // Un risque inconnu est refusé.
    assert!(matches!(
        ent.violations_decider_risque(etab, dir, DIR, id, "moyen", "x pour test")
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
}

// ─────────────────────────── Notification, retard, non-notification ───────────────────────────

/// R-RG21, CL27-46 : au-delà de 72 h, la notification est **acceptée** avec le motif du retard, jamais refusée ; sans
/// motif, la saisie le demande. La notification initiale est figée (la base le garantit) ; les compléments s'ajoutent.
#[tokio::test]
async fn g5_notification_tardive_acceptee_avec_motif_puis_complement() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let maintenant = instant();
    let id = violation_connue_il_y_a(&ent, &cast, maintenant, Duration::hours(96)).await;
    let notif_le = maintenant - Duration::hours(2);
    assert!(matches!(
        ent.violations_notifier_cnil(etab, dir, DIR, id, notif_le, "CNIL-2027-0042", None)
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    // Avant la prise de connaissance, ou dans le futur : refusé.
    assert!(matches!(
        ent.violations_notifier_cnil(
            etab,
            dir,
            DIR,
            id,
            maintenant - Duration::hours(97),
            "CNIL-1",
            Some("x pour test")
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    assert!(matches!(
        ent.violations_notifier_cnil(
            etab,
            dir,
            DIR,
            id,
            maintenant + Duration::hours(1),
            "CNIL-1",
            None
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let v = ent
        .violations_notifier_cnil(
            etab,
            dir,
            DIR,
            id,
            notif_le,
            "CNIL-2027-0042",
            Some(
                "incident appris pendant les vacances, direction joignable seulement à la rentrée",
            ),
        )
        .await
        .expect("notification tardive acceptée");
    assert_eq!(v.etat, "notifiee");
    assert_eq!(v.reference_cnil.as_deref(), Some("CNIL-2027-0042"));
    assert!(v.retard_motif.is_some());
    assert!(!v.en_retard, "notifiée : plus en retard");
    assert_eq!(v.notifications.len(), 1);
    // Une seconde notification « initiale » est refusée ; un complément s'ajoute.
    assert!(matches!(
        ent.violations_notifier_cnil(etab, dir, DIR, id, notif_le, "CNIL-2", Some("x pour test"))
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    let v = ent
        .violations_notification_complementaire(
            etab,
            dir,
            DIR,
            id,
            maintenant - Duration::hours(1),
            "CNIL-2027-0042-2",
            "nombre de personnes précisé : 118",
        )
        .await
        .unwrap();
    assert_eq!(v.notifications.len(), 2);
    assert_eq!(v.notifications[1].nature, "complementaire");
    // La base refuse de réécrire la notification ou ses lignes.
    let r = sqlx::query("UPDATE rgp_violation SET reference_cnil = 'faux' WHERE id = $1")
        .bind(id)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err(), "notification CNIL figée");
    let r = sqlx::query("DELETE FROM rgp_violation_notification WHERE violation_id = $1")
        .bind(id)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err(), "notifications en ajout seul");
    // Une notification dans les 72 h n'exige aucun motif.
    let id2 = violation_connue_il_y_a(&ent, &cast, maintenant, Duration::hours(10)).await;
    let v2 = ent
        .violations_notifier_cnil(
            etab,
            dir,
            DIR,
            id2,
            maintenant - Duration::hours(1),
            "CNIL-2027-0043",
            Some("ignoré"),
        )
        .await
        .unwrap();
    assert!(
        v2.retard_motif.is_none(),
        "dans le délai, aucun motif de retard n'est conservé"
    );
}

/// Art. 33.5 : la non-notification n'est possible que pour un risque décidé faible, par la direction ; la clôture exige
/// la décision complète ; après la clôture, la base refuse toute modification ; la revue est fixée à 30 jours.
#[tokio::test]
async fn g5_non_notification_cloture_et_registre_fige() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let maintenant = instant();
    let id = violation_connue_il_y_a(&ent, &cast, maintenant, Duration::hours(3)).await;
    // Clore trop tôt : refusé, avec ce qui manque.
    match ent
        .violations_clore(etab, dir, DIR, id, "rien appris")
        .await
    {
        Err(EntDbError::ConflitDossierDetail(m)) => assert!(m.contains("qualifier"), "{m}"),
        r => panic!("clôture prématurée acceptée : {r:?}"),
    }
    ent.violations_qualifier(etab, dir, DIR, id, qualif(&["scol.registre"], false))
        .await
        .unwrap();
    // Non-notification sans risque décidé : refusée.
    assert!(matches!(
        ent.violations_justifier_non_notification(etab, dir, DIR, id, "risque faible")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.violations_avis_dpo(etab, dir, DIR, id, "avis : faible")
        .await
        .unwrap();
    ent.violations_decider_risque(etab, dir, DIR, id, "faible", "nom et classe seulement")
        .await
        .unwrap();
    // Le secrétariat (sans `violations.decider`) ne justifie pas la non-notification.
    assert!(matches!(
        ent.violations_justifier_non_notification(
            etab,
            cast.administratif.user_id,
            ADF,
            id,
            "faible"
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    let v = ent
        .violations_justifier_non_notification(
            etab,
            dir,
            DIR,
            id,
            "risque faible : noms et classes, aucune donnée sensible",
        )
        .await
        .unwrap();
    assert!(v.non_notification_motif.is_some());
    // Relever le risque après une non-notification : voir `g5_risque_releve_apres_non_notification`.
    ent.violations_ajouter_mesure(
        etab,
        dir,
        DIR,
        id,
        "prise",
        "message de retrait envoyé aux familles",
    )
    .await
    .unwrap();
    ent.violations_ajouter_mesure(
        etab,
        dir,
        DIR,
        id,
        "envisagee",
        "liste de diffusion par classe, plus par niveau",
    )
    .await
    .unwrap();
    assert!(matches!(
        ent.violations_ajouter_mesure(etab, dir, DIR, id, "oubliee", "x pour test")
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let v = ent
        .violations_clore(
            etab,
            dir,
            DIR,
            id,
            "listes de diffusion revues ; rappel fait à l'équipe",
        )
        .await
        .expect("clôture");
    assert_eq!(v.etat, "close");
    assert_eq!(v.mesures.len(), 2);
    let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
    assert_eq!(v.revue_le, Some(jour + Duration::days(30)));
    // Plus rien ne change : le service le dit, la base le garantit.
    assert!(matches!(
        ent.violations_ajouter_mesure(etab, dir, DIR, id, "prise", "trop tard")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    for sql in [
        "UPDATE rgp_violation SET intitule = 'réécrit' WHERE id = $1",
        "DELETE FROM rgp_violation WHERE id = $1",
        "INSERT INTO rgp_violation_mesure (id, etablissement_id, violation_id, nature, texte)
           SELECT gen_random_uuid(), etablissement_id, id, 'prise', 'ajout tardif' FROM rgp_violation WHERE id = $1",
        "DELETE FROM rgp_violation_categorie WHERE violation_id = $1",
        "UPDATE rgp_violation_etape SET texte = 'x' WHERE violation_id = $1",
    ] {
        let r = sqlx::query(sql).bind(id).execute(ent.pool_verif()).await;
        assert!(r.is_err(), "la base doit refuser : {sql}");
    }
}

/// CL27-47 et art. 34 : données de santé → élevé proposé ; une violation à risque élevé ne se clôt pas sans
/// information des personnes ou exception motivée (art. 34.3), décidée par la direction.
#[tokio::test]
async fn g5_risque_eleve_exige_information_ou_exception() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let maintenant = instant();
    let id = violation_connue_il_y_a(&ent, &cast, maintenant, Duration::hours(1)).await;
    let v = ent
        .violations_qualifier(etab, dir, DIR, id, qualif(&["sante.dossier"], true))
        .await
        .unwrap();
    assert_eq!(v.risque_propose.as_deref(), Some("eleve"));
    // L'exception à l'information n'a de sens que pour un risque élevé décidé.
    assert!(matches!(
        ent.violations_motiver_non_information(etab, dir, DIR, id, "chiffré")
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.violations_avis_dpo(
        etab,
        dir,
        DIR,
        id,
        "DPO : risque élevé, informer les familles",
    )
    .await
    .unwrap();
    ent.violations_decider_risque(etab, dir, DIR, id, "eleve", "données de santé de mineurs")
        .await
        .unwrap();
    ent.violations_notifier_cnil(
        etab,
        dir,
        DIR,
        id,
        maintenant - Duration::minutes(30),
        "CNIL-2027-0100",
        None,
    )
    .await
    .unwrap();
    // Risque élevé décidé : la non-notification est impossible.
    match ent.violations_clore(etab, dir, DIR, id, "fin").await {
        Err(EntDbError::ConflitDossierDetail(m)) => assert!(m.contains("informer"), "{m}"),
        r => panic!("clôture sans information acceptée : {r:?}"),
    }
    let v = ent
        .violations_motiver_non_information(
            etab,
            dir,
            DIR,
            id,
            "fichier chiffré, clé non compromise (art. 34.3 a)",
        )
        .await
        .unwrap();
    assert!(v.manque_pour_clore.is_empty(), "{:?}", v.manque_pour_clore);
    ent.violations_clore(etab, dir, DIR, id, "chiffrement des exports généralisé")
        .await
        .expect("clôture");
}

/// RGP-259 : un incident de sécurité sans atteinte aux données est au même registre, sans horloge exigée, et se clôt
/// sans risque ni notification ; il ne redevient pas un incident une fois la violation décidée.
#[tokio::test]
async fn g5_incident_de_securite_sans_atteinte() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let maintenant = instant();
    let m = ent
        .violations_declarer_a(
            etab,
            cast.cpe.user_id,
            "cpe",
            Declaration {
                intitule: "tentative d'hameçonnage reçue",
                incident_securite: true,
                ..Default::default()
            },
            maintenant,
        )
        .await
        .unwrap();
    // Le déclarant ne range pas lui-même l'incident hors des violations : son avis est noté, l'horloge court.
    let v = ent
        .violations_fiche_a(etab, dir, DIR, m.id, maintenant)
        .await
        .unwrap();
    assert_eq!(v.nature_incident, "violation");
    assert!(v
        .chronologie
        .iter()
        .any(|e| e.texte.as_deref().is_some_and(|t| t.contains("aucune donnée personnelle"))));
    // La direction requalifie.
    let mut q = qualif(&[], false);
    q.nature_incident = "incident_securite";
    q.confidentialite = false;
    ent.violations_qualifier(etab, dir, DIR, m.id, q).await.expect("requalification");
    let v = ent
        .violations_fiche_a(etab, dir, DIR, m.id, maintenant + Duration::hours(100))
        .await
        .unwrap();
    assert_eq!(v.nature_incident, "incident_securite");
    assert!(
        !v.en_retard,
        "aucune horloge CNIL pour un incident sans atteinte"
    );
    assert!(matches!(
        ent.violations_notifier_cnil(etab, dir, DIR, m.id, maintenant, "CNIL-x", None)
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.violations_clore(
        etab,
        dir,
        DIR,
        m.id,
        "personne n'a cliqué ; rappel à l'équipe",
    )
    .await
    .expect("clôture");
    // Une violation décidée ne redevient pas un simple incident.
    let id = violation_connue_il_y_a(&ent, &cast, maintenant, Duration::hours(1)).await;
    ent.violations_qualifier(etab, dir, DIR, id, qualif(&["scol.registre"], false))
        .await
        .unwrap();
    ent.violations_avis_dpo(etab, dir, DIR, id, "avis")
        .await
        .unwrap();
    ent.violations_decider_risque(etab, dir, DIR, id, "faible", "nom et classe")
        .await
        .unwrap();
    let mut q = qualif(&[], false);
    q.nature_incident = "incident_securite";
    assert!(matches!(
        ent.violations_qualifier(etab, dir, DIR, id, q).await,
        Err(EntDbError::ConflitDossier(_))
    ));
}

// ─────────────────────────── Port système et alertes d'échéance ───────────────────────────

/// RGP-241 : la supervision déclare par le port ; la même alerte ne crée jamais deux violations.
#[tokio::test]
async fn g5_port_systeme_idempotent() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let source = Uuid::new_v4();
    let (id, nouveau) = ent
        .violations_declarer_systeme(
            etab,
            "socle28.export_anormal",
            source,
            "export complet depuis un pays inhabituel",
            None,
            Some(640),
        )
        .await
        .unwrap();
    assert!(nouveau);
    let (id2, nouveau2) = ent
        .violations_declarer_systeme(
            etab,
            "socle28.export_anormal",
            source,
            "export complet depuis un pays inhabituel",
            None,
            Some(640),
        )
        .await
        .unwrap();
    assert_eq!((id2, nouveau2), (id, false));
    let v = ent
        .violations_fiche(etab, cast.direction.user_id, DIR, id)
        .await
        .unwrap();
    assert_eq!(v.origine, "supervision");
    assert_eq!(v.source_code.as_deref(), Some("socle28.export_anormal"));
    assert_eq!(v.personnes_nombre, Some(640));
    assert!(v.declaree_par_compte_id.is_none());
    let alertes = compter(
        &ent,
        "SELECT count(*) FROM ent_notification WHERE kind = 'rgpd.violation_declaree' AND corps LIKE '%' ||
           (SELECT 'V-' || annee || '-' || lpad(numero::text, 3, '0') FROM rgp_violation WHERE id = $1) || '%'",
        id,
    )
    .await;
    assert!(
        alertes >= 1,
        "la direction est prévenue d'une déclaration automatique"
    );
    assert!(matches!(
        ent.violations_declarer_systeme(etab, "Code Invalide", source, "x pour test", None, None)
            .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
}

/// §18.1 : alertes H-24 puis H-6 avant l'échéance d'une violation non réglée, une seule fois par palier ; le palier se
/// franchit (une tâche arrêtée ne fait pas manquer l'alerte) ; une violation notifiée n'alerte plus.
#[tokio::test]
async fn g5_alertes_h24_h6_une_fois_par_palier() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let t0 = instant();
    let id = violation_connue_il_y_a(&ent, &cast, t0, Duration::zero()).await;
    let notifiee = violation_connue_il_y_a(&ent, &cast, t0, Duration::zero()).await;
    ent.violations_notifier_cnil(etab, dir, DIR, notifiee, t0, "CNIL-2027-0200", None)
        .await
        .unwrap();
    let alertes = |palier: &'static str| {
        let ent = &ent;
        async move {
            compter(
                ent,
                &format!(
                    "SELECT count(*) FROM rgp_violation_etape WHERE violation_id = $1 AND etape_code = 'echeance_alertee'
                       AND texte = '{palier}'"
                ),
                id,
            )
            .await
        }
    };
    // À H-30 : rien.
    ent.violations_alerter_echeances_a(etab, t0 + Duration::hours(42))
        .await
        .unwrap();
    assert_eq!(alertes("H-24").await, 0);
    // À H-23 : H-24 ; rejoué : pas de doublon.
    ent.violations_alerter_echeances_a(etab, t0 + Duration::hours(49))
        .await
        .unwrap();
    ent.violations_alerter_echeances_a(etab, t0 + Duration::hours(50))
        .await
        .unwrap();
    assert_eq!(alertes("H-24").await, 1);
    // La tâche ne repasse qu'à H-2 : H-6 est franchi, il part.
    ent.violations_alerter_echeances_a(etab, t0 + Duration::hours(70))
        .await
        .unwrap();
    assert_eq!(alertes("H-6").await, 1);
    // Après l'échéance : plus d'alerte (le retard est affiché au registre).
    ent.violations_alerter_echeances_a(etab, t0 + Duration::hours(73))
        .await
        .unwrap();
    assert_eq!(alertes("H-6").await, 1);
    let avis = compter(
        &ent,
        &format!(
            "SELECT count(*) FROM ent_notification WHERE kind = 'rgpd.violation_echeance_72h' AND destinataire_user_id = '{dir}'
               AND corps LIKE '%' || (SELECT 'V-' || annee || '-' || lpad(numero::text, 3, '0') FROM rgp_violation WHERE id = $1) || '%'"
        ),
        id,
    )
    .await;
    assert_eq!(avis, 2, "un avis par palier dans l'ENT");
    let rien: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM rgp_violation_etape WHERE violation_id = $1 AND etape_code = 'echeance_alertee'",
    )
    .bind(notifiee)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(rien, 0, "une violation notifiée n'alerte plus");
}

/// Cloisonnement : un autre établissement ne voit ni la fiche ni la ligne au registre ; ses numéros sont à lui.
#[tokio::test]
async fn g5_cloisonnement_entre_etablissements() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let id = violation_connue_il_y_a(&ent, &cast, instant(), Duration::hours(1)).await;
    assert!(matches!(
        ent.violations_fiche(autre.etablissement_id, autre.direction.user_id, DIR, id)
            .await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));
    let reg = ent
        .violations_registre(
            autre.etablissement_id,
            autre.direction.user_id,
            DIR,
            FiltreRegistre::default(),
        )
        .await
        .unwrap();
    assert!(!reg.iter().any(|l| l.id == id));
    assert!(matches!(
        ent.violations_qualifier(
            autre.etablissement_id,
            autre.direction.user_id,
            DIR,
            id,
            qualif(&[], false)
        )
        .await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));
    // La direction de A ne gère pas B avec sa session de A.
    assert!(matches!(
        ent.violations_fiche(autre.etablissement_id, cast.direction.user_id, DIR, id)
            .await,
        Err(EntDbError::Forbidden) | Err(EntDbError::Domain(DomainError::NotFound))
    ));
}

// ─────────────────────────────── Personnes concernées et information (art. 34) ───────────────────────────────

/// RGP-244, RGP-252 : la direction calcule les personnes depuis les élèves touchés (titulaires de l'autorité
/// parentale joignables), relit l'aperçu puis confirme **par le nombre** ; l'information part une fois, aux seuls
/// destinataires calculés, et fige le calcul.
#[tokio::test]
async fn g5_personnes_calculees_puis_informees_une_fois() {
    use crate::violations_information::{OriginePersonnes, KIND_PERSONNES};
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let maintenant = instant();
    let id = violation_connue_il_y_a(&ent, &cast, maintenant, Duration::hours(2)).await;
    ent.violations_qualifier(etab, dir, DIR, id, qualif(&["eva.note"], true))
        .await
        .unwrap();
    // Un professeur ne calcule pas les personnes.
    assert!(matches!(
        ent.violations_calculer_personnes(
            etab,
            cast.prof.user_id,
            "prof",
            id,
            OriginePersonnes::Eleves(vec![cast.eleve_lea_id])
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    // Liste vide refusée.
    assert!(ent
        .violations_calculer_personnes(etab, dir, DIR, id, OriginePersonnes::Eleves(vec![]))
        .await
        .is_err());
    let v = ent
        .violations_calculer_personnes(
            etab,
            dir,
            DIR,
            id,
            OriginePersonnes::Eleves(vec![
                cast.eleve_lea_id,
                cast.eleve_tom_id,
                cast.eleve_lea_id,
            ]),
        )
        .await
        .expect("calcul");
    assert_eq!(v.personnes_nombre, Some(2), "doublon retiré");
    assert!(!v.personnes_estime);
    let a = ent
        .violations_apercu_information(etab, dir, DIR, id)
        .await
        .unwrap();
    assert_eq!(a.personnes, 2);
    assert!(
        a.destinataires >= 2,
        "au moins un titulaire par élève : {a:?}"
    );
    assert!(!a.deja_informees);
    let message =
        "Des notes de votre enfant ont été visibles par d'autres familles pendant une heure.";
    // Risque non décidé : rien ne part.
    assert!(matches!(
        ent.violations_informer_personnes_a(
            etab,
            dir,
            DIR,
            id,
            message,
            a.destinataires,
            maintenant
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    ent.violations_avis_dpo(etab, dir, DIR, id, "DPO : informer les familles concernées")
        .await
        .unwrap();
    ent.violations_decider_risque(etab, dir, DIR, id, "eleve", "notes de mineurs visibles")
        .await
        .unwrap();
    // Confirmation par un nombre faux : rien ne part.
    assert!(matches!(
        ent.violations_informer_personnes_a(
            etab,
            dir,
            DIR,
            id,
            message,
            a.destinataires + 1,
            maintenant
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    let avant: i64 =
        compter(&ent, &format!("SELECT count(*) FROM ent_notification WHERE kind = '{KIND_PERSONNES}' AND source_event_id = $1"), id)
            .await;
    assert_eq!(avant, 0);
    let v = ent
        .violations_informer_personnes_a(etab, dir, DIR, id, message, a.destinataires, maintenant)
        .await
        .expect("information");
    assert_eq!(v.personnes_informees_nombre, Some(a.destinataires as i32));
    assert_eq!(v.personnes_informees_le, Some(maintenant));
    assert!(v
        .chronologie
        .iter()
        .any(|e| e.etape == "personnes_informees"));
    let parent_lea = compter(
        &ent,
        &format!(
            "SELECT count(*) FROM ent_notification WHERE kind = '{KIND_PERSONNES}' AND destinataire_user_id = '{}'
                AND etablissement_id = (SELECT etablissement_id FROM rgp_violation WHERE id = $1)",
            cast.parent_lea.user_id
        ),
        id,
    )
    .await;
    assert_eq!(parent_lea, 1, "le titulaire de Léa est informé");
    let prof = compter(
        &ent,
        &format!(
            "SELECT count(*) FROM ent_notification WHERE kind = '{KIND_PERSONNES}' AND destinataire_user_id = '{}'
                AND etablissement_id = (SELECT etablissement_id FROM rgp_violation WHERE id = $1)",
            cast.prof.user_id
        ),
        id,
    )
    .await;
    assert_eq!(prof, 0, "personne d'autre");
    // Une seule fois ; le calcul est figé.
    assert!(matches!(
        ent.violations_informer_personnes_a(
            etab,
            dir,
            DIR,
            id,
            message,
            a.destinataires,
            maintenant
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(matches!(
        ent.violations_calculer_personnes(
            etab,
            dir,
            DIR,
            id,
            OriginePersonnes::Eleves(vec![cast.eleve_lea_id])
        )
        .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(
        ent.violations_apercu_information(etab, dir, DIR, id)
            .await
            .unwrap()
            .deja_informees
    );
    // En base aussi : l'information ne se réécrit pas.
    let r = sqlx::query("UPDATE rgp_violation SET personnes_informees_nombre = 1 WHERE id = $1")
        .bind(id)
        .execute(ent.pool_verif())
        .await;
    assert!(r.is_err(), "information figée en base");
}

/// Un compte désigné doit appartenir à l'établissement (identifiants de personnels, CL27-42).
#[tokio::test]
async fn g5_personnes_par_comptes_de_l_etablissement() {
    use crate::violations_information::OriginePersonnes;
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let id = violation_connue_il_y_a(&ent, &cast, instant(), Duration::hours(1)).await;
    let v = ent
        .violations_calculer_personnes(
            etab,
            dir,
            DIR,
            id,
            OriginePersonnes::Comptes(vec![cast.prof.user_id, cast.cpe.user_id]),
        )
        .await
        .expect("calcul");
    assert_eq!(v.personnes_nombre, Some(2));
    assert!(matches!(
        ent.violations_calculer_personnes(
            etab,
            dir,
            DIR,
            id,
            OriginePersonnes::Comptes(vec![Uuid::new_v4()])
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
}

// ─────────────────────────────── Violations de la plateforme (R-RG36) ───────────────────────────────

/// RGP-254, RGP-255, CL27-44 : l'éditeur déclare une fois, propage à deux établissements avec leurs seuls éléments ;
/// chacun voit sa violation d'origine `plateforme`, son horloge part de l'information, sa direction est alertée ; la
/// propagation est idempotente ; la notification de ScolIA n'existe que pour la couche compte.
#[tokio::test]
async fn g5_violation_de_plateforme_propagee_a_chaque_etablissement() {
    use crate::violations_plateforme::DeclarationPlateforme;
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let maintenant = instant();
    let d = DeclarationPlateforme {
        intitule: "Journaux d'un sous-traitant d'envoi de courriels exposés",
        description:
            "Un compartiment de stockage du sous-traitant d'envoi a été exposé six heures.",
        connue_le: maintenant - Duration::hours(3),
        confidentialite: true,
        ..Default::default()
    };
    // Rien d'atteint : refusé.
    assert!(matches!(
        ent.violations_plateforme_declarer(
            "Équipe sécurité ScolIA",
            DeclarationPlateforme {
                confidentialite: false,
                ..d.clone()
            },
            maintenant
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    let (pv, reference) = ent
        .violations_plateforme_declarer("Équipe sécurité ScolIA", d.clone(), maintenant)
        .await
        .unwrap();
    assert!(
        reference.starts_with("P-") && reference.len() >= 10,
        "{reference}"
    );
    let (_, reference2) = ent
        .violations_plateforme_declarer("Équipe sécurité ScolIA", d, maintenant)
        .await
        .unwrap();
    assert_ne!(reference, reference2, "numérotation sans doublon");

    let informe = maintenant + Duration::hours(1);
    let (va, cree) = ent
        .violations_plateforme_propager(
            pv,
            cast.etablissement_id,
            "212 adresses de familles du lycée",
            Some(212),
            informe,
        )
        .await
        .unwrap();
    assert!(cree);
    let (vb, _) = ent
        .violations_plateforme_propager(
            pv,
            autre.etablissement_id,
            "37 adresses de personnels",
            Some(37),
            informe,
        )
        .await
        .unwrap();
    // Idempotent.
    let (va2, cree2) = ent
        .violations_plateforme_propager(pv, cast.etablissement_id, "autre texte", None, informe)
        .await
        .unwrap();
    assert_eq!(va2, va);
    assert!(!cree2);

    let fa = ent
        .violations_fiche_a(
            cast.etablissement_id,
            cast.direction.user_id,
            DIR,
            va,
            informe,
        )
        .await
        .unwrap();
    assert_eq!(fa.origine, "plateforme");
    assert!(fa.plateforme);
    assert!(fa.intitule.starts_with(&reference));
    assert_eq!(
        fa.connue_le, informe,
        "l'horloge de l'établissement part de l'information par l'éditeur"
    );
    assert!(fa.confidentialite);
    let e = fa
        .plateforme_element
        .as_ref()
        .expect("éléments de la plateforme");
    assert_eq!(e.reference, reference);
    assert_eq!(
        e.elements, "212 adresses de familles du lycée",
        "la seconde propagation n'a rien réécrit"
    );
    assert_eq!(e.personnes_nombre, Some(212));
    assert!(!e.couche_compte);
    // Seuls ses éléments : rien de l'autre établissement dans la fiche.
    let json = serde_json::to_string(&fa).unwrap();
    assert!(
        !json.contains("37 adresses"),
        "éléments d'un autre établissement exposés"
    );
    assert!(fa.chronologie.iter().any(|x| x.etape == "propagee"));
    let fb = ent
        .violations_fiche_a(
            autre.etablissement_id,
            autre.direction.user_id,
            DIR,
            vb,
            informe,
        )
        .await
        .unwrap();
    assert_eq!(
        fb.plateforme_element.unwrap().elements,
        "37 adresses de personnels"
    );
    // La direction de chaque établissement est alertée.
    let alerte = compter(
        &ent,
        &format!(
            "SELECT count(*) FROM ent_notification WHERE kind = 'rgpd.violation_declaree' AND destinataire_user_id = '{}'
                AND etablissement_id = (SELECT etablissement_id FROM rgp_violation WHERE id = $1)
                AND corps LIKE '%{}%'",
            cast.direction.user_id, fa.reference
        ),
        va,
    )
    .await;
    assert!(alerte >= 1, "direction alertée");

    // ScolIA n'est que sous-traitant ici : pas de notification de sa part.
    assert!(matches!(
        ent.violations_plateforme_notifier_cnil(pv, maintenant, "CNIL-P-1", informe)
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    // Close : plus de propagation, mais les violations propagées restent au registre.
    ent.violations_plateforme_clore(pv, informe).await.unwrap();
    assert!(ent.violations_plateforme_clore(pv, informe).await.is_err());
    let troisieme = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    assert!(matches!(
        ent.violations_plateforme_propager(pv, troisieme.etablissement_id, "rien", None, informe)
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
    assert!(ent
        .violations_fiche_a(
            cast.etablissement_id,
            cast.direction.user_id,
            DIR,
            va,
            informe
        )
        .await
        .is_ok());
}

/// RGP-311 (R-RG50) : quand la couche compte est touchée, ScolIA est responsable et saisit sa notification, une fois.
#[tokio::test]
async fn g5_couche_compte_notifiee_par_scolia() {
    use crate::violations_plateforme::DeclarationPlateforme;
    let Some((ent, _cast)) = labo().await else {
        return;
    };
    let maintenant = instant();
    let (pv, _) = ent
        .violations_plateforme_declarer(
            "Équipe sécurité ScolIA",
            DeclarationPlateforme {
                intitule: "Jetons de session rejouables",
                description: "Des jetons de session ont pu être rejoués pendant deux heures.",
                connue_le: maintenant - Duration::hours(10),
                confidentialite: true,
                couche_compte: true,
                ..Default::default()
            },
            maintenant,
        )
        .await
        .unwrap();
    assert!(matches!(
        ent.violations_plateforme_notifier_cnil(
            pv,
            maintenant - Duration::hours(11),
            "CNIL-P-2",
            maintenant
        )
        .await,
        Err(EntDbError::SaisieInvalide(_))
    ));
    ent.violations_plateforme_notifier_cnil(
        pv,
        maintenant - Duration::hours(1),
        "CNIL-P-2",
        maintenant,
    )
    .await
    .unwrap();
    assert!(matches!(
        ent.violations_plateforme_notifier_cnil(pv, maintenant, "CNIL-P-3", maintenant)
            .await,
        Err(EntDbError::ConflitDossier(_))
    ));
}

/// Contre-revue : des faits nouveaux relèvent le risque après une non-notification justifiée ; la justification
/// tombe et la notification (tardive, motivée au besoin) redevient possible.
#[tokio::test]
async fn g5_risque_releve_apres_non_notification() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let etab = cast.etablissement_id;
    let dir = cast.direction.user_id;
    let maintenant = instant();
    let id = violation_connue_il_y_a(&ent, &cast, maintenant, Duration::hours(2)).await;
    ent.violations_qualifier(etab, dir, DIR, id, qualif(&["scol.registre"], false))
        .await
        .unwrap();
    ent.violations_avis_dpo(etab, dir, DIR, id, "DPO : nom et classe seulement")
        .await
        .unwrap();
    ent.violations_decider_risque(etab, dir, DIR, id, "faible", "nom et classe")
        .await
        .unwrap();
    ent.violations_justifier_non_notification(etab, dir, DIR, id, "risque faible : nom et classe seulement")
        .await
        .unwrap();
    let v = ent
        .violations_decider_risque(etab, dir, DIR, id, "avere", "les notes étaient aussi jointes")
        .await
        .expect("risque relevé");
    assert!(v.non_notification_motif.is_none(), "la justification tombe");
    assert!(v
        .chronologie
        .iter()
        .any(|e| e.texte.as_deref().is_some_and(|t| t.contains("justification de non-notification tombe"))));
    ent.violations_notifier_cnil(etab, dir, DIR, id, maintenant, "CNIL-2027-0200", None)
        .await
        .expect("notification après relèvement");
}

// ---------------------------------------------------------------------------------------------
// Complément G5 : console de l'éditeur et fiches PDF (docs/socles/27_G5_COMPLEMENT_LIVRAISON.md)
// ---------------------------------------------------------------------------------------------

/// Console de l'éditeur : la violation se retrouve par sa référence (casse indifférente) ou son identifiant ; la liste
/// des propagations ne nomme que les établissements réellement informés, avec la référence de leur registre ; la cible
/// d'une propagation se lit par son slug (confirmation).
#[tokio::test]
async fn g5c_console_editeur_trouver_lister_propagations() {
    use crate::violations_plateforme::DeclarationPlateforme;
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let maintenant = instant();
    let (pv, reference) = ent
        .violations_plateforme_declarer(
            "Équipe sécurité ScolIA",
            DeclarationPlateforme {
                intitule: "Sauvegarde d'un sous-traitant exposée",
                description: "Une sauvegarde chiffrée a été rendue publique une heure.",
                connue_le: maintenant - Duration::hours(2),
                disponibilite: true,
                ..Default::default()
            },
            maintenant,
        )
        .await
        .unwrap();
    let v = ent.violations_plateforme_trouver(&reference).await.unwrap();
    assert_eq!(v.id, pv);
    assert_eq!(v.etat, "ouverte");
    assert_eq!(
        ent.violations_plateforme_trouver(&reference.to_lowercase()).await.unwrap().id,
        pv,
        "référence saisie en minuscules"
    );
    assert_eq!(ent.violations_plateforme_trouver(&pv.to_string()).await.unwrap().reference, reference);
    assert!(matches!(
        ent.violations_plateforme_trouver("P-1999-999").await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));
    assert!(ent
        .violations_plateforme_lister()
        .await
        .unwrap()
        .iter()
        .any(|l| l.id == pv));
    // Personne n'est informé tant que l'éditeur n'a pas propagé.
    assert!(ent.violations_plateforme_propagations(pv).await.unwrap().is_empty());
    // Le labo ne crée pas toujours la fiche de l'établissement (la production l'a toujours).
    sqlx::query(
        "INSERT INTO ent_etablissement (id, slug, nom) VALUES ($1, 'g5c-' || substr($1::text, 1, 8), 'Collège G5c')
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(cast.etablissement_id)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let cible = ent.violations_plateforme_cible(cast.etablissement_id).await.unwrap();
    assert!(!cible.slug.is_empty());
    assert!(matches!(
        ent.violations_plateforme_cible(Uuid::new_v4()).await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));
    let (va, _) = ent
        .violations_plateforme_propager(pv, cast.etablissement_id, "travaux de 3e indisponibles", Some(90), maintenant)
        .await
        .unwrap();
    let p = ent.violations_plateforme_propagations(pv).await.unwrap();
    assert_eq!(p.len(), 1, "seul l'établissement informé : {p:?}");
    assert_eq!(p[0].etablissement_id, cast.etablissement_id);
    assert_eq!(p[0].etablissement_slug, cible.slug);
    assert_eq!(p[0].personnes_nombre, Some(90));
    let fiche = ent
        .violations_fiche_a(cast.etablissement_id, cast.direction.user_id, DIR, va, maintenant)
        .await
        .unwrap();
    assert_eq!(p[0].reference_locale, fiche.reference);
    assert!(!p.iter().any(|x| x.etablissement_id == autre.etablissement_id));
}

/// Fiches PDF : réservées aux gestionnaires, instants dans le fuseau de l'établissement, filtre dit en clair, 404
/// neutre pour une violation d'un autre établissement.
#[tokio::test]
async fn g5c_fiches_pdf_droits_fuseau_et_cloisonnement() {
    let Some((ent, cast)) = labo().await else {
        return;
    };
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let maintenant = instant();
    let id = violation_connue_il_y_a(&ent, &cast, maintenant, Duration::hours(5)).await;
    let (reference, pdf) = ent
        .violations_fiche_pdf_a(cast.etablissement_id, cast.direction.user_id, DIR, id, maintenant)
        .await
        .unwrap();
    assert!(reference.starts_with("V-"));
    let texte = ent_pdf::releve::texte(&pdf);
    assert!(texte.contains(&reference), "{texte}");
    assert!(texte.contains("liste de classe envoyée"), "{texte}");
    // Heure de Paris, comme la base l'écrit pour l'établissement.
    let attendu: String = sqlx::query_scalar(
        "SELECT to_char(($1::timestamptz - interval '5 hours') AT TIME ZONE 'Europe/Paris', 'DD/MM/YYYY \"à\" HH24 \"h\" MI')",
    )
    .bind(maintenant)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(texte.contains(&attendu), "prise de connaissance « {attendu} » : {texte}");
    // Un professeur n'a pas le registre ; une autre direction ne voit pas cette violation (404 neutre).
    assert!(matches!(
        ent.violations_fiche_pdf_a(cast.etablissement_id, cast.prof.user_id, PROF, id, maintenant).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.violations_fiche_pdf_a(autre.etablissement_id, autre.direction.user_id, DIR, id, maintenant).await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));
    let registre = ent
        .violations_registre_pdf(
            cast.etablissement_id,
            cast.direction.user_id,
            DIR,
            FiltreRegistre {
                etat: Some("ouverte"),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let t = ent_pdf::releve::texte(&registre);
    assert!(t.contains(&reference) && t.contains("état « à qualifier »"), "{t}");
    assert!(matches!(
        ent.violations_registre_pdf(cast.etablissement_id, cast.prof.user_id, PROF, FiltreRegistre::default())
            .await,
        Err(EntDbError::Forbidden)
    ));
}
