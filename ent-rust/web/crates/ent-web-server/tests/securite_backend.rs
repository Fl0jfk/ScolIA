//! Écrans « Sécurité du compte » contre le **vrai** backend (`ent_api::app`) et une base
//! PostgreSQL **locale** de test : contrat de `/auth/me` (portée, assurance), appareils,
//! passkeys, politique de sécurité, et refus d'une session « enrôlement ».
//!
//! Lancement : comme `contre_backend.rs` (`ENT_DATABASE_URL` vers une base locale de test,
//! `--features labo`, `--test-threads=1`). Sans `ENT_DATABASE_URL`, les tests sont ignorés.
//! La parcours WebAuthn lui-même n'est pas testé ici (aucun authentificateur).
#![cfg(feature = "labo")]

use std::net::SocketAddr;

use ent_web_api::{ApiClient, ApiError};
use ent_web_app::noyau::portee::{decision_redirection, Portee, ENROLEMENT};
use ent_web_server::labo::{self, Labo};

async fn demarrer() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent");
        return None;
    };
    let parsed = ent_db::assert_local_postgres_url(&url)
        .unwrap_or_else(|e| panic!("base de test locale uniquement : {e}"));
    let nom_base = parsed.path().trim_start_matches('/').to_ascii_lowercase();
    assert!(
        ["test", "proto", "lab"]
            .iter()
            .any(|m| nom_base.contains(m)),
        "le nom de la base doit contenir test, proto ou lab (refusé : {nom_base})"
    );
    let (app, _) = ent_web_server::routeur_complet()
        .await
        .expect("démarrage ent-api + web");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    Some(format!("http://{addr}"))
}

async fn client_du_role(base: &str, l: &Labo, role: &str) -> ApiClient {
    let p = l
        .personnel
        .iter()
        .find(|p| p.role == role)
        .unwrap_or_else(|| panic!("personnel de labo « {role} »"));
    let s = labo::connecter(base, l, &p.email).await.expect("connexion");
    ApiClient::new(base, Some(s.token))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn moi_appareils_passkeys_contre_vrai_backend() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let prof = client_du_role(&base, &l, "prof").await;

    // `/auth/me` complet : session de mot de passe, navigateur, pas d'enrôlement.
    let m = prof.moi_securite().await.expect("moi_securite");
    assert_eq!(m.role, "prof");
    assert_eq!(m.portee, "complete");
    assert_eq!(m.assurance, "mot_de_passe");
    assert_eq!(m.client, "web");
    assert_eq!(Portee::depuis(&m.portee), Portee::Complete);
    assert_eq!(
        decision_redirection(Portee::depuis(&m.portee), "/web"),
        None
    );
    assert_eq!(m.identite().user_id, prof.moi().await.unwrap().user_id);

    // Passkeys : liste lisible (vide pour un compte de labo).
    assert!(prof.passkeys().await.expect("passkeys").is_empty());
    // Une suppression sans mot de passe actuel est refusée (403), sans effet.
    let r = prof.supprimer_passkey(uuid::Uuid::nil(), "").await;
    assert!(
        matches!(
            r,
            Err(ApiError::Interdit(_)) | Err(ApiError::Introuvable(_))
        ),
        "{r:?}"
    );

    // Appareils : exactement une session « courante » ; l'autre est déconnectable.
    let seconde = client_du_role(&base, &l, "prof").await;
    let liste = prof.appareils().await.expect("appareils");
    assert_eq!(liste.iter().filter(|a| a.courante).count(), 1);
    let autre = liste.iter().find(|a| !a.courante).expect("2e session");
    prof.deconnecter_appareil(autre.id)
        .await
        .expect("déconnexion");
    assert_eq!(
        seconde.moi().await.unwrap_err(),
        ApiError::NonConnecte,
        "l'appareil déconnecté perd sa session"
    );
    assert!(matches!(
        prof.deconnecter_appareil(autre.id).await,
        Err(ApiError::Introuvable(_))
    ));
    assert!(
        prof.moi().await.is_ok(),
        "la session courante reste valable"
    );
    // Session de l'appareil de quelqu'un d'autre : introuvable (aucune fuite).
    let direction = client_du_role(&base, &l, "direction").await;
    let ids_direction = direction.appareils().await.unwrap();
    let id_direction = ids_direction.iter().find(|a| a.courante).unwrap().id;
    assert!(matches!(
        prof.deconnecter_appareil(id_direction).await,
        Err(ApiError::Introuvable(_))
    ));
    assert!(direction.moi().await.is_ok());

    // Sans session : 401 partout.
    let anonyme = ApiClient::new(&base, None);
    assert_eq!(
        anonyme.moi_securite().await.unwrap_err(),
        ApiError::NonConnecte
    );
    assert_eq!(
        anonyme.appareils().await.unwrap_err(),
        ApiError::NonConnecte
    );
    assert_eq!(anonyme.passkeys().await.unwrap_err(), ApiError::NonConnecte);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn politique_et_session_d_enrolement_contre_vrai_backend() {
    let Some(base) = demarrer().await else { return };
    let l = labo::bootstrap(&base).await.expect("labo");
    let prof = client_du_role(&base, &l, "prof").await;
    let direction = client_du_role(&base, &l, "direction").await;

    // Réservé à Admin et Direction (403 pour un enseignant, en lecture comme en écriture).
    assert!(matches!(
        prof.politique_securite().await,
        Err(ApiError::Interdit(_))
    ));
    assert!(matches!(
        prof.definir_politique_securite(&["prof".into()]).await,
        Err(ApiError::Interdit(_))
    ));

    let avant = direction.politique_securite().await.expect("lecture");
    // Rôle de labo rarement utilisé par les autres tests : la comptabilité.
    let r = direction
        .definir_politique_securite(&["compta".into()])
        .await
        .expect("écriture");
    assert_eq!(r.passkey_obligatoire_roles, ["compta"]);
    assert_eq!(
        direction
            .politique_securite()
            .await
            .unwrap()
            .passkey_obligatoire_roles,
        ["compta"]
    );
    // Rôle inconnu : refusé, politique inchangée.
    assert!(direction
        .definir_politique_securite(&["inconnu".into()])
        .await
        .is_err());

    // Un compte du rôle concerné ouvre une session « enrôlement ».
    if l.personnel.iter().any(|p| p.role == "compta") {
        let compta = client_du_role(&base, &l, "compta").await;
        let m = compta.moi_securite().await.expect("moi compta");
        assert_eq!(m.portee, "enrolement");
        assert!(m.passkey_obligatoire);
        assert_eq!(
            decision_redirection(Portee::depuis(&m.portee), "/web/classes"),
            Some(ENROLEMENT)
        );
        // Toute autre route : 403 `passkey_enrolement_requis`, reconnu par l'interface.
        let refus = compta.fils_messages().await.unwrap_err();
        assert!(refus.est_enrolement_requis(), "{refus:?}");
        // Routes d'enrôlement et de gestion restent ouvertes.
        assert!(compta.passkeys().await.expect("passkeys").is_empty());
        compta
            .passkey_enrolement_debut()
            .await
            .expect("défi d'enrôlement");
        compta.deconnexion().await.expect("déconnexion possible");
    }

    // Restauration de la politique d'origine.
    direction
        .definir_politique_securite(&avant.passkey_obligatoire_roles)
        .await
        .expect("restauration");
}
