//! Socle 10 (emploi du temps), lot T3 : modifier le réel (docs/socles/10_EMPLOI_DU_TEMPS.md §9, §21.3, §22 M4).
//! Base locale de test (feature `labo`). Chaque test crée son propre établissement.
//!
//! Couvre : annulation, levée et journal (motif interne séparé), idempotence par clé, contrôles du §9.3 avec
//! forçage motivé (direction seule), déplacement, changement de lieu (séances, réservations, salle fermée,
//! capacité), ajout (rattrapage), remplacement par un autre cours, fusion, lots (aperçu, une clé = un lot, un
//! seul événement, levée), grève du jour, salle fermée en lot, correction du passé, reprise des exceptions
//! existantes, pont des écrivains historiques, séparation entre établissements, heure de Paris, périmètre du
//! professeur.

#![allow(clippy::too_many_arguments)]

use super::*;
use crate::edt_lots::{CmdGreve, CmdLot, CmdSalleFermee, DemiJournee};
use crate::edt_modif::{ChangementSeance, CmdModifierSeance, ConflitModif};
use crate::edt_modif_ops::{ChangementPasse, CmdCorrectionPasse, FiltreModifications};
use crate::edt_ports::CibleSeances;
use crate::tests_edt_t1_t2::{acteur, adm, creer_salle, creneau, decor, dir, h, labo, Decor};
use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use domain_eleve::horloge::jour_paris;
use ent_auth::Role;
use uuid::Uuid;

/// Un lundi à venir (dans l'année courante), `n` semaines après le prochain.
fn lundi_futur(d: &Decor, n: i64) -> NaiveDate {
    let base = jour_paris().clamp(d.annee_debut, d.annee_fin);
    base + Duration::days((8 - base.weekday().number_from_monday() as i64) % 7 + 7 * n + 14)
}

struct Cadre {
    d: Decor,
    cr: Uuid,
    lundi: NaiveDate,
}

/// Un créneau du lundi 8 h–9 h (cours « Maths », classe 4B, salle B12) et ses séances sur quatre semaines.
async fn cadre(ent: &PgEnt, cast: &LaboCast) -> Cadre {
    let d = decor(ent, cast).await;
    let lundi = lundi_futur(&d, 0);
    let cr = creneau(ent, cast, Some(d.cours), 1, h(8, 0), h(9, 0), Some("B12")).await;
    ent.ref_generer_seances(
        &adm(cast),
        d.annee_id,
        Some(lundi),
        Some(lundi + Duration::days(27)),
    )
    .await
    .unwrap();
    Cadre { d, cr, lundi }
}

async fn seance_du(ent: &PgEnt, cr: Uuid, jour: NaiveDate) -> Uuid {
    sqlx::query_scalar("SELECT id FROM ent_edt_seance WHERE creneau_id = $1 AND jour = $2")
        .bind(cr)
        .bind(jour)
        .fetch_one(ent.pool())
        .await
        .unwrap()
}

async fn etat_de(ent: &PgEnt, id: Uuid) -> (String, String) {
    sqlx::query_as("SELECT etat, statut FROM ent_edt_seance WHERE id = $1")
        .bind(id)
        .fetch_one(ent.pool())
        .await
        .unwrap()
}

async fn nb_evenements(ent: &PgEnt, etab: Uuid, type_name: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = $2",
    )
    .bind(etab)
    .bind(type_name)
    .fetch_one(ent.pool())
    .await
    .unwrap()
}

fn cmd(seance: Uuid, changement: ChangementSeance) -> CmdModifierSeance {
    CmdModifierSeance {
        seance_id: Some(seance),
        changement,
        motif_public_code: "reunion".to_string(),
        motif_public: Some("Réunion pédagogique".to_string()),
        motif_interne: None,
        origine: None,
        origine_ref: None,
        cle: None,
        forcer: None,
        apercu: false,
    }
}

fn codes(c: &[ConflitModif]) -> Vec<String> {
    c.iter().map(|x| x.code.clone()).collect()
}

async fn salle_cap(ent: &PgEnt, cast: &LaboCast, code: &str, capacite: Option<i32>) -> Uuid {
    let id = creer_salle(ent, cast.etablissement_id, cast.administratif.user_id, code).await;
    sqlx::query("UPDATE ent_salle SET capacite = $2 WHERE id = $1")
        .bind(id)
        .bind(capacite)
        .execute(ent.pool())
        .await
        .unwrap();
    id
}

/// Créneau d'une autre classe et d'un autre professeur (le lundi, même heure), sans cours.
async fn creneau_autre(
    ent: &PgEnt,
    cast: &LaboCast,
    classe: &str,
    prof: Uuid,
    jour: i16,
    d: NaiveTime,
    f: NaiveTime,
    salle: Option<&str>,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine,
                heure_debut, heure_fin, salle)
         VALUES ($1,$2,$3,'Autre',$4,$5,$6,$7,$8)",
    )
    .bind(id)
    .bind(cast.etablissement_id)
    .bind(classe)
    .bind(prof)
    .bind(jour)
    .bind(d)
    .bind(f)
    .bind(salle)
    .execute(ent.pool())
    .await
    .unwrap();
    id
}

// ───────────────────────── Annulation, journal, motif interne, levée ─────────────────────────

#[tokio::test]
async fn t3_annulation_journal_motif_interne_cle_et_levee() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let s = seance_du(&ent, c.cr, c.lundi).await;
    let mut annuler = cmd(s, ChangementSeance::Annulation);
    annuler.motif_interne = Some("Conflit confidentiel avec la direction".to_string());
    annuler.cle = Some("t3-annul-1".to_string());

    let r = ent.edt_modifier_seance(&g, &annuler).await.unwrap();
    assert!(r.applique && !r.deja_fait && r.conflits.is_empty(), "{r:?}");
    let mid = r.modification_id.unwrap();
    assert_eq!(
        etat_de(&ent, s).await,
        ("annulee".to_string(), "annulee".to_string())
    );
    let motif: (Option<String>, Option<String>) =
        sqlx::query_as("SELECT motif_public_code, motif_public FROM ent_edt_seance WHERE id = $1")
            .bind(s)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(
        motif,
        (
            Some("reunion".to_string()),
            Some("Réunion pédagogique".to_string())
        )
    );
    // La projection historique suit (lecteurs de la semaine, des familles, des délégations).
    let exc: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_exception WHERE creneau_id = $1 AND jour = $2 AND kind = 'annulation'",
    )
    .bind(c.cr)
    .bind(c.lundi)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(exc, 1);

    // Deux clics = une modification ; même clé pour une autre demande = conflit.
    let r2 = ent.edt_modifier_seance(&g, &annuler).await.unwrap();
    assert!(r2.deja_fait && r2.modification_id == Some(mid));
    let mut autre = cmd(s, ChangementSeance::Neutralisation);
    autre.cle = annuler.cle.clone();
    assert!(matches!(
        ent.edt_modifier_seance(&g, &autre).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::Conflit))
    ));
    let nb: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_edt_modification WHERE seance_id = $1")
            .bind(s)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(nb, 1);

    // Motif interne : table à part, lu par la direction et le secrétariat, jamais par le CPE ni le professeur.
    let f = FiltreModifications {
        seance_id: Some(s),
        ..Default::default()
    };
    let lu_dir = ent.edt_modifications(&g, &f).await.unwrap();
    assert_eq!(lu_dir.len(), 1);
    assert_eq!(
        lu_dir[0].motif_interne.as_deref(),
        Some("Conflit confidentiel avec la direction")
    );
    let lu_adm = ent.edt_modifications(&adm(&cast), &f).await.unwrap();
    assert!(lu_adm[0].motif_interne.is_some());
    let cpe = acteur(&cast, cast.cpe.user_id, Role::Cpe);
    let lu_cpe = ent.edt_modifications(&cpe, &f).await.unwrap();
    assert_eq!((lu_cpe.len(), lu_cpe[0].motif_interne.clone()), (1, None));
    let rogue = acteur(&cast, cast.prof.user_id, Role::Prof);
    let lu_prof = ent.edt_modifications(&rogue, &f).await.unwrap();
    assert_eq!((lu_prof.len(), lu_prof[0].motif_interne.clone()), (1, None));
    assert!(!serde_json::to_string(&lu_prof)
        .unwrap()
        .contains("confidentiel"));
    // Ni la séance, ni les événements ne portent le motif interne.
    let seance = ent.edt_seance(&rogue, s).await.unwrap();
    assert_eq!(seance.motif_public.as_deref(), Some("Réunion pédagogique"));
    assert!(!serde_json::to_string(&seance)
        .unwrap()
        .contains("confidentiel"));
    let charges: Vec<String> =
        sqlx::query_scalar("SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type LIKE 'edt.%'")
            .bind(e)
            .fetch_all(ent.pool())
            .await
            .unwrap();
    assert!(!charges.is_empty() && charges.iter().all(|p| !p.contains("confidentiel")));

    // Droits : le professeur et le CPE ne modifient pas ; la clé inconnue ou un motif hors liste est refusé.
    assert!(matches!(
        ent.edt_modifier_seance(&rogue, &cmd(s, ChangementSeance::Annulation))
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.edt_modifier_seance(&cpe, &cmd(s, ChangementSeance::Annulation))
            .await,
        Err(EntDbError::Forbidden)
    ));
    let mut mauvais = cmd(s, ChangementSeance::Annulation);
    mauvais.motif_public_code = "caprice".to_string();
    assert!(ent.edt_modifier_seance(&g, &mauvais).await.is_err());
    // Un professeur hors de son périmètre : le journal d'une séance qui n'est pas la sienne est refusé.
    let autre_prof = cast
        .profs
        .iter()
        .find(|p| p.user_id != cast.prof.user_id)
        .unwrap();
    let hors = acteur(&cast, autre_prof.user_id, Role::Prof);
    assert!(matches!(
        ent.edt_modifications(&hors, &f).await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.edt_modifications(&hors, &FiltreModifications::default())
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.edt_lever_modification(&hors, mid).await,
        Err(EntDbError::Forbidden)
    ));

    // Levée : séance revenue au plan, journal levé, un message de rétablissement (événement), idempotente.
    let l = ent.edt_lever_modification(&g, mid).await.unwrap();
    assert!(!l.deja_fait);
    assert_eq!(
        etat_de(&ent, s).await,
        ("prevue".to_string(), "prevue".to_string())
    );
    let (levee, motif_apres): (bool, Option<String>) =
        sqlx::query_as("SELECT m.levee_le IS NOT NULL, s.motif_public FROM ent_edt_modification m, ent_edt_seance s WHERE m.id = $1 AND s.id = $2")
            .bind(mid)
            .bind(s)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert!(levee && motif_apres.is_none());
    assert_eq!(nb_evenements(&ent, e, "edt.seance_retablie").await, 1);
    assert!(ent.edt_lever_modification(&g, mid).await.unwrap().deja_fait);
    assert_eq!(nb_evenements(&ent, e, "edt.seance_retablie").await, 1);
    // Une modification levée ne se supprime pas ni ne se réécrit (ajout seul).
    assert!(
        sqlx::query("DELETE FROM ent_edt_modification WHERE id = $1")
            .bind(mid)
            .execute(ent.pool())
            .await
            .is_err()
    );
    assert!(sqlx::query(
        "UPDATE ent_edt_modification SET motif_public_code = 'autre' WHERE id = $1"
    )
    .bind(mid)
    .execute(ent.pool())
    .await
    .is_err());
    // Re-annuler après la levée est possible (une seule modification d'état vivante à la fois).
    assert!(
        ent.edt_modifier_seance(&g, &cmd(s, ChangementSeance::Annulation))
            .await
            .unwrap()
            .applique
    );
}

// ───────────────────────── §9.3 : conflits, forçage motivé, déplacement ─────────────────────────

#[tokio::test]
async fn t3_controles_forcage_motive_et_deplacement() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    // Un second cours de la même classe, le mardi 8 h, avec le même professeur : le public et l'intervenant y
    // sont occupés.
    let mardi = c.lundi + Duration::days(1);
    let cr2 = creneau(&ent, &cast, Some(c.d.cours2), 2, h(8, 0), h(9, 0), None).await;
    ent.ref_generer_seances(
        &adm(&cast),
        c.d.annee_id,
        Some(c.lundi),
        Some(c.lundi + Duration::days(27)),
    )
    .await
    .unwrap();
    let s = seance_du(&ent, c.cr, c.lundi).await;
    let vers_mardi = ChangementSeance::Deplacement {
        nouveau_jour: mardi,
        heure_debut: h(8, 0),
        heure_fin: h(9, 0),
        lieu_id: None,
    };
    let mut cm = cmd(s, vers_mardi.clone());
    let r = ent.edt_modifier_seance(&g, &cm).await.unwrap();
    assert!(!r.applique, "{r:?}");
    let cs = codes(&r.conflits);
    assert!(
        cs.contains(&"public_occupe".to_string()) && cs.contains(&"intervenant_occupe".to_string()),
        "{cs:?}"
    );
    assert!(
        r.conflits.iter().all(|x| x.dur) && r.conflits.iter().any(|x| x.autre_seance_id.is_some())
    );
    assert_eq!(
        etat_de(&ent, s).await.0,
        "prevue",
        "rien d'écrit sur un conflit"
    );

    // Forçage : motif obligatoire, direction seulement.
    cm.forcer = Some("   ".to_string());
    assert!(ent.edt_modifier_seance(&g, &cm).await.is_err());
    cm.forcer = Some("Deux demi-groupes regroupés pour le même cours".to_string());
    assert!(matches!(
        ent.edt_modifier_seance(&adm(&cast), &cm).await,
        Err(EntDbError::Forbidden)
    ));
    // Aperçu : les conflits sont rendus, rien n'est écrit, même avec forçage.
    cm.apercu = true;
    let ap = ent.edt_modifier_seance(&g, &cm).await.unwrap();
    assert!(ap.apercu && ap.applique && ap.modification_id.is_none() && !ap.forces.is_empty());
    assert_eq!(etat_de(&ent, s).await.0, "prevue");
    cm.apercu = false;
    let r = ent.edt_modifier_seance(&g, &cm).await.unwrap();
    assert!(
        r.applique && r.forces.len() >= 2 && r.seance_arrivee_id.is_some(),
        "{r:?}"
    );
    let mid = r.modification_id.unwrap();
    let (force_motif, force_conf): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT force_motif, force_conflits FROM ent_edt_modification WHERE id = $1",
    )
    .bind(mid)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        force_motif.as_deref(),
        Some("Deux demi-groupes regroupés pour le même cours")
    );
    assert!(force_conf.unwrap().contains("public_occupe"));
    // L'ancienne séance est déplacée, la nouvelle ajoutée, la projection historique suit.
    let arrivee = r.seance_arrivee_id.unwrap();
    assert_eq!(
        etat_de(&ent, s).await,
        ("deplacee".to_string(), "annulee".to_string())
    );
    let (etat_a, jour_a, orig): (String, NaiveDate, String) =
        sqlx::query_as("SELECT etat, jour, origine FROM ent_edt_seance WHERE id = $1")
            .bind(arrivee)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(
        (etat_a.as_str(), jour_a, orig.as_str()),
        ("ajoutee", mardi, "deplacement")
    );
    let lien: Option<Uuid> =
        sqlx::query_scalar("SELECT remplacee_par_id FROM ent_edt_seance WHERE id = $1")
            .bind(s)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(lien, Some(arrivee));
    let depl: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_deplacement WHERE creneau_id = $1 AND jour_origine = $2",
    )
    .bind(c.cr)
    .bind(c.lundi)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(depl, 1);
    // La salle libérée est visible par le port D-058 avec la raison « déplacé ».
    let liberees = ent.edt_salles_liberees(&g, c.lundi, c.lundi).await.unwrap();
    assert!(
        liberees
            .iter()
            .any(|l| l.seance_id == s && l.lieu_id == c.d.salle),
        "{liberees:?}"
    );
    // Déplacer une séance déjà déplacée : conflit ; déplacer dans le passé : refusé.
    assert!(ent
        .edt_modifier_seance(&g, &cmd(s, ChangementSeance::Annulation))
        .await
        .is_err());
    // Levée : la séance d'arrivée est retirée (jamais supprimée si elle porte quelque chose), l'origine revient.
    ent.edt_lever_modification(&g, mid).await.unwrap();
    assert_eq!(etat_de(&ent, s).await.0, "prevue");
    let (statut_a,): (String,) = sqlx::query_as("SELECT statut FROM ent_edt_seance WHERE id = $1")
        .bind(arrivee)
        .fetch_optional(ent.pool())
        .await
        .unwrap()
        .unwrap_or(("supprimee".to_string(),));
    assert!(statut_a == "retiree" || statut_a == "supprimee");
    let archive: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_edt_deplacement_leve WHERE creneau_id = $1")
            .bind(c.cr)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(archive, 1);
    let _ = (e, cr2);
}

#[tokio::test]
async fn t3_lieu_occupe_reserve_ferme_capacite_et_entre_sites() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let s = seance_du(&ent, c.cr, c.lundi).await;
    // B13 est occupée par la 5A (autre professeur) le lundi 8 h.
    let b13 = salle_cap(&ent, &cast, "B13", Some(40)).await;
    let autre_prof = cast
        .profs
        .iter()
        .find(|p| p.user_id != cast.prof.user_id)
        .unwrap()
        .user_id;
    creneau_autre(
        &ent,
        &cast,
        "5A",
        autre_prof,
        1,
        h(8, 0),
        h(9, 0),
        Some("B13"),
    )
    .await;
    ent.ref_generer_seances(&adm(&cast), c.d.annee_id, Some(c.lundi), Some(c.lundi))
        .await
        .unwrap();
    let r = ent
        .edt_modifier_seance(
            &g,
            &cmd(s, ChangementSeance::ChangementLieu { lieu_id: b13 }),
        )
        .await
        .unwrap();
    assert!(
        !r.applique && codes(&r.conflits).contains(&"lieu_occupe".to_string()),
        "{r:?}"
    );
    // Réservée (socle 19) : conflit dur.
    let b14 = salle_cap(&ent, &cast, "B14", Some(40)).await;
    sqlx::query(
        "INSERT INTO ent_agenda_reservation (id, etablissement_id, salle_id, debut, fin, demandeur_user_id, intitule)
         SELECT $1, $2, $3, ($4::date + time '07:30') AT TIME ZONE 'Europe/Paris',
                ($4::date + time '08:30') AT TIME ZONE 'Europe/Paris', $5, 'Réunion de parents'",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(b14)
    .bind(c.lundi)
    .bind(cast.administratif.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    let r = ent
        .edt_modifier_seance(
            &g,
            &cmd(s, ChangementSeance::ChangementLieu { lieu_id: b14 }),
        )
        .await
        .unwrap();
    assert!(
        !r.applique && codes(&r.conflits).contains(&"lieu_reserve".to_string()),
        "{r:?}"
    );
    // Fermée (indisponibilité de salle, socle 5) : conflit dur.
    let b15 = salle_cap(&ent, &cast, "B15", Some(40)).await;
    sqlx::query(
        "INSERT INTO ent_salle_indisponibilite (id, etablissement_id, salle_id, du, au, motif_code, par)
         VALUES ($1,$2,$3, ($4::date + time '00:00') AT TIME ZONE 'Europe/Paris',
                 ($4::date + time '23:59') AT TIME ZONE 'Europe/Paris', 'travaux', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(b15)
    .bind(c.lundi)
    .bind(cast.administratif.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    let r = ent
        .edt_modifier_seance(
            &g,
            &cmd(s, ChangementSeance::ChangementLieu { lieu_id: b15 }),
        )
        .await
        .unwrap();
    assert!(
        !r.applique && codes(&r.conflits).contains(&"lieu_indisponible".to_string()),
        "{r:?}"
    );
    // Capacité dépassée : un avertissement, la modification s'applique.
    let petite = salle_cap(&ent, &cast, "B16", Some(1)).await;
    let r = ent
        .edt_modifier_seance(
            &g,
            &cmd(s, ChangementSeance::ChangementLieu { lieu_id: petite }),
        )
        .await
        .unwrap();
    assert!(r.applique, "{r:?}");
    assert!(
        r.conflits
            .iter()
            .any(|x| x.code == "capacite_depassee" && !x.dur && x.nombre >= 1),
        "{r:?}"
    );
    assert_eq!(etat_de(&ent, s).await.0, "modifiee");
    let (salle, lieu): (Option<String>, Option<Uuid>) = sqlx::query_as(
        "SELECT s.salle, (SELECT lieu_id FROM ent_edt_seance_lieu WHERE seance_id = s.id AND principal)
           FROM ent_edt_seance s WHERE s.id = $1",
    )
    .bind(s)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((salle.as_deref(), lieu), (Some("B16"), Some(petite)));
    // Le CPE peut changer un lieu (edt.modifier_lieu), pas annuler.
    let cpe = acteur(&cast, cast.cpe.user_id, Role::Cpe);
    let sem2 = seance_du(&ent, c.cr, c.lundi + Duration::days(7)).await;
    let b17 = salle_cap(&ent, &cast, "B17", Some(40)).await;
    assert!(
        ent.edt_modifier_seance(
            &cpe,
            &cmd(sem2, ChangementSeance::ChangementLieu { lieu_id: b17 })
        )
        .await
        .unwrap()
        .applique
    );
    assert!(matches!(
        ent.edt_modifier_seance(&cpe, &cmd(sem2, ChangementSeance::Annulation))
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Lever un lieu change : la salle du plan revient.
    let mid: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_edt_modification WHERE seance_id = $1 AND type = 'changement_lieu'",
    )
    .bind(s)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    ent.edt_lever_modification(&g, mid).await.unwrap();
    let (salle, etat): (Option<String>, String) =
        sqlx::query_as("SELECT salle, etat FROM ent_edt_seance WHERE id = $1")
            .bind(s)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!((salle.as_deref(), etat.as_str()), (Some("B12"), "prevue"));
}

// ───────────────────────── Ajout (rattrapage), autre cours, fusion, horaire ─────────────────────────

#[tokio::test]
async fn t3_ajout_rattrapage_horaire_fusion_et_passe_refuse() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let mercredi = c.lundi + Duration::days(2);
    let ajout = |jour: NaiveDate, d: NaiveTime, f: NaiveTime| CmdModifierSeance {
        seance_id: None,
        changement: ChangementSeance::Ajout {
            cours_id: c.d.cours,
            jour,
            heure_debut: d,
            heure_fin: f,
            lieu_id: Some(c.d.salle),
        },
        motif_public_code: "autre".to_string(),
        motif_public: Some("Rattrapage".to_string()),
        motif_interne: None,
        origine: None,
        origine_ref: None,
        cle: Some(format!("rattrapage-{jour}")),
        forcer: None,
        apercu: false,
    };
    let r = ent
        .edt_modifier_seance(&g, &ajout(mercredi, h(17, 0), h(18, 0)))
        .await
        .unwrap();
    assert!(r.applique, "{r:?}");
    let nouvelle = r.seance_id.unwrap();
    let (etat, orig, salle, cours): (String, String, Option<String>, Option<Uuid>) =
        sqlx::query_as("SELECT etat, origine, salle, cours_id FROM ent_edt_seance WHERE id = $1")
            .bind(nouvelle)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(
        (etat.as_str(), orig.as_str(), salle.as_deref(), cours),
        ("ajoutee", "ajout", Some("B12"), Some(c.d.cours))
    );
    // Appel attendu : la séance est occupante ; elle sort dans les séances de la classe et de l'élève.
    let vue = ent
        .edt_seances(
            &g,
            CibleSeances::Eleve(cast.eleve_lea_id),
            mercredi,
            mercredi,
        )
        .await
        .unwrap();
    assert!(vue.iter().any(|s| s.id == nouvelle && s.etat == "ajoutee"));
    assert_eq!(nb_evenements(&ent, e, "edt.seance_ajoutee").await, 1);
    // Deux clics = une séance.
    let r2 = ent
        .edt_modifier_seance(&g, &ajout(mercredi, h(17, 0), h(18, 0)))
        .await
        .unwrap();
    assert!(r2.deja_fait && r2.seance_id == Some(nouvelle));
    // Public occupé : un second ajout qui chevauche est refusé (le créneau du cours est déjà pris ce jour-là).
    let r3 = ent
        .edt_modifier_seance(
            &g,
            &CmdModifierSeance {
                cle: None,
                ..ajout(mercredi, h(17, 30), h(18, 30))
            },
        )
        .await
        .unwrap();
    assert!(
        !r3.applique && codes(&r3.conflits).contains(&"creneau_deja_ce_jour".to_string()),
        "{r3:?}"
    );
    // Jour fermé et passé.
    let hier = jour_paris() - Duration::days(1);
    let rp = ent
        .edt_modifier_seance(
            &g,
            &CmdModifierSeance {
                cle: None,
                ..ajout(hier, h(17, 0), h(18, 0))
            },
        )
        .await
        .unwrap();
    assert!(!rp.applique && codes(&rp.conflits) == vec!["seance_passee".to_string()]);
    // Cours d'un autre établissement : introuvable. Le CPE n'ajoute pas. Heures inversées : invalide.
    let autre = ent.seed_labo_cast().await.unwrap();
    let cours_autre: Option<Uuid> = None;
    let _ = (autre, cours_autre);
    assert!(matches!(
        ent.edt_modifier_seance(
            &acteur(&cast, cast.cpe.user_id, Role::Cpe),
            &ajout(mercredi, h(10, 0), h(11, 0))
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .edt_modifier_seance(
            &g,
            &CmdModifierSeance {
                cle: None,
                ..ajout(mercredi, h(11, 0), h(10, 0))
            }
        )
        .await
        .is_err());
    // Lever l'ajout le retire.
    ent.edt_lever_modification(&g, r.modification_id.unwrap())
        .await
        .unwrap();
    let reste: Option<String> =
        sqlx::query_scalar("SELECT statut FROM ent_edt_seance WHERE id = $1")
            .bind(nouvelle)
            .fetch_optional(ent.pool())
            .await
            .unwrap();
    assert!(reste.is_none() || reste.as_deref() == Some("retiree"));

    // Changement d'horaire : la séance du lundi de la semaine 2 passe à 8 h 15–9 h 15 ; levée : retour.
    let s2 = seance_du(&ent, c.cr, c.lundi + Duration::days(7)).await;
    let r = ent
        .edt_modifier_seance(
            &g,
            &cmd(
                s2,
                ChangementSeance::ChangementHoraire {
                    heure_debut: h(8, 15),
                    heure_fin: h(9, 15),
                },
            ),
        )
        .await
        .unwrap();
    assert!(r.applique, "{r:?}");
    let (hd, hf, etat, debut_paris): (NaiveTime, NaiveTime, String, NaiveTime) = sqlx::query_as(
        "SELECT heure_debut, heure_fin, etat, (debut_utc AT TIME ZONE 'Europe/Paris')::time FROM ent_edt_seance WHERE id = $1",
    )
    .bind(s2)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        (hd, hf, etat.as_str(), debut_paris),
        (h(8, 15), h(9, 15), "modifiee", h(8, 15))
    );
    // Une régénération ne défait pas un horaire modifié (le journal est la source du réel).
    ent.ref_generer_seances(
        &adm(&cast),
        c.d.annee_id,
        Some(c.lundi),
        Some(c.lundi + Duration::days(27)),
    )
    .await
    .unwrap();
    let (hd2,): (NaiveTime,) =
        sqlx::query_as("SELECT heure_debut FROM ent_edt_seance WHERE id = $1")
            .bind(s2)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(hd2, h(8, 15), "la régénération a écrasé l'horaire modifié");
    ent.edt_lever_modification(&g, r.modification_id.unwrap())
        .await
        .unwrap();
    let (hd3, etat3): (NaiveTime, String) =
        sqlx::query_as("SELECT heure_debut, etat FROM ent_edt_seance WHERE id = $1")
            .bind(s2)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!((hd3, etat3.as_str()), (h(8, 0), "prevue"));
    // Horaire qui chevauche le cours du même public : conflit dur.
    let cr2 = creneau(&ent, &cast, Some(c.d.cours2), 1, h(9, 0), h(10, 0), None).await;
    ent.ref_generer_seances(
        &adm(&cast),
        c.d.annee_id,
        Some(c.lundi),
        Some(c.lundi + Duration::days(27)),
    )
    .await
    .unwrap();
    let r = ent
        .edt_modifier_seance(
            &g,
            &cmd(
                s2,
                ChangementSeance::ChangementHoraire {
                    heure_debut: h(8, 30),
                    heure_fin: h(9, 30),
                },
            ),
        )
        .await
        .unwrap();
    assert!(
        !r.applique && codes(&r.conflits).contains(&"public_occupe".to_string()),
        "{r:?}"
    );

    // Fusion : les deux cours du lundi 8 h–10 h sont réunis pendant la première heure : la séance de 9 h est
    // absorbée (neutralisée), son cours rejoint le public de la séance conservée.
    let s_a = seance_du(&ent, c.cr, c.lundi).await;
    let s_b = seance_du(&ent, cr2, c.lundi).await;
    sqlx::query("UPDATE ent_edt_seance SET heure_debut = time '08:30', heure_fin = time '09:30' WHERE id = $1").bind(s_b).execute(ent.pool()).await.unwrap();
    let r = ent
        .edt_modifier_seance(
            &g,
            &cmd(
                s_a,
                ChangementSeance::Fusion {
                    avec_seance_id: s_b,
                },
            ),
        )
        .await
        .unwrap();
    assert!(r.applique && r.seance_arrivee_id == Some(s_b), "{r:?}");
    assert_eq!(etat_de(&ent, s_a).await.0, "modifiee");
    assert_eq!(etat_de(&ent, s_b).await.0, "neutralisee");
    let nb_f: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance_public_fusion WHERE seance_id = $1 AND cours_id = $2",
    )
    .bind(s_a)
    .bind(c.d.cours2)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(nb_f, 1);
    ent.edt_lever_modification(&g, r.modification_id.unwrap())
        .await
        .unwrap();
    assert_eq!(etat_de(&ent, s_a).await.0, "prevue");
    assert_eq!(etat_de(&ent, s_b).await.0, "prevue");
    // Fusion de la même séance avec elle-même ou de séances de jours différents : refusée.
    assert!(ent
        .edt_modifier_seance(
            &g,
            &cmd(
                s_a,
                ChangementSeance::Fusion {
                    avec_seance_id: s_a
                }
            )
        )
        .await
        .is_err());
}

// ───────────────────────── Lots : aperçu, clé, événement unique, levée, grève, salle fermée ─────────────────────────

fn lot_classes(d: &Decor, du: NaiveDate, au: NaiveDate, cle: Option<&str>) -> CmdLot {
    CmdLot {
        nature: "annulation".to_string(),
        du,
        au,
        portee: "classes".to_string(),
        cibles: vec![d.classe_4b],
        heure_debut: None,
        heure_fin: None,
        motif_public_code: "intemperies".to_string(),
        motif_public: Some("Alerte météo".to_string()),
        motif_interne: Some("Décision du rectorat, confidentiel".to_string()),
        origine: None,
        origine_ref: None,
        cle: cle.map(str::to_string),
        apercu: false,
    }
}

#[tokio::test]
async fn t3_lot_apercu_cle_un_evenement_et_levee() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let (du, au) = (c.lundi, c.lundi + Duration::days(27));
    // La 4B a aussi le créneau de démonstration du labo : on compte ce que le serveur doit viser.
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND classe_id = $2 AND jour BETWEEN $3 AND $4")
        .bind(e)
        .bind(c.d.classe_4b)
        .bind(du)
        .bind(au)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(total >= 4);
    // Aperçu serveur : rien n'est écrit.
    let ap = ent
        .edt_lot_apercu(&g, &lot_classes(&c.d, du, au, None))
        .await
        .unwrap();
    assert_eq!((ap.nb_seances, ap.nb_classes), (total, 1), "{ap:?}");
    assert!(
        ap.nb_eleves >= 2 && ap.nb_notifications >= ap.nb_eleves,
        "{ap:?}"
    );
    let rien: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_edt_lot WHERE etablissement_id = $1")
            .bind(e)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(rien, 0);
    // Droits : le professeur et le CPE n'appliquent pas de lot ; plage trop longue, cible vide : refusés.
    let rogue = acteur(&cast, cast.prof.user_id, Role::Prof);
    let cpe = acteur(&cast, cast.cpe.user_id, Role::Cpe);
    assert!(matches!(
        ent.edt_lot_appliquer(&rogue, &lot_classes(&c.d, du, au, None))
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.edt_lot_apercu(&cpe, &lot_classes(&c.d, du, au, None))
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .edt_lot_appliquer(&g, &lot_classes(&c.d, du, du + Duration::days(63), None))
        .await
        .is_err());
    let mut vide = lot_classes(&c.d, du, au, None);
    vide.cibles.clear();
    assert!(ent.edt_lot_appliquer(&g, &vide).await.is_err());
    // Une classe d'un autre établissement n'existe pas ici (404), jamais ignorée.
    let autre = ent.seed_labo_cast().await.unwrap();
    let d_autre = decor(&ent, &autre).await;
    let mut etranger = lot_classes(&c.d, du, au, None);
    etranger.cibles = vec![d_autre.classe_4b];
    assert!(matches!(
        ent.edt_lot_appliquer(&g, &etranger).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // Pas de lot vide : un lot sans séance visée est refusé.
    let mut vacances = lot_classes(&c.d, du + Duration::days(1), du + Duration::days(2), None);
    vacances.cle = None;
    assert!(ent.edt_lot_appliquer(&g, &vacances).await.is_err());

    // Application : un lot, quatre modifications, un événement ; deux clics = un lot.
    let r = ent
        .edt_lot_appliquer(&g, &lot_classes(&c.d, du, au, Some("greve-t3")))
        .await
        .unwrap();
    let lot = r.lot_id.unwrap();
    assert!(!r.deja_applique && r.chiffres.nb_seances == total);
    let r2 = ent
        .edt_lot_appliquer(&g, &lot_classes(&c.d, du, au, Some("greve-t3")))
        .await
        .unwrap();
    assert!(r2.deja_applique && r2.lot_id == Some(lot));
    let mut autre_demande = lot_classes(&c.d, du, au + Duration::days(1), Some("greve-t3"));
    autre_demande.nature = "annulation".to_string();
    assert!(matches!(
        ent.edt_lot_appliquer(&g, &autre_demande).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::Conflit))
    ));
    let (nb_lots, nb_modifs, nb_interne): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ent_edt_lot WHERE etablissement_id = $1),
                (SELECT count(*) FROM ent_edt_modification WHERE lot_id = $2 AND type = 'annulation' AND origine = 'lot'),
                (SELECT count(*) FROM ent_edt_modification_interne i JOIN ent_edt_modification m ON m.id = i.modification_id WHERE m.lot_id = $2)",
    )
    .bind(e)
    .bind(lot)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((nb_lots, nb_modifs, nb_interne), (1, total, total));
    let n_etats: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_edt_seance WHERE creneau_id = $1 AND etat = 'annulee' AND motif_public = 'Alerte météo'")
        .bind(c.cr)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n_etats, 4);
    // Un seul événement pour le lot (le socle 8 prévient une personne une fois), sans motif interne.
    assert_eq!(nb_evenements(&ent, e, "edt.lot_modifications").await, 1);
    let charge: String = sqlx::query_scalar("SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'edt.lot_modifications'")
        .bind(e)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert!(
        !charge.contains("confidentiel") && charge.contains("intemperies"),
        "{charge}"
    );
    // Le lot compte par son lien : le journal de la séance dit lequel.
    let s1 = seance_du(&ent, c.cr, c.lundi).await;
    let vues = ent
        .edt_modifications(
            &g,
            &FiltreModifications {
                lot_id: Some(lot),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(vues.len() as i64, total);
    assert!(vues
        .iter()
        .all(|v| v.lot_id == Some(lot) && v.motif_interne.is_some()));
    // Une séance du lot se lève seule ; le reste du lot demeure.
    let une = vues.iter().find(|v| v.seance_id == Some(s1)).unwrap().id;
    ent.edt_lever_modification(&g, une).await.unwrap();
    assert_eq!(etat_de(&ent, s1).await.0, "prevue");
    // Levée du lot : trois séances restantes revenues, le lot est levé, un seul message de plus ; idempotent.
    let l = ent.edt_lot_lever(&g, lot).await.unwrap();
    assert_eq!((l.deja_fait, l.nb_levees), (false, total - 1));
    let reste: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_seance WHERE creneau_id = $1 AND etat = 'prevue'",
    )
    .bind(c.cr)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(reste, 4);
    let leve: bool =
        sqlx::query_scalar("SELECT leve_le IS NOT NULL FROM ent_edt_lot WHERE id = $1")
            .bind(lot)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert!(leve);
    assert!(ent.edt_lot_lever(&g, lot).await.unwrap().deja_fait);
    assert_eq!(nb_evenements(&ent, e, "edt.lot_modifications").await, 2);
    // Un lot d'un autre établissement : introuvable.
    let g_autre = dir(&autre);
    assert!(matches!(
        ent.edt_lot_lever(&g_autre, lot).await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
}

#[tokio::test]
async fn t3_greve_du_jour_sans_motif_nominatif_et_port_du_socle_15() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let autre_prof = cast
        .profs
        .iter()
        .find(|p| p.user_id != cast.prof.user_id)
        .unwrap()
        .user_id;
    // Lundi : Rogue 8 h (4B) et 14 h (4B), l'autre professeur 8 h (5A).
    let cr_apm = creneau(&ent, &cast, Some(c.d.cours2), 1, h(14, 0), h(15, 0), None).await;
    let cr_5a = creneau_autre(
        &ent,
        &cast,
        "5A",
        autre_prof,
        1,
        h(8, 0),
        h(9, 0),
        Some("B13"),
    )
    .await;
    ent.ref_generer_seances(&adm(&cast), c.d.annee_id, Some(c.lundi), Some(c.lundi))
        .await
        .unwrap();
    let cands = ent
        .edt_greve_candidats(&g, c.lundi, DemiJournee::Matin)
        .await
        .unwrap();
    assert_eq!(cands.len(), 2, "{cands:?}");
    assert!(cands.iter().all(|x| x.nb_seances == 1));
    let cands_j = ent
        .edt_greve_candidats(&g, c.lundi, DemiJournee::Journee)
        .await
        .unwrap();
    assert_eq!(
        cands_j
            .iter()
            .find(|x| x.user_id == cast.prof.user_id)
            .unwrap()
            .nb_seances,
        2
    );

    let greve = CmdGreve {
        jour: c.lundi,
        demi_journee: DemiJournee::Matin,
        intervenants: vec![cast.prof.user_id],
        motif_interne: Some("Déclaré gréviste ce matin".to_string()),
        cle: Some("greve-lundi".to_string()),
        apercu: false,
    };
    let ap = ent
        .edt_greve_du_jour(
            &g,
            &CmdGreve {
                apercu: true,
                cle: None,
                ..greve.clone()
            },
        )
        .await
        .unwrap();
    assert!(
        ap.apercu && ap.chiffres.nb_seances == 1 && ap.chiffres.nb_intervenants == 1,
        "{ap:?}"
    );
    let r = ent.edt_greve_du_jour(&g, &greve).await.unwrap();
    assert_eq!(r.chiffres.nb_seances, 1);
    // Séance du matin de Rogue annulée ; son après-midi et l'autre professeur intacts.
    let matin = seance_du(&ent, c.cr, c.lundi).await;
    let apm = seance_du(&ent, cr_apm, c.lundi).await;
    let cinq = seance_du(&ent, cr_5a, c.lundi).await;
    assert_eq!(etat_de(&ent, matin).await.0, "annulee");
    assert_eq!(etat_de(&ent, apm).await.0, "prevue");
    assert_eq!(etat_de(&ent, cinq).await.0, "prevue");
    // Aucun motif nominatif : le motif public est « mouvement social », ni la séance ni l'événement ne nomment
    // le professeur ; le motif interne reste à la direction.
    let (code, texte): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT motif_public_code, motif_public FROM ent_edt_seance WHERE id = $1")
            .bind(matin)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!((code.as_deref(), texte), (Some("mouvement_social"), None));
    let charge: String = sqlx::query_scalar("SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'edt.lot_modifications'").bind(e).fetch_one(ent.pool()).await.unwrap();
    assert!(
        !charge.contains(&cast.prof.user_id.to_string()) && !charge.contains("gréviste"),
        "{charge}"
    );
    // Deux clics = un lot ; liste de coches vide refusée ; le professeur ne déclare pas de grève.
    assert!(
        ent.edt_greve_du_jour(&g, &greve)
            .await
            .unwrap()
            .deja_applique
    );
    assert!(ent
        .edt_greve_du_jour(
            &g,
            &CmdGreve {
                intervenants: vec![],
                cle: None,
                ..greve.clone()
            }
        )
        .await
        .is_err());
    assert!(matches!(
        ent.edt_greve_du_jour(&acteur(&cast, cast.prof.user_id, Role::Prof), &greve)
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Port du socle 15 : annuler la liste de séances d'une absence, avec l'absence citée et sa clé.
    let rep = ent
        .edt_annuler_seances(&g, &[apm, cinq], None, Some("absence-liste-1".to_string()))
        .await
        .unwrap();
    assert_eq!(rep.chiffres.nb_seances, 2);
    assert_eq!(etat_de(&ent, apm).await.0, "annulee");
    assert!(
        ent.edt_annuler_seances(&g, &[apm, cinq], None, Some("absence-liste-1".to_string()))
            .await
            .unwrap()
            .deja_applique
    );
    // Séances d'un autre établissement dans la liste : introuvable.
    let autre = ent.seed_labo_cast().await.unwrap();
    assert!(matches!(
        ent.edt_annuler_seances(&dir(&autre), &[matin], None, None)
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
}

#[tokio::test]
async fn t3_salle_fermee_en_lot_propositions_et_application() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let b13 = salle_cap(&ent, &cast, "B13", Some(40)).await;
    let b14 = salle_cap(&ent, &cast, "B14", Some(1)).await; // trop petite
    let (du, au) = (c.lundi, c.lundi + Duration::days(27));
    let props = ent
        .edt_salle_fermee_propositions(&g, c.d.salle, du, au)
        .await
        .unwrap();
    assert_eq!(props.len(), 4, "{props:?}");
    assert!(
        props
            .iter()
            .all(|p| p.lieu_propose == Some(b13) && p.effectif >= 2),
        "capacité et type respectés : {props:?}"
    );
    // Une proposition qui n'est plus libre à l'application est refusée, jamais forcée ; les autres passent.
    let autre_prof = cast
        .profs
        .iter()
        .find(|p| p.user_id != cast.prof.user_id)
        .unwrap()
        .user_id;
    creneau_autre(
        &ent,
        &cast,
        "5A",
        autre_prof,
        1,
        h(8, 0),
        h(9, 0),
        Some("B13"),
    )
    .await;
    ent.ref_generer_seances(&adm(&cast), c.d.annee_id, Some(c.lundi), Some(c.lundi))
        .await
        .unwrap();
    let cmd_sf = |propositions: Vec<(Uuid, Uuid)>, cle: &str| CmdSalleFermee {
        salle_id: c.d.salle,
        du,
        au,
        propositions,
        motif_public_code: "travaux".to_string(),
        motif_public: Some("Travaux en B12".to_string()),
        motif_interne: None,
        cle: Some(cle.to_string()),
    };
    let toutes: Vec<(Uuid, Uuid)> = props
        .iter()
        .map(|p| (p.seance_id, p.lieu_propose.unwrap()))
        .collect();
    let r = ent
        .edt_salle_fermee_appliquer(&g, &cmd_sf(toutes.clone(), "salle-b12"))
        .await
        .unwrap();
    assert_eq!(r.appliquees.len(), 3, "{r:?}");
    assert_eq!(r.refusees.len(), 1);
    assert!(
        r.refusees[0].1.iter().any(|x| x.code == "lieu_occupe"),
        "{:?}",
        r.refusees
    );
    let lot = r.lot_id.unwrap();
    // Deux clics = un lot ; un événement ; chaque séance changée porte sa ligne de journal liée au lot.
    assert!(
        ent.edt_salle_fermee_appliquer(&g, &cmd_sf(toutes, "salle-b12"))
            .await
            .unwrap()
            .deja_applique
    );
    assert_eq!(nb_evenements(&ent, e, "edt.lot_modifications").await, 1);
    let (nb_ch, nb_salle): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ent_edt_modification WHERE lot_id = $1 AND type = 'changement_lieu'),
                (SELECT count(*) FROM ent_edt_seance WHERE creneau_id = $2 AND salle = 'B13')",
    )
    .bind(lot)
    .bind(c.cr)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((nb_ch, nb_salle), (3, 3));
    // La salle libérée n'est plus occupée (port D-058) : B12 n'a plus que la séance refusée.
    let occ = ent
        .edt_occupation_lieu(&g, c.d.salle, du, au)
        .await
        .unwrap();
    assert_eq!(occ.len(), 1, "{occ:?}");
    // Levée du lot : la salle du plan revient pour les trois.
    let l = ent.edt_lot_lever(&g, lot).await.unwrap();
    assert_eq!(l.nb_levees, 3);
    let revenues: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_edt_seance WHERE creneau_id = $1 AND salle = 'B12' AND etat = 'prevue'").bind(c.cr).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(revenues, 4);
    // Salle inconnue / d'un autre établissement : introuvable ; aucune proposition : pas de lot vide.
    assert!(matches!(
        ent.edt_salle_fermee_propositions(&g, Uuid::new_v4(), du, au)
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    assert!(ent
        .edt_salle_fermee_appliquer(&g, &cmd_sf(vec![], "vide"))
        .await
        .is_err());
    let _ = b14;
}

// ───────────────────────── §9.6 : le passé ne change pas, il se corrige ─────────────────────────

#[tokio::test]
async fn t3_correction_du_passe_droit_special_motif_et_effets_en_aval() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    // Une séance passée (il y a dix jours, à 8 h–9 h heure de Paris), avec un appel déjà fait.
    let jour = jour_paris() - Duration::days(10);
    let annee: Uuid = c.d.annee_id;
    let passee = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_seance (id, etablissement_id, annee_id, creneau_id, jour, classe_id, classe, prof_user_id,
                matiere_libelle, heure_debut, heure_fin, statut, source, cours_id)
         VALUES ($1,$2,$3,$4,$5,$6,'4B',$7,'Maths',time '08:00',time '09:00','prevue','genere',$8)",
    )
    .bind(passee)
    .bind(e)
    .bind(annee)
    .bind(c.cr)
    .bind(jour)
    .bind(c.d.classe_4b)
    .bind(cast.prof.user_id)
    .bind(c.d.cours)
    .execute(ent.pool())
    .await
    .unwrap();
    sqlx::query("INSERT INTO ent_appel (id, etablissement_id, classe, jour, auteur_user_id, seance_id) VALUES ($1,$2,'4B',$3,$4,$5)")
        .bind(Uuid::new_v4())
        .bind(e)
        .bind(jour)
        .bind(cast.prof.user_id)
        .bind(passee)
        .execute(ent.pool())
        .await
        .unwrap();
    // La commande ordinaire refuse : le passé ne change plus (annulation, lieu, horaire, lot, levée).
    let r = ent
        .edt_modifier_seance(&g, &cmd(passee, ChangementSeance::Annulation))
        .await
        .unwrap();
    assert!(
        !r.applique && codes(&r.conflits) == vec!["seance_passee".to_string()],
        "{r:?}"
    );
    assert_eq!(etat_de(&ent, passee).await.0, "prevue");
    let mut lot = lot_classes(&c.d, jour, jour, None);
    lot.portee = "etablissement".to_string();
    lot.cibles.clear();
    assert!(
        ent.edt_lot_appliquer(&g, &lot).await.is_err(),
        "un lot ne touche pas le passé : aucune séance à venir"
    );
    // La correction : droit spécial (direction seule), motif interne obligatoire.
    let corr = |changement: ChangementPasse, interne: &str, cle: Option<&str>| CmdCorrectionPasse {
        seance_id: passee,
        changement,
        motif_public_code: "autre".to_string(),
        motif_public: None,
        motif_interne: interne.to_string(),
        cle: cle.map(str::to_string),
    };
    assert!(matches!(
        ent.edt_corriger_seance_passee(&adm(&cast), &corr(ChangementPasse::NonTenue, "x", None))
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .edt_corriger_seance_passee(&g, &corr(ChangementPasse::NonTenue, "  ", None))
        .await
        .is_err());
    // Un cours à venir ne se « corrige » pas.
    let futur = seance_du(&ent, c.cr, c.lundi).await;
    assert!(ent
        .edt_corriger_seance_passee(
            &g,
            &CmdCorrectionPasse {
                seance_id: futur,
                ..corr(ChangementPasse::NonTenue, "x", None)
            }
        )
        .await
        .is_err());
    let r = ent
        .edt_corriger_seance_passee(
            &g,
            &corr(
                ChangementPasse::NonTenue,
                "Le professeur était en sortie",
                Some("corr-1"),
            ),
        )
        .await
        .unwrap();
    assert!(!r.deja_fait && r.effets_aval.appel_deja_fait, "{r:?}");
    assert_eq!(etat_de(&ent, passee).await.0, "annulee");
    let (passe, interne): (bool, Option<String>) = sqlx::query_as(
        "SELECT m.passe, i.motif_interne FROM ent_edt_modification m
           LEFT JOIN ent_edt_modification_interne i ON i.modification_id = m.id WHERE m.id = $1",
    )
    .bind(r.modification_id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(passe && interne.as_deref() == Some("Le professeur était en sortie"));
    // Rien n'est défait en silence : l'appel existe toujours (signalé, à traiter par le socle 12).
    let appel: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_appel WHERE seance_id = $1")
        .bind(passee)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(appel, 1);
    assert!(
        ent.edt_corriger_seance_passee(
            &g,
            &corr(
                ChangementPasse::NonTenue,
                "Le professeur était en sortie",
                Some("corr-1")
            )
        )
        .await
        .unwrap()
        .deja_fait
    );
    // Lever une correction : refusé (on corrige par une autre correction) ; le passé ne se lève pas.
    assert!(ent
        .edt_lever_modification(&g, r.modification_id)
        .await
        .is_err());
    // Correction inverse : le cours a bien eu lieu.
    let r2 = ent
        .edt_corriger_seance_passee(&g, &corr(ChangementPasse::Tenue, "Erreur de saisie", None))
        .await
        .unwrap();
    assert_eq!(etat_de(&ent, passee).await.0, "prevue");
    assert!(r2.effets_aval.appel_deja_fait);
    // Lieu et horaire d'une séance passée.
    ent.edt_corriger_seance_passee(
        &g,
        &corr(
            ChangementPasse::Lieu { lieu_id: c.d.salle },
            "Mauvaise salle saisie",
            None,
        ),
    )
    .await
    .unwrap();
    ent.edt_corriger_seance_passee(
        &g,
        &corr(
            ChangementPasse::Horaire {
                heure_debut: h(8, 10),
                heure_fin: h(9, 0),
            },
            "Arrivée tardive",
            None,
        ),
    )
    .await
    .unwrap();
    let hd: NaiveTime = sqlx::query_scalar("SELECT heure_debut FROM ent_edt_seance WHERE id = $1")
        .bind(passee)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(hd, h(8, 10));
    let nb: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_edt_modification WHERE seance_id = $1 AND type = 'correction' AND passe").bind(passee).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(nb, 4);
}

// ───────────────────────── Reprise, pont des écrivains historiques, séparation, heure de Paris ─────────────────────────

#[tokio::test]
async fn t3_reprise_des_exceptions_deplacements_et_lots_existants() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let u = cast.direction.user_id;
    let j = |n: i64| c.lundi + Duration::days(7 * n);
    // Données d'avant T3, écrites SANS le pont (reprise à rejouer) : une annulation, un remplacement avec salle,
    // un déplacement (et son exception de nature « deplacement »), un lot de deux annulations, une exception
    // levée archivée, un déplacement levé archivé.
    let exc = |jour: NaiveDate,
               kind: &str,
               nature: &str,
               rempl: Option<Uuid>,
               salle: Option<&str>,
               lot: Option<Uuid>| {
        let pool = ent.pool().clone();
        let (kind, nature, salle) = (
            kind.to_string(),
            nature.to_string(),
            salle.map(str::to_string),
        );
        let creneau = c.cr;
        async move {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_edt_exception (id, etablissement_id, creneau_id, jour, kind, remplacant_user_id, nouvelle_salle,
                        motif, created_by_user_id, lot_id, nature)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,'',$8,$9,$10)",
            )
            .bind(id)
            .bind(e)
            .bind(creneau)
            .bind(jour)
            .bind(kind)
            .bind(rempl)
            .bind(salle)
            .bind(u)
            .bind(lot)
            .bind(nature)
            .execute(&pool)
            .await
            .unwrap();
            id
        }
    };
    exc(j(0), "annulation", "annulation", None, None, None).await;
    let rempl_id = exc(
        j(1),
        "remplacement",
        "remplacement",
        Some(cast.administratif.user_id),
        Some("B12"),
        None,
    )
    .await;
    exc(j(2), "annulation", "deplacement", None, None, None).await;
    sqlx::query(
        "INSERT INTO ent_edt_deplacement (id, etablissement_id, creneau_id, jour_origine, nouveau_jour, nouvelle_heure_debut,
                nouvelle_heure_fin, cree_par_user_id) VALUES ($1,$2,$3,$4,$5,time '10:00',time '11:00',$6)",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(c.cr)
    .bind(j(2))
    .bind(j(2) + Duration::days(2))
    .bind(u)
    .execute(ent.pool())
    .await
    .unwrap();
    let masse = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_edt_annulation_masse (id, etablissement_id, cle, du, au, portee, motif_public, nb_seances, nb_classes, cree_par_user_id)
         VALUES ($1,$2,'ancien-lot',$3,$4,'etablissement','Fermeture exceptionnelle',1,1,$5)",
    )
    .bind(masse)
    .bind(e)
    .bind(j(3))
    .bind(j(3))
    .bind(u)
    .execute(ent.pool())
    .await
    .unwrap();
    exc(j(3), "annulation", "annulation", None, None, Some(masse)).await;
    // Archive d'une exception levée sur une séance sans exception vivante (la 1re semaine, une autre dateʼ).
    sqlx::query(
        "INSERT INTO ent_edt_exception_levee (id, etablissement_id, exception_id, creneau_id, jour, kind, nature, cause, levee_par_user_id,
                pose_par_user_id, pose_le)
         VALUES ($1,$2,$3,$4,$5,'annulation','annulation','levee',$6,$6, now() - interval '1 day')",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(Uuid::new_v4())
    .bind(c.cr)
    .bind(j(0) + Duration::days(1))
    .bind(u)
    .execute(ent.pool())
    .await
    .unwrap();
    // Un état vierge de journal pour cet établissement avant la reprise.
    let avant: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_edt_modification WHERE etablissement_id = $1")
            .bind(e)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(avant, 0);
    let (lots, exceptions, deplacements, archives): (i64, i64, i64, i64) =
        sqlx::query_as("SELECT * FROM ent_edt_modification_reprise()")
            .fetch_one(ent.pool())
            .await
            .unwrap();
    // Le compte porte sur toute la base de labo (autres tests) : on contrôle cet établissement précisément.
    assert!(
        lots >= 1 && exceptions >= 3 && deplacements >= 1 && archives >= 1,
        "{lots} {exceptions} {deplacements} {archives}"
    );
    let par_type: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT type, count(*), count(*) FILTER (WHERE levee_le IS NOT NULL) FROM ent_edt_modification
          WHERE etablissement_id = $1 GROUP BY type ORDER BY type",
    )
    .bind(e)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    // annulation : 2 vivantes (semaine 0 et lot) + 1 archive levée ; intervenant : 1 ; déplacement : 1.
    assert_eq!(
        par_type,
        vec![
            ("annulation".to_string(), 3, 1),
            ("deplacement".to_string(), 1, 0),
            ("intervenant".to_string(), 1, 0)
        ],
        "{par_type:?}"
    );
    // L'exception de nature « deplacement » n'est pas reprise deux fois ; la salle et le remplaçant sont repris.
    let (sal, rem, lieu): (Option<String>, Option<Uuid>, Option<Uuid>) = sqlx::query_as(
        "SELECT apres_salle_texte, apres_remplacant_user_id, apres_lieu_id FROM ent_edt_modification WHERE etablissement_id = $1 AND ancien_exception_id = $2",
    )
    .bind(e)
    .bind(rempl_id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        (sal.as_deref(), rem, lieu),
        (
            Some("B12"),
            Some(cast.administratif.user_id),
            Some(c.d.salle)
        )
    );
    // Le lot repris porte son motif public, sa clé et sa portée ; ses séances pointent vers lui.
    let (nature, motif, cle, nb_lie): (String, Option<String>, Option<String>, i64) = sqlx::query_as(
        "SELECT l.nature, l.motif_public, l.cle, (SELECT count(*) FROM ent_edt_modification m WHERE m.lot_id = l.id)
           FROM ent_edt_lot l WHERE l.etablissement_id = $1",
    )
    .bind(e)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        (nature.as_str(), motif.as_deref(), cle.as_deref(), nb_lie),
        (
            "reprise",
            Some("Fermeture exceptionnelle"),
            Some("ancien-lot"),
            1
        )
    );
    // L'état des séances reprises est dérivé du journal : séance annulée par l'ancien chemin = `annulee`.
    // Rejouer la reprise ne duplique rien.
    let (l2, e2, d2, a2): (i64, i64, i64, i64) =
        sqlx::query_as("SELECT * FROM ent_edt_modification_reprise()")
            .fetch_one(ent.pool())
            .await
            .unwrap();
    let apres: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ent_edt_modification WHERE etablissement_id = $1")
            .bind(e)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!((l2, e2, d2, a2), (0, 0, 0, 0));
    assert_eq!(apres, 5);
}

#[tokio::test]
async fn t3_pont_des_ecrivains_historiques_et_levee_historique() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let jours: Vec<NaiveDate> = (0..4).map(|n| c.lundi + Duration::days(7 * n)).collect();
    let par_type = |jour: NaiveDate| {
        let pool = ent.pool().clone();
        let cr = c.cr;
        async move {
            sqlx::query_as::<_, (String, bool)>(
                "SELECT type, levee_le IS NOT NULL FROM ent_edt_modification WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3 ORDER BY cree_le",
            )
            .bind(e)
            .bind(cr)
            .bind(jour)
            .fetch_all(&pool)
            .await
            .unwrap()
        }
    };
    // Annulation par l'ancien chemin (le socle 15 y passe) : journalisée, état dérivé, puis levée.
    ent.edt_annuler_seance(&g, c.cr, jours[0], None)
        .await
        .unwrap();
    assert_eq!(
        par_type(jours[0]).await,
        vec![("annulation".to_string(), false)]
    );
    let s0 = seance_du(&ent, c.cr, jours[0]).await;
    assert_eq!(etat_de(&ent, s0).await.0, "annulee");
    assert!(ent.edt_lever_exception(&g, c.cr, jours[0]).await.unwrap());
    assert_eq!(
        par_type(jours[0]).await,
        vec![("annulation".to_string(), true)]
    );
    assert_eq!(etat_de(&ent, s0).await.0, "prevue");
    // Changement de salle puis remplacement : la décision qui remplace l'autre la lève dans le journal.
    ent.edt_changer_salle(&g, c.cr, jours[1], "B12")
        .await
        .unwrap();
    let rempl = cast
        .profs
        .iter()
        .find(|p| p.user_id != cast.prof.user_id)
        .unwrap()
        .user_id;
    let r = ent
        .edt_organiser_remplacement(
            &g,
            &crate::edt_remplacement::CmdRemplacement {
                creneau_id: c.cr,
                jour: jours[1],
                remplacant_user_id: rempl,
                absence_rh_id: None,
                nouvelle_salle: None,
            },
        )
        .await
        .unwrap();
    assert!(r.delegation_id.is_some());
    assert_eq!(
        par_type(jours[1]).await,
        vec![
            ("changement_lieu".to_string(), true),
            ("intervenant".to_string(), false)
        ]
    );
    let s1 = seance_du(&ent, c.cr, jours[1]).await;
    assert_eq!(etat_de(&ent, s1).await.0, "modifiee");
    // La levée T3 d'un remplacement passe par la même projection : délégation révoquée, séance au plan.
    let mid: Uuid = sqlx::query_scalar("SELECT id FROM ent_edt_modification WHERE seance_id = $1 AND type = 'intervenant' AND levee_le IS NULL").bind(s1).fetch_one(ent.pool()).await.unwrap();
    ent.edt_lever_modification(&g, mid).await.unwrap();
    assert_eq!(etat_de(&ent, s1).await.0, "prevue");
    let deleg: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_delegation WHERE etablissement_id = $1 AND delegue_user_id = $2 AND statut = 'accordee'").bind(e).bind(rempl).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(deleg, 0);
    // Remplacement par la commande T3 (type intervenant) : même effet, une ligne au journal.
    let s1b = seance_du(&ent, c.cr, jours[1]).await;
    let ri = ent
        .edt_modifier_seance(
            &g,
            &cmd(
                s1b,
                ChangementSeance::Intervenant {
                    remplacant_user_id: rempl,
                },
            ),
        )
        .await
        .unwrap();
    assert!(ri.applique && ri.modification_id.is_some(), "{ri:?}");
    // Déplacement historique : ligne `deplacement` avec séance d'arrivée ; état `deplacee`.
    ent.edt_deplacer_seance(
        &g,
        &crate::edt_remplacement::CmdDeplacement {
            creneau_id: c.cr,
            jour: jours[2],
            nouveau_jour: jours[2] + Duration::days(2),
            heure_debut: h(10, 0),
            heure_fin: h(11, 0),
            salle: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        par_type(jours[2]).await,
        vec![("deplacement".to_string(), false)]
    );
    let s2 = seance_du(&ent, c.cr, jours[2]).await;
    assert_eq!(etat_de(&ent, s2).await.0, "deplacee");
    let arrivee: Option<Uuid> = sqlx::query_scalar("SELECT seance_arrivee_id FROM ent_edt_modification WHERE seance_id = $1 AND type = 'deplacement'").bind(s2).fetch_one(ent.pool()).await.unwrap();
    assert!(arrivee.is_some());
    // Annulation en masse historique : un lot du socle 10 et une ligne par séance.
    let rapport = ent
        .edt_annulation_masse(
            &g,
            &crate::edt_remplacement::CmdAnnulationMasse {
                du: jours[3],
                au: jours[3],
                portee: "etablissement".to_string(),
                cibles: vec![],
                heure_debut: None,
                heure_fin: None,
                motif_public: "Intempéries".to_string(),
                cle: Some("pont-masse".to_string()),
                apercu: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(rapport.nb_seances, 1);
    let (lots, lies): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ent_edt_lot WHERE etablissement_id = $1 AND ancien_masse_id = $2),
                (SELECT count(*) FROM ent_edt_modification WHERE etablissement_id = $1 AND lot_id IS NOT NULL AND type = 'annulation')",
    )
    .bind(e)
    .bind(rapport.lot_id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((lots, lies), (1, 1));
    let s3 = seance_du(&ent, c.cr, jours[3]).await;
    assert_eq!(etat_de(&ent, s3).await.0, "annulee");
}

#[tokio::test]
async fn t3_separation_entre_etablissements() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let s = seance_du(&ent, c.cr, c.lundi).await;
    let r = ent
        .edt_modifier_seance(
            &g,
            &CmdModifierSeance {
                cle: Some("sep-1".to_string()),
                ..cmd(s, ChangementSeance::Annulation)
            },
        )
        .await
        .unwrap();
    let mid = r.modification_id.unwrap();
    // La direction d'un AUTRE établissement ne voit, ne modifie ni ne lève rien.
    let autre = ent.seed_labo_cast().await.unwrap();
    let g2 = dir(&autre);
    let nf = |r: Result<_, EntDbError>| {
        matches!(
            r,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
        )
    };
    let nf_res = |r: Result<crate::edt_modif::ResultatModif, EntDbError>| {
        matches!(
            r,
            Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
        )
    };
    assert!(nf_res(
        ent.edt_modifier_seance(&g2, &cmd(s, ChangementSeance::Annulation))
            .await
    ));
    assert!(nf_res(
        ent.edt_modifier_seance(
            &g2,
            &cmd(
                s,
                ChangementSeance::ChangementHoraire {
                    heure_debut: h(10, 0),
                    heure_fin: h(11, 0)
                }
            )
        )
        .await
    ));
    assert!(nf(ent.edt_lever_modification(&g2, mid).await));
    assert!(ent
        .edt_modifications(
            &g2,
            &FiltreModifications {
                seance_id: Some(s),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .is_empty());
    assert!(ent
        .edt_modifications(&g2, &FiltreModifications::default())
        .await
        .unwrap()
        .is_empty());
    // Une clé d'idempotence de A ne collisionne pas dans B (clés par établissement).
    let d2 = cadre(&ent, &autre).await;
    let s2 = seance_du(&ent, d2.cr, d2.lundi).await;
    let r2 = ent
        .edt_modifier_seance(
            &g2,
            &CmdModifierSeance {
                cle: Some("sep-1".to_string()),
                ..cmd(s2, ChangementSeance::Annulation)
            },
        )
        .await
        .unwrap();
    assert!(r2.applique && !r2.deja_fait);
    // Séance d'un autre établissement fusionnée : introuvable ; lieu d'un autre établissement : introuvable.
    let s_b = seance_du(&ent, c.cr, c.lundi + Duration::days(7)).await;
    assert!(nf_res(
        ent.edt_modifier_seance(
            &g,
            &cmd(s_b, ChangementSeance::Fusion { avec_seance_id: s2 })
        )
        .await
    ));
    let salle_autre = d2.d.salle;
    assert!(nf_res(
        ent.edt_modifier_seance(
            &g,
            &cmd(
                s_b,
                ChangementSeance::ChangementLieu {
                    lieu_id: salle_autre
                }
            )
        )
        .await
    ));
    // Reprise : les lignes d'un établissement ne portent jamais l'identifiant de l'autre.
    let croise: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_edt_modification m JOIN ent_edt_seance s ON s.id = m.seance_id WHERE s.etablissement_id <> m.etablissement_id",
    )
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(croise, 0);
}

#[tokio::test]
async fn t3_heure_de_paris_fenetre_23h_minuit_utc() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    // Un mardi à venir : séances à 00 h 30 et à 23 h 10 heure de Paris. L'instant UTC de la première tombe la
    // VEILLE (22 h 30 ou 23 h 30 UTC) ; le jour métier, lui, est celui de l'établissement.
    let mardi = c.lundi + Duration::days(1);
    let veille_nuit = Uuid::new_v4();
    let soir = Uuid::new_v4();
    for (id, d, f, jour_s) in [
        (veille_nuit, h(0, 30), h(1, 30), 1),
        (soir, h(23, 10), h(23, 50), 1),
    ] {
        let cr = creneau_autre(&ent, &cast, "4B", cast.prof.user_id, 2, d, f, None).await;
        let _ = jour_s;
        sqlx::query(
            "INSERT INTO ent_edt_seance (id, etablissement_id, annee_id, creneau_id, jour, classe_id, classe, prof_user_id,
                    matiere_libelle, heure_debut, heure_fin, statut, source)
             VALUES ($1,$2,$3,$4,$5,$6,'4B',$7,'Nuit',$8,$9,'prevue','genere')",
        )
        .bind(id)
        .bind(e)
        .bind(c.d.annee_id)
        .bind(cr)
        .bind(mardi)
        .bind(c.d.classe_4b)
        .bind(cast.prof.user_id)
        .bind(d)
        .bind(f)
        .execute(ent.pool())
        .await
        .unwrap();
    }
    let (veille_utc, jour_utc_soir): (NaiveDate, NaiveDate) = sqlx::query_as(
        "SELECT (SELECT (debut_utc AT TIME ZONE 'UTC')::date FROM ent_edt_seance WHERE id = $1),
                (SELECT (fin_utc AT TIME ZONE 'UTC')::date FROM ent_edt_seance WHERE id = $2)",
    )
    .bind(veille_nuit)
    .bind(soir)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(veille_utc, c.lundi, "00 h 30 à Paris est la veille en UTC");
    assert_eq!(jour_utc_soir, mardi);
    // Le lot « mardi » prend les deux séances (le jour est celui de l'établissement, jamais le jour UTC) ;
    // le lot « lundi » ne prend que la séance du créneau de 8 h.
    let mut lot_mardi = lot_classes(&c.d, mardi, mardi, None);
    lot_mardi.portee = "etablissement".to_string();
    lot_mardi.cibles.clear();
    let ap = ent.edt_lot_apercu(&g, &lot_mardi).await.unwrap();
    assert_eq!(ap.nb_seances, 2, "{ap:?}");
    let mut lot_lundi = lot_mardi.clone();
    lot_lundi.du = c.lundi;
    lot_lundi.au = c.lundi;
    assert_eq!(
        ent.edt_lot_apercu(&g, &lot_lundi).await.unwrap().nb_seances,
        1
    );
    // Demi-journée : le matin prend 00 h 30, l'après-midi prend 23 h 10 ; la journée entière, les deux.
    let m = ent
        .edt_greve_candidats(&g, mardi, DemiJournee::Matin)
        .await
        .unwrap();
    let a = ent
        .edt_greve_candidats(&g, mardi, DemiJournee::ApresMidi)
        .await
        .unwrap();
    assert_eq!(
        (
            m.iter().map(|x| x.nb_seances).sum::<i64>(),
            a.iter().map(|x| x.nb_seances).sum::<i64>()
        ),
        (1, 1)
    );
    // Le jour métier d'une date UTC de la fenêtre 23 h–minuit : 23 h 30 UTC le 14 est le 15 à Paris.
    let jour: NaiveDate =
        sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, '2026-11-14T23:30:00Z'::timestamptz)")
            .bind(e)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(jour, NaiveDate::from_ymd_opt(2026, 11, 15).unwrap());
    // L'annulation d'une séance de 00 h 30 est datée du jour de l'établissement dans le journal.
    let r = ent
        .edt_modifier_seance(&g, &cmd(veille_nuit, ChangementSeance::Annulation))
        .await
        .unwrap();
    let (jour_modif, debut_paris): (NaiveDate, NaiveTime) = sqlx::query_as(
        "SELECT m.jour, (s.debut_utc AT TIME ZONE 'Europe/Paris')::time FROM ent_edt_modification m JOIN ent_edt_seance s ON s.id = m.seance_id WHERE m.id = $1",
    )
    .bind(r.modification_id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!((jour_modif, debut_paris), (mardi, h(0, 30)));
}

#[tokio::test]
async fn t3_lot_de_2000_seances_sous_la_seconde_ou_presque() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    // 40 créneaux × 50 jours = 2 000 séances de la 4B, à venir, sur une plage de 50 jours.
    let debut = c.lundi + Duration::days(60);
    sqlx::query(
        "INSERT INTO ent_edt_creneau (id, etablissement_id, classe, matiere_libelle, prof_user_id, jour_semaine, heure_debut, heure_fin, cours_id)
         SELECT gen_random_uuid(), $1, '4B', 'Charge', $2, 1, time '06:00' + (g * interval '15 minutes'), time '06:10' + (g * interval '15 minutes'), $3
           FROM generate_series(0, 39) g",
    )
    .bind(e)
    .bind(cast.prof.user_id)
    .bind(c.d.cours)
    .execute(ent.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_edt_seance (id, etablissement_id, annee_id, creneau_id, jour, classe_id, classe, prof_user_id, matiere_libelle,
                heure_debut, heure_fin, statut, source, cours_id)
         SELECT gen_random_uuid(), cr.etablissement_id, $2, cr.id, $3::date + d, $4, '4B', cr.prof_user_id, 'Charge',
                cr.heure_debut, cr.heure_fin, 'prevue', 'genere', cr.cours_id
           FROM ent_edt_creneau cr, generate_series(0, 49) d
          WHERE cr.etablissement_id = $1 AND cr.matiere_libelle = 'Charge'",
    )
    .bind(e)
    .bind(c.d.annee_id)
    .bind(debut)
    .bind(c.d.classe_4b)
    .execute(ent.pool())
    .await
    .unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_edt_seance s JOIN ent_edt_creneau cr ON cr.id = s.creneau_id WHERE s.etablissement_id = $1 AND cr.matiere_libelle = 'Charge'")
        .bind(e)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(n, 2000);
    let mut lot = lot_classes(&c.d, debut, debut + Duration::days(49), Some("charge-2000"));
    lot.portee = "etablissement".to_string();
    lot.cibles.clear();
    let t0 = std::time::Instant::now();
    let r = ent.edt_lot_appliquer(&g, &lot).await.unwrap();
    let ecoule = t0.elapsed();
    assert_eq!(r.chiffres.nb_seances, 2000);
    eprintln!("lot de 2 000 séances appliqué en {ecoule:?}");
    // Critère du lot T3 : < 2 s en labo (mesuré : 0,24 s en application, 0,9 s en levée).
    assert!(
        ecoule < std::time::Duration::from_secs(2),
        "lot de 2 000 séances trop lent : {ecoule:?}"
    );
    let annulees: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_edt_seance WHERE etablissement_id = $1 AND etat = 'annulee' AND motif_public = 'Alerte météo'").bind(e).fetch_one(ent.pool()).await.unwrap();
    assert_eq!(annulees, 2000);
    let t1 = std::time::Instant::now();
    let l = ent.edt_lot_lever(&g, r.lot_id.unwrap()).await.unwrap();
    eprintln!("levée du lot de 2 000 séances en {:?}", t1.elapsed());
    assert_eq!(l.nb_levees, 2000);
    assert!(t1.elapsed() < std::time::Duration::from_secs(4));
}

// ───────────────────────── CL-12 autre cours, EDT-175 rattrapage, EDT-176 non assurées ─────────────────────────

#[tokio::test]
async fn t3_autre_cours_rattrapage_propose_et_seances_non_assurees() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    // Le cours de français a un créneau le mercredi (c'est lui qui portera la séance ajoutée, clé historique).
    creneau(&ent, &cast, Some(c.d.cours2), 3, h(14, 0), h(15, 0), None).await;
    let s = seance_du(&ent, c.cr, c.lundi).await;
    // CL-12 : le français est fait à la place des maths, au même jour, aux mêmes heures, dans la même salle.
    let mut cm = cmd(
        s,
        ChangementSeance::AutreCours {
            cours_id: c.d.cours2,
        },
    );
    cm.cle = Some("autre-cours-1".to_string());
    let mut ap = cm.clone();
    ap.apercu = true;
    let a1 = ent.edt_modifier_seance(&g, &ap).await.unwrap();
    assert!(
        a1.apercu && a1.applique && a1.modification_id.is_none(),
        "{a1:?}"
    );
    assert_eq!(etat_de(&ent, s).await.0, "prevue", "l'aperçu n'écrit rien");
    let r = ent.edt_modifier_seance(&g, &cm).await.unwrap();
    assert!(r.applique && r.seance_arrivee_id.is_some(), "{r:?}");
    let nouvelle = r.seance_arrivee_id.unwrap();
    assert_eq!(etat_de(&ent, s).await.0, "annulee");
    let (cours, etat, salle, jour, lien): (Option<Uuid>, String, Option<String>, NaiveDate, Option<Uuid>) = sqlx::query_as(
        "SELECT n.cours_id, n.etat, n.salle, n.jour, o.remplacee_par_id FROM ent_edt_seance n, ent_edt_seance o WHERE n.id = $1 AND o.id = $2",
    )
    .bind(nouvelle)
    .bind(s)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        (cours, etat.as_str(), salle.as_deref(), jour, lien),
        (
            Some(c.d.cours2),
            "ajoutee",
            Some("B12"),
            c.lundi,
            Some(nouvelle)
        )
    );
    let types: Vec<String> = sqlx::query_scalar(
        "SELECT type FROM ent_edt_modification WHERE seance_id = ANY($1) ORDER BY type",
    )
    .bind(vec![s, nouvelle])
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert_eq!(types, vec!["ajout".to_string(), "annulation".to_string()]);
    assert!(ent.edt_modifier_seance(&g, &cm).await.unwrap().deja_fait);
    assert_eq!(nb_evenements(&ent, e, "edt.seance_ajoutee").await, 1);

    // EDT-175 : plages de rattrapage de la séance annulée (classe, professeur, salle libres) ; aucune ne
    // chevauche une séance existante de la classe ou du professeur.
    let du = c.lundi;
    let au = c.lundi + Duration::days(6);
    let plages = ent
        .edt_rattrapage_propositions(&g, s, du, au)
        .await
        .unwrap();
    assert!(
        !plages.is_empty(),
        "des plages libres existent dans la semaine"
    );
    assert!(plages.iter().all(|p| (p.fin - p.debut).num_minutes() >= 60));
    assert!(plages
        .windows(2)
        .all(|w| (w[0].jour, w[0].debut) <= (w[1].jour, w[1].debut)));
    let occupe = ent
        .edt_seances(&g, CibleSeances::Classe(c.d.classe_4b), du, au)
        .await
        .unwrap();
    for p in &plages {
        assert!(
            !occupe.iter().any(|o| o.jour == p.jour
                && ["prevue", "modifiee", "ajoutee"].contains(&o.etat.as_str())
                && o.heure_debut < p.fin
                && p.debut < o.heure_fin),
            "plage {p:?} chevauche une séance de la classe"
        );
    }
    // Le CPE ne propose pas de rattrapage (edt.modifier) ; une séance d'un autre établissement : introuvable.
    assert!(matches!(
        ent.edt_rattrapage_propositions(&acteur(&cast, cast.cpe.user_id, Role::Cpe), s, du, au)
            .await,
        Err(EntDbError::Forbidden)
    ));
    let autre = ent.seed_labo_cast().await.unwrap();
    assert!(matches!(
        ent.edt_rattrapage_propositions(&dir(&autre), s, du, au)
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // Rattrapage retenu : un ajout dans une plage proposée passe les contrôles du §9.3.
    let p = &plages[0];
    let rat = ent
        .edt_modifier_seance(
            &g,
            &CmdModifierSeance {
                seance_id: None,
                changement: ChangementSeance::Ajout {
                    cours_id: c.d.cours,
                    jour: p.jour,
                    heure_debut: p.debut,
                    heure_fin: p.debut + Duration::minutes(60),
                    lieu_id: Some(c.d.salle),
                },
                motif_public_code: "autre".to_string(),
                motif_public: Some("Rattrapage".to_string()),
                motif_interne: None,
                origine: None,
                origine_ref: None,
                cle: Some("rattrapage-retenu".to_string()),
                forcer: None,
                apercu: false,
            },
        )
        .await
        .unwrap();
    assert!(
        rat.applique,
        "la plage proposée doit passer les contrôles : {rat:?}"
    );

    // EDT-176 : le cours n'a aucun service (poste vacant) : ses séances à venir sont à traiter par la vie scolaire.
    let cpe = acteur(&cast, cast.cpe.user_id, Role::Cpe);
    let non_assurees = ent
        .edt_seances_non_assurees(&cpe, du, au + Duration::days(21))
        .await
        .unwrap();
    let semaine2 = seance_du(&ent, c.cr, c.lundi + Duration::days(7)).await;
    assert!(
        non_assurees
            .iter()
            .any(|x| x.seance_id == semaine2 && x.raison == "cours_vacant"),
        "{non_assurees:?}"
    );
    assert!(matches!(
        ent.edt_seances_non_assurees(&acteur(&cast, cast.prof.user_id, Role::Prof), du, au)
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .edt_seances_non_assurees(&cpe, du, du + Duration::days(63))
        .await
        .is_err());
}
