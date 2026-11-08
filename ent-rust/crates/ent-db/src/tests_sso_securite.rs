//! Socle 1 — A7 (SSO OIDC) et A8 (compteur de passkey) contre la base locale de test
//! (feature `labo`). Sans `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use ent_auth::Role;
use ent_oidc::testing::{FakeOidcTransport, TestIdp};
use std::sync::Arc;
use uuid::Uuid;

fn param(url: &str, k: &str) -> String {
    url::Url::parse(url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == k)
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default()
}

struct Ctx {
    ent: PgEnt,
    etab: Uuid,
    idp: TestIdp,
    transport: Arc<FakeOidcTransport>,
    slug: String,
}

async fn ctx() -> Option<Ctx> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let etab = Uuid::new_v4();
    let mut idp = TestIdp::new();
    idp.email = format!("sso-{}@idp.test", Uuid::new_v4());
    idp.subject = format!("sub-{}", Uuid::new_v4());
    idp.client_secret = format!("secret-{}", Uuid::new_v4());
    let transport = Arc::new(FakeOidcTransport::new(idp.clone()));
    ent.set_oidc_transport(transport.clone());
    let slug = "idp-secu".to_string();
    ent.upsert_oidc_provider_for_actor(
        etab,
        Role::Direction.as_str(),
        OidcProviderParams {
            slug: slug.clone(),
            libelle: "IdP sécurité".into(),
            issuer: idp.issuer.clone(),
            client_id: idp.client_id.clone(),
            client_secret: idp.client_secret.clone(),
            scopes: None,
            redirect_uri: None,
        },
    )
    .await
    .expect("config SSO");
    Some(Ctx {
        ent,
        etab,
        idp,
        transport,
        slug,
    })
}

impl Ctx {
    /// Démarre un parcours et arme le faux IdP ; renvoie (state, lien navigateur).
    async fn demarrer(&self) -> (String, String) {
        let (url, lien) = self
            .ent
            .oidc_start(self.etab, &self.slug)
            .await
            .expect("start");
        self.transport.arm_nonce(&param(&url, "nonce"));
        self.transport
            .arm_code_challenge(&param(&url, "code_challenge"));
        (param(&url, "state"), lien)
    }

    async fn compte_local(&self, email: &str) -> Uuid {
        self.ent
            .register_user(
                email,
                "SsoLocal1!SsoLocal",
                Some("Compte SSO"),
                self.etab,
                Role::Prof.as_str(),
            )
            .await
            .expect("compte local")
            .id
    }

    async fn secret_en_base(&self) -> String {
        sqlx::query_scalar(
            "SELECT client_secret FROM ent_oidc_provider WHERE etablissement_id=$1 AND slug=$2",
        )
        .bind(self.etab)
        .bind(&self.slug)
        .fetch_one(self.ent.pool())
        .await
        .unwrap()
    }
}

// ---- A7 : secret client chiffré ---------------------------------------------

#[tokio::test]
async fn sso_secret_client_chiffre_en_base_et_dechiffre_a_l_echange() {
    let Some(c) = ctx().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let en_base = c.secret_en_base().await;
    assert!(
        en_base.starts_with("enc:v1:"),
        "secret chiffré en base : {en_base}"
    );
    assert!(
        !en_base.contains(&c.idp.client_secret),
        "aucun clair en base"
    );

    // Les vues de gestion et le Debug ne divulguent rien.
    let liste = c
        .ent
        .list_oidc_providers_for_actor(c.etab, Role::Direction.as_str())
        .await
        .unwrap();
    assert!(!format!("{:?}", liste[0]).contains(&c.idp.client_secret));

    // À l'échange de code, l'IdP reçoit le secret EN CLAIR (déchiffré en mémoire).
    c.compte_local(&c.idp.email).await;
    let (state, lien) = c.demarrer().await;
    c.ent
        .oidc_complete("code", &state, &lien)
        .await
        .expect("connexion");
    assert_eq!(
        c.transport.last_client_secret().as_deref(),
        Some(c.idp.client_secret.as_str())
    );
}

#[tokio::test]
async fn sso_secret_herite_en_clair_lu_puis_rechiffre() {
    let Some(c) = ctx().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    c.compte_local(&c.idp.email).await;
    // Simule une ligne antérieure à la migration : secret en clair.
    sqlx::query(
        "UPDATE ent_oidc_provider SET client_secret=$3 WHERE etablissement_id=$1 AND slug=$2",
    )
    .bind(c.etab)
    .bind(&c.slug)
    .bind(&c.idp.client_secret)
    .execute(c.ent.pool())
    .await
    .unwrap();
    assert!(!c.secret_en_base().await.starts_with("enc:v1:"));

    // Lecture rétrocompatible : la connexion fonctionne, avec le bon secret…
    let (state, lien) = c.demarrer().await;
    c.ent
        .oidc_complete("code", &state, &lien)
        .await
        .expect("connexion avec secret hérité");
    assert_eq!(
        c.transport.last_client_secret().as_deref(),
        Some(c.idp.client_secret.as_str())
    );
    // … et la valeur est rechiffrée au passage.
    assert!(c.secret_en_base().await.starts_with("enc:v1:"));

    // Rechiffrement de masse (boot) : idempotent.
    sqlx::query("UPDATE ent_oidc_provider SET client_secret='clair-bis' WHERE etablissement_id=$1")
        .bind(c.etab)
        .execute(c.ent.pool())
        .await
        .unwrap();
    assert!(c.ent.rechiffrer_secrets_oidc().await.unwrap() >= 1);
    assert!(c.secret_en_base().await.starts_with("enc:v1:"));
    assert_eq!(c.ent.rechiffrer_secrets_oidc().await.unwrap(), 0);
}

// ---- A7 : email_verified -----------------------------------------------------

#[tokio::test]
async fn sso_rattachement_par_email_exige_email_verified_true() {
    let Some(c) = ctx().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let user = c.compte_local(&c.idp.email).await;
    for faux in [Some(false), None] {
        c.transport.set_email_verified(faux);
        let (state, lien) = c.demarrer().await;
        let r = c.ent.oidc_complete("code", &state, &lien).await;
        assert!(
            matches!(
                r,
                Err(EntDbError::Oidc(ent_oidc::OidcError::EmailNotVerified))
            ),
            "email_verified={faux:?} doit être refusé, obtenu {r:?}"
        );
    }
    let nb: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_oidc_identity WHERE user_id=$1")
        .bind(user)
        .fetch_one(c.ent.pool())
        .await
        .unwrap();
    assert_eq!(nb, 0, "aucun rattachement créé par un e-mail non vérifié");

    // Vérifié → rattachement et session.
    c.transport.set_email_verified(Some(true));
    let (state, lien) = c.demarrer().await;
    match c
        .ent
        .oidc_complete("code", &state, &lien)
        .await
        .expect("connexion")
    {
        LoginOutcome::Session(s) => assert_eq!(s.user_id, user),
        other => panic!("session attendue : {other:?}"),
    }
    // Rattachement explicite existant par `sub` : l'état de l'e-mail n'a plus d'importance.
    c.transport.set_email_verified(None);
    let (state, lien) = c.demarrer().await;
    assert!(matches!(
        c.ent.oidc_complete("code", &state, &lien).await,
        Ok(LoginOutcome::Session(_))
    ));
}

// ---- A7 : state lié au navigateur -------------------------------------------

#[tokio::test]
async fn sso_retour_lie_au_navigateur_et_a_usage_unique() {
    let Some(c) = ctx().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    c.compte_local(&c.idp.email).await;

    // Autre navigateur (cookie différent), cookie absent, cookie vide : refus.
    for mauvais in ["autre-navigateur", "", "   "] {
        let (state, _lien) = c.demarrer().await;
        let r = c.ent.oidc_complete("code", &state, mauvais).await;
        assert!(
            matches!(r, Err(EntDbError::OidcStateInvalide)),
            "{mauvais:?} → {r:?}"
        );
    }
    // Un refus consomme le state : le bon navigateur ne peut pas le « rattraper ».
    let (state, lien) = c.demarrer().await;
    assert!(c.ent.oidc_complete("code", &state, "x").await.is_err());
    assert!(matches!(
        c.ent.oidc_complete("code", &state, &lien).await,
        Err(EntDbError::OidcStateInvalide)
    ));
    // Le cookie d'un parcours n'ouvre pas le state d'un autre parcours.
    let (state_a, lien_a) = c.demarrer().await;
    let (state_b, lien_b) = c.demarrer().await;
    assert!(c
        .ent
        .oidc_complete("code", &state_a, &lien_b)
        .await
        .is_err());
    assert!(c
        .ent
        .oidc_complete("code", &state_b, &lien_a)
        .await
        .is_err());

    // Parcours nominal puis rejeu.
    let (state, lien) = c.demarrer().await;
    assert!(matches!(
        c.ent.oidc_complete("code", &state, &lien).await,
        Ok(LoginOutcome::Session(_))
    ));
    assert!(
        c.ent.oidc_complete("code", &state, &lien).await.is_err(),
        "rejeu refusé"
    );

    // État antérieur à la migration (sans binding_hash) : refusé.
    let (state, lien) = c.demarrer().await;
    sqlx::query("UPDATE ent_oidc_state SET binding_hash=NULL WHERE etablissement_id=$1")
        .bind(c.etab)
        .execute(c.ent.pool())
        .await
        .unwrap();
    assert!(c.ent.oidc_complete("code", &state, &lien).await.is_err());
}

// ---- A8 : compteur de signature ---------------------------------------------

#[tokio::test]
async fn passkey_compteur_regresse_refuse_journalise_et_non_ecrase() {
    use ent_auth::{CreationChallengeResponse, RequestChallengeResponse};
    use webauthn_authenticator_rs::prelude::WebauthnAuthenticator;
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;

    let url = match std::env::var("ENT_DATABASE_URL") {
        Ok(u) => u,
        Err(_) => return eprintln!("skip: ENT_DATABASE_URL"),
    };
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let etab = Uuid::new_v4();
    let email = format!("clone-{}@ent.local", Uuid::new_v4());
    let user = ent
        .register_user(
            &email,
            "CloneLocal1!Clone",
            Some("Claire Clone"),
            etab,
            Role::Prof.as_str(),
        )
        .await
        .expect("compte");
    let mut dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let origin = url::Url::parse("http://localhost:3000").unwrap();
    let (reg_token, options) = ent
        .passkey_register_start_for_actor(etab, user.id)
        .await
        .unwrap();
    let ccr: CreationChallengeResponse = serde_json::from_value(options).unwrap();
    let reg = dev.do_registration(origin.clone(), ccr).unwrap();
    ent.passkey_register_finish_for_actor(etab, user.id, &reg_token, &reg, Some("Clé".into()))
        .await
        .unwrap();

    async fn tenter(
        ent: &PgEnt,
        dev: &mut WebauthnAuthenticator<SoftPasskey>,
        origin: &url::Url,
        etab: Uuid,
        email: &str,
    ) -> Result<crate::auth_repo::SessionInfo, EntDbError> {
        let (t, opts, _) = ent.passkey_login_start(etab, email).await.unwrap().unwrap();
        let rcr: RequestChallengeResponse = serde_json::from_value(opts).unwrap();
        let assertion = dev.do_authentication(origin.clone(), rcr).unwrap();
        ent.passkey_login_finish(&t, &assertion).await
    }

    // Connexions normales : le compteur progresse (1, 2).
    for _ in 0..2 {
        tenter(&ent, &mut dev, &origin, etab, &email)
            .await
            .expect("connexion normale");
    }
    let lire_compteur = || async {
        let v: serde_json::Value =
            sqlx::query_scalar("SELECT passkey FROM ent_passkey_credential WHERE user_id=$1")
                .bind(user.id)
                .fetch_one(ent.pool())
                .await
                .unwrap();
        v.pointer("/cred/counter").and_then(|c| c.as_u64()).unwrap()
    };
    assert_eq!(lire_compteur().await, 2);

    // Un clone a fait monter le compteur côté serveur à 1000 : l'original recule.
    sqlx::query(
        "UPDATE ent_passkey_credential SET passkey = jsonb_set(passkey, '{cred,counter}', '1000') WHERE user_id=$1",
    )
    .bind(user.id)
    .execute(ent.pool())
    .await
    .unwrap();
    let r = tenter(&ent, &mut dev, &origin, etab, &email).await;
    assert!(
        matches!(
            r,
            Err(EntDbError::Passkey(
                ent_auth::PasskeyError::PossibleCompromise
            ))
        ),
        "compteur régressé refusé : {r:?}"
    );
    assert_eq!(
        lire_compteur().await,
        1000,
        "le compteur n'est jamais abaissé"
    );
    let msg = r.unwrap_err().to_string();
    assert!(
        msg.contains("authentification par passkey refusée"),
        "{msg}"
    );

    let nb: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_journal_modif WHERE etablissement_id=$1 AND action='passkey_compteur_regresse'",
    )
    .bind(etab)
    .fetch_one(ent.pool())
    .await
    .unwrap();
    assert_eq!(nb, 1, "la régression est journalisée");
}
