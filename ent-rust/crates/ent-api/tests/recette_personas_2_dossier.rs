//! Recette personas 2 : la fiche historique `GET /eleves/{id}/dossier` doit respecter la
//! minimisation A1 (RP2-01). Routeur réel en process, base PostgreSQL locale de test, un
//! établissement de labo frais par scénario.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

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
    groupe: Option<Uuid>,
    matiere: Option<(Uuid, String)>,
}

fn uuid(v: &Value) -> Uuid {
    Uuid::parse_str(v.as_str().unwrap_or_else(|| panic!("uuid attendu : {v}"))).expect("uuid")
}

fn jour_semaine_courant() -> u32 {
    use chrono::Datelike;
    chrono::Utc::now().weekday().number_from_monday()
}

fn aujourdhui() -> String {
    chrono::Utc::now().date_naive().to_string()
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
            groupe: None,
            matiere: None,
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
        let mut b = Request::builder().method(m).uri(chemin);
        if let Some(j) = jeton {
            b = b.header("authorization", format!("Bearer {j}"));
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
}

/// Entrée de `liens` d'un responsable, retrouvée par son nom affiché.
fn entree<'a>(rep: &'a Rep, nom: &str) -> Option<&'a Value> {
    rep.json["liens"]
        .as_array()
        .unwrap_or_else(|| panic!("liens absent : {}", rep.texte))
        .iter()
        .find(|l| l["display_name"].as_str() == Some(nom))
}

fn aucun_payeur(rep: &Rep, qui: &str) {
    for l in rep.json["liens"].as_array().expect("liens") {
        assert_ne!(l["est_payeur"], json!(true), "{qui} lit un payeur : {l}");
        assert!(
            l["part_paiement"].is_null(),
            "{qui} lit une part de paiement : {l}"
        );
    }
}

#[test]
fn rp2_01_dossier_historique_respecte_la_minimisation_a1() {
    rt().block_on(async {
        let Some(s) = Scene::nouvelle().await else {
            return;
        };
        let harry = s.eleve("Harry");
        let chemin = format!("/eleves/{harry}/dossier");

        // 1. Un second parent, autre foyer, créé par le secrétariat.
        let f = s
            .req(
                "POST",
                "/foyers",
                Some("Administratif"),
                Some(&json!({"libelle": "Foyer James"})),
            )
            .await;
        assert!(f.statut.is_success(), "foyer : {} {}", f.statut, f.texte);
        let foyer = uuid(&f.json["id"]);
        let email_james = format!("james-{}@exemple.lab", Uuid::new_v4());
        let l = s
            .req(
                "POST",
                &format!("/foyers/{foyer}/liens"),
                Some("Administratif"),
                Some(&json!({
                    "eleve_id": harry, "email": email_james, "display_name": "James Potter",
                    "lien": "pere", "peut_voir_notes": true, "peut_voir_vs": true,
                    "peut_justifier_absence": true, "peut_autoriser_sorties": false,
                    "est_payeur": false
                })),
            )
            .await;
        assert!(l.statut.is_success(), "lien : {} {}", l.statut, l.texte);
        let pool = s.pool().await;
        sqlx::query(
            "UPDATE ent_user SET password_hash = \
             (SELECT password_hash FROM ent_user WHERE email = $1) WHERE email = $2",
        )
        .bind(s.email("Direction"))
        .bind(&email_james)
        .execute(&pool)
        .await
        .expect("mot de passe de labo");
        pool.close().await;
        let jeton_james = s.connecter(&email_james).await;

        let email_lily = s.email("Lily");

        // 2. James (père séparé) : de Lily, la désignation seulement.
        let rep = s.appel("GET", &chemin, Some(&jeton_james), None).await;
        assert_eq!(rep.statut, StatusCode::OK, "James : {}", rep.texte);
        let lily = entree(&rep, "Lily Potter")
            .unwrap_or_else(|| panic!("Lily absente de la désignation : {}", rep.texte));
        assert_eq!(lily["lien"], json!("mere"), "{lily}");
        assert!(
            lily["email"].is_null(),
            "e-mail de Lily lu par James : {lily}"
        );
        assert!(lily["telephone"].is_null(), "{lily}");
        assert!(lily["user_id"].is_null(), "user_id de Lily lu : {lily}");
        assert!(lily["foyer_id"].is_null(), "foyer de Lily lu : {lily}");
        assert!(lily["foyer_libelle"].is_null(), "{lily}");
        assert_ne!(
            lily["est_payeur"],
            json!(true),
            "payeur de Lily lu : {lily}"
        );
        assert!(lily["part_paiement"].is_null(), "{lily}");
        assert!(
            !rep.texte.contains(&email_lily),
            "l'e-mail de Lily fuit dans la réponse : {}",
            rep.texte
        );
        // Son propre lien reste entier.
        let moi = entree(&rep, "James Potter").expect("lien de James");
        assert_eq!(moi["email"], json!(email_james), "{moi}");
        assert_eq!(moi["foyer_id"], json!(foyer), "{moi}");

        // 3. Professeur de la classe et CPE : pas de payeurs.
        for acteur in ["Prof 1", "CPE"] {
            let rep = s.req("GET", &chemin, Some(acteur), None).await;
            assert_eq!(rep.statut, StatusCode::OK, "{acteur} : {}", rep.texte);
            aucun_payeur(&rep, acteur);
        }

        // 4. Non-régression : Direction voit tout de Lily ; Lily se voit en entier.
        let rep = s.req("GET", &chemin, Some("Direction"), None).await;
        let lily = entree(&rep, "Lily Potter").expect("Lily vue par la Direction");
        assert_eq!(lily["email"], json!(email_lily), "{lily}");
        assert_eq!(lily["est_payeur"], json!(true), "{lily}");
        assert_eq!(lily["part_paiement"], json!(100), "{lily}");
        let rep = s.req("GET", &chemin, Some("Lily"), None).await;
        assert_eq!(rep.statut, StatusCode::OK, "Lily : {}", rep.texte);
        let lily = entree(&rep, "Lily Potter").expect("lien de Lily");
        assert_eq!(lily["email"], json!(email_lily), "{lily}");
        assert_eq!(lily["est_payeur"], json!(true), "{lily}");
    });
}
