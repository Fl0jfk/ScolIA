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
