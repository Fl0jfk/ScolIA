//! Messagerie, suites E1 (passe 5) : lot non affamé par des avis verrouillés (faible 1), « autre établissement
//! invisible » prouvé par la RLS sous le rôle applicatif (faible 2), `expedier` observe la borne `xmin` hors flux
//! (faible 3). Le limiteur de journal (faible 4) a son test unitaire dans `journal_limite`.
//!
//! Aucune heure réelle : instants de la base, injectés dans `expedier`.

use super::tests_messagerie_m1::labo;
use super::tests_messagerie_m5::fil_de_lily;
use super::tests_messagerie_p2::{apres_l_echeance, avis_planifie, statut_avis};
use super::tests_messagerie_p3::{code, pool_applicatif};
use super::tests_messagerie_revue::message_differe;
use super::*;
use crate::tenant_tx::RlsMode;
use chrono::{DateTime, Utc};
use std::time::Duration;
use uuid::Uuid;

/// Faible 1 : `taille_lot` avis visent des messages verrouillés ; l'avis libre, derrière eux, est quand même traité au
/// passage **suivant** (même instant injecté) : les reportés ont leur `planifie_pour` repoussé, ils ne reprennent plus la
/// tête de file.
#[tokio::test]
async fn p5_f1_un_lot_plein_d_avis_verrouilles_ne_affame_pas_les_avis_libres() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
    );
    let (fil, _) = fil_de_lily(&ent, &cast, "Lot affamé").await;
    let m1 = message_differe(&ent, etab, dir, fil, "Verrouillé 1").await;
    let m2 = message_differe(&ent, etab, dir, fil, "Verrouillé 2").await;
    let libre = message_differe(&ent, etab, dir, fil, "Libre").await;
    let a1 = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, m1).await;
    let a2 = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, m2).await;
    let a_libre = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, libre).await;
    // L'avis libre est le plus récent : sans report, les deux verrouillés (taille_lot = 2) le précèdent toujours.
    sqlx::query("UPDATE ent_notification_envoi SET planifie_pour = planifie_pour + interval '1 minute' WHERE id = $1")
        .bind(a_libre)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let apres = apres_l_echeance(&ent).await;

    let mut lente = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *lente)
        .await
        .unwrap();
    for m in [m1, m2] {
        sqlx::query("SELECT 1 FROM ent_msg WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
            .bind(etab)
            .bind(m)
            .fetch_one(&mut *lente)
            .await
            .unwrap();
    }
    let exp = ent.expediteur(crate::notifications::ConfigExpedition {
        taille_lot: 2,
        etablissement: Some(etab),
        attente_verrou: Duration::from_millis(200),
        ..crate::notifications::ConfigExpedition::default()
    });
    let b1 = exp.expedier(apres).await.unwrap();
    assert_eq!(
        (b1.reclames, b1.reportes),
        (2, 2),
        "premier lot : les deux verrouillés, reportés : {b1:?}"
    );
    // Deuxième passage, même instant : les verrouillés sont repoussés, le lot atteint l'avis libre.
    let b2 = exp.expedier(apres).await.unwrap();
    assert_eq!(b2.reclames, 1, "le lot ne tourne plus à vide : {b2:?}");
    let (statut, motif) = statut_avis(&ent, a_libre).await;
    assert!(
        !matches!(statut.as_str(), "en_attente" | "envoi"),
        "l'avis libre est traité : {statut}"
    );
    assert_ne!(motif.as_deref(), Some("message_annule"));
    for a in [a1, a2] {
        assert_eq!(
            statut_avis(&ent, a).await.0,
            "en_attente",
            "reporté, pas perdu"
        );
    }
    // Annulation terminée : plus tard, les reportés repartent.
    lente.rollback().await.unwrap();
    let plus_tard = apres + chrono::Duration::minutes(1);
    let b3 = exp.expedier(plus_tard).await.unwrap();
    assert_eq!(
        b3.reclames, 2,
        "les reportés repartent une fois l'attente passée : {b3:?}"
    );
    assert_eq!(b3.reportes, 0);
}

/// Faible 2 : l'établissement d'un autre contexte ne voit rien **par la RLS** (sans `WHERE etablissement_id`), sous le
/// rôle applicatif non propriétaire, non superutilisateur, sans contournement.
#[tokio::test]
async fn p5_f2_un_autre_etablissement_est_invisible_par_la_rls_sous_le_role_applicatif() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "RLS autre établissement").await;
    let m = message_differe(&ent, etab, dir, fil, "Visible de son établissement").await;
    let app = pool_applicatif(&ent).await;
    let (bypass,): (bool,) = sqlx::query_as(
        "SELECT rolbypassrls OR rolsuper FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_one(&app)
    .await
    .unwrap();
    assert!(
        !bypass,
        "précondition : le rôle applicatif ne contourne pas la RLS"
    );

    async fn compter(app: &sqlx::PgPool, contexte: Uuid, m: Uuid) -> i64 {
        let mut tx = TenantTx::begin_mode(app, contexte, RlsMode::Strict)
            .await
            .unwrap();
        // Volontairement **sans** `etablissement_id` dans le filtre : seule la RLS peut cacher la ligne.
        let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_msg WHERE id = $1")
            .bind(m)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        n
    }
    assert_eq!(
        compter(&app, etab, m).await,
        1,
        "témoin : visible dans son établissement"
    );
    assert_eq!(
        compter(&app, Uuid::new_v4(), m).await,
        0,
        "invisible dans le contexte d'un autre établissement"
    );

    // Et le verrou de ligne : même avec le bon `WHERE`, le contexte d'un autre établissement ne voit rien.
    let mut tx = TenantTx::begin_mode(&app, Uuid::new_v4(), RlsMode::Strict)
        .await
        .unwrap();
    let r: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM ent_msg WHERE id = $1 FOR KEY SHARE")
        .bind(m)
        .fetch_optional(&mut *tx)
        .await
        .unwrap();
    assert!(
        r.is_none(),
        "FOR KEY SHARE ne trouve rien hors contexte (RLS)"
    );
    drop(tx);
    // Le dépôt expose le code SQLSTATE (version brute) : un verrou tenu rend 55P03, jamais « inexistant ».
    let repo = crate::msg_repo::PgMsgRepo::new(app.clone());
    let mut lente = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *lente)
        .await
        .unwrap();
    sqlx::query("SELECT 1 FROM ent_msg WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
        .bind(etab)
        .bind(m)
        .fetch_one(&mut *lente)
        .await
        .unwrap();
    let r = repo
        .message_existe_sous_verrou_brut(etab, m, Duration::from_millis(200))
        .await;
    assert_eq!(
        r.as_ref().err().and_then(code).as_deref(),
        Some("55P03"),
        "verrou tenu : {r:?}"
    );
    lente.rollback().await.unwrap();
}

/// Faible 3 : un passage de `expedier` observe la borne `xmin` hors flux (le compteur de `msg_xmin_gele` avance).
#[tokio::test]
async fn p5_f3_expedier_observe_la_borne_xmin_hors_flux() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let apres = apres_l_echeance(&ent).await;
    let exp = ent.expediteur(crate::notifications::ConfigExpedition {
        etablissement: Some(cast.etablissement_id),
        ..crate::notifications::ConfigExpedition::default()
    });
    // Contre-revue du pilote (M1) : compteur propre à l'établissement neuf du test (aucun voisin ne l'incrémente).
    assert_eq!(crate::msg_xmin_gele::observations_hors_flux(cast.etablissement_id), 0);
    exp.expedier(apres).await.unwrap();
    assert!(
        crate::msg_xmin_gele::observations_hors_flux(cast.etablissement_id) >= 1,
        "un passage de l'expéditeur observe la borne xmin hors flux pour son établissement"
    );
}

/// Verrouille le message `FOR UPDATE` dans une transaction ouverte (une annulation en cours) : tant qu'elle vit, tout
/// contrôle `FOR KEY SHARE` attend.
async fn annulation_en_cours(
    ent: &PgEnt,
    etab: Uuid,
    message: Uuid,
) -> sqlx::Transaction<'static, sqlx::Postgres> {
    let mut lente = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *lente)
        .await
        .unwrap();
    sqlx::query("SELECT 1 FROM ent_msg WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
        .bind(etab)
        .bind(message)
        .fetch_one(&mut *lente)
        .await
        .unwrap();
    lente
}

/// T3 (faible 4 de E1 passe 5) : `message_existe_sous_verrou` (version de domaine, appelée par l'API) et
/// `message_existe_sous_verrou_brut` (version des tests, SQLSTATE lisible) rendent le **même verdict** dans les trois
/// états : message existant, message inconnu, message verrouillé par une annulation en cours (la version de domaine rend
/// alors exactement la réduction de l'erreur brute par `pg_err::map_sqlx`, jamais un SQLSTATE).
#[tokio::test]
async fn p5_e1_l_enveloppe_et_la_version_brute_rendent_le_meme_verdict() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Équivalence enveloppe").await;
    let m = message_differe(&ent, etab, dir, fil, "Existant").await;
    let attente = Duration::from_millis(200);
    let repo = crate::msg_repo::PgMsgRepo::new(ent.pool_verif().clone());

    // Existant, puis inconnu.
    for (id, attendu) in [(m, true), (Uuid::new_v4(), false)] {
        let brut = repo.message_existe_sous_verrou_brut(etab, id, attente).await.unwrap();
        let enveloppe = repo.message_existe_sous_verrou(etab, id, attente).await.unwrap();
        assert_eq!((brut, enveloppe), (attendu, attendu), "message {id}");
    }

    // Verrouillé : les deux échouent ; le brut garde 55P03, l'enveloppe en rend la réduction de domaine.
    let lente = annulation_en_cours(&ent, etab, m).await;
    let brut = repo.message_existe_sous_verrou_brut(etab, m, attente).await.unwrap_err();
    assert_eq!(code(&brut).as_deref(), Some("55P03"), "{brut:?}");
    let enveloppe = repo.message_existe_sous_verrou(etab, m, attente).await.unwrap_err();
    assert_eq!(
        format!("{enveloppe:?}"),
        format!("{:?}", crate::pg_err::map_sqlx("msg_repo", brut)),
        "l'enveloppe est la réduction de l'erreur brute"
    );
    lente.rollback().await.unwrap();
    // Verrou levé : les deux revoient le message.
    assert!(repo.message_existe_sous_verrou_brut(etab, m, attente).await.unwrap());
    assert!(repo.message_existe_sous_verrou(etab, m, attente).await.unwrap());
}

/// T3 (E1 passe 5, test à ajouter) : une annulation **validée après le report** de l'avis. Le distributeur trouve le
/// message verrouillé par l'annulation en cours : l'avis est reporté (`en_attente`, pas perdu). L'annulation est ensuite
/// validée (le message disparaît) : au passage suivant, l'avis est écarté avec le motif `message_annule`, et rien n'est parti.
#[tokio::test]
async fn p5_e1_une_annulation_validee_apres_le_report_ecarte_l_avis_avec_message_annule() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Annulation après report").await;
    let m = message_differe(&ent, etab, dir, fil, "Annulé après report").await;
    let avis = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, m).await;
    let apres = apres_l_echeance(&ent).await;

    let mut lente = annulation_en_cours(&ent, etab, m).await;
    let exp = ent.expediteur(crate::notifications::ConfigExpedition {
        taille_lot: 10,
        etablissement: Some(etab),
        attente_verrou: Duration::from_millis(200),
        ..crate::notifications::ConfigExpedition::default()
    });
    let b1 = exp.expedier(apres).await.unwrap();
    assert!(b1.reportes >= 1, "l'avis est reporté, le message étant verrouillé : {b1:?}");
    assert_eq!(statut_avis(&ent, avis).await.0, "en_attente", "reporté, pas perdu ni parti");

    // L'annulation est validée : le message disparaît (dans la transaction qui le tenait).
    sqlx::query("DELETE FROM ent_msg WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(m)
        .execute(&mut *lente)
        .await
        .unwrap();
    lente.commit().await.unwrap();

    // Passage suivant, plus tard que le report : l'avis est écarté pour message annulé, rien n'est parti.
    let plus_tard = apres + chrono::Duration::minutes(5);
    // Palier des faibles, lot 3 (faible de la contre-revue T3) : la constante « report < 5 min » est vérifiée, pas
    // supposée ; sinon le passage suivant ne reprendrait pas l'avis et le test passerait pour une autre raison.
    let replanifie: DateTime<Utc> =
        sqlx::query_scalar("SELECT planifie_pour FROM ent_notification_envoi WHERE id = $1")
            .bind(avis)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert!(replanifie > apres && replanifie < plus_tard, "report de {replanifie} hors de ]{apres}, {plus_tard}[");
    let b2 = exp.expedier(plus_tard).await.unwrap();
    assert_eq!(
        statut_avis(&ent, avis).await,
        ("ignore".to_string(), Some("message_annule".to_string())),
        "annulation validée après report : avis écarté : {b2:?}"
    );
}

/// Palier des faibles, lot 3 (faible 2 de la contre-revue E1 passe 5) : trois avis du **même** message verrouillé ne
/// coûtent qu'une attente de verrou dans le lot (les suivants sont reportés aussitôt), et tous les trois sont reportés.
#[tokio::test]
async fn faibles_l3_f2_un_message_verrouille_n_est_attendu_qu_une_fois_par_lot() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Une attente par lot").await;
    let m = message_differe(&ent, etab, dir, fil, "Verrouillé longtemps").await;
    let mut avis = Vec::new();
    for canal in ["push", "email", "sms"] {
        avis.push(avis_canal(&ent, etab, lily, cast.eleve_lea_id, m, canal).await);
    }
    let apres = apres_l_echeance(&ent).await;
    let lente = annulation_en_cours(&ent, etab, m).await;
    let attente = Duration::from_millis(1500);
    let exp = ent.expediteur(crate::notifications::ConfigExpedition {
        taille_lot: 10,
        etablissement: Some(etab),
        attente_verrou: attente,
        ..crate::notifications::ConfigExpedition::default()
    });
    // Mesure d'une durée écoulée (pas une date) : une attente de 1,5 s, pas trois.
    let debut = std::time::Instant::now();
    let b = exp.expedier(apres).await.unwrap();
    let duree = debut.elapsed();
    assert!(b.reportes >= 3, "les trois avis sont reportés : {b:?}");
    assert!(
        duree < attente * 2,
        "une seule attente de verrou pour le même message ({duree:?})"
    );
    for a in avis {
        assert_eq!(statut_avis(&ent, a).await.0, "en_attente", "reporté, pas perdu");
    }
    lente.rollback().await.unwrap();
}

/// Palier des faibles, lot 3 (faible 3 de la contre-revue E1 passe 5, D-069) : un avis dont le contrôle d'existence
/// échoue encore plus de 24 h après son **premier** échec passe en `echec` (`controle_existence_impossible`) au lieu de
/// reboucler. Contre-revue : un avis créé il y a longtemps (planifié loin devant) dont c'est le premier échec est seulement
/// reporté, jamais perdu.
#[tokio::test]
async fn faibles_l3_f3_un_avis_en_echec_de_controle_depuis_24_heures_passe_en_echec() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (cast.etablissement_id, cast.direction.user_id, cast.parent_lea.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Report sans fin").await;
    let m = message_differe(&ent, etab, dir, fil, "Toujours verrouillé").await;
    let vieux = avis_canal(&ent, etab, lily, cast.eleve_lea_id, m, "push").await;
    let ancien_neuf = avis_canal(&ent, etab, lily, cast.eleve_lea_id, m, "email").await;
    let apres = apres_l_echeance(&ent).await;
    // `vieux` : premier échec de contrôle il y a 25 h. `ancien_neuf` : créé il y a 36 h (planifié loin devant), jamais en
    // échec de contrôle.
    sqlx::query("UPDATE ent_notification_envoi SET controle_echec_depuis = $2 - interval '25 hours' WHERE id = $1")
        .bind(vieux)
        .bind(apres)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    sqlx::query("UPDATE ent_notification_envoi SET created_at = $2 - interval '36 hours' WHERE id = $1")
        .bind(ancien_neuf)
        .bind(apres)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let lente = annulation_en_cours(&ent, etab, m).await;
    let exp = ent.expediteur(crate::notifications::ConfigExpedition {
        taille_lot: 10,
        etablissement: Some(etab),
        attente_verrou: Duration::from_millis(200),
        ..crate::notifications::ConfigExpedition::default()
    });
    let b = exp.expedier(apres).await.unwrap();
    assert!(b.echecs >= 1 && b.reportes >= 1, "{b:?}");
    assert_eq!(
        statut_avis(&ent, vieux).await,
        ("echec".to_string(), Some("controle_existence_impossible".to_string()))
    );
    assert_eq!(statut_avis(&ent, ancien_neuf).await.0, "en_attente", "premier échec : reporté, pas perdu");
    let depuis: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT controle_echec_depuis FROM ent_notification_envoi WHERE id = $1")
            .bind(ancien_neuf)
            .fetch_one(ent.pool_verif())
            .await
            .unwrap();
    assert_eq!(depuis, Some(apres), "premier échec daté de l'instant du passage");
    lente.rollback().await.unwrap();
}

/// Avis d'un message sur un canal donné (même message, plusieurs canaux : clé de dédoublonnage par canal).
async fn avis_canal(ent: &PgEnt, etab: Uuid, lily: Uuid, eleve: Uuid, m: Uuid, canal: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_notification_envoi
            (id, etablissement_id, destinataire_user_id, canal, kind, titre, corps, eleve_id,
             source_type, source_id, cle_dedup, droit_requis, planifie_pour)
         VALUES ($1,$2,$3,$7,'messagerie','Nouveau message','Un message vous attend',$4,'systeme',$5,$6,'lien',
                 now() + interval '2 hours')",
    )
    .bind(id)
    .bind(etab)
    .bind(lily)
    .bind(eleve)
    .bind(m)
    .bind(format!("msg-avis:{m}:{canal}"))
    .bind(canal)
    .execute(ent.pool_verif())
    .await
    .unwrap();
    id
}
