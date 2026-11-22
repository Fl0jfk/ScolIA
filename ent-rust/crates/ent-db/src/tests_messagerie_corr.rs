//! Messagerie, correctifs de la revue de sécurité du lot M5-M6 (`docs/socles/16_MESSAGERIE_M5_M6_LIVRAISON.md`,
//! tableau des correctifs) : un **témoin** précède chaque attaque, chaque scénario sème son propre établissement, aucun
//! test ne dépend de l'heure réelle (les instants sont posés par SQL à partir de l'horloge de la base, jamais comparés
//! à un jour UTC).
//!
//! | Point | Test(s) |
//! |---|---|
//! | M1 : fil C3, aucun événement avec élève ni lecteur | `corr_m1_fil_c3_aucun_evenement_ne_porte_l_eleve_ni_un_lecteur` |
//! | M2 : 404 uniforme sur les routes de lecture d'un fil | `corr_m2_404_uniforme_sur_les_routes_de_lecture_du_fil` (+ HTTP : `recette_personas_1`) |
//! | M3 : avis `reponse_attendue` relu par destinataire, C3 sans élève | `corr_m3_avis_reponse_attendue_relit_le_droit_de_chaque_destinataire` |
//! | M4 : annulation d'un message différé (cartes, accès, lectures, objets) ; pointeur sans auteur ni différé | `corr_m4_annulation_d_un_message_differe_avec_carte_piece_et_lecture`, `corr_m4_pointeur_exclut_ses_propres_messages` |
//! | M5 : purge des pièces refusées (7 / 30 jours) | `corr_m5_purge_des_pieces_infectees_et_rejetees` |
//! | M8 : lien de 60 s, via l'application, ouverture journalisée et relue | `corr_m8_lien_borne_via_l_application_ouverture_journalisee` |
//! | F1 : 5 pièces sous verrou, message retiré ou différé | `corr_f1_cinq_pieces_sous_verrou_et_message_retire_ou_differe` |
//! | F2 : `lu_par_personne_id` à la première lecture | `corr_f2_lu_par_n_est_pose_qu_a_la_premiere_lecture` |
//! | F3 : photo sans EXIF ni GPS, orientation gardée | `corr_f3_photo_sans_gps_orientation_conservee` |
//! | F4 : élève protégé (E7-E8), pièces, cartes, avis | `corr_f4_eleve_protege_pieces_cartes_et_avis` |
//! | F5 : type et nom de la carte selon le document réel | `corr_f5_carte_servie_selon_le_document_reel` |
//! | F9 : une erreur sur un établissement n'arrête pas la passe | `corr_f9_la_passe_continue_malgre_un_etablissement_en_echec` |

use super::tests_messagerie_m0a::poser_interdiction;
use super::tests_messagerie_m1::{compter, ecrit, labo, lit, ouvrir, role};
use super::tests_messagerie_m3::{boite, ouvrir_boite, personne_de, rep};
use super::tests_messagerie_m5::{
    deposer, est_conflit, est_introuvable, etat_piece, fil_de_lily, lever_quarantaine, nb_acces, EICAR, PDF_CHIFFRE,
    PDF_REEL, PHOTO_GPS,
};
use super::tests_messagerie_m6::{cree_le, dernier_message, pointeurs};
use super::*;
use chrono::Duration;
use ent_auth::Role;
use uuid::Uuid;

async fn cles_piece(ent: &PgEnt, piece: Uuid) -> (String, Option<String>) {
    sqlx::query_as("SELECT storage_key, cle_objet_servi FROM ent_msg_piece WHERE id = $1")
        .bind(piece)
        .fetch_one(ent.pool_verif())
        .await
        .unwrap()
}

async fn objet_supprime_au_registre(ent: &PgEnt, etab: Uuid, cle: &str) -> bool {
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_objet WHERE etablissement_id = $1 AND cle_objet = $2 AND supprime_le IS NOT NULL",
    )
    .bind(etab)
    .bind(cle)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    n == 1
}

async fn vieillir_analyse(ent: &PgEnt, piece: Uuid, jours: i32) {
    sqlx::query(
        "UPDATE ent_msg_piece SET analysee_le = now() - make_interval(days => $2),
                derniere_analyse_le = now() - make_interval(days => $2) WHERE id = $1",
    )
    .bind(piece)
    .bind(jours)
    .execute(ent.pool_verif())
    .await
    .unwrap();
}

// ───────────────────────── M1 : fil C3 ─────────────────────────

#[tokio::test]
async fn corr_m1_fil_c3_aucun_evenement_ne_porte_l_eleve_ni_un_lecteur() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, inf) = (cast.direction.user_id, cast.infirmieres[0].user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (b, _) = boite(&ent, etab, dir, "infirmerie", &[inf]).await;
    // La famille écrit à l'infirmerie (fil C3), l'infirmière lit (accusé), répond, la famille lit et dépose une pièce.
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, b, harry, "Ventre").await;
    assert!(lit(&ent, etab, inf, Role::Infirmerie, o.fil_id).await, "témoin : l'infirmière lit");
    ent.repondre_au_nom_de_boite_for_actor(role(Role::Infirmerie), rep(etab, inf, o.fil_id, "Je passe."), b)
        .await
        .unwrap();
    assert!(lit(&ent, etab, lily, Role::Parent, o.fil_id).await, "témoin : la famille lit la réponse");
    let meta = deposer(&ent, etab, o.message_id, lily, Role::Parent, "ordonnance.pdf", "application/pdf", PDF_REEL)
        .await
        .expect("témoin : la famille dépose une pièce");
    lever_quarantaine(&ent, etab).await;
    assert!(lit(&ent, etab, inf, Role::Infirmerie, o.fil_id).await);
    assert!(meta.id != Uuid::nil());

    let types: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT type FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2",
    )
    .bind(etab)
    .bind(o.fil_id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    for attendu in ["msg.piece", "messagerie.piece_analysee"] {
        assert!(types.iter().any(|t| t == attendu), "témoin : événement {attendu} publié ({types:?})");
    }
    let avec_eleve = compter(
        &ent,
        "SELECT count(*) FROM ent_metier_event WHERE aggregate_id = $1 AND eleve_id IS NOT NULL",
        &[o.fil_id],
    )
    .await;
    assert_eq!(avec_eleve, 0, "aucun événement d'un fil C3 ne porte l'élève : {types:?}");
    let charges: Vec<String> = sqlx::query_scalar(
        "SELECT payload::text || ' ' || coalesce(actor_user_id::text, '')
           FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2 AND type = 'msg.accuse'",
    )
    .bind(etab)
    .bind(o.fil_id)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    for c in &charges {
        assert!(!c.contains("lecteur") && !c.contains(&inf.to_string()) && !c.contains(&lily.to_string()), "{c}");
    }

    // Non-régression : sur un fil ordinaire, l'événement garde l'élève, mais plus de lecteur dans la charge utile.
    let (fil, _) = fil_de_lily(&ent, &cast, "Ordinaire").await;
    assert!(lit(&ent, etab, lily, Role::Parent, fil).await);
    let ordinaires: Vec<(Option<Uuid>, String)> = sqlx::query_as(
        "SELECT eleve_id, payload::text FROM ent_metier_event WHERE aggregate_id = $1 AND type = 'msg.accuse'",
    )
    .bind(fil)
    .fetch_all(ent.pool_verif())
    .await
    .unwrap();
    assert!(!ordinaires.is_empty(), "témoin : un accusé a été publié");
    assert!(ordinaires.iter().all(|(e, p)| e.is_some() && !p.contains("lecteur")), "{ordinaires:?}");
}

// ───────────────────────── M2 : 404 uniforme ─────────────────────────

#[tokio::test]
async fn corr_m2_404_uniforme_sur_les_routes_de_lecture_du_fil() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (lily, tom, prof) = (cast.parent_lea.user_id, cast.parent_tom.user_id, cast.prof.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Privé").await;
    let absent = Uuid::new_v4();

    // Chaque route de lecture d'un fil, appelée par un acteur donné, vue comme `Result<(), EntDbError>`.
    async fn route(ent: &PgEnt, nom: &str, etab: Uuid, user: Uuid, r: Role, fil: Uuid) -> Result<(), EntDbError> {
        match nom {
            "fil" => ent.list_messages_for_actor(etab, user, role(r), fil).await.map(|_| ()),
            "cartes" => ent.cartes_du_fil_for_actor(etab, user, role(r), fil).await.map(|_| ()),
            "lecture" => ent.lecture_du_fil_for_actor(etab, user, role(r), fil).await.map(|_| ()),
            "lu" => ent.marquer_lu_for_actor(etab, user, role(r), fil, None).await.map(|_| ()),
            autre => panic!("route inconnue : {autre}"),
        }
    }
    for nom in ["fil", "cartes", "lecture", "lu"] {
        route(&ent, nom, etab, lily, Role::Parent, fil)
            .await
            .unwrap_or_else(|e| panic!("témoin : Lily lit son fil (« {nom} ») : {e:?}"));
        for (tiers, r) in [(tom, Role::Parent), (prof, Role::Prof)] {
            let existant = route(&ent, nom, etab, tiers, r, fil).await.expect_err("fil d'autrui");
            let inexistant = route(&ent, nom, etab, tiers, r, absent).await.expect_err("fil absent");
            assert!(
                !matches!(existant, EntDbError::Forbidden),
                "« {nom} » ({r:?}) : un 403 révèle l'existence du fil : {existant:?}"
            );
            // Même erreur exacte, donc même statut et même corps côté HTTP.
            assert_eq!(
                format!("{existant:?}"),
                format!("{inexistant:?}"),
                "« {nom} » ({r:?}) : un tiers distingue un fil existant d'un fil absent"
            );
        }
    }
}

// ───────────────────────── M3 : avis relu par destinataire ─────────────────────────

#[tokio::test]
async fn corr_m3_avis_reponse_attendue_relit_le_droit_de_chaque_destinataire() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, inf) = (cast.direction.user_id, cast.cpe.user_id, cast.infirmieres[0].user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe]).await;
    let (sante, _) = boite(&ent, etab, dir, "infirmerie", &[inf]).await;
    let a = ouvrir_boite(&ent, etab, lily, Role::Parent, vs, harry, "A").await;
    let b = ouvrir_boite(&ent, etab, lily, Role::Parent, vs, harry, "B").await;
    let c3 = ouvrir_boite(&ent, etab, lily, Role::Parent, sante, harry, "Santé").await;
    let nb = |sql: &'static str| {
        let ent = &ent;
        async move { compter(ent, sql, &[lily]).await }
    };

    // Témoin : sans interdiction, la famille reçoit l'avis neutre.
    ent.avertir_reponse_attendue(etab, a.fil_id).await;
    let apres_a = nb("SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1 AND kind = 'messagerie_famille'").await;
    assert_eq!(apres_a, 1, "témoin : l'avis est émis à un destinataire qui a le droit");
    // Fil C3 : l'avis part, sans élève.
    ent.avertir_reponse_attendue(etab, c3.fil_id).await;
    let sans_eleve = nb("SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1 AND kind = 'messagerie_famille' AND eleve_id IS NULL").await;
    assert_eq!(sans_eleve, 1, "avis d'un fil C3 : jamais l'élève (celui du fil ordinaire le porte)");

    // Attaque : une interdiction de communication vise Lily ; plus rien ne lui est envoyé (droit relu avant l'envoi).
    poser_interdiction(&ent, etab, harry, lily, dir).await;
    let avant = nb("SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1 AND kind = 'messagerie_famille'").await;
    ent.avertir_reponse_attendue(etab, b.fil_id).await;
    let apres = nb("SELECT count(*) FROM ent_notification WHERE destinataire_user_id = $1 AND kind = 'messagerie_famille'").await;
    assert_eq!(apres, avant, "un parent visé par une interdiction ne reçoit rien");
}

// ───────────────────────── M4 : annulation d'un message différé ─────────────────────────

#[tokio::test]
async fn corr_m4_annulation_d_un_message_differe_avec_carte_piece_et_lecture() {
    use crate::tests_documents_d4::W;
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = w.m.etab;
    let (dir, lily) = (w.cast.direction.user_id, w.cast.parent_lea.user_id);
    let eleve = w.eleve("Henry", "Maël").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.certificat(eleve).await;
    let fils = ouvrir(&w.ent, etab, Role::Direction, dir, eleve, "Programmé").await;
    let (fil, premier) = (fils[0].fil.id, fils[0].message.id);
    assert!(ecrit(&w.ent, etab, dir, Role::Direction, fil, "Message à annuler").await);
    let m2 = dernier_message(&w.ent, etab, fil, dir).await;
    let piece = deposer(&w.ent, etab, m2.id, dir, Role::Direction, "ordonnance.pdf", "application/pdf", PDF_REEL)
        .await
        .unwrap();
    lever_quarantaine(&w.ent, etab).await;
    let carte = w
        .ent
        .joindre_carte_document_for_actor(etab, dir, role(Role::Direction), m2.id, doc.document_id)
        .await
        .expect("carte jointe");
    // Une carte sur un message **remis** : elle ne doit jamais pouvoir disparaître.
    let carte_remise = w
        .ent
        .joindre_carte_document_for_actor(etab, dir, role(Role::Direction), premier, doc.document_id)
        .await
        .expect("carte du premier message");
    // La famille télécharge la pièce (journal d'accès) et lit (pointeur « lu jusqu'à »).
    w.ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), piece.id).await.unwrap();
    assert!(w.ent.marquer_lu_for_actor(etab, lily, role(Role::Parent), fil, None).await.unwrap());
    assert_eq!(pointeurs(&w.ent, fil).await.len(), 1, "témoin : une lecture est posée");
    // État hérité à traiter : un message dont la remise est encore à venir, avec accès et lecture qui le visent.
    sqlx::query("UPDATE ent_msg SET remise_prevue_le = now() + interval '2 hours' WHERE id = $1")
        .bind(m2.id)
        .execute(w.ent.pool_verif())
        .await
        .unwrap();
    let (cle_orig, cle_servie) = cles_piece(&w.ent, piece.id).await;
    let cle_servie = cle_servie.expect("pièce saine");
    assert!(w.ent.objet_existe(&cle_orig).await.unwrap() && w.ent.objet_existe(&cle_servie).await.unwrap());

    // Attaques : un autre compte ne l'annule pas ; l'exception du déclencheur ne couvre pas un message remis ;
    // aucune suppression hors annulation.
    assert!(w.ent.annuler_message_differe_for_actor(etab, lily, role(Role::Parent), m2.id).await.is_err());
    let mut tx = w.ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('ent.msg_annulation', $1, true)")
        .bind(premier.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let r = sqlx::query("DELETE FROM ent_msg_carte_document WHERE id = $1")
        .bind(carte_remise.id)
        .execute(&mut *tx)
        .await;
    assert!(r.is_err(), "l'exception est bornée aux messages jamais remis");
    drop(tx);
    let r = sqlx::query("DELETE FROM ent_msg_carte_document WHERE id = $1")
        .bind(carte.id)
        .execute(w.ent.pool_verif())
        .await;
    assert!(r.is_err(), "ajout seul hors annulation");
    assert!(
        sqlx::query("DELETE FROM ent_msg_piece_acces WHERE piece_id = $1")
            .bind(piece.id)
            .execute(w.ent.pool_verif())
            .await
            .is_err(),
        "journal d'accès en ajout seul hors annulation"
    );

    // L'auteur annule : tout disparaît dans la même transaction, objets compris.
    w.ent
        .annuler_message_differe_for_actor(etab, dir, role(Role::Direction), m2.id)
        .await
        .expect("annulation (R-M48)");
    for (sql, attendu) in [
        ("SELECT count(*) FROM ent_msg WHERE id = $1", 0),
        ("SELECT count(*) FROM ent_msg_piece WHERE message_id = $1", 0),
        ("SELECT count(*) FROM ent_msg_carte_document WHERE message_id = $1", 0),
        ("SELECT count(*) FROM ent_msg_lecture WHERE lu_jusqua_message_id = $1", 0),
    ] {
        assert_eq!(compter(&w.ent, sql, &[m2.id]).await, attendu, "{sql}");
    }
    assert_eq!(compter(&w.ent, "SELECT count(*) FROM ent_msg_piece_acces WHERE piece_id = $1", &[piece.id]).await, 0);
    assert!(!w.ent.objet_existe(&cle_orig).await.unwrap(), "original détruit");
    assert!(!w.ent.objet_existe(&cle_servie).await.unwrap(), "fichier servi détruit");
    assert!(objet_supprime_au_registre(&w.ent, etab, &cle_orig).await);
    assert!(objet_supprime_au_registre(&w.ent, etab, &cle_servie).await);
    // La carte du message remis est intacte, le fil reste lisible.
    assert_eq!(compter(&w.ent, "SELECT count(*) FROM ent_msg_carte_document WHERE message_id = $1", &[premier]).await, 1);
    assert!(lit(&w.ent, etab, lily, Role::Parent, fil).await);
}

#[tokio::test]
async fn corr_m4_pointeur_exclut_ses_propres_messages() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, lily) = (cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, premier) = fil_de_lily(&ent, &cast, "Pointeur").await;
    // Lily répond (son propre message), puis lit : « un auteur n'accuse pas ses messages ».
    assert!(ecrit(&ent, etab, lily, Role::Parent, fil, "Merci, bien reçu.").await);
    assert!(ent.marquer_lu_for_actor(etab, lily, role(Role::Parent), fil, None).await.unwrap());
    let p = pointeurs(&ent, fil).await;
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].0, premier, "le pointeur vise le message de la direction, jamais celui de Lily");
    // Marquer lu jusqu'à son propre message : rien n'avance.
    let siens = dernier_message(&ent, etab, fil, lily).await;
    assert_eq!(siens.auteur_user_id, lily);
    assert!(!ent.marquer_lu_for_actor(etab, lily, role(Role::Parent), fil, Some(siens.id)).await.unwrap());
    assert_eq!(pointeurs(&ent, fil).await[0].0, premier);
    // Témoin : la direction qui lit la réponse de Lily pose, elle, un pointeur sur ce message.
    assert!(ent.marquer_lu_for_actor(etab, dir, role(Role::Direction), fil, None).await.unwrap());
    assert!(pointeurs(&ent, fil).await.iter().any(|(m, _)| *m == siens.id));
}

// ───────────────────────── M5 : purge des pièces refusées ─────────────────────────

#[tokio::test]
async fn corr_m5_purge_des_pieces_infectees_et_rejetees() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    assert_eq!((crate::RETENTION_MSG_INFECTEES_JOURS, crate::RETENTION_MSG_REJETEES_JOURS), (7, 30));
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (_fil, msg) = fil_de_lily(&ent, &cast, "Purge").await;
    let mut infecte = PDF_REEL.to_vec();
    infecte.extend_from_slice(b"\n%");
    infecte.extend_from_slice(EICAR);
    infecte.push(b'\n');
    let saine = deposer(&ent, etab, msg, dir, Role::Direction, "ok.pdf", "application/pdf", PDF_REEL).await.unwrap();
    let mal = deposer(&ent, etab, msg, dir, Role::Direction, "mal.pdf", "application/pdf", &infecte).await.unwrap();
    let rejetee = deposer(&ent, etab, msg, dir, Role::Direction, "chiffre.pdf", "application/pdf", PDF_CHIFFRE).await.unwrap();
    let bilan = lever_quarantaine(&ent, etab).await;
    assert_eq!((bilan.saines, bilan.infectees, bilan.rejetees), (1, 1, 1), "{bilan:?}");
    let (cle_mal, _) = cles_piece(&ent, mal.id).await;
    let (cle_rej, _) = cles_piece(&ent, rejetee.id).await;
    let (cle_saine, _) = cles_piece(&ent, saine.id).await;

    // Trop récent : rien n'est purgé (témoin que la purge ne détruit pas à l'aveugle).
    let b = ent.purger_pieces_messagerie_refusees(etab, 7, 30).await.unwrap();
    assert_eq!((b.purges, b.echecs), (0, 0));
    assert!(ent.objet_existe(&cle_mal).await.unwrap() && ent.objet_existe(&cle_rej).await.unwrap());

    // 8 jours : l'infectée part, la rejetée reste (30 jours), la saine n'est jamais touchée.
    for p in [mal.id, rejetee.id, saine.id] {
        vieillir_analyse(&ent, p, 8).await;
    }
    let b = ent.purger_pieces_messagerie_refusees(etab, 7, 30).await.unwrap();
    assert_eq!((b.purges, b.echecs), (1, 0), "{b:?}");
    assert!(!ent.objet_existe(&cle_mal).await.unwrap(), "original infecté détruit");
    assert!(objet_supprime_au_registre(&ent, etab, &cle_mal).await, "marqué au registre");
    assert!(ent.objet_existe(&cle_rej).await.unwrap(), "rejetée : 30 jours");
    // Idempotente.
    assert_eq!(ent.purger_pieces_messagerie_refusees(etab, 7, 30).await.unwrap().purges, 0);

    // 31 jours : la rejetée part aussi. La durée est un paramètre.
    for p in [rejetee.id, saine.id] {
        vieillir_analyse(&ent, p, 31).await;
    }
    let b = ent.purger_pieces_messagerie_refusees_tache(7, 30).await.unwrap();
    assert!(b.purges >= 1 && b.echecs == 0, "{b:?}");
    assert!(!ent.objet_existe(&cle_rej).await.unwrap() && objet_supprime_au_registre(&ent, etab, &cle_rej).await);
    assert!(ent.objet_existe(&cle_saine).await.unwrap(), "une pièce saine n'est jamais purgée");
    assert_eq!(etat_piece(&ent, saine.id).await, "saine");
}

// ───────────────────────── M8 : liens de messagerie ─────────────────────────

#[tokio::test]
async fn corr_m8_lien_borne_via_l_application_ouverture_journalisee() {
    use ent_object::presign::{Presigner, S3Config};
    let Some((mut ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let secret = [9u8; 32];
    // Une durée configurée à 15 minutes : la messagerie reste à 60 s.
    ent.installer_presigner(Presigner::local(&secret, 900).unwrap());
    let verif = Presigner::local(&secret, 900).unwrap();
    assert_eq!(verif.ttl_secs(), 900);
    let (etab, dir, lily, tom) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id, cast.parent_tom.user_id);
    let (fil, msg) = fil_de_lily(&ent, &cast, "Liens").await;
    let meta = deposer(&ent, etab, msg, dir, Role::Direction, "certificat.pdf", "application/pdf", PDF_REEL).await.unwrap();
    lever_quarantaine(&ent, etab).await;
    let avant = ent_object::presign::now_unix();
    let lien = ent.lien_piece_message_for_actor(etab, lily, role(Role::Parent), meta.id).await.expect("témoin : lien");
    assert!(lien.expires_at - avant <= 61, "60 s au plus malgré 900 s configurées : {} s", lien.expires_at - avant);
    let token = lien.url.trim_start_matches("/fichiers/");
    let claims = verif.verify_local(token, lien.expires_at - 1).expect("jeton valide");
    assert_eq!(claims.user_id, lily);

    // L'ouverture est journalisée à côté de l'émission, droits relus.
    assert_eq!(nb_acces(&ent, etab, meta.id, "lien").await, 1);
    assert!(ent.controler_ouverture_piece_message(etab, lily, role(Role::Parent), &claims.key).await.unwrap());
    assert_eq!(nb_acces(&ent, etab, meta.id, "ouverture").await, 1);
    // Une clé qui n'est pas celle d'un fichier servi de messagerie n'est pas concernée.
    assert!(!ent.controler_ouverture_piece_message(etab, lily, role(Role::Parent), &format!("{etab}/autre/x.bin")).await.unwrap());
    // Un tiers, même muni de la clé : 404, rien n'est journalisé.
    let e = ent.controler_ouverture_piece_message(etab, tom, role(Role::Parent), &claims.key).await.expect_err("tiers");
    assert!(est_introuvable(&e), "{e:?}");
    // Un participant retiré entre l'émission et l'ouverture : 404 (droits relus à l'ouverture).
    sqlx::query(
        "UPDATE ent_msg_participant SET au = now(), motif_sortie = 'sortie_volontaire'
          WHERE etablissement_id = $1 AND fil_id = $2 AND cote = 'famille' AND au IS NULL",
    )
    .bind(etab)
    .bind(fil)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    let e = ent
        .controler_ouverture_piece_message(etab, lily, role(Role::Parent), &claims.key)
        .await
        .expect_err("participant retiré");
    assert!(est_introuvable(&e), "{e:?}");
    assert_eq!(nb_acces(&ent, etab, meta.id, "ouverture").await, 1, "aucune ouverture refusée n'est journalisée");

    // Mode S3 : jamais d'URL SigV4 directe pour la messagerie. Avec un signataire d'application, le lien passe par
    // le point de téléchargement de l'application ; sans, aucun lien n'est émis (fermé par défaut).
    let (fil2, msg2) = fil_de_lily(&ent, &cast, "Liens S3").await;
    let _ = fil2;
    let meta2 = deposer(&ent, etab, msg2, dir, Role::Direction, "certificat.pdf", "application/pdf", PDF_REEL).await.unwrap();
    lever_quarantaine(&ent, etab).await;
    let cfg = || S3Config::new("b", "fr-par", "AK", "SECRET-S3", None, false).unwrap();
    ent.installer_presigner(Presigner::s3(cfg(), 600).avec_signataire_application(&secret));
    let l = ent.lien_piece_message_for_actor(etab, dir, role(Role::Direction), meta2.id).await.expect("lien S3 via l'application");
    assert!(l.url.starts_with("/fichiers/") && !l.url.contains("X-Amz"), "{}", l.url);
    assert!(l.expires_at - ent_object::presign::now_unix() <= 61);
    ent.installer_presigner(Presigner::s3(cfg(), 600));
    let e = ent.lien_piece_message_for_actor(etab, dir, role(Role::Direction), meta2.id).await.expect_err("sans signataire");
    assert!(matches!(e, EntDbError::LiensIndisponibles), "{e:?}");
}

// ───────────────────────── F1 : limite sous verrou, message retiré ou différé ─────────────────────────

#[tokio::test]
async fn corr_f1_cinq_pieces_sous_verrou_et_message_retire_ou_differe() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, msg) = fil_de_lily(&ent, &cast, "Cinq").await;
    // Huit dépôts simultanés : exactement cinq réussissent, jamais six.
    let e = &ent;
    let d = |i: u32| async move {
        (i, deposer(e, etab, msg, dir, Role::Direction, "p.pdf", "application/pdf", PDF_REEL).await)
    };
    let (r0, r1, r2, r3, r4, r5, r6, r7) = tokio::join!(d(0), d(1), d(2), d(3), d(4), d(5), d(6), d(7));
    let resultats: Vec<_> = [r0, r1, r2, r3, r4, r5, r6, r7].into_iter().map(|(_, r)| r).collect();
    let reussis = resultats.iter().filter(|r| r.is_ok()).count();
    assert_eq!(reussis, 5, "{resultats:?}");
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_msg_piece WHERE message_id = $1", &[msg]).await, 5);
    assert!(resultats.iter().filter_map(|r| r.as_ref().err()).all(|e| matches!(e, EntDbError::SaisieInvalide(_))));

    // Message retiré : refus, et l'objet déposé est détruit (pas d'orphelin).
    let (_f2, m2) = {
        assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Autre message").await);
        let m = dernier_message(&ent, etab, fil, dir).await;
        (fil, m.id)
    };
    deposer(&ent, etab, m2, dir, Role::Direction, "ok.pdf", "application/pdf", PDF_REEL)
        .await
        .expect("témoin : dépôt sur un message ordinaire");
    ent.retirer_message_for_actor(etab, dir, role(Role::Direction), m2, None).await.expect("message retiré");
    let e = deposer(&ent, etab, m2, dir, Role::Direction, "x.pdf", "application/pdf", PDF_REEL).await.expect_err("retiré");
    assert!(est_conflit(&e), "{e:?}");
    // Message différé : un autre participant est refusé ; l'auteur peut joindre une pièce à son propre message (la
    // pièce naît en quarantaine, suit le message, et n'est visible de personne d'autre avant la remise).
    assert!(ecrit(&ent, etab, dir, Role::Direction, fil, "Troisième").await);
    let m3 = dernier_message(&ent, etab, fil, dir).await;
    sqlx::query("UPDATE ent_msg SET remise_prevue_le = now() + interval '1 hour' WHERE id = $1")
        .bind(m3.id)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let lily = cast.parent_lea.user_id;
    let e = deposer(&ent, etab, m3.id, lily, Role::Parent, "x.pdf", "application/pdf", PDF_REEL).await.expect_err("autre auteur");
    // Un autre que l'auteur est refusé dès le domaine (jamais atteint le dépôt) ; le repli du dépôt (« différé » ->
    // conflit) reste une seconde barrière.
    assert!(
        matches!(e, EntDbError::Domain(DomainError::MsgPiecePasAuteur)) || est_conflit(&e),
        "{e:?}"
    );
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_msg_piece WHERE message_id = $1", &[m3.id]).await, 0);
    let propre = deposer(&ent, etab, m3.id, dir, Role::Direction, "mien.pdf", "application/pdf", PDF_REEL)
        .await
        .expect("l'auteur joint une pièce à son message différé");
    assert_eq!(compter(&ent, "SELECT count(*) FROM ent_msg_piece WHERE message_id = $1", &[m3.id]).await, 1);
    assert_eq!(propre.message_id, m3.id);
}

// ───────────────────────── F2 : « lu par » à la première lecture ─────────────────────────

#[tokio::test]
async fn corr_f2_lu_par_n_est_pose_qu_a_la_premiere_lecture() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = cast.etablissement_id;
    let (dir, cpe, adm) = (cast.direction.user_id, cast.cpe.user_id, cast.administratif.user_id);
    let (lily, harry) = (cast.parent_lea.user_id, cast.eleve_lea_id);
    let (vs, _) = boite(&ent, etab, dir, "vie_scolaire", &[cpe, adm]).await;
    let o = ouvrir_boite(&ent, etab, lily, Role::Parent, vs, harry, "Lu par").await;
    assert!(lit(&ent, etab, cpe, Role::Cpe, o.fil_id).await);
    let premier = pointeurs(&ent, o.fil_id).await;
    let cpe_personne = personne_de(&ent, etab, cpe).await;
    assert_eq!(premier[0].1, Some(cpe_personne), "témoin : le premier membre est gardé");
    // La famille écrit de nouveau ; un autre membre lit : le pointeur avance, « lu par » ne change pas.
    assert!(ecrit(&ent, etab, lily, Role::Parent, o.fil_id, "Relance").await);
    assert!(lit(&ent, etab, adm, Role::Administratif, o.fil_id).await);
    let apres = pointeurs(&ent, o.fil_id).await;
    assert_eq!(apres.len(), 1);
    assert_ne!(apres[0].0, premier[0].0, "le pointeur a avancé");
    assert_eq!(apres[0].1, Some(cpe_personne), "« lu par » reste celui de la première lecture");
    let _ = cree_le(&ent, apres[0].0).await + Duration::seconds(1);
}

// ───────────────────────── F3 : photo ─────────────────────────

#[tokio::test]
async fn corr_f3_photo_sans_gps_orientation_conservee() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (_fil, msg) = fil_de_lily(&ent, &cast, "Photo").await;
    let photo = deposer(&ent, etab, msg, dir, Role::Direction, "IMG.jpg", "image/jpeg", PHOTO_GPS).await.unwrap();
    // La photo de départ est orientée (6) et géolocalisée.
    assert!(PHOTO_GPS.windows(4).any(|w| w == b"Exif"));
    lever_quarantaine(&ent, etab).await;
    let p = ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), photo.id).await.unwrap();
    let c = &p.contenu;
    assert!(c.starts_with(&[0xff, 0xd8]));
    // Aucune position, aucun fabricant : un seul segment EXIF minimal, qui ne porte que l'orientation (balise 0x0112).
    assert!(!c.windows(3).any(|w| w == b"GPS") && !c.windows(8).any(|w| w == b"Fabrican"));
    let exif = c.windows(6).position(|w| w == b"Exif\0\0").expect("segment d'orientation conservé");
    let tiff = &c[exif + 6..];
    assert_eq!(&tiff[..2], b"MM");
    assert_eq!(u16::from_be_bytes([tiff[8], tiff[9]]), 1, "une seule balise");
    assert_eq!(u16::from_be_bytes([tiff[10], tiff[11]]), 0x0112);
    assert_eq!(tiff[19], 6, "orientation d'origine : l'image s'affiche à l'endroit");
}

// ───────────────────────── F4 : élève protégé (E7-E8) ─────────────────────────

#[tokio::test]
async fn corr_f4_eleve_protege_pieces_cartes_et_avis() {
    use crate::tests_documents_d4::W;
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = w.m.etab;
    let (dir, lily, adm) = (w.cast.direction.user_id, w.cast.parent_lea.user_id, w.cast.administratif.user_id);
    let eleve = w.eleve("Henry", "Maël").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.certificat(eleve).await;
    let fils = ouvrir(&w.ent, etab, Role::Direction, dir, eleve, "Protégé").await;
    let (fil, msg) = (fils[0].fil.id, fils[0].message.id);
    let piece = deposer(&w.ent, etab, msg, dir, Role::Direction, "certificat.pdf", "application/pdf", PDF_REEL).await.unwrap();
    lever_quarantaine(&w.ent, etab).await;
    let carte = w
        .ent
        .joindre_carte_document_for_actor(etab, dir, role(Role::Direction), msg, doc.document_id)
        .await
        .unwrap();
    // Témoins avant protection : un administratif (pont des fils de foyer) lit le fil, la pièce, les cartes ; la
    // famille aussi.
    assert!(lit(&w.ent, etab, adm, Role::Administratif, fil).await, "témoin : l'administratif lit le fil");
    assert!(w.ent.telecharger_piece_for_actor(etab, adm, role(Role::Administratif), piece.id).await.is_ok());
    assert!(w.ent.cartes_du_fil_for_actor(etab, adm, role(Role::Administratif), fil).await.is_ok());
    assert!(w.ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), piece.id).await.is_ok());

    // L'élève devient protégé (mesure de confidentialité renforcée).
    w.ent
        .poser_confidentialite_a(
            role(Role::Direction),
            crate::eleve_confidentialite::PoserConfidentialite {
                etablissement_id: etab,
                eleve_id: eleve,
                motif: "Mesure de protection".into(),
                mesures: vec![],
                du: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
                au: None,
                acteur: dir,
            },
            chrono::Utc::now(),
        )
        .await
        .expect("mesure posée");

    // E7 : pour un tiers (administratif sans G6 ni lien), le fil, ses pièces, ses cartes et son état de lecture sont
    // ceux d'un élève absent : 404 uniforme, aucun indice.
    let inconnu = Uuid::new_v4();
    let e = w.ent.telecharger_piece_for_actor(etab, adm, role(Role::Administratif), piece.id).await.expect_err("pièce");
    let e0 = w.ent.telecharger_piece_for_actor(etab, adm, role(Role::Administratif), inconnu).await.expect_err("absente");
    assert!(est_introuvable(&e), "{e:?}");
    assert_eq!(format!("{e:?}"), format!("{e0:?}"));
    assert!(est_introuvable(&w.ent.lien_piece_message_for_actor(etab, adm, role(Role::Administratif), piece.id).await.unwrap_err())
        || matches!(w.ent.lien_piece_message_for_actor(etab, adm, role(Role::Administratif), piece.id).await.unwrap_err(), EntDbError::LiensIndisponibles));
    assert!(est_introuvable(&w.ent.cartes_du_fil_for_actor(etab, adm, role(Role::Administratif), fil).await.unwrap_err()));
    assert!(est_introuvable(&w.ent.ouvrir_carte_document_for_actor(etab, adm, role(Role::Administratif), carte.id).await.unwrap_err()));
    assert!(est_introuvable(&w.ent.lecture_du_fil_for_actor(etab, adm, role(Role::Administratif), fil).await.unwrap_err()));
    assert!(est_introuvable(&w.ent.list_messages_for_actor(etab, adm, role(Role::Administratif), fil).await.unwrap_err()));
    // La protection vise les tiers, pas les proches : la famille lit toujours, et la direction (G6) aussi.
    assert!(w.ent.telecharger_piece_for_actor(etab, lily, role(Role::Parent), piece.id).await.is_ok());
    assert!(w.ent.cartes_du_fil_for_actor(etab, dir, role(Role::Direction), fil).await.is_ok());
    // E8 : l'avis « réponse attendue » part toujours à la famille, avec le texte neutre, sans le nom de l'élève.
    w.ent.avertir_reponse_attendue(etab, fil).await;
    let textes: Vec<String> = sqlx::query_scalar(
        "SELECT titre || ' ' || corps FROM ent_notification WHERE destinataire_user_id = $1 AND kind = 'messagerie_famille'",
    )
    .bind(lily)
    .fetch_all(w.ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(textes.len(), 1, "témoin : l'avis est émis à la famille");
    assert!(!textes[0].contains("Henry") && !textes[0].contains("Maël"), "{textes:?}");
}

// ───────────────────────── F5 : carte servie selon le document réel ─────────────────────────

#[tokio::test]
async fn corr_f5_carte_servie_selon_le_document_reel() {
    use crate::tests_documents_d4::W;
    let Some(w) = W::nouveau().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let etab = w.m.etab;
    let (dir, lily) = (w.cast.direction.user_id, w.cast.parent_lea.user_id);
    let eleve = w.eleve("Henry", "Maël").await;
    w.lier(eleve, &w.lily, true).await;
    let doc = w.certificat(eleve).await;
    let fils = ouvrir(&w.ent, etab, Role::Direction, dir, eleve, "Certificat").await;
    let carte = w
        .ent
        .joindre_carte_document_for_actor(etab, dir, role(Role::Direction), fils[0].message.id, doc.document_id)
        .await
        .unwrap();
    let ouverte = w.ent.ouvrir_carte_document_for_actor(etab, lily, role(Role::Parent), carte.id).await.unwrap();
    // Le type vient de la signature des octets, le nom du titre du document ; jamais « document.pdf » en dur.
    assert!(ouverte.octets.starts_with(b"%PDF-"));
    assert_eq!(ouverte.mime, "application/pdf");
    assert!(ouverte.nom.ends_with(".pdf") && ouverte.nom != "document.pdf", "{}", ouverte.nom);
    assert!(!ouverte.nom.contains('/') && !ouverte.nom.contains(".."), "{}", ouverte.nom);
}

// ───────────────────────── F9 : la passe continue ─────────────────────────

#[tokio::test]
async fn corr_f9_la_passe_continue_malgre_un_etablissement_en_echec() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (_fil, msg) = fil_de_lily(&ent, &cast, "Passe").await;
    let meta = deposer(&ent, etab, msg, dir, Role::Direction, "ok.pdf", "application/pdf", PDF_REEL).await.unwrap();
    // Un second établissement dont l'analyse échoue (l'original a disparu du magasin) : la passe n'est pas interrompue.
    let autre = ent.seed_labo_cast().await.unwrap();
    let (_f2, m2) = fil_de_lily(&ent, &autre, "Passe B").await;
    let cassee = deposer(&ent, autre.etablissement_id, m2, autre.direction.user_id, Role::Direction, "b.pdf", "application/pdf", PDF_REEL)
        .await
        .unwrap();
    let (cle, _) = cles_piece(&ent, cassee.id).await;
    ent.objet_supprimer(&cle).await.unwrap();
    let bilan = ent.analyser_pieces_messagerie_tache(super::tests_messagerie_m5::AvTest::sain(), 50).await.expect("la passe ne s'interrompt pas");
    assert_eq!(etat_piece(&ent, meta.id).await, "saine", "témoin : l'établissement sain est traité ({bilan:?})");
    assert_eq!(etat_piece(&ent, cassee.id).await, "en_quarantaine");
    assert!(bilan.en_echec >= 1, "{bilan:?}");
}
