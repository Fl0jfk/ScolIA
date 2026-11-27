//! Socle 11, lot MB1 (serveur) : enveloppe d'erreur codée et `Idempotency-Key` générique, en HTTP sur la base locale
//! (routes de labo `/mobile/labo/*`). Tous les instants du registre viennent de l'horloge injectée (règle 33).

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{DateTime, Duration, TimeZone, Utc};
use ent_types::erreurs::CodeErreur;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicI64, Ordering};
use tower::ServiceExt;
use uuid::Uuid;

const MDP: &str = "DevLocalPass1!";

struct Banc {
    app: axum::Router,
    ent: Arc<PgEnt>,
    etab: Uuid,
    /// Décalage (secondes) de l'horloge injectée par rapport à `T0`.
    decalage: Arc<AtomicI64>,
    base: DateTime<Utc>,
}

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2027, 3, 8, 10, 0, 0).unwrap()
}

async fn banc() -> Option<Banc> {
    banc_a(t0()).await
}

/// Banc dont l'horloge injectée part de `base`. La purge globale supprime tout ce qui est plus vieux que 24 h **à son
/// instant** : le test de purge part d'une base très ancienne pour ne jamais effacer les clés en cours des autres
/// tests (qui tournent en parallèle sur la même base avec `t0`).
async fn banc_a(base: DateTime<Utc>) -> Option<Banc> {
    let url = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())?;
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let mut state = AppState::nouveau(ent);
    let decalage = Arc::new(AtomicI64::new(0));
    let d = decalage.clone();
    state.horloge = Arc::new(move || base + Duration::seconds(d.load(Ordering::SeqCst)));
    let ent = state.ent.clone();
    Some(Banc {
        app: app(state),
        ent,
        etab: Uuid::new_v4(),
        decalage,
        base,
    })
}

struct Reponse {
    statut: StatusCode,
    en_tetes: axum::http::HeaderMap,
    texte: String,
}

impl Reponse {
    fn json(&self) -> Value {
        serde_json::from_str(&self.texte)
            .unwrap_or_else(|_| panic!("corps non JSON : {}", self.texte))
    }
    fn code(&self) -> String {
        self.json()["erreur"]["code"]
            .as_str()
            .unwrap_or("")
            .to_string()
    }
}

async fn envoyer(
    app: &axum::Router,
    methode: &str,
    uri: &str,
    jeton: Option<&str>,
    en_tetes: &[(&str, &str)],
    corps: Option<Value>,
) -> Reponse {
    let mut b = Request::builder().method(methode).uri(uri);
    if let Some(j) = jeton {
        b = b.header("authorization", format!("Bearer {j}"));
    }
    for (k, v) in en_tetes {
        b = b.header(*k, *v);
    }
    let corps = match corps {
        Some(c) => {
            b = b.header("content-type", "application/json");
            Body::from(serde_json::to_vec(&c).unwrap())
        }
        None => Body::empty(),
    };
    let res = app.clone().oneshot(b.body(corps).unwrap()).await.unwrap();
    let (parts, body) = res.into_parts();
    let octets = axum::body::to_bytes(body, 1 << 20).await.unwrap();
    Reponse {
        statut: parts.status,
        en_tetes: parts.headers,
        texte: String::from_utf8_lossy(&octets).to_string(),
    }
}

async fn inscrire(b: &Banc) -> (String, String) {
    let email = format!("mb1-{}@ent.local", Uuid::new_v4().simple());
    let r = envoyer(
        &b.app,
        "POST",
        "/auth/register",
        None,
        &[],
        Some(json!({"email": email, "password": MDP, "etablissement_id": b.etab, "role": "administratif"})),
    )
    .await;
    assert_eq!(r.statut, StatusCode::CREATED, "register : {}", r.texte);
    (email.clone(), connecter(b, &email).await)
}

async fn connecter(b: &Banc, email: &str) -> String {
    let r = envoyer(
        &b.app,
        "POST",
        "/auth/login",
        None,
        &[],
        Some(json!({"email": email, "password": MDP, "etablissement_id": b.etab})),
    )
    .await;
    assert_eq!(r.statut, StatusCode::OK, "login : {}", r.texte);
    r.json()["token"].as_str().unwrap().to_string()
}

const APPLI: (&str, &str) = ("x-scolia-appli", "android/1.4.2 (build 142)");

fn cle() -> String {
    Uuid::new_v4().to_string()
}

async fn echo(b: &Banc, jeton: &str, k: &str, nom: &str, delai_ms: u64) -> Reponse {
    envoyer(
        &b.app,
        "POST",
        "/mobile/labo/echo",
        Some(jeton),
        &[APPLI, ("idempotency-key", k)],
        Some(json!({"nom": nom, "delai_ms": delai_ms})),
    )
    .await
}

async fn executions(b: &Banc, jeton: &str, nom: &str) -> u64 {
    let r = envoyer(
        &b.app,
        "GET",
        &format!("/mobile/labo/echo/{nom}"),
        Some(jeton),
        &[APPLI],
        None,
    )
    .await;
    r.json()["execution"].as_u64().unwrap()
}

#[tokio::test]
async fn mb1_enveloppe_401_403_404_422_et_web_inchange() {
    let Some(b) = banc().await else {
        return eprintln!("skip MB1 : ENT_DATABASE_URL");
    };
    let (_, jeton) = inscrire(&b).await;

    // 401 : pas de session. Route `/mobile/*` : enveloppe même sans en-tête d'appli.
    let r = envoyer(
        &b.app,
        "POST",
        "/mobile/labo/echo",
        None,
        &[],
        Some(json!({"nom": "x"})),
    )
    .await;
    assert_eq!(r.statut, StatusCode::UNAUTHORIZED);
    assert_eq!(r.code(), "session.expiree");
    let e = &r.json()["erreur"];
    assert_eq!(e["message"], CodeErreur::SessionExpiree.message_neutre());
    assert!(e["champs"].as_array().unwrap().is_empty() && e["reessayer_apres_s"].is_null());
    // `trace` = identifiant de requête corrélé (même valeur que l'en-tête de réponse), jamais une donnée.
    let rid = r.en_tetes.get("x-request-id").unwrap().to_str().unwrap();
    assert_eq!(e["trace"], rid);
    assert!(
        r.en_tetes.get("x-scolia-code").is_none(),
        "l'en-tête interne ne sort jamais"
    );

    // Le web garde sa réponse actuelle (texte), pour la même route d'une route non-mobile.
    let web = envoyer(&b.app, "GET", "/roles/mes-droits", None, &[], None).await;
    assert_eq!(web.statut, StatusCode::UNAUTHORIZED);
    assert!(
        serde_json::from_str::<Value>(&web.texte).is_err(),
        "web inchangé : {}",
        web.texte
    );
    // … et reçoit l'enveloppe quand il déclare l'appli.
    let app_r = envoyer(&b.app, "GET", "/roles/mes-droits", None, &[APPLI], None).await;
    assert_eq!(app_r.statut, StatusCode::UNAUTHORIZED);
    assert_eq!(app_r.code(), "session.expiree");

    // Route d'authentification : pas de faux « session expirée ».
    let login = envoyer(
        &b.app,
        "POST",
        "/auth/login",
        None,
        &[APPLI],
        Some(json!({"email": "inconnu@ent.local", "password": "x", "etablissement_id": b.etab})),
    )
    .await;
    assert_eq!(login.statut, StatusCode::UNAUTHORIZED);
    assert_eq!(login.code(), "auth.refusee");

    // 403 et 404 : le texte de la route (qui trahit la cause) n'est jamais reproduit.
    for (statut, code) in [
        (403, CodeErreur::DroitRefuse),
        (404, CodeErreur::ObjetIntrouvable),
    ] {
        let r = envoyer(
            &b.app,
            "POST",
            &format!("/mobile/labo/echec/{statut}"),
            Some(&jeton),
            &[APPLI, ("idempotency-key", &cle())],
            None,
        )
        .await;
        assert_eq!(r.statut.as_u16(), statut);
        assert_eq!(r.code(), code.as_str());
        assert_eq!(r.json()["erreur"]["message"], code.message_neutre());
        for interdit in ["interdiction", "existe", "périmètre", "élève"] {
            assert!(
                !r.texte.contains(interdit),
                "fuite « {interdit} » dans {}",
                r.texte
            );
        }
    }
    // Le web voit toujours le texte d'origine de la route.
    let web403 = envoyer(
        &b.app,
        "GET",
        "/mobile/labo/echec/403",
        Some(&jeton),
        &[],
        None,
    )
    .await;
    assert!(
        web403.json()["erreur"]["code"] == "droit.refuse",
        "/mobile/* est toujours enveloppé"
    );

    // 422 : code par défaut, message de la route conservé.
    let r = envoyer(
        &b.app,
        "POST",
        "/mobile/labo/echec/422",
        Some(&jeton),
        &[APPLI, ("idempotency-key", &cle())],
        None,
    )
    .await;
    assert_eq!(r.statut, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.code(), "regle.non_respectee");
    assert_eq!(r.json()["erreur"]["message"], "échec simulé 422");
}

#[tokio::test]
async fn mb1_idempotence_rejeu_identique_conflit_et_cle_obligatoire() {
    let Some(b) = banc().await else {
        return eprintln!("skip MB1 : ENT_DATABASE_URL");
    };
    let (_, jeton) = inscrire(&b).await;
    let nom = format!("rejeu-{}", Uuid::new_v4().simple());
    let k = cle();

    let r1 = echo(&b, &jeton, &k, &nom, 0).await;
    assert_eq!(r1.statut, StatusCode::CREATED, "{}", r1.texte);
    assert_eq!(r1.json()["execution"], 1);
    assert!(r1.en_tetes.get("idempotent-replayed").is_none());

    // Même clé, même corps : même réponse (statut, corps), sans seconde exécution.
    let r2 = echo(&b, &jeton, &k, &nom, 0).await;
    assert_eq!(r2.statut, StatusCode::CREATED);
    assert_eq!(r2.texte, r1.texte);
    assert_eq!(r2.en_tetes.get("idempotent-replayed").unwrap(), "true");
    assert_eq!(
        r2.en_tetes.get("content-type"),
        r1.en_tetes.get("content-type")
    );
    assert_eq!(executions(&b, &jeton, &nom).await, 1);

    // Même clé, corps différent : conflit codé, rien d'exécuté.
    let r3 = echo(&b, &jeton, &k, &format!("{nom}-autre"), 0).await;
    assert_eq!(r3.statut, StatusCode::CONFLICT);
    assert_eq!(r3.code(), "idempotence.conflit");
    // Même clé, même corps, autre route : conflit aussi (l'empreinte porte le chemin).
    let r4 = envoyer(
        &b.app,
        "POST",
        "/mobile/labo/echec/422",
        Some(&jeton),
        &[APPLI, ("idempotency-key", &k)],
        Some(json!({"nom": nom, "delai_ms": 0})),
    )
    .await;
    assert_eq!(r4.code(), "idempotence.conflit");

    // Appli sans clé : 400. Clé invalide : 400. Web sans clé : exécuté normalement.
    let sans = envoyer(
        &b.app,
        "POST",
        "/mobile/labo/echo",
        Some(&jeton),
        &[APPLI],
        Some(json!({"nom": nom})),
    )
    .await;
    assert_eq!(
        (sans.statut, sans.code().as_str()),
        (StatusCode::BAD_REQUEST, "idempotence.requise")
    );
    let mauvaise = echo(&b, &jeton, "pas-un-uuid", &nom, 0).await;
    assert_eq!(
        (mauvaise.statut, mauvaise.code().as_str()),
        (StatusCode::BAD_REQUEST, "idempotence.cle_invalide")
    );
    let web = envoyer(
        &b.app,
        "POST",
        "/mobile/labo/echo",
        Some(&jeton),
        &[],
        Some(json!({"nom": nom})),
    )
    .await;
    assert_eq!(web.statut, StatusCode::CREATED);
    assert_eq!(
        web.json()["execution"],
        2,
        "le web sans clé s'exécute normalement"
    );

    // Un refus n'est pas conservé : la clé est libérée, un nouvel essai s'exécute pour de bon.
    let kf = cle();
    for _ in 0..2 {
        let r = envoyer(
            &b.app,
            "POST",
            "/mobile/labo/echec/422",
            Some(&jeton),
            &[APPLI, ("idempotency-key", &kf)],
            None,
        )
        .await;
        assert_eq!(r.statut, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(r.en_tetes.get("idempotent-replayed").is_none());
    }
}

#[tokio::test]
async fn mb1_idempotence_concurrence_une_seule_execution() {
    let Some(b) = banc().await else {
        return eprintln!("skip MB1 : ENT_DATABASE_URL");
    };
    let (_, jeton) = inscrire(&b).await;
    let nom = format!("conc-{}", Uuid::new_v4().simple());
    let k = cle();
    let (a, c, d) = tokio::join!(
        echo(&b, &jeton, &k, &nom, 400),
        echo(&b, &jeton, &k, &nom, 400),
        echo(&b, &jeton, &k, &nom, 400)
    );
    for r in [&a, &c, &d] {
        assert_eq!(r.statut, StatusCode::CREATED, "{}", r.texte);
        assert_eq!(
            r.json()["execution"],
            1,
            "toutes les réponses sont celles de l'unique exécution"
        );
    }
    assert_eq!(executions(&b, &jeton, &nom).await, 1, "une seule exécution");
    let rejouees = [&a, &c, &d]
        .iter()
        .filter(|r| r.en_tetes.contains_key("idempotent-replayed"))
        .count();
    assert_eq!(rejouees, 2);
}

#[tokio::test]
async fn mb1_idempotence_cloisonnee_par_session_et_par_personne() {
    let Some(b) = banc().await else {
        return eprintln!("skip MB1 : ENT_DATABASE_URL");
    };
    let (email, session_a) = inscrire(&b).await;
    let session_a2 = connecter(&b, &email).await;
    let (_, autre_personne) = inscrire(&b).await;
    let nom = format!("cloison-{}", Uuid::new_v4().simple());
    let k = cle();

    assert_eq!(
        echo(&b, &session_a, &k, &nom, 0).await.json()["execution"],
        1
    );
    // Même personne, autre session : la clé ne rejoue rien, l'écriture s'exécute.
    let r = echo(&b, &session_a2, &k, &nom, 0).await;
    assert_eq!(r.statut, StatusCode::CREATED);
    assert_eq!(r.json()["execution"], 2);
    assert!(r.en_tetes.get("idempotent-replayed").is_none());
    // Autre personne, même établissement : idem.
    let r = echo(&b, &autre_personne, &k, &nom, 0).await;
    assert_eq!(r.json()["execution"], 3);
    assert!(r.en_tetes.get("idempotent-replayed").is_none());
    // Et chacune rejoue bien la sienne.
    assert_eq!(
        echo(&b, &session_a2, &k, &nom, 0).await.json()["execution"],
        2
    );
    assert_eq!(
        echo(&b, &session_a, &k, &nom, 0).await.json()["execution"],
        1
    );
    assert_eq!(executions(&b, &session_a, &nom).await, 3);

    // Au niveau de la base : trois lignes, une par session, dans l'établissement de ces sessions.
    let url = std::env::var("ENT_DATABASE_URL").unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let n: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_idempotence_cle WHERE etablissement_id = $1 AND cle = $2",
    )
    .bind(b.etab)
    .bind(Uuid::parse_str(&k).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n.0, 3);
}

#[tokio::test]
async fn mb1_idempotence_purge_a_24_heures_horloge_injectee() {
    let Some(b) = banc_a(t0() - Duration::days(1000)).await else {
        return eprintln!("skip MB1 : ENT_DATABASE_URL");
    };
    let (_, jeton) = inscrire(&b).await;
    let nom = format!("purge-{}", Uuid::new_v4().simple());
    let k = cle();

    assert_eq!(echo(&b, &jeton, &k, &nom, 0).await.json()["execution"], 1);
    // À 23 h 59 : toujours rejoué.
    b.decalage.store(24 * 3600 - 60, Ordering::SeqCst);
    let r = echo(&b, &jeton, &k, &nom, 0).await;
    assert_eq!(r.json()["execution"], 1);
    assert!(r.en_tetes.contains_key("idempotent-replayed"));
    // À 24 h 01 : la purge supprime l'entrée (au moins la nôtre), et la clé ne rejoue plus rien.
    b.decalage.store(24 * 3600 + 60, Ordering::SeqCst);
    // F7 : la passe périodique elle-même, à l'instant injecté (et non `Utc::now()`).
    let bilan = crate::purge::une_passe_a(&b.ent, b.base + Duration::seconds(24 * 3600 + 60)).await;
    assert_eq!(bilan.echecs, 0);
    let supprimees = bilan.cles_idempotence;
    assert!(
        supprimees >= 1,
        "la purge doit supprimer l'entrée de plus de 24 h ({supprimees})"
    );
    let r = echo(&b, &jeton, &k, &nom, 0).await;
    assert_eq!(r.statut, StatusCode::CREATED);
    assert_eq!(r.json()["execution"], 2, "après 24 h la clé est neuve");
    assert!(!r.en_tetes.contains_key("idempotent-replayed"));
}

#[tokio::test]
async fn mb1_idempotence_cle_expiree_non_purgee_ne_rejoue_pas() {
    let Some(b) = banc().await else {
        return eprintln!("skip MB1 : ENT_DATABASE_URL");
    };
    let (_, jeton) = inscrire(&b).await;
    let nom = format!("expire-{}", Uuid::new_v4().simple());
    let k = cle();
    assert_eq!(echo(&b, &jeton, &k, &nom, 0).await.json()["execution"], 1);
    // Sans passage de la purge globale : la revendication elle-même ignore (et supprime) ce qui a plus de 24 h.
    b.decalage.store(25 * 3600, Ordering::SeqCst);
    let r = echo(&b, &jeton, &k, &nom, 0).await;
    assert_eq!(r.json()["execution"], 2);
    assert!(!r.en_tetes.contains_key("idempotent-replayed"));
}

/// Faible F1 (contre-revue MB1) : un texte de base de données relayé par une route (400, 409, 503) ne sort jamais
/// dans l'enveloppe ; et un 5xx ne garde pas la clé (un nouvel essai s'exécute pour de bon, rien n'est rejoué).
#[tokio::test]
async fn mb1_f1_texte_technique_jamais_relaye_et_5xx_non_conserve() {
    let Some(b) = banc().await else {
        return eprintln!("skip MB1 : ENT_DATABASE_URL");
    };
    let (_, jeton) = inscrire(&b).await;
    for (statut, code) in [
        (400, CodeErreur::RequeteInvalide),
        (409, CodeErreur::ObjetConflit),
        (503, CodeErreur::ServiceMaintenance),
    ] {
        let k = cle();
        for essai in 0..2 {
            let r = envoyer(
                &b.app,
                "POST",
                &format!("/mobile/labo/echec/{statut}"),
                Some(&jeton),
                &[APPLI, ("idempotency-key", &k)],
                None,
            )
            .await;
            assert_eq!(r.statut.as_u16(), statut, "{}", r.texte);
            assert_eq!(r.code(), code.as_str());
            assert_eq!(r.json()["erreur"]["message"], code.message_neutre());
            for interdit in ["duplicate", "constraint", "ent_eleve", "database", "sqlx", "pool"] {
                assert!(!r.texte.contains(interdit), "fuite « {interdit} » : {}", r.texte);
            }
            assert!(
                r.en_tetes.get("idempotent-replayed").is_none(),
                "essai {essai} : un échec n'est jamais rejoué"
            );
        }
    }
}

/// Faible F4 (contre-revue MB1) : l'en-tête interne `x-scolia-code` ne sort jamais, y compris vers le web (route
/// hors `/mobile`, sans en-tête d'appli) quand c'est une couche **intérieure** (idempotence) qui l'a posé.
#[tokio::test]
async fn mb1_f4_en_tete_interne_jamais_servi_au_web() {
    let Some(b) = banc().await else {
        return eprintln!("skip MB1 : ENT_DATABASE_URL");
    };
    let (_, jeton) = inscrire(&b).await;
    let r = envoyer(
        &b.app,
        "POST",
        "/roles/mes-droits",
        Some(&jeton),
        &[("idempotency-key", "pas-un-uuid")],
        Some(json!({})),
    )
    .await;
    assert_eq!(r.statut, StatusCode::BAD_REQUEST, "{}", r.texte);
    assert!(r.en_tetes.get("x-scolia-code").is_none());
    assert!(
        serde_json::from_str::<Value>(&r.texte).is_err(),
        "le web reçoit le texte, pas l'enveloppe : {}",
        r.texte
    );
}

/// Tests manquants de la contre-revue MB1 : le web qui envoie une clé est rejoué comme l'appli ; la requête (`?x=1`)
/// fait partie de l'empreinte ; une même clé dans un autre établissement ne rejoue rien.
#[tokio::test]
async fn mb1_web_avec_cle_requete_dans_l_empreinte_et_autre_etablissement() {
    let Some(b) = banc().await else {
        return eprintln!("skip MB1 : ENT_DATABASE_URL");
    };
    let (_, jeton) = inscrire(&b).await;
    let nom = format!("web-{}", Uuid::new_v4().simple());
    let k = cle();
    let corps = json!({"nom": nom, "delai_ms": 0});
    let web = |uri: &'static str| {
        let (app, jeton, k, corps) = (b.app.clone(), jeton.clone(), k.clone(), corps.clone());
        async move {
            envoyer(&app, "POST", uri, Some(&jeton), &[("idempotency-key", &k)], Some(corps)).await
        }
    };
    let r1 = web("/mobile/labo/echo").await;
    assert_eq!(r1.statut, StatusCode::CREATED, "{}", r1.texte);
    let r2 = web("/mobile/labo/echo").await;
    assert_eq!(r2.texte, r1.texte);
    assert_eq!(r2.en_tetes.get("idempotent-replayed").unwrap(), "true");
    let r3 = web("/mobile/labo/echo?x=1").await;
    assert_eq!(r3.statut, StatusCode::CONFLICT);
    assert_eq!(r3.code(), "idempotence.conflit");
    assert_eq!(executions(&b, &jeton, &nom).await, 1);

    // Autre établissement, même clé, même corps : exécuté, rien de rejoué.
    let Some(b2) = banc().await else { return };
    let (_, jeton2) = inscrire(&b2).await;
    let r = envoyer(
        &b2.app,
        "POST",
        "/mobile/labo/echo",
        Some(&jeton2),
        &[APPLI, ("idempotency-key", &k)],
        Some(json!({"nom": nom, "delai_ms": 0})),
    )
    .await;
    assert_eq!(r.statut, StatusCode::CREATED);
    assert!(r.en_tetes.get("idempotent-replayed").is_none());
    assert_eq!(r.json()["execution"], 2);
}
