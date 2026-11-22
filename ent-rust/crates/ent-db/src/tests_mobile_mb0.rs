//! Socle 11, lot MB0 : correctifs de sécurité mobile (DS1 à DS5, MEN-11), base locale de test
//! (feature `labo`). Les durées sont comparées à des écarts relatifs à l'heure de la session
//! (jamais à une date calendaire) : aucun jour UTC ni heure réelle codés en dur.

use super::*;
use crate::auth_repo::PgAuthRepo;
use chrono::Duration;
use ent_auth::Role;
use uuid::Uuid;

const MDP: &str = "Mobile-Mb0-Pass-1!";

async fn ent() -> Option<PgEnt> {
    let url = crate::url_guard::url_labo_ou_echec()?;
    assert_local_postgres_url(&url).expect("ENT_DATABASE_URL doit être locale");
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    Some(ent)
}

async fn compte(ent: &PgEnt, etab: Uuid) -> (String, Uuid) {
    let email = format!("mb0-{}@ent.local", Uuid::new_v4().simple());
    let u = ent
        .register_user(&email, MDP, None, etab, "prof")
        .await
        .expect("inscription");
    (email, u.id)
}

async fn connexion(ent: &PgEnt, email: &str, etab: Uuid, client: ClientSession) -> SessionInfo {
    let ip = {
        let b = Uuid::new_v4().into_bytes();
        format!("10.{}.{}.{}", b[0], b[1], b[2])
    };
    match ent
        .login_start_client(email, MDP, etab, Some(&ip), client, Some("test".into()))
        .await
        .expect("connexion")
    {
        LoginOutcome::Session(s) => s,
        autre => panic!("session attendue : {autre:?}"),
    }
}

/// Émet une session « passkey » directement (le parcours WebAuthn n'est pas l'objet du lot).
async fn session_passkey(
    ent: &PgEnt,
    user: Uuid,
    email: &str,
    etab: Uuid,
    client: ClientSession,
) -> SessionInfo {
    PgAuthRepo::new(ent.pool().clone())
        .issue_session(
            user,
            etab,
            email,
            Role::Prof.as_str(),
            &OptionsSession {
                client,
                assurance: AssuranceSession::Passkey,
                portee: PorteeSession::Complete,
                appareil: None,
                ip: None,
            },
        )
        .await
        .expect("session passkey")
}

#[test]
fn client_effectif_exige_une_passkey() {
    assert_eq!(
        client_effectif(ClientSession::Mobile, AssuranceSession::MotDePasse),
        ClientSession::Web
    );
    assert_eq!(
        client_effectif(ClientSession::Mobile, AssuranceSession::Sso),
        ClientSession::Web
    );
    assert_eq!(
        client_effectif(ClientSession::Mobile, AssuranceSession::Passkey),
        ClientSession::Mobile
    );
    assert_eq!(
        client_effectif(ClientSession::Web, AssuranceSession::Passkey),
        ClientSession::Web
    );
}

/// MEN-11 : « mobile » + mot de passe = durée standard du web, jamais 90 jours.
#[tokio::test]
async fn mobile_declare_sans_passkey_recoit_la_duree_web() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, _) = compte(&ent, etab).await;
    let s = connexion(&ent, &email, etab, ClientSession::Mobile).await;
    assert_eq!(s.client, ClientSession::Web);
    let duree = s.expires_at - chrono::Utc::now();
    assert!(duree <= Duration::hours(13), "durée trop longue : {duree}");
    // Même chose côté base : le client stocké est le client effectif.
    let (client, inactivite): (String, Option<i32>) =
        sqlx::query_as("SELECT client, inactivite_max_secs FROM ent_session WHERE id = $1")
            .bind(s.session_id)
            .fetch_one(ent.pool())
            .await
            .expect("lecture");
    assert_eq!(client, "web");
    assert!(
        inactivite.is_some(),
        "une session web garde son délai d'inactivité"
    );
}

/// Aucune régression : le client web et la session mobile passkey gardent leurs durées.
#[tokio::test]
async fn clients_web_et_mobile_passkey_inchanges() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let web = connexion(&ent, &email, etab, ClientSession::Web).await;
    assert_eq!(web.client, ClientSession::Web);
    assert!(web.expires_at - chrono::Utc::now() <= Duration::hours(13));
    let mobile = session_passkey(&ent, user, &email, etab, ClientSession::Mobile).await;
    assert_eq!(mobile.client, ClientSession::Mobile);
    assert!(mobile.expires_at - chrono::Utc::now() > Duration::days(80));
    // Passkey mais client web : durée web.
    let web_pk = session_passkey(&ent, user, &email, etab, ClientSession::Web).await;
    assert!(web_pk.expires_at - chrono::Utc::now() <= Duration::hours(13));
}

/// La prolongation glissante ne s'applique pas à une session ouverte par mot de passe, même si
/// la base porte « mobile » (session antérieure au correctif).
#[tokio::test]
async fn prolongation_glissante_refusee_sans_passkey() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, _) = compte(&ent, etab).await;
    let s = connexion(&ent, &email, etab, ClientSession::Web).await;
    let avant = s.expires_at;
    sqlx::query(
        "UPDATE ent_session SET client = 'mobile', last_seen_at = now() - interval '2 minutes' WHERE id = $1",
    )
    .bind(s.session_id)
    .execute(ent.pool())
    .await
    .expect("forge");
    let relue = ent
        .resolve_session(&format!("Bearer {}", s.token))
        .await
        .expect("résolution");
    assert!(
        relue.expires_at <= avant + Duration::seconds(1),
        "prolongation indue : {} > {}",
        relue.expires_at,
        avant
    );
}

async fn inscrire(ent: &PgEnt, etab: Uuid, user: Uuid, token: &str, session: Uuid) -> PushDevice {
    ent.register_push_device_for_actor(etab, user, "android", token, None, session)
        .await
        .expect("inscription push")
}

async fn actifs(ent: &PgEnt, etab: Uuid, user: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM ent_push_device WHERE etablissement_id = $1 AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(etab)
    .bind(user)
    .fetch_one(ent.pool())
    .await
    .expect("compte")
}

/// DS1 : révoquer la session coupe la notification de l'appareil, dans la même transaction.
#[tokio::test]
async fn revocation_de_session_coupe_les_notifications() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let s = connexion(&ent, &email, etab, ClientSession::Web).await;
    let d = inscrire(
        &ent,
        etab,
        user,
        &format!("tok-{}", Uuid::new_v4()),
        s.session_id,
    )
    .await;
    assert_eq!(actifs(&ent, etab, user).await, 1);
    ent.logout(&format!("Bearer {}", s.token))
        .await
        .expect("déconnexion");
    assert_eq!(actifs(&ent, etab, user).await, 0);
    let motif: Option<String> =
        sqlx::query_scalar("SELECT revoque_motif FROM ent_push_device WHERE id = $1")
            .bind(d.id)
            .fetch_one(ent.pool())
            .await
            .expect("motif");
    assert_eq!(motif.as_deref(), Some("session_revoquee"));
}

/// DS1 : la révocation globale (appareil perdu) coupe aussi tous les appareils de la personne.
#[tokio::test]
async fn revocation_globale_coupe_tous_les_appareils() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let s1 = connexion(&ent, &email, etab, ClientSession::Web).await;
    let s2 = connexion(&ent, &email, etab, ClientSession::Web).await;
    inscrire(
        &ent,
        etab,
        user,
        &format!("tok-{}", Uuid::new_v4()),
        s1.session_id,
    )
    .await;
    inscrire(
        &ent,
        etab,
        user,
        &format!("tok-{}", Uuid::new_v4()),
        s2.session_id,
    )
    .await;
    assert_eq!(actifs(&ent, etab, user).await, 2);
    sqlx::query("UPDATE ent_session SET revoked_at = now() WHERE user_id = $1")
        .bind(user)
        .execute(ent.pool())
        .await
        .expect("révocation");
    assert_eq!(actifs(&ent, etab, user).await, 0);
}

/// DS3 : un jeton inscrit par B pour le téléphone de A révoque l'inscription de A (avec trace) ;
/// A n'est plus joignable sur ce jeton, B l'est.
#[tokio::test]
async fn jeton_reaffecte_revoque_l_ancien_titulaire() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email_a, a) = compte(&ent, etab).await;
    let (email_b, b) = compte(&ent, etab).await;
    let sa = connexion(&ent, &email_a, etab, ClientSession::Web).await;
    let sb = connexion(&ent, &email_b, etab, ClientSession::Web).await;
    let jeton = format!("tok-{}", Uuid::new_v4());
    inscrire(&ent, etab, a, &jeton, sa.session_id).await;
    inscrire(&ent, etab, b, &jeton, sb.session_id).await;
    assert_eq!(actifs(&ent, etab, a).await, 0);
    assert_eq!(actifs(&ent, etab, b).await, 1);
    let motif: Option<String> = sqlx::query_scalar(
        "SELECT revoque_motif FROM ent_push_device WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(a)
    .fetch_one(ent.pool())
    .await
    .expect("motif");
    assert_eq!(motif.as_deref(), Some("reaffecte"));
    // L'empreinte est posée ; ré-inscrire le même jeton par la même session ne duplique rien.
    inscrire(&ent, etab, b, &jeton, sb.session_id).await;
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM ent_push_device WHERE etablissement_id = $1 AND token_empreinte IS NOT NULL")
        .bind(etab)
        .fetch_one(ent.pool())
        .await
        .expect("compte");
    assert_eq!(n, 2);
}

/// DS4 : désinscription propre ; refusée pour l'appareil d'une autre personne.
#[tokio::test]
async fn desinscription_reservee_au_proprietaire() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email_a, a) = compte(&ent, etab).await;
    let (_, b) = compte(&ent, etab).await;
    let sa = connexion(&ent, &email_a, etab, ClientSession::Web).await;
    let d = inscrire(
        &ent,
        etab,
        a,
        &format!("tok-{}", Uuid::new_v4()),
        sa.session_id,
    )
    .await;
    assert!(!ent
        .desinscrire_push_device(etab, b, d.id)
        .await
        .expect("autre"));
    assert_eq!(actifs(&ent, etab, a).await, 1);
    assert!(ent
        .desinscrire_push_device(etab, a, d.id)
        .await
        .expect("propriétaire"));
    assert_eq!(actifs(&ent, etab, a).await, 0);
    assert!(!ent
        .desinscrire_push_device(etab, a, d.id)
        .await
        .expect("déjà fait"));
}

/// DS5 / F5 : le registre exporte les métadonnées de l'appareil sans le jeton ni l'empreinte, et purge à la restauration.
#[test]
fn registre_jetons_non_exportes_et_purges() {
    let t = crate::registre_donnees::REGISTRE
        .iter()
        .find(|t| t.nom == "ent_push_device")
        .expect("table au registre");
    assert!(
        t.export_portabilite,
        "la personne a accès aux métadonnées de ses appareils"
    );
    assert!(t.colonnes_hors_export.contains(&"token"));
    assert!(t.colonnes_hors_export.contains(&"token_empreinte"));
    assert!(matches!(
        t.neutralisation,
        crate::registre_donnees::Neutralisation::Purge
    ));
}

// ---- Correctifs de revue (E1, E2, M1 à M5, F2 à F5) ----------------------------------------------

fn repo(ent: &PgEnt) -> crate::push_repo::PgPushRepo {
    crate::push_repo::PgPushRepo::new(ent.pool().clone())
}

fn jeton() -> String {
    format!("tok-{}", Uuid::new_v4())
}

/// Appareils retenus pour l'envoi (fonction SQL unique `ent_push_actif`, règle 1).
async fn eligibles(ent: &PgEnt, etab: Uuid, user: Uuid) -> usize {
    repo(&ent)
        .list_active_for_users(etab, &[user])
        .await
        .expect("liste")
        .len()
}

/// Même règle, relue par les autres chemins d'envoi (contacts du destinataire, responsables, campagnes).
async fn push_actif_sql(ent: &PgEnt, etab: Uuid, user: Uuid) -> bool {
    sqlx::query_scalar(
        "SELECT count(*) > 0 FROM ent_push_device
          WHERE etablissement_id = $1 AND user_id = $2 AND ent_push_actif(revoked_at, session_id, last_seen_at)",
    )
    .bind(etab)
    .bind(user)
    .fetch_one(ent.pool())
    .await
    .expect("ent_push_actif")
}

/// E1 : un appareil lié à une session expirée n'est plus éligible, même sans révocation explicite.
#[tokio::test]
async fn appareil_d_une_session_expiree_n_est_plus_eligible() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let s = connexion(&ent, &email, etab, ClientSession::Web).await;
    inscrire(&ent, etab, user, &jeton(), s.session_id).await;
    assert_eq!(eligibles(&ent, etab, user).await, 1);
    assert!(push_actif_sql(&ent, etab, user).await);
    sqlx::query("UPDATE ent_session SET expires_at = now() - interval '1 minute' WHERE id = $1")
        .bind(s.session_id)
        .execute(ent.pool())
        .await
        .expect("expiration");
    assert_eq!(eligibles(&ent, etab, user).await, 0);
    assert!(!push_actif_sql(&ent, etab, user).await);
}

/// E1 : session web inactive (délai de 2 h dépassé) : plus d'envoi.
#[tokio::test]
async fn appareil_d_une_session_inactive_n_est_plus_eligible() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let s = connexion(&ent, &email, etab, ClientSession::Web).await;
    inscrire(&ent, etab, user, &jeton(), s.session_id).await;
    sqlx::query("UPDATE ent_session SET last_seen_at = now() - interval '3 hours' WHERE id = $1")
        .bind(s.session_id)
        .execute(ent.pool())
        .await
        .expect("inactivité");
    assert_eq!(eligibles(&ent, etab, user).await, 0);
    assert!(!push_actif_sql(&ent, etab, user).await);
    // Activité reprise : l'appareil redevient éligible (la session, elle, n'a pas été close).
    sqlx::query("UPDATE ent_session SET last_seen_at = now() WHERE id = $1")
        .bind(s.session_id)
        .execute(ent.pool())
        .await
        .expect("activité");
    assert_eq!(eligibles(&ent, etab, user).await, 1);
}

/// E1 : la purge des sessions expirées révoque d'abord les appareils (motif `expire`, trace gardée),
/// puis détache la session (SET NULL) : plus d'effacement silencieux.
#[tokio::test]
async fn purge_des_sessions_expirees_garde_la_trace_de_l_appareil() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let s = connexion(&ent, &email, etab, ClientSession::Web).await;
    let d = inscrire(&ent, etab, user, &jeton(), s.session_id).await;
    sqlx::query("UPDATE ent_session SET expires_at = now() - interval '1 day' WHERE id = $1")
        .bind(s.session_id)
        .execute(ent.pool())
        .await
        .expect("expiration");
    assert!(ent.purge_expired_sessions().await.expect("purge") >= 1);
    let (revoque, motif, session): (bool, Option<String>, Option<Uuid>) = sqlx::query_as(
        "SELECT revoked_at IS NOT NULL, revoque_motif, session_id FROM ent_push_device WHERE id = $1",
    )
    .bind(d.id)
    .fetch_one(ent.pool())
    .await
    .expect("l'appareil doit rester en base (trace)");
    assert!(revoque);
    assert_eq!(motif.as_deref(), Some("expire"));
    assert_eq!(session, None);
}

/// M1 : inscription avec une session révoquée (course inscription / révocation) ou expirée : refus 401.
#[tokio::test]
async fn inscription_avec_une_session_morte_est_refusee() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let revoquee = connexion(&ent, &email, etab, ClientSession::Web).await;
    ent.logout(&format!("Bearer {}", revoquee.token))
        .await
        .expect("déconnexion");
    let r = ent
        .register_push_device_for_actor(etab, user, "android", &jeton(), None, revoquee.session_id)
        .await;
    assert!(
        matches!(
            r,
            Err(EntDbError::Auth(ent_auth::AuthError::InvalidSession))
        ),
        "{r:?}"
    );
    let expiree = connexion(&ent, &email, etab, ClientSession::Web).await;
    sqlx::query("UPDATE ent_session SET expires_at = now() - interval '1 minute' WHERE id = $1")
        .bind(expiree.session_id)
        .execute(ent.pool())
        .await
        .expect("expiration");
    let r = ent
        .register_push_device_for_actor(etab, user, "android", &jeton(), None, expiree.session_id)
        .await;
    assert!(
        matches!(
            r,
            Err(EntDbError::Auth(ent_auth::AuthError::InvalidSession))
        ),
        "{r:?}"
    );
    // Session d'une autre personne : refus aussi (la session doit appartenir à l'acteur).
    let (email_b, b) = compte(&ent, etab).await;
    let sb = connexion(&ent, &email_b, etab, ClientSession::Web).await;
    let r = ent
        .register_push_device_for_actor(etab, user, "android", &jeton(), None, sb.session_id)
        .await;
    assert!(
        matches!(
            r,
            Err(EntDbError::Auth(ent_auth::AuthError::InvalidSession))
        ),
        "{r:?}"
    );
    assert_eq!(
        actifs(&ent, etab, user).await + actifs(&ent, etab, b).await,
        0
    );
}

/// M2 (règle 2) : un jeton physique n'a qu'un titulaire, tous établissements confondus ; une même
/// personne peut l'avoir dans plusieurs établissements.
#[tokio::test]
async fn un_jeton_n_a_qu_un_titulaire_entre_etablissements() {
    let Some(ent) = ent().await else { return };
    let (etab1, etab2) = (Uuid::new_v4(), Uuid::new_v4());
    let (email_a, a) = compte(&ent, etab1).await;
    let (email_b, b) = compte(&ent, etab2).await;
    let sa = connexion(&ent, &email_a, etab1, ClientSession::Web).await;
    let sb = connexion(&ent, &email_b, etab2, ClientSession::Web).await;
    let t = jeton();
    inscrire(&ent, etab1, a, &t, sa.session_id).await;
    inscrire(&ent, etab2, b, &t, sb.session_id).await;
    assert_eq!(
        actifs(&ent, etab1, a).await,
        0,
        "A (autre établissement) n'est plus titulaire"
    );
    assert_eq!(actifs(&ent, etab2, b).await, 1);
    let motif: Option<String> = sqlx::query_scalar(
        "SELECT revoque_motif FROM ent_push_device WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab1)
    .bind(a)
    .fetch_one(ent.pool())
    .await
    .expect("motif");
    assert_eq!(motif.as_deref(), Some("reaffecte"));
    // Le jeton n'existe en clair que sur la ligne active : un seul titulaire actif au total.
    let n: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT user_id) FROM ent_push_device WHERE token_empreinte = $1 AND revoked_at IS NULL",
    )
    .bind(ent_auth::hash_token(&t))
    .fetch_one(ent.pool())
    .await
    .expect("titulaires");
    assert_eq!(n, 1);
    // Même personne dans deux établissements (inscriptions sans session : pas de lien de membre requis) :
    // aucune révocation.
    let t2 = jeton();
    ent.register_push_device_sans_session(etab1, a, "ios", &t2, None)
        .await
        .expect("etab1");
    ent.register_push_device_sans_session(etab2, a, "ios", &t2, None)
        .await
        .expect("etab2");
    assert_eq!(actifs(&ent, etab1, a).await, 1);
    assert_eq!(actifs(&ent, etab2, a).await, 1);
}

/// M3 : même personne, nouvelle session, même jeton : `session_id` est mis à jour sur la ligne
/// existante, sans révocation ; `reaffecte_depuis` ne contient que d'autres personnes.
#[tokio::test]
async fn meme_personne_nouvelle_session_meme_jeton_met_a_jour_la_ligne() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let s1 = connexion(&ent, &email, etab, ClientSession::Web).await;
    let s2 = connexion(&ent, &email, etab, ClientSession::Web).await;
    let t = jeton();
    let d1 = inscrire(&ent, etab, user, &t, s1.session_id).await;
    let (d2, anciens) = repo(&ent)
        .upsert_device(crate::push_repo::RegisterPushDevice {
            etablissement_id: etab,
            user_id: user,
            platform: ent_push::PushPlatform::Android,
            token: t.clone(),
            app_id: None,
            session_id: Some(s2.session_id),
        })
        .await
        .expect("réinscription");
    assert_eq!(d1.id, d2.id, "même ligne");
    assert!(anciens.is_empty(), "aucun autre titulaire : {anciens:?}");
    let (n, session, revoque): (i64, Option<Uuid>, bool) = sqlx::query_as(
        "SELECT count(*) OVER (), session_id, revoked_at IS NOT NULL FROM ent_push_device
          WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(user)
    .fetch_one(ent.pool())
    .await
    .expect("ligne");
    assert_eq!((n, session, revoque), (1, Some(s2.session_id), false));
    // L'ancienne session n'a plus d'effet sur l'appareil ; la nouvelle oui.
    ent.logout(&format!("Bearer {}", s1.token))
        .await
        .expect("déconnexion");
    assert_eq!(actifs(&ent, etab, user).await, 1);
    ent.logout(&format!("Bearer {}", s2.token))
        .await
        .expect("déconnexion");
    assert_eq!(actifs(&ent, etab, user).await, 0);
}

/// F5 : deux inscriptions simultanées du même jeton ne donnent jamais d'erreur ni de doublon.
#[tokio::test]
async fn inscriptions_simultanees_du_meme_jeton_sans_erreur() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let s = connexion(&ent, &email, etab, ClientSession::Web).await;
    let t = jeton();
    let (r1, r2, r3) = tokio::join!(
        ent.register_push_device_for_actor(etab, user, "android", &t, None, s.session_id),
        ent.register_push_device_for_actor(etab, user, "android", &t, None, s.session_id),
        ent.register_push_device_for_actor(etab, user, "android", &t, None, s.session_id),
    );
    let (d1, d2, d3) = (r1.expect("1"), r2.expect("2"), r3.expect("3"));
    assert_eq!((d1.id, d1.id), (d2.id, d3.id));
    assert_eq!(actifs(&ent, etab, user).await, 1);
}

/// M4 (règle 3) : l'historique d'envoi ne contient jamais le jeton (empreinte et référence seulement) ;
/// la relecture au moment de l'envoi ne rend rien si l'inscription a été révoquée entre-temps.
#[tokio::test]
async fn l_historique_d_envoi_ne_garde_pas_le_jeton() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let s = connexion(&ent, &email, etab, ClientSession::Web).await;
    let t = jeton();
    let d = inscrire(&ent, etab, user, &t, s.session_id).await;
    assert_eq!(
        repo(&ent)
            .relire_jeton(etab, d.id)
            .await
            .expect("relecture")
            .as_deref(),
        Some(t.as_str())
    );
    repo(&ent)
        .insert_outbox(crate::push_repo::InsertPushOutbox {
            etablissement_id: etab,
            eleve_id: None,
            user_id: Some(user),
            kind: "test".into(),
            title: "t".into(),
            body: "b".into(),
            platform: Some("android".into()),
            device_id: Some(d.id),
            status: ent_push::PushSendStatus::DryRun,
            error: None,
            source_aggregate: None,
            source_id: None,
            message_id: None,
        })
        .await
        .expect("outbox");
    let (token, empreinte, device): (Option<String>, Option<String>, Option<Uuid>) = sqlx::query_as(
        "SELECT token, token_empreinte, device_id FROM ent_push_outbox WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(user)
    .fetch_one(ent.pool())
    .await
    .expect("ligne");
    assert_eq!(token, None, "jamais le jeton en clair dans l'historique");
    assert_eq!(
        empreinte.as_deref(),
        Some(ent_auth::hash_token(&t).as_str())
    );
    assert_eq!(device, Some(d.id));
    // Garde-fou en base : une écriture qui tenterait d'y remettre le jeton est refusée.
    let r = sqlx::query(
        "INSERT INTO ent_push_outbox (id, etablissement_id, kind, title, body, token, status)
         VALUES ($1, $2, 'test', 't', 'b', $3, 'dry_run')",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(&t)
    .execute(ent.pool())
    .await;
    assert!(
        r.is_err(),
        "le jeton en clair ne doit plus entrer dans ent_push_outbox"
    );
    // Révoquée entre la sélection et l'envoi : plus de jeton, donc plus d'envoi.
    ent.desinscrire_push_device(etab, user, d.id)
        .await
        .expect("désinscription");
    assert_eq!(
        repo(&ent)
            .relire_jeton(etab, d.id)
            .await
            .expect("relecture"),
        None
    );
}

/// M5 / F2 : `passkey_login_finish` avec un client « mobile » est le seul chemin à durée mobile ;
/// la lecture (`resolve_session`) renvoie le client effectif, jamais le déclaré.
#[tokio::test]
async fn passkey_mobile_donne_la_duree_mobile_et_la_lecture_la_confirme() {
    use ent_auth::{CreationChallengeResponse, RequestChallengeResponse};
    use webauthn_authenticator_rs::prelude::WebauthnAuthenticator;
    use webauthn_authenticator_rs::softpasskey::SoftPasskey;
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let origine = url::Url::parse("http://localhost:3000").expect("origine");
    let mut dev = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let (reg_token, options) = ent
        .passkey_register_start_for_actor(etab, user)
        .await
        .expect("start");
    let ccr: CreationChallengeResponse = serde_json::from_value(options).expect("options");
    let reg = dev
        .do_registration(origine.clone(), ccr)
        .expect("enregistrement");
    ent.passkey_register_finish_for_actor(etab, user, &reg_token, &reg, Some("Clé".into()))
        .await
        .expect("enrôlement");
    for (declare, mobile_attendu) in [(ClientSession::Mobile, true), (ClientSession::Web, false)] {
        let (token, options, _) = ent
            .passkey_login_start_client(etab, &email, declare)
            .await
            .expect("start login")
            .expect("passkeys présentes");
        let rcr: RequestChallengeResponse = serde_json::from_value(options).expect("options");
        let assertion = dev
            .do_authentication(origine.clone(), rcr)
            .expect("assertion");
        let s = ent
            .passkey_login_finish(&token, &assertion)
            .await
            .expect("session passkey");
        assert_eq!(s.assurance, AssuranceSession::Passkey);
        assert_eq!(s.client == ClientSession::Mobile, mobile_attendu);
        let duree = s.expires_at - chrono::Utc::now();
        assert_eq!(duree > Duration::days(80), mobile_attendu, "durée {duree}");
        let relue = ent
            .resolve_session(&format!("Bearer {}", s.token))
            .await
            .expect("lecture");
        assert_eq!(relue.client == ClientSession::Mobile, mobile_attendu);
    }
    // F2 : une ligne « mobile » sans passkey (antérieure au correctif) est lue comme « web ».
    let web = connexion(&ent, &compte(&ent, etab).await.0, etab, ClientSession::Web).await;
    sqlx::query("UPDATE ent_session SET client = 'mobile' WHERE id = $1")
        .bind(web.session_id)
        .execute(ent.pool())
        .await
        .expect("forge");
    let relue = ent
        .resolve_session(&format!("Bearer {}", web.token))
        .await
        .expect("lecture");
    assert_eq!(
        relue.client,
        ClientSession::Web,
        "F2 : client effectif à la lecture"
    );
}

/// M5 : `completer_session_apres_enrolement` ne donne jamais la durée mobile à une session sans passkey,
/// même si la ligne porte « mobile ».
#[tokio::test]
async fn fin_d_enrolement_ne_donne_pas_la_duree_mobile_sans_passkey() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let auth = PgAuthRepo::new(ent.pool().clone());
    let s = auth
        .issue_session(
            user,
            etab,
            &email,
            Role::Prof.as_str(),
            &OptionsSession {
                client: ClientSession::Mobile,
                assurance: AssuranceSession::MotDePasse,
                portee: PorteeSession::Enrolement,
                appareil: None,
                ip: None,
            },
        )
        .await
        .expect("session d'enrôlement");
    sqlx::query("UPDATE ent_session SET client = 'mobile' WHERE id = $1")
        .bind(s.session_id)
        .execute(ent.pool())
        .await
        .expect("forge");
    auth.completer_session_apres_enrolement(s.session_id)
        .await
        .expect("fin d'enrôlement");
    let (portee, expire, inactivite): (String, chrono::DateTime<chrono::Utc>, Option<i32>) =
        sqlx::query_as(
            "SELECT portee, expires_at, inactivite_max_secs FROM ent_session WHERE id = $1",
        )
        .bind(s.session_id)
        .fetch_one(ent.pool())
        .await
        .expect("lecture");
    assert_eq!(portee, "complete");
    assert!(
        expire - chrono::Utc::now() <= Duration::hours(13),
        "durée web attendue"
    );
    assert!(inactivite.is_some());
}

/// M5 / F3 : la migration ramène les sessions « mobile » sans passkey à la durée web (plafond compté
/// depuis la création : une session plus ancienne est close, son appareil révoqué), remplit les
/// empreintes manquantes ; son rejeu est sans effet.
#[tokio::test]
async fn migration_2220_ramene_les_sessions_mobiles_sans_passkey_et_se_rejoue_sans_effet() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let recente = connexion(&ent, &email, etab, ClientSession::Web).await;
    let ancienne = connexion(&ent, &email, etab, ClientSession::Web).await;
    let passkey_session = session_passkey(&ent, user, &email, etab, ClientSession::Mobile).await;
    // État antérieur au lot : « mobile » sans passkey, 90 jours, sans délai d'inactivité.
    for (id, age) in [
        (recente.session_id, "1 hour"),
        (ancienne.session_id, "13 hours"),
    ] {
        sqlx::query(
            "UPDATE ent_session SET client = 'mobile', inactivite_max_secs = NULL,
                    created_at = now() - $2::interval, expires_at = now() + interval '90 days' WHERE id = $1",
        )
        .bind(id)
        .bind(age)
        .execute(ent.pool())
        .await
        .expect("état antérieur");
    }
    let d_ancienne = inscrire(&ent, etab, user, &jeton(), ancienne.session_id).await;
    sqlx::query("UPDATE ent_push_device SET token_empreinte = NULL WHERE id = $1")
        .bind(d_ancienne.id)
        .execute(ent.pool())
        .await
        .expect("empreinte absente");
    let sql = include_str!("../../../migrations/2220_mob_mb0_appareils_push.sql");
    sqlx::raw_sql(sql)
        .execute(ent.pool())
        .await
        .expect("rejeu de la migration");
    let lire = |id: Uuid| {
        let pool = ent.pool().clone();
        async move {
            sqlx::query_as::<
                _,
                (
                    String,
                    chrono::DateTime<chrono::Utc>,
                    chrono::DateTime<chrono::Utc>,
                    Option<i32>,
                    bool,
                ),
            >(
                "SELECT client, expires_at, created_at, inactivite_max_secs, revoked_at IS NOT NULL
                   FROM ent_session WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("lecture")
        }
    };
    let (client, expire, cree, inactivite, close) = lire(recente.session_id).await;
    assert_eq!(client, "web");
    assert!(
        expire <= cree + Duration::hours(12) + Duration::seconds(1),
        "plafond depuis la création"
    );
    assert_eq!(inactivite, Some(7200));
    assert!(!close);
    let (_, _, _, _, close) = lire(ancienne.session_id).await;
    assert!(close, "session déjà au-delà du plafond : close");
    assert_eq!(
        actifs(&ent, etab, user).await,
        0,
        "l'appareil de la session close est révoqué"
    );
    let empreinte: Option<String> =
        sqlx::query_scalar("SELECT token_empreinte FROM ent_push_device WHERE id = $1")
            .bind(d_ancienne.id)
            .fetch_one(ent.pool())
            .await
            .expect("empreinte");
    assert!(empreinte.is_some(), "empreinte remplie");
    // La session passkey n'est pas touchée.
    let (client, expire, _, _, close) = lire(passkey_session.session_id).await;
    assert_eq!(client, "mobile");
    assert!(expire - chrono::Utc::now() > Duration::days(80) && !close);
    // Rejeu : aucun changement.
    let avant = lire(recente.session_id).await;
    sqlx::raw_sql(sql)
        .execute(ent.pool())
        .await
        .expect("second rejeu");
    assert_eq!(lire(recente.session_id).await, avant);
}
