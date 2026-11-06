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

// ───────────────────────── RF-01 : prise de compte par changement d'e-mail ─────────────────────────

/// Dernier jeton d'un lien `…/<chemin>?token=…` envoyé à `a` (messagerie à blanc).
fn lien_envoye(ent: &PgEnt, a: &str, chemin: &str) -> Option<String> {
    ent.mailer()
        .dry_run_corps()
        .into_iter()
        .rev()
        .filter(|(to, _, _)| to == a)
        .find_map(|(_, _, corps)| {
            let i = corps.find(&format!("{chemin}?token="))? + chemin.len() + "?token=".len();
            Some(corps[i..].chars().take_while(|c| c.is_ascii_hexdigit()).collect::<String>())
        })
}

async fn email_de(pool: &sqlx::PgPool, id: Uuid) -> String {
    sqlx::query_scalar("SELECT email FROM ent_user WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn gele(pool: &sqlx::PgPool, id: Uuid) -> bool {
    sqlx::query_scalar("SELECT COALESCE(recuperation_gel_jusqu > now(), false) FROM ent_user WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

const BASE: &str = "http://localhost:8088";

/// Rejoue l'enchaînement décrit par la revue (RF-01) et vérifie qu'il échoue :
/// 1. un Administratif ne change pas l'e-mail d'un compte du personnel ;
/// 2. une Direction le peut, mais rien ne change avant la confirmation par la nouvelle
///    adresse, et l'ancienne adresse reçoit un lien d'annulation ;
/// 3. même confirmée par l'attaquant, « mot de passe oublié » n'envoie rien, un jeton de
///    réinitialisation est refusé et une récupération de passkey en cours est annulée
///    (gel 72 h) ;
/// 4. le lien d'annulation de la victime rétablit son adresse ;
/// 5. un pair (Direction → Direction, Admin → Admin) est refusé.
#[tokio::test]
async fn rf01_prise_de_compte_par_changement_d_email_echoue() {
    let Some(mut ent) = ent().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    ent.install_mail_dry_run();
    let url = std::env::var("ENT_DATABASE_URL").unwrap();
    let pool = sqlx::PgPool::connect(&url).await.expect("pool");
    let etab = Uuid::new_v4();
    let sfx = Uuid::new_v4().simple().to_string();
    let mk = |r: &str| format!("rf01-{r}-{sfx}@ent.local");
    let dir = ent.register_user(&mk("dir"), MDP, None, etab, "direction").await.unwrap();
    let dir2 = ent.register_user(&mk("dir2"), MDP, None, etab, "direction").await.unwrap();
    let adm = ent.register_user(&mk("adm"), MDP, None, etab, "administratif").await.unwrap();
    let admin = ent.register_user(&mk("admin"), MDP, None, etab, "admin").await.unwrap();
    let admin2 = ent.register_user(&mk("admin2"), MDP, None, etab, "admin").await.unwrap();
    let victime = ent.register_user(&mk("psy"), MDP, None, etab, "psy").await.unwrap();
    let pirate = format!("rf01-pirate-{sfx}@ent.local");

    // 1. L'Administratif (voie historique de l'attaque) est refusé sur le personnel.
    let r = ent
        .set_user_email_for_actor("administratif", etab, adm.id, victime.id, &pirate, None, BASE)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "administratif → psy : {r:?}");
    // 5. Pairs refusés.
    for (role, acteur, cible) in [("direction", dir.id, dir2.id), ("admin", admin.id, admin2.id)] {
        let r = ent.set_user_email_for_actor(role, etab, acteur, cible, &pirate, None, BASE).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "pair {role} : {r:?}");
    }

    // La victime a une récupération de passkey en attente (cas d'un compte à passkey).
    let recup = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_recuperation_passkey
             (id, user_id, etablissement_id, statut, confirm_hash, confirm_expires_at,
              annul_hash, final_hash, effective_at, final_expires_at)
         VALUES ($1, $2, $3, 'en_attente', $4, now() + interval '1 hour', $5, $6,
                 now() + interval '1 hour', now() + interval '3 days')",
    )
    .bind(recup)
    .bind(victime.id)
    .bind(etab)
    .bind(format!("c-{recup}"))
    .bind(format!("a-{recup}"))
    .bind(format!("f-{recup}"))
    .execute(&pool)
    .await
    .expect("récupération factice");

    // 2. La Direction demande le changement : rien n'est effectif.
    ent.set_user_email_for_actor("direction", etab, dir.id, victime.id, &pirate, None, BASE)
        .await
        .expect("demande acceptée (niveau inférieur)");
    assert_eq!(
        email_de(&pool, victime.id).await,
        mk("psy"),
        "l'adresse ne change pas avant confirmation"
    );
    let annul = lien_envoye(&ent, &mk("psy"), "/email/annuler")
        .expect("l'ancienne adresse reçoit un lien d'annulation");
    let confirm = lien_envoye(&ent, &pirate, "/email/confirmer")
        .expect("la nouvelle adresse reçoit un lien de confirmation");
    // La récupération en cours est annulée dès la demande.
    let (statut,): (String,) =
        sqlx::query_as("SELECT statut FROM ent_recuperation_passkey WHERE id = $1")
            .bind(recup)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(statut, "annulee", "récupération en cours annulée");

    // 3. L'attaquant confirme (il contrôle la nouvelle boîte) : l'adresse change…
    ent.confirmer_changement_email(&confirm).await.expect("confirmation");
    assert_eq!(email_de(&pool, victime.id).await, pirate);
    // … le lien est à usage unique …
    assert!(matches!(
        ent.confirmer_changement_email(&confirm).await,
        Err(EntDbError::EmailLienInvalide)
    ));
    // … mais « mot de passe oublié » vers sa boîte est gelé : aucun e-mail.
    let avant = ent.mailer().dry_run_corps().len();
    let envoye = ent
        .demander_reinitialisation_mot_de_passe(&pirate, BASE)
        .await
        .expect("réponse neutre");
    assert!(!envoye, "gel : pas de lien de réinitialisation");
    assert_eq!(ent.mailer().dry_run_corps().len(), avant, "aucun e-mail parti");
    // Un jeton de réinitialisation obtenu autrement est refusé pendant le gel.
    let jeton = ent.reset_token_for_test(&pirate).await.unwrap().unwrap();
    let r = ent.reinitialiser_mot_de_passe_par_token(&jeton, "Pirate-Pass-9876!").await;
    assert!(r.is_err(), "réinitialisation refusée pendant le gel : {r:?}");
    // Le mot de passe de la victime n'a pas changé.
    assert!(ent
        .login_start_client(&pirate, MDP, etab, Some("10.1.1.1"), ClientSession::Web, None)
        .await
        .is_ok());
    // La récupération de passkey est gelée (même avec une passkey).
    sqlx::query(
        "INSERT INTO ent_passkey_credential (id, etablissement_id, user_id, cred_id, passkey, name)
         VALUES ($1, $2, $3, $4, '{}'::jsonb, 'rf01')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(victime.id)
    .bind(Uuid::new_v4().to_string())
    .execute(&pool)
    .await
    .expect("passkey factice");
    assert!(
        !ent.demander_recuperation_passkey(&pirate, etab, BASE).await.unwrap(),
        "récupération de passkey gelée"
    );

    // 4. La victime annule via l'ancienne adresse : son adresse est rétablie, gel maintenu.
    ent.annuler_changement_email(&annul).await.expect("rétablissement");
    assert_eq!(email_de(&pool, victime.id).await, mk("psy"));
    assert!(gele(&pool, victime.id).await, "gel maintenu");
    assert!(matches!(
        ent.annuler_changement_email(&annul).await,
        Err(EntDbError::EmailLienInvalide)
    ));
}

/// Un changement fait par soi-même n'est pas gelé mais reste soumis à la confirmation,
/// et un lien expiré ne confirme rien.
#[tokio::test]
async fn rf01_changement_personnel_confirme_sans_gel() {
    let Some(mut ent) = ent().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    ent.install_mail_dry_run();
    let url = std::env::var("ENT_DATABASE_URL").unwrap();
    let pool = sqlx::PgPool::connect(&url).await.expect("pool");
    let etab = Uuid::new_v4();
    let sfx = Uuid::new_v4().simple().to_string();
    let ancien = format!("rf01s-{sfx}@ent.local");
    let nouveau = format!("rf01s-nouveau-{sfx}@ent.local");
    let u = ent.register_user(&ancien, MDP, None, etab, "prof").await.unwrap();
    ent.set_user_email_for_actor("prof", etab, u.id, u.id, &nouveau, Some(MDP), BASE)
        .await
        .expect("demande");
    assert_eq!(email_de(&pool, u.id).await, ancien);
    let confirm = lien_envoye(&ent, &nouveau, "/email/confirmer").unwrap();
    sqlx::query("UPDATE ent_email_change SET confirm_expires_at = now() - interval '1 second' WHERE user_id = $1")
        .bind(u.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        ent.confirmer_changement_email(&confirm).await,
        Err(EntDbError::EmailLienInvalide)
    ));
    ent.set_user_email_for_actor("prof", etab, u.id, u.id, &nouveau, Some(MDP), BASE)
        .await
        .unwrap();
    let confirm = lien_envoye(&ent, &nouveau, "/email/confirmer").unwrap();
    ent.confirmer_changement_email(&confirm).await.unwrap();
    assert_eq!(email_de(&pool, u.id).await, nouveau);
    assert!(!gele(&pool, u.id).await, "pas de gel pour soi-même");
}

// ───────────────────────── RF-04 (d à g) : chemins annexes bornés aux classes enseignées ─────────────────────────

/// Contexte commun : un professeur, une classe suivie (celle de Léa) et une classe non
/// suivie (celle de Tom), si elles diffèrent dans le cast labo.
struct Rf04 {
    ent: PgEnt,
    etab: Uuid,
    prof: Uuid,
    suivie: String,
    non_suivie: String,
    periode: Uuid,
    eleve_suivi: Uuid,
    eleve_non_suivi: Uuid,
    direction: Uuid,
}

async fn rf04() -> Option<Rf04> {
    let (ent, cast, pool) = labo().await?;
    let etab = cast.etablissement_id;
    let prof = cast.profs[0].user_id;
    let classe = |id: Uuid| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, String>("SELECT classe FROM ent_eleve WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap()
        }
    };
    let (eleve_suivi, eleve_non_suivi) = (cast.eleve_lea_id, cast.eleve_tom_id);
    let suivie = classe(eleve_suivi).await;
    let non_suivie = classe(eleve_non_suivi).await;
    assert!(ent.prof_suit_eleve(etab, prof, eleve_suivi).await.unwrap());
    assert!(!ent.prof_suit_eleve(etab, prof, eleve_non_suivi).await.unwrap());
    assert_ne!(suivie, non_suivie, "le cast doit avoir deux classes distinctes");
    let periode = ent.list_note_periodes(etab).await.unwrap()[0].id;
    Some(Rf04 {
        ent,
        etab,
        prof,
        suivie,
        non_suivie,
        periode,
        eleve_suivi,
        eleve_non_suivi,
        direction: cast.direction.user_id,
    })
}

/// RF-04 (d) : exports LSL et LSUN refusés pour une classe non enseignée.
#[tokio::test]
async fn rf04d_exports_lsl_lsun_bornes_aux_classes_enseignees() {
    let Some(c) = rf04().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let jour = domain_eleve::today_utc();
    let lsl = |classe: &str| domain_eleve::ExporterLsl {
        etablissement_id: c.etab,
        periode_id: c.periode,
        classe: Some(classe.to_string()),
        jour_reference: jour,
    };
    let lsun = |classe: &str| domain_eleve::ExporterLsun {
        etablissement_id: c.etab,
        periode_id: c.periode,
        classe: Some(classe.to_string()),
        jour_reference: jour,
    };
    let r = c.ent.exporter_lsl_csv_for_actor("prof", c.prof, lsl(&c.non_suivie)).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "LSL classe non enseignée : {r:?}");
    let r = c.ent.exporter_lsun_csv_for_actor("prof", c.prof, lsun(&c.non_suivie)).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "LSUN classe non enseignée : {r:?}");
    let r = c.ent.exporter_lsl_csv_for_actor("prof", c.prof, lsl(&c.suivie)).await;
    assert!(!matches!(r, Err(EntDbError::Forbidden)), "LSL classe enseignée : {r:?}");
    let r = c.ent.exporter_lsun_csv_for_actor("prof", c.prof, lsun(&c.suivie)).await;
    assert!(!matches!(r, Err(EntDbError::Forbidden)), "LSUN classe enseignée : {r:?}");
    // Notanet et Parcoursup : même borne (revue indépendante).
    let r = c
        .ent
        .exporter_notanet_csv_for_actor(
            "prof",
            c.prof,
            domain_eleve::ExporterNotanet {
                etablissement_id: c.etab,
                periode_id: c.periode,
                classe: c.non_suivie.clone(),
            },
            jour,
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "Notanet classe non enseignée : {r:?}");
    let r = c
        .ent
        .exporter_parcoursup_csv_for_actor(
            "prof",
            c.prof,
            domain_eleve::ExporterParcoursup {
                etablissement_id: c.etab,
                annee_scolaire: "2026-2027".into(),
                classe: c.non_suivie.clone(),
            },
            jour,
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "Parcoursup classe non enseignée : {r:?}");
    // La direction garde sa portée.
    let r = c.ent.exporter_lsl_csv_for_actor("direction", c.direction, lsl(&c.non_suivie)).await;
    assert!(!matches!(r, Err(EntDbError::Forbidden)), "direction : {r:?}");
}

/// RF-04 (e) : publication d'un bulletin refusée pour un élève non suivi.
#[tokio::test]
async fn rf04e_publication_bulletin_bornee_aux_eleves_suivis() {
    let Some(c) = rf04().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let r = c
        .ent
        .publier_bulletin_for_actor("prof", c.etab, c.eleve_non_suivi, c.periode, c.prof)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "élève non suivi : {r:?}");
    let r = c
        .ent
        .publier_bulletin_for_actor("prof", c.etab, c.eleve_suivi, c.periode, c.prof)
        .await;
    assert!(!matches!(r, Err(EntDbError::Forbidden)), "élève suivi : {r:?}");
}

/// RF-04 (f) : conseils de classe — un professeur n'agit que sur ses classes ; la
/// direction garde la portée de l'établissement.
#[tokio::test]
async fn rf04f_conseils_de_classe_bornes_aux_classes_enseignees() {
    let Some(c) = rf04().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let creer = |classe: &str, actor: Uuid| domain_eleve::CreerConseilClasse {
        etablissement_id: c.etab,
        classe: classe.to_string(),
        periode_libelle: "RF-04".into(),
        jour: domain_eleve::today_utc(),
        heure: chrono::NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
        lieu: "Salle".into(),
        actor_user_id: actor,
    };
    let r = c.ent.creer_conseil_classe_for_actor("prof", creer(&c.non_suivie, c.prof)).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "création hors classe : {r:?}");
    // Conseil d'une classe non enseignée créé par la direction : le prof n'y touche pas.
    let conseil = c
        .ent
        .creer_conseil_classe_for_actor("direction", creer(&c.non_suivie, c.direction))
        .await
        .expect("direction : toute classe");
    let r = c
        .ent
        .tenir_conseil_classe_for_actor(c.etab, "prof", conseil.id, c.prof)
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "tenir hors classe : {r:?}");
    let r = c
        .ent
        .ajouter_dossier_conseil_for_actor(
            "prof",
            domain_eleve::AjouterDossierConseil {
                etablissement_id: c.etab,
                conseil_id: conseil.id,
                eleve_id: c.eleve_non_suivi,
                actor_user_id: c.prof,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "dossier hors classe : {r:?}");
    let r = c.ent.list_dossiers_conseil_for_actor(c.etab, c.prof, "prof", conseil.id).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "liste des dossiers hors classe : {r:?}");
    let liste = c
        .ent
        .list_conseils_classe_for_actor(c.etab, c.prof, "prof", None)
        .await
        .expect("liste");
    assert!(liste.iter().all(|x| x.classe != c.non_suivie), "liste bornée aux classes du prof");
    // Décision sur le dossier d'un élève non suivi (créé par la direction) : refusée.
    let dossier = c
        .ent
        .ajouter_dossier_conseil_for_actor(
            "direction",
            domain_eleve::AjouterDossierConseil {
                etablissement_id: c.etab,
                conseil_id: conseil.id,
                eleve_id: c.eleve_non_suivi,
                actor_user_id: c.direction,
            },
        )
        .await
        .expect("direction : ajout");
    let r = c
        .ent
        .decider_dossier_conseil_for_actor(
            "prof",
            domain_eleve::DeciderDossierConseil {
                etablissement_id: c.etab,
                dossier_id: dossier.id,
                appreciation: "x".into(),
                mention: "aucune".into(),
                decision_texte: "x".into(),
                visible_famille: true,
                actor_user_id: c.prof,
            },
        )
        .await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "décision hors périmètre : {r:?}");
    // Sa propre classe reste possible.
    let r = c.ent.creer_conseil_classe_for_actor("prof", creer(&c.suivie, c.prof)).await;
    assert!(r.is_ok(), "classe enseignée : {r:?}");
}

/// RF-04 (g) : devoirs et cahier de textes bornés aux classes enseignées.
#[tokio::test]
async fn rf04g_devoirs_et_cahier_de_textes_bornes_aux_classes_enseignees() {
    let Some(c) = rf04().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let matiere = c.ent.list_matieres(c.etab).await.unwrap()[0].id;
    let periode_ouverte = c
        .ent
        .list_note_periodes(c.etab)
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.statut.autorise_saisie_notes())
        .expect("le cast labo doit avoir une période de notes ouverte")
        .id;
    let devoir = |classe: &str, actor: Uuid| domain_eleve::CreerDevoir {
        etablissement_id: c.etab,
        periode_id: periode_ouverte,
        matiere_id: matiere,
        classe: classe.to_string(),
        titre: "RF-04".into(),
        date_devoir: domain_eleve::today_utc(),
        coef: 1.0,
        sur: 20.0,
        actor_user_id: actor,
    };
    let r = c.ent.creer_devoir_for_actor("prof", devoir(&c.non_suivie, c.prof)).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "devoir hors classe : {r:?}");
    // Un devoir d'une classe non enseignée (créé par la direction) ne se supprime ni ne se liste.
    let autre = c
        .ent
        .creer_devoir_for_actor("direction", devoir(&c.non_suivie, c.direction))
        .await
        .expect("direction : toute classe");
    let r = c.ent.supprimer_devoir_for_actor(c.etab, "prof", c.prof, autre.id).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "suppression hors classe : {r:?}");
    let liste = c
        .ent
        .list_devoirs_pour_acteur(c.etab, c.prof, "prof", periode_ouverte, None)
        .await
        .expect("liste");
    assert!(liste.iter().all(|d| d.classe != c.non_suivie), "liste bornée");
    let r = c.ent.creer_devoir_for_actor("prof", devoir(&c.suivie, c.prof)).await;
    assert!(!matches!(r, Err(EntDbError::Forbidden)), "classe enseignée : {r:?}");

    let cahier = |classe: &str| domain_eleve::CreerCahierTexte {
        etablissement_id: c.etab,
        classe: classe.to_string(),
        matiere_libelle: "Test".into(),
        titre: "RF-04".into(),
        consigne: "x".into(),
        pour_le: domain_eleve::today_utc(),
        cree_le: domain_eleve::today_utc(),
        actor_user_id: c.prof,
    };
    let r = c.ent.creer_cahier_texte_for_actor("prof", cahier(&c.non_suivie)).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "cahier hors classe : {r:?}");
    let r = c.ent.creer_cahier_texte_for_actor("prof", cahier(&c.suivie)).await;
    assert!(r.is_ok(), "cahier classe enseignée : {r:?}");
}

/// RF-01 (revue indépendante) : deux changements enchaînés par un tiers — le lien
/// d'annulation du PREMIER rétablit l'adresse d'origine ; au-delà de 3 demandes par
/// 24 h sur une même cible, refus.
#[tokio::test]
async fn rf01_chaine_de_changements_retablie_et_plafonnee() {
    let Some(mut ent) = ent().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    ent.install_mail_dry_run();
    let url = std::env::var("ENT_DATABASE_URL").unwrap();
    let pool = sqlx::PgPool::connect(&url).await.expect("pool");
    let etab = Uuid::new_v4();
    let sfx = Uuid::new_v4().simple().to_string();
    let mk = |r: &str| format!("rf01c-{r}-{sfx}@ent.local");
    let dir = ent.register_user(&mk("dir"), MDP, None, etab, "direction").await.unwrap();
    let cible = ent.register_user(&mk("prof"), MDP, None, etab, "prof").await.unwrap();
    let (a1, a2) = (mk("a1"), mk("a2"));
    ent.set_user_email_for_actor("direction", etab, dir.id, cible.id, &a1, None, BASE).await.unwrap();
    let annul1 = lien_envoye(&ent, &mk("prof"), "/email/annuler").unwrap();
    let c1 = lien_envoye(&ent, &a1, "/email/confirmer").unwrap();
    ent.confirmer_changement_email(&c1).await.unwrap();
    ent.set_user_email_for_actor("direction", etab, dir.id, cible.id, &a2, None, BASE).await.unwrap();
    let c2 = lien_envoye(&ent, &a2, "/email/confirmer").unwrap();
    ent.confirmer_changement_email(&c2).await.unwrap();
    assert_eq!(email_de(&pool, cible.id).await, a2);
    ent.annuler_changement_email(&annul1).await.expect("annulation du premier lien");
    assert_eq!(email_de(&pool, cible.id).await, mk("prof"), "adresse d'origine rétablie");
    // Plafond : 2 demandes déjà faites, une 3ᵉ passe, la 4ᵉ est refusée.
    ent.set_user_email_for_actor("direction", etab, dir.id, cible.id, &mk("a3"), None, BASE).await.unwrap();
    let r = ent
        .set_user_email_for_actor("direction", etab, dir.id, cible.id, &mk("a4"), None, BASE)
        .await;
    assert!(r.is_err(), "plafond de demandes : {r:?}");
}

// ───────────────────────── Revue socle 1 : constats faibles (portée du professeur) ─────────────────────────

/// Lecture du cahier de textes d'une autre classe refusée au professeur ; sa classe reste lisible.
#[tokio::test]
async fn faible_cahier_de_textes_lecture_bornee_a_la_classe_enseignee() {
    let Some(c) = rf04().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let lire = |classe: &str, actor: Uuid, role: &'static str| {
        c.ent.list_cahier_texte_for_actor(
            c.etab,
            actor,
            role,
            Some(classe.to_string()),
            None,
            None,
            None,
        )
    };
    let r = lire(&c.non_suivie, c.prof, "prof").await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "autre classe : {r:?}");
    let r = lire(&c.suivie, c.prof, "prof").await;
    assert!(r.is_ok(), "classe enseignée : {r:?}");
    let r = lire(&c.non_suivie, c.direction, "direction").await;
    assert!(r.is_ok(), "direction : toute classe : {r:?}");
}

/// Une note ne s'enregistre pas sur le devoir d'une autre classe que celle de l'élève.
#[tokio::test]
async fn faible_note_sur_un_devoir_d_une_autre_classe_refusee() {
    let Some(c) = rf04().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let matiere = c.ent.list_matieres(c.etab).await.unwrap()[0].id;
    let periode = c
        .ent
        .list_note_periodes(c.etab)
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.statut.autorise_saisie_notes())
        .expect("période de notes ouverte")
        .id;
    let devoir = |classe: &str| domain_eleve::CreerDevoir {
        etablissement_id: c.etab,
        periode_id: periode,
        matiere_id: matiere,
        classe: classe.to_string(),
        titre: "Faible".into(),
        date_devoir: domain_eleve::today_utc(),
        coef: 1.0,
        sur: 20.0,
        actor_user_id: c.direction,
    };
    let autre = c.ent.creer_devoir_for_actor("direction", devoir(&c.non_suivie)).await.unwrap();
    let sien = c.ent.creer_devoir_for_actor("direction", devoir(&c.suivie)).await.unwrap();
    let note = |devoir_id: Uuid| crate::note_repo::SaisirNote {
        etablissement_id: c.etab,
        eleve_id: c.eleve_suivi,
        matiere_id: matiere,
        valeur: 12.0,
        sur: 20.0,
        date_note: domain_eleve::today_utc(),
        auteur_user_id: c.prof,
        libelle: None,
        devoir_id: Some(devoir_id),
        periode_id: None,
    };
    let r = c.ent.saisir_note_for_actor("prof", note(autre.id)).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "devoir d'une autre classe : {r:?}");
    let r = c.ent.saisir_note_for_actor("prof", note(sien.id)).await;
    assert!(r.is_ok(), "devoir de la classe de l'élève : {r:?}");
}

/// Trombinoscope : le professeur ne voit que ses élèves ; direction voit la classe.
#[tokio::test]
async fn faible_trombinoscope_borne_aux_eleves_du_professeur() {
    let Some(c) = rf04().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let jour = domain_eleve::today_utc();
    let autre = c
        .ent
        .list_trombi_classe_for_actor(c.etab, c.prof, "prof", &c.non_suivie, jour)
        .await
        .unwrap();
    assert!(autre.is_empty(), "classe non enseignée : {} élèves vus", autre.len());
    let sien = c
        .ent
        .list_trombi_classe_for_actor(c.etab, c.prof, "prof", &c.suivie, jour)
        .await
        .unwrap();
    assert!(sien.iter().any(|t| t.eleve_id == c.eleve_suivi));
    let tout = c
        .ent
        .list_trombi_classe_for_actor(c.etab, c.direction, "direction", &c.non_suivie, jour)
        .await
        .unwrap();
    assert!(tout.iter().any(|t| t.eleve_id == c.eleve_non_suivi), "direction : toute la classe");
}

/// D-022 : l'export ONDE est réservé aux administratifs ; jamais le professeur.
#[tokio::test]
async fn faible_export_onde_reserve_aux_administratifs() {
    let Some(c) = rf04().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cmd = |classe: Option<String>| domain_eleve::ExporterOnde {
        etablissement_id: c.etab,
        classe,
        jour_reference: domain_eleve::today_utc(),
    };
    for role in ["prof", "cpe", "compta"] {
        let r = c.ent.exporter_onde_csv_for_actor(role, cmd(Some(c.non_suivie.clone()))).await;
        assert!(matches!(r, Err(EntDbError::Forbidden)), "{role} : {r:?}");
    }
    let r = c.ent.exporter_onde_csv_for_actor("direction", cmd(Some(c.non_suivie.clone()))).await;
    assert!(r.is_ok(), "direction : {r:?}");
}

/// RF-11 : le verrou par IP (bourrage) ne ferme pas la porte aux comptes déjà connus depuis
/// ce réseau (NAT d'établissement), mais refuse toujours un compte inconnu depuis cette IP.
#[tokio::test]
async fn rf11_verrou_ip_epargne_les_comptes_connus_du_reseau() {
    struct Garde;
    impl Drop for Garde {
        fn drop(&mut self) {
            std::env::remove_var("ENT_LOGIN_MAX_FAILED_IP");
        }
    }
    let Some(ent) = ent().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let connu = format!("rf11c-{}@ent.local", Uuid::new_v4().simple());
    let inconnu = format!("rf11i-{}@ent.local", Uuid::new_v4().simple());
    ent.register_user(&connu, MDP, None, etab, "prof").await.unwrap();
    ent.register_user(&inconnu, MDP, None, etab, "prof").await.unwrap();
    let b = Uuid::new_v4().into_bytes();
    let ip = format!("10.{}.{}.{}", b[0], b[1], b[2]);

    // Le professeur « connu » s'est déjà connecté depuis le collège.
    ent.login_with_ip(&connu, MDP, etab, Some(&ip)).await.expect("connexion initiale");

    let _g = Garde;
    std::env::set_var("ENT_LOGIN_MAX_FAILED_IP", "3");
    // Un élève épuise le seuil par IP avec des comptes quelconques.
    for i in 0..6 {
        let r = ent
            .login_with_ip(&format!("bourrage-{i}-{}@ent.local", Uuid::new_v4().simple()), "x", etab, Some(&ip))
            .await;
        assert!(r.is_err());
    }
    let r = ent.login_with_ip(&inconnu, MDP, etab, Some(&ip)).await;
    assert!(format!("{:?}", r.err()).contains("TooMany"), "compte sans session depuis cette IP : refusé");
    let r = ent.login_with_ip(&connu, MDP, etab, Some(&ip)).await;
    assert!(r.is_ok(), "compte connu depuis ce réseau : épargné : {:?}", r.err());
}

/// RF-26 : `%` et `_` saisis dans une recherche du personnel sont littéraux (plus de joker).
#[tokio::test]
async fn faible_rf26_recherche_personnel_sans_jokers() {
    let Some(ent) = ent().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    for n in 0..2 {
        let email = format!("rf26-{n}-{}@ent.local", Uuid::new_v4().simple());
        ent.register_user(&email, MDP, None, etab, "prof").await.unwrap();
    }
    assert!(ent.search_staff(etab, "rf26", 100).await.unwrap().len() >= 2);
    for joker in ["%", "_", "r_26", "%26-%", "\\"] {
        let r = ent.search_staff(etab, joker, 100).await.unwrap();
        assert!(r.is_empty(), "« {joker} » ne doit rien ramener : {} résultats", r.len());
    }
}

/// RF-33 : l'e-mail n'est plus écrit en clair dans le journal `auth` (empreinte seulement).
#[tokio::test]
async fn faible_rf33_journal_auth_sans_email_en_clair() {
    let Some((ent, cast, pool)) = labo().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let email = cast.administratif.email.clone();
    // Une connexion réussie écrit une trace d'accès `auth`.
    let _ = ent.login_with_ip(&email, crate::seed::LABO_PASSWORD, cast.etablissement_id, None).await;
    let en_clair: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_journal_modif WHERE domaine = 'auth' AND payload::text ILIKE '%' || $1 || '%'",
    )
    .bind(&email)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(en_clair, 0, "e-mail en clair dans le journal auth");
    let avec_empreinte: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_journal_modif WHERE domaine = 'auth' AND payload ? 'email_empreinte' AND etablissement_id = $1",
    )
    .bind(cast.etablissement_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(avec_empreinte > 0, "la trace d'accès doit porter l'empreinte");
}

/// RF-35 : une contrainte d'emploi du temps ne vise qu'un professeur de l'établissement.
#[tokio::test]
async fn faible_rf35_contrainte_edt_prof_non_membre_refusee() {
    let Some(c) = rf04().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let cmd = |prof: Uuid| domain_eleve::CreerEdtContrainte {
        etablissement_id: c.etab,
        kind: domain_eleve::EdtContrainteKind::ProfIndispo,
        prof_user_id: Some(prof),
        salle_code: None,
        jour_semaine: Some(1),
        heure_debut: None,
        heure_fin: None,
        detail: "RF-35".into(),
        actor_user_id: c.direction,
    };
    let r = c.ent.creer_edt_contrainte_for_actor("direction", cmd(Uuid::new_v4())).await;
    assert!(r.is_err(), "professeur inconnu de l'établissement : {r:?}");
    let r = c.ent.creer_edt_contrainte_for_actor("direction", cmd(c.prof)).await;
    assert!(r.is_ok(), "professeur de l'établissement : {r:?}");
}

/// RF-15 : le professeur ne liste que les dossiers de ses élèves, et ne voit pas (même le
/// titre de) un document de santé auquel il n'a pas droit.
#[tokio::test]
async fn faible_rf15_metadonnees_documents_bornees() {
    let Some(c) = rf04().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    use domain_eleve::{DeposerDocument, DocConfidentialite, DocTiroir};
    let depot = |eleve: Uuid, titre: &str, tiroir, conf| DeposerDocument {
        etablissement_id: c.etab,
        eleve_id: eleve,
        tiroir,
        titre: titre.into(),
        code: None,
        mime_type: None,
        storage_key: None,
        confidentialite: conf,
        actor_user_id: c.direction,
    };
    for eleve in [c.eleve_suivi, c.eleve_non_suivi] {
        c.ent
            .deposer_document(depot(eleve, "RF15-pathologie-secrete", DocTiroir::Sante, DocConfidentialite::Sante))
            .await
            .unwrap();
        c.ent
            .deposer_document(depot(eleve, "RF15-standard", DocTiroir::Scolaire, DocConfidentialite::Standard))
            .await
            .unwrap();
    }
    let r = c.ent.list_documents_meta_eleve_for_actor(c.etab, c.prof, "prof", c.eleve_non_suivi).await;
    assert!(matches!(r, Err(EntDbError::Forbidden)), "élève d'une autre classe : {r:?}");
    let liste = c
        .ent
        .list_documents_meta_eleve_for_actor(c.etab, c.prof, "prof", c.eleve_suivi)
        .await
        .expect("élève suivi");
    assert!(liste.iter().all(|(d, _)| !d.titre.contains("pathologie")), "titre santé exposé");
    let vu = c
        .ent
        .list_documents_meta_eleve_for_actor(c.etab, c.direction, "direction", c.eleve_non_suivi)
        .await
        .unwrap();
    assert!(vu.iter().any(|(d, _)| d.titre.contains("RF15-standard")), "direction : dossier complet");
}
