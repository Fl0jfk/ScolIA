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
    let duree = duree_accordee(&ent, s.session_id).await;
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
    assert!(duree_accordee(&ent, web.session_id).await <= Duration::hours(13));
    let mobile = session_passkey(&ent, user, &email, etab, ClientSession::Mobile).await;
    assert_eq!(mobile.client, ClientSession::Mobile);
    assert!(duree_accordee(&ent, mobile.session_id).await > Duration::days(80));
    // Passkey mais client web : durée web.
    let web_pk = session_passkey(&ent, user, &email, etab, ClientSession::Web).await;
    assert!(duree_accordee(&ent, web_pk.session_id).await <= Duration::hours(13));
}

/// La prolongation glissante ne s'applique pas à une session ouverte par mot de passe, même si
/// la base porte « mobile » (session antérieure au correctif).
#[tokio::test]
async fn prolongation_glissante_refusee_sans_passkey() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, _) = compte(&ent, etab).await;
    let s = connexion(&ent, &email, etab, ClientSession::Web).await;
    let avant = duree_accordee(&ent, s.session_id).await;
    sqlx::query(
        "UPDATE ent_session SET client = 'mobile', last_seen_at = now() - interval '2 minutes' WHERE id = $1",
    )
    .bind(s.session_id)
    .execute(ent.pool())
    .await
    .expect("forge");
    ent.resolve_session(&format!("Bearer {}", s.token))
        .await
        .expect("résolution");
    let apres = duree_accordee(&ent, s.session_id).await;
    assert!(
        apres <= avant + Duration::seconds(1),
        "prolongation indue : {apres} > {avant}"
    );
}

/// F-7 : durée accordée à la session (`expires_at − created_at`), calculée côté SQL : ni heure réelle ni
/// jour UTC dans le test, seulement un écart entre deux colonnes de la même ligne.
async fn duree_accordee(ent: &PgEnt, session_id: Uuid) -> Duration {
    let secs: f64 = sqlx::query_scalar(
        "SELECT extract(epoch FROM (expires_at - created_at))::float8 FROM ent_session WHERE id = $1",
    )
    .bind(session_id)
    .fetch_one(ent.pool())
    .await
    .expect("durée");
    Duration::milliseconds((secs * 1000.0) as i64)
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

/// DS5 / F5 : le registre exporte les métadonnées de l'appareil sans le jeton ni l'empreinte, et révoque tout à la restauration.
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
        crate::registre_donnees::Neutralisation::Maj { set, .. } if set.contains("revoked_at")
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
    repo(ent)
        .list_active_for_users(etab, &[user])
        .await
        .expect("liste")
        .len()
}

/// Même règle, relue par les autres chemins d'envoi (contacts du destinataire, responsables, campagnes).
async fn push_actif_sql(ent: &PgEnt, etab: Uuid, user: Uuid) -> bool {
    sqlx::query_scalar(
        "SELECT count(*) > 0 FROM ent_push_device
          WHERE etablissement_id = $1 AND user_id = $2 AND ent_push_actif(revoked_at, session_id, last_seen_at, user_id, etablissement_id)",
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
            platform: "android".to_string(),
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
        let duree = duree_accordee(&ent, s.session_id).await;
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
    let (portee, inactivite): (String, Option<i32>) =
        sqlx::query_as("SELECT portee, inactivite_max_secs FROM ent_session WHERE id = $1")
            .bind(s.session_id)
            .fetch_one(ent.pool())
            .await
            .expect("lecture");
    assert_eq!(portee, "complete");
    assert!(
        duree_accordee(&ent, s.session_id).await <= Duration::hours(13),
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
    let (client, _, _, _, close) = lire(passkey_session.session_id).await;
    assert_eq!(client, "mobile");
    assert!(duree_accordee(&ent, passkey_session.session_id).await > Duration::days(80) && !close);
    // Rejeu : aucun changement.
    let avant = lire(recente.session_id).await;
    sqlx::raw_sql(sql)
        .execute(ent.pool())
        .await
        .expect("second rejeu");
    assert_eq!(lire(recente.session_id).await, avant);
}

// ---- Passe 2 (M-A à M-D, F-1 à F-7) --------------------------------------------------------------
// Aucun test ne dépend de l'heure réelle ni d'un jour UTC : les dates d'accès sont posées par rapport
// à `ent_aujourdhui_etab` (côté SQL), les durées sont des écarts calculés par la base.

/// Revoit l'état d'un accès (`ent_membership`) : `date_fin` posée à « hier » (jour de l'établissement).
async fn membre_echu(ent: &PgEnt, etab: Uuid, user: Uuid, echu: bool) {
    sqlx::query(
        "UPDATE ent_membership
            SET date_fin = CASE WHEN $3 THEN ent_aujourdhui_etab($1) - 1 ELSE NULL END
          WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(user)
    .bind(echu)
    .execute(ent.pool())
    .await
    .expect("date_fin");
}

async fn verifier_resolution(ent: &PgEnt, s: &SessionInfo) -> bool {
    ent.resolve_session(&format!("Bearer {}", s.token))
        .await
        .is_ok()
}

/// M-A : une base qui a déjà appliqué l'ancienne 2220 porte une clé étrangère en CASCADE ; le rejeu la
/// remplace par SET NULL (la purge d'une session ne doit jamais effacer l'appareil ni sa trace), et un
/// second rejeu ne la touche plus.
#[tokio::test]
async fn migration_2220_remplace_la_cle_en_cascade_d_une_base_deja_migree() {
    let Some(ent) = ent().await else { return };
    let sql = include_str!("../../../migrations/2220_mob_mb0_appareils_push.sql");
    // État de l'ancienne version : clé en CASCADE.
    sqlx::raw_sql(
        "ALTER TABLE ent_push_device DROP CONSTRAINT IF EXISTS ent_push_device_session_id_fkey;
         ALTER TABLE ent_push_device ADD CONSTRAINT ent_push_device_session_id_fkey
             FOREIGN KEY (session_id) REFERENCES ent_session (id) ON DELETE CASCADE;",
    )
    .execute(ent.pool())
    .await
    .expect("ancienne clé");
    let action = || async {
        sqlx::query_as::<_, (String, i32)>(
            "SELECT confdeltype::text, oid::int4 FROM pg_constraint
              WHERE conname = 'ent_push_device_session_id_fkey' AND conrelid = 'ent_push_device'::regclass",
        )
        .fetch_one(ent.pool())
        .await
        .expect("clé")
    };
    assert_eq!(action().await.0, "c", "préalable : ancienne clé en CASCADE");
    sqlx::raw_sql(sql).execute(ent.pool()).await.expect("rejeu");
    let (apres, oid) = action().await;
    assert_eq!(apres, "n", "SET NULL après le rejeu");
    sqlx::raw_sql(sql)
        .execute(ent.pool())
        .await
        .expect("second rejeu");
    assert_eq!(
        action().await,
        ("n".to_string(), oid),
        "second rejeu : clé non touchée"
    );
    // Effet réel : supprimer la session garde l'appareil (révoqué, trace).
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let s = connexion(&ent, &email, etab, ClientSession::Web).await;
    let d = inscrire(&ent, etab, user, &jeton(), s.session_id).await;
    sqlx::query("DELETE FROM ent_session WHERE id = $1")
        .bind(s.session_id)
        .execute(ent.pool())
        .await
        .expect("suppression de la session");
    let (revoque, session): (bool, Option<Uuid>) = sqlx::query_as(
        "SELECT revoked_at IS NOT NULL, session_id FROM ent_push_device WHERE id = $1",
    )
    .bind(d.id)
    .fetch_one(ent.pool())
    .await
    .expect("l'appareil doit rester");
    assert!(revoque && session.is_none());
}

/// M-B (règle 2) : le jeton détenu dans un autre établissement est révoqué, avec trace dans cet
/// établissement, mais l'établissement appelant n'en apprend ni l'identité ni l'existence : ni dans la
/// réponse du dépôt, ni dans les événements, ni dans la valeur rendue par la fonction SQL.
#[tokio::test]
async fn reaffectation_entre_etablissements_ne_revele_rien_a_l_appelant() {
    let Some(ent) = ent().await else { return };
    let (etab_a, etab_b) = (Uuid::new_v4(), Uuid::new_v4());
    let (email_a, a) = compte(&ent, etab_a).await;
    let (email_b, b) = compte(&ent, etab_b).await;
    let sa = connexion(&ent, &email_a, etab_a, ClientSession::Web).await;
    let sb = connexion(&ent, &email_b, etab_b, ClientSession::Web).await;
    let t = jeton();
    let da = inscrire(&ent, etab_a, a, &t, sa.session_id).await;
    let db = inscrire(&ent, etab_b, b, &t, sb.session_id).await;
    // Trace chez A, dans l'établissement d'origine.
    let (revoque, motif): (bool, Option<String>) = sqlx::query_as(
        "SELECT revoked_at IS NOT NULL, revoque_motif FROM ent_push_device WHERE id = $1 AND etablissement_id = $2",
    )
    .bind(da.id)
    .bind(etab_a)
    .fetch_one(ent.pool())
    .await
    .expect("ligne de A conservée");
    assert!(revoque);
    assert_eq!(motif.as_deref(), Some("reaffecte"));
    // Chez B : aucun identifiant de A (ni de son établissement) dans les événements.
    let evenements: Vec<String> =
        sqlx::query_scalar("SELECT payload::text FROM ent_metier_event WHERE etablissement_id = $1 AND aggregate_id = $2")
            .bind(etab_b)
            .bind(db.id)
            .fetch_all(ent.pool())
            .await
            .expect("événements");
    assert!(!evenements.is_empty(), "l'inscription est journalisée");
    for e in &evenements {
        for interdit in [a.to_string(), etab_a.to_string(), da.id.to_string()] {
            assert!(!e.contains(&interdit), "fuite inter-établissements : {e}");
        }
        assert!(
            e.contains(r#""reaffecte_depuis": []"#),
            "aucun ancien titulaire visible : {e}"
        );
    }
    // La fonction SQL, appelée côté B, ne rend rien d'un autre établissement…
    let t2 = jeton();
    inscrire(&ent, etab_a, a, &t2, sa.session_id).await;
    let rendus: Vec<Uuid> = sqlx::query_scalar("SELECT ent_push_reaffecter($1, $2, $3)")
        .bind(ent_auth::hash_token(&t2))
        .bind(b)
        .bind(etab_b)
        .fetch_all(ent.pool())
        .await
        .expect("fonction");
    assert!(rendus.is_empty(), "rien ne traverse : {rendus:?}");
    assert_eq!(
        actifs(&ent, etab_a, a).await,
        0,
        "le jeton de A est pourtant révoqué chez A"
    );
    // …mais rend les anciens titulaires de SON établissement (contrôle positif).
    let (email_c, c) = compte(&ent, etab_b).await;
    let sc = connexion(&ent, &email_c, etab_b, ClientSession::Web).await;
    let t3 = jeton();
    inscrire(&ent, etab_b, c, &t3, sc.session_id).await;
    let (_, anciens) = repo(&ent)
        .upsert_device(crate::push_repo::RegisterPushDevice {
            etablissement_id: etab_b,
            user_id: b,
            platform: "android".to_string(),
            token: t3,
            app_id: None,
            session_id: Some(sb.session_id),
        })
        .await
        .expect("inscription de B");
    assert_eq!(
        anciens,
        vec![c],
        "l'ancien titulaire du MÊME établissement est rendu"
    );
}

/// M-C (règle 3) : les fonctions du lot n'ont pas d'exécution publique et un `search_path` réduit ;
/// un rôle sans droit ne peut pas appeler la fonction `SECURITY DEFINER`.
#[tokio::test]
async fn fonctions_du_lot_moindre_privilege() {
    let Some(ent) = ent().await else { return };
    let lignes: Vec<(String, bool, bool, Option<Vec<String>>)> = sqlx::query_as(
        "SELECT p.proname::text, p.prosecdef, has_function_privilege('public', p.oid, 'EXECUTE'), p.proconfig
           FROM pg_proc p
          WHERE p.pronamespace = 'public'::regnamespace
            AND p.proname IN ('ent_push_reaffecter', 'ent_trg_push_device_empreinte',
                              'ent_trg_session_revoque_push', 'ent_trg_session_supprimee_push')
          ORDER BY 1",
    )
    .fetch_all(ent.pool())
    .await
    .expect("catalogue");
    assert_eq!(lignes.len(), 4, "{lignes:?}");
    for (nom, definer, public, config) in &lignes {
        assert!(!public, "{nom} ne doit pas être exécutable par PUBLIC");
        assert_eq!(
            config.as_deref(),
            Some(&["search_path=pg_catalog, pg_temp".to_string()][..]),
            "{nom} : search_path réduit"
        );
        assert_eq!(
            *definer,
            nom == "ent_push_reaffecter",
            "{nom} : SECURITY DEFINER seulement pour la réaffectation"
        );
    }
    // Un rôle quelconque est refusé (42501) ; on le crée de façon idempotente.
    sqlx::raw_sql(
        "DO $$ BEGIN CREATE ROLE ent_sans_droit_mb0 NOLOGIN NOSUPERUSER;
         EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL; END $$;",
    )
    .execute(ent.pool())
    .await
    .expect("rôle");
    let mut c = ent.pool().acquire().await.expect("connexion");
    sqlx::query("SET ROLE ent_sans_droit_mb0")
        .execute(&mut *c)
        .await
        .expect("set role");
    let r = sqlx::query("SELECT ent_push_reaffecter('x', gen_random_uuid(), gen_random_uuid())")
        .fetch_all(&mut *c)
        .await;
    let code = match r {
        Err(sqlx::Error::Database(e)) => e.code().map(|c| c.to_string()),
        autre => panic!("refus attendu, obtenu : {autre:?}"),
    };
    assert_eq!(code.as_deref(), Some("42501"));
    sqlx::query("RESET ROLE")
        .execute(&mut *c)
        .await
        .expect("reset");
}

/// M-D (règle 1) : un appareil ne reçoit plus rien dès que la session ne serait plus acceptée par l'API :
/// accès échu, accès suspendu, agent suspendu, casquette parent fermée, portée réduite.
#[tokio::test]
async fn acces_echu_suspendu_ou_casquette_fermee_ne_recoit_plus_de_notification() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let s = connexion(&ent, &email, etab, ClientSession::Web).await;
    inscrire(&ent, etab, user, &jeton(), s.session_id).await;
    let sans_session = jeton();
    ent.register_push_device_sans_session(etab, user, "ios", &sans_session, None)
        .await
        .expect("appareil sans session");
    assert_eq!(eligibles(&ent, etab, user).await, 2);
    assert!(verifier_resolution(&ent, &s).await);

    // 1. Accès échu (`date_fin` passée) : l'API refuse la session, plus aucun envoi (avec ou sans session).
    membre_echu(&ent, etab, user, true).await;
    assert!(
        !verifier_resolution(&ent, &s).await,
        "préalable : l'API refuse"
    );
    assert_eq!(eligibles(&ent, etab, user).await, 0, "accès échu");
    assert!(!push_actif_sql(&ent, etab, user).await);
    membre_echu(&ent, etab, user, false).await;
    assert_eq!(eligibles(&ent, etab, user).await, 2, "accès rétabli");

    // 2. Accès suspendu.
    sqlx::query(
        "UPDATE ent_membership SET etat = 'suspendu' WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(user)
    .execute(ent.pool())
    .await
    .expect("suspension de l'accès");
    assert!(!verifier_resolution(&ent, &s).await);
    assert_eq!(eligibles(&ent, etab, user).await, 0, "accès suspendu");
    sqlx::query(
        "UPDATE ent_membership SET etat = 'ouvert' WHERE etablissement_id = $1 AND user_id = $2",
    )
    .bind(etab)
    .bind(user)
    .execute(ent.pool())
    .await
    .expect("réouverture");
    assert_eq!(eligibles(&ent, etab, user).await, 2);

    // 3. Agent suspendu (disponibilité, congé…) : suspension d'engagement qui coupe l'accès.
    let engagement: Option<Uuid> = sqlx::query_scalar(
        "SELECT e.id FROM ent_agent_engagement e
           JOIN ent_agent a ON a.etablissement_id = e.etablissement_id AND a.id = e.agent_id
           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE p.etablissement_id = $1 AND p.user_id = $2 AND e.annule_le IS NULL LIMIT 1",
    )
    .bind(etab)
    .bind(user)
    .fetch_optional(ent.pool())
    .await
    .expect("engagement");
    let engagement = engagement.expect("un compte du personnel a un agent et un engagement");
    let suspension = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_agent_engagement_suspension (id, etablissement_id, engagement_id, du, au, motif_code, acces_suspendu)
         VALUES ($1, $2, $3, ent_aujourdhui_etab($2) - 1, NULL, 'autre', true)",
    )
    .bind(suspension)
    .bind(etab)
    .bind(engagement)
    .execute(ent.pool())
    .await
    .expect("suspension d'engagement");
    assert!(
        !verifier_resolution(&ent, &s).await,
        "préalable : l'API refuse un agent suspendu"
    );
    assert_eq!(eligibles(&ent, etab, user).await, 0, "agent suspendu");
    sqlx::query("UPDATE ent_agent_engagement_suspension SET annule_le = now() WHERE id = $1")
        .bind(suspension)
        .execute(ent.pool())
        .await
        .expect("levée");
    assert_eq!(eligibles(&ent, etab, user).await, 2);

    // 4. Casquette parent : session « parent » sans casquette ouverte, puis fermée.
    sqlx::query("UPDATE ent_session SET casquette = 'parent' WHERE id = $1")
        .bind(s.session_id)
        .execute(ent.pool())
        .await
        .expect("casquette de la session");
    assert!(
        !verifier_resolution(&ent, &s).await,
        "préalable : casquette parent absente"
    );
    assert_eq!(
        eligibles(&ent, etab, user).await,
        1,
        "seul l'appareil sans session reste (la session est refusée)"
    );
    sqlx::query(
        "INSERT INTO ent_membership_casquette (user_id, etablissement_id, casquette, du, etat)
         VALUES ($2, $1, 'parent', ent_aujourdhui_etab($1) - 1, 'ouvert')",
    )
    .bind(etab)
    .bind(user)
    .execute(ent.pool())
    .await
    .expect("casquette parent");
    assert!(
        verifier_resolution(&ent, &s).await,
        "casquette parent ouverte : l'API accepte"
    );
    assert_eq!(eligibles(&ent, etab, user).await, 2);
    sqlx::query("UPDATE ent_membership_casquette SET etat = 'ferme' WHERE etablissement_id = $1 AND user_id = $2")
        .bind(etab)
        .bind(user)
        .execute(ent.pool())
        .await
        .expect("fermeture de la casquette");
    assert!(!verifier_resolution(&ent, &s).await);
    assert_eq!(
        eligibles(&ent, etab, user).await,
        1,
        "casquette parent fermée"
    );

    // 5. Portée réduite (enrôlement) : pas de notification.
    sqlx::query(
        "UPDATE ent_session SET casquette = 'principale', portee = 'enrolement' WHERE id = $1",
    )
    .bind(s.session_id)
    .execute(ent.pool())
    .await
    .expect("portée");
    assert_eq!(eligibles(&ent, etab, user).await, 1, "portée réduite");
    sqlx::query("UPDATE ent_session SET portee = 'complete' WHERE id = $1")
        .bind(s.session_id)
        .execute(ent.pool())
        .await
        .expect("portée complète");
    assert_eq!(eligibles(&ent, etab, user).await, 2);

    // Les chemins d'envoi autres que le dépôt (campagnes, contacts) lisent la même fonction : parité.
    membre_echu(&ent, etab, user, true).await;
    assert!(!push_actif_sql(&ent, etab, user).await);
    membre_echu(&ent, etab, user, false).await;
    assert!(push_actif_sql(&ent, etab, user).await);
}

/// F-3 : le contrôle de session précède la validation du corps (une session morte donne 401 même avec un
/// corps invalide) ; avec une session vivante, le corps invalide garde son erreur.
#[tokio::test]
async fn session_controlee_avant_la_validation_du_corps() {
    let Some(ent) = ent().await else { return };
    let etab = Uuid::new_v4();
    let (email, user) = compte(&ent, etab).await;
    let morte = connexion(&ent, &email, etab, ClientSession::Web).await;
    ent.logout(&format!("Bearer {}", morte.token))
        .await
        .expect("déconnexion");
    let vivante = connexion(&ent, &email, etab, ClientSession::Web).await;
    for (plateforme, jeton_corps) in [
        ("android", ""),
        ("android", "   "),
        ("inconnue", "abc"),
        ("inconnue", ""),
    ] {
        let r = ent
            .register_push_device_for_actor(
                etab,
                user,
                plateforme,
                jeton_corps,
                None,
                morte.session_id,
            )
            .await;
        assert!(
            matches!(
                r,
                Err(EntDbError::Auth(ent_auth::AuthError::InvalidSession))
            ),
            "{plateforme}/{jeton_corps:?} : {r:?}"
        );
    }
    let r = ent
        .register_push_device_for_actor(etab, user, "android", "", None, vivante.session_id)
        .await;
    assert!(
        matches!(r, Err(EntDbError::Domain(_))),
        "jeton vide : {r:?}"
    );
    let r = ent
        .register_push_device_for_actor(etab, user, "inconnue", "abc", None, vivante.session_id)
        .await;
    assert!(
        matches!(r, Err(EntDbError::Forbidden)),
        "plateforme inconnue : {r:?}"
    );
}

/// F-3 : le cas « aucune ligne retournée » (une autre personne détient le jeton par une écriture directe
/// concurrente, hors verrou) devient un conflit (409), jamais une erreur serveur. L'écriture rivale est
/// forcée : une transaction non validée tient l'inscription active pendant que la nôtre attend l'unicité.
#[tokio::test]
async fn ecriture_rivale_concurrente_donne_un_conflit_et_jamais_une_erreur_serveur() {
    let Some(ent) = ent().await else { return };
    let ent = std::sync::Arc::new(ent);
    let etab = Uuid::new_v4();
    let (email_a, a) = compte(&ent, etab).await;
    let (_, b) = compte(&ent, etab).await;
    let sa = connexion(&ent, &email_a, etab, ClientSession::Web).await;
    let t = jeton();
    let mut rival = ent.pool().begin().await.expect("transaction rivale");
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *rival)
        .await
        .expect("contexte");
    sqlx::query(
        "INSERT INTO ent_push_device (id, etablissement_id, user_id, platform, token, last_seen_at)
         VALUES ($1, $2, $3, 'android', $4, now())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(b)
    .bind(&t)
    .execute(&mut *rival)
    .await
    .expect("inscription rivale non validée");
    let ent2 = std::sync::Arc::clone(&ent);
    let tache = tokio::spawn({
        let t = t.clone();
        async move {
            ent2.register_push_device_for_actor(etab, a, "android", &t, None, sa.session_id)
                .await
        }
    });
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    rival.commit().await.expect("validation de la rivale");
    let r = tache.await.expect("tâche");
    assert!(
        matches!(&r, Err(EntDbError::Refus(crate::RefusMetier::Conflit, _))),
        "conflit attendu, obtenu : {r:?}"
    );
    assert_eq!(actifs(&ent, etab, b).await, 1, "la rivale garde son jeton");
    assert_eq!(actifs(&ent, etab, a).await, 0);
}

/// F-4 : une restauration réelle (neutralisation des files d'envoi) révoque les appareils actifs avec le
/// motif `restauration` (accepté par le `CHECK`) ; les autres établissements ne sont pas touchés.
#[tokio::test]
async fn restauration_reelle_revoque_les_appareils_avec_le_motif_restauration() {
    let Some(ent) = ent().await else { return };
    let (etab, autre) = (Uuid::new_v4(), Uuid::new_v4());
    let (email, user) = compte(&ent, etab).await;
    let (email_o, user_o) = compte(&ent, autre).await;
    let s = connexion(&ent, &email, etab, ClientSession::Web).await;
    let so = connexion(&ent, &email_o, autre, ClientSession::Web).await;
    let d = inscrire(&ent, etab, user, &jeton(), s.session_id).await;
    inscrire(&ent, autre, user_o, &jeton(), so.session_id).await;
    let mut tx = ent.pool().begin().await.expect("transaction");
    let r = crate::registre_donnees::neutraliser_files_restauration(
        &mut tx,
        crate::registre_donnees::PerimetreNeutralisation::Etablissement(etab),
    )
    .await
    .expect("restauration");
    tx.commit().await.expect("validation");
    assert!(r.total_neutralise() >= 1);
    let (revoque, motif): (bool, Option<String>) = sqlx::query_as(
        "SELECT revoked_at IS NOT NULL, revoque_motif FROM ent_push_device WHERE id = $1",
    )
    .bind(d.id)
    .fetch_one(ent.pool())
    .await
    .expect("appareil");
    assert!(revoque);
    assert_eq!(motif.as_deref(), Some("restauration"));
    assert_eq!(eligibles(&ent, etab, user).await, 0);
    assert_eq!(
        actifs(&ent, autre, user_o).await,
        1,
        "l'autre établissement est intact"
    );
}
