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
use chrono::{DateTime, Duration, Utc};
use domain_eleve::{DomainError, MsgPiece};
use ent_auth::Role;
use uuid::Uuid;

async fn maintenant(ent: &PgEnt) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT now()").fetch_one(ent.pool_verif()).await.unwrap()
}

async fn flux(
    ent: &PgEnt,
    etab: Uuid,
    user: Uuid,
    r: Role,
    apres: DateTime<Utc>,
) -> crate::FluxMessagerie {
    ent.flux_messagerie_for_actor(etab, user, role(r), Some((apres, None, None)), 0).await.expect("flux")
}

async fn fils_du_flux(ent: &PgEnt, etab: Uuid, user: Uuid, r: Role, apres: DateTime<Utc>) -> Vec<Uuid> {
    flux(ent, etab, user, r, apres).await.fils.into_iter().map(|f| f.fil_id).collect()
}

/// Recule **tous** les messages de ces fils (l'événement d'un fil est celui de son dernier message) à `maintenant − secondes + i s` (i = rang du fil) : des instants
/// distincts, déjà stables (au-delà de la marge de 5 s) ; remplace l'attente du temps réel.
async fn reculer_nouveaux(ent: &PgEnt, etab: Uuid, fils: &[Uuid], secondes: i64) {
    for (i, f) in fils.iter().enumerate() {
        sqlx::query(
            "UPDATE ent_msg SET created_at = now() - make_interval(secs => $4) + make_interval(secs => $3)
              WHERE etablissement_id = $1 AND fil_id = $2",
        )
        .bind(etab)
        .bind(f)
        .bind(i as f64)
        .bind(secondes as f64)
        .execute(ent.pool_verif())
        .await
        .unwrap();
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
    let avant = maintenant(&ent).await;
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Après l'ajout").await);
    // Témoin : Lily lit et reçoit.
    assert!(lit(&ent, etab, lily, Role::Parent, fil).await);
    assert_eq!(fils_du_flux(&ent, etab, lily, Role::Parent, avant).await, vec![fil], "témoin : la famille participante");
    // Le flux n'annonce que ce que la lecture accorde, pour chaque compte, avec son rôle de session.
    for (qui, r) in [(prof, Role::Prof), (cast.cpe.user_id, Role::Cpe), (cast.administratif.user_id, Role::Administratif)] {
        let peut_lire = lit(&ent, etab, qui, r, fil).await;
        let recu = fils_du_flux(&ent, etab, qui, r, avant).await.contains(&fil);
        assert!(!recu || peut_lire, "le flux révèle un fil que la lecture refuse ({r:?})");
    }
    assert!(!lit(&ent, etab, prof, Role::Prof, fil).await, "toujours sans droit après l'ajout");
    assert!(fils_du_flux(&ent, etab, prof, Role::Prof, avant).await.is_empty(), "participant sans droit : rien");
    // Le même compte avec un rôle de session qui n'est pas celui de sa lecture ne reçoit rien non plus : le rôle
    // transmis est celui de la session (un compte « parent » n'est pas jugé comme un personnel).
    assert!(fils_du_flux(&ent, etab, prof, Role::Parent, avant).await.is_empty());

    // Droit retiré : une interdiction de communication vise Lily, la lecture se ferme, le flux se tait aussitôt.
    poser_interdiction(&ent, etab, harry, lily, dir).await;
    assert!(!lit(&ent, etab, lily, Role::Parent, fil).await, "lecture refusée après l'interdiction");
    assert!(fils_du_flux(&ent, etab, lily, Role::Parent, avant).await.is_empty(), "plus rien dans le flux");
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
    let avant = maintenant(&ent).await - Duration::seconds(120);
    for f in &fils {
        assert!(ecrit(&ent, etab, dir, Role::Direction, *f, "Nouveau").await);
    }
    // Instants explicites et distincts, au-delà de la marge de 5 s (aucune dépendance au temps qui passe).
    reculer_nouveaux(&ent, etab, &fils, 60).await;
    // Pagination par limite 2 : chaque appel rend la position du dernier événement rendu et `tronque`.
    let mut vus = Vec::new();
    let mut pos = (avant, None, None);
    let mut tours = 0;
    let mut tronquees = 0;
    // Le client reprend toujours depuis la position rendue (tronquée ou non) jusqu'à avoir tout reçu ; le labo est
    // partagé : d'autres fils d'autres tests défilent aussi, on s'arrête quand les nôtres sont tous passés.
    while fils.iter().any(|f| !vus.contains(f)) {
        let r = ent
            .flux_messagerie_limite(etab, lily, role(Role::Parent), Some(pos), 0, 2)
            .await
            .expect("flux");
        assert!(r.fils.len() <= 2);
        vus.extend(r.fils.iter().map(|f| f.fil_id));
        tours += 1;
        assert!(tours < 400, "pas de boucle (le curseur progresse toujours)");
        if r.tronque {
            tronquees += 1;
            assert_eq!(r.fils.len(), 2, "une liste tronquée est pleine");
            assert!(r.curseur_fil.is_some(), "la position porte le fil du dernier événement rendu");
            assert_eq!(r.curseur, r.fils.last().unwrap().instant, "curseur = instant du dernier événement rendu");
        }
        pos = (r.curseur, r.curseur_fil, r.curseur_genre.clone());
    }
    assert!(tronquees >= 1, "la liste a bien été tronquée");
    let mut attendus = fils.clone();
    attendus.sort();
    vus.retain(|f| attendus.contains(f)); // le labo est partagé : d'autres tests écrivent dans d'autres fils
    vus.sort();
    assert_eq!(vus, attendus, "aucun événement sauté, aucun doublon");

    // Égalité d'instant : quatre messages posés au même instant exact ne se perdent pas à la coupure.
    let t = maintenant(&ent).await - Duration::seconds(30);
    sqlx::query("UPDATE ent_msg SET created_at = $2 WHERE etablissement_id = $1 AND fil_id = ANY($3)")
        .bind(etab)
        .bind(t)
        .bind(&fils)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let mut vus = Vec::new();
    let mut pos = (avant, None, None);
    for tour in 0..400 {
        if fils.iter().all(|f| vus.contains(f)) {
            break;
        }
        assert!(tour < 399, "pas de boucle");
        let r = ent.flux_messagerie_limite(etab, lily, role(Role::Parent), Some(pos), 0, 1).await.unwrap();
        vus.extend(r.fils.iter().map(|f| f.fil_id));
        pos = (r.curseur, r.curseur_fil, r.curseur_genre.clone());
    }
    vus.retain(|f| attendus.contains(f));
    vus.sort();
    assert_eq!(vus, attendus, "quatre événements au même instant : tous rendus");

    // Fenêtre maximale : un curseur plus vieux qu'elle demande la resynchronisation complète.
    let vieux = maintenant(&ent).await - Duration::seconds(crate::msg_repo::FLUX_FENETRE_SECONDES as i64 + 60);
    let r = flux(&ent, etab, lily, Role::Parent, vieux).await;
    assert!(r.resynchro && r.fils.is_empty() && !r.tronque);
    let r = flux(&ent, etab, lily, Role::Parent, maintenant(&ent).await - Duration::seconds(30)).await;
    assert!(!r.resynchro, "témoin : un curseur récent ne l'exige pas");
}

// ───────────────────────── M3 : une seule horloge, aucun message perdu ─────────────────────────

#[tokio::test]
async fn revue_m3_message_valide_tard_n_est_pas_perdu() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Validé tard").await;
    let avant = maintenant(&ent).await;
    // Une transaction écrit un message (horodaté par la base à son début) et **ne valide qu'après** la marge du
    // curseur : un flux interrogé entre-temps ne doit pas avancer son curseur au-delà de ce message.
    let mut tx = TenantTx::begin(ent.pool(), etab).await.unwrap();
    let mid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_msg (id, etablissement_id, fil_id, auteur_user_id, corps, created_at)
         VALUES ($1, $2, $3, $4, 'Validé tard', now())",
    )
    .bind(mid)
    .bind(etab)
    .bind(fil)
    .bind(dir)
    .execute(&mut *tx)
    .await
    .unwrap();
    let debut_tx: DateTime<Utc> = sqlx::query_scalar("SELECT created_at FROM ent_msg WHERE id = $1")
        .bind(mid)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    // Le message est invisible tant que la transaction n'est pas validée.
    let r1 = flux(&ent, etab, lily, Role::Parent, avant).await;
    // (le fil a un premier message déjà remis : on regarde l'instant, jamais le seul identifiant du fil)
    assert!(r1.fils.iter().filter(|f| f.fil_id == fil).all(|f| f.instant < debut_tx));
    assert!(r1.curseur <= debut_tx, "le curseur ne dépasse pas le début de la plus ancienne transaction en cours");
    // La plus ancienne transaction en cours est **visible** du rôle de l'application et débute au plus tard avec ce
    // message : la borne sûre ne peut donc pas la dépasser (la marge de 5 s ne sert que de plancher). Aucune attente.
    let ancienne = ent.messages.debut_plus_ancienne_transaction().await.unwrap();
    assert!(ancienne.is_some_and(|t| t <= debut_tx), "transaction en cours visible : {ancienne:?} / {debut_tx}");
    assert!(ent.verifier_flux_messagerie().await, "le rôle de l'application voit les transactions des autres sessions");
    let r2 = flux(&ent, etab, lily, Role::Parent, r1.curseur).await;
    assert!(
        r2.fils.iter().filter(|f| f.fil_id == fil).all(|f| f.instant < debut_tx),
        "le message en cours de validation reste invisible : {r2:?}"
    );
    assert!(r2.curseur <= debut_tx, "le curseur reste au plus au début de la transaction en cours : {} > {debut_tx}", r2.curseur);
    tx.commit().await.unwrap();
    // Après la validation, le message paraît à partir du curseur rendu pendant la transaction.
    let r3 = flux(&ent, etab, lily, Role::Parent, r2.curseur).await;
    assert!(
        r3.fils.iter().any(|f| f.fil_id == fil && f.instant >= debut_tx),
        "message validé tard : jamais perdu ({r3:?})"
    );
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
    let avant = maintenant(&ent).await;
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Corps secret du retrait").await);
    // L'auteur ne reçoit pas l'annonce de son propre message ; le destinataire, si.
    assert!(fils_du_flux(&ent, etab, dir, Role::Direction, avant).await.is_empty(), "l'auteur n'est pas averti de lui-même");
    let f = flux(&ent, etab, lily, Role::Parent, avant).await;
    assert_eq!(f.fils.len(), 1);
    assert_eq!(f.fils[0].genre, "nouveau");
    // Retrait : signalé au destinataire (genre `retire`), sans contenu ; pas à celui qui retire.
    let m = dernier_message(&ent, etab, fil, dir).await;
    let apres_envoi = maintenant(&ent).await;
    ent.retirer_message_for_actor(etab, dir, role(Role::Direction), m.id, None).await.expect("retrait");
    let f = flux(&ent, etab, lily, Role::Parent, apres_envoi).await;
    assert_eq!(f.fils.len(), 1, "{f:?}");
    assert_eq!((f.fils[0].fil_id, f.fils[0].genre.as_str()), (fil, "retire"));
    let json = serde_json::to_string(&f).unwrap();
    assert!(!json.contains("secret") && !json.contains("retrait"), "aucun contenu : {json}");
    assert!(fils_du_flux(&ent, etab, dir, Role::Direction, apres_envoi).await.is_empty(), "celui qui retire n'est pas averti");
    // Un message retiré ne compte plus comme « nouveau » : depuis `avant`, un seul événement (le retrait).
    let tout = flux(&ent, etab, lily, Role::Parent, avant).await;
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

// ───────────────────────── M-A : une transaction longue ne fige pas le curseur ─────────────────────────

#[test]
fn borne_sure_progresse_toujours_et_ne_depasse_jamais_la_fenetre() {
    use crate::msg_repo::{borne_sure, FLUX_FENETRE_SECONDES};
    let t = Utc::now(); // instant quelconque : la fonction est pure, aucune horloge n'est lue
    let s = Duration::seconds;
    assert_eq!(borne_sure(t, None), t - s(5), "aucune transaction visible : marge de 5 s");
    assert_eq!(borne_sure(t, Some(t - s(2))), t - s(5), "transaction récente : la marge");
    assert_eq!(borne_sure(t, Some(t - s(30))), t - s(30), "transaction de 30 s : la borne la suit (rien n'est sauté)");
    // Transaction plus vieille que la fenêtre (autovacuum, sauvegarde, session « idle in transaction ») : la borne
    // ne recule pas au-delà du plancher, donc un curseur rendu reste **dans** la fenêtre et ne déclenche pas de
    // resynchronisation en boucle.
    for age in [FLUX_FENETRE_SECONDES as i64 + 1, 3_600, 86_400] {
        let b = borne_sure(t, Some(t - s(age)));
        assert!(b > t - s(FLUX_FENETRE_SECONDES as i64), "borne dans la fenêtre ({age} s) : {}", (t - b).num_seconds());
        assert!(b <= t - s(5));
    }
}

#[tokio::test]
async fn revue_ma_curseur_progresse_avec_une_transaction_ouverte_sans_resynchro_en_boucle() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, lily) = (cast.etablissement_id, cast.parent_lea.user_id);
    // Une transaction reste ouverte (aucune écriture) pendant tout le test.
    let mut longue = ent.pool().begin().await.unwrap();
    sqlx::query("SELECT 1").execute(&mut *longue).await.unwrap();
    let mut position = maintenant(&ent).await - Duration::seconds(300);
    let mut tours = 0;
    loop {
        let r = flux(&ent, etab, lily, Role::Parent, position).await;
        assert!(!r.resynchro, "une transaction ouverte ne provoque pas de resynchronisation");
        assert!(r.curseur >= position, "le curseur ne recule pas : {} < {position}", r.curseur);
        let progresse = r.curseur > position;
        position = r.curseur;
        tours += 1;
        if progresse || tours >= 3 {
            assert!(progresse, "le curseur progresse malgré la transaction ouverte");
            break;
        }
    }
    longue.rollback().await.unwrap();
}

// ───────────────────────── M-B : la troncature ne saute jamais un message validé tard ─────────────────────────

#[tokio::test]
async fn revue_mb_troncature_borne_le_curseur_par_la_limite_sure() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let mut fils = Vec::new();
    for i in 0..4 {
        fils.push(fil_de_lily(&ent, &cast, &format!("Borne {i}")).await.0);
    }
    let avant = maintenant(&ent).await - Duration::seconds(120);
    for f in &fils {
        assert!(ecrit(&ent, etab, dir, Role::Direction, *f, "Nouveau").await);
    }
    // Les quatre événements sont **dans la marge de 5 s** (instants récents, après la limite sûre) : une liste
    // tronquée ne peut pas rendre un curseur qui les dépasse (un message plus ancien, validé plus tard, serait sauté).
    let r = ent.flux_messagerie_limite(etab, lily, role(Role::Parent), Some((avant, None, None)), 0, 2).await.unwrap();
    let borne_max = maintenant(&ent).await - Duration::seconds(5);
    assert!(r.curseur <= borne_max, "curseur borné par la limite sûre : {} > {borne_max}", r.curseur);
    assert!(r.fils.iter().all(|f| f.instant <= r.curseur), "rien au-delà du curseur rendu");
    // Le temps passe (instants reculés : aucune attente) : tout paraît, une fois, jusqu'au bout, sans boucle.
    reculer_nouveaux(&ent, etab, &fils, 60).await;
    let mut vus = Vec::new();
    let mut pos = (avant, None, None);
    for tour in 0..400 {
        if fils.iter().all(|f| vus.contains(f)) {
            break;
        }
        assert!(tour < 399, "pas de boucle");
        let r = ent.flux_messagerie_limite(etab, lily, role(Role::Parent), Some(pos), 0, 2).await.unwrap();
        vus.extend(r.fils.iter().map(|f| f.fil_id));
        pos = (r.curseur, r.curseur_fil, r.curseur_genre.clone());
    }
    vus.retain(|f| fils.contains(f));
    vus.sort();
    let mut attendus = fils.clone();
    attendus.sort();
    assert_eq!(vus, attendus, "aucun événement sauté");
}

// ───────────────────────── F-4 : curseur (instant, fil, genre) complet ─────────────────────────

#[tokio::test]
async fn revue_f4_curseur_a_trois_termes_aucun_saut_entre_genres() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Genres").await;
    let avant = maintenant(&ent).await - Duration::seconds(120);
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Nouveau").await);
    let nouveau = dernier_message(&ent, etab, fil, dir).await.id;
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "À retirer").await);
    let a_retirer = dernier_message(&ent, etab, fil, dir).await.id;
    ent.retirer_message_for_actor(etab, dir, role(Role::Direction), a_retirer, None).await.expect("retrait");
    // Un événement « nouveau » et un événement « retiré » au **même instant et pour le même fil**.
    let t = maintenant(&ent).await - Duration::seconds(30);
    // Tous les autres messages du fil au même instant : l'agrégat (dernier instant par fil et genre) vaut `t`.
    sqlx::query("UPDATE ent_msg SET created_at = $2 WHERE fil_id = $1 AND id <> $3")
        .bind(fil)
        .bind(t)
        .bind(a_retirer)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    assert!(nouveau != a_retirer);
    sqlx::query("UPDATE ent_msg SET created_at = $2 - interval '1 hour', retire_le = $2 WHERE id = $1")
        .bind(a_retirer)
        .bind(t)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let mut genres = Vec::new();
    let mut pos = (avant, None, None);
    for tour in 0..400 {
        if genres.len() >= 2 {
            break;
        }
        assert!(tour < 399, "pas de boucle");
        let r = ent.flux_messagerie_limite(etab, lily, role(Role::Parent), Some(pos), 0, 1).await.unwrap();
        genres.extend(r.fils.iter().filter(|f| f.fil_id == fil && f.instant == t).map(|f| f.genre.clone()));
        pos = (r.curseur, r.curseur_fil, r.curseur_genre.clone());
    }
    genres.sort();
    assert_eq!(genres, vec!["nouveau".to_string(), "retire".to_string()], "les deux genres au même (instant, fil) sont rendus");
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
async fn revue_f6_jugement_groupe_equivaut_a_la_boucle_et_va_plus_vite() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let mut ids = Vec::new();
    for i in 0..200 {
        ids.push(fil_de_lily(&ent, &cast, &format!("Candidat {i}")).await.0);
    }
    // Un fil d'un autre élève (Tom) : un parent de Léa ne le lit pas.
    let acteurs: Vec<(Uuid, Role)> = vec![
        (cast.parent_lea.user_id, Role::Parent),
        (cast.parent_tom.user_id, Role::Parent),
        (dir, Role::Direction),
        (cast.prof.user_id, Role::Prof),
        (cast.profs.iter().map(|p| p.user_id).find(|u| *u != cast.prof.user_id).unwrap(), Role::Prof),
        (cast.cpe.user_id, Role::Cpe),
        (cast.administratif.user_id, Role::Administratif),
    ];
    let candidats = ent.messages.get_fils_lot(etab, &ids).await.unwrap();
    assert_eq!(candidats.len(), 200, "une requête rend les 200 fils");
    // Équivalence avec le jugement individuel, pour chaque type d'acteur, sur 30 fils.
    for (user, r) in &acteurs {
        let echantillon = &candidats[..30];
        let lot = ent.actor_can_access_fils_lot(etab, *user, role(*r), echantillon).await.unwrap();
        for f in echantillon {
            let seul = ent.actor_can_access_fil(etab, *user, role(*r), f).await.unwrap();
            assert_eq!(lot.contains(&f.id), seul, "lot ≠ individuel pour {r:?} sur le fil {}", f.id);
        }
    }
    // Temps mesuré pour 200 candidats : le jugement groupé est nettement plus rapide que la boucle (N+1).
    let t0 = std::time::Instant::now();
    let lot = ent.actor_can_access_fils_lot(etab, dir, role(Role::Direction), &candidats).await.unwrap();
    let duree_lot = t0.elapsed();
    let t1 = std::time::Instant::now();
    let mut seuls = 0;
    for f in &candidats {
        if ent.actor_can_access_fil(etab, dir, role(Role::Direction), f).await.unwrap() {
            seuls += 1;
        }
    }
    let duree_boucle = t1.elapsed();
    eprintln!("F-6 : 200 candidats, groupé {duree_lot:?}, boucle {duree_boucle:?}");
    assert_eq!(lot.len(), seuls);
    assert!(duree_lot < duree_boucle, "groupé {duree_lot:?} doit battre la boucle {duree_boucle:?}");
    assert!(duree_lot < std::time::Duration::from_secs(2), "200 candidats en moins de 2 s : {duree_lot:?}");
}
