//! Revue de sécurité finale du socle 1 (2026-11-06) : tests d'attaque contre la base
//! locale de test (feature `labo`). Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.
//! Détail des constats : `docs/REVUE_SECURITE_SOCLE1.md`.

use super::*;
use ent_auth::AuthError;
use uuid::Uuid;

async fn ent() -> Option<PgEnt> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

const MDP: &str = "Revue-Finale-Pass-1!";

/// RF-03 : un « e-mail » forgé `compte-ip:victime@x|<ip>` ne doit pas incrémenter le
/// verrou (compte, IP) de la victime depuis une autre IP.
#[tokio::test]
async fn rf03_cle_de_verrou_non_empoisonnable() {
    let Some(ent) = ent().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let victime = format!("rf03-{}@ent.local", Uuid::new_v4().simple());
    ent.register_user(&victime, MDP, None, etab, "prof").await.expect("inscription");
    let b = Uuid::new_v4().into_bytes();
    let ip_victime = format!("10.{}.{}.{}", b[0], b[1], b[2]);
    let ip_attaquant = format!("10.{}.{}.{}", b[3], b[4], b[5]);
    let forge = format!("compte-ip:{victime}|{ip_victime}");

    // 10 échecs depuis l'IP de l'attaquant, avec la clé forgée comme « e-mail ».
    for _ in 0..10 {
        let r = ent
            .login_start_client(&forge, "mauvais", etab, Some(&ip_attaquant), ClientSession::Web, None)
            .await;
        assert!(r.is_err());
    }
    // La victime se connecte toujours depuis son IP.
    let r = ent
        .login_start_client(&victime, MDP, etab, Some(&ip_victime), ClientSession::Web, None)
        .await;
    assert!(r.is_ok(), "victime verrouillée par une clé forgée : {:?}", r.err());
}

/// RF-05 : `POST /auth/password` compte les échecs du mot de passe actuel (verrou
/// `reauth:` après 5 échecs), même avec le bon mot de passe ensuite.
#[tokio::test]
async fn rf05_changement_de_mot_de_passe_limite() {
    let Some(ent) = ent().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let email = format!("rf05-{}@ent.local", Uuid::new_v4().simple());
    let u = ent.register_user(&email, MDP, None, etab, "prof").await.expect("inscription");
    let nouveau = "Nouveau-Pass-Solide-2!";
    for i in 0..5 {
        let r = ent
            .change_password(etab, u.id, Uuid::new_v4(), &format!("essai-{i}-xxxxxx"), nouveau)
            .await;
        assert!(r.is_err(), "essai {i}");
    }
    let r = ent.change_password(etab, u.id, Uuid::new_v4(), MDP, nouveau).await;
    assert!(
        matches!(r, Err(EntDbError::Auth(AuthError::TooManyAttempts))),
        "6e essai (bon mot de passe) doit être verrouillé : {r:?}"
    );
    // Le compteur de connexion n'est pas touché : la connexion reste possible.
    let r = ent
        .login_start_client(&email, MDP, etab, Some("10.9.9.9"), ClientSession::Web, None)
        .await;
    assert!(r.is_ok(), "{:?}", r.err());
}

async fn labo() -> Option<(PgEnt, LaboCast, sqlx::PgPool)> {
    let url = std::env::var("ENT_DATABASE_URL").ok()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    ent.install_brain_p0();
    let cast = ent.seed_labo_cast().await.expect("seed cast");
    let pool = sqlx::PgPool::connect(&url).await.expect("pool");
    Some((ent, cast, pool))
}

/// RF-10 : un parent dont le droit « autoriser les sorties » est décoché ne peut ni
/// poser d'autorisation de sortie ni demander une sortie exceptionnelle.
#[tokio::test]
async fn rf10_parent_sans_droit_sorties_refuse() {
    let Some((ent, cast, pool)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let lien: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT l.user_id, l.eleve_id FROM ent_eleve_lien_actif l
           JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND m.role = 'parent' AND NOT l.peut_autoriser_sorties
          LIMIT 1",
    )
    .bind(etab)
    .fetch_optional(&pool)
    .await
    .expect("lien");
    let Some((parent, eleve)) = lien else {
        panic!("le cast labo doit contenir un parent sans droit de sortie");
    };
    let r = ent
        .upsert_autorisation_sortie_for_actor(
            "parent",
            domain_eleve::UpsertAutorisationSortie {
                etablissement_id: etab,
                eleve_id: eleve,
                matin: true,
                midi: true,
                soir: true,
                interdit_etablissement: false,
                actor_user_id: parent,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
}

/// RF-11 : un professeur n'écrit pas l'appréciation d'un élève qu'il ne suit pas.
#[tokio::test]
async fn rf11_appreciation_bornee_aux_eleves_suivis() {
    let Some((ent, cast, _)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let rogue = cast.profs[0].user_id;
    let (harry, draco) = (cast.eleve_lea_id, cast.eleve_tom_id);
    let periode = ent.list_periodes(etab, draco).await.expect("periodes");
    let Some(p) = periode.first() else {
        panic!("le cast labo doit avoir une période");
    };
    let cmd = |eleve_id| domain_eleve::SaisirAppreciation {
        etablissement_id: etab,
        periode_id: p.id,
        eleve_id,
        matiere_id: None,
        texte: "Revue finale".into(),
        actor_user_id: rogue,
    };
    let r = ent.saisir_appreciation_for_actor("prof", cmd(draco)).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "élève non suivi : {r:?}");
    let r = ent.saisir_appreciation_for_actor("prof", cmd(harry)).await;
    assert!(!matches!(r, Err(EntDbError::Forbidden)), "élève suivi : {r:?}");
}

/// RF-12 : historique des e-mails et SMS (coordonnées des familles) fermé aux
/// professeurs et autres personnels sans `audit.lire`.
#[tokio::test]
async fn rf12_boites_d_envoi_reservees() {
    let Some((ent, cast, _)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    for role in ["prof", "compta", "infirmerie", "psy"] {
        assert!(ent.list_mails_recents_for_actor(etab, role, 5).await.is_err(), "mails {role}");
        assert!(ent.list_sms_recents_for_actor(etab, role, 5).await.is_err(), "sms {role}");
    }
    assert!(ent.list_mails_recents_for_actor(etab, "administratif", 5).await.is_ok());
    assert!(ent.list_sms_recents_for_actor(etab, "cpe", 5).await.is_ok());
    assert!(ent.list_sms_recents_for_actor(etab, "direction", 5).await.is_ok());
}

/// RF-13 : la liste des dispenses du jour (motif médical) vue par un professeur ne
/// contient que des élèves qu'il suit.
#[tokio::test]
async fn rf13_dispenses_du_jour_bornees_au_professeur() {
    let Some((ent, cast, _)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = cast.etablissement_id;
    let rogue = cast.profs[0].user_id;
    let jour = domain_eleve::today_utc();
    let toutes = ent
        .list_dispenses_vs_jour_for_actor(etab, cast.cpe.user_id, "cpe", jour)
        .await
        .expect("cpe");
    let vues = ent
        .list_dispenses_vs_jour_for_actor(etab, rogue, "prof", jour)
        .await
        .expect("prof");
    for d in &vues {
        assert!(ent.prof_suit_eleve(etab, rogue, d.eleve_id).await.unwrap(), "{d:?}");
    }
    assert!(vues.len() <= toutes.len());
}

/// RF-15 (D-013) : le personnel de A ne change pas le téléphone (identité globale)
/// d'une personne aussi membre de B ; il le peut pour un membre de A seulement.
#[tokio::test]
async fn rf15_telephone_d_un_double_membre_non_modifiable() {
    let Some(ent) = ent().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let url = std::env::var("ENT_DATABASE_URL").unwrap();
    let pool = sqlx::PgPool::connect(&url).await.expect("pool");
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let u = |p: &str| format!("rf15-{p}-{}@ent.local", Uuid::new_v4().simple());
    let admin = ent.register_user(&u("admin"), MDP, None, a, "admin").await.unwrap();
    let double = ent.register_user(&u("double"), MDP, None, a, "parent").await.unwrap();
    let simple = ent.register_user(&u("simple"), MDP, None, a, "parent").await.unwrap();
    sqlx::query(
        "INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1, $2, 'parent', now())",
    )
    .bind(double.id)
    .bind(b)
    .execute(&pool)
    .await
    .unwrap();
    let r = ent
        .set_user_telephone_for_actor("admin", a, admin.id, double.id, "+33600000001")
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "{r:?}");
    ent.set_user_telephone_for_actor("admin", a, admin.id, simple.id, "+33600000002")
        .await
        .expect("membre de A seulement");
    // La personne elle-même garde la main sur son numéro.
    ent.set_user_telephone_for_actor("parent", a, double.id, double.id, "+33600000003")
        .await
        .expect("soi-même");
}
