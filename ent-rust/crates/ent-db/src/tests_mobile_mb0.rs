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

async fn inscrire(
    ent: &PgEnt,
    etab: Uuid,
    user: Uuid,
    token: &str,
    session: Option<Uuid>,
) -> PushDevice {
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
        Some(s.session_id),
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
        Some(s1.session_id),
    )
    .await;
    inscrire(
        &ent,
        etab,
        user,
        &format!("tok-{}", Uuid::new_v4()),
        Some(s2.session_id),
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
    inscrire(&ent, etab, a, &jeton, Some(sa.session_id)).await;
    inscrire(&ent, etab, b, &jeton, Some(sb.session_id)).await;
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
    inscrire(&ent, etab, b, &jeton, Some(sb.session_id)).await;
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
        Some(sa.session_id),
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

/// DS5 : le registre ne classe plus les jetons d'appareils à l'export et les purge à la restauration.
#[test]
fn registre_jetons_non_exportes_et_purges() {
    let t = crate::registre_donnees::REGISTRE
        .iter()
        .find(|t| t.nom == "ent_push_device")
        .expect("table au registre");
    assert!(
        !t.export_portabilite,
        "les jetons d'appareils ne partent pas dans la portabilité"
    );
    assert!(matches!(
        t.neutralisation,
        crate::registre_donnees::Neutralisation::Purge
    ));
}
