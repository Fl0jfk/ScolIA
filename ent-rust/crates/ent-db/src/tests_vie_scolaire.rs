//! Socle 2, lot S2-4 : vie scolaire complète contre la base locale de test
//! (feature `labo`). Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.
//!
//! Les dates de test sont dans ~4 mois (premier lundi du mois de J+120) : un mois
//! que le jeu de données du labo (qui vit aujourd'hui) ne touche pas, et dont la
//! première semaine pleine (lundi→vendredi) tient dans le même mois.

use super::*;
use crate::appel_repo::{Appel, LigneEtat};
use crate::outbox::ConfigDistribution;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use domain_eleve::{today_utc, DeclareAbsence, DomainError};
use ent_auth::{Action, Portee, Role};
use uuid::Uuid;

async fn labo() -> Option<(PgEnt, LaboCast)> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    let (ent, cast) = crate::tests_rls_stricte::labo_selon_mode(&url, |e| { e.install_brain_p0(); }).await;
    Some((ent, cast))
}

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

/// Premier lundi du mois de J+120.
fn lundi() -> NaiveDate {
    let mut d = (today_utc() + Duration::days(120)).with_day(1).unwrap();
    while d.weekday() != Weekday::Mon {
        d += Duration::days(1);
    }
    d
}

fn config(etab: Uuid) -> ConfigDistribution {
    ConfigDistribution {
        taille_lot: 100,
        max_tentatives: 3,
        delai_base: std::time::Duration::ZERO,
        delai_max: std::time::Duration::ZERO,
        intervalle: std::time::Duration::from_millis(20),
        etablissement: Some(etab),
    }
}

async fn distribuer(ent: &PgEnt, etab: Uuid) {
    ent.distributeur(config(etab)).distribuer_tout().await.unwrap();
}

fn prof(cast: &LaboCast) -> (Uuid, &'static str) {
    (cast.prof.user_id, Role::Prof.as_str())
}

/// Séance 4B de `debut` à `fin` le `jour`, appel ouvert, `etats` appliqués (clôturé si `clore`).
async fn seance_appel(
    ent: &PgEnt,
    cast: &LaboCast,
    jour: NaiveDate,
    debut: (u32, u32),
    fin: (u32, u32),
    etats: &[(Uuid, LigneEtat)],
    clore: bool,
) -> (Uuid, Appel) {
    let etab = cast.etablissement_id;
    let c = ent
        .seed_creneau_du_jour(etab, "4B", "Test S2-4", cast.prof.user_id, jour, t(debut.0, debut.1), t(fin.0, fin.1), None)
        .await
        .expect("créneau");
    let (appel, _) = ent
        .ouvrir_appel(etab, c.id, jour, cast.prof.user_id, Role::Prof.as_str())
        .await
        .expect("ouvrir");
    for (eleve, etat) in etats {
        ent.marquer_appel_ligne(etab, appel.id, cast.prof.user_id, Role::Prof.as_str(), *eleve, etat.clone())
            .await
            .expect("marquer");
    }
    if clore {
        ent.clore_appel(etab, appel.id, cast.prof.user_id, Role::Prof.as_str(), None)
            .await
            .expect("clore");
    }
    (c.id, appel)
}

async fn compter(ent: &PgEnt, sql: &str, a: Uuid) -> i64 {
    let (n,): (i64,) = sqlx::query_as(sql).bind(a).fetch_one(ent.pool_verif()).await.unwrap();
    n
}

async fn permettre_justifier(ent: &PgEnt, cast: &LaboCast, eleve: Uuid, user: Uuid, oui: bool) {
    sqlx::query("UPDATE ent_eleve_lien SET peut_justifier_absence = $4 WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(cast.etablissement_id)
        .bind(eleve)
        .bind(user)
        .bind(oui)
        .execute(ent.pool_verif())
        .await
        .unwrap();
}

fn decl(etab: Uuid, eleve: Uuid, debut: NaiveDate, fin: NaiveDate) -> DeclarerAbsence {
    DeclarerAbsence {
        etablissement_id: etab,
        eleve_id: eleve,
        debut_jour: debut,
        debut_heure: None,
        fin_jour: fin,
        fin_heure: None,
        motif: "maladie".into(),
        commentaire: Some("Grippe".into()),
        declarant_nom: Some("Mme Granger".into()),
        declarant_qualite: Some("mère".into()),
        piece_document_id: None,
    }
}

async fn prevenue(ent: &PgEnt, appel: Uuid, eleve: Uuid) -> bool {
    let (p,): (bool,) = sqlx::query_as("SELECT prevenue FROM ent_absence_seance WHERE appel_id = $1 AND eleve_id = $2")
        .bind(appel)
        .bind(eleve)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    p
}

async fn notifs(ent: &PgEnt, etab: Uuid, user: Uuid, kind: &str) -> usize {
    ent.mes_notifications(etab, user, 200)
        .await
        .unwrap()
        .into_iter()
        .filter(|n| n.kind == kind)
        .count()
}

// ─── 1. Absences à la séance → demi-journées dérivées ─────────────────────────

#[tokio::test]
async fn demi_journees_derivees_des_appels_et_cas_limites() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let lun = lundi();
    let mar = lun + Duration::days(1);

    // Lundi : deux séances le matin → une seule demi-journée, 120 min.
    let (_, a1) = seance_appel(&ent, &cast, lun, (8, 0), (9, 0), &[(lea, LigneEtat::Absent)], true).await;
    let (_, a2) = seance_appel(&ent, &cast, lun, (9, 0), (10, 0), &[(lea, LigneEtat::Absent)], true).await;
    let dj = ent.demi_journees(etab, Some(lea), lun, lun).await.unwrap();
    assert_eq!(dj.len(), 1);
    assert_eq!((dj[0].periode, dj[0].minutes), (Periode::Matin, 120));
    assert!(dj[0].non_justifiee);

    // Justification de l'absence du jour (rattachée aux deux séances).
    sqlx::query("UPDATE ent_absence SET statut = 'justifiee' WHERE id = (SELECT absence_id FROM ent_absence_seance WHERE appel_id = $1)")
        .bind(a1.id).execute(ent.pool_verif()).await.unwrap();
    let dj = ent.demi_journees(etab, Some(lea), lun, lun).await.unwrap();
    // AS0 (DA-05, DA-12) : les deux séances de la journée sont rattachées à une seule
    // absence ; la justifier couvre ses deux séances (plus de justification ligne par ligne).
    assert_eq!((dj[0].minutes, dj[0].minutes_non_justifiees), (120, 0));
    assert!(dj[0].comptee && !dj[0].non_justifiee, "absence justifiée : comptée, plus au décompte");
    // Tout justifié : comptée, plus au décompte réglementaire.
    sqlx::query("UPDATE ent_absence SET statut = 'justifiee' WHERE id = (SELECT absence_id FROM ent_absence_seance WHERE appel_id = $1)")
        .bind(a2.id).execute(ent.pool_verif()).await.unwrap();
    let dj = ent.demi_journees(etab, Some(lea), lun, lun).await.unwrap();
    assert!(dj[0].comptee && !dj[0].non_justifiee);
    // Remise « présent » : la séance ne contribue plus.
    sqlx::query("UPDATE ent_absence SET statut = 'annulee_present' WHERE id IN (SELECT absence_id FROM ent_absence_seance WHERE appel_id = ANY($1))")
        .bind(vec![a1.id, a2.id]).execute(ent.pool_verif()).await.unwrap();
    assert!(ent.demi_journees(etab, Some(lea), lun, lun).await.unwrap().is_empty());

    // Mardi : séance 11:30–12:30, ventilée 30 / 30 (seuil par défaut 30).
    seance_appel(&ent, &cast, mar, (11, 30), (12, 30), &[(lea, LigneEtat::Absent)], true).await;
    let dj = ent.demi_journees(etab, Some(lea), mar, mar).await.unwrap();
    assert_eq!(dj.len(), 2);
    assert!(dj.iter().all(|d| d.minutes == 30 && d.comptee));

    // Règle paramétrable (cpe : assiduite.signaler) : seuil 31 → ni l'une ni l'autre.
    let cpe = (cast.cpe.user_id, Role::Cpe.as_str());
    let regle = RegleAssiduite { seuil_minutes: 31, ..RegleAssiduite::default() };
    ent.definir_regle_assiduite(etab, cpe.0, cpe.1, regle).await.unwrap();
    let dj = ent.demi_journees(etab, Some(lea), mar, mar).await.unwrap();
    assert!(dj.iter().all(|d| !d.comptee));
    // Prof : pas le droit de paramétrer ; valeurs hors bornes refusées.
    assert!(matches!(
        ent.definir_regle_assiduite(etab, cast.prof.user_id, "prof", regle).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .definir_regle_assiduite(etab, cpe.0, cpe.1, RegleAssiduite { seuil_minutes: 0, ..regle })
        .await
        .is_err());
    // Coupure à 12:30 : la séance 11:30–12:30 est tout entière le matin (60 min).
    let regle = RegleAssiduite { heure_coupure: t(12, 30), ..RegleAssiduite::default() };
    ent.definir_regle_assiduite(etab, cpe.0, cpe.1, regle).await.unwrap();
    let dj = ent.demi_journees(etab, Some(lea), mar, mar).await.unwrap();
    assert_eq!(dj.len(), 1);
    assert_eq!((dj[0].periode, dj[0].minutes), (Periode::Matin, 60));
}

// ─── 2. Reprise des absences existantes (à la journée) ───────────────────────

#[tokio::test]
async fn reprise_des_absences_a_la_journee_et_decompte_mensuel() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let tom = cast.eleve_tom_id;
    let lun = lundi();
    // Deux semaines (lundi → dimanche suivant) : 10 jours ouvrés, week-ends exclus.
    let abs = ent
        .declare_absence(DeclareAbsence {
            etablissement_id: etab,
            eleve_id: tom,
            debut: lun,
            fin: lun + Duration::days(13),
            motif: "maladie".into(),
            actor_user_id: None,
        })
        .await
        .unwrap();
    // Avant toute matérialisation : le repli compte déjà 20 demi-journées.
    let dj = ent.demi_journees(etab, Some(tom), lun, lun + Duration::days(13)).await.unwrap();
    assert_eq!(dj.iter().filter(|d| d.non_justifiee).count(), 20);
    assert!(dj.iter().all(|d| d.jour.weekday().number_from_monday() < 6));
    // La migration (reprise) matérialise puis reste idempotente : pas de double compte.
    crate::migrate::rejouer_tout(ent.pool_verif()).await.unwrap(); // rejeu des fichiers (reprise), hors registre
    crate::migrate::rejouer_tout(ent.pool_verif()).await.unwrap(); // rejeu des fichiers (reprise), hors registre
    let lignes = compter(&ent, "SELECT count(*) FROM ent_absence_seance WHERE absence_id = $1", abs.id).await;
    assert_eq!(lignes, 20);
    let dj = ent.demi_journees(etab, Some(tom), lun, lun + Duration::days(13)).await.unwrap();
    assert_eq!(dj.iter().filter(|d| d.non_justifiee).count(), 20);

    // Décompte du mois : seule la part dans le mois compte (la seconde semaine peut
    // déborder). Le premier lundi est ≤ 7 : la 1re semaine est entièrement dans le mois.
    let cpe = (cast.cpe.user_id, Role::Cpe.as_str());
    let d = ent.decompte_mensuel_for_actor(etab, cpe.0, cpe.1, lun).await.unwrap();
    let ligne = d.eleves.iter().find(|e| e.eleve_id == tom).expect("tom au décompte");
    assert!(ligne.demi_journees_non_justifiees >= 10);
    assert!(ligne.seuil_atteint && d.seuil_signalement == 4);
    // Le décompte est réservé à assiduite.signaler.
    assert!(matches!(
        ent.decompte_mensuel_for_actor(etab, cast.prof.user_id, "prof", lun).await,
        Err(EntDbError::Forbidden)
    ));
    // Isolation : un autre établissement ne voit rien de cet élève.
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let d2 = ent
        .decompte_mensuel_for_actor(autre.etablissement_id, autre.cpe.user_id, "cpe", lun)
        .await
        .unwrap();
    assert!(d2.eleves.iter().all(|e| e.eleve_id != tom));
}

// ─── 3. Déclarations et rapprochement ────────────────────────────────────────

#[tokio::test]
async fn famille_declare_puis_rapprochement_a_l_appel() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let parent = (cast.parent_lea.user_id, "parent");
    permettre_justifier(&ent, &cast, lea, parent.0, true).await;
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_vs = true WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(lea).bind(parent.0).execute(ent.pool_verif()).await.unwrap();
    let lun = lundi();

    // Déclaration avant : lundi de 08:00 à 09:00 seulement.
    let mut c = decl(etab, lea, lun, lun);
    c.debut_heure = Some(t(8, 0));
    c.fin_heure = Some(t(9, 0));
    let d = ent.declarer_absence_famille(parent.0, parent.1, c).await.unwrap();
    assert_eq!(d.source, "famille");
    assert_eq!(d.declarant_user_id, Some(parent.0));
    assert!(d.declarant_nom.is_none(), "l'identité vient du compte, pas de la saisie");

    // Séance 8–9 : couverte (bornes incluses) → prévenue. Séance 9–10 : non couverte.
    let (_, a1) = seance_appel(&ent, &cast, lun, (8, 0), (9, 0), &[(lea, LigneEtat::Absent)], true).await;
    let (_, a2) = seance_appel(&ent, &cast, lun, (9, 0), (10, 0), &[(lea, LigneEtat::Absent)], true).await;
    assert!(prevenue(&ent, a1.id, lea).await);
    assert!(!prevenue(&ent, a2.id, lea).await);
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_absence_rapprochement WHERE declaration_id = $1", d.id).await,
        1
    );
    // Réaction : la famille n'est notifiée que pour l'absence NON annoncée.
    distribuer(&ent, etab).await;
    assert_eq!(notifs(&ent, etab, parent.0, "absence_constatee").await, 1);

    // Déclaration après coup : mardi, constat d'abord, déclaration ensuite.
    let mar = lun + Duration::days(1);
    let (_, a3) = seance_appel(&ent, &cast, mar, (8, 0), (9, 0), &[(lea, LigneEtat::Absent)], true).await;
    assert!(!prevenue(&ent, a3.id, lea).await);
    let d2 = ent.declarer_absence_famille(parent.0, parent.1, decl(etab, lea, mar, mar)).await.unwrap();
    assert!(prevenue(&ent, a3.id, lea).await, "rapprochement dans l'autre sens");
    assert_eq!(
        compter(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = 'vs.absence_rapprochee'", etab).await,
        1
    );

    // Annulation : l'absence redevient non prévenue ; double annulation refusée.
    ent.annuler_declaration(etab, d2.id, parent.0, parent.1).await.unwrap();
    assert!(!prevenue(&ent, a3.id, lea).await);
    assert!(matches!(
        ent.annuler_declaration(etab, d2.id, parent.0, parent.1).await,
        Err(EntDbError::Domain(DomainError::Conflit))
    ));
    // Un autre responsable (sans lien avec Léa) ne peut pas annuler la déclaration de lundi.
    assert!(matches!(
        ent.annuler_declaration(etab, d.id, cast.parent_tom.user_id, "parent").await,
        Err(EntDbError::Forbidden)
    ));
    // Deux déclarations se recouvrent : annuler l'une garde « prévenue » via l'autre.
    let d3 = ent.declarer_absence_famille(parent.0, parent.1, decl(etab, lea, mar, mar)).await.unwrap();
    // (Une déclaration strictement identique est idempotente, DA-22 : d4 a donc une plage horaire.)
    let mut c4 = decl(etab, lea, mar, mar);
    c4.debut_heure = Some(t(8, 0));
    c4.fin_heure = Some(t(9, 0));
    let d4 = ent.declarer_absence_famille(parent.0, parent.1, c4).await.unwrap();
    assert_ne!(d3.id, d4.id);
    ent.annuler_declaration(etab, d3.id, parent.0, parent.1).await.unwrap();
    assert!(prevenue(&ent, a3.id, lea).await, "d4 couvre encore la séance");
    ent.annuler_declaration(etab, d4.id, parent.0, parent.1).await.unwrap();
    assert!(!prevenue(&ent, a3.id, lea).await);
}

#[tokio::test]
async fn droits_et_validations_des_declarations() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let lun = lundi();
    permettre_justifier(&ent, &cast, lea, cast.parent_lea.user_id, true).await;

    // Famille : autre foyer → refus ; sans droit de justifier → refus.
    assert!(matches!(
        ent.declarer_absence_famille(cast.parent_tom.user_id, "parent", decl(etab, lea, lun, lun)).await,
        Err(EntDbError::Forbidden)
    ));
    permettre_justifier(&ent, &cast, lea, cast.parent_lea.user_id, false).await;
    assert!(matches!(
        ent.declarer_absence_famille(cast.parent_lea.user_id, "parent", decl(etab, lea, lun, lun)).await,
        Err(EntDbError::Forbidden)
    ));
    permettre_justifier(&ent, &cast, lea, cast.parent_lea.user_id, true).await;
    // Un élève mineur (compte face élève) ne déclare pas.
    if let Some(compte) = cast.eleves.iter().find(|e| e.id == lea).and_then(|e| e.compte.clone()) {
        assert!(matches!(
            ent.declarer_absence_famille(compte.user_id, "eleve", decl(etab, lea, lun, lun)).await,
            Err(EntDbError::Forbidden)
        ));
    }
    // Un rôle personnel ne passe pas par la porte « famille ».
    assert!(matches!(
        ent.declarer_absence_famille(cast.cpe.user_id, "cpe", decl(etab, lea, lun, lun)).await,
        Err(EntDbError::Forbidden)
    ));

    let p = (cast.parent_lea.user_id, "parent");
    // Validations : motif inconnu, période inversée, heures inversées, trop loin, pièce étrangère.
    let mut c = decl(etab, lea, lun, lun);
    c.motif = "inconnu_xx".into();
    assert!(matches!(ent.declarer_absence_famille(p.0, p.1, c).await, Err(EntDbError::MotifInconnu)));
    assert!(ent.declarer_absence_famille(p.0, p.1, decl(etab, lea, lun + Duration::days(1), lun)).await.is_err());
    let mut c = decl(etab, lea, lun, lun);
    c.debut_heure = Some(t(10, 0));
    c.fin_heure = Some(t(9, 0));
    assert!(ent.declarer_absence_famille(p.0, p.1, c).await.is_err());
    assert!(ent.declarer_absence_famille(p.0, p.1, decl(etab, lea, lun + Duration::days(400), lun + Duration::days(401))).await.is_err());
    let mut c = decl(etab, lea, lun, lun);
    c.piece_document_id = Some(Uuid::new_v4());
    assert!(ent.declarer_absence_famille(p.0, p.1, c).await.is_err());
    let mut c = decl(etab, lea, lun, lun);
    c.commentaire = Some("x".repeat(1001));
    assert!(ent.declarer_absence_famille(p.0, p.1, c).await.is_err());

    // Accueil : identité de l'appelant obligatoire ; rôles autorisés par le moteur.
    let adm = (cast.administratif.user_id, Role::Administratif.as_str());
    let mut sans_nom = decl(etab, lea, lun, lun);
    sans_nom.declarant_nom = None;
    assert!(ent.declarer_absence_accueil(adm.0, adm.1, sans_nom).await.is_err());
    let d = ent.declarer_absence_accueil(adm.0, adm.1, decl(etab, lea, lun, lun)).await.unwrap();
    assert_eq!((d.source.as_str(), d.declarant_nom.as_deref()), ("accueil", Some("Mme Granger")));
    assert!(d.declarant_user_id.is_none());
    ent.declarer_absence_accueil(cast.cpe.user_id, "cpe", decl(etab, lea, lun, lun)).await.unwrap();
    // Professeur et famille : pas `absences.declarer`.
    assert!(matches!(
        ent.declarer_absence_accueil(cast.prof.user_id, "prof", decl(etab, lea, lun, lun)).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.declarer_absence_accueil(cast.parent_lea.user_id, "parent", decl(etab, lea, lun, lun)).await,
        Err(EntDbError::Forbidden)
    ));

    // Isolation : l'élève d'un autre établissement est introuvable ici.
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let r = ent
        .declarer_absence_accueil(adm.0, adm.1, decl(etab, autre.eleve_lea_id, lun, lun))
        .await;
    assert!(matches!(r, Err(EntDbError::Domain(DomainError::NotFound))));
    // Lecture des déclarations : famille liée oui, autre foyer non.
    assert!(!ent.declarations_eleve_for_actor(etab, cast.cpe.user_id, "cpe", lea).await.unwrap().is_empty());
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_vs = true WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(lea).bind(cast.parent_lea.user_id).execute(ent.pool_verif()).await.unwrap();
    assert!(ent.declarations_eleve_for_actor(etab, p.0, p.1, lea).await.is_ok());
    assert!(matches!(
        ent.declarations_eleve_for_actor(etab, cast.parent_tom.user_id, "parent", lea).await,
        Err(EntDbError::Forbidden)
    ));
}

#[tokio::test]
async fn role_parametrable_avec_absences_declarer() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let lun = lundi();
    // Le professeur n'a pas l'action par défaut (modèle fidèle à l'existant).
    assert!(matches!(
        ent.declarer_absence_accueil(cast.prof.user_id, "prof", decl(etab, lea, lun, lun)).await,
        Err(EntDbError::Forbidden)
    ));
    // L'établissement crée un rôle « Accueil renfort » et l'attribue au professeur.
    let role = ent
        .creer_role_for_actor(
            etab,
            cast.admin.user_id,
            "admin",
            RoleSaisie {
                nom: format!("Accueil renfort {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![Droit { action: Action::AbsencesDeclarer, portee: Portee::Etablissement, cibles: vec![] }],
            },
        )
        .await
        .unwrap();
    ent.attribuer_role_for_actor(etab, cast.admin.user_id, "admin", role.id, cast.prof.user_id, None)
        .await
        .unwrap();
    ent.declarer_absence_accueil(cast.prof.user_id, "prof", decl(etab, lea, lun, lun)).await.unwrap();
    // Mais pas la vérification d'appel : il n'a pas `appel.verifier`.
    assert!(matches!(
        ent.verifier_appels_for_actor(etab, cast.prof.user_id, "prof", lun, lun.and_hms_opt(9, 0, 0).unwrap()).await,
        Err(EntDbError::Forbidden)
    ));
}

// ─── 4. Vérification de l'appel ──────────────────────────────────────────────

fn at(jour: NaiveDate, h: u32, m: u32) -> NaiveDateTime {
    jour.and_hms_opt(h, m, 0).unwrap()
}

#[tokio::test]
async fn verification_des_appels_non_faits_et_relance() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lun = lundi();
    let cpe = (cast.cpe.user_id, Role::Cpe.as_str());
    let c = ent
        .seed_creneau_du_jour(etab, "4B", "Vérif", cast.prof.user_id, lun, t(8, 0), t(9, 0), None)
        .await
        .unwrap();
    let ligne = |v: &VerificationAppels| v.seances_sans_appel.iter().find(|s| s.creneau_id == c.id).cloned();

    // Avant H+15 : rien. À H+15 pile : appel non fait.
    let v = ent.verifier_appels_for_actor(etab, cpe.0, cpe.1, lun, at(lun, 8, 14)).await.unwrap();
    assert!(ligne(&v).is_none());
    let v = ent.verifier_appels_for_actor(etab, cpe.0, cpe.1, lun, at(lun, 8, 15)).await.unwrap();
    let s = ligne(&v).expect("non fait à H+15");
    assert_eq!((s.etat.as_str(), s.minutes_depuis_debut, s.nb_relances), ("non_fait", 15, 0));
    assert_eq!(s.prof_user_id, cast.prof.user_id);
    // Un jour futur n'est pas signalé ; un jour passé l'est.
    let v = ent.verifier_appels_for_actor(etab, cpe.0, cpe.1, lun, at(lun - Duration::days(1), 20, 0)).await.unwrap();
    assert!(ligne(&v).is_none());
    let v = ent.verifier_appels_for_actor(etab, cpe.0, cpe.1, lun, at(lun + Duration::days(1), 8, 0)).await.unwrap();
    assert!(ligne(&v).is_some());

    // Appel ouvert : pas signalé pendant la séance, « non clos » après la fin.
    let (appel, _) = ent.ouvrir_appel(etab, c.id, lun, cast.prof.user_id, "prof").await.unwrap();
    let v = ent.verifier_appels_for_actor(etab, cpe.0, cpe.1, lun, at(lun, 8, 30)).await.unwrap();
    assert!(ligne(&v).is_none());
    let v = ent.verifier_appels_for_actor(etab, cpe.0, cpe.1, lun, at(lun, 9, 0)).await.unwrap();
    assert_eq!(ligne(&v).unwrap().etat, "non_clos");

    // Relance : tracée, comptée, notifiée à l'enseignant ; refusée à un non-habilité.
    let r = ent.relancer_appel(etab, cpe.0, cpe.1, c.id, lun).await.unwrap();
    assert_eq!(r.destinataire_user_id, cast.prof.user_id);
    ent.relancer_appel(etab, cpe.0, cpe.1, c.id, lun).await.unwrap();
    let v = ent.verifier_appels_for_actor(etab, cpe.0, cpe.1, lun, at(lun, 9, 5)).await.unwrap();
    assert_eq!(ligne(&v).unwrap().nb_relances, 2);
    distribuer(&ent, etab).await;
    assert_eq!(notifs(&ent, etab, cast.prof.user_id, "appel_a_faire").await, 2);
    assert!(matches!(
        ent.relancer_appel(etab, cast.prof.user_id, "prof", c.id, lun).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.relancer_appel(etab, cpe.0, cpe.1, Uuid::new_v4(), lun).await,
        Err(EntDbError::Domain(DomainError::NotFound))
    ));

    // Appel clos : disparaît de la liste, plus de relance possible.
    ent.clore_appel(etab, appel.id, cast.prof.user_id, "prof", None).await.unwrap();
    let v = ent.verifier_appels_for_actor(etab, cpe.0, cpe.1, lun, at(lun, 9, 30)).await.unwrap();
    assert!(ligne(&v).is_none());
    assert!(matches!(
        ent.relancer_appel(etab, cpe.0, cpe.1, c.id, lun).await,
        Err(EntDbError::Domain(DomainError::Conflit))
    ));
    // Annulation EDT : une séance annulée n'est jamais « non faite ».
    let c2 = ent
        .seed_creneau_du_jour(etab, "4B", "Annulée", cast.prof.user_id, lun, t(10, 0), t(11, 0), None)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ent_edt_exception (id, etablissement_id, creneau_id, jour, kind, motif, created_by_user_id) VALUES ($1,$2,$3,$4,'annulation','grève',$5)")
        .bind(Uuid::new_v4()).bind(etab).bind(c2.id).bind(lun).bind(cast.admin.user_id).execute(ent.pool_verif()).await.unwrap();
    let v = ent.verifier_appels_for_actor(etab, cpe.0, cpe.1, lun, at(lun, 12, 0)).await.unwrap();
    assert!(v.seances_sans_appel.iter().all(|s| s.creneau_id != c2.id));

    // Droits et isolation.
    assert!(matches!(
        ent.verifier_appels_for_actor(etab, cast.prof.user_id, "prof", lun, at(lun, 9, 0)).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.verifier_appels_for_actor(etab, cast.parent_lea.user_id, "parent", lun, at(lun, 9, 0)).await,
        Err(EntDbError::Forbidden)
    ));
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let v = ent
        .verifier_appels_for_actor(autre.etablissement_id, autre.cpe.user_id, "cpe", lun, at(lun + Duration::days(1), 8, 0))
        .await
        .unwrap();
    assert!(v.seances_sans_appel.iter().all(|s| s.creneau_id != c.id));
    // Un CPE d'un autre établissement ne peut pas relancer une séance qui n'est pas la sienne.
    assert!(ent.relancer_appel(autre.etablissement_id, autre.cpe.user_id, "cpe", c.id, lun).await.is_err());
}

#[tokio::test]
async fn incoherences_entre_appels() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let lun = lundi();
    let cpe = (cast.cpe.user_id, Role::Cpe.as_str());
    ent.declarer_absence_accueil(
        cast.administratif.user_id,
        "administratif",
        {
            let mut c = decl(etab, lea, lun, lun);
            c.debut_heure = Some(t(10, 0));
            c.fin_heure = Some(t(11, 0));
            c
        },
    )
    .await
    .unwrap();
    // Absent à 8h, présent à 9h (séance suivante) ; présent à 10h malgré la déclaration.
    seance_appel(&ent, &cast, lun, (8, 0), (9, 0), &[(lea, LigneEtat::Absent)], true).await;
    seance_appel(&ent, &cast, lun, (9, 0), (10, 0), &[], true).await;
    seance_appel(&ent, &cast, lun, (10, 0), (11, 0), &[], true).await;
    let v = ent.verifier_appels_for_actor(etab, cpe.0, cpe.1, lun, at(lun, 12, 0)).await.unwrap();
    let mes: Vec<_> = v.incoherences.iter().filter(|i| i.eleve_id == lea).collect();
    assert!(mes.iter().any(|i| i.kind == "absent_puis_present"));
    assert!(mes.iter().any(|i| i.kind == "present_malgre_declaration"));
    // L'incohérence n'apparaît pas pour un autre jour.
    let v = ent.verifier_appels_for_actor(etab, cpe.0, cpe.1, lun + Duration::days(1), at(lun, 12, 0)).await.unwrap();
    assert!(v.incoherences.iter().all(|i| i.eleve_id != lea));
}

// ─── 5. Punitions ≠ sanctions ─────────────────────────────────────────────────

#[tokio::test]
async fn punitions_distinctes_des_sanctions() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let hors_classe = cast.eleves.iter().find(|e| e.classe == "5A").expect("élève de 5A").id;
    let dir = (cast.direction.user_id, "direction");
    // Types : amorçage par la direction seulement.
    assert!(matches!(
        ent.amorcer_types_punition(etab, cast.cpe.user_id, "cpe").await,
        Err(EntDbError::Forbidden)
    ));
    let types = ent.amorcer_types_punition(etab, dir.0, dir.1).await.unwrap();
    assert_eq!(types.len(), 4);
    assert_eq!(ent.amorcer_types_punition(etab, dir.0, dir.1).await.unwrap().len(), 4, "idempotent");
    let retenue = types.iter().find(|t| t.code == "retenue").unwrap().id;

    let avant = compter(&ent, "SELECT count(*) FROM ent_sanction WHERE etablissement_id = $1", etab).await;
    let cmd = |eleve: Uuid| DonnerPunition {
        etablissement_id: etab,
        eleve_id: eleve,
        type_id: retenue,
        jour: today_utc(),
        motif: "Bavardages répétés".into(),
        echeance: Some(today_utc() + Duration::days(3)),
        duree_minutes: Some(60),
        incident_id: None,
    };
    // Le professeur punit un élève qu'il enseigne ; pas un autre.
    let p = ent.donner_punition(cast.prof.user_id, "prof", cmd(lea)).await.unwrap();
    assert_eq!(p.statut, "donnee");
    assert!(matches!(
        ent.donner_punition(cast.prof.user_id, "prof", cmd(hors_classe)).await,
        Err(EntDbError::Forbidden)
    ));
    // La vie scolaire punit tout l'établissement.
    ent.donner_punition(cast.cpe.user_id, "cpe", cmd(hors_classe)).await.unwrap();
    // Une punition n'est jamais une sanction : aucune ligne dans ent_sanction.
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_sanction WHERE etablissement_id = $1", etab).await, avant);
    // Validations : motif vide, échéance antérieure, durée, type inconnu, famille.
    let mut c = cmd(lea);
    c.motif = "  ".into();
    assert!(ent.donner_punition(cast.cpe.user_id, "cpe", c).await.is_err());
    let mut c = cmd(lea);
    c.echeance = Some(today_utc() - Duration::days(1));
    assert!(ent.donner_punition(cast.cpe.user_id, "cpe", c).await.is_err());
    let mut c = cmd(lea);
    c.type_id = Uuid::new_v4();
    assert!(ent.donner_punition(cast.cpe.user_id, "cpe", c).await.is_err());
    assert!(matches!(
        ent.donner_punition(cast.parent_lea.user_id, "parent", cmd(lea)).await,
        Err(EntDbError::Forbidden)
    ));
    // Lecture : famille liée (peut_voir_vs) oui, autre foyer non.
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_vs = true WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(lea).bind(cast.parent_lea.user_id).execute(ent.pool_verif()).await.unwrap();
    assert_eq!(ent.punitions_eleve_for_actor(etab, cast.parent_lea.user_id, "parent", lea).await.unwrap().len(), 1);
    assert!(matches!(
        ent.punitions_eleve_for_actor(etab, cast.parent_tom.user_id, "parent", lea).await,
        Err(EntDbError::Forbidden)
    ));
    // Le professeur ne voit pas les punitions d'un élève hors de ses classes.
    assert!(matches!(
        ent.punitions_eleve_for_actor(etab, cast.prof.user_id, "prof", hors_classe).await,
        Err(EntDbError::Forbidden)
    ));
    // Clôture : effectuée une fois ; une seconde fois refusée.
    let p = ent.clore_punition(etab, p.id, cast.prof.user_id, "prof", "effectuee").await.unwrap();
    assert_eq!(p.statut, "effectuee");
    assert!(matches!(
        ent.clore_punition(etab, p.id, cast.prof.user_id, "prof", "annulee").await,
        Err(EntDbError::Domain(DomainError::Conflit))
    ));
    assert!(ent.clore_punition(etab, p.id, cast.prof.user_id, "prof", "n_importe_quoi").await.is_err());
    // Événement publié en transaction.
    assert!(compter(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = 'vs.punition_donnee'", etab).await >= 2);
    // Isolation : le type d'un autre établissement est refusé.
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    assert!(ent
        .donner_punition(autre.cpe.user_id, "cpe", DonnerPunition { etablissement_id: autre.etablissement_id, eleve_id: autre.eleve_lea_id, type_id: retenue, ..cmd(lea) })
        .await
        .is_err());
}

// ─── 6. Effacement planifié des sanctions ────────────────────────────────────

async fn sanction(ent: &PgEnt, cast: &LaboCast, code: &str, categorie: &str, jour: NaiveDate) -> (Uuid, Uuid) {
    let etab = cast.etablissement_id;
    let typ = ent
        .creer_sanction_type(domain_eleve::CreerReferentielVs {
            etablissement_id: etab,
            code: code.into(),
            libelle: code.into(),
            actor_user_id: None,
        })
        .await
        .unwrap();
    ent.categoriser_type_sanction(etab, cast.direction.user_id, "direction", typ.id, categorie)
        .await
        .unwrap();
    let s = ent
        .prononcer_sanction(domain_eleve::PrononcerSanction {
            etablissement_id: etab,
            eleve_id: cast.eleve_lea_id,
            type_id: typ.id,
            motif_id: None,
            incident_id: None,
            jour,
            duree_minutes: None,
            note: Some("détail confidentiel".into()),
            actor_user_id: cast.direction.user_id,
        })
        .await
        .unwrap();
    (typ.id, s.id)
}

#[tokio::test]
async fn effacement_automatique_des_sanctions() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let d = |y, m, j| NaiveDate::from_ymd_opt(y, m, j).unwrap();
    let dir = (cast.direction.user_id, "direction");
    let (_, avert) = sanction(&ent, &cast, "avert", "avertissement", d(2026, 10, 12)).await;
    let (_, exclu) = sanction(&ent, &cast, "exclu_temp", "exclusion_temporaire_classe", d(2026, 10, 12)).await;
    let (_, defin) = sanction(&ent, &cast, "exclu_def", "exclusion_definitive", d(2026, 10, 12)).await;
    let (_, sans_cat) = {
        let typ = ent
            .creer_sanction_type(domain_eleve::CreerReferentielVs { etablissement_id: etab, code: "libre".into(), libelle: "libre".into(), actor_user_id: None })
            .await
            .unwrap();
        let s = ent
            .prononcer_sanction(domain_eleve::PrononcerSanction { etablissement_id: etab, eleve_id: lea, type_id: typ.id, motif_id: None, incident_id: None, jour: d(2026, 10, 12), duree_minutes: None, note: None, actor_user_id: dir.0 })
            .await
            .unwrap();
        (typ.id, s.id)
    };
    let listes = || ent.list_sanctions_eleve_for_actor(etab, dir.0, dir.1, lea);
    // Le labo a déjà quelques sanctions pour Léa (sans catégorie : jamais effacées).
    let initiales = listes().await.unwrap().len() - 0;
    assert!(initiales >= 4);

    // Veille de la rentrée : rien. Le 1er septembre : l'avertissement.
    let r = ent.effacer_sanctions_echues(etab, d(2027, 8, 31), None).await.unwrap();
    assert!(r.effacees.is_empty());
    let r = ent.effacer_sanctions_echues(etab, d(2027, 9, 1), None).await.unwrap();
    assert_eq!(r.effacees, vec![avert]);
    // Idempotent ; le texte libre est vidé ; la sanction disparaît du dossier.
    assert!(ent.effacer_sanctions_echues(etab, d(2027, 9, 1), None).await.unwrap().effacees.is_empty());
    let (note,): (Option<String>,) = sqlx::query_as("SELECT note FROM ent_sanction WHERE id = $1").bind(avert).fetch_one(ent.pool_verif()).await.unwrap();
    assert!(note.is_none());
    assert!(listes().await.unwrap().iter().all(|s| s.id != avert));
    // Délai en mois (12 mois par défaut pour l'exclusion temporaire) : 12 octobre 2027.
    let r = ent.effacer_sanctions_echues(etab, d(2027, 10, 11), None).await.unwrap();
    assert!(r.effacees.is_empty());
    let r = ent.effacer_sanctions_echues(etab, d(2027, 10, 12), None).await.unwrap();
    assert_eq!(r.effacees, vec![exclu]);
    // Exclusion définitive et type sans catégorie : jamais effacés automatiquement.
    let r = ent.effacer_sanctions_echues(etab, d(2060, 1, 1), None).await.unwrap();
    assert!(r.effacees.is_empty());
    let restantes: Vec<Uuid> = listes().await.unwrap().iter().map(|s| s.id).collect();
    assert!(restantes.contains(&defin) && restantes.contains(&sans_cat));
    // Règle paramétrée : l'exclusion définitive devient effaçable après 120 mois.
    ent.definir_regle_effacement(etab, dir.0, dir.1, "exclusion_definitive", ModeEffacement::DelaiMois(120)).await.unwrap();
    assert!(ent.definir_regle_effacement(etab, dir.0, dir.1, "exclusion_definitive", ModeEffacement::DelaiMois(121)).await.is_err());
    assert!(ent.definir_regle_effacement(etab, dir.0, dir.1, "inconnue", ModeEffacement::Jamais).await.is_err());
    let r = ent.effacer_sanctions_echues(etab, d(2036, 10, 12), None).await.unwrap();
    assert_eq!(r.effacees, vec![defin]);
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = 'vs.sanction_effacee'", etab).await, 3);
    // Un effacement manuel à une date future est refusé (irréversible).
    assert!(ent.effacer_sanctions_echues_for_actor(etab, dir.0, dir.1, today_utc() + Duration::days(400)).await.is_err());
    // Droits : CPE et professeur ne gèrent pas la discipline ; catégorie inconnue refusée.
    assert!(matches!(
        ent.effacer_sanctions_echues_for_actor(etab, cast.cpe.user_id, "cpe", d(2027, 9, 1)).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.categoriser_type_sanction(etab, cast.prof.user_id, "prof", Uuid::new_v4(), "blame").await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent.categoriser_type_sanction(etab, dir.0, dir.1, Uuid::new_v4(), "blame").await.is_err());
    // Isolation : une autre école n'efface rien dans celle-ci.
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    assert!(ent.effacer_sanctions_echues(autre.etablissement_id, d(2060, 1, 1), None).await.unwrap().effacees.is_empty());
}

// ─── 7. Procédure disciplinaire ──────────────────────────────────────────────

#[tokio::test]
async fn procedure_disciplinaire_contradictoire_et_conseil() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let dir = (cast.direction.user_id, "direction");
    let d = |y, m, j| NaiveDate::from_ymd_opt(y, m, j).unwrap();
    let cmd = |instance: &str| EngagerProcedure {
        etablissement_id: etab,
        eleve_id: lea,
        incident_id: None,
        instance: instance.into(),
        faits: "Violence verbale en classe".into(),
        delai_convocation_jours: None,
    };
    // Droits : le CPE et le professeur n'engagent pas de procédure.
    assert!(matches!(ent.engager_procedure(cast.cpe.user_id, "cpe", cmd("chef_etablissement")).await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.engager_procedure(cast.prof.user_id, "prof", cmd("chef_etablissement")).await, Err(EntDbError::Forbidden)));
    assert!(ent.engager_procedure(dir.0, dir.1, cmd("tribunal")).await.is_err());

    // Chef d'établissement : pas de décision sans observations de l'élève.
    let p = ent.engager_procedure(dir.0, dir.1, cmd("chef_etablissement")).await.unwrap();
    assert!(matches!(
        ent.decider_procedure(etab, p.id, dir.0, dir.1, "Avertissement", d(2026, 11, 20), None).await,
        Err(EntDbError::Domain(DomainError::Conflit))
    ));
    // Le conseil ne se convoque pas pour une procédure de chef d'établissement.
    assert!(ent.convoquer_conseil(etab, p.id, dir.0, dir.1, d(2026, 11, 20), d(2026, 12, 20)).await.is_err());
    assert!(ent.recueillir_observations(etab, p.id, dir.0, dir.1, today_utc() + Duration::days(2)).await.is_err());
    let p = ent.recueillir_observations(etab, p.id, dir.0, dir.1, today_utc()).await.unwrap();
    assert!(p.observations_recueillies_le.is_some());
    // La sanction liée doit concerner le même élève.
    let (_, s_autre) = {
        let typ = ent.creer_sanction_type(domain_eleve::CreerReferentielVs { etablissement_id: etab, code: "blame".into(), libelle: "Blâme".into(), actor_user_id: None }).await.unwrap();
        let s = ent.prononcer_sanction(domain_eleve::PrononcerSanction { etablissement_id: etab, eleve_id: cast.eleve_tom_id, type_id: typ.id, motif_id: None, incident_id: None, jour: today_utc(), duree_minutes: None, note: None, actor_user_id: dir.0 }).await.unwrap();
        (typ.id, s.id)
    };
    assert!(ent.decider_procedure(etab, p.id, dir.0, dir.1, "Blâme", today_utc(), Some(s_autre)).await.is_err());
    let p = ent.decider_procedure(etab, p.id, dir.0, dir.1, "Blâme", today_utc(), None).await.unwrap();
    assert_eq!(p.statut, "decidee");
    assert!(ent.decider_procedure(etab, p.id, dir.0, dir.1, "Encore", today_utc(), None).await.is_err(), "décision déjà prise");
    assert!(ent.classer_procedure(etab, p.id, dir.0, dir.1).await.is_err());

    // Conseil de discipline : convocation à 8 jours minimum.
    let p = ent.engager_procedure(dir.0, dir.1, cmd("conseil_discipline")).await.unwrap();
    let envoi = today_utc();
    assert!(ent.convoquer_conseil(etab, p.id, dir.0, dir.1, envoi, envoi + Duration::days(7)).await.is_err(), "7 jours < 8");
    assert!(ent.convoquer_conseil(etab, p.id, dir.0, dir.1, envoi - Duration::days(30), envoi + Duration::days(8)).await.is_err(), "convocation antérieure à l'engagement");
    let p = ent.convoquer_conseil(etab, p.id, dir.0, dir.1, envoi, envoi + Duration::days(8)).await.unwrap();
    assert_eq!((p.statut.as_str(), p.conseil_le), ("convoquee", Some(envoi + Duration::days(8))));
    // Pas de décision avant la date du conseil (dates ramenées dans le passé en base).
    sqlx::query("UPDATE ent_procedure_disciplinaire SET engagee_le = engagee_le - 30, convocation_envoyee_le = convocation_envoyee_le - 30, conseil_le = conseil_le - 30 WHERE id = $1")
        .bind(p.id).execute(ent.pool_verif()).await.unwrap();
    let conseil = envoi + Duration::days(8) - Duration::days(30);
    assert!(matches!(
        ent.decider_procedure(etab, p.id, dir.0, dir.1, "Exclusion", conseil - Duration::days(1), None).await,
        Err(EntDbError::Domain(DomainError::Conflit))
    ));
    // Jamais de décision datée dans le futur.
    assert!(ent.decider_procedure(etab, p.id, dir.0, dir.1, "Exclusion", today_utc() + Duration::days(3), None).await.is_err());
    let p = ent.decider_procedure(etab, p.id, dir.0, dir.1, "Exclusion temporaire", conseil, None).await.unwrap();
    assert_eq!(p.statut, "decidee");
    assert!(compter(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = 'vs.conseil_discipline_decide'", etab).await == 1);
    assert_eq!(ent.procedures_eleve_for_actor(etab, dir.0, dir.1, lea).await.unwrap().len(), 2);
    // Classement sans suite.
    let p3 = ent.engager_procedure(dir.0, dir.1, cmd("chef_etablissement")).await.unwrap();
    assert_eq!(ent.classer_procedure(etab, p3.id, dir.0, dir.1).await.unwrap().statut, "close");
    // Isolation.
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    assert!(ent.classer_procedure(autre.etablissement_id, p3.id, autre.direction.user_id, "direction").await.is_err());
}

// ─── 8. Signalement d'assiduité ──────────────────────────────────────────────

#[tokio::test]
async fn signalement_d_assiduite_cpe_propose_direction_decide() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let tom = cast.eleves.iter().find(|e| e.classe == "4B" && e.id != lea).expect("autre élève de 4B").id;
    let lun = lundi();
    let cpe = (cast.cpe.user_id, Role::Cpe.as_str());
    let dir = (cast.direction.user_id, "direction");
    // Léa : 4 matinées non justifiées dans le mois ; Tom : 3.
    let mut appels = Vec::new();
    for k in 0..4 {
        let jour = lun + Duration::days(k);
        let etats: Vec<(Uuid, LigneEtat)> = if k < 3 { vec![(lea, LigneEtat::Absent), (tom, LigneEtat::Absent)] } else { vec![(lea, LigneEtat::Absent)] };
        let (_, a) = seance_appel(&ent, &cast, jour, (8, 0), (9, 0), &etats, true).await;
        appels.push(a);
    }
    let d = ent.decompte_mensuel_for_actor(etab, cpe.0, cpe.1, lun).await.unwrap();
    let n = |e: Uuid| d.eleves.iter().find(|x| x.eleve_id == e).map(|x| (x.demi_journees_non_justifiees, x.seuil_atteint));
    assert_eq!(n(lea), Some((4, true)));
    assert_eq!(n(tom), Some((3, false)));

    // Le CPE propose : seul Léa, dossier détaillé.
    let props = ent.proposer_signalements(etab, cpe.0, cpe.1, lun).await.unwrap();
    assert_eq!(props.len(), 1);
    assert_eq!((props[0].eleve_id, props[0].demi_journees, props[0].statut.as_str()), (lea, 4, "propose"));
    assert_eq!(props[0].dossier["demi_journees"].as_array().unwrap().len(), 4);
    // Idempotent.
    assert_eq!(ent.proposer_signalements(etab, cpe.0, cpe.1, lun).await.unwrap().len(), 1);
    // Le CPE ne décide pas ; le professeur ne propose pas.
    let id = props[0].id;
    assert!(matches!(ent.decider_signalement(etab, id, cpe.0, cpe.1, "valider").await, Err(EntDbError::Forbidden)));
    assert!(matches!(ent.proposer_signalements(etab, cast.prof.user_id, "prof", lun).await, Err(EntDbError::Forbidden)));
    // Émettre avant de valider : refusé. Puis valider → émettre.
    assert!(matches!(ent.decider_signalement(etab, id, dir.0, dir.1, "emettre").await, Err(EntDbError::Domain(DomainError::Conflit))));
    assert_eq!(ent.decider_signalement(etab, id, dir.0, dir.1, "valider").await.unwrap().statut, "valide");
    assert_eq!(ent.decider_signalement(etab, id, dir.0, dir.1, "emettre").await.unwrap().statut, "emis");
    assert!(ent.decider_signalement(etab, id, dir.0, dir.1, "emettre").await.is_err());
    assert!(ent.decider_signalement(etab, id, dir.0, dir.1, "n_importe_quoi").await.is_err());
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_outbox WHERE etablissement_id = $1 AND type = 'vs.signalement_assiduite_emis'", etab).await, 1);
    // Une proposition décidée n'est plus écrasée par un recalcul.
    sqlx::query("UPDATE ent_absence SET statut = 'justifiee' WHERE id IN (SELECT absence_id FROM ent_absence_seance WHERE appel_id = $1)")
        .bind(appels[0].id).execute(ent.pool_verif()).await.unwrap();
    let props = ent.proposer_signalements(etab, cpe.0, cpe.1, lun).await.unwrap();
    assert_eq!((props[0].statut.as_str(), props[0].demi_journees), ("emis", 4));
    // Après justification d'une matinée : 3 demi-journées non justifiées, seuil non atteint.
    let d = ent.decompte_mensuel_for_actor(etab, cpe.0, cpe.1, lun).await.unwrap();
    assert_eq!(d.eleves.iter().find(|x| x.eleve_id == lea).map(|x| (x.demi_journees_non_justifiees, x.seuil_atteint)), Some((3, false)));
    // Liste du mois ; isolation.
    assert_eq!(ent.signalements_for_actor(etab, cpe.0, cpe.1, lun).await.unwrap().len(), 1);
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    assert!(ent.signalements_for_actor(autre.etablissement_id, autre.cpe.user_id, "cpe", lun).await.unwrap().is_empty());
    assert!(ent.decider_signalement(autre.etablissement_id, id, autre.direction.user_id, "direction", "refuser").await.is_err());
}

// ─── 9. Parcours de bout en bout ─────────────────────────────────────────────

#[tokio::test]
async fn parcours_journee_type_famille_appel_vie_scolaire() {
    let Some((ent, cast)) = labo().await else { return eprintln!("skip: ENT_DATABASE_URL") };
    let etab = cast.etablissement_id;
    let lea = cast.eleve_lea_id;
    let lun = lundi();
    let parent = (cast.parent_lea.user_id, "parent");
    permettre_justifier(&ent, &cast, lea, parent.0, true).await;

    // 1. La famille prévient la veille au soir.
    ent.declarer_absence_famille(parent.0, parent.1, decl(etab, lea, lun, lun)).await.unwrap();
    // 2. L'accueil reçoit aussi un appel d'un responsable pour un autre élève.
    ent.declarer_absence_accueil(
        cast.administratif.user_id,
        "administratif",
        decl(etab, cast.eleve_tom_id, lun, lun),
    )
    .await
    .unwrap();
    // 3. Le professeur ouvre l'appel : Léa est « attendue absente » (A1).
    let c = ent
        .seed_creneau_du_jour(etab, "4B", "Maths", cast.prof.user_id, lun, t(8, 0), t(9, 0), None)
        .await
        .unwrap();
    let (appel, lignes) = ent.ouvrir_appel(etab, c.id, lun, cast.prof.user_id, "prof").await.unwrap();
    let attendus = ent.attendus_absents_appel(etab, appel.id, cast.prof.user_id, "prof").await.unwrap();
    assert!(attendus.contains(&lea));
    assert!(attendus.iter().all(|a| lignes.iter().any(|l| l.eleve_id == *a)));
    // Un autre enseignant n'a pas accès à cette prévision.
    let autre_prof = cast.profs.iter().find(|p| p.user_id != cast.prof.user_id).unwrap();
    assert!(ent.attendus_absents_appel(etab, appel.id, autre_prof.user_id, "prof").await.is_err());
    // 4. Le professeur constate, clôture.
    ent.marquer_appel_ligne(etab, appel.id, cast.prof.user_id, "prof", lea, LigneEtat::Absent).await.unwrap();
    ent.clore_appel(etab, appel.id, cast.prof.user_id, "prof", None).await.unwrap();
    // 5. La vie scolaire voit une absence prévenue, la déclaration de Tom (non constatée),
    //    aucune séance sans appel avant l'heure.
    let cpe = (cast.cpe.user_id, "cpe");
    let tb = ent.tableau_du_jour_for_actor(etab, cpe.0, cpe.1, lun, at(lun, 8, 10)).await.unwrap();
    let a = tb.absents.iter().find(|a| a.eleve_id == lea).expect("Léa absente");
    assert!(a.prevenue && a.statut == "a_traiter");
    assert_eq!(a.seances, vec!["08:00–09:00".to_string()]);
    assert!(tb.declarations.len() >= 2);
    assert!(tb.declarations.iter().any(|d| d.declaration.eleve_id == cast.eleve_tom_id));
    assert!(tb.verification.seances_sans_appel.iter().all(|s| s.creneau_id != c.id));
    assert!(tb.a_traiter >= 1);
    // 6. La famille n'a reçu aucune notification d'« absence constatée » : elle était prévenue.
    distribuer(&ent, etab).await;
    assert_eq!(notifs(&ent, etab, parent.0, "absence_constatee").await, 0);
    // 7. La famille voit les demi-journées de son enfant (droit de lire la vie scolaire).
    sqlx::query("UPDATE ent_eleve_lien SET peut_voir_vs = true WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab).bind(lea).bind(parent.0).execute(ent.pool_verif()).await.unwrap();
    let dj = ent.demi_journees_eleve_for_actor(etab, parent.0, parent.1, lea, lun, lun).await.unwrap();
    assert_eq!(dj.len(), 1);
    assert!(matches!(
        ent.demi_journees_eleve_for_actor(etab, cast.parent_tom.user_id, "parent", lea, lun, lun).await,
        Err(EntDbError::Forbidden)
    ));
    // 8. Le tableau du jour est réservé à la vie scolaire / l'accueil.
    assert!(matches!(
        ent.tableau_du_jour_for_actor(etab, cast.prof.user_id, "prof", lun, at(lun, 9, 0)).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.tableau_du_jour_for_actor(etab, parent.0, parent.1, lun, at(lun, 9, 0)).await,
        Err(EntDbError::Forbidden)
    ));
    ent.tableau_du_jour_for_actor(etab, cast.administratif.user_id, "administratif", lun, at(lun, 9, 0))
        .await
        .unwrap();
    // 9. Isolation : le tableau d'un autre établissement est vide de ces absents.
    let autre = crate::tests_rls_stricte::seed_supplementaire(&ent).await;
    let tb2 = ent
        .tableau_du_jour_for_actor(autre.etablissement_id, autre.cpe.user_id, "cpe", lun, at(lun, 9, 0))
        .await
        .unwrap();
    assert!(tb2.absents.iter().all(|a| a.eleve_id != lea));
    assert!(tb2.declarations.iter().all(|d| d.declaration.etablissement_id == autre.etablissement_id));
}
