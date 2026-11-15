#![allow(clippy::too_many_arguments)]
//! Socle 10 (emploi du temps), lot T4 : notifications par le socle 8, « demain » et « maintenant »
//! (docs/socles/10_EMPLOI_DU_TEMPS.md §9.5, §14, §18.1). Base locale de test (feature `labo`).
//!
//! L'heure de décision d'une modification est celle de son événement : les tests la fixent (`created_at` de
//! l'événement) pour rejouer « décidé à 17 h pour demain 8 h » sans dépendre de l'heure du serveur de test.

use super::*;
use crate::edt_demain::Maintenant;
use crate::edt_lots::CmdLot;
use crate::edt_modif::{ChangementSeance, CmdModifierSeance};
use crate::outbox::ConfigDistribution;
use crate::tests_edt_t1_t2::{acteur, adm, creneau, decor, dir, h, labo, Decor};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use domain_eleve::horloge::jour_paris;
use ent_auth::Role;
use uuid::Uuid;

fn lundi_futur(d: &Decor, n: i64) -> NaiveDate {
    let base = jour_paris().clamp(d.annee_debut, d.annee_fin);
    base + Duration::days((8 - base.weekday().number_from_monday() as i64) % 7 + 7 * n + 14)
}

struct Cadre {
    d: Decor,
    cr: Uuid,
    lundi: NaiveDate,
}

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

fn cmd(seance: Uuid, changement: ChangementSeance) -> CmdModifierSeance {
    CmdModifierSeance {
        seance_id: Some(seance),
        changement,
        motif_public_code: "intemperies".to_string(),
        motif_public: None,
        motif_interne: Some("détail interne confidentiel".to_string()),
        origine: None,
        origine_ref: None,
        cle: None,
        forcer: None,
        apercu: false,
    }
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

/// Fixe l'instant de décision des événements `type_name` de l'établissement (avant distribution).
async fn dater_evenements(ent: &PgEnt, etab: Uuid, type_name: &str, instant: DateTime<Utc>) {
    sqlx::query(
        "UPDATE ent_metier_event SET created_at = $3
          WHERE etablissement_id = $1 AND type = $2 AND id IN
                (SELECT e.id FROM ent_metier_event e LEFT JOIN ent_event_traitement t ON t.event_id = e.id
                  WHERE e.etablissement_id = $1 AND e.type = $2 AND t.event_id IS NULL)",
    )
    .bind(etab)
    .bind(type_name)
    .bind(instant)
    .execute(ent.pool())
    .await
    .unwrap();
}

/// Heure murale de Paris → instant UTC (calculé par PostgreSQL : aucune dépendance de fuseau côté test).
async fn paris(ent: &PgEnt, jour: NaiveDate, heure: NaiveTime) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT ($1::date + $2::time) AT TIME ZONE 'Europe/Paris'")
        .bind(jour)
        .bind(heure)
        .fetch_one(ent.pool())
        .await
        .unwrap()
}

#[derive(sqlx::FromRow, Debug, Clone)]
struct Notif {
    kind: String,
    titre: String,
    corps: String,
}

async fn notifs(ent: &PgEnt, user: Uuid, kind_prefixe: &str) -> Vec<Notif> {
    sqlx::query_as(
        "SELECT kind, titre, corps FROM ent_notification WHERE destinataire_user_id = $1 AND kind LIKE $2 || '%' ORDER BY created_at, id",
    )
    .bind(user)
    .bind(kind_prefixe)
    .fetch_all(ent.pool())
    .await
    .unwrap()
}

#[derive(sqlx::FromRow, Debug, Clone)]
struct Envoi {
    #[allow(dead_code)]
    canal: String,
    planifie_pour: DateTime<Utc>,
    regroupable: bool,
    cle_dedup: String,
}

async fn envois(ent: &PgEnt, user: Uuid) -> Vec<Envoi> {
    sqlx::query_as(
        "SELECT canal, planifie_pour, regroupable, cle_dedup FROM ent_notification_envoi
          WHERE destinataire_user_id = $1 AND cle_dedup LIKE 'edt:%' ORDER BY planifie_pour, cle_dedup, canal",
    )
    .bind(user)
    .fetch_all(ent.pool())
    .await
    .unwrap()
}

// ───────────────────────── Annulation de demain 8 h décidée à 17 h ─────────────────────────

#[tokio::test]
async fn t4_annulation_de_demain_decidee_a_17h_veille_rappel_et_retablissement() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let s = seance_du(&ent, c.cr, c.lundi).await;
    let veille = c.lundi - Duration::days(1);
    let r = ent
        .edt_modifier_seance(&g, &cmd(s, ChangementSeance::Annulation))
        .await
        .unwrap();
    assert!(r.applique);
    // Décidée la veille à 17 h (heure de Paris) pour 8 h le lendemain.
    dater_evenements(
        &ent,
        e,
        "edt.seance_annulee",
        paris(&ent, veille, h(17, 0)).await,
    )
    .await;
    dater_evenements(
        &ent,
        e,
        "edt.exception",
        paris(&ent, veille, h(17, 0)).await,
    )
    .await;
    distribuer(&ent, e).await;
    let harry = &cast.eleves[0];
    let compte = harry.compte.as_ref().unwrap().user_id;
    // L'élève et sa famille sont prévenus (public à la date) ; le professeur aussi ; jamais un autre élève.
    let n_el = notifs(&ent, compte, "edt_annulation").await;
    let n_par = notifs(&ent, harry.parent.user_id, "edt_annulation").await;
    let n_prof = notifs(&ent, cast.prof.user_id, "edt_annulation").await;
    assert_eq!(
        (n_el.len(), n_par.len(), n_prof.len()),
        (1, 1, 1),
        "{n_el:?} {n_par:?} {n_prof:?}"
    );
    let neville = cast.eleves.iter().find(|x| x.prenom == "Neville").unwrap();
    assert!(
        notifs(&ent, neville.parent.user_id, "edt_")
            .await
            .is_empty(),
        "classe 3C non concernée"
    );
    // Le motif public est donné, jamais le motif interne ; un seul message (l'alias historique ne double pas).
    assert!(
        n_el[0].corps.contains("intempéries") && !n_el[0].corps.contains("confidentiel"),
        "{}",
        n_el[0].corps
    );
    assert!(n_el[0].titre.starts_with("Cours annulé"));
    // Urgence U2 : message de la veille à 18 h 30, rappelé le matin à 6 h 30 ; jamais regroupé.
    let ev = envois(&ent, compte).await;
    assert!(
        !ev.is_empty(),
        "l'élève a un e-mail : un envoi est planifié"
    );
    let heure_veille = paris(&ent, veille, h(18, 30)).await;
    let heure_rappel = paris(&ent, c.lundi, h(6, 30)).await;
    let premiers: Vec<&Envoi> = ev
        .iter()
        .filter(|x| !x.cle_dedup.ends_with(":rappel"))
        .collect();
    let rappels: Vec<&Envoi> = ev
        .iter()
        .filter(|x| x.cle_dedup.ends_with(":rappel"))
        .collect();
    assert!(
        premiers
            .iter()
            .all(|x| x.planifie_pour == heure_veille && !x.regroupable),
        "{premiers:?}"
    );
    assert!(
        !rappels.is_empty() && rappels.iter().all(|x| x.planifie_pour == heure_rappel),
        "{rappels:?}"
    );
    // Rétablissement : levée décidée à 20 h ; seuls les prévenus reçoivent le message (même urgence : tout de suite).
    let mid: Uuid = sqlx::query_scalar(
        "SELECT id FROM ent_edt_modification WHERE seance_id = $1 AND type = 'annulation'",
    )
    .bind(s)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    ent.edt_lever_modification(&g, mid).await.unwrap();
    dater_evenements(
        &ent,
        e,
        "edt.seance_retablie",
        paris(&ent, veille, h(20, 0)).await,
    )
    .await;
    distribuer(&ent, e).await;
    for u in [compte, harry.parent.user_id, cast.prof.user_id] {
        let r = notifs(&ent, u, "edt_retabli").await;
        assert_eq!(r.len(), 1, "{u}: {r:?}");
        assert!(r[0].corps.contains("aura lieu normalement"));
    }
    assert!(notifs(&ent, neville.parent.user_id, "edt_retabli")
        .await
        .is_empty());
    // Rejeu de la distribution : rien en double.
    distribuer(&ent, e).await;
    assert_eq!(notifs(&ent, compte, "edt_").await.len(), 2);
    // Séparation entre établissements : l'autre école ne reçoit rien.
    let autre = ent.seed_labo_cast().await.unwrap();
    for p in [&autre.parent_lea, &autre.prof, &autre.direction] {
        assert!(notifs(&ent, p.user_id, "edt_").await.is_empty());
    }
}

#[tokio::test]
async fn t4_heure_de_paris_decision_a_minuit_vingt_en_heure_locale() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let s = seance_du(&ent, c.cr, c.lundi).await;
    ent.edt_modifier_seance(&g, &cmd(s, ChangementSeance::Annulation))
        .await
        .unwrap();
    // Décision à 23 h 20 UTC, deux jours avant la séance : il est déjà 0 h 20 (ou 1 h 20) la veille à Paris, la
    // séance de 8 h est donc « demain avant 10 h » : urgente, message de la veille à 18 h 30. Un calcul en jour UTC
    // y verrait une séance dans deux jours, ordinaire, regroupée.
    let instant = Utc.from_utc_datetime(&(c.lundi - Duration::days(2)).and_time(h(23, 20)));
    dater_evenements(&ent, e, "edt.seance_annulee", instant).await;
    dater_evenements(&ent, e, "edt.exception", instant).await;
    distribuer(&ent, e).await;
    let compte = cast.eleves[0].compte.as_ref().unwrap().user_id;
    let ev = envois(&ent, compte).await;
    assert!(!ev.is_empty());
    let veille = c.lundi - Duration::days(1);
    let heure_veille = paris(&ent, veille, h(18, 30)).await;
    assert!(
        ev.iter()
            .filter(|x| !x.cle_dedup.ends_with(":rappel"))
            .all(|x| !x.regroupable && x.planifie_pour == heure_veille),
        "{ev:?}"
    );
    // Décision à 17 h 45 UTC la veille (18 h 45 ou 19 h 45 à Paris, après l'heure du message de la veille) : tout
    // de suite, pas de report (un calcul en UTC la verrait avant 18 h 30).
    let s2 = seance_du(&ent, c.cr, c.lundi + Duration::days(7)).await;
    ent.edt_modifier_seance(&g, &cmd(s2, ChangementSeance::Annulation))
        .await
        .unwrap();
    let instant2 = Utc.from_utc_datetime(&(c.lundi + Duration::days(6)).and_time(h(17, 45)));
    dater_evenements(&ent, e, "edt.seance_annulee", instant2).await;
    dater_evenements(&ent, e, "edt.exception", instant2).await;
    distribuer(&ent, e).await;
    let ev2: Vec<Envoi> = envois(&ent, compte)
        .await
        .into_iter()
        .filter(|x| x.planifie_pour == instant2)
        .collect();
    assert!(
        !ev2.is_empty(),
        "envoi immédiat attendu : {:?}",
        envois(&ent, compte).await
    );
}

// ───────────────────────── Un lot : un seul message par personne ─────────────────────────

#[tokio::test]
async fn t4_un_lot_un_seul_message_par_personne_et_retablissement_des_seuls_prevenus() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let (du, au) = (c.lundi, c.lundi + Duration::days(27));
    let lot = CmdLot {
        nature: "annulation".to_string(),
        du,
        au,
        portee: "classes".to_string(),
        cibles: vec![c.d.classe_4b],
        heure_debut: Some(h(8, 0)),
        heure_fin: Some(h(9, 0)),
        motif_public_code: "intemperies".to_string(),
        motif_public: Some("Alerte météo".to_string()),
        motif_interne: Some("interne confidentiel".to_string()),
        origine: None,
        origine_ref: None,
        cle: Some("lot-t4".to_string()),
        apercu: false,
    };
    let r = ent.edt_lot_appliquer(&g, &lot).await.unwrap();
    let n = r.chiffres.nb_seances;
    assert!(n >= 4);
    distribuer(&ent, e).await;
    let harry = &cast.eleves[0];
    let compte = harry.compte.as_ref().unwrap().user_id;
    // Quatre séances, une seule notification par personne, avec le nombre de séances qui la concernent.
    for u in [compte, harry.parent.user_id, cast.prof.user_id] {
        let v = notifs(&ent, u, "edt_annulation_masse").await;
        assert_eq!(v.len(), 1, "{u}: {v:?}");
        assert!(
            v[0].corps.contains("annulés")
                && v[0].corps.contains("Alerte météo")
                && !v[0].corps.contains("confidentiel"),
            "{:?}",
            v[0]
        );
    }
    assert!(
        notifs(&ent, compte, "edt_annulation")
            .await
            .iter()
            .all(|x| x.kind == "edt_annulation_masse"),
        "pas de message par séance"
    );
    let neville = cast.eleves.iter().find(|x| x.prenom == "Neville").unwrap();
    assert!(notifs(&ent, neville.parent.user_id, "edt_")
        .await
        .is_empty());
    // Ordinaire dans trois semaines : regroupé dans le résumé de la personne (aucun envoi immédiat).
    assert!(envois(&ent, compte).await.iter().all(|x| x.regroupable));
    // Levée du lot : un seul message de rétablissement, à ceux qui avaient été prévenus.
    ent.edt_lot_lever(&g, r.lot_id.unwrap()).await.unwrap();
    distribuer(&ent, e).await;
    for u in [compte, harry.parent.user_id, cast.prof.user_id] {
        assert_eq!(notifs(&ent, u, "edt_retabli").await.len(), 1);
    }
    assert!(notifs(&ent, neville.parent.user_id, "edt_retabli")
        .await
        .is_empty());
    // Un lot levé avant la distribution de sa pose ne laisse aucun message périmé.
    let lot2 = CmdLot {
        cle: Some("lot-t4-bis".to_string()),
        du: du + Duration::days(7),
        au: du + Duration::days(7),
        ..lot.clone()
    };
    let r2 = ent.edt_lot_appliquer(&g, &lot2).await.unwrap();
    ent.edt_lot_lever(&g, r2.lot_id.unwrap()).await.unwrap();
    let avant = notifs(&ent, compte, "edt_").await.len();
    distribuer(&ent, e).await;
    assert_eq!(notifs(&ent, compte, "edt_").await.len(), avant);
}

// ───────────────────────── Public daté : inclusions, exclusions, majorité, interdictions ─────────────────────────

#[tokio::test]
async fn t4_destinataires_public_a_la_date_inclusion_exclusion_majeur_et_interdiction() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let s = seance_du(&ent, c.cr, c.lundi).await;
    let neville = cast.eleves.iter().find(|x| x.prenom == "Neville").unwrap(); // 3C : hors du cours
    let hermione = cast.eleves.iter().find(|x| x.prenom == "Hermione").unwrap(); // 4B
    let harry = &cast.eleves[0]; // 4B
                                 // Inclusion individuelle (ULIS) de Neville à la date ; exclusion d'Hermione (dispense) à la date.
    for (eleve, nature, motif) in [
        (neville.id, "inclusion", "ulis"),
        (hermione.id, "exclusion", "dispense_cours"),
    ] {
        sqlx::query(
            "INSERT INTO ent_cours_membre (id, etablissement_id, cours_id, eleve_id, nature, motif_code, du, au)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(Uuid::new_v4())
        .bind(e)
        .bind(c.d.cours)
        .bind(eleve)
        .bind(nature)
        .bind(motif)
        .bind(c.lundi)
        .bind(c.lundi)
        .execute(ent.pool())
        .await
        .unwrap();
    }
    // Harry devient majeur : ses responsables ne sont plus prévenus (R-E32), lui oui.
    {
        // La date de naissance passe par le canal de projection de la garde d'identité (seule écriture directe admise).
        let mut tx = ent.pool().begin().await.unwrap();
        sqlx::query("SELECT set_config('ent.identite_projection', 'on', true)")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE ent_eleve SET date_naissance = $2 WHERE id = $1")
            .bind(harry.id)
            .bind(c.lundi - Duration::days(365 * 19))
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    // Une interdiction de communication visant le responsable d'Hermione : il ne reçoit rien.
    let personne_h: Uuid = sqlx::query_scalar(
        "SELECT personne_id FROM ent_eleve_lien WHERE eleve_id = $1 AND user_id = $2",
    )
    .bind(hermione.id)
    .bind(hermione.parent.user_id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1,$2,$3,$4,ARRAY['communication'],'decision_justice','Ne pas communiquer',$5,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(hermione.id)
    .bind(personne_h)
    .bind(c.lundi - Duration::days(30))
    .bind(cast.direction.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    ent.edt_modifier_seance(
        &g,
        &cmd(s, ChangementSeance::ChangementLieu { lieu_id: c.d.salle }),
    )
    .await
    .ok();
    ent.edt_modifier_seance(&g, &cmd(s, ChangementSeance::Annulation))
        .await
        .unwrap();
    distribuer(&ent, e).await;
    let recu = |u: Uuid| {
        let pool = ent.pool().clone();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1 AND kind LIKE 'edt_%'")
                .bind(u)
                .fetch_one(&pool)
                .await
                .unwrap()
        }
    };
    assert_eq!(
        recu(
            neville
                .compte
                .as_ref()
                .map(|c| c.user_id)
                .unwrap_or_else(Uuid::nil)
        )
        .await,
        if neville.compte.is_some() { 1 } else { 0 }
    );
    assert_eq!(
        recu(neville.parent.user_id).await,
        1,
        "inclusion à la date : prévenus"
    );
    assert_eq!(
        recu(hermione.parent.user_id).await,
        0,
        "exclusion et interdiction : rien"
    );
    if let Some(co) = &hermione.compte {
        assert_eq!(
            recu(co.user_id).await,
            0,
            "élève exclu à la date : pas concerné"
        );
    }
    assert_eq!(
        recu(harry.parent.user_id).await,
        0,
        "élève majeur : ses responsables ne sont pas prévenus par défaut"
    );
    assert_eq!(
        recu(harry.compte.as_ref().unwrap().user_id).await,
        1,
        "l'élève majeur l'est"
    );
    // Le lendemain (hors inclusion) : Neville n'est plus concerné.
    let s2 = seance_du(&ent, c.cr, c.lundi + Duration::days(7)).await;
    ent.edt_modifier_seance(&g, &cmd(s2, ChangementSeance::Annulation))
        .await
        .unwrap();
    let avant = recu(neville.parent.user_id).await;
    distribuer(&ent, e).await;
    assert_eq!(
        recu(neville.parent.user_id).await,
        avant,
        "hors de la période d'inclusion : rien"
    );
}

// ───────────────────────── Alias : un seul message par changement, quel que soit l'écrivain ─────────────────────────

#[tokio::test]
async fn t4_alias_des_anciens_noms_un_seul_message_par_changement() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let j1 = c.lundi;
    let j2 = c.lundi + Duration::days(7);
    // Écrivain historique : `edt.exception` porte le `modification_id` ; écrivain T3 : même chose.
    ent.edt_annuler_seance(&g, c.cr, j1, None).await.unwrap();
    let s2 = seance_du(&ent, c.cr, j2).await;
    ent.edt_modifier_seance(&g, &cmd(s2, ChangementSeance::Annulation))
        .await
        .unwrap();
    let sans_id: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_metier_event WHERE etablissement_id = $1 AND type = 'edt.exception' AND NOT (payload ? 'modification_id')",
    )
    .bind(e)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        sans_id, 0,
        "chaque alias porte le modification_id du journal"
    );
    distribuer(&ent, e).await;
    distribuer(&ent, e).await;
    let compte = cast.eleves[0].compte.as_ref().unwrap().user_id;
    let v = notifs(&ent, compte, "edt_annulation").await;
    assert_eq!(v.len(), 2, "une annulation, un message : {v:?}");
    // Les anciens noms sont documentés avec leur remplaçant.
    assert!(crate::edt_notifications::ALIAS_ANCIENS_NOMS
        .iter()
        .any(|(a, n)| *a == "edt.exception_levee" && n.contains("retablie")));
}

// ───────────────────────── « Demain », élèves sans cours, « maintenant » ─────────────────────────

#[tokio::test]
async fn t4_tableau_demain_permanences_salles_fermees_absents_et_droits() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let g = dir(&cast);
    let jour = c.lundi;
    // Une absence validée de Rogue ce jour-là : sa séance de 8 h est à traiter ; une salle fermée ; un cours annulé.
    sqlx::query(
        "INSERT INTO ent_absence_rh (id, etablissement_id, user_id, debut, fin, motif, statut, created_by_user_id)
         VALUES ($1,$2,$3,$4,$4,'interne','validee',$5)",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(cast.prof.user_id)
    .bind(jour)
    .bind(cast.direction.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO ent_salle_indisponibilite (id, etablissement_id, salle_id, du, au, motif_code, par)
         VALUES ($1,$2,$3, ($4::date + time '00:00') AT TIME ZONE 'Europe/Paris', ($4::date + time '23:59') AT TIME ZONE 'Europe/Paris', 'travaux', $5)",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(c.d.salle)
    .bind(jour)
    .bind(cast.administratif.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    let cpe = acteur(&cast, cast.cpe.user_id, Role::Cpe);
    let t = ent.edt_tableau_demain(&cpe, Some(jour)).await.unwrap();
    assert_eq!(t.jour, jour);
    let rogue = t
        .intervenants_absents
        .iter()
        .find(|x| x.user_id == cast.prof.user_id)
        .expect("absent connu");
    assert!(
        rogue.nb_seances >= 1 && rogue.nb_sans_decision >= 1,
        "{rogue:?}"
    );
    assert_eq!(
        t.remplacements_a_faire,
        t.intervenants_absents
            .iter()
            .map(|x| x.nb_sans_decision)
            .sum::<i64>()
    );
    assert!(
        t.salles_fermees
            .iter()
            .any(|x| x.salle_id == c.d.salle && x.nb_seances_a_deplacer >= 1),
        "{:?}",
        t.salles_fermees
    );
    assert!(
        t.seances_non_assurees
            .iter()
            .any(|x| x.raison == "cours_vacant"),
        "cours sans service : poste vacant"
    );
    // Le professeur est absent : on annule sa séance ; elle sort des « à faire » et fait une permanence de 8 h.
    let s = seance_du(&ent, c.cr, jour).await;
    ent.edt_modifier_seance(&g, &cmd(s, ChangementSeance::Annulation))
        .await
        .unwrap();
    let t2 = ent.edt_tableau_demain(&g, Some(jour)).await.unwrap();
    assert_eq!(t2.changements.annulees, t.changements.annulees + 1);
    let perm = t2
        .permanences
        .iter()
        .find(|p| p.heure_debut == h(8, 0))
        .expect("permanence de 8 h");
    assert!(
        perm.effectif >= 2,
        "Harry et Hermione n'ont plus cours : {perm:?}"
    );
    // Élèves sans cours : la liste nominative (ids), filtrable par plage.
    let sans = ent
        .edt_eleves_sans_cours(&cpe, jour, Some(h(8, 0)), Some(h(9, 0)))
        .await
        .unwrap();
    assert!(
        sans.iter().any(|x| x.eleve_id == cast.eleves[0].id) && sans.len() as i64 >= perm.effectif
    );
    assert!(ent
        .edt_eleves_sans_cours(&cpe, jour, Some(h(10, 0)), Some(h(11, 0)))
        .await
        .unwrap()
        .is_empty());
    assert!(ent
        .edt_eleves_sans_cours(&cpe, jour, Some(h(9, 0)), None)
        .await
        .is_err());
    // Une séance neutralisée est un « ailleurs » (examen, sortie) : pas de permanence pour elle.
    // Notifications : celles de l'emploi du temps sont comptées (envoyées / à envoyer).
    distribuer(&ent, e).await;
    let t3 = ent.edt_tableau_demain(&g, Some(jour)).await.unwrap();
    assert!(
        t3.notifications.envoyees + t3.notifications.a_envoyer >= 1,
        "{:?}",
        t3.notifications
    );
    // Droits : direction, secrétariat et vie scolaire ; ni le professeur, ni les familles, ni l'accueil.
    for u in [cast.prof.user_id] {
        assert!(matches!(
            ent.edt_tableau_demain(&acteur(&cast, u, Role::Prof), Some(jour))
                .await,
            Err(EntDbError::Forbidden)
        ));
    }
    assert!(matches!(
        ent.edt_tableau_demain(
            &acteur(&cast, cast.parent_lea.user_id, Role::Parent),
            Some(jour)
        )
        .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(ent
        .edt_tableau_demain(&adm(&cast), Some(jour))
        .await
        .is_ok());
    let autre = ent.seed_labo_cast().await.unwrap();
    let t_autre = ent
        .edt_tableau_demain(&dir(&autre), Some(jour))
        .await
        .unwrap();
    assert!(
        t_autre.intervenants_absents.is_empty()
            && t_autre.salles_fermees.is_empty()
            && t_autre.permanences.is_empty()
    );
}

#[tokio::test]
async fn t4_maintenant_eleve_journalise_interdiction_protection_et_presence_du_professeur() {
    let Some((ent, cast)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let e = cast.etablissement_id;
    let c = cadre(&ent, &cast).await;
    let cpe = acteur(&cast, cast.cpe.user_id, Role::Cpe);
    let accueil = acteur(&cast, cast.cpe.user_id, Role::Accueil);
    let g = dir(&cast);
    let harry = &cast.eleves[0];
    let s = seance_du(&ent, c.cr, c.lundi).await;
    let (debut, fin): (DateTime<Utc>, DateTime<Utc>) =
        sqlx::query_as("SELECT debut_utc, fin_utc FROM ent_edt_seance WHERE id = $1")
            .bind(s)
            .fetch_one(ent.pool())
            .await
            .unwrap();
    let pendant = debut + Duration::minutes(10);
    let apres = fin + Duration::hours(3);
    // Pendant la séance : cours, salle, intervenant ; après : pas de cours.
    match ent
        .edt_maintenant_eleve(&cpe, harry.id, "recherche", None, Some(pendant))
        .await
        .unwrap()
    {
        Maintenant::EnCours {
            seance_id,
            lieu_id,
            intervenant_user_id,
            matiere,
            ..
        } => {
            assert_eq!(
                (seance_id, lieu_id, intervenant_user_id),
                (s, Some(c.d.salle), cast.prof.user_id)
            );
            assert!(!matiere.is_empty());
        }
        autre => panic!("attendu EnCours : {autre:?}"),
    }
    assert_eq!(
        ent.edt_maintenant_eleve(&cpe, harry.id, "recherche", None, Some(apres))
            .await
            .unwrap(),
        Maintenant::PasDeCours
    );
    // Séance annulée : plus « en cours » (permanence).
    ent.edt_modifier_seance(&g, &cmd(s, ChangementSeance::Annulation))
        .await
        .unwrap();
    assert_eq!(
        ent.edt_maintenant_eleve(&cpe, harry.id, "recherche", None, Some(pendant))
            .await
            .unwrap(),
        Maintenant::PasDeCours
    );
    ent.edt_lever_modification(
        &g,
        sqlx::query_scalar(
            "SELECT id FROM ent_edt_modification WHERE seance_id = $1 AND levee_le IS NULL",
        )
        .bind(s)
        .fetch_one(ent.pool())
        .await
        .unwrap(),
    )
    .await
    .unwrap();
    // Chaque lecture est journalisée (qui, quel élève, quel usage, la nature de la réponse) ; jamais le lieu.
    let journal: Vec<(Uuid, Uuid, String, String)> = sqlx::query_as(
        "SELECT lecteur_user_id, eleve_id, usage, reponse FROM ent_edt_localisation_journal WHERE etablissement_id = $1 ORDER BY le, id",
    )
    .bind(e)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert_eq!(journal.len(), 3);
    assert_eq!(
        (
            journal[0].0,
            journal[0].1,
            journal[0].2.as_str(),
            journal[0].3.as_str()
        ),
        (cast.cpe.user_id, harry.id, "recherche", "en_cours")
    );
    assert!(journal.iter().skip(1).all(|j| j.3 == "pas_de_cours"));
    assert!(
        sqlx::query("DELETE FROM ent_edt_localisation_journal")
            .execute(ent.pool())
            .await
            .is_err(),
        "ajout seul"
    );
    assert!(
        sqlx::query("UPDATE ent_edt_localisation_journal SET usage = 'urgence'")
            .execute(ent.pool())
            .await
            .is_err()
    );
    // L'usage est obligatoire et connu ; l'accueil peut lire (droit), le professeur, la famille et l'élève non.
    assert!(ent
        .edt_maintenant_eleve(&cpe, harry.id, "curiosite", None, Some(pendant))
        .await
        .is_err());
    assert!(ent
        .edt_maintenant_eleve(&accueil, harry.id, "remise", None, Some(pendant))
        .await
        .is_ok());
    for (u, r) in [
        (cast.prof.user_id, Role::Prof),
        (harry.parent.user_id, Role::Parent),
        (harry.compte.as_ref().unwrap().user_id, Role::Eleve),
    ] {
        assert!(matches!(
            ent.edt_maintenant_eleve(
                &acteur(&cast, u, r),
                harry.id,
                "recherche",
                None,
                Some(pendant)
            )
            .await,
            Err(EntDbError::Forbidden)
        ));
    }
    // Personne sous interdiction de remise : « indisponible », la même réponse que pour un élève protégé.
    let hermione = cast.eleves.iter().find(|x| x.prenom == "Hermione").unwrap();
    let personne: Uuid = sqlx::query_scalar(
        "SELECT personne_id FROM ent_eleve_lien WHERE eleve_id = $1 AND user_id = $2",
    )
    .bind(hermione.id)
    .bind(hermione.parent.user_id)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert!(
        matches!(
            ent.edt_maintenant_eleve(&cpe, hermione.id, "remise", Some(personne), Some(pendant))
                .await
                .unwrap(),
            Maintenant::EnCours { .. }
        ),
        "sans interdiction : réponse normale"
    );
    sqlx::query(
        "INSERT INTO ent_eleve_interdiction (id, etablissement_id, eleve_id, personne_id, objet, fondement, consigne, du, decide_par)
         VALUES ($1,$2,$3,$4,ARRAY['remise'],'decision_justice','Ne pas remettre',$5,$6)",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(hermione.id)
    .bind(personne)
    .bind(c.lundi - Duration::days(30))
    .bind(cast.direction.user_id)
    .execute(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        ent.edt_maintenant_eleve(&cpe, hermione.id, "remise", Some(personne), Some(pendant))
            .await
            .unwrap(),
        Maintenant::Indisponible
    );
    // Un élève sous confidentialité renforcée : indisponible pour la vie scolaire, lisible par la direction.
    sqlx::query(
        "INSERT INTO ent_eleve_confidentialite (id, etablissement_id, eleve_id, motif_scelle, du) VALUES ($1,$2,$3,'enc:v1:x',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(e)
    .bind(harry.id)
    .bind(c.lundi - Duration::days(30))
    .execute(ent.pool())
    .await
    .unwrap();
    assert_eq!(
        ent.edt_maintenant_eleve(&cpe, harry.id, "recherche", None, Some(pendant))
            .await
            .unwrap(),
        Maintenant::Indisponible
    );
    assert!(matches!(
        ent.edt_maintenant_eleve(&g, harry.id, "urgence", None, Some(pendant))
            .await
            .unwrap(),
        Maintenant::EnCours { .. }
    ));
    // Élève d'un autre établissement : introuvable ; le journal ne garde rien d'un refus de droit.
    let autre = ent.seed_labo_cast().await.unwrap();
    assert!(matches!(
        ent.edt_maintenant_eleve(&dir(&autre), harry.id, "recherche", None, Some(pendant))
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
    // « Maintenant » pour une salle (ce qui l'occupe, pas qui) et pour un professeur (présent oui / non seulement).
    let occ = ent
        .edt_maintenant_salle(&cpe, c.d.salle, Some(pendant))
        .await
        .unwrap();
    assert!(occ.occupee && occ.seance_id == Some(s));
    assert!(
        !ent.edt_maintenant_salle(&cpe, c.d.salle, Some(apres))
            .await
            .unwrap()
            .occupee
    );
    assert!(
        ent.edt_maintenant_professeur(&cpe, cast.prof.user_id, Some(pendant))
            .await
            .unwrap()
            .present
    );
    assert!(
        !ent.edt_maintenant_professeur(&cpe, cast.prof.user_id, Some(apres))
            .await
            .unwrap()
            .present,
        "après sa dernière séance : non"
    );
    // Entre deux séances du même jour : présent (pas de lieu, pas de « où »).
    creneau(&ent, &cast, Some(c.d.cours2), 1, h(14, 0), h(15, 0), None).await;
    ent.ref_generer_seances(&adm(&cast), c.d.annee_id, Some(c.lundi), Some(c.lundi))
        .await
        .unwrap();
    let midi = paris(&ent, c.lundi, h(12, 0)).await;
    assert!(
        ent.edt_maintenant_professeur(&cpe, cast.prof.user_id, Some(midi))
            .await
            .unwrap()
            .present
    );
    let json = serde_json::to_string(
        &ent.edt_maintenant_professeur(&cpe, cast.prof.user_id, Some(midi))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(json, "{\"present\":true}");
    assert!(matches!(
        ent.edt_maintenant_professeur(&cpe, Uuid::new_v4(), Some(midi))
            .await,
        Err(EntDbError::Domain(domain_eleve::DomainError::NotFound))
    ));
}
