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
    ent.flux_messagerie_for_actor(etab, user, role(r), Some((apres, None)), 0).await.expect("flux")
}

async fn fils_du_flux(ent: &PgEnt, etab: Uuid, user: Uuid, r: Role, apres: DateTime<Utc>) -> Vec<Uuid> {
    flux(ent, etab, user, r, apres).await.fils.into_iter().map(|f| f.fil_id).collect()
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
    let prof = cast.prof.user_id;
    // Le professeur devient participant du fil sans y avoir le droit de lecture (participation seule : ni l'élève
    // suivi, ni le pont des rôles réservés) : la vue de participation le voit, le jugement de lecture le refuse.
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
    if !lit(&ent, etab, prof, Role::Prof, fil).await {
        assert!(fils_du_flux(&ent, etab, prof, Role::Prof, avant).await.is_empty(), "participant sans droit : rien");
    }
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
    let avant = maintenant(&ent).await;
    for f in &fils {
        assert!(ecrit(&ent, etab, dir, Role::Direction, *f, "Nouveau").await);
    }
    // Pagination par limite 2 : chaque appel rend la position du dernier événement rendu et `tronque`.
    let mut vus = Vec::new();
    let mut pos = (avant, None);
    let mut tours = 0;
    loop {
        let r = ent
            .flux_messagerie_limite(etab, lily, role(Role::Parent), Some(pos), 0, 2)
            .await
            .expect("flux");
        assert!(r.fils.len() <= 2);
        vus.extend(r.fils.iter().map(|f| f.fil_id));
        tours += 1;
        if !r.tronque {
            break;
        }
        assert_eq!(r.fils.len(), 2, "une liste tronquée est pleine");
        assert!(r.curseur_fil.is_some(), "la position porte le fil du dernier événement rendu");
        assert_eq!(r.curseur, r.fils.last().unwrap().instant, "curseur = instant du dernier événement rendu");
        pos = (r.curseur, r.curseur_fil);
        assert!(tours < 50, "pas de boucle");
    }
    assert!(tours >= 2, "la liste a bien été tronquée");
    let mut attendus = fils.clone();
    attendus.sort();
    vus.retain(|f| attendus.contains(f)); // le labo est partagé : d'autres tests écrivent dans d'autres fils
    vus.sort();
    assert_eq!(vus, attendus, "aucun événement sauté, aucun doublon");

    // Égalité d'instant : quatre messages posés au même instant exact ne se perdent pas à la coupure.
    let t = maintenant(&ent).await;
    sqlx::query("UPDATE ent_msg SET created_at = $2 WHERE etablissement_id = $1 AND fil_id = ANY($3) AND corps = 'Nouveau'")
        .bind(etab)
        .bind(t)
        .bind(&fils)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let mut vus = Vec::new();
    let mut pos = (avant, None);
    for _ in 0..50 {
        let r = ent.flux_messagerie_limite(etab, lily, role(Role::Parent), Some(pos), 0, 1).await.unwrap();
        vus.extend(r.fils.iter().map(|f| f.fil_id));
        if !r.tronque {
            break;
        }
        pos = (r.curseur, r.curseur_fil);
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
    tokio::time::sleep(std::time::Duration::from_secs(6)).await; // plus que la marge de 5 s
    let r2 = flux(&ent, etab, lily, Role::Parent, r1.curseur).await;
    assert!(
        r2.fils.iter().filter(|f| f.fil_id == fil).all(|f| f.instant < debut_tx),
        "le message en cours de validation reste invisible : {r2:?}"
    );
    assert!(r2.curseur <= debut_tx, "le curseur reste au début de la transaction en cours : {} > {debut_tx}", r2.curseur);
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
