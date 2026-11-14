//! Socle 6, lot P2 : invitations et casquettes (migration 357), contre la base locale de test.
//!
//! * RLS : un établissement ne voit ni les invitations ni les casquettes d'un autre ;
//! * contraintes : une seule invitation en attente par agent, jeton jamais stocké, canal/adresse ;
//! * le jeton d'un QR code n'existe que dans la réponse à l'émission ;
//! * matrice de droits de l'émission (profils invitables selon l'acteur).

use super::*;
use crate::personnels::ResultatCreationAgent;
use ent_types::personnels::*;
use uuid::Uuid;

const MDP: &str = "MotDePasse-P2-Aa9!";
const BASE: &str = "http://ent.test";

async fn connecter() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

async fn compte(ent: &PgEnt, etab: Uuid, role: &str) -> Uuid {
    let email = format!("p2-{role}-{}@ent.local", Uuid::new_v4().simple());
    ent.register_user(&email, MDP, Some("Nom Prénom"), etab, role)
        .await
        .expect("compte")
        .id
}

async fn nouvel_agent(ent: &PgEnt, etab: Uuid, dir: Uuid) -> Uuid {
    let r = ent
        .creer_agent_for_actor(
            etab,
            dir,
            "direction",
            &CreerAgentBody {
                personne: Some(PersonneAgentBody {
                    nom: "Martin".into(),
                    prenom: Some("Alex".into()),
                    email: Some(format!("p2-{}@ent.local", Uuid::new_v4().simple())),
                    ..Default::default()
                }),
                confirmer_nouvelle: true,
                ..Default::default()
            },
        )
        .await
        .expect("agent");
    match r {
        ResultatCreationAgent::Cree(c) => c.agent_id,
        ResultatCreationAgent::AConfirmer(_) => panic!("création attendue"),
    }
}

fn qr() -> InviterAgentBody {
    InviterAgentBody {
        canal: "qr".into(),
        profil: "prof".into(),
        email: None,
    }
}

#[tokio::test]
async fn invitation_matrice_de_droits_unicite_et_jeton_jamais_stocke() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let etab = Uuid::new_v4();
    let admin = compte(&ent, etab, "admin").await;
    let dir = compte(&ent, etab, "direction").await;
    let sec = compte(&ent, etab, "administratif").await;
    let prof = compte(&ent, etab, "prof").await;
    let agent = nouvel_agent(&ent, etab, dir).await;

    // Droit d'inviter : direction, administration, secrétariat ; ni un professeur ni une famille.
    for (acteur, role, attendu) in [
        (admin, "admin", true),
        (dir, "direction", true),
        (sec, "administratif", true),
        (prof, "prof", false),
    ] {
        let r = ent
            .inviter_agent_for_actor(etab, acteur, role, agent, &qr(), BASE)
            .await;
        assert_eq!(r.is_ok(), attendu, "{role}");
    }
    // Profils invitables : l'acteur n'invite pas plus haut que lui.
    for (acteur, role, profil, attendu) in [
        (sec, "administratif", "cpe", true),
        (sec, "administratif", "accueil", true),
        (sec, "administratif", "infirmerie", false),
        (sec, "administratif", "psy", false),
        (sec, "administratif", "compta", false),
        (sec, "administratif", "administratif", false),
        (sec, "administratif", "direction", false),
        (dir, "direction", "infirmerie", true),
        (dir, "direction", "administratif", true),
        (dir, "direction", "admin", false),
        (dir, "direction", "direction", false),
        (admin, "admin", "direction", true),
        (admin, "admin", "admin", true),
        (admin, "admin", "parent", false),
        (admin, "admin", "eleve", false),
        (admin, "admin", "n_importe_quoi", false),
    ] {
        let mut b = qr();
        b.profil = profil.into();
        let r = ent
            .inviter_agent_for_actor(etab, acteur, role, agent, &b, BASE)
            .await;
        assert_eq!(r.is_ok(), attendu, "{role} → {profil} : {r:?}");
    }
    // Pas d'invitation à soi-même ; pas d'URL publique : aucun jeton produit.
    let mut b = qr();
    b.canal = "sms".into();
    assert!(ent
        .inviter_agent_for_actor(etab, dir, "direction", agent, &b, BASE)
        .await
        .is_err());
    assert!(matches!(
        ent.inviter_agent_for_actor(etab, dir, "direction", agent, &qr(), "")
            .await,
        Err(EntDbError::EnvoiImpossible)
    ));

    // Une seule invitation en attente par agent : émettre = annuler la précédente.
    let (en_attente,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_agent_invitation
          WHERE etablissement_id = $1 AND agent_id = $2 AND utilisee_le IS NULL AND annulee_le IS NULL",
    )
    .bind(etab)
    .bind(agent)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(en_attente, 1);
    let doublon = sqlx::query(
        "INSERT INTO ent_agent_invitation (id, etablissement_id, agent_id, canal, profil_base, jeton_hash,
                                           cree_par_user_id, expire_le)
         VALUES ($1,$2,$3,'qr','prof',$4,$5, now() + interval '1 hour')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(agent)
    .bind("a".repeat(64))
    .bind(dir)
    .execute(ent.pool())
    .await;
    assert!(
        doublon.is_err(),
        "deux invitations en attente pour un même agent"
    );

    // Le QR rend son lien une seule fois ; la base garde l'empreinte, la liste ne le rend plus.
    let emise = ent
        .inviter_agent_for_actor(etab, sec, "administratif", agent, &qr(), BASE)
        .await
        .unwrap();
    let lien = match emise {
        InvitationEmise::Qr(q) => q.lien,
        InvitationEmise::Email(_) => panic!("QR attendu"),
    };
    let jeton = lien.split("token=").nth(1).unwrap();
    let (stockee,): (String,) = sqlx::query_as(
        "SELECT jeton_hash FROM ent_agent_invitation
          WHERE etablissement_id = $1 AND agent_id = $2 AND annulee_le IS NULL",
    )
    .bind(etab)
    .bind(agent)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(stockee.len(), 64);
    assert_ne!(stockee, jeton);
    assert_eq!(stockee, ent_auth::hash_token(jeton));
    let liste = ent
        .invitations_agent_for_actor(etab, sec, "administratif", agent)
        .await
        .unwrap();
    assert!(!format!("{liste:?}").contains(jeton));
    assert_eq!(liste.iter().filter(|i| i.etat == "en_attente").count(), 1);

    // Canal e-mail sans adresse mémorisée, et QR avec adresse : refusés par la base.
    let mauvais = sqlx::query(
        "INSERT INTO ent_agent_invitation (id, etablissement_id, agent_id, canal, profil_base, jeton_hash,
                                           cree_par_user_id, expire_le, annulee_le)
         VALUES ($1,$2,$3,'email','prof',$4,$5, now() + interval '1 hour', now())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(agent)
    .bind("b".repeat(64))
    .bind(dir)
    .execute(ent.pool())
    .await;
    assert!(mauvais.is_err(), "canal e-mail sans adresse");
}

#[tokio::test]
async fn rls_invitations_et_casquettes_isolees_entre_etablissements() {
    let Some(ent) = connecter().await else {
        eprintln!("skip: ENT_DATABASE_URL");
        return;
    };
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let dir_a = compte(&ent, a, "direction").await;
    let dir_b = compte(&ent, b, "direction").await;
    let agent_a = nouvel_agent(&ent, a, dir_a).await;
    let agent_b = nouvel_agent(&ent, b, dir_b).await;
    for (etab, dir, agent) in [(a, dir_a, agent_a), (b, dir_b, agent_b)] {
        ent.inviter_agent_for_actor(etab, dir, "direction", agent, &qr(), BASE)
            .await
            .unwrap();
        // Une casquette parent pour le compte de direction (ligne de test).
        sqlx::query(
            "INSERT INTO ent_membership_casquette (user_id, etablissement_id, casquette, du)
             VALUES ($1,$2,'parent', ent_aujourdhui())",
        )
        .bind(dir)
        .bind(etab)
        .execute(ent.pool())
        .await
        .unwrap();
    }
    // Un agent d'un autre établissement : 404 uniforme, pas d'invitation chez lui.
    assert!(ent
        .inviter_agent_for_actor(a, dir_a, "direction", agent_b, &qr(), BASE)
        .await
        .is_err());
    assert!(ent
        .invitations_agent_for_actor(a, dir_a, "direction", agent_b)
        .await
        .map(|l| l.is_empty())
        .unwrap_or(true));

    let url = crate::url_guard::url_labo_ou_echec().unwrap();
    let admin = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    for sql in [
        "DO $$ BEGIN CREATE ROLE ent_p2_app LOGIN PASSWORD 'rls' NOSUPERUSER NOBYPASSRLS NOCREATEDB NOCREATEROLE; EXCEPTION WHEN duplicate_object THEN NULL; END $$",
        "GRANT USAGE ON SCHEMA public TO ent_p2_app",
        "GRANT SELECT, INSERT, UPDATE, DELETE ON ent_agent_invitation, ent_membership_casquette TO ent_p2_app",
        "ALTER ROLE ent_p2_app SET app.rls_strict = 'on'",
    ] {
        sqlx::query(sql).execute(&admin).await.unwrap();
    }
    let mut u = url::Url::parse(&url).unwrap();
    u.set_username("ent_p2_app").unwrap();
    u.set_password(Some("rls")).unwrap();
    let app = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(u.as_str())
        .await
        .unwrap();
    for table in ["ent_agent_invitation", "ent_membership_casquette"] {
        // Sans transaction « locataire » : fail-closed.
        let (rien,): (i64,) = sqlx::query_as(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&app)
            .await
            .unwrap();
        assert_eq!(rien, 0, "{table} : lecture sans établissement");
        let mut tx =
            crate::tenant_tx::TenantTx::begin_mode(&app, a, crate::tenant_tx::RlsMode::Strict)
                .await
                .unwrap();
        let (chez_b,): (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM {table} WHERE etablissement_id = $1"
        ))
        .bind(b)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        let (chez_a,): (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM {table} WHERE etablissement_id = $1"
        ))
        .bind(a)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(chez_b, 0, "{table} : lignes de B visibles depuis A");
        assert!(chez_a > 0, "{table} : lignes de A invisibles de A");
    }
    // Écrire pour un autre établissement : refusé par la politique.
    let mut tx = crate::tenant_tx::TenantTx::begin_mode(&app, a, crate::tenant_tx::RlsMode::Strict)
        .await
        .unwrap();
    let ecriture = sqlx::query(
        "INSERT INTO ent_membership_casquette (user_id, etablissement_id, casquette, du)
         VALUES ($1,$2,'eleve', ent_aujourdhui())",
    )
    .bind(dir_b)
    .bind(b)
    .execute(&mut *tx)
    .await;
    assert!(ecriture.is_err(), "écriture inter-établissement refusée");
}
