//! Audit Fable 1, S-01 : prise de compte inter-établissements par le SSO d'un
//! établissement. Scénarios d'attaque du rapport et non-régressions
//! (`docs/audits/CORRECTIF_S01_B02.md`). Base locale de test (feature `labo`) ;
//! sans `ENT_DATABASE_URL`, les scénarios sont ignorés.

use super::*;
use ent_auth::{CreationChallengeResponse, Role};
use ent_oidc::testing::{FakeOidcTransport, TestIdp};
use std::sync::Arc;
use uuid::Uuid;
use webauthn_authenticator_rs::prelude::WebauthnAuthenticator;
use webauthn_authenticator_rs::softpasskey::SoftPasskey;

const MDP: &str = "VictimeLocale1!Victime";

fn param(url: &str, k: &str) -> String {
    url::Url::parse(url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == k)
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default()
}

/// A : établissement dont la direction déclare un fournisseur qu'elle contrôle.
/// B : autre établissement de la victime.
struct Monde {
    ent: PgEnt,
    a: Uuid,
    b: Uuid,
    transport: Arc<FakeOidcTransport>,
    slug: String,
    provider_id: Uuid,
    direction_a: Uuid,
}

async fn monde() -> Option<Monde> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let mut ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let mut idp = TestIdp::new();
    idp.subject = format!("sub-{}", Uuid::new_v4());
    idp.client_secret = format!("secret-{}", Uuid::new_v4());
    let transport = Arc::new(FakeOidcTransport::new(idp.clone()));
    ent.set_oidc_transport(transport.clone());
    let direction_a = ent
        .register_user(
            &format!("dir-a-{}@ent.local", Uuid::new_v4()),
            MDP,
            Some("Direction A"),
            a,
            Role::Direction.as_str(),
        )
        .await
        .expect("direction A")
        .id;
    let slug = "idp-de-a".to_string();
    let provider = ent
        .upsert_oidc_provider_for_actor(
            a,
            Role::Direction.as_str(),
            OidcProviderParams {
                slug: slug.clone(),
                libelle: "IdP contrôlé par A".into(),
                issuer: idp.issuer.clone(),
                client_id: idp.client_id.clone(),
                client_secret: idp.client_secret.clone(),
                scopes: None,
                redirect_uri: None,
            },
        )
        .await
        .expect("fournisseur de A");
    Some(Monde {
        ent,
        a,
        b,
        transport,
        slug,
        provider_id: provider.id,
        direction_a,
    })
}

impl Monde {
    /// Le fournisseur de A affirme l'adresse `email`, vérifiée, pour un `sub` neuf.
    async fn sso_affirme(&self, email: &str) -> Result<LoginOutcome, EntDbError> {
        self.transport.set_email(email);
        self.transport.set_subject(&format!("sub-{}", Uuid::new_v4()));
        self.transport.set_email_verified(Some(true));
        self.sso_courant().await
    }

    /// Parcours SSO complet avec l'identité actuellement armée sur le faux IdP.
    async fn sso_courant(&self) -> Result<LoginOutcome, EntDbError> {
        let (url, lien) = self.ent.oidc_start(self.a, &self.slug).await.expect("start");
        self.transport.arm_nonce(&param(&url, "nonce"));
        self.transport.arm_code_challenge(&param(&url, "code_challenge"));
        self.ent
            .oidc_complete("code", &param(&url, "state"), &lien)
            .await
    }

    async fn compte(&self, etab: Uuid, role: Role) -> (Uuid, String) {
        let email = format!("victime-{}@ent.local", Uuid::new_v4());
        let id = self
            .ent
            .register_user(&email, MDP, Some("Victime"), etab, role.as_str())
            .await
            .expect("compte")
            .id;
        (id, email)
    }

    async fn ajouter_appartenance(&self, user: Uuid, etab: Uuid, role: Role) {
        sqlx::query(
            "INSERT INTO ent_membership (user_id, etablissement_id, role, created_at)
             VALUES ($1, $2, $3, now())",
        )
        .bind(user)
        .bind(etab)
        .bind(role.as_str())
        .execute(self.ent.pool())
        .await
        .expect("appartenance");
    }

    async fn rattachements(&self, user: Uuid) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM ent_oidc_identity WHERE user_id = $1")
            .bind(user)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    async fn sessions(&self, user: Uuid) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM ent_session WHERE user_id = $1")
            .bind(user)
            .fetch_one(self.ent.pool())
            .await
            .unwrap()
    }

    /// La direction de A rend la passkey obligatoire pour les professeurs.
    async fn passkey_obligatoire_profs(&self) {
        self.ent
            .definir_politique_securite_for_actor(
                self.a,
                self.direction_a,
                Role::Direction.as_str(),
                &[Role::Prof.as_str().to_string()],
            )
            .await
            .expect("politique passkey");
    }

    /// Enrôle une passkey de test (le titulaire, par son compte).
    async fn enroler(&self, etab: Uuid, user: Uuid) {
        let mut dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
        let (token, options) = self
            .ent
            .passkey_register_start_for_actor(etab, user)
            .await
            .expect("start enrôlement");
        let ccr: CreationChallengeResponse = serde_json::from_value(options).unwrap();
        let reg = dev.do_registration(origine(), ccr).unwrap();
        self.ent
            .passkey_register_finish_for_actor(etab, user, &token, &reg, Some("Clé".into()))
            .await
            .expect("enrôlement");
    }

    /// Tente l'enrôlement depuis une session d'enrôlement SSO, avec ou sans mot
    /// de passe ; renvoie le résultat de l'étape finale.
    async fn enroler_depuis_session(
        &self,
        session: &crate::auth_repo::SessionInfo,
        mot_de_passe: &str,
    ) -> Result<crate::passkey_repo::PasskeyRow, EntDbError> {
        let mut dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
        let (token, options) = self
            .ent
            .passkey_register_start_for_actor(session.etablissement_id, session.user_id)
            .await
            .expect("start enrôlement");
        let ccr: CreationChallengeResponse = serde_json::from_value(options).unwrap();
        let reg = dev.do_registration(origine(), ccr).unwrap();
        self.ent
            .passkey_register_finish_session(
                session.etablissement_id,
                session.user_id,
                session.session_id,
                session.portee,
                session.assurance,
                mot_de_passe,
                &token,
                &reg,
                Some("Clé de l'attaquant".into()),
            )
            .await
    }
}

fn origine() -> url::Url {
    url::Url::parse("http://localhost:3000").unwrap()
}

fn session_enrolement(r: Result<LoginOutcome, EntDbError>) -> crate::auth_repo::SessionInfo {
    match r.expect("connexion SSO") {
        LoginOutcome::Session(s) => {
            assert_eq!(s.portee, PorteeSession::Enrolement, "session d'enrôlement attendue");
            assert_eq!(s.assurance, AssuranceSession::Sso);
            s
        }
        autre => panic!("session attendue : {autre:?}"),
    }
}

/// Scénario du rapport : la victime est membre de A et de B, sans passkey ; la
/// direction de A rend la passkey obligatoire et son fournisseur affirme
/// l'adresse de la victime. Aucun rattachement, aucune session.
#[tokio::test]
async fn s01_fournisseur_de_a_ne_rattache_pas_une_identite_aussi_dans_b() {
    let Some(m) = monde().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    m.passkey_obligatoire_profs().await;
    let (victime, email) = m.compte(m.b, Role::Prof).await;
    m.ajouter_appartenance(victime, m.a, Role::Prof).await;

    let r = m.sso_affirme(&email).await;
    assert!(matches!(r, Err(EntDbError::OidcCompteInconnu)), "{r:?}");
    assert_eq!(m.rattachements(victime).await, 0, "aucun rattachement écrit");
    assert_eq!(m.sessions(victime).await, 0, "aucune session ouverte");
    // La victime n'a toujours aucune passkey : rien n'ouvre B à l'attaquant.
    assert!(m.ent.passkey_login_start(m.b, &email).await.unwrap().is_none());

    // Appartenance fermée dans B : compte aussi (prudence).
    let (fermee, email_f) = m.compte(m.b, Role::Prof).await;
    m.ajouter_appartenance(fermee, m.a, Role::Prof).await;
    sqlx::query("UPDATE ent_membership SET etat = 'ferme' WHERE user_id = $1 AND etablissement_id = $2")
        .bind(fermee)
        .bind(m.b)
        .execute(m.ent.pool())
        .await
        .ok();
    assert!(matches!(
        m.sso_affirme(&email_f).await,
        Err(EntDbError::OidcCompteInconnu)
    ));
    assert_eq!(m.rattachements(fermee).await, 0);
}

/// Une identité qui a déjà une passkey a ses propres secrets : pas de rattachement
/// automatique, même si elle n'appartient qu'à A.
#[tokio::test]
async fn s01_pas_de_rattachement_par_email_d_un_compte_a_passkey() {
    let Some(m) = monde().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (victime, email) = m.compte(m.a, Role::Prof).await;
    m.enroler(m.a, victime).await;
    let r = m.sso_affirme(&email).await;
    assert!(matches!(r, Err(EntDbError::OidcCompteInconnu)), "{r:?}");
    assert_eq!(m.rattachements(victime).await, 0);
}

/// Audit S-01 (d) : l'appartenance est contrôlée AVANT d'écrire le rattachement.
/// Une identité qui n'est pas membre de A n'est jamais rattachée au fournisseur de A.
#[tokio::test]
async fn s01_non_membre_de_a_jamais_rattache() {
    let Some(m) = monde().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    let (victime, email) = m.compte(m.b, Role::Prof).await;
    let r = m.sso_affirme(&email).await;
    assert!(matches!(r, Err(EntDbError::OidcCompteInconnu)), "{r:?}");
    assert_eq!(m.rattachements(victime).await, 0, "aucun rattachement fantôme");
}

/// Non-régression : un professeur créé par A, seulement dans A, sans passkey, se
/// connecte par le SSO de A et enrôle sa passkey sans mot de passe.
#[tokio::test]
async fn s01_compte_propre_a_a_rattache_et_enrole_sans_mot_de_passe() {
    let Some(m) = monde().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    m.passkey_obligatoire_profs().await;
    let (prof, email) = m.compte(m.a, Role::Prof).await;
    let session = session_enrolement(m.sso_affirme(&email).await);
    assert_eq!(session.user_id, prof);
    assert_eq!(session.etablissement_id, m.a);
    assert_eq!(m.rattachements(prof).await, 1);
    m.enroler_depuis_session(&session, "")
        .await
        .expect("enrôlement SSO d'un compte propre à A");
    assert_eq!(m.ent.count_passkeys_for_user(prof).await.unwrap(), 1);
}

/// Rattachement antérieur au correctif vers une identité de A et de B : la
/// connexion SSO reste limitée à A, et la passkey de l'attaquant n'est plus
/// acceptée sans le mot de passe de la victime.
#[tokio::test]
async fn s01_rattachement_existant_n_enrole_pas_de_passkey_sans_mot_de_passe() {
    let Some(m) = monde().await else {
        return eprintln!("skip: ENT_DATABASE_URL");
    };
    m.passkey_obligatoire_profs().await;
    let (victime, email) = m.compte(m.b, Role::Prof).await;
    m.ajouter_appartenance(victime, m.a, Role::Prof).await;
    let sub = format!("sub-ancien-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO ent_oidc_identity (id, provider_id, subject, user_id) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(m.provider_id)
    .bind(&sub)
    .bind(victime)
    .execute(m.ent.pool())
    .await
    .expect("rattachement antérieur");
    m.transport.set_subject(&sub);
    m.transport.set_email(&email);

    let session = session_enrolement(m.sso_courant().await);
    assert_eq!(session.etablissement_id, m.a, "session dans A seulement");

    let r = m.enroler_depuis_session(&session, "").await;
    assert!(matches!(r, Err(EntDbError::MotDePasseActuelInvalide)), "{r:?}");
    assert_eq!(m.ent.count_passkeys_for_user(victime).await.unwrap(), 0);
    assert!(
        m.ent.passkey_login_start(m.b, &email).await.unwrap().is_none(),
        "aucune passkey utilisable dans B"
    );

    // Le titulaire, lui, enrôle avec son mot de passe (non-régression).
    m.enroler_depuis_session(&session, MDP)
        .await
        .expect("enrôlement par le titulaire");
    assert_eq!(m.ent.count_passkeys_for_user(victime).await.unwrap(), 1);
}
