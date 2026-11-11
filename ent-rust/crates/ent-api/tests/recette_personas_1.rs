//! Recette personas 1 (2026-11-07) : tests d'attaque issus de docs/recette/RECETTE_PERSONAS_1.md.
//! Les tests `#[ignore = "FAILLE OUVERTE …"]` décrivent une faille non corrigée : ils échouent tant
//! qu'elle existe (`cargo test -p ent-api --features labo --test recette_personas_1 -- --ignored`).
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
        // P0 : plus de mot de passe temporaire, le personnel est invité par e-mail : la messagerie
        // doit être « prête » (dry-run, aucun envoi réel) pour que la création aboutisse.
        std::env::set_var("ENT_MAIL_DRY_RUN", "1");
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
    domain_eleve::horloge::jour_paris().to_string()
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

    /// Classe mixte : groupe « 4B LV2 allemand » (Harry et Hermione, pas Ginny) confié à Prof 3,
    /// avec ou sans créneau d'EDT de groupe.
    async fn classe_mixte(avec_creneau: bool) -> Option<Scene> {
        let mut s = Scene::nouvelle().await?;
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
        let m0 = m
            .json
            .as_array()
            .and_then(|a| a.first())
            .expect("matière")
            .clone();
        let matiere = uuid(&m0["id"]);
        let libelle = m0["libelle"].as_str().unwrap().to_string();
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
        s.groupe = Some(groupe);
        s.matiere = Some((matiere, libelle.clone()));
        if avec_creneau {
            let c = s
                .req(
                    "POST",
                    "/edt/admin/creneaux",
                    Some("Direction"),
                    Some(&json!({
                        "classe":"4B","groupe_id":groupe,"matiere_libelle":libelle,
                        "prof_user_id":prof3,"jour_semaine":jour_semaine_courant(),
                        "heure_debut":"10:00:00","heure_fin":"11:00:00","salle":null,
                        "site":"principal","semaine_ab":"toutes"
                    })),
                )
                .await;
            assert_eq!(c.statut, StatusCode::CREATED, "créneau : {}", c.texte);
        }
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

// ───────────────────────── Failles corrigées ─────────────────────────

// RP1-23 : avec un créneau de groupe, Prof 3 lisait notes/absences/… de Ginny (200), hors groupe.
#[test]
fn rp1_c23_creneau_de_groupe_n_ouvre_pas_la_classe() {
    lance(async {
        let s = scene!(Scene::classe_mixte(true));
        let ginny = s.eleve("Ginny");
        for suffixe in ["notes", "absences", "appreciations", "punitions"] {
            let r = s
                .req(
                    "GET",
                    &format!("/eleves/{ginny}/{suffixe}"),
                    Some("Prof 3"),
                    None,
                )
                .await;
            assert!(
                r.est_refus(),
                "Prof 3 lit /{suffixe} de Ginny : {} {}",
                r.statut,
                r.texte
            );
        }
        let harry = s.eleve("Harry");
        let r = s
            .req(
                "GET",
                &format!("/eleves/{harry}/notes"),
                Some("Prof 3"),
                None,
            )
            .await;
        assert_eq!(
            r.statut,
            StatusCode::OK,
            "contrôle positif Harry : {}",
            r.texte
        );
    });
}

// RP1-22 : la préparation du conseil montrait Ginny (hors groupe) au professeur du groupe.
#[test]
fn rp1_c22_conseil_borne_au_groupe() {
    lance(async {
        for avec_creneau in [false, true] {
            let s = scene!(Scene::classe_mixte(avec_creneau));
            let p = s
                .req("GET", "/evaluations/periodes", Some("Direction"), None)
                .await;
            // T0 (démo, fermée, juin-août) ne contient aucun élève : on prend la période ouverte (T1).
            let periode = uuid(
                &p.json
                    .as_array()
                    .and_then(|a| a.iter().find(|x| x["statut"] == "ouverte"))
                    .expect("période ouverte")["id"],
            );
            let c = s
                .req(
                    "POST",
                    "/evaluations/conseils",
                    Some("Direction"),
                    Some(
                        &json!({"classe_id":s.classe_4b,"periode_id":periode,"jour":"2026-12-15",
                        "heure":"17:00:00","lieu":"Salle 12"}),
                    ),
                )
                .await;
            assert_eq!(c.statut, StatusCode::CREATED, "conseil : {}", c.texte);
            let id = uuid(&c.json["id"]);
            let ids = |r: &Rep| -> Vec<String> {
                r.json["eleves"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|e| e["eleve_id"].as_str().unwrap_or("").to_string())
                    .collect()
            };
            let (harry, hermione, ginny) = (
                s.eleve("Harry").to_string(),
                s.eleve("Hermione").to_string(),
                s.eleve("Ginny").to_string(),
            );
            let r = s
                .req(
                    "GET",
                    &format!("/evaluations/conseils/{id}/preparation"),
                    Some("Prof 3"),
                    None,
                )
                .await;
            assert_eq!(r.statut, StatusCode::OK, "préparation Prof 3 : {}", r.texte);
            let v = ids(&r);
            assert!(
                v.contains(&harry) && v.contains(&hermione),
                "créneau={avec_creneau} : {v:?}"
            );
            assert!(
                !v.contains(&ginny),
                "créneau={avec_creneau} : Ginny visible : {v:?}"
            );
            let d = s
                .req(
                    "GET",
                    &format!("/evaluations/conseils/{id}/preparation"),
                    Some("Direction"),
                    None,
                )
                .await;
            let v = ids(&d);
            assert!(
                v.contains(&harry) && v.contains(&hermione) && v.contains(&ginny),
                "la direction voit les trois : {v:?}"
            );
        }
    });
}

// ───────────────────────── À corriger juste après (tests normaux) ─────────────────────────

// RP1-24 : l'appel d'un cours de groupe listait Ginny et acceptait de la marquer absente (204).
#[test]
fn rp1_c24_appel_de_groupe_borne_au_groupe() {
    lance(async {
        let s = scene!(Scene::classe_mixte(true));
        let jour = aujourdhui();
        let m = s
            .req(
                "GET",
                &format!("/edt/mes-creneaux?jour={jour}"),
                Some("Prof 3"),
                None,
            )
            .await;
        assert!(m.statut.is_success(), "mes-creneaux : {}", m.texte);
        let c = m
            .json
            .as_array()
            .and_then(|a| a.iter().find(|c| c["classe"] == "4B"))
            .unwrap_or_else(|| panic!("créneau de Prof 3 absent : {}", m.texte))
            .clone();
        let cid = if c["creneau_id"].is_null() {
            c["id"].clone()
        } else {
            c["creneau_id"].clone()
        };
        let o = s
            .req(
                "POST",
                "/appels",
                Some("Prof 3"),
                Some(&json!({"creneau_id":cid,"jour":jour})),
            )
            .await;
        assert_eq!(
            o.statut,
            StatusCode::CREATED,
            "ouverture de l'appel : {}",
            o.texte
        );
        let id = uuid(&o.json["id"]);
        let g = s
            .req("GET", &format!("/appels/{id}"), Some("Prof 3"), None)
            .await;
        assert!(g.statut.is_success(), "lecture de l'appel : {}", g.texte);
        let lignes: Vec<String> = g.json["lignes"]
            .as_array()
            .expect("lignes")
            .iter()
            .map(|l| l["eleve_id"].as_str().unwrap_or("").to_string())
            .collect();
        let ginny = s.eleve("Ginny");
        assert!(
            lignes.contains(&s.eleve("Harry").to_string()),
            "Harry attendu : {lignes:?}"
        );
        assert!(
            !lignes.contains(&ginny.to_string()),
            "Ginny dans l'appel du groupe : {lignes:?}"
        );
        let m = s
            .req(
                "POST",
                &format!("/appels/{id}/lignes"),
                Some("Prof 3"),
                Some(&json!({"eleve_id":ginny,"etat":"absent"})),
            )
            .await;
        assert!(
            !m.statut.is_success(),
            "Ginny marquée absente par Prof 3 : {}",
            m.statut
        );
    });
}

// RP1-10 : Harry (14 ans) signait seul un document à signer (200, statut « signe »).
#[test]
fn rp1_c10_eleve_mineur_ne_signe_pas_seul() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let harry = s.eleve("Harry");
        let l = s
            .req(
                "GET",
                &format!("/eleves/{harry}/docs-a-signer"),
                Some("Direction"),
                None,
            )
            .await;
        let a_signer = l.json.as_array().and_then(|a| {
            a.iter()
                .find(|d| d["statut"] != "signe" && d["statut"] != "annule")
                .cloned()
        });
        let id = match a_signer {
            Some(d) => uuid(&d["id"]),
            None => {
                let p = s
                    .req(
                        "POST",
                        &format!("/eleves/{harry}/docs-a-signer"),
                        Some("Direction"),
                        Some(&json!({"code":"reglement-rp1","titre":"Règlement intérieur",
                            "corps":"Je m'engage à respecter le règlement.","annee_scolaire":"2026-2027"})),
                    )
                    .await;
                assert_eq!(p.statut, StatusCode::CREATED, "publication : {}", p.texte);
                uuid(&p.json["id"])
            }
        };
        let e = s
            .req(
                "POST",
                &format!("/docs-a-signer/{id}/signer"),
                Some("Eleve - Harry"),
                None,
            )
            .await;
        assert!(
            !e.statut.is_success(),
            "Harry mineur a signé seul : {} {}",
            e.statut,
            e.texte
        );
        let p = s
            .req(
                "POST",
                &format!("/docs-a-signer/{id}/signer"),
                Some("Lily Potter"),
                None,
            )
            .await;
        assert_eq!(
            p.statut,
            StatusCode::OK,
            "Lily doit pouvoir signer : {}",
            p.texte
        );
    });
}

// ───────────────────────── Failles ouvertes ─────────────────────────

// RP1-03 : Prof 3 (groupe de 2, sans créneau) → fiche de classe 4B et trombi avec Ginny (200).
#[test]
fn rp1_c03_classe_et_trombi_bornes_au_groupe() {
    lance(async {
        let s = scene!(Scene::classe_mixte(false));
        let ginny = s.eleve("Ginny");
        let c = s
            .req(
                "GET",
                &format!("/referentiel/classes/{}", s.classe_4b),
                Some("Prof 3"),
                None,
            )
            .await;
        let vu = c.statut.is_success() && c.json["eleves"].to_string().contains(&ginny.to_string());
        assert!(
            !vu,
            "Ginny dans /referentiel/classes/4B pour Prof 3 : {}",
            c.texte
        );
        let t = s
            .req("GET", "/classes/4B/trombi", Some("Prof 3"), None)
            .await;
        let vu = t.statut.is_success()
            && (t.texte.contains(&ginny.to_string()) || t.texte.contains("Ginny"));
        assert!(!vu, "Ginny dans le trombi 4B pour Prof 3 : {}", t.texte);
    });
}

// RP1-04 : Prof 5 (sans classe) → chronologie et photo de Harry en 200.
#[test]
fn rp1_c04_professeur_sans_classe_ne_lit_pas_la_chronologie() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let harry = s.eleve("Harry");
        for chemin in [
            format!("/dossier/eleves/{harry}/chronologie"),
            format!("/eleves/{harry}/photo"),
        ] {
            let r = s.req("GET", &chemin, Some("Prof 5"), None).await;
            assert!(r.est_refus(), "Prof 5 lit {chemin} : {}", r.statut);
        }
    });
}

// RP1-07 : Compta → journal, alertes CPE, dispenses VS (« asthme »), conseils de Harry en 200.
#[test]
fn rp1_c07_comptabilite_hors_metier() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let harry = s.eleve("Harry");
        for suffixe in ["journal", "alertes-cpe", "dispenses-vs", "conseils"] {
            let r = s
                .req(
                    "GET",
                    &format!("/eleves/{harry}/{suffixe}"),
                    Some("Compta"),
                    None,
                )
                .await;
            assert!(
                r.est_refus(),
                "Compta lit /{suffixe} de Harry : {}",
                r.statut
            );
        }
    });
}

// RP1-11 : James (père, autre foyer) lisait le fil privé ouvert par Lily (200 avec le message).
#[test]
fn rp1_c11_parent_separe_ne_lit_pas_le_fil_de_l_autre_parent() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let harry = s.eleve("Harry");
        let f = s
            .req(
                "POST",
                "/foyers",
                Some("Administratif"),
                Some(&json!({"libelle":"Foyer James Potter"})),
            )
            .await;
        assert!(f.statut.is_success(), "foyer : {} {}", f.statut, f.texte);
        let foyer = uuid(&f.json["id"]);
        let email = format!("james-{}@ent.lab", Uuid::new_v4());
        let l = s
            .req(
                "POST",
                &format!("/foyers/{foyer}/liens"),
                Some("Administratif"),
                Some(&json!({"eleve_id":harry,"email":email,"display_name":"James Potter",
                    "lien":"père","peut_voir_notes":true,"peut_voir_vs":true,
                    "peut_justifier_absence":true,"peut_autoriser_sorties":false,"est_payeur":false})),
            )
            .await;
        assert_eq!(l.statut, StatusCode::CREATED, "lien : {}", l.texte);
        // Pas de mot de passe connu (RP1-13) : on copie celui de Lily dans la base de test.
        let pool = s.pool().await;
        let n = sqlx::query(
            "UPDATE ent_user SET password_hash = (SELECT password_hash FROM ent_user WHERE email = $1) WHERE email = $2",
        )
        .bind(s.email("Lily Potter"))
        .bind(&email)
        .execute(&pool)
        .await
        .expect("mot de passe de James")
        .rows_affected();
        pool.close().await;
        assert_eq!(n, 1, "compte de James introuvable");
        let jeton_james = s.connecter(&email).await;

        let o = s
            .req(
                "POST",
                "/messages/fils",
                Some("Lily Potter"),
                Some(
                    &json!({"eleve_id":harry,"sujet":"Situation familiale — confidentiel",
                    "message":"Nouvelle adresse : 4 rue des Lilas"}),
                ),
            )
            .await;
        assert_eq!(o.statut, StatusCode::CREATED, "fil de Lily : {}", o.texte);
        let fil = o.json["fil"]["id"].as_str().expect("id du fil").to_string();

        let liste = s
            .appel("GET", "/messages/fils", Some(&jeton_james), None)
            .await;
        assert!(
            !liste.texte.contains(&fil),
            "le fil de Lily est listé pour James : {}",
            liste.texte
        );
        let g = s
            .appel(
                "GET",
                &format!("/messages/fils/{fil}"),
                Some(&jeton_james),
                None,
            )
            .await;
        assert!(
            g.est_refus(),
            "James lit le fil de Lily : {} {}",
            g.statut,
            g.texte
        );
    });
}

// RP1-20 : Rogue lit « asthme » (dispenses-vs) et le type de dispositif « pap » (dossier) de Harry.
#[test]
fn rp1_c20_professeur_ne_voit_ni_motif_medical_ni_type_de_dispositif() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let harry = s.eleve("Harry");
        // Les données doivent exister, sinon le test ne prouve rien.
        let d = s
            .req(
                "GET",
                &format!("/eleves/{harry}/dispenses-vs"),
                Some("Direction"),
                None,
            )
            .await;
        assert!(
            d.texte.contains("asthme"),
            "dispense « asthme » non semée : {}",
            d.texte
        );
        let dos = s
            .req(
                "GET",
                &format!("/eleves/{harry}/dossier"),
                Some("Direction"),
                None,
            )
            .await;
        assert!(
            dos.texte.contains("\"kind\":\"pap\""),
            "dispositif PAP non semé : {}",
            dos.texte
        );
        let r = s
            .req(
                "GET",
                &format!("/eleves/{harry}/dispenses-vs"),
                Some("Prof 1"),
                None,
            )
            .await;
        assert!(
            !r.texte.contains("asthme"),
            "Rogue lit le motif médical : {}",
            r.texte
        );
        let r = s
            .req(
                "GET",
                &format!("/eleves/{harry}/dossier"),
                Some("Prof 1"),
                None,
            )
            .await;
        assert!(
            !r.texte.contains("\"kind\":\"pap\""),
            "Rogue lit le type de dispositif : {}",
            r.texte
        );
    });
}

// RP1-27 : Molly → créneaux de rencontre de Ginny : Flitwick (hors groupe) proposé.
#[test]
fn rp1_c27_rencontre_sans_prof_hors_groupe() {
    lance(async {
        let s = scene!(Scene::classe_mixte(true));
        let prof3 = s.user("Prof 3");
        let r = s
            .req(
                "POST",
                "/rencontres",
                Some("Direction"),
                Some(&json!({"titre":"Rencontres 4B","classe_id":s.classe_4b})),
            )
            .await;
        assert_eq!(r.statut, StatusCode::CREATED, "rencontre : {}", r.texte);
        let id = uuid(&r.json["id"]);
        let c = s
            .req(
                "POST",
                &format!("/rencontres/{id}/creneaux"),
                Some("Direction"),
                Some(
                    &json!({"enseignant_user_id":prof3,"debut":"2026-12-20T16:00:00Z",
                    "duree_minutes":10,"nombre":2}),
                ),
            )
            .await;
        assert!(c.statut.is_success(), "créneaux : {} {}", c.statut, c.texte);
        let o = s
            .req(
                "POST",
                &format!("/rencontres/{id}/ouvrir"),
                Some("Direction"),
                None,
            )
            .await;
        assert!(
            o.statut.is_success(),
            "ouverture : {} {}",
            o.statut,
            o.texte
        );
        let ginny = s.eleve("Ginny");
        let m = s
            .req(
                "GET",
                &format!("/famille/rencontres/{id}/enfants/{ginny}/creneaux"),
                Some("Molly"),
                None,
            )
            .await;
        let propose = m.statut.is_success()
            && m.json
                .as_array()
                .map(|a| a.iter().any(|e| e["enseignant_user_id"] == json!(prof3)))
                .unwrap_or(false);
        assert!(
            !propose,
            "Flitwick proposé à Molly pour Ginny : {}",
            m.texte
        );
    });
}

// ───────────────────────── Fonctions manquantes livrées ─────────────────────────

// RP1-02 : gérer les membres d'un groupe par l'API (droit, périmètre D-022, journal), sans SQL.
#[test]
fn rp1_c02_gerer_les_membres_d_un_groupe() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let g = s
            .req(
                "POST",
                "/referentiel/groupes",
                Some("Direction"),
                Some(&json!({"code":"4B-ALL","libelle":"4B LV2 allemand","classe_id":s.classe_4b})),
            )
            .await;
        assert!(g.statut.is_success(), "groupe : {}", g.texte);
        let groupe = uuid(&g.json["id"]);
        let m = s.req("GET", "/matieres", Some("Direction"), None).await;
        let matiere = uuid(&m.json.as_array().and_then(|a| a.first()).expect("matière")["id"]);
        let sv = s
            .req(
                "POST",
                "/referentiel/services",
                Some("Direction"),
                Some(&json!({"prof_user_id":s.user("Prof 3"),"groupe_id":groupe,"matiere_id":matiere})),
            )
            .await;
        assert!(sv.statut.is_success(), "service : {}", sv.texte);
        let (harry, ginny) = (s.eleve("Harry"), s.eleve("Ginny"));
        let chemin = format!("/referentiel/groupes/{groupe}/membres");

        // Droit : ni le professeur, ni la famille, ni l'élève, ni la comptabilité n'écrivent.
        for qui in ["Prof 3", "Lily Potter", "Harry", "Compta"] {
            let r = s
                .req("POST", &chemin, Some(qui), Some(&json!({"eleve_id":harry})))
                .await;
            assert!(r.est_refus(), "{qui} ajoute un membre : {}", r.statut);
        }
        // Le secrétariat et la direction gèrent.
        let r = s
            .req("POST", &chemin, Some("Administratif"), Some(&json!({"eleve_id":harry})))
            .await;
        assert_eq!(r.statut, StatusCode::CREATED, "ajout Harry : {}", r.texte);
        let r = s
            .req("POST", &chemin, Some("Direction"), Some(&json!({"eleve_id":harry})))
            .await;
        assert_eq!(r.statut, StatusCode::CONFLICT, "doublon : {}", r.texte);
        let r = s
            .req("POST", &chemin, Some("Direction"), Some(&json!({"eleve_id":ginny})))
            .await;
        assert_eq!(r.statut, StatusCode::CREATED, "ajout Ginny : {}", r.texte);

        // Liste : la direction voit les deux ; le professeur du groupe les deux (ce sont ses
        // élèves) ; un professeur sans lien aucun.
        let l = s.req("GET", &chemin, Some("Direction"), None).await;
        assert_eq!(l.json.as_array().map(Vec::len), Some(2), "membres : {}", l.texte);
        let l = s.req("GET", &chemin, Some("Prof 3"), None).await;
        assert!(l.texte.contains(&ginny.to_string()), "Prof 3 : {}", l.texte);
        let l = s.req("GET", &chemin, Some("Prof 5"), None).await;
        assert!(
            !l.texte.contains(&ginny.to_string()) && !l.texte.contains(&harry.to_string()),
            "Prof 5 voit des membres : {}",
            l.texte
        );
        for qui in ["Lily Potter", "Harry"] {
            let l = s.req("GET", &chemin, Some(qui), None).await;
            assert!(l.est_refus(), "{qui} liste les membres : {}", l.statut);
        }

        // Retrait : l'élève sort du périmètre du professeur de groupe.
        let r = s
            .req(
                "DELETE",
                &format!("{chemin}/{ginny}?date_fin=2000-01-01"),
                Some("Direction"),
                None,
            )
            .await;
        assert_eq!(r.statut, StatusCode::NO_CONTENT, "retrait : {}", r.texte);
        let l = s.req("GET", &chemin, Some("Direction"), None).await;
        assert!(!l.texte.contains(&ginny.to_string()), "Ginny reste : {}", l.texte);
        let r = s
            .req("DELETE", &format!("{chemin}/{ginny}"), Some("Direction"), None)
            .await;
        assert!(r.est_refus(), "second retrait : {}", r.statut);

        // Journal : une ligne par geste, avec l'auteur.
        let j = s
            .req("GET", "/audit/journal?domaine=referentiel", Some("Direction"), None)
            .await;
        assert!(
            j.texte.contains("groupe_membre_ajoute") && j.texte.contains("groupe_membre_retire"),
            "journal de groupe absent : {}",
            j.texte
        );
    });
}

// RP1-06 : profil de base « personnel » sans droit propre ; les rôles D-017 s'ajoutent au-dessus.
#[test]
fn rp1_c06_profil_personnel_minimal() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let harry = s.eleve("Harry");
        let email = format!("accueil-{}@ent.lab", Uuid::new_v4());
        let p = s
            .req(
                "POST",
                "/etablissement/personnel",
                Some("Direction"),
                Some(&json!({"email":email,"display_name":"Accueil Test","role":"personnel"})),
            )
            .await;
        assert_eq!(p.statut, StatusCode::CREATED, "création : {}", p.texte);
        assert!(p.json.get("mot_de_passe_temporaire").is_none(), "aucun secret renvoyé");
        let user = uuid(&p.json["user_id"]);
        // La personne définirait son mot de passe par le lien d'invitation (testé en P0) : ici,
        // SQL de la base de test — elle reçoit le mot de passe de labo pour ouvrir une session.
        let pool = s.pool().await;
        sqlx::query(
            "UPDATE ent_user SET password_hash = (SELECT password_hash FROM ent_user WHERE id = $2) WHERE id = $1",
        )
        .bind(user)
        .bind(s.users["Direction"])
        .execute(&pool)
        .await
        .expect("mot de passe de labo");
        pool.close().await;
        let mdp = s.mdp.clone();
        let l = s
            .appel(
                "POST",
                "/auth/login",
                None,
                Some(&json!({"email":email,"password":mdp,"etablissement_id":s.etab})),
            )
            .await;
        assert_eq!(l.statut, StatusCode::OK, "connexion : {}", l.texte);
        let jeton = l.json["token"].as_str().expect("jeton").to_string();

        // Sans rôle : aucune donnée d'élève, aucun outil d'administration.
        for chemin in [
            "/audit/journal".to_string(),
            "/mails".to_string(),
            "/sms".to_string(),
            "/facturation/factures".to_string(),
            "/exports/onde".to_string(),
            "/documents/grants/pending".to_string(),
            format!("/dossier/eleves/{harry}/responsables"),
            format!("/eleves/{harry}/notes"),
            format!("/eleves/{harry}/absences"),
            format!("/eleves/{harry}/journal"),
            format!("/dossier/eleves/{harry}/chronologie"),
        ] {
            let r = s.appel("GET", &chemin, Some(&jeton), None).await;
            assert!(r.est_refus(), "profil minimal lit {chemin} : {}", r.statut);
        }
        let decl = json!({"eleve_id":harry,"debut_jour":aujourdhui(),"fin_jour":aujourdhui(),
            "motif":"maladie","declarant_nom":"Mme Potter","declarant_qualite":"mère"});
        let r = s
            .appel("POST", "/vs/declarations/accueil", Some(&jeton), Some(&decl))
            .await;
        assert!(r.est_refus(), "déclaration sans rôle : {}", r.statut);

        // Le rôle « Accueil » (absences.declarer) ajoute exactement ce droit.
        let role = s
            .req(
                "POST",
                "/roles",
                Some("Direction"),
                Some(&json!({"nom":"Accueil","droits":[{"action":"absences.declarer","portee":"etablissement"}]})),
            )
            .await;
        assert!(role.statut.is_success(), "rôle : {}", role.texte);
        let rid = uuid(&role.json["id"]);
        let a = s
            .req("PUT", &format!("/roles/{rid}/membres/{user}"), Some("Direction"), Some(&json!({})))
            .await;
        assert_eq!(a.statut, StatusCode::NO_CONTENT, "attribution : {}", a.texte);
        let r = s
            .appel("POST", "/vs/declarations/accueil", Some(&jeton), Some(&decl))
            .await;
        assert_eq!(r.statut, StatusCode::CREATED, "déclaration avec rôle : {}", r.texte);
        for chemin in ["/audit/journal", "/facturation/factures", "/exports/onde"] {
            let r = s.appel("GET", chemin, Some(&jeton), None).await;
            assert!(r.est_refus(), "le rôle Accueil ouvre {chemin} : {}", r.statut);
        }
        let r = s
            .appel("GET", &format!("/eleves/{harry}/notes"), Some(&jeton), None)
            .await;
        assert!(r.est_refus(), "le rôle Accueil lit les notes : {}", r.statut);
    });
}

// RP1-11 (suite) : relances de facture et documents de famille ne passent pas d'un foyer à l'autre.
#[test]
fn rp1_c11_bis_documents_famille_cloisonnes_par_foyer() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let harry = s.eleve("Harry");
        let pool = s.pool().await;
        // Un second foyer (James) et un document de famille déposé par Lily.
        let f = s
            .req("POST", "/foyers", Some("Administratif"), Some(&json!({"libelle":"Foyer James"})))
            .await;
        let foyer = uuid(&f.json["id"]);
        let email = format!("james-{}@ent.lab", Uuid::new_v4());
        let l = s
            .req(
                "POST",
                &format!("/foyers/{foyer}/liens"),
                Some("Administratif"),
                Some(&json!({"eleve_id":harry,"email":email,"display_name":"James Potter","lien":"père",
                    "peut_voir_notes":true,"peut_voir_vs":true,"peut_justifier_absence":true,
                    "peut_autoriser_sorties":false,"est_payeur":false})),
            )
            .await;
        assert_eq!(l.statut, StatusCode::CREATED, "lien : {}", l.texte);
        sqlx::query("UPDATE ent_user SET password_hash = (SELECT password_hash FROM ent_user WHERE email = $1) WHERE email = $2")
            .bind(s.email("Lily Potter"))
            .bind(&email)
            .execute(&pool)
            .await
            .expect("mot de passe de James");
        let jeton_james = s.connecter(&email).await;
        let (lily,): (Uuid,) = sqlx::query_as("SELECT id FROM ent_user WHERE email = $1")
            .bind(s.email("Lily Potter"))
            .fetch_one(&pool)
            .await
            .expect("compte de Lily");
        let doc = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_eleve_document (id, etablissement_id, eleve_id, tiroir, titre, confidentialite, source, created_by_user_id)
             VALUES ($1,$2,$3,'famille','Justificatif de domicile — nouvelle adresse','standard','depot_famille',$4)",
        )
        .bind(doc)
        .bind(s.etab)
        .bind(harry)
        .bind(lily)
        .execute(&pool)
        .await
        .expect("document de Lily");
        // Une relance de facture adressée aux payeurs du foyer de Lily.
        let fil = Uuid::new_v4();
        let (foyer_lily, payeuse): (Uuid, bool) = sqlx::query_as(
            "SELECT l.foyer_id, l.est_payeur FROM ent_eleve_lien l JOIN ent_user u ON u.id = l.user_id
              WHERE u.email = $1 AND l.eleve_id = $2",
        )
        .bind(s.email("Lily Potter"))
        .bind(harry)
        .fetch_one(&pool)
        .await
        .expect("foyer de Lily");
        assert!(payeuse, "Lily doit être payeuse dans le labo");
        sqlx::query(
            "INSERT INTO ent_msg_fil (id, etablissement_id, eleve_id, sujet, created_by_user_id, foyer_id)
             VALUES ($1,$2,$3,'Relance facture LAB-TEST',$4,$5)",
        )
        .bind(fil)
        .bind(s.etab)
        .bind(harry)
        .bind(s.user("Direction"))
        .bind(foyer_lily)
        .execute(&pool)
        .await
        .expect("fil de relance");
        pool.close().await;
        let liste = s.req("GET", "/messages/fils", Some("Lily Potter"), None).await;
        assert!(liste.texte.contains(&fil.to_string()), "Lily ne voit pas sa relance : {}", liste.texte);
        let liste = s.appel("GET", "/messages/fils", Some(&jeton_james), None).await;
        assert!(!liste.texte.contains(&fil.to_string()), "James voit la relance de Lily : {}", liste.texte);
        let g = s
            .appel("GET", &format!("/messages/fils/{fil}"), Some(&jeton_james), None)
            .await;
        assert!(g.est_refus(), "James lit la relance de Lily : {}", g.statut);
        let chez_lily = s
            .req("GET", &format!("/eleves/{harry}/documents"), Some("Lily Potter"), None)
            .await;
        assert!(chez_lily.texte.contains("nouvelle adresse"), "Lily ne voit pas son document : {}", chez_lily.texte);
        let chez_james = s
            .appel("GET", &format!("/eleves/{harry}/documents"), Some(&jeton_james), None)
            .await;
        assert!(
            !chez_james.texte.contains("nouvelle adresse") && !chez_james.texte.contains(&doc.to_string()),
            "James voit le document de Lily : {}",
            chez_james.texte
        );
    });
}

// RP1-30 : les types de punition existent dès le premier jour (amorçage automatique).
#[test]
fn rp1_c30_types_de_punition_amorces_pour_la_vie_scolaire() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let t = s.req("GET", "/vs/punition-types", Some("CPE"), None).await;
        assert!(t.statut.is_success(), "types : {} {}", t.statut, t.texte);
        assert!(
            t.json.as_array().is_some_and(|a| a.len() >= 4),
            "aucun type de punition pour la CPE le premier jour : {}",
            t.texte
        );
        let encore = s.req("GET", "/vs/punition-types", Some("CPE"), None).await;
        assert_eq!(t.json, encore.json, "l'amorçage doit être idempotent");
    });
}
