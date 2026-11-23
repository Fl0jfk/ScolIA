//! Messagerie, correctifs de la revue du pilote sur le lot « suite » (M12 et pièce sur message différé) :
//! `docs/socles/16_MESSAGERIE_M12_SUITE_LIVRAISON.md`, section « Correctifs de revue ».
//!
//! Règles : un test de refus exige le **refus exact** (règle 3) ; le flux ne révèle jamais plus que la lecture
//! (règle 1) ; un curseur ne saute jamais d'événement (règle 2) ; aucune heure réelle de l'appelant : les instants
//! viennent de la base (règle 33).

use super::tests_messagerie_m0a::poser_interdiction;
use super::tests_messagerie_m1::{compter, ecrit, labo, lit, role};
use super::tests_messagerie_m5::{deposer, est_conflit, fil_de_lily, lever_quarantaine, PDF_REEL};
use super::tests_messagerie_m6::dernier_message;
use super::*;
use crate::tenant_tx::TenantTx;
use chrono::{Duration, Utc};
use domain_eleve::{DomainError, MsgPiece};
use ent_auth::Role;
use uuid::Uuid;

/// Curseur de départ du flux : tout ce qui est sous la borne `xmin` est connu. Aucune horloge de l'appelant.
async fn depart(ent: &PgEnt, etab: Uuid) -> String {
    ent.messages.position_depart_flux(etab).await.expect("position de départ").encoder()
}

async fn flux(ent: &PgEnt, etab: Uuid, user: Uuid, r: Role, apres: &str) -> crate::FluxMessagerie {
    ent.flux_messagerie_for_actor(etab, user, role(r), Some(apres), 0).await.expect("flux")
}

async fn fils_du_flux(ent: &PgEnt, etab: Uuid, user: Uuid, r: Role, apres: &str) -> Vec<Uuid> {
    flux(ent, etab, user, r, apres).await.fils.into_iter().map(|f| f.fil_id).collect()
}

/// Réglages de test : une « seconde » du flux dure `ms` millisecondes (temps injecté, aucune attente réelle de 25 s) ;
/// `fils_max` petit pour provoquer la troncature.
fn reglages(ms: u64, fils_max: i64) -> crate::ReglagesFlux {
    crate::ReglagesFlux { seconde: std::time::Duration::from_millis(ms), fils_max }
}

/// Compte les requêtes SQL émises par un futur (événements `sqlx::query` de `tracing`) : l'absence de N+1 se prouve en
/// comptant, jamais en mesurant une durée.
mod compte_requetes {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tracing::instrument::WithSubscriber;

    struct Compteur(Arc<AtomicUsize>);

    impl tracing::Subscriber for Compteur {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, e: &tracing::Event<'_>) {
            if e.metadata().target() == "sqlx::query" {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    /// Rend le résultat du futur et le nombre de requêtes SQL qu'il a émises.
    pub async fn compter<F: std::future::Future>(f: F) -> (F::Output, usize) {
        let n = Arc::new(AtomicUsize::new(0));
        let sortie = f.with_subscriber(Compteur(n.clone())).await;
        (sortie, n.load(Ordering::SeqCst))
    }
}

/// Un message différé (remise dans une heure) de la direction dans le fil de Lily ; rend l'identifiant du message.
async fn message_differe(ent: &PgEnt, etab: Uuid, dir: Uuid, fil: Uuid, corps: &str) -> Uuid {
    assert!(ecrit(ent, etab, dir, Role::Direction, fil, corps).await);
    let m = dernier_message(ent, etab, fil, dir).await;
    sqlx::query("UPDATE ent_msg SET remise_prevue_le = now() + interval '1 hour' WHERE id = $1")
        .bind(m.id)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    m.id
}

// ───────────────────────── M1 : le flux ne révèle jamais plus que la lecture ─────────────────────────

#[tokio::test]
async fn revue_m1_flux_suit_le_jugement_de_lecture_avec_le_role() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily, harry) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id, cast.eleve_lea_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Flux et droits").await;
    // Participant **sans droit de lecture**, construit explicitement : un professeur d'une autre classe que celle de
    // l'élève du fil (le labo en compte plusieurs), ajouté à la participation. La vue de participation le voit, le
    // jugement de lecture le refuse : la précondition est affirmée, jamais supposée.
    let prof = cast
        .profs
        .iter()
        .map(|p| p.user_id)
        .find(|u| *u != cast.prof.user_id)
        .expect("un second professeur dans le labo");
    assert!(!lit(&ent, etab, prof, Role::Prof, fil).await, "précondition : ce professeur n'a pas le droit de lire ce fil");
    sqlx::query("SELECT ent_msg_ajouter_participant($1, $2, 'ajoute', 'a_ecrit')")
        .bind(fil)
        .bind(prof)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let avant = depart(&ent, etab).await;
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Après l'ajout").await);
    // Témoin : Lily lit et reçoit.
    assert!(lit(&ent, etab, lily, Role::Parent, fil).await);
    assert_eq!(fils_du_flux(&ent, etab, lily, Role::Parent, &avant).await, vec![fil], "témoin : la famille participante");
    // Le flux n'annonce que ce que la lecture accorde, pour chaque compte, avec son rôle de session.
    for (qui, r) in [(prof, Role::Prof), (cast.cpe.user_id, Role::Cpe), (cast.administratif.user_id, Role::Administratif)] {
        let peut_lire = lit(&ent, etab, qui, r, fil).await;
        let recu = fils_du_flux(&ent, etab, qui, r, &avant).await.contains(&fil);
        assert!(!recu || peut_lire, "le flux révèle un fil que la lecture refuse ({r:?})");
    }
    assert!(!lit(&ent, etab, prof, Role::Prof, fil).await, "toujours sans droit après l'ajout");
    assert!(fils_du_flux(&ent, etab, prof, Role::Prof, &avant).await.is_empty(), "participant sans droit : rien");
    // Le même compte avec un rôle de session qui n'est pas celui de sa lecture ne reçoit rien non plus : le rôle
    // transmis est celui de la session (un compte « parent » n'est pas jugé comme un personnel).
    assert!(fils_du_flux(&ent, etab, prof, Role::Parent, &avant).await.is_empty());

    // Droit retiré : une interdiction de communication vise Lily, la lecture se ferme, le flux se tait aussitôt.
    poser_interdiction(&ent, etab, harry, lily, dir).await;
    assert!(!lit(&ent, etab, lily, Role::Parent, fil).await, "lecture refusée après l'interdiction");
    assert!(fils_du_flux(&ent, etab, lily, Role::Parent, &avant).await.is_empty(), "plus rien dans le flux");
}

// ───────────────────────── M2 : troncature, ordre total, fenêtre ─────────────────────────

#[tokio::test]
async fn revue_m2_liste_tronquee_le_curseur_ne_saute_aucun_evenement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let mut fils = Vec::new();
    for i in 0..4 {
        fils.push(fil_de_lily(&ent, &cast, &format!("Troncature {i}")).await.0);
    }
    let avant = depart(&ent, etab).await;
    for f in &fils {
        assert!(ecrit(&ent, etab, dir, Role::Direction, *f, "Nouveau").await);
    }
    // Pagination par limite 2 : chaque appel rend la position du dernier événement rendu et `tronque`. Le client
    // reprend toujours depuis la position rendue ; le labo est partagé : on s'arrête quand les fils de ce test sont passés.
    let mut vus = Vec::new();
    let mut pos = avant.clone();
    let (mut tours, mut tronquees) = (0, 0);
    while fils.iter().any(|f| !vus.contains(f)) {
        let r = ent
            .flux_messagerie_reglages(etab, lily, role(Role::Parent), Some(&pos), 0, reglages(1, 2))
            .await
            .expect("flux");
        assert!(r.fils.len() <= 2);
        vus.extend(r.fils.iter().map(|f| f.fil_id));
        tours += 1;
        assert!(tours < 400, "pas de boucle (le curseur progresse toujours)");
        if r.tronque {
            tronquees += 1;
            assert_eq!(r.fils.len(), 2, "une liste tronquée est pleine");
        }
        pos = r.curseur;
    }
    assert!(tronquees >= 1, "la liste a bien été tronquée");
    let mut attendus = fils.clone();
    attendus.sort();
    vus.retain(|f| attendus.contains(f)); // le labo est partagé : d'autres tests écrivent dans d'autres fils
    let nb = vus.len();
    vus.sort();
    assert_eq!(vus, attendus, "aucun événement sauté");
    assert_eq!(nb, attendus.len(), "aucun doublon");

    // Même transaction : quatre messages portent le **même** `xid` ; l'ordre `(xid, fil, genre)` les départage et la
    // coupure (limite 1) ne perd aucun fil.
    let avant = depart(&ent, etab).await;
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let mut ids = Vec::new();
    for f in &fils {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_msg (id, etablissement_id, fil_id, auteur_user_id, corps) VALUES ($1, $2, $3, $4, 'Même transaction')",
        )
        .bind(id)
        .bind(etab)
        .bind(f)
        .bind(dir)
        .execute(&mut *tx)
        .await
        .unwrap();
        ids.push(id);
    }
    tx.commit().await.unwrap();
    let distincts: i64 = sqlx::query_scalar("SELECT count(DISTINCT msg_xid) FROM ent_msg WHERE id = ANY($1)")
        .bind(&ids)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(distincts, 1, "précondition : un seul identifiant de transaction");
    let mut vus = Vec::new();
    let mut pos = avant;
    for tour in 0..400 {
        if fils.iter().all(|f| vus.contains(f)) {
            break;
        }
        assert!(tour < 399, "pas de boucle");
        let r = ent
            .flux_messagerie_reglages(etab, lily, role(Role::Parent), Some(&pos), 0, reglages(1, 1))
            .await
            .unwrap();
        vus.extend(r.fils.iter().map(|f| f.fil_id));
        pos = r.curseur;
    }
    vus.retain(|f| attendus.contains(f));
    vus.sort();
    assert_eq!(vus, attendus, "quatre événements de même xid : tous rendus");

    // Fenêtre maximale : un curseur émis il y a plus longtemps (âge porté par le curseur lui-même, aucune horloge
    // réelle) demande la resynchronisation complète ; un ancien format ou une valeur fabriquée aussi.
    let mut vieux = crate::PositionFlux::decoder(&depart(&ent, etab).await).expect("curseur lisible");
    vieux.emis -= Duration::seconds(crate::msg_repo::FLUX_FENETRE_SECONDES as i64 + 60);
    let r = flux(&ent, etab, lily, Role::Parent, &vieux.encoder()).await;
    assert!(r.resynchro && r.fils.is_empty() && !r.tronque);
    for illisible in ["2026-11-22T10:00:00Z", "v2.x.-.-.0.-.0", ""] {
        let r = flux(&ent, etab, lily, Role::Parent, illisible).await;
        assert!(r.resynchro && r.fils.is_empty(), "curseur illisible « {illisible} » : resynchronisation");
        assert!(crate::PositionFlux::decoder(&r.curseur).is_some(), "le curseur rendu repart d'une base saine");
    }
    let recent = depart(&ent, etab).await;
    assert!(!flux(&ent, etab, lily, Role::Parent, &recent).await.resynchro, "témoin : un curseur récent ne l'exige pas");
}

#[tokio::test]
async fn revue_m2_transaction_longue_valide_tard_est_servie_au_tour_suivant_avec_troncature() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (f_long, _) = fil_de_lily(&ent, &cast, "Transaction longue").await;
    let (f_court, _) = fil_de_lily(&ent, &cast, "Transaction courte").await;
    let c0 = depart(&ent, etab).await;

    // Une transaction écrit **tôt** (son `xid` est le plus ancien) et ne valide que **tard**.
    let mut longue = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let m_long = Uuid::new_v4();
    sqlx::query("INSERT INTO ent_msg (id, etablissement_id, fil_id, auteur_user_id, corps) VALUES ($1, $2, $3, $4, 'Écrit tôt')")
        .bind(m_long)
        .bind(etab)
        .bind(f_long)
        .bind(dir)
        .execute(&mut *longue)
        .await
        .unwrap();
    // Pendant ce temps, une transaction **plus récente** (xid plus grand) écrit et valide.
    assert!(ecrit(&ent, etab, dir, Role::Direction, f_court, "Écrit après, validé avant").await);
    // Un tour passe pendant que la transaction est ouverte : le message validé APRÈS elle n'est pas servi (son xid est
    // au-delà de la borne) et le curseur ne dépasse pas la transaction ouverte.
    let r1 = flux(&ent, etab, lily, Role::Parent, &c0).await;
    assert!(!r1.resynchro);
    assert!(!r1.fils.iter().any(|f| f.fil_id == f_court), "pas servi tant qu'un xid plus ancien est ouvert : {r1:?}");
    assert!(!r1.fils.iter().any(|f| f.fil_id == f_long), "le message non validé est invisible");
    let p1 = crate::PositionFlux::decoder(&r1.curseur).unwrap();
    let borne_xid: i64 = sqlx::query_scalar("SELECT pg_snapshot_xmin(pg_current_snapshot())::text::bigint")
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(p1.xid as i64 <= borne_xid, "le curseur reste sous la plus ancienne transaction ouverte");

    // La transaction valide : au tour suivant, **les deux** sont servis, dans l'ordre des xid, malgré une troncature
    // à un événement par réponse (le message écrit tôt d'abord, rien ne saute).
    longue.commit().await.unwrap();
    let plus_ancien: bool = sqlx::query_scalar(
        "SELECT (SELECT msg_xid FROM ent_msg WHERE id = $1)
              < (SELECT max(msg_xid) FROM ent_msg WHERE fil_id = $2 AND corps = 'Écrit après, validé avant')",
    )
    .bind(m_long)
    .bind(f_court)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(plus_ancien, "précondition : le message écrit tôt a le xid le plus ancien, bien qu'il soit validé après");
    let mut ordre = Vec::new();
    let mut pos = r1.curseur.clone();
    let mut tronquees = 0;
    for tour in 0..200 {
        if [f_long, f_court].iter().all(|f| ordre.contains(f)) {
            break;
        }
        assert!(tour < 199, "pas de boucle");
        let r = ent
            .flux_messagerie_reglages(etab, lily, role(Role::Parent), Some(&pos), 0, reglages(1, 1))
            .await
            .unwrap();
        tronquees += usize::from(r.tronque);
        ordre.extend(r.fils.iter().map(|f| f.fil_id).filter(|f| *f == f_long || *f == f_court));
        pos = r.curseur;
    }
    assert_eq!(ordre, vec![f_long, f_court], "écrit tôt, validé tard : servi, avant le plus récent ({tronquees} troncatures)");
}

// ───────────────────────── M3 : plus de répétition, plus de boucle serrée ─────────────────────────

#[tokio::test]
async fn revue_m3_un_evenement_n_est_servi_qu_une_fois_et_le_rappel_attend() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Rappel").await;
    let c0 = depart(&ent, etab).await;
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Un événement").await);
    // Une « seconde » du flux = 20 ms (temps injecté) : l'attente de 5 s demandée dure 100 ms.
    let r = reglages(20, 200);
    let r1 = ent.flux_messagerie_reglages(etab, lily, role(Role::Parent), Some(&c0), 5, r).await.unwrap();
    assert!(r1.fils.iter().any(|f| f.fil_id == fil), "premier rappel : l'événement");
    // Second rappel avec le curseur rendu : pas le même événement, et l'appel **attend** (jamais de réponse immédiate).
    let t0 = tokio::time::Instant::now();
    let (r2, requetes_attente) =
        compte_requetes::compter(ent.flux_messagerie_reglages(etab, lily, role(Role::Parent), Some(&r1.curseur), 5, r)).await;
    let r2 = r2.unwrap();
    assert!(!r2.fils.iter().any(|f| f.fil_id == fil), "l'événement déjà rendu n'est pas répété : {r2:?}");
    assert!(
        t0.elapsed() >= r.seconde * 5,
        "le rappel suivant attend la durée demandée ({:?} < {:?})",
        t0.elapsed(),
        r.seconde * 5
    );
    // Pas de boucle serrée : l'attente relit au pas de 2 « secondes », pas en continu (comparaison avec un appel sans attente).
    let (_, requetes_une) =
        compte_requetes::compter(ent.flux_messagerie_reglages(etab, lily, role(Role::Parent), Some(&r1.curseur), 0, r)).await;
    assert!(requetes_attente >= requetes_une, "au moins une relecture");
    assert!(
        requetes_attente <= requetes_une * 6,
        "relectures bornées au pas : {requetes_attente} requêtes contre {requetes_une} pour une seule"
    );
    // La position ne recule jamais, et un nouvel événement est servi (témoin).
    let (p1, p2) = (crate::PositionFlux::decoder(&r1.curseur).unwrap(), crate::PositionFlux::decoder(&r2.curseur).unwrap());
    assert!(p2.xid >= p1.xid, "le curseur ne recule pas");
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Un second événement").await);
    let r3 = ent.flux_messagerie_reglages(etab, lily, role(Role::Parent), Some(&r2.curseur), 5, r).await.unwrap();
    assert!(r3.fils.iter().any(|f| f.fil_id == fil), "un événement postérieur est servi");
}

#[tokio::test]
async fn revue_differe_une_echeance_est_servie_une_fois_et_pas_avant() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Échéance").await;
    let differe = message_differe(&ent, etab, dir, fil, "Plus tard").await;
    let c0 = depart(&ent, etab).await;
    let r1 = flux(&ent, etab, lily, Role::Parent, &c0).await;
    assert!(!r1.fils.iter().any(|f| f.fil_id == fil), "avant l'échéance : muet");
    // L'échéance arrive (sans horloge réelle : la base la ramène à l'instant de l'UPDATE).
    sqlx::query("UPDATE ent_msg SET remise_prevue_le = clock_timestamp() WHERE id = $1")
        .bind(differe)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let r2 = flux(&ent, etab, lily, Role::Parent, &r1.curseur).await;
    assert_eq!(r2.fils.iter().filter(|f| f.fil_id == fil && f.genre == "nouveau").count(), 1, "{r2:?}");
    let r3 = flux(&ent, etab, lily, Role::Parent, &r2.curseur).await;
    assert!(!r3.fils.iter().any(|f| f.fil_id == fil), "servi une seule fois : {r3:?}");
    // L'auteur, lui, n'est pas averti de son propre message.
    assert!(!fils_du_flux(&ent, etab, dir, Role::Direction, &c0).await.contains(&fil));
}

#[tokio::test]
async fn revue_m3_created_at_vient_de_la_base() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Horloge unique").await;
    // Un message dont l'horodatage d'application serait très en retard : la base impose son horloge.
    let mut m = dernier_message(&ent, etab, fil, dir).await;
    m.id = Uuid::new_v4();
    m.created_at = Utc::now() - Duration::days(3650);
    ent.messages.insert_message_remise(&m, None, None).await.expect("insertion");
    let (ecart,): (f64,) = sqlx::query_as("SELECT extract(epoch FROM (now() - created_at))::float8 FROM ent_msg WHERE id = $1")
        .bind(m.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!((0.0..300.0).contains(&ecart), "created_at posé par la base, pas par l'application : {ecart} s");
}

// ───────────────────────── F2 : ni ses propres messages, retrait signalé sans contenu ─────────────────────────

#[tokio::test]
async fn revue_f2_flux_n_annonce_pas_ses_propres_messages_et_signale_un_retrait() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Auteur et lecteur").await;
    let avant = depart(&ent, etab).await;
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Corps secret du retrait").await);
    // L'auteur ne reçoit pas l'annonce de son propre message ; le destinataire, si.
    assert!(fils_du_flux(&ent, etab, dir, Role::Direction, &avant).await.is_empty(), "l'auteur n'est pas averti de lui-même");
    let f = flux(&ent, etab, lily, Role::Parent, &avant).await;
    assert_eq!(f.fils.len(), 1);
    assert_eq!(f.fils[0].genre, "nouveau");
    // Retrait : signalé au destinataire (genre `retire`), sans contenu ; pas à celui qui retire.
    let m = dernier_message(&ent, etab, fil, dir).await;
    let apres_envoi = depart(&ent, etab).await;
    ent.retirer_message_for_actor(etab, dir, role(Role::Direction), m.id, None).await.expect("retrait");
    // Le retrait porte l'identifiant de sa propre transaction (déclencheur), postérieur à celui du message.
    let posterieur: bool = sqlx::query_scalar("SELECT retire_xid > msg_xid FROM ent_msg WHERE id = $1")
        .bind(m.id)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap();
    assert!(posterieur, "retire_xid posé par le déclencheur, après msg_xid");
    let f = flux(&ent, etab, lily, Role::Parent, &apres_envoi).await;
    assert_eq!(f.fils.len(), 1, "{f:?}");
    assert_eq!((f.fils[0].fil_id, f.fils[0].genre.as_str()), (fil, "retire"));
    let json = serde_json::to_string(&f).unwrap();
    assert!(!json.contains("secret") && !json.contains("retrait"), "aucun contenu : {json}");
    assert!(fils_du_flux(&ent, etab, dir, Role::Direction, &apres_envoi).await.is_empty(), "celui qui retire n'est pas averti");
    // Un message retiré ne compte plus comme « nouveau » : depuis `avant`, un seul événement (le retrait).
    let tout = flux(&ent, etab, lily, Role::Parent, &avant).await;
    assert!(tout.fils.iter().all(|f| f.genre == "retire"), "{tout:?}");
}

// ───────────────────────── Pièce sur message différé ─────────────────────────

fn piece_directe(etab: Uuid, fil: Uuid, message: Uuid, par: Uuid) -> MsgPiece {
    MsgPiece {
        id: Uuid::new_v4(),
        etablissement_id: etab,
        fil_id: fil,
        message_id: message,
        nom: "x.pdf".into(),
        mime_type: "application/pdf".into(),
        taille_octets: 4,
        contenu: Vec::new(),
        storage_key: Some(format!("{etab}/messages/{}.pdf", Uuid::new_v4())),
        sha256: Some("0".repeat(64)),
        created_by_user_id: par,
        created_at: Utc::now(),
        etat: "en_quarantaine".into(),
    }
}

#[tokio::test]
async fn revue_f1_depot_direct_par_un_autre_que_l_auteur_d_un_differe_exige_message_differe() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Dépôt direct").await;
    let differe = message_differe(&ent, etab, dir, fil, "Plus tard").await;
    // Niveau dépôt (aucun contrôle du domaine) : `created_by` autre que l'auteur d'un message différé.
    let r = ent.messages.insert_piece(&piece_directe(etab, fil, differe, lily), 5).await.expect("dépôt");
    assert_eq!(r, crate::msg_repo::InsertionPiece::MessageDiffere);
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_msg_piece WHERE message_id = $1", &[differe]).await, 0);
    // Témoin : l'auteur, lui, peut.
    let r = ent.messages.insert_piece(&piece_directe(etab, fil, differe, dir), 5).await.expect("dépôt de l'auteur");
    assert_eq!(r, crate::msg_repo::InsertionPiece::Inseree);
}

#[tokio::test]
async fn revue_message_differe_d_autrui_est_un_404_pas_un_refus_d_auteur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Existence").await;
    let differe = message_differe(&ent, etab, dir, fil, "Programmé").await;
    // Un non-auteur qui vise le message différé reçoit exactement ce que reçoit un identifiant inconnu.
    let e_differe = deposer(&ent, etab, differe, lily, Role::Parent, "x.pdf", "application/pdf", PDF_REEL)
        .await
        .expect_err("refus");
    let e_inconnu = deposer(&ent, etab, Uuid::new_v4(), lily, Role::Parent, "x.pdf", "application/pdf", PDF_REEL)
        .await
        .expect_err("refus");
    assert!(matches!(e_differe, EntDbError::Domain(DomainError::NotFound)), "{e_differe:?}");
    assert!(matches!(e_inconnu, EntDbError::Domain(DomainError::NotFound)), "{e_inconnu:?}");
    // Sur un message **remis** et visible du non-auteur : le refus d'auteur reste exact.
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Remis").await);
    let remis = dernier_message(&ent, etab, fil, dir).await;
    let e = deposer(&ent, etab, remis.id, lily, Role::Parent, "x.pdf", "application/pdf", PDF_REEL)
        .await
        .expect_err("refus");
    assert!(matches!(e, EntDbError::Domain(DomainError::MsgPiecePasAuteur)), "{e:?}");
}

#[tokio::test]
async fn revue_pieces_du_differe_invisibles_avant_remise_quarantaine_apres() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Pièce différée").await;
    let differe = message_differe(&ent, etab, dir, fil, "Avec pièce").await;
    let piece = deposer(&ent, etab, differe, dir, Role::Direction, "joint.pdf", "application/pdf", PDF_REEL)
        .await
        .expect("l'auteur joint une pièce à son message différé");
    let pieces_vues = |lues: &[domain_eleve::MessageLue]| -> usize { lues.iter().map(|m| m.pieces.len()).sum() };
    // Avant la remise : ni le message ni les métadonnées de la pièce n'existent pour le destinataire.
    let (_, lues) = ent.list_messages_for_actor(etab, lily, role(Role::Parent), fil).await.unwrap();
    assert!(lues.iter().all(|m| m.message.id != differe), "message invisible");
    assert_eq!(pieces_vues(&lues), 0, "métadonnées de pièce invisibles du destinataire");
    let json = serde_json::to_string(&lues.iter().map(|m| m.pieces.clone()).collect::<Vec<_>>()).unwrap();
    assert!(!json.contains("joint.pdf") && !json.contains(&piece.id.to_string()), "{json}");
    // L'auteur, lui, voit sa pièce.
    let (_, siennes) = ent.list_messages_for_actor(etab, dir, role(Role::Direction), fil).await.unwrap();
    assert_eq!(pieces_vues(&siennes), 1, "témoin : l'auteur voit sa pièce");
    // Le téléchargement par le destinataire est refusé avec exactement la réponse d'une pièce inconnue.
    let e = ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), piece.id).await.expect_err("refus");
    let inconnue = ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), Uuid::new_v4()).await.expect_err("refus");
    assert_eq!(format!("{e:?}"), format!("{inconnue:?}"), "pas de fuite d'existence");

    // Remise passée : le message paraît, la pièce aussi, mais encore **en quarantaine** : non téléchargeable.
    sqlx::query("UPDATE ent_msg SET remise_prevue_le = now() - interval '1 second' WHERE id = $1")
        .bind(differe)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let (_, lues) = ent.list_messages_for_actor(etab, lily, role(Role::Parent), fil).await.unwrap();
    assert_eq!(pieces_vues(&lues), 1, "après la remise, la pièce est listée");
    let e = ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), piece.id).await.expect_err("quarantaine");
    assert!(est_conflit(&e), "quarantaine : {e:?}");
    // Analyse saine : elle est enfin servie.
    lever_quarantaine(&ent, etab).await;
    let servie = ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), piece.id).await.expect("servie une fois saine");
    assert!(!servie.contenu.is_empty());
}

#[tokio::test]
async fn revue_differe_plafond_de_cinq_pieces_et_annulation_detruit_les_objets() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Plafond").await;
    let differe = message_differe(&ent, etab, dir, fil, "Six pièces").await;
    let mut pieces = Vec::new();
    for i in 0..5 {
        pieces.push(
            deposer(&ent, etab, differe, dir, Role::Direction, &format!("p{i}.pdf"), "application/pdf", PDF_REEL)
                .await
                .unwrap_or_else(|e| panic!("pièce {i} : {e:?}")),
        );
    }
    let e = deposer(&ent, etab, differe, dir, Role::Direction, "sixieme.pdf", "application/pdf", PDF_REEL)
        .await
        .expect_err("plafond");
    assert!(matches!(e, EntDbError::SaisieInvalide(_)), "{e:?}");
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_msg_piece WHERE message_id = $1", &[differe]).await, 5);
    // Annulation du différé : message, pièces et objets disparaissent (rien d'orphelin).
    let cles: Vec<String> = sqlx::query_scalar("SELECT storage_key FROM ent_msg_piece WHERE message_id = $1")
        .bind(differe)
        .fetch_all(ent.pool_verif())
        .await
        .unwrap();
    assert_eq!(cles.len(), 5);
    for c in &cles {
        assert!(ent.objet_existe(c).await.unwrap(), "témoin : l'objet existe avant l'annulation");
    }
    ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), differe).await.expect("annulation");
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_msg WHERE id = $1", &[differe]).await, 0);
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_msg_piece WHERE message_id = $1", &[differe]).await, 0);
    for c in &cles {
        assert!(!ent.objet_existe(c).await.unwrap(), "objet détruit : {c}");
    }
}

// ───────────────────────── F-4 : position (xid, fil, genre) complète ─────────────────────────

#[tokio::test]
async fn revue_f4_position_a_trois_termes_aucun_saut_entre_genres() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Genres").await;
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "À retirer").await);
    let a_retirer = dernier_message(&ent, etab, fil, dir).await.id;
    let avant = depart(&ent, etab).await;
    // Dans **une** transaction : un message nouveau et le retrait d'un autre message du même fil : même `xid`, même
    // fil, deux genres. La limite 1 coupe entre les deux : aucun ne doit se perdre.
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    sqlx::query("INSERT INTO ent_msg (id, etablissement_id, fil_id, auteur_user_id, corps) VALUES ($1, $2, $3, $4, 'Nouveau')")
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(fil)
        .bind(dir)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE ent_msg SET retire_le = now(), retire_par = $2, retrait_par = 'auteur', retrait_motif_code = 'erreur_auteur' WHERE id = $1")
        .bind(a_retirer)
        .bind(dir)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let (xn, xr): (String, String) = sqlx::query_as(
        "SELECT (SELECT max(msg_xid)::text FROM ent_msg WHERE fil_id = $1 AND corps = 'Nouveau'),
                (SELECT retire_xid::text FROM ent_msg WHERE id = $2)",
    )
    .bind(fil)
    .bind(a_retirer)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(xn, xr, "précondition : même identifiant de transaction");
    let mut genres = Vec::new();
    let mut pos = avant;
    for tour in 0..400 {
        if genres.len() >= 2 {
            break;
        }
        assert!(tour < 399, "pas de boucle");
        let r = ent
            .flux_messagerie_reglages(etab, lily, role(Role::Parent), Some(&pos), 0, reglages(1, 1))
            .await
            .unwrap();
        genres.extend(r.fils.iter().filter(|f| f.fil_id == fil).map(|f| f.genre.clone()));
        pos = r.curseur;
    }
    genres.sort();
    assert_eq!(genres, vec!["nouveau".to_string(), "retire".to_string()], "les deux genres du même (xid, fil) sont rendus");
}

// ───────────────────────── M-C : règle 3 sur annuler, corriger, retirer ─────────────────────────

#[tokio::test]
async fn revue_mc_differe_d_autrui_est_un_404_sur_annuler_corriger_et_retirer() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Règle 3").await;
    let differe = message_differe(&ent, etab, dir, fil, "Programmé").await;
    let r = role(Role::Parent);
    let inconnu = Uuid::new_v4();
    // Annuler.
    let e_differe = ent.annuler_message_differe_for_actor(etab, lily, r, differe).await.expect_err("refus");
    let e_inconnu = ent.annuler_message_differe_for_actor(etab, lily, r, inconnu).await.expect_err("refus");
    assert!(matches!(e_differe, EntDbError::Domain(DomainError::NotFound)), "annuler : {e_differe:?}");
    assert!(matches!(e_inconnu, EntDbError::Domain(DomainError::NotFound)), "annuler : {e_inconnu:?}");
    // Corriger.
    let e_differe = ent.corriger_message_fil_for_actor(etab, lily, r, differe, "Autre texte").await.expect_err("refus");
    let e_inconnu = ent.corriger_message_fil_for_actor(etab, lily, r, inconnu, "Autre texte").await.expect_err("refus");
    assert!(matches!(e_differe, EntDbError::Domain(DomainError::NotFound)), "corriger : {e_differe:?}");
    assert!(matches!(e_inconnu, EntDbError::Domain(DomainError::NotFound)), "corriger : {e_inconnu:?}");
    // Retirer.
    let e_differe = ent.retirer_message_for_actor(etab, lily, r, differe, None).await.expect_err("refus");
    let e_inconnu = ent.retirer_message_for_actor(etab, lily, r, inconnu, None).await.expect_err("refus");
    assert!(matches!(e_differe, EntDbError::Domain(DomainError::NotFound)), "retirer : {e_differe:?}");
    assert!(matches!(e_inconnu, EntDbError::Domain(DomainError::NotFound)), "retirer : {e_inconnu:?}");
    // Le différé n'a pas bougé ; l'auteur, lui, garde la main (témoin).
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_msg WHERE id = $1 AND retire_le IS NULL AND corps = 'Programmé'", &[differe]).await, 1);
    ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), differe).await.expect("l'auteur annule");
    // Sur un message **remis** et visible du non-auteur, le refus d'auteur reste exact (`Forbidden`).
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Remis").await);
    let remis = dernier_message(&ent, etab, fil, dir).await.id;
    let e = ent.retirer_message_for_actor(etab, lily, r, remis, None).await.expect_err("refus");
    assert!(matches!(e, EntDbError::Forbidden), "retirer un message remis d'autrui : {e:?}");
    let e = ent.corriger_message_fil_for_actor(etab, lily, r, remis, "x").await.expect_err("refus");
    assert!(matches!(e, EntDbError::Forbidden), "corriger un message remis d'autrui : {e:?}");
}

// ───────────────────────── F-5 : une seule horloge, ordre stable ─────────────────────────

#[tokio::test]
async fn revue_f5_delais_sur_l_horloge_de_la_base_et_ordre_stable() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Une horloge").await;
    for corps in ["Un", "Deux", "Trois"] {
        assert!(ecrit(&ent, etab, dir, Role::Direction, fil, corps).await);
    }
    // Ordre : insertion strictement croissante (instant propre à chaque message, identifiant en second critère).
    let (_, lus) = ent.list_messages_for_actor(etab, dir, role(Role::Direction), fil).await.unwrap();
    let corps: Vec<&str> = lus.iter().map(|m| m.message.corps.as_str()).collect();
    let rang = |c: &str| corps.iter().position(|x| *x == c).unwrap();
    assert!(rang("Un") < rang("Deux") && rang("Deux") < rang("Trois"), "{corps:?}");
    let instants: Vec<_> = lus.iter().map(|m| m.message.created_at).collect();
    assert!(instants.windows(2).all(|w| w[0] < w[1]), "instants distincts et croissants : {instants:?}");
    // Délais : le retrait (10 min) et la correction (15 min) se mesurent sur l'horloge de la base. Un message
    // dont `created_at` (base) a 12 minutes : correction encore possible, retrait refusé — sans horloge d'application.
    let m = dernier_message(&ent, etab, fil, dir).await.id;
    sqlx::query("UPDATE ent_msg SET created_at = now() - interval '12 minutes' WHERE id = $1")
        .bind(m)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let e = ent.retirer_message_for_actor(etab, dir, role(Role::Direction), m, None).await.expect_err("12 min : retrait refusé");
    assert!(matches!(e, EntDbError::ConflitDossier(_)), "{e:?}");
    ent.corriger_message_fil_for_actor(etab, dir, role(Role::Direction), m, "Trois, corrigé").await.expect("12 min : correction permise");
    sqlx::query("UPDATE ent_msg SET created_at = now() - interval '16 minutes' WHERE id = $1")
        .bind(m)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    assert!(ent.corriger_message_fil_for_actor(etab, dir, role(Role::Direction), m, "Trop tard").await.is_err(), "16 min : correction refusée");
}

// ───────────────────────── F-6 : jugement de lecture groupé, sans N+1 ─────────────────────────

#[tokio::test]
async fn revue_f6_jugement_groupe_equivaut_a_la_boucle_sur_tous_les_cas_et_sans_n_plus_1() {
    use super::tests_messagerie_m1::{lier, nouvel_utilisateur, ouvrir, ouvrir_un};
    use super::tests_messagerie_m3::{boite, ouvrir_boite};
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily, harry, draco) =
        (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id, cast.eleve_lea_id, cast.eleve_tom_id);
    // Un parent **sans lien communicant** avec Harry (lien de test sans droit de communication particulier) et un
    // responsable d'un autre élève : la famille n'est pas un bloc.
    let sans_lien = nouvel_utilisateur(&ent, etab, "parent", "SansLien").await;
    lier(&ent, etab, harry, sans_lien, None).await;

    let mut ids = Vec::new();
    // Plusieurs fils d'un élève (Harry), ouverts par la direction vers la famille.
    for i in 0..40 {
        ids.push(fil_de_lily(&ent, &cast, &format!("Candidat {i}")).await.0);
    }
    // Un autre élève (Draco).
    ids.extend(ouvrir(&ent, etab, Role::Direction, dir, draco, "Autre élève").await.iter().map(|f| f.fil.id));
    // Un fil **réservé** : Lily écrit à l'établissement (fil privé, lisible des seuls rôles réservés).
    let reserve = ouvrir_un(&ent, etab, Role::Parent, lily, harry, "Fil réservé").await.fil.id;
    ids.push(reserve);
    // Un fil de **boîte** : la CPE est membre de la boîte, le professeur non.
    let (b, _) = boite(&ent, etab, dir, "vie_scolaire", &[cast.cpe.user_id]).await;
    let fil_boite = ouvrir_boite(&ent, etab, lily, Role::Parent, b, harry, "Boîte").await.fil_id;
    ids.push(fil_boite);
    // Un élève **protégé** : le fil de Draco devient celui d'un élève protégé pour les tiers.
    ent.poser_confidentialite_a(
        role(Role::Direction),
        crate::eleve_confidentialite::PoserConfidentialite {
            etablissement_id: etab,
            eleve_id: draco,
            motif: "Mesure de protection".into(),
            mesures: vec![],
            du: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            au: None,
            acteur: dir,
        },
        chrono::DateTime::from_timestamp(1_780_000_000, 0).unwrap(), // « maintenant » fixé, jamais l'horloge réelle
    )
    .await
    .expect("mesure posée");

    let autre_prof = cast.profs.iter().map(|p| p.user_id).find(|u| *u != cast.prof.user_id).expect("second professeur");
    let acteurs: Vec<(Uuid, Role)> = vec![
        (lily, Role::Parent),
        (cast.parent_tom.user_id, Role::Parent),
        (sans_lien, Role::Parent),
        (dir, Role::Direction),
        (cast.prof.user_id, Role::Prof),
        (autre_prof, Role::Prof),
        (cast.cpe.user_id, Role::Cpe),
        (cast.administratif.user_id, Role::Administratif),
    ];
    let candidats = ent.messages.get_fils_lot(etab, &ids).await.unwrap();
    assert_eq!(candidats.len(), ids.len(), "une requête rend tous les fils");
    // Équivalence avec le jugement individuel, acteur par acteur et fil par fil, sur tous les cas.
    let (mut accordes, mut refuses) = (0usize, 0usize);
    for (user, r) in &acteurs {
        let lot = ent.actor_can_access_fils_lot(etab, *user, role(*r), &candidats).await.unwrap();
        for f in &candidats {
            let seul = ent.actor_can_access_fil(etab, *user, role(*r), f).await.unwrap();
            assert_eq!(lot.contains(&f.id), seul, "lot ≠ individuel pour {r:?} ({user}) sur le fil {} ({})", f.id, f.sujet);
            if seul { accordes += 1 } else { refuses += 1 }
        }
    }
    assert!(accordes > 0 && refuses > 0, "le jeu couvre des accès accordés et refusés ({accordes} / {refuses})");
    // Préconditions des cas voulus : le fil de boîte est lu par un membre, pas par un autre professeur ; le fil réservé
    // l'est par la CPE, pas par un professeur.
    let fil = |id: Uuid| candidats.iter().find(|f| f.id == id).unwrap();
    assert!(ent.actor_can_access_fil(etab, cast.cpe.user_id, role(Role::Cpe), fil(fil_boite)).await.unwrap());
    assert!(!ent.actor_can_access_fil(etab, autre_prof, role(Role::Prof), fil(fil_boite)).await.unwrap());
    assert!(!ent.actor_can_access_fil(etab, autre_prof, role(Role::Prof), fil(reserve)).await.unwrap());

    // Sans N+1 : on COMPTE les requêtes. Le jugement groupé ne dépend pas du nombre de fils (mêmes élèves, mêmes
    // boîtes) ; la boucle, elle, en émet au moins une par fil.
    let (_, groupe_peu) = compte_requetes::compter(ent.actor_can_access_fils_lot(etab, dir, role(Role::Direction), &candidats[..10])).await;
    let (_, groupe_tout) = compte_requetes::compter(ent.actor_can_access_fils_lot(etab, dir, role(Role::Direction), &candidats)).await;
    assert!(groupe_peu > 0, "le compteur voit les requêtes");
    assert_eq!(groupe_peu, groupe_tout, "personnel : même nombre de requêtes pour 10 et {} fils", candidats.len());
    let (_, boucle) = compte_requetes::compter(async {
        for f in &candidats {
            ent.actor_can_access_fil(etab, dir, role(Role::Direction), f).await.unwrap();
        }
    })
    .await;
    assert!(boucle >= candidats.len(), "la boucle émet au moins une requête par fil ({boucle})");
    assert!(groupe_tout * 5 < boucle, "groupé {groupe_tout} requêtes contre {boucle} pour la boucle");
    // Famille : une série par **élève distinct**, jamais par fil (les 40 fils de Harry coûtent comme 10).
    let harry_peu: Vec<_> = candidats.iter().filter(|f| f.eleve_id == harry).take(10).cloned().collect();
    let harry_tout: Vec<_> = candidats.iter().filter(|f| f.eleve_id == harry).cloned().collect();
    assert!(harry_tout.len() > harry_peu.len());
    let (_, fam_peu) = compte_requetes::compter(ent.actor_can_access_fils_lot(etab, lily, role(Role::Parent), &harry_peu)).await;
    let (_, fam_tout) = compte_requetes::compter(ent.actor_can_access_fils_lot(etab, lily, role(Role::Parent), &harry_tout)).await;
    assert_eq!(fam_peu, fam_tout, "famille : le coût suit les élèves, pas les fils");
}

// ───────────────────────── E1 : corriger puis annuler un différé ─────────────────────────

fn conflit_exact(e: &EntDbError, texte: &str) -> bool {
    matches!(e, EntDbError::ConflitDossier(m) if *m == texte)
}

/// Tout ce que l'annulation doit avoir fait disparaître pour ce message.
async fn restes(ent: &PgEnt, message: Uuid) -> Vec<(&'static str, i64)> {
    let mut v = Vec::new();
    for (nom, sql) in [
        ("message", "SELECT count(*) FROM ent_msg WHERE id = $1"),
        ("version", "SELECT count(*) FROM ent_msg_version WHERE message_id = $1"),
        ("carte", "SELECT count(*) FROM ent_msg_carte_document WHERE message_id = $1"),
        ("pièce", "SELECT count(*) FROM ent_msg_piece WHERE message_id = $1"),
        (
            "avis planifié",
            "SELECT count(*) FROM ent_notification_envoi WHERE source_id = $1 AND statut = 'en_attente'",
        ),
    ] {
        v.push((nom, compter(ent, sql, &[message]).await));
    }
    v
}

#[tokio::test]
async fn e1_corriger_puis_annuler_ne_remet_rien_et_ne_laisse_ni_version_ni_carte_ni_avis() {
    use crate::tests_documents_d4::W;
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = w.m.etab;
    let (dir, lily) = (w.cast.direction.user_id, w.cast.parent_lea.user_id);
    let eleve = w.eleve("Henry", "Maël").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.certificat(eleve).await;
    let fils = super::tests_messagerie_m1::ouvrir(&w.ent, etab, Role::Direction, dir, eleve, "Hésitation").await;
    let fil = fils[0].fil.id;
    assert!(ecrit(&w.ent, etab, dir, Role::Direction, fil, "Premier jet").await);
    let m = dernier_message(&w.ent, etab, fil, dir).await.id;
    w.ent
        .joindre_carte_document_for_actor(etab, dir, role(Role::Direction), m, doc.document_id)
        .await
        .expect("carte jointe");
    // Le message devient un différé (remise dans 2 h) avec son avis planifié, puis l'auteur **corrige** (il hésite).
    sqlx::query("UPDATE ent_msg SET remise_prevue_le = now() + interval '2 hours' WHERE id = $1")
        .bind(m)
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO ent_notification_envoi
            (id, etablissement_id, destinataire_user_id, canal, kind, titre, corps, eleve_id,
             source_type, source_id, cle_dedup, droit_requis, planifie_pour)
         VALUES ($1,$2,$3,'push','messagerie','Nouveau message','Un message vous attend',$4,'systeme',$5,$6,'lien', now() + interval '2 hours')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(lily)
    .bind(eleve)
    .bind(m)
    .bind(format!("msg-avis:{m}"))
    .execute(w.ent.pool_verif())
    .await
    .unwrap();
    w.ent
        .corriger_message_fil_for_actor(etab, dir, role(Role::Direction), m, "Deuxième jet")
        .await
        .expect("correction d'un différé");
    let avant = restes(&w.ent, m).await;
    assert_eq!(avant, vec![("message", 1), ("version", 1), ("carte", 1), ("pièce", 0), ("avis planifié", 1)], "précondition");
    // Puis il **annule** : l'annulation est totale.
    w.ent
        .annuler_message_differe_for_actor(etab, dir, role(Role::Direction), m)
        .await
        .expect("annulation après correction (E1)");
    assert_eq!(
        restes(&w.ent, m).await,
        vec![("message", 0), ("version", 0), ("carte", 0), ("pièce", 0), ("avis planifié", 0)],
        "plus rien : ni message, ni version, ni carte, ni avis planifié"
    );
    // Rien n'est remis : même une fois l'échéance passée, la famille ne voit aucun message de ce fil.
    let (_, lues) = w.ent.list_messages_for_actor(etab, lily, role(Role::Parent), fil).await.unwrap();
    assert!(lues.iter().all(|l| l.message.id != m && l.message.corps != "Deuxième jet"), "aucune remise");
    let avis_annule = compter(
        &w.ent,
        "SELECT count(*) FROM ent_notification_envoi WHERE source_id = $1 AND statut = 'annule'",
        &[m],
    )
    .await;
    assert_eq!(avis_annule, 1, "l'avis planifié est annulé, pas laissé en attente");
}

#[tokio::test]
async fn e1_corriger_deux_fois_puis_annuler() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Deux corrections").await;
    let m = message_differe(&ent, etab, dir, fil, "Version 1").await;
    for texte in ["Version 2", "Version 3"] {
        ent.corriger_message_fil_for_actor(etab, dir, role(Role::Direction), m, texte).await.expect("correction");
    }
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_msg_version WHERE message_id = $1", &[m]).await, 2, "précondition");
    ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), m).await.expect("annulation");
    assert_eq!(
        restes(&ent, m).await,
        vec![("message", 0), ("version", 0), ("carte", 0), ("pièce", 0), ("avis planifié", 0)]
    );
    // Les versions d'un message remis restent en ajout seul, même avec le réglage d'annulation (garde bornée).
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Remis").await);
    let remis = dernier_message(&ent, etab, fil, dir).await.id;
    ent.corriger_message_fil_for_actor(etab, dir, role(Role::Direction), remis, "Remis, corrigé").await.expect("correction");
    let mut tx = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.msg_annulation', $1, true)").bind(remis.to_string()).execute(&mut *tx).await.unwrap();
    let r = sqlx::query("DELETE FROM ent_msg_version WHERE message_id = $1").bind(remis).execute(&mut *tx).await;
    assert!(r.is_err(), "l'exception ne couvre pas un message déjà remis");
}

#[tokio::test]
async fn e1_annuler_apres_la_remise_est_refuse_explicitement() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Trop tard").await;
    let m = message_differe(&ent, etab, dir, fil, "Parti").await;
    // L'échéance est passée : le message est remis.
    sqlx::query("UPDATE ent_msg SET remise_prevue_le = now() - interval '1 second' WHERE id = $1")
        .bind(m)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let e = ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), m).await.expect_err("refus");
    assert!(conflit_exact(&e, "message déjà remis : retire-le"), "{e:?}");
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_msg WHERE id = $1", &[m]).await, 1, "rien n'est supprimé");
    // La voie indiquée marche : le retrait.
    ent.retirer_message_for_actor(etab, dir, role(Role::Direction), m, None).await.expect("retire-le");
}

#[tokio::test]
async fn e1_si_la_base_refuse_l_annulation_elle_n_a_pas_lieu_et_l_auteur_le_sait() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Refus de la base").await;
    let m = message_differe(&ent, etab, dir, fil, "Version 1").await;
    ent.corriger_message_fil_for_actor(etab, dir, role(Role::Direction), m, "Version 2").await.expect("correction");
    // Une référence que l'annulation ne connaît pas (table de test) : la suppression du message viole une clé étrangère.
    let table = format!("e1_ref_{}", Uuid::new_v4().simple());
    sqlx::query(&format!(
        "CREATE TABLE {table} (etablissement_id UUID NOT NULL, message_id UUID NOT NULL,
            FOREIGN KEY (etablissement_id, message_id) REFERENCES ent_msg (etablissement_id, id))"
    ))
    .execute(ent.pool_verif())
    .await
    .unwrap();
    sqlx::query(&format!("INSERT INTO {table} VALUES ($1, $2)")).bind(etab).bind(m).execute(ent.pool_verif()).await.unwrap();
    let e = ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), m).await.expect_err("refus");
    sqlx::query(&format!("DROP TABLE {table}")).execute(ent.pool_verif()).await.unwrap();
    assert!(
        conflit_exact(&e, "annulation refusée : le message n'a pas été annulé et sera remis normalement"),
        "erreur métier exploitable, pas ReferenceInconnue : {e:?}"
    );
    assert!(!matches!(e, EntDbError::Domain(DomainError::ReferenceInconnue)));
    // Rien n'a été supprimé (tout ou rien) : le message, sa version, et l'auteur peut réessayer ou le retirer.
    assert_eq!(restes(&ent, m).await[..2], [("message", 1), ("version", 1)]);
    ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), m).await.expect("la référence a disparu : annulation");
}
