//! Messagerie, suites E1 (passe 4, migration 494) : droit d'exécution du jeton d'annulation (M1), budget d'attente de
//! verrou d'un lot (F1), contrôle d'existence sous rôle applicatif non propriétaire (F5), vrai flux famille avec un
//! co-parent (F7). F6 (`xmin` hors flux, verrou empoisonné) : tests de `msg_xmin_gele`. F8 : la sonde de verrou de
//! `tests_messagerie_p3` ne regarde plus que les sessions bloquées par sa propre connexion.
//!
//! Aucune heure réelle : instants de la base ; les attentes sont des durées de verrou côté base ou des sondes de verrou.

use super::tests_messagerie_m1::{labo, role};
use super::tests_messagerie_m5::{est_introuvable, fil_de_lily};
use super::tests_messagerie_p2::{apres_l_echeance, avis_planifie, statut_avis};
use super::tests_messagerie_p3::{code, pool_applicatif, ROLE_APP};
use super::tests_messagerie_revue::{depart, flux, message_differe};
use super::*;
use crate::notifications::{BudgetVerrou, ATTENTE_MINIMALE};
use ent_auth::Role;
use std::time::Duration;
use uuid::Uuid;

const ROLE_SANS: &str = "ent_s16_p4_sans";

/// Rôle **sans droit d'écriture** sur la messagerie (lecture seule), 494 rejouée : elle ne lui donne rien.
pub(super) async fn pool_sans_droit(ent: &PgEnt) -> sqlx::PgPool {
    let admin = ent.pool_verif();
    sqlx::query(&format!(
        "DO $$ BEGIN CREATE ROLE {ROLE_SANS} LOGIN PASSWORD 'p4' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; \
         EXCEPTION WHEN duplicate_object THEN NULL; END $$"
    ))
    .execute(admin)
    .await
    .unwrap();
    for sql in [
        format!("REVOKE ALL ON ALL TABLES IN SCHEMA public FROM {ROLE_SANS}"),
        format!("GRANT USAGE ON SCHEMA public TO {ROLE_SANS}"),
        format!("GRANT SELECT ON ALL TABLES IN SCHEMA public TO {ROLE_SANS}"),
    ] {
        sqlx::query(&sql).execute(admin).await.unwrap();
    }
    sqlx::raw_sql(include_str!(
        "../../../migrations/494_msg_garde_jeton_droit_execution.sql"
    ))
    .execute(admin)
    .await
    .unwrap();
    let url = crate::url_guard::url_labo_ou_echec().expect("url labo");
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username(ROLE_SANS).unwrap();
    u.set_password(Some("p4")).unwrap();
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(u.as_str())
        .await
        .unwrap()
}

// ───────────────────────── M1 : droit d'exécution du jeton ─────────────────────────

/// `PUBLIC` n'a plus le droit d'exécuter `ent_msg_annulation_ouvrir` ; un rôle sans droit d'écriture sur la messagerie
/// est refusé (42501) ; le rôle applicatif (qui écrit dans `ent_msg`) l'obtient par 494 et l'exécute.
#[tokio::test]
async fn p4_m1_le_jeton_d_annulation_n_est_plus_executable_par_public() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Jeton et droit d'exécution").await;
    let m = message_differe(&ent, etab, dir, fil, "A planifier").await;
    let app = pool_applicatif(&ent).await;
    let sans = pool_sans_droit(&ent).await;

    // Catalogue : aucun droit d'exécution pour PUBLIC (l'ACL par défaut d'une fonction l'accorde ; REVOKE l'a retiré).
    let (public,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM pg_proc p
           CROSS JOIN LATERAL aclexplode(coalesce(p.proacl, acldefault('f', p.proowner))) a
          WHERE p.proname = 'ent_msg_annulation_ouvrir' AND a.grantee = 0 AND a.privilege_type = 'EXECUTE'",
    )
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert_eq!(public, 0, "PUBLIC ne peut plus exécuter la fonction");
    let (app_ok, sans_ok): (bool, bool) = sqlx::query_as(
        "SELECT has_function_privilege($1, 'ent_msg_annulation_ouvrir(uuid,uuid,uuid)', 'EXECUTE'),
                has_function_privilege($2, 'ent_msg_annulation_ouvrir(uuid,uuid,uuid)', 'EXECUTE')",
    )
    .bind(ROLE_APP)
    .bind(ROLE_SANS)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(
        app_ok,
        "le rôle applicatif (qui écrit dans ent_msg) a le droit"
    );
    assert!(!sans_ok, "un rôle sans le GRANT ne l'a pas");

    // Appel réel : refus exact sans le GRANT (le rôle n'a pas le droit d'exécuter, avant toute vérification de l'auteur).
    let mut tx = sans.begin().await.unwrap();
    let e = sqlx::query("SELECT ent_msg_annulation_ouvrir($1, $2, $3)")
        .bind(etab)
        .bind(m)
        .bind(dir)
        .execute(&mut *tx)
        .await
        .expect_err("sans le GRANT");
    assert_eq!(code(&e).as_deref(), Some("42501"), "{e:?}");
    assert!(e.to_string().contains("permission denied"), "{e}");
    drop(tx);

    // Témoin : sous le rôle applicatif, l'auteur ouvre le jeton.
    let mut tx = app.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SELECT ent_msg_annulation_ouvrir($1, $2, $3)")
        .bind(etab)
        .bind(m)
        .bind(dir)
        .execute(&mut *tx)
        .await
        .expect("le rôle applicatif exécute la fonction");
    drop(tx);
}

/// Contrôle au démarrage : le rôle applicatif a le droit (`Accorde`) ; un rôle sans `GRANT` est signalé (`Manquant`).
#[tokio::test]
async fn p4_a_le_controle_au_demarrage_signale_le_grant_manquant() {
    use crate::controle_demarrage::{droit_annulation, DroitAnnulation};
    let Some((ent, _cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let app = pool_applicatif(&ent).await;
    let sans = pool_sans_droit(&ent).await;
    assert_eq!(droit_annulation(&app).await.unwrap(), DroitAnnulation::Accorde);
    assert_eq!(droit_annulation(&sans).await.unwrap(), DroitAnnulation::Manquant);
    // Le point d'entrée du démarrage rend le même verdict sur le pool de l'instance (propriétaire de la base du labo).
    assert_eq!(
        ent.controler_droit_annulation_messagerie().await,
        droit_annulation(ent.pool()).await.unwrap()
    );
}

// ───────────────────────── F1 : attente de verrou cumulée d'un lot ─────────────────────────

#[test]
fn p4_f1_le_budget_se_partage_et_s_epuise_sans_devenir_nul() {
    let mut b = BudgetVerrou::nouveau(Duration::from_secs(10));
    // Un contrôle reçoit au plus son attente propre, au plus le budget restant.
    assert_eq!(b.accorder(Duration::from_secs(4)), Duration::from_secs(4));
    b.consommer(Duration::from_secs(4));
    b.consommer(Duration::from_secs(4));
    assert_eq!(b.restant(), Duration::from_secs(2));
    assert_eq!(
        b.accorder(Duration::from_secs(4)),
        Duration::from_secs(2),
        "plafonné au restant"
    );
    // Au-delà du budget : saturation, et l'attente ne tombe jamais à zéro (lock_timeout = 0 voudrait dire « sans limite »).
    b.consommer(Duration::from_secs(60));
    assert_eq!(b.restant(), Duration::ZERO);
    assert_eq!(b.accorder(Duration::from_secs(4)), ATTENTE_MINIMALE);
    assert!(ATTENTE_MINIMALE > Duration::ZERO);
    // Un budget nul d'emblée (perime_apres = 0) est équivalent.
    assert_eq!(
        BudgetVerrou::nouveau(Duration::ZERO).accorder(Duration::from_secs(5)),
        ATTENTE_MINIMALE
    );
    // La somme des attentes accordées d'une suite de contrôles bloqués ne dépasse pas le budget, plus 1 ms par contrôle
    // une fois épuisé.
    let total = Duration::from_millis(900);
    let mut b = BudgetVerrou::nouveau(total);
    let mut accordees = Duration::ZERO;
    let controles = 50u32;
    for _ in 0..controles {
        let a = b.accorder(Duration::from_millis(400));
        accordees += a;
        b.consommer(a); // pire cas : chaque contrôle attend tout ce qu'on lui accorde
    }
    assert!(
        accordees <= total + ATTENTE_MINIMALE * controles,
        "{accordees:?}"
    );
    assert!(accordees >= total, "le budget est entièrement utilisable");
}

/// Un lot dont **tous** les messages sont verrouillés par une annulation lente : l'attente totale est bornée par
/// `perime_apres` (ici 300 ms), pas par `attente_verrou` (60 s) × nombre d'avis. Les avis sont reportés, jamais perdus ;
/// le lot finit ; l'avis du message libre est traité.
#[tokio::test]
async fn p4_f1_l_attente_cumulee_d_un_lot_est_bornee_par_perime_apres() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
    );
    let (fil, _) = fil_de_lily(&ent, &cast, "Budget de verrou").await;
    let m1 = message_differe(&ent, etab, dir, fil, "Verrouillé 1").await;
    let m2 = message_differe(&ent, etab, dir, fil, "Verrouillé 2").await;
    let m3 = message_differe(&ent, etab, dir, fil, "Verrouillé 3").await;
    let libre = message_differe(&ent, etab, dir, fil, "Libre").await;
    let mut avis = Vec::new();
    for m in [m1, m2, m3] {
        avis.push(avis_planifie(&ent, etab, lily, cast.eleve_lea_id, m).await);
    }
    let avis_libre = avis_planifie(&ent, etab, lily, cast.eleve_lea_id, libre).await;
    let apres = apres_l_echeance(&ent).await;

    // Une annulation lente tient les trois messages.
    let mut lente = ent.pool_verif().begin().await.unwrap();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *lente)
        .await
        .unwrap();
    for m in [m1, m2, m3] {
        sqlx::query("SELECT 1 FROM ent_msg WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
            .bind(etab)
            .bind(m)
            .fetch_one(&mut *lente)
            .await
            .unwrap();
    }
    let exp = ent.expediteur(crate::notifications::ConfigExpedition {
        taille_lot: 500,
        etablissement: Some(etab),
        attente_verrou: Duration::from_secs(60),
        perime_apres: Duration::from_millis(300),
        ..crate::notifications::ConfigExpedition::default()
    });
    // Sans budget partagé, trois attentes de 60 s : le délai de 30 s ci-dessous échouerait.
    let bilan = tokio::time::timeout(Duration::from_secs(30), exp.expedier(apres))
        .await
        .expect("l'attente cumulée du lot est bornée par perime_apres")
        .expect("le lot n'est pas interrompu");
    assert_eq!(
        bilan.reportes, 3,
        "les trois avis verrouillés sont reportés : {bilan:?}"
    );
    for a in &avis {
        assert_eq!(
            statut_avis(&ent, *a).await.0,
            "en_attente",
            "reporté, pas perdu"
        );
    }
    let (statut, motif) = statut_avis(&ent, avis_libre).await;
    assert!(
        !matches!(statut.as_str(), "en_attente" | "envoi"),
        "le message libre est traité : {statut}"
    );
    assert_ne!(motif.as_deref(), Some("message_annule"));
    lente.rollback().await.unwrap();
}

// ───────────────────────── F5 : contrôle d'existence sous rôle applicatif ─────────────────────────

/// `message_existe_sous_verrou` (`SELECT … FOR KEY SHARE`, `lock_timeout` posé) tourne sous le rôle applicatif **non
/// propriétaire** (droits `SELECT/INSERT/UPDATE/DELETE`, comme `ent_app`) : existe, inconnu, autre établissement, verrou
/// tenu. Un rôle en lecture seule ne peut pas poser le verrou de ligne (privilège `UPDATE` requis) : `ent_app` doit
/// garder `UPDATE` sur `ent_msg`.
#[tokio::test]
async fn p4_f5_message_existe_sous_verrou_sous_role_applicatif_non_proprietaire() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir) = (cast.etablissement_id, cast.direction.user_id);
    let (fil, _) = fil_de_lily(&ent, &cast, "Existence sous rôle applicatif").await;
    let m = message_differe(&ent, etab, dir, fil, "Existe").await;
    let app = pool_applicatif(&ent).await;
    let (est_super, est_prop): (bool, bool) = sqlx::query_as(
        "SELECT r.rolsuper, c.relowner = r.oid FROM pg_roles r, pg_class c
          WHERE r.rolname = $1 AND c.oid = 'public.ent_msg'::regclass",
    )
    .bind(ROLE_APP)
    .fetch_one(ent.pool_verif())
    .await
    .unwrap();
    assert!(
        !est_super && !est_prop,
        "précondition : rôle non propriétaire, non superutilisateur"
    );

    let repo = crate::msg_repo::PgMsgRepo::new(app.clone());
    let court = Duration::from_millis(200);
    assert!(
        repo.message_existe_sous_verrou(etab, m, Duration::from_secs(5))
            .await
            .unwrap(),
        "le message existe"
    );
    assert!(
        !repo
            .message_existe_sous_verrou(etab, Uuid::new_v4(), court)
            .await
            .unwrap(),
        "message inconnu"
    );
    assert!(
        !repo
            .message_existe_sous_verrou(Uuid::new_v4(), m, court)
            .await
            .unwrap(),
        "autre établissement : invisible (RLS), donc inexistant"
    );

    // Verrou tenu par une annulation en cours : le contrôle échoue au bout de l'attente (lock_timeout), il n'attend pas
    // indéfiniment, et ne dit pas « inexistant ».
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
    let r = tokio::time::timeout(
        Duration::from_secs(30),
        repo.message_existe_sous_verrou(etab, m, court),
    )
    .await
    .expect("borné par lock_timeout");
    assert!(
        r.is_err(),
        "verrou tenu : erreur (reportée par l'appelant), pas « inexistant » : {r:?}"
    );
    lente.rollback().await.unwrap();
    assert!(
        repo.message_existe_sous_verrou(etab, m, court)
            .await
            .unwrap(),
        "verrou rendu : le message existe"
    );

    // Un rôle en lecture seule ne peut pas poser `FOR KEY SHARE` : le droit `UPDATE` d'`ent_app` est nécessaire.
    let sans = pool_sans_droit(&ent).await;
    let lecture_seule = crate::msg_repo::PgMsgRepo::new(sans);
    let r = lecture_seule
        .message_existe_sous_verrou_brut(etab, m, court)
        .await;
    // Passe 5, faible 2 : le refus est prouvé par son code SQLSTATE (42501), pas seulement par `is_err()`.
    assert_eq!(
        r.as_ref().err().and_then(code).as_deref(),
        Some("42501"),
        "sans UPDATE sur ent_msg : refusé (privilège insuffisant) : {r:?}"
    );
}

// ───────────────────────── F7 : le vrai flux famille, avec un co-parent ─────────────────────────

/// Co-parent de Léa : même foyer, même élève, compte propre, créé **avant** l'ouverture des fils (il en est participant).
async fn co_parent(ent: &PgEnt, cast: &LaboCast) -> Uuid {
    let p = ent
        .register_login(
            &format!("coparent-{}@ent.lab", Uuid::new_v4().simple()),
            Role::Parent,
            "Co-parent de Léa",
            cast.etablissement_id,
        )
        .await
        .expect("compte du co-parent");
    ent.link_parent(crate::foyer_repo::CreateLien {
        etablissement_id: cast.etablissement_id,
        eleve_id: cast.eleve_lea_id,
        user_id: p.user_id,
        foyer_id: Some(cast.foyer_lea_id),
        lien: "père".into(),
        peut_voir_notes: true,
        peut_voir_vs: true,
        peut_justifier_absence: true,
        peut_autoriser_sorties: true,
        est_payeur: false,
        part_paiement: None,
    })
    .await
    .expect("lien du co-parent");
    p.user_id
}

/// Fil dont le seul message est un différé annulé, par le **vrai** chemin (annulation, listes, lecture du fil, flux) et
/// pour les **deux** parents du foyer : aucun ne voit plus le fil, ni son sujet, ni un événement du flux. Témoin : un fil
/// dont le message est remis est vu des deux. Le créateur d'un fil vide (côté famille) le garde, **pas** son co-parent :
/// c'est la règle de la migration 493 (le premier message en deux temps est un brouillon de son auteur).
#[tokio::test]
async fn p4_f7_flux_famille_co_parent_le_fil_vide_apres_annulation_n_est_lisible_de_personne() {
    let Some((ent, cast)) = labo().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (etab, dir, lily) = (
        cast.etablissement_id,
        cast.direction.user_id,
        cast.parent_lea.user_id,
    );
    let coparent = co_parent(&ent, &cast).await;
    let (fil, premier) = fil_de_lily(&ent, &cast, "Sujet-annule-du-co-parent").await;
    sqlx::query("UPDATE ent_msg SET remise_prevue_le = now() + interval '1 hour' WHERE id = $1")
        .bind(premier)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    let (temoin, _) = fil_de_lily(&ent, &cast, "Fil-avec-message-remis").await;
    let curseur = depart(&ent, etab).await;

    let familles = [(lily, "mère"), (coparent, "co-parent")];
    let voit_liste = |user: Uuid, f: Uuid| {
        let ent = &ent;
        async move {
            ent.list_fils_for_actor(etab, user, role(Role::Parent), None, false, None)
                .await
                .unwrap()
                .iter()
                .any(|x| x.id == f)
        }
    };
    // Témoin : le fil remis est vu des deux parents (donc le co-parent est bien participant).
    for (user, nom) in familles {
        assert!(
            voit_liste(user, temoin).await,
            "{nom} : le fil remis est listé"
        );
    }
    // Précondition : le fil dont le message est encore différé n'est listé pour personne côté famille.
    for (user, nom) in familles {
        assert!(
            !voit_liste(user, fil).await,
            "{nom} : précondition, rien avant la remise"
        );
    }

    ent.annuler_message_differe_for_actor(etab, dir, role(Role::Direction), premier)
        .await
        .expect("annulation");

    for (user, nom) in familles {
        assert!(
            !voit_liste(user, fil).await,
            "{nom} : le fil vidé n'est plus listé"
        );
        // Lecture directe du fil : 404 uniforme (un fil vidé ne se distingue pas d'un fil absent).
        let e = ent
            .list_messages_for_actor(etab, user, role(Role::Parent), fil)
            .await
            .expect_err("le fil vidé n'est pas lisible de la famille");
        assert!(est_introuvable(&e), "{nom} : {e:?}");
        // Le flux, depuis le curseur d'avant l'annulation : aucun événement pour ce fil.
        let r = flux(&ent, etab, user, Role::Parent, &curseur).await;
        assert!(!r.resynchro, "{nom} : curseur sain : {r:?}");
        assert!(
            r.fils.iter().all(|f| f.fil_id != fil),
            "{nom} : aucun événement du flux pour le fil vidé : {r:?}"
        );
    }

    // Le créateur côté famille garde son fil vide ; son co-parent ne le voit pas (règle de 493).
    sqlx::query("UPDATE ent_msg_fil SET created_by_user_id = $2 WHERE id = $1")
        .bind(fil)
        .bind(lily)
        .execute(ent.pool_verif())
        .await
        .unwrap();
    assert!(
        voit_liste(lily, fil).await,
        "le créateur garde son fil vide"
    );
    assert!(
        !voit_liste(coparent, fil).await,
        "le co-parent ne voit pas le fil vide d'un autre"
    );
}
