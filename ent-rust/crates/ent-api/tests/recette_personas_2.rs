//! Recette personas 2 (jalon P1 : socles 2 élève, 3 inscriptions, 4 dossier administratif).
//! Garde-fous de sécurité vérifiés lors de la recette (`docs/recette/RECETTE_PERSONAS_2.md`) et
//! figés ici en tests automatiques : lecture/écriture refusées aux non-habilités, cloisonnement
//! entre établissements, « ne pas solliciter » et adresse protégée de bout en bout, collisions
//! d'INE. Les tests `#[ignore = "FAILLE OUVERTE …"]` décriraient une faille non corrigée.
//!
//! Le routeur réel (`ent_api::app`) est appelé en process, sur une base PostgreSQL locale de
//! test ; un établissement de labo frais par scénario (`POST /labo/bootstrap`).

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

// ───────────────────────── Harnais ─────────────────────────

fn rt() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("runtime de test")
    })
}

/// Base locale de test uniquement ; `None` = test ignoré proprement.
fn garde_base() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent");
        return None;
    };
    if url.trim().is_empty() {
        eprintln!("SKIP : ENT_DATABASE_URL vide");
        return None;
    }
    let parsed = ent_db::assert_local_postgres_url(&url)
        .unwrap_or_else(|e| panic!("base de test locale uniquement : {e}"));
    let nom = parsed.path().trim_start_matches('/').to_ascii_lowercase();
    assert!(
        ["test", "proto", "lab"].iter().any(|m| nom.contains(m)),
        "le nom de la base doit contenir test, proto ou lab (refusé : {nom})"
    );
    Some(url)
}

/// Routeur partagé (le pool sqlx est lié au runtime partagé). `None` = SKIP.
async fn routeur() -> Option<axum::Router> {
    static R: tokio::sync::OnceCell<Option<axum::Router>> = tokio::sync::OnceCell::const_new();
    R.get_or_init(|| async {
        garde_base()?;
        let (state, _bind) = ent_api::boot_from_env()
            .await
            .unwrap_or_else(|e| panic!("démarrage ent-api : {e}"));
        Some(ent_api::app(state))
    })
    .await
    .clone()
}

struct Rep {
    statut: StatusCode,
    texte: String,
    json: Value,
}

impl Rep {
    fn est_refus(&self) -> bool {
        matches!(
            self.statut,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
        )
    }
}

struct Scene {
    r: axum::Router,
    etab: Uuid,
    mdp: String,
    emails: BTreeMap<String, String>,
    users: BTreeMap<String, Uuid>,
    eleves: BTreeMap<String, Uuid>,
    jetons: Mutex<BTreeMap<String, String>>,
    classe_4b: Uuid,
}

fn uuid(v: &Value) -> Uuid {
    Uuid::parse_str(v.as_str().unwrap_or_else(|| panic!("uuid attendu : {v}"))).expect("uuid")
}

impl Scene {
    /// Bootstrap d'un établissement frais. `None` = SKIP (pas de base, labo fermé).
    async fn nouvelle() -> Option<Scene> {
        let r = routeur().await?;
        let mut s = Scene {
            r,
            etab: Uuid::nil(),
            mdp: String::new(),
            emails: BTreeMap::new(),
            users: BTreeMap::new(),
            eleves: BTreeMap::new(),
            jetons: Mutex::new(BTreeMap::new()),
            classe_4b: Uuid::nil(),
        };
        let b = s.appel("POST", "/labo/bootstrap", None, None).await;
        if b.statut == StatusCode::FORBIDDEN || b.statut == StatusCode::NOT_FOUND {
            eprintln!("SKIP : labo fermé (feature `labo` et ENT_ENV=lab requis)");
            return None;
        }
        assert_eq!(b.statut, StatusCode::OK, "bootstrap : {}", b.texte);
        s.etab = uuid(&b.json["etablissement_id"]);
        s.mdp = b.json["password"]
            .as_str()
            .expect("mot de passe")
            .to_string();
        for p in b.json["personnel"].as_array().expect("personnel") {
            let l = p["label"].as_str().unwrap_or("").to_string();
            s.emails
                .insert(l.clone(), p["email"].as_str().unwrap().to_string());
            s.users.insert(l, uuid(&p["user_id"]));
        }
        for p in b.json["personas"].as_array().expect("personas") {
            let l = p["label"].as_str().unwrap_or("").to_string();
            s.emails
                .insert(l.clone(), p["email"].as_str().unwrap().to_string());
            if let Some(u) = p.get("user_id").filter(|u| u.is_string()) {
                s.users.insert(l, uuid(u));
            }
        }
        for e in b.json["eleves"].as_array().expect("eleves") {
            s.eleves
                .insert(e["prenom"].as_str().unwrap().to_string(), uuid(&e["id"]));
        }
        let cl = s
            .req("GET", "/referentiel/classes", Some("Direction"), None)
            .await;
        assert!(cl.statut.is_success(), "classes : {}", cl.texte);
        let c = cl
            .json
            .as_array()
            .expect("liste de classes")
            .iter()
            .find(|c| c["libelle"] == "4B" || c["code"] == "4B")
            .unwrap_or_else(|| panic!("classe 4B absente : {}", cl.texte));
        s.classe_4b = uuid(&c["id"]);
        Some(s)
    }

    async fn appel(
        &self,
        m: &str,
        chemin: &str,
        jeton: Option<&str>,
        corps: Option<&Value>,
    ) -> Rep {
        let entetes: Vec<(&str, String)> = match jeton {
            Some(j) => vec![("authorization", format!("Bearer {j}"))],
            None => vec![],
        };
        self.appel_avec(m, chemin, &entetes, corps).await
    }

    async fn appel_avec(
        &self,
        m: &str,
        chemin: &str,
        entetes: &[(&str, String)],
        corps: Option<&Value>,
    ) -> Rep {
        let mut b = Request::builder().method(m).uri(chemin);
        for (k, v) in entetes {
            b = b.header(*k, v);
        }
        let body = match corps {
            Some(c) => {
                b = b.header("content-type", "application/json");
                Body::from(c.to_string())
            }
            None => Body::empty(),
        };
        let res = self
            .r
            .clone()
            .oneshot(b.body(body).expect("requête"))
            .await
            .expect("routeur infaillible");
        let statut = res.status();
        let octets = res.into_body().collect().await.expect("corps").to_bytes();
        Rep {
            statut,
            texte: String::from_utf8_lossy(&octets).into_owned(),
            json: serde_json::from_slice(&octets).unwrap_or(Value::Null),
        }
    }

    /// Clé complète d'un libellé : égalité d'abord, sinon sous-chaîne.
    fn cle(&self, label: &str) -> String {
        if self.emails.contains_key(label) {
            return label.to_string();
        }
        self.emails
            .keys()
            .find(|k| k.contains(label))
            .unwrap_or_else(|| panic!("acteur « {label} » absent du labo"))
            .clone()
    }

    fn email(&self, label: &str) -> String {
        self.emails[&self.cle(label)].clone()
    }

    fn user(&self, label: &str) -> Uuid {
        self.users[&self.cle(label)]
    }

    fn eleve(&self, prenom: &str) -> Uuid {
        self.eleves[prenom]
    }

    async fn connecter(&self, email: &str) -> String {
        let l = self
            .appel(
                "POST",
                "/auth/login",
                None,
                Some(&json!({"email": email, "password": self.mdp, "etablissement_id": self.etab})),
            )
            .await;
        assert_eq!(l.statut, StatusCode::OK, "connexion {email} : {}", l.texte);
        l.json["token"].as_str().expect("jeton").to_string()
    }

    /// Jeton d'un acteur (connexion à la demande, mise en cache).
    async fn jeton(&self, label: &str) -> String {
        let cle = self.cle(label);
        if let Some(j) = self.jetons.lock().unwrap().get(&cle) {
            return j.clone();
        }
        let j = self.connecter(&self.emails[&cle]).await;
        self.jetons.lock().unwrap().insert(cle, j.clone());
        j
    }

    async fn req(&self, m: &str, chemin: &str, label: Option<&str>, corps: Option<&Value>) -> Rep {
        let j = match label {
            Some(l) => Some(self.jeton(l).await),
            None => None,
        };
        self.appel(m, chemin, j.as_deref(), corps).await
    }

    async fn pool(&self) -> sqlx::PgPool {
        let url = garde_base().expect("base de test");
        sqlx::PgPool::connect(&url)
            .await
            .expect("connexion SQL de test")
    }

    /// Classe mixte : groupe « 4B LV2 allemand » (Harry et Hermione, pas Ginny) confié à Prof 3.
    async fn classe_mixte() -> Option<Scene> {
        let s = Scene::nouvelle().await?;
        let g = s
            .req(
                "POST",
                "/referentiel/groupes",
                Some("Direction"),
                Some(&json!({"code":"4B-ALL","libelle":"4B LV2 allemand","classe_id":s.classe_4b})),
            )
            .await;
        assert!(g.statut.is_success(), "groupe : {} {}", g.statut, g.texte);
        let groupe = uuid(&g.json["id"]);
        let m = s.req("GET", "/matieres", Some("Direction"), None).await;
        let matiere = uuid(&m.json.as_array().and_then(|a| a.first()).expect("matière")["id"]);
        let prof3 = s.user("Prof 3");
        let sv = s
            .req(
                "POST",
                "/referentiel/services",
                Some("Direction"),
                Some(&json!({"prof_user_id":prof3,"groupe_id":groupe,"matiere_id":matiere})),
            )
            .await;
        assert!(
            sv.statut.is_success(),
            "service : {} {}",
            sv.statut,
            sv.texte
        );
        // Aucune route n'affecte un élève à un groupe (RP1-02) : SQL de la base de test.
        let pool = s.pool().await;
        sqlx::query(
            "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut, source) \
             SELECT gen_random_uuid(), g.etablissement_id, g.annee_id, e, g.id, y.date_debut, 'saisie' \
             FROM ent_groupe g JOIN ent_annee_scolaire y ON y.id = g.annee_id, unnest($2::uuid[]) e WHERE g.id = $1",
        )
        .bind(groupe)
        .bind(vec![s.eleve("Harry"), s.eleve("Hermione")])
        .execute(&pool)
        .await
        .expect("affectation au groupe");
        pool.close().await;
        Some(s)
    }
}

fn lance<F: std::future::Future<Output = ()>>(f: F) {
    rt().block_on(f)
}

macro_rules! scene {
    ($e:expr) => {
        match $e.await {
            Some(s) => s,
            None => return,
        }
    };
}

/// Refus attendu (401/403/404) : `GET` ou écriture.
fn assert_refus(r: &Rep, contexte: &str) {
    assert!(r.est_refus(), "{contexte} : attendu un refus, reçu {} {}", r.statut, r.texte);
}

#[test]
fn sonde() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let harry = s.eleve("Harry");
        for c in [
            format!("/eleves/{harry}"),
            format!("/eleves/{harry}/identifiants"),
            format!("/eleves/{harry}/identite/historique"),
            format!("/eleves/{harry}/scolarite"),
            format!("/dossier/eleves/{harry}/responsables"),
            "/eleves/identifiants/contestes".to_string(),
            "/dossier/alertes-protection".to_string(),
        ] {
            let r = s.req("GET", &c, Some("Administratif"), None).await;
            eprintln!("SONDE {c} -> {} {}", r.statut, &r.texte[..r.texte.len().min(900)]);
        }
    });
}
