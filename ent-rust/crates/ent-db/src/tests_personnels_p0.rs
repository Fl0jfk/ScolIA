//! Socle 6, lot P0 : correctifs de sécurité des accès (migrations 350 et 351), contre la base
//! locale de test.
//!
//! * un accès fermé, suspendu ou échu refuse la connexion et invalide sessions et jetons ouverts ;
//! * un établissement ne perd jamais son dernier administrateur (commandes, SQL direct, course) ;
//! * attributions de rôle datées : début futur, historique, modification sans écrasement ;
//! * bord de journée dans le fuseau de l'établissement (deux fuseaux) ;
//! * isolation entre établissements.
//!
//! Chaque test crée ses propres établissements (identifiants neufs) : pas de dépendance au labo.

use super::*;
use chrono::{Duration, NaiveDate};
use ent_auth::{Action, Portee};
use uuid::Uuid;

const MDP: &str = "MotDePasse-P0-Aa9!";

async fn connecter() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

async fn compte(ent: &PgEnt, etab: Uuid, role: &str) -> Uuid {
    let email = format!("p0-{role}-{}@ent.local", Uuid::new_v4().simple());
    ent.register_user(&email, MDP, Some(role), etab, role)
        .await
        .expect("compte")
        .id
}

async fn email_de(ent: &PgEnt, user: Uuid) -> String {
    let (e,): (String,) = sqlx::query_as("SELECT email FROM ent_user WHERE id = $1")
        .bind(user)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    e
}

async fn connexion(ent: &PgEnt, etab: Uuid, user: Uuid) -> Result<SessionInfo, EntDbError> {
    let email = email_de(ent, user).await;
    ent.login(&email, MDP, etab).await
}

async fn fixer_fuseau(ent: &PgEnt, etab: Uuid, fuseau: &str) {
    sqlx::query(
        "INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau) VALUES ($1, $2)
         ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = EXCLUDED.fuseau",
    )
    .bind(etab)
    .bind(fuseau)
    .execute(ent.pool())
    .await
    .unwrap();
}

async fn fixer_fin(ent: &PgEnt, etab: Uuid, user: Uuid, fin: Option<NaiveDate>) {
    sqlx::query(
        "UPDATE ent_membership SET date_fin = $3 WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(user)
    .bind(fin)
    .execute(ent.pool())
    .await
    .unwrap();
}

fn est_dernier_admin(r: &Result<impl std::fmt::Debug, EntDbError>) -> bool {
    matches!(r, Err(EntDbError::Refus(RefusMetier::Conflit, m)) if m.contains("dernier administrateur"))
}

#[tokio::test]
async fn acces_ferme_refuse_connexion_session_et_jeton_sans_toucher_aux_autres_etablissements() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let admin = compte(&ent, a, "admin").await;
    let _admin2 = compte(&ent, a, "admin").await;
    let prof = compte(&ent, a, "prof").await;
    // La même personne a un second accès, dans l'établissement B.
    sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1, $2, 'prof', now())")
        .bind(prof)
        .bind(b)
        .execute(ent.pool())
        .await
        .unwrap();

    let s_a = connexion(&ent, a, prof).await.expect("connexion A");
    let s_b = connexion(&ent, b, prof).await.expect("connexion B");
    ent.resolve_session(&format!("Bearer {}", s_a.token))
        .await
        .expect("session A");

    // Appareil de notification enregistré pour A.
    sqlx::query("INSERT INTO ent_push_device (id, etablissement_id, user_id, platform, token) VALUES ($1, $2, $3, 'ios', $4)")
        .bind(Uuid::new_v4())
        .bind(a)
        .bind(prof)
        .bind(Uuid::new_v4().to_string())
        .execute(ent.pool())
        .await
        .unwrap();

    // Un prof ne ferme rien ; la direction ne ferme pas un admin ; on ne se ferme pas soi-même.
    let dir = compte(&ent, a, "direction").await;
    assert!(matches!(
        ent.acces_fermer_for_actor(a, prof, "prof", admin, "depart")
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.acces_fermer_for_actor(a, dir, "direction", admin, "depart")
            .await,
        Err(EntDbError::Forbidden)
    ));
    assert!(matches!(
        ent.acces_fermer_for_actor(a, admin, "admin", admin, "depart")
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Motif inconnu : refusé avant toute écriture.
    assert!(matches!(
        ent.acces_fermer_for_actor(a, admin, "admin", prof, "n'importe quoi")
            .await,
        Err(EntDbError::Refus(RefusMetier::Invalide, _))
    ));
    ent.resolve_session(&format!("Bearer {}", s_a.token))
        .await
        .expect("toujours ouverte");

    // Fermeture par l'admin.
    let etat = ent
        .acces_fermer_for_actor(a, admin, "admin", prof, "fin_contrat")
        .await
        .unwrap();
    assert_eq!(etat, EtatAcces::Ferme);
    // Idempotent.
    assert_eq!(
        ent.acces_fermer_for_actor(a, admin, "admin", prof, "fin_contrat")
            .await
            .unwrap(),
        EtatAcces::Ferme
    );
    // Connexion refusée, session et jeton déjà ouverts invalidés.
    assert!(
        connexion(&ent, a, prof).await.is_err(),
        "connexion refusée sur A"
    );
    assert!(
        ent.resolve_session(&format!("Bearer {}", s_a.token))
            .await
            .is_err(),
        "jeton déjà ouvert refusé"
    );
    // Isolation : l'accès B, sa session et sa connexion ne sont pas touchés.
    ent.resolve_session(&format!("Bearer {}", s_b.token))
        .await
        .expect("session B intacte");
    connexion(&ent, b, prof).await.expect("connexion B intacte");
    let (appareils,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_push_device WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(a)
    .bind(prof)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(appareils, 0, "appareils de notification de A supprimés");
    // Aucune session ne peut être rouverte par un autre chemin que la connexion.
    assert!(!ent.auth_repo().is_member(a, prof).await.unwrap());
    assert!(ent.auth_repo().is_member(b, prof).await.unwrap());

    // Suspendre un accès fermé : conflit ; rouvrir puis suspendre : ok, réversible.
    assert!(matches!(
        ent.acces_suspendre_for_actor(a, admin, "admin", prof, "conge")
            .await,
        Err(EntDbError::Refus(RefusMetier::Conflit, _))
    ));
    ent.acces_rouvrir_for_actor(a, admin, "admin", prof)
        .await
        .unwrap();
    connexion(&ent, a, prof).await.expect("rouvert");
    ent.acces_suspendre_for_actor(a, admin, "admin", prof, "conservatoire")
        .await
        .unwrap();
    assert!(
        connexion(&ent, a, prof).await.is_err(),
        "suspendu : connexion refusée"
    );
    let e = ent.acces_etat(a, prof).await.unwrap().unwrap();
    assert_eq!(
        (e.etat, e.motif.as_deref()),
        (EtatAcces::Suspendu, Some("conservatoire"))
    );
    ent.acces_rouvrir_for_actor(a, admin, "admin", prof)
        .await
        .unwrap();

    // Journal : acteur réel, aucune adresse en clair.
    let lignes: Vec<(Option<Uuid>, String)> = sqlx::query_as(
        "SELECT actor_user_id, payload::text FROM ent_journal_modif
          WHERE etablissement_id = $1 AND action IN ('acces_ferme','acces_suspendu','acces_rouvert')",
    )
    .bind(a)
    .fetch_all(ent.pool())
    .await
    .unwrap();
    assert!(lignes.len() >= 4, "{lignes:?}");
    let email = email_de(&ent, prof).await;
    for (acteur, payload) in &lignes {
        assert_eq!(*acteur, Some(admin));
        assert!(
            !payload.contains(&email),
            "adresse en clair dans le journal"
        );
    }

    // Une cible d'un autre établissement : refus uniforme (isolation).
    let admin_b = compte(&ent, b, "admin").await;
    let _admin_b2 = compte(&ent, b, "admin").await;
    let inconnu_de_b = compte(&ent, a, "cpe").await;
    assert!(matches!(
        ent.acces_fermer_for_actor(b, admin_b, "admin", inconnu_de_b, "depart")
            .await,
        Err(EntDbError::Forbidden)
    ));
    // Une famille n'est pas concernée par ces commandes.
    let parent = compte(&ent, a, "parent").await;
    assert!(matches!(
        ent.acces_fermer_for_actor(a, admin, "admin", parent, "depart")
            .await,
        Err(EntDbError::Forbidden)
    ));
}

#[tokio::test]
async fn dernier_administrateur_protege_par_les_commandes_le_sql_direct_et_la_concurrence() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let a = Uuid::new_v4();
    let admin1 = compte(&ent, a, "admin").await;
    let admin2 = compte(&ent, a, "admin").await;
    let aujourdhui = ent.aujourdhui_etablissement(a).await.unwrap();

    // Commandes : deux administrateurs, on peut en fermer un seul.
    ent.acces_fermer_for_actor(a, admin1, "admin", admin2, "depart")
        .await
        .unwrap();
    // `admin2` est désormais fermé ; le dernier est `admin1` : plus aucun chemin ne le retire.
    let r = ent
        .acces_fermer_for_actor(a, admin2, "admin", admin1, "depart")
        .await;
    assert!(est_dernier_admin(&r), "{r:?}");
    let r = ent
        .acces_suspendre_for_actor(a, admin2, "admin", admin1, "conge")
        .await;
    assert!(est_dernier_admin(&r), "{r:?}");
    let r = ent
        .acces_programmer_fin_for_actor(
            a,
            admin2,
            "admin",
            admin1,
            Some(aujourdhui + Duration::days(30)),
        )
        .await;
    assert!(est_dernier_admin(&r), "expiration : {r:?}");

    // SQL direct (maintenance, autre chemin applicatif) : le déclencheur refuse fermeture,
    // suspension, date de fin, retrait de profil et suppression.
    for sql in [
        "UPDATE ent_membership SET etat = 'ferme' WHERE etablissement_id = $1 AND user_id = $2",
        "UPDATE ent_membership SET etat = 'suspendu' WHERE etablissement_id = $1 AND user_id = $2",
        "UPDATE ent_membership SET date_fin = CURRENT_DATE + 5 WHERE etablissement_id = $1 AND user_id = $2",
        "UPDATE ent_membership SET role = 'prof' WHERE etablissement_id = $1 AND user_id = $2",
        "DELETE FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
    ] {
        let r = sqlx::query(sql).bind(a).bind(admin1).execute(ent.pool()).await;
        let msg = format!("{r:?}");
        assert!(msg.contains("dernier_administrateur"), "{sql} → {msg}");
    }
    let e = ent.acces_etat(a, admin1).await.unwrap().unwrap();
    assert_eq!(
        (e.etat, e.date_fin),
        (EtatAcces::Ouvert, None),
        "rien n'a été écrit"
    );

    // Rouvrir le second puis programmer une fin sur l'un des deux : permis ; sur l'autre : refusé.
    ent.acces_rouvrir_for_actor(a, admin1, "admin", admin2)
        .await
        .unwrap();
    ent.acces_programmer_fin_for_actor(
        a,
        admin1,
        "admin",
        admin2,
        Some(aujourdhui + Duration::days(30)),
    )
    .await
    .unwrap();
    // `admin2` a une fin : il n'est plus un administrateur durable, `admin1` redevient le dernier.
    let r = ent
        .acces_programmer_fin_for_actor(
            a,
            admin2,
            "admin",
            admin1,
            Some(aujourdhui + Duration::days(60)),
        )
        .await;
    assert!(est_dernier_admin(&r), "{r:?}");
    // Date de fin passée ou trop lointaine : refusée.
    let r = ent
        .acces_programmer_fin_for_actor(
            a,
            admin1,
            "admin",
            admin2,
            Some(aujourdhui - Duration::days(1)),
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::Refus(RefusMetier::Invalide, _))),
        "{r:?}"
    );

    // Course : deux administrateurs durables, deux fermetures simultanées → une seule passe.
    let c = Uuid::new_v4();
    let (u1, u2) = (
        compte(&ent, c, "admin").await,
        compte(&ent, c, "admin").await,
    );
    let fermer = |pool: sqlx::PgPool, user: Uuid| async move {
        let mut tx = pool.begin().await.unwrap();
        let r = sqlx::query(
            "UPDATE ent_membership SET etat = 'ferme' WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(c)
        .bind(user)
        .execute(&mut *tx)
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        match r {
            Ok(_) => {
                tx.commit().await.unwrap();
                true
            }
            Err(_) => false,
        }
    };
    let (r1, r2) = tokio::join!(
        fermer(ent.pool().clone(), u1),
        fermer(ent.pool().clone(), u2)
    );
    assert_eq!(
        [r1, r2].iter().filter(|ok| **ok).count(),
        1,
        "deux retraits simultanés ne doivent jamais vider l'établissement"
    );
    let (restants,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_membership WHERE etablissement_id = $1 AND role = 'admin' AND etat = 'ouvert'")
        .bind(c)
        .fetch_one(ent.pool())
        .await
        .unwrap();
    assert_eq!(restants, 1);
}

#[tokio::test]
async fn bord_de_journee_deux_fuseaux_acces_et_attribution_echus_a_l_heure_locale() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    // Kiritimati (UTC+14) et Pago Pago (UTC-11) : au moins un jour civil d'écart à tout instant.
    let (kiri, pago, paris) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    fixer_fuseau(&ent, kiri, "Pacific/Kiritimati").await;
    fixer_fuseau(&ent, pago, "Pacific/Pago_Pago").await;
    let (j_kiri, j_pago, j_paris) = (
        ent.aujourdhui_etablissement(kiri).await.unwrap(),
        ent.aujourdhui_etablissement(pago).await.unwrap(),
        ent.aujourdhui_etablissement(paris).await.unwrap(), // sans paramètre : Europe/Paris
    );
    assert!(
        j_kiri > j_pago,
        "deux fuseaux, deux jours : {j_kiri} / {j_pago}"
    );
    let (j_paris_sql,): (NaiveDate,) =
        sqlx::query_as("SELECT (now() AT TIME ZONE 'Europe/Paris')::date")
            .fetch_one(ent.pool())
            .await
            .unwrap();
    assert_eq!(j_paris, j_paris_sql, "valeur par défaut : Europe/Paris");

    // Accès : date de fin = « aujourd'hui à Pago Pago » → ouvert à Pago Pago (jusqu'au soir),
    // déjà échu à Kiritimati (où il est déjà le lendemain).
    let (p_kiri, p_pago) = (
        compte(&ent, kiri, "prof").await,
        compte(&ent, pago, "prof").await,
    );
    let _ = (
        compte(&ent, kiri, "admin").await,
        compte(&ent, pago, "admin").await,
    );
    fixer_fin(&ent, kiri, p_kiri, Some(j_pago)).await;
    fixer_fin(&ent, pago, p_pago, Some(j_pago)).await;
    connexion(&ent, pago, p_pago)
        .await
        .expect("Pago Pago : encore ouvert jusqu'au soir");
    assert!(
        connexion(&ent, kiri, p_kiri).await.is_err(),
        "Kiritimati : échu"
    );
    assert!(ent.auth_repo().is_member(pago, p_pago).await.unwrap());
    assert!(!ent.auth_repo().is_member(kiri, p_kiri).await.unwrap());

    // Une session ouverte avant l'échéance est invalidée au changement de jour local : on avance la
    // fin d'hier (jour local) et la session existante est refusée sans révocation explicite.
    let s = connexion(&ent, pago, p_pago).await.unwrap();
    fixer_fin(&ent, pago, p_pago, Some(j_pago - Duration::days(1))).await;
    assert!(ent
        .resolve_session(&format!("Bearer {}", s.token))
        .await
        .is_err());

    // Attribution de rôle : même mécanique (la fin s'éteint le soir, jour de l'établissement).
    // Cible : un compta (pas de NotesLire en portée établissement par défaut).
    let droit = Droit {
        action: Action::NotesLire,
        portee: Portee::Etablissement,
        cibles: vec![],
    };
    let a_droit = |droits: &[Droit]| {
        droits
            .iter()
            .any(|d| d.action == Action::NotesLire && d.portee == Portee::Etablissement)
    };
    for (etab, jour_local, attendu_actif) in [(pago, j_pago, true), (kiri, j_pago, false)] {
        let admin = compte(&ent, etab, "admin").await;
        let cible = compte(&ent, etab, "compta").await;
        assert!(
            !a_droit(&ent.droits_effectifs(etab, cible, "compta").await.unwrap()),
            "pas de droit par défaut"
        );
        let role = ent
            .creer_role_for_actor(
                etab,
                admin,
                "admin",
                RoleSaisie {
                    nom: format!("Lecture {}", &Uuid::new_v4().simple().to_string()[..6]),
                    description: None,
                    modele: None,
                    droits: vec![droit.clone()],
                },
            )
            .await
            .unwrap();
        // Insertion directe : l'API refuse une fin déjà passée dans le fuseau de l'établissement.
        sqlx::query(
            "INSERT INTO ent_role_attribution (role_id, etablissement_id, user_id, debut, fin)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(role.id)
        .bind(etab)
        .bind(cible)
        .bind(jour_local - Duration::days(30))
        .bind(jour_local)
        .execute(ent.pool())
        .await
        .unwrap();
        let actif = a_droit(&ent.droits_effectifs(etab, cible, "compta").await.unwrap());
        assert_eq!(
            actif, attendu_actif,
            "fin = {jour_local} dans l'établissement {etab}"
        );
    }
}

#[tokio::test]
async fn attributions_datees_debut_futur_historique_et_aucun_ecrasement() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let a = Uuid::new_v4();
    let admin = compte(&ent, a, "admin").await;
    let cible = compte(&ent, a, "compta").await;
    let jour = ent.aujourdhui_etablissement(a).await.unwrap();
    let role = ent
        .creer_role_for_actor(
            a,
            admin,
            "admin",
            RoleSaisie {
                nom: format!("Lecture {}", &Uuid::new_v4().simple().to_string()[..6]),
                description: None,
                modele: None,
                droits: vec![Droit {
                    action: Action::NotesLire,
                    portee: Portee::Etablissement,
                    cibles: vec![],
                }],
            },
        )
        .await
        .unwrap();
    let a_droit = |droits: Vec<Droit>| {
        droits
            .iter()
            .any(|d| d.action == Action::NotesLire && d.portee == Portee::Etablissement)
    };
    let droits = || async { a_droit(ent.droits_effectifs(a, cible, "compta").await.unwrap()) };
    let nb = |sql: &'static str| {
        let ent = &ent;
        async move {
            let (n,): (i64,) = sqlx::query_as(sql)
                .bind(a)
                .bind(role.id)
                .bind(cible)
                .fetch_one(ent.pool())
                .await
                .unwrap();
            n
        }
    };

    // Début futur (rentrée) : accepté, aucun droit avant la date.
    let rentree = jour + Duration::days(40);
    ent.attribuer_role_periode_for_actor(a, admin, "admin", role.id, cible, Some(rentree), None)
        .await
        .unwrap();
    assert!(
        !droits().await,
        "attribution à début futur : pas de droit avant la date"
    );
    // Début passé, fin avant le début : refusés.
    for (d, f) in [
        (Some(jour - Duration::days(1)), None),
        (Some(rentree), Some(rentree - Duration::days(1))),
    ] {
        assert!(ent
            .attribuer_role_periode_for_actor(a, admin, "admin", role.id, cible, d, f)
            .await
            .is_err());
    }
    // Chevauchement avec l'attribution future : conflit (jamais d'écrasement).
    let r = ent
        .attribuer_role_periode_for_actor(
            a,
            admin,
            "admin",
            role.id,
            cible,
            Some(rentree + Duration::days(2)),
            None,
        )
        .await;
    assert!(
        matches!(r, Err(EntDbError::Refus(RefusMetier::Conflit, _))),
        "{r:?}"
    );
    // Retrait : close la ligne future, l'historique reste.
    assert!(ent
        .retirer_role_for_actor(a, admin, "admin", role.id, cible)
        .await
        .unwrap());
    assert!(
        !ent.retirer_role_for_actor(a, admin, "admin", role.id, cible)
            .await
            .unwrap(),
        "déjà retiré"
    );
    assert_eq!(
        nb("SELECT count(*) FROM ent_role_attribution WHERE etablissement_id = $1 AND role_id = $2 AND user_id = $3 AND clos_le IS NOT NULL").await,
        1
    );

    // Attribution immédiate, puis modification de la fin : l'ancienne ligne est close, une nouvelle
    // prend le relais avec le même début ; rien n'est écrasé.
    ent.attribuer_role_for_actor(a, admin, "admin", role.id, cible, None)
        .await
        .unwrap();
    assert!(droits().await);
    ent.attribuer_role_for_actor(
        a,
        admin,
        "admin",
        role.id,
        cible,
        Some(jour + Duration::days(10)),
    )
    .await
    .unwrap();
    assert!(droits().await);
    assert_eq!(
        nb("SELECT count(*) FROM ent_role_attribution WHERE etablissement_id = $1 AND role_id = $2 AND user_id = $3").await,
        3,
        "historique : future close + initiale close + modifiée ouverte"
    );
    assert_eq!(
        nb("SELECT count(*) FROM ent_role_attribution WHERE etablissement_id = $1 AND role_id = $2 AND user_id = $3 AND clos_le IS NULL AND source = 'modification' AND source_ref IS NOT NULL").await,
        1
    );
    // Les membres du rôle ne comptent la personne qu'une fois, lignes closes exclues.
    let roles = ent.list_roles_for_actor(a, admin, "admin").await.unwrap();
    let r = roles.iter().find(|r| r.id == role.id).unwrap();
    assert_eq!(r.membres, vec![cible]);
    // Retrait : droits coupés immédiatement.
    assert!(ent
        .retirer_role_for_actor(a, admin, "admin", role.id, cible)
        .await
        .unwrap());
    assert!(!droits().await);
    // Une personne qui n'est pas un personnel, ou d'un autre établissement : refus.
    let parent = compte(&ent, a, "parent").await;
    assert!(ent
        .attribuer_role_for_actor(a, admin, "admin", role.id, parent, None)
        .await
        .is_err());
    let autre = Uuid::new_v4();
    let admin_autre = compte(&ent, autre, "admin").await;
    assert!(
        ent.attribuer_role_for_actor(autre, admin_autre, "admin", role.id, cible, None)
            .await
            .is_err(),
        "rôle d'un autre établissement"
    );
    // Course : le chevauchement est aussi refusé par la base (contrainte d'exclusion).
    let r = sqlx::query(
        "INSERT INTO ent_role_attribution (role_id, etablissement_id, user_id, debut, fin) VALUES ($1, $2, $3, $4, NULL), ($1, $2, $3, $5, NULL)",
    )
    .bind(role.id)
    .bind(a)
    .bind(cible)
    .bind(jour)
    .bind(jour + Duration::days(1))
    .execute(ent.pool())
    .await;
    assert!(format!("{r:?}").contains("sans_chevauchement"), "{r:?}");
}

#[tokio::test]
async fn annuaire_du_personnel_sans_telephone_ni_acces_ferme() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let a = Uuid::new_v4();
    let admin = compte(&ent, a, "admin").await;
    let _admin2 = compte(&ent, a, "admin").await;
    let prof = compte(&ent, a, "prof").await;
    ent.auth_repo()
        .set_telephone(prof, "0612345678")
        .await
        .unwrap();
    // Le téléphone existe en base mais n'est pas exposé : `StaffMember` n'a plus ce champ.
    let annuaire = ent.search_staff(a, "", 50).await.unwrap();
    assert!(annuaire.iter().any(|m| m.user_id == prof));
    assert!(!format!("{annuaire:?}").contains("0612345678"));
    assert!(!format!("{:?}", ent.get_staff_member(a, prof).await.unwrap()).contains("0612345678"));
    // Un accès fermé disparaît de l'annuaire.
    ent.acces_fermer_for_actor(a, admin, "admin", prof, "depart")
        .await
        .unwrap();
    assert!(!ent
        .search_staff(a, "", 50)
        .await
        .unwrap()
        .iter()
        .any(|m| m.user_id == prof));
}

#[tokio::test]
async fn perimetre_prof_suit_eleve_utilise_le_jour_de_l_etablissement() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    // Un élève et un prof inconnus de l'établissement : défaut sûr (false), quel que soit le fuseau,
    // et la variante sans date lit le jour de l'établissement sans erreur.
    let (kiri, pago) = (Uuid::new_v4(), Uuid::new_v4());
    fixer_fuseau(&ent, kiri, "Pacific/Kiritimati").await;
    fixer_fuseau(&ent, pago, "Pacific/Pago_Pago").await;
    for etab in [kiri, pago] {
        let jour = ent.aujourdhui_etablissement(etab).await.unwrap();
        let (prof, eleve) = (Uuid::new_v4(), Uuid::new_v4());
        assert!(!ent.prof_suit_eleve(etab, prof, eleve).await.unwrap());
        assert_eq!(
            ent.prof_suit_eleve(etab, prof, eleve).await.unwrap(),
            ent.prof_suit_eleve_le(etab, prof, eleve, jour)
                .await
                .unwrap()
        );
    }
}
