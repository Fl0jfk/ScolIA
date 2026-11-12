//! Socle 1, C2 (D-012) : l'établissement est déduit de l'adresse, jamais saisi.
//!
//! Tests contre le vrai routeur `ent-api` et une base PostgreSQL locale de test
//! (`ENT_DATABASE_URL`, sinon SKIP). Sans feature `labo` : comptes créés par le
//! service. Les variables `ENT_TENANT_PAR_HOTE` / `ENT_TRUST_PROXY` sont
//! modifiées : les tests de ce binaire sont sérialisés par un verrou.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ent_api::{app, AppState};
use ent_db::PgEnt;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Mutex;
use tower::ServiceExt;
use uuid::Uuid;

const MDP: &str = "TenantHote-Pass1!";

static VERROU: Mutex<()> = Mutex::new(());

/// Fixe des variables d'environnement pour la durée d'un test (restaurées,
/// même en cas d'échec).
struct Env(Vec<&'static str>);

impl Env {
    fn new(vars: &[(&'static str, Option<&str>)]) -> Self {
        for (k, v) in vars {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
        Env(vars.iter().map(|(k, _)| *k).collect())
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        for k in &self.0 {
            std::env::remove_var(k);
        }
    }
}

struct Monde {
    app: axum::Router,
    ent: std::sync::Arc<PgEnt>,
    pool: sqlx::PgPool,
    a: Uuid,
    b: Uuid,
    hote_a: String,
    hote_b: String,
    /// Membre de A et de B (admin des deux).
    commun: String,
    /// Membre de A seulement.
    seul_a: String,
    /// Membre de B seulement.
    seul_b: String,
}

async fn monde() -> Option<Monde> {
    let url = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty());
    let Some(url) = url else {
        eprintln!("SKIP tenant_hote : ENT_DATABASE_URL absent");
        return None;
    };
    let mut ent = PgEnt::connect(&url).await.expect("connexion");
    ent.migrate().await.expect("migrations");
    let state = AppState::nouveau(ent);
    let ent = state.ent.clone();
    let pool = sqlx::PgPool::connect(&url).await.expect("pool");
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let suffixe = Uuid::new_v4().simple().to_string();
    let (hote_a, hote_b) = (
        format!("a-{}.test.scolia.fr", &suffixe[..12]),
        format!("b-{}.test.scolia.fr", &suffixe[..12]),
    );
    let mel = |n: &str| format!("c2-{n}-{suffixe}@ent.local");
    let (commun, seul_a, seul_b) = (mel("commun"), mel("a"), mel("b"));
    ent.register_user(&commun, MDP, None, a, "admin")
        .await
        .expect("commun A");
    ent.register_user(&seul_a, MDP, None, a, "direction")
        .await
        .expect("seul A");
    ent.register_user(&seul_b, MDP, None, b, "admin")
        .await
        .expect("seul B");
    let admin: (Uuid,) = sqlx::query_as("SELECT id FROM ent_user WHERE email = $1")
        .bind(&commun)
        .fetch_one(&pool)
        .await
        .expect("id admin");
    sqlx::query(
        "INSERT INTO ent_membership (user_id, etablissement_id, role, created_at)
         VALUES ($1, $2, 'admin', now())",
    )
    .bind(admin.0)
    .bind(b)
    .execute(&pool)
    .await
    .expect("commun B");
    for (etab, slug, hote) in [(a, "a", &hote_a), (b, "b", &hote_b)] {
        // B-02 : établissement et domaine sont validés par la plateforme.
        ent.declarer_etablissement_plateforme(
            etab,
            &format!("c2{slug}-{}", &suffixe[..12]),
            &format!("Établissement {slug}"),
        )
        .await
        .expect("déclaration");
        ent.ajouter_domaine_plateforme(etab, hote)
            .await
            .expect("domaine");
    }
    Some(Monde {
        app: app(state),
        ent,
        pool,
        a,
        b,
        hote_a,
        hote_b,
        commun,
        seul_a,
        seul_b,
    })
}

struct Rep {
    statut: StatusCode,
    corps: Value,
}

impl Monde {
    async fn appel(
        &self,
        methode: &str,
        chemin: &str,
        en_tetes: &[(&str, &str)],
        jeton: Option<&str>,
        corps: Option<Value>,
    ) -> Rep {
        let mut req = Request::builder().method(methode).uri(chemin);
        for (k, v) in en_tetes {
            req = req.header(*k, *v);
        }
        if let Some(j) = jeton {
            req = req.header("authorization", format!("Bearer {j}"));
        }
        let req = match corps {
            Some(c) => req
                .header("content-type", "application/json")
                .body(Body::from(c.to_string())),
            None => req.body(Body::empty()),
        }
        .unwrap();
        let res = self.app.clone().oneshot(req).await.unwrap();
        let statut = res.status();
        let octets = res.into_body().collect().await.unwrap().to_bytes();
        let corps = serde_json::from_slice(&octets)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&octets).into_owned()));
        Rep { statut, corps }
    }

    async fn connexion(&self, hote: Option<&str>, email: &str, etab: Option<Uuid>) -> Rep {
        let mut corps = json!({ "email": email, "password": MDP });
        if let Some(e) = etab {
            corps["etablissement_id"] = json!(e);
        }
        let en_tetes: Vec<(&str, &str)> = hote.map(|h| vec![("host", h)]).unwrap_or_default();
        self.appel("POST", "/auth/login", &en_tetes, None, Some(corps))
            .await
    }

    async fn jeton(&self, hote: Option<&str>, email: &str, etab: Option<Uuid>) -> String {
        let r = self.connexion(hote, email, etab).await;
        assert_eq!(
            r.statut,
            StatusCode::OK,
            "connexion {email} : {:?}",
            r.corps
        );
        r.corps["token"].as_str().expect("jeton").to_string()
    }
}

fn verrou() -> std::sync::MutexGuard<'static, ()> {
    VERROU.lock().unwrap_or_else(|e| e.into_inner())
}

#[tokio::test]
async fn hote_impose_l_etablissement_malgre_le_corps() {
    let _v = verrou();
    let _e = Env::new(&[("ENT_TENANT_PAR_HOTE", None), ("ENT_TRUST_PROXY", None)]);
    let Some(m) = monde().await else { return };

    // Membre de A et B, connexion sur l'adresse de A en demandant B : session A.
    let r = m.connexion(Some(&m.hote_a), &m.commun, Some(m.b)).await;
    assert_eq!(r.statut, StatusCode::OK, "{:?}", r.corps);
    assert_eq!(
        r.corps["etablissement_id"],
        json!(m.a),
        "établissement imposé par l'hôte"
    );

    // Sans etablissement_id du tout : l'adresse suffit.
    let r = m.connexion(Some(&m.hote_b), &m.commun, None).await;
    assert_eq!(r.statut, StatusCode::OK, "{:?}", r.corps);
    assert_eq!(r.corps["etablissement_id"], json!(m.b));

    // Fiche publique de l'établissement de l'adresse.
    let r = m
        .appel(
            "GET",
            "/auth/etablissement",
            &[("host", &m.hote_a)],
            None,
            None,
        )
        .await;
    assert_eq!(r.statut, StatusCode::OK);
    assert_eq!(r.corps["id"], json!(m.a));
    assert_eq!(r.corps["nom"], "Établissement a");

    // Fournisseurs SSO publics : l'etablissement_id de la query est ignoré.
    let r = m
        .appel(
            "GET",
            &format!("/auth/oidc/public?etablissement_id={}", m.b),
            &[("host", &m.hote_a)],
            None,
            None,
        )
        .await;
    assert_eq!(r.statut, StatusCode::OK);

    // Passkey sans mot de passe : réponse générique, établissement de l'hôte.
    let r = m
        .appel(
            "POST",
            "/auth/passkey/login/start",
            &[("host", &m.hote_a)],
            None,
            Some(json!({ "email": m.seul_a })),
        )
        .await;
    assert_ne!(
        r.statut,
        StatusCode::BAD_REQUEST,
        "pas d'etablissement_id requis : {:?}",
        r.corps
    );

    // Démarrage SSO : sans etablissement_id, résolu par l'hôte (fournisseur absent → 4xx, pas 400 « établissement requis »).
    let r = m
        .appel(
            "POST",
            "/auth/oidc/ent/start",
            &[("host", &m.hote_a)],
            None,
            Some(json!({})),
        )
        .await;
    assert_ne!(r.corps, json!("établissement requis"));
}

#[tokio::test]
async fn non_membre_meme_reponse_que_identifiants_invalides() {
    let _v = verrou();
    let _e = Env::new(&[("ENT_TENANT_PAR_HOTE", None), ("ENT_TRUST_PROXY", None)]);
    let Some(m) = monde().await else { return };

    // Membre de B seulement, sur l'adresse de A (et en demandant B dans le corps).
    let non_membre = m.connexion(Some(&m.hote_a), &m.seul_b, Some(m.b)).await;
    let mauvais = m
        .appel(
            "POST",
            "/auth/login",
            &[("host", &m.hote_a)],
            None,
            Some(json!({ "email": m.seul_a, "password": "mauvais-mot-de-passe" })),
        )
        .await;
    assert_eq!(
        non_membre.statut,
        StatusCode::UNAUTHORIZED,
        "{:?}",
        non_membre.corps
    );
    assert_eq!(non_membre.statut, mauvais.statut);
    assert_eq!(non_membre.corps, mauvais.corps, "même message");

    // La cause reste journalisée côté serveur (échec de connexion dans A).
    let n: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_journal_modif
          WHERE etablissement_id = $1 AND action = 'connexion_echec'",
    )
    .bind(m.a)
    .fetch_one(&m.pool)
    .await
    .unwrap();
    assert!(n.0 >= 1, "échec journalisé");
}

#[tokio::test]
async fn strict_hote_inconnu_404() {
    let _v = verrou();
    let _e = Env::new(&[
        ("ENT_TENANT_PAR_HOTE", Some("strict")),
        ("ENT_TRUST_PROXY", None),
    ]);
    let Some(m) = monde().await else { return };
    let inconnu = [("host", "inconnu.test.scolia.fr")];

    let r = m
        .connexion(Some("inconnu.test.scolia.fr"), &m.seul_a, Some(m.a))
        .await;
    assert_eq!(r.statut, StatusCode::NOT_FOUND, "{:?}", r.corps);
    // Sans en-tête Host non plus.
    let r = m.connexion(None, &m.seul_a, Some(m.a)).await;
    assert_eq!(r.statut, StatusCode::NOT_FOUND);
    for chemin in ["/auth/etablissement", "/auth/me", "/auth/oidc/public"] {
        let r = m.appel("GET", chemin, &inconnu, None, None).await;
        assert_eq!(r.statut, StatusCode::NOT_FOUND, "{chemin}");
    }
    // Sondes exemptées.
    for chemin in ["/health", "/ready"] {
        let r = m.appel("GET", chemin, &inconnu, None, None).await;
        assert_eq!(r.statut, StatusCode::OK, "{chemin}");
    }
    let r = m.appel("GET", "/metrics", &inconnu, None, None).await;
    assert_ne!(
        r.corps,
        json!("établissement inconnu pour cette adresse"),
        "/metrics garde sa propre politique"
    );

    // Hôte connu : etablissement_id du client ignoré.
    let r = m.connexion(Some(&m.hote_a), &m.commun, Some(m.b)).await;
    assert_eq!(r.statut, StatusCode::OK);
    assert_eq!(r.corps["etablissement_id"], json!(m.a));
}

#[tokio::test]
async fn session_de_a_sur_l_hote_de_b_401() {
    let _v = verrou();
    for mode in [Some("strict"), None] {
        let _e = Env::new(&[("ENT_TENANT_PAR_HOTE", mode), ("ENT_TRUST_PROXY", None)]);
        let Some(m) = monde().await else { return };
        let jeton = m.jeton(Some(&m.hote_a), &m.commun, None).await;

        let r = m
            .appel(
                "GET",
                "/auth/me",
                &[("host", &m.hote_a)],
                Some(&jeton),
                None,
            )
            .await;
        assert_eq!(r.statut, StatusCode::OK, "{mode:?}");
        let r = m
            .appel(
                "GET",
                "/auth/me",
                &[("host", &m.hote_b)],
                Some(&jeton),
                None,
            )
            .await;
        assert_eq!(
            r.statut,
            StatusCode::UNAUTHORIZED,
            "{mode:?} : session A sur l'hôte B"
        );
        let r = m
            .appel(
                "GET",
                "/etablissement/domaines",
                &[("host", &m.hote_b)],
                Some(&jeton),
                None,
            )
            .await;
        assert_eq!(r.statut, StatusCode::UNAUTHORIZED, "{mode:?}");
    }
}

#[tokio::test]
async fn souple_comportement_actuel() {
    let _v = verrou();
    let _e = Env::new(&[
        ("ENT_TENANT_PAR_HOTE", Some("souple")),
        ("ENT_TRUST_PROXY", None),
    ]);
    let Some(m) = monde().await else { return };

    for hote in [None, Some("inconnu.test.scolia.fr"), Some("[::1]:8088")] {
        let r = m.connexion(hote, &m.commun, Some(m.b)).await;
        assert_eq!(r.statut, StatusCode::OK, "{hote:?}");
        assert_eq!(
            r.corps["etablissement_id"],
            json!(m.b),
            "corps respecté ({hote:?})"
        );
        let jeton = r.corps["token"].as_str().unwrap().to_string();
        let en_tetes: Vec<(&str, &str)> = hote.map(|h| vec![("host", h)]).unwrap_or_default();
        let r = m
            .appel("GET", "/auth/me", &en_tetes, Some(&jeton), None)
            .await;
        assert_eq!(r.statut, StatusCode::OK, "{hote:?}");
    }
    // Hôte inconnu, sans etablissement_id : 400 explicite.
    let r = m
        .connexion(Some("inconnu.test.scolia.fr"), &m.commun, None)
        .await;
    assert_eq!(r.statut, StatusCode::BAD_REQUEST);
    let r = m
        .appel(
            "GET",
            "/auth/etablissement",
            &[("host", "inconnu.test.scolia.fr")],
            None,
            None,
        )
        .await;
    assert_eq!(r.statut, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn normalisation_de_l_hote() {
    let _v = verrou();
    let _e = Env::new(&[
        ("ENT_TENANT_PAR_HOTE", Some("strict")),
        ("ENT_TRUST_PROXY", None),
    ]);
    let Some(m) = monde().await else { return };

    let majuscules = m.hote_a.to_uppercase();
    for hote in [
        majuscules.clone(),
        format!("{}:8443", m.hote_a),
        format!("{}.", m.hote_a),
        format!("{majuscules}.:443"),
    ] {
        let r = m
            .appel("GET", "/auth/etablissement", &[("host", &hote)], None, None)
            .await;
        assert_eq!(r.statut, StatusCode::OK, "{hote}");
        assert_eq!(r.corps["id"], json!(m.a), "{hote}");
    }
    for hote in [
        format!("{}:abc", m.hote_a),
        format!("x.{}", m.hote_a),
        format!("{}..", m.hote_a),
    ] {
        let r = m
            .appel("GET", "/auth/etablissement", &[("host", &hote)], None, None)
            .await;
        assert_eq!(r.statut, StatusCode::NOT_FOUND, "{hote}");
    }
}

#[tokio::test]
async fn x_forwarded_host_ignore_sans_proxy_de_confiance() {
    let _v = verrou();
    let Some(m) = ({
        let _e = Env::new(&[("ENT_TENANT_PAR_HOTE", None), ("ENT_TRUST_PROXY", None)]);
        monde().await
    }) else {
        return;
    };
    let en_tetes = [
        ("host", "inconnu.test.scolia.fr"),
        ("x-forwarded-host", m.hote_a.as_str()),
    ];
    {
        let _e = Env::new(&[
            ("ENT_TENANT_PAR_HOTE", Some("strict")),
            ("ENT_TRUST_PROXY", None),
        ]);
        let r = m
            .appel("GET", "/auth/etablissement", &en_tetes, None, None)
            .await;
        assert_eq!(r.statut, StatusCode::NOT_FOUND, "X-Forwarded-Host ignoré");
        // Et un Host connu ne peut pas être détourné par X-Forwarded-Host.
        let r = m
            .appel(
                "GET",
                "/auth/etablissement",
                &[("host", &m.hote_b), ("x-forwarded-host", &m.hote_a)],
                None,
                None,
            )
            .await;
        assert_eq!(r.corps["id"], json!(m.b));
    }
    {
        let _e = Env::new(&[
            ("ENT_TENANT_PAR_HOTE", Some("strict")),
            ("ENT_TRUST_PROXY", Some("1")),
        ]);
        let r = m
            .appel("GET", "/auth/etablissement", &en_tetes, None, None)
            .await;
        assert_eq!(r.statut, StatusCode::OK, "derrière un proxy de confiance");
        assert_eq!(r.corps["id"], json!(m.a));
    }
}

#[tokio::test]
async fn administration_des_domaines() {
    let _v = verrou();
    let _e = Env::new(&[("ENT_TENANT_PAR_HOTE", None), ("ENT_TRUST_PROXY", None)]);
    let Some(m) = monde().await else { return };
    let h = [("host", m.hote_a.as_str())];
    let admin = m.jeton(Some(&m.hote_a), &m.commun, None).await;
    let direction = m.jeton(Some(&m.hote_a), &m.seul_a, None).await;

    // Direction : refusé (lecture comme écriture).
    let r = m
        .appel("GET", "/etablissement/domaines", &h, Some(&direction), None)
        .await;
    assert_eq!(r.statut, StatusCode::FORBIDDEN);
    let r = m
        .appel(
            "POST",
            "/etablissement/domaines",
            &h,
            Some(&direction),
            Some(json!({"domaine": "x.test.scolia.fr"})),
        )
        .await;
    assert_eq!(r.statut, StatusCode::FORBIDDEN);

    // Audit Fable 1, B-02 : l'administrateur d'établissement LIT ses domaines mais
    // n'en ajoute ni n'en retire aucun (aucune preuve de possession) ; il ne change
    // pas non plus son slug. Le domaine est validé par la plateforme.
    let r = m
        .appel("GET", "/etablissement/domaines", &h, Some(&admin), None)
        .await;
    assert_eq!(r.statut, StatusCode::OK);
    assert_eq!(r.corps["domaines"], json!([m.hote_a]));
    let nouveau = format!("alias-{}", m.hote_a);
    for domaine in [
        format!("{}:443", nouveau.to_uppercase()),
        m.hote_b.clone(),
        "college-b.scolia.fr".to_string(),
        "app.scolia.fr".to_string(),
    ] {
        let r = m
            .appel(
                "POST",
                "/etablissement/domaines",
                &h,
                Some(&admin),
                Some(json!({ "domaine": domaine })),
            )
            .await;
        assert_eq!(r.statut, StatusCode::FORBIDDEN, "ajout par l'admin : {domaine}");
    }
    assert_eq!(m.ent.resoudre_domaine(&nouveau).await.unwrap(), None);
    assert_eq!(m.ent.resoudre_domaine("college-b.scolia.fr").await.unwrap(), None);
    for domaine in [&m.hote_a, &m.hote_b] {
        let r = m
            .appel(
                "DELETE",
                &format!("/etablissement/domaines/{domaine}"),
                &h,
                Some(&admin),
                None,
            )
            .await;
        assert_eq!(r.statut, StatusCode::FORBIDDEN, "retrait par l'admin : {domaine}");
    }
    assert_eq!(m.ent.resoudre_domaine(&m.hote_a).await.unwrap(), Some(m.a));
    assert_eq!(m.ent.resoudre_domaine(&m.hote_b).await.unwrap(), Some(m.b));

    // Slug immuable pour l'admin ; le nom affiché se renomme.
    let fiche = m.ent.etablissement_info(m.a).await.unwrap().expect("fiche A");
    let r = m
        .appel(
            "PUT",
            "/etablissement/fiche",
            &h,
            Some(&admin),
            Some(json!({ "slug": "college-b", "nom": "Collège B" })),
        )
        .await;
    assert_eq!(r.statut, StatusCode::FORBIDDEN, "changement de slug refusé");
    let r = m
        .appel(
            "PUT",
            "/etablissement/fiche",
            &h,
            Some(&admin),
            Some(json!({ "slug": fiche.slug, "nom": "Établissement A renommé" })),
        )
        .await;
    assert_eq!(r.statut, StatusCode::OK, "{:?}", r.corps);
    let apres = m.ent.etablissement_info(m.a).await.unwrap().expect("fiche A");
    assert_eq!(apres.slug, fiche.slug);
    assert_eq!(apres.nom, "Établissement A renommé");

    // Opérateur de la plateforme : ajout normalisé, conflit, hôtes réservés, retrait.
    assert_eq!(
        m.ent
            .ajouter_domaine_plateforme(m.a, &format!("{}:443", nouveau.to_uppercase()))
            .await
            .unwrap(),
        nouveau
    );
    assert_eq!(m.ent.resoudre_domaine(&nouveau).await.unwrap(), Some(m.a));
    assert!(matches!(
        m.ent.ajouter_domaine_plateforme(m.a, &m.hote_b).await,
        Err(ent_db::EntDbError::Domain(domain_eleve::DomainError::Conflit))
    ));
    for reserve in ["scolia.fr", "app.scolia.fr", "www.scolia.fr", "pas un domaine"] {
        assert!(
            matches!(
                m.ent.ajouter_domaine_plateforme(m.a, reserve).await,
                Err(ent_db::EntDbError::Domain(domain_eleve::DomainError::ValeurInvalide))
            ),
            "{reserve}"
        );
    }
    assert!(m.ent.retirer_domaine_plateforme(m.a, &nouveau).await.unwrap());
    assert!(!m.ent.retirer_domaine_plateforme(m.a, &m.hote_b).await.unwrap());
    assert_eq!(m.ent.resoudre_domaine(&nouveau).await.unwrap(), None);
    assert_eq!(m.ent.resoudre_domaine(&m.hote_b).await.unwrap(), Some(m.b));

    // Journalisé, acteur « plateforme ».
    let n: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_journal_modif
          WHERE etablissement_id = $1
            AND action IN ('etablissement_domaine_ajoute', 'etablissement_domaine_retire')
            AND actor_user_id IS NULL
            AND payload->>'acteur' = 'plateforme'",
    )
    .bind(m.a)
    .fetch_one(&m.pool)
    .await
    .unwrap();
    assert!(
        n.0 >= 3,
        "ajouts (monde + alias) et retrait journalisés : {}",
        n.0
    );

    // Établissement désactivé : son adresse ne résout plus.
    sqlx::query("UPDATE ent_etablissement SET actif = false WHERE id = $1")
        .bind(m.b)
        .execute(&m.pool)
        .await
        .unwrap();
    assert_eq!(m.ent.resoudre_domaine(&m.hote_b).await.unwrap(), None);
}
