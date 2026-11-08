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
    assert!(
        r.est_refus(),
        "{contexte} : attendu un refus, reçu {} {}",
        r.statut,
        r.texte
    );
}

/// Responsables d'un élève lus par la direction (liste JSON).
async fn responsables_de(s: &Scene, eleve: Uuid) -> Vec<Value> {
    let r = s
        .req(
            "GET",
            &format!("/dossier/eleves/{eleve}/responsables"),
            Some("Direction"),
            None,
        )
        .await;
    assert_eq!(r.statut, StatusCode::OK, "responsables : {}", r.texte);
    r.json.as_array().expect("liste").clone()
}

fn trouve<'a>(liste: &'a [Value], prenom_ou_nom: &str) -> &'a Value {
    liste
        .iter()
        .find(|r| {
            r["personne"]["nom"]
                .as_str()
                .is_some_and(|n| n.contains(prenom_ou_nom))
                || r["personne"]["prenom"]
                    .as_str()
                    .is_some_and(|n| n.contains(prenom_ou_nom))
        })
        .unwrap_or_else(|| panic!("responsable « {prenom_ou_nom} » absent"))
}

// ───────────────────────── Tests ─────────────────────────

#[test]
fn rp2_t01_famille_ne_lit_ni_historique_ni_identifiants() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let harry = s.eleve("Harry");
        // Contrôle positif : l'élève existe et le secrétariat lit l'historique.
        let ok = s
            .req(
                "GET",
                &format!("/eleves/{harry}/identite/historique"),
                Some("Administratif"),
                None,
            )
            .await;
        assert_eq!(ok.statut, StatusCode::OK, "contrôle positif : {}", ok.texte);
        for qui in ["Lily Potter", "Narcissa"] {
            for suffixe in ["identite/historique", "identifiants"] {
                let r = s
                    .req(
                        "GET",
                        &format!("/eleves/{harry}/{suffixe}"),
                        Some(qui),
                        None,
                    )
                    .await;
                assert_refus(&r, &format!("{qui} lit /{suffixe} de Harry"));
            }
        }
        for suffixe in ["fiche", "responsables", "chronologie"] {
            let r = s
                .req(
                    "GET",
                    &format!("/dossier/eleves/{harry}/{suffixe}"),
                    Some("Narcissa"),
                    None,
                )
                .await;
            assert_refus(&r, &format!("Narcissa lit /dossier/.../{suffixe} de Harry"));
        }
    });
}

#[test]
fn rp2_t02_ne_pas_solliciter_et_adresse_protegee_de_bout_en_bout() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let adresse = format!("12 rue Cachée-{}", Uuid::new_v4());
        let reference = format!("JAF-{}", Uuid::new_v4());
        let email_marc = format!("marc-{}@exemple.lab", Uuid::new_v4());

        // Campagne d'admission ouverte par le secrétariat.
        let c = s
            .req(
                "POST",
                "/inscriptions/campagnes",
                Some("Administratif"),
                Some(
                    &json!({"nature":"admission","libelle":"Admissions 2027-2028",
                    "annee_cible":"2027-2028","date_ouverture":"2026-11-01",
                    "date_cloture":"2027-06-30","message_famille":null,
                    "niveaux":[{"code":"4","capacite":30}],"pieces":[]}),
                ),
            )
            .await;
        assert!(c.statut.is_success(), "campagne : {} {}", c.statut, c.texte);
        let camp = uuid(&c.json["id"]);
        let o = s
            .req(
                "POST",
                &format!("/inscriptions/campagnes/{camp}/ouvrir"),
                Some("Administratif"),
                None,
            )
            .await;
        assert!(
            o.statut.is_success(),
            "ouverture : {} {}",
            o.statut,
            o.texte
        );

        // Lily (parent connecté) obtient un jeton de préinscription depuis sa session.
        let j = s
            .req("POST", "/preinscription/session", Some("Lily Potter"), None)
            .await;
        assert!(j.statut.is_success(), "jeton : {} {}", j.statut, j.texte);
        let jeton = j.json["jeton"].as_str().expect("jeton").to_string();

        let corps = json!({"campagne_id":camp,"utiliser_mes_infos":false,"enregistrer_profil":false,
            "eleve":{"nom":"Martin","prenom":"Rose","date_naissance":"2013-04-02","sexe":"F",
                "ine":null,"niveau_demande":"4","regime_demande":"demi_pension",
                "etablissement_origine":null},
            "responsable":{"nom":"Potter","prenom":"Lily","civilite":"mme",
                "email":s.email("Lily Potter"),"telephone":"0611111111","adresse":adresse,
                "lien":"mere","meme_foyer":true,"autorite_parentale":true,
                "reserve":{"adresse_a_proteger":true,"ne_pas_solliciter":false,
                    "ne_pas_solliciter_ref":null,"langue":null,"besoin_interprete":false,
                    "organisme":null,"nature_code":null}},
            "second_parent":{"nom":"Martin","prenom":"Marc","civilite":"m","email":email_marc,
                "telephone":"0622222222","adresse":"3 avenue du Père","lien":"pere",
                "meme_foyer":false,"autorite_parentale":true,
                "reserve":{"adresse_a_proteger":false,"ne_pas_solliciter":true,
                    "ne_pas_solliciter_ref":reference,"langue":null,"besoin_interprete":false,
                    "organisme":null,"nature_code":null}}});
        let d = s
            .appel_avec(
                "POST",
                "/preinscription/dossiers",
                &[("x-preinscription", jeton.clone())],
                Some(&corps),
            )
            .await;
        assert_eq!(d.statut, StatusCode::CREATED, "dépôt : {}", d.texte);
        // (a) la réponse faite à la famille ne contient pas l'adresse protégée.
        assert!(
            !d.texte.contains(&adresse),
            "l'adresse protégée est renvoyée à la famille : {}",
            d.texte
        );
        let dossier = uuid(&d.json["id"]);

        // Instruction, décision (direction), correspondances, validation.
        let st = s
            .req(
                "POST",
                &format!("/inscriptions/dossiers/{dossier}/statut"),
                Some("Administratif"),
                Some(&json!({"statut":"en_instruction","motif":null})),
            )
            .await;
        assert!(
            st.statut.is_success(),
            "instruction : {} {}",
            st.statut,
            st.texte
        );
        let de = s
            .req(
                "POST",
                &format!("/inscriptions/dossiers/{dossier}/decision"),
                Some("Direction"),
                Some(&json!({"decision":"admis","motif":null})),
            )
            .await;
        assert!(
            de.statut.is_success(),
            "décision : {} {}",
            de.statut,
            de.texte
        );
        let co = s
            .req(
                "GET",
                &format!("/inscriptions/dossiers/{dossier}/correspondances"),
                Some("Direction"),
                None,
            )
            .await;
        assert_eq!(co.statut, StatusCode::OK, "correspondances : {}", co.texte);
        let resp: Vec<Value> = co.json["responsables"]
            .as_array()
            .expect("responsables")
            .iter()
            .map(|r| {
                json!({"responsable_id":r["responsable_id"],
                    "personne":{"choix":"nouvelle"},"foyer":null})
            })
            .collect();
        assert_eq!(resp.len(), 2, "deux responsables attendus : {}", co.texte);
        let v = s
            .req(
                "POST",
                &format!("/inscriptions/dossiers/{dossier}/valider"),
                Some("Direction"),
                Some(
                    &json!({"classe_id":null,"regime":null,"niveau":"4","parcours":null,
                    "responsables":resp,"eleve":{"choix":"nouveau"}}),
                ),
            )
            .await;
        assert!(
            v.statut.is_success(),
            "validation : {} {}",
            v.statut,
            v.texte
        );
        let nouvel = uuid(&v.json["eleve_id"]);

        // (b) aucune invitation pour le second parent (une pour Lily est permise).
        let pool = s.pool().await;
        let inv: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_inscription_invitation x WHERE x::text ILIKE $1",
        )
        .bind(format!("%{email_marc}%"))
        .fetch_one(&pool)
        .await
        .expect("invitations");
        assert_eq!(
            inv, 0,
            "invitation émise pour le parent « ne pas solliciter »"
        );

        // (c) les deux personnes apparaissent dans les alertes de la direction.
        let liens = responsables_de(&s, nouvel).await;
        let lily = trouve(&liens, "Lily");
        let marc = trouve(&liens, "Marc");
        let lily_p = lily["personne"]["id"].clone();
        let marc_p = marc["personne"]["id"].clone();
        let al = s
            .req(
                "GET",
                "/dossier/alertes-protection",
                Some("Direction"),
                None,
            )
            .await;
        assert_eq!(al.statut, StatusCode::OK, "alertes : {}", al.texte);
        let a_traiter = al.json["adresses_a_traiter"].as_array().expect("adresses");
        assert!(
            a_traiter.iter().any(|x| x["personne_id"] == lily_p),
            "Lily absente de adresses_a_traiter : {}",
            al.texte
        );
        let nps = al.json["ne_pas_solliciter_a_confirmer"]
            .as_array()
            .expect("nps");
        assert!(
            nps.iter().any(|x| x["personne_id"] == marc_p),
            "Marc absent de ne_pas_solliciter_a_confirmer : {}",
            al.texte
        );

        // (d) ni l'adresse ni la référence judiciaire en clair dans aucune table.
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT table_name::text FROM information_schema.tables \
             WHERE table_schema='public' AND table_type='BASE TABLE'",
        )
        .fetch_all(&pool)
        .await
        .expect("tables");
        assert!(
            tables.len() > 50,
            "liste de tables suspecte : {}",
            tables.len()
        );
        for marqueur in [&adresse, &reference] {
            for t in &tables {
                let n: i64 = sqlx::query_scalar(&format!(
                    "SELECT count(*) FROM \"{t}\" x WHERE x::text ILIKE $1"
                ))
                .bind(format!("%{marqueur}%"))
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|e| panic!("scan de {t} : {e}"));
                assert_eq!(n, 0, "« {marqueur} » en clair dans la table {t}");
            }
        }
        pool.close().await;

        // (e) vue A1 : ni le secrétariat ni le professeur n'obtiennent l'adresse de Lily.
        for qui in ["Administratif", "Prof 1"] {
            let r = s
                .req(
                    "GET",
                    &format!("/dossier/eleves/{nouvel}/responsables"),
                    Some(qui),
                    None,
                )
                .await;
            assert!(
                !r.texte.contains(&adresse),
                "{qui} obtient l'adresse protégée : {}",
                r.texte
            );
            if r.statut.is_success() {
                let l = trouve(r.json.as_array().expect("liste"), "Lily");
                assert!(
                    l["personne"]["adresse"].is_null(),
                    "{qui} : adresse de Lily non nulle : {}",
                    l["personne"]
                );
            } else {
                assert_refus(&r, &format!("{qui} sur le nouvel élève"));
            }
        }
    });
}

#[test]
fn rp2_t03_professeur_de_groupe_borne_sur_les_routes_des_socles_2_a_4() {
    lance(async {
        let s = scene!(Scene::classe_mixte());
        for prenom in ["Ginny", "Draco"] {
            let id = s.eleve(prenom);
            let chemins = [
                format!("/eleves/{id}"),
                format!("/dossier/eleves/{id}/fiche"),
                format!("/dossier/eleves/{id}/responsables"),
                format!("/dossier/eleves/{id}/chronologie"),
                format!("/dossier/eleves/{id}/contacts"),
                format!("/dossier/eleves/{id}/interdictions"),
                format!("/eleves/{id}/scolarite"),
                format!("/eleves/{id}/identifiants"),
                format!("/eleves/{id}/identite/historique"),
                format!("/eleves/{id}/photo"),
                format!("/eleves/{id}/journal"),
            ];
            for c in chemins {
                let r = s.req("GET", &c, Some("Prof 3"), None).await;
                assert_refus(&r, &format!("Prof 3 lit {c} ({prenom})"));
            }
        }
        let harry = s.eleve("Harry");
        for c in [
            format!("/eleves/{harry}"),
            format!("/dossier/eleves/{harry}/fiche"),
        ] {
            let r = s.req("GET", &c, Some("Prof 3"), None).await;
            assert_eq!(
                r.statut,
                StatusCode::OK,
                "contrôle positif {c} : {}",
                r.texte
            );
        }
        for c in [
            format!("/eleves/{harry}/identifiants"),
            format!("/eleves/{harry}/identite/historique"),
        ] {
            let r = s.req("GET", &c, Some("Prof 3"), None).await;
            assert_refus(&r, &format!("Prof 3 lit {c} (Harry)"));
        }
        let r = s
            .req("GET", "/eleves/search?q=4B", Some("Prof 3"), None)
            .await;
        assert_eq!(r.statut, StatusCode::OK, "recherche : {}", r.texte);
        assert!(
            r.texte.contains(&harry.to_string()),
            "Harry attendu dans la recherche : {}",
            r.texte
        );
        for prenom in ["Ginny", "Draco"] {
            assert!(
                !r.texte.contains(&s.eleve(prenom).to_string()),
                "{prenom} dans la recherche de Prof 3 : {}",
                r.texte
            );
        }
    });
}

#[test]
fn rp2_t04_autre_etablissement_refuse_sur_les_routes_des_socles_2_a_4() {
    lance(async {
        let a = scene!(Scene::nouvelle());
        let b = scene!(Scene::nouvelle());
        assert_ne!(a.etab, b.etab);
        let harry = a.eleve("Harry");
        let liens = responsables_de(&a, harry).await;
        let lily = trouve(&liens, "Lily");
        let lien = uuid(&lily["lien_id"]);
        let personne = uuid(&lily["personne"]["id"]);
        // Aucun dossier d'inscription de A n'est créé ici : les routes d'inscriptions sont
        // couvertes par les tests de séparation du crate.

        let lectures = [
            format!("/eleves/{harry}"),
            format!("/eleves/{harry}/identite/historique"),
            format!("/eleves/{harry}/identifiants"),
            format!("/eleves/{harry}/scolarite"),
            format!("/dossier/eleves/{harry}/fiche"),
            format!("/dossier/eleves/{harry}/responsables"),
            format!("/dossier/eleves/{harry}/contacts"),
            format!("/dossier/eleves/{harry}/interdictions"),
            format!("/dossier/eleves/{harry}/chronologie"),
            format!("/dossier/personnes/{personne}"),
        ];
        // Contrôle positif : la direction de A lit bien ces routes.
        for c in &lectures {
            let r = a.req("GET", c, Some("Direction"), None).await;
            assert_eq!(
                r.statut,
                StatusCode::OK,
                "contrôle positif A {c} : {}",
                r.texte
            );
            let r = b.req("GET", c, Some("Direction"), None).await;
            // Une liste vide (filtrée par établissement) n'est pas une fuite : rien n'est révélé.
            let vide =
                r.statut == StatusCode::OK && r.json.as_array().is_some_and(|a| a.is_empty());
            assert!(
                r.est_refus() || vide,
                "direction de B lit {c} : {} {}",
                r.statut,
                r.texte
            );
        }
        let ecritures: [(&str, String, Value); 5] = [
            (
                "POST",
                format!("/eleves/{harry}/identite/corrections"),
                json!({"champs":{"prenoms":["Harold"]},"motif_code":"faute_de_frappe","motif":"x"}),
            ),
            (
                "POST",
                format!("/eleves/{harry}/identifiants"),
                json!({"systeme":"INE","valeur":"000000000XX"}),
            ),
            (
                "POST",
                format!("/eleves/{harry}/scolarisations/cloturer"),
                json!({"date_sortie":"2026-11-07","motif":"demenagement","destination":{}}),
            ),
            (
                "POST",
                format!("/dossier/liens/{lien}/suspendre"),
                json!({"motif":"motif de test","reference":null}),
            ),
            (
                "POST",
                format!("/dossier/personnes/{personne}/ne-pas-solliciter"),
                json!({"actif":true,"reference":null}),
            ),
        ];
        for (m, c, corps) in &ecritures {
            let r = b.req(m, c, Some("Direction"), Some(corps)).await;
            assert_refus(&r, &format!("direction de B écrit {m} {c}"));
        }
        // Rien n'a changé côté A.
        let e = a
            .req("GET", &format!("/eleves/{harry}"), Some("Direction"), None)
            .await;
        assert_eq!(e.json["prenom"], "Harry", "prénom modifié : {}", e.texte);
        assert_eq!(
            e.json["status"], "actif",
            "scolarité clôturée : {}",
            e.texte
        );
        let liens = responsables_de(&a, harry).await;
        let lily = trouve(&liens, "Lily");
        assert!(lily["suspendu_le"].is_null(), "lien suspendu : {lily}");
        let p = a
            .req(
                "GET",
                &format!("/dossier/personnes/{personne}"),
                Some("Direction"),
                None,
            )
            .await;
        assert!(
            !p.texte.contains("ne_pas_solliciter\":true"),
            "personne passée en ne pas solliciter : {}",
            p.texte
        );
        let ids = a
            .req(
                "GET",
                &format!("/eleves/{harry}/identifiants"),
                Some("Direction"),
                None,
            )
            .await;
        assert!(
            !ids.texte.contains("000000000XX"),
            "INE ajouté : {}",
            ids.texte
        );
    });
}

#[test]
fn rp2_t05_ine_collision_contestee_puis_resolue() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let (harry, hermione) = (s.eleve("Harry"), s.eleve("Hermione"));
        let lu = s
            .req(
                "GET",
                &format!("/eleves/{harry}/identifiants"),
                Some("Administratif"),
                None,
            )
            .await;
        assert_eq!(
            lu.statut,
            StatusCode::OK,
            "identifiants Harry : {}",
            lu.texte
        );
        let ine_harry = lu
            .json
            .as_array()
            .and_then(|a| a.iter().find(|i| i["systeme"] == "INE"))
            .and_then(|i| i["valeur"].as_str())
            .expect("INE de Harry")
            .to_string();

        let chemin = format!("/eleves/{hermione}/identifiants");
        let ajout = |valeur: String| json!({"systeme":"INE","valeur":valeur,"provenance":"saisie"});
        let c = s
            .req(
                "POST",
                &chemin,
                Some("Administratif"),
                Some(&ajout(ine_harry)),
            )
            .await;
        assert_eq!(c.statut, StatusCode::CREATED, "collision : {}", c.texte);
        assert_eq!(c.json["identifiant"]["statut"], "conteste", "{}", c.texte);
        let conteste = uuid(&c.json["identifiant"]["id"]);
        let l = s
            .req(
                "GET",
                "/eleves/identifiants/contestes",
                Some("Administratif"),
                None,
            )
            .await;
        assert_eq!(l.statut, StatusCode::OK, "contestés : {}", l.texte);
        assert!(
            l.texte.contains(&conteste.to_string()),
            "INE contesté absent de la liste : {}",
            l.texte
        );

        let bon = "0123456789A".to_string();
        let d = s
            .req(
                "POST",
                &chemin,
                Some("Administratif"),
                Some(&ajout(bon.clone())),
            )
            .await;
        assert_eq!(d.statut, StatusCode::CREATED, "INE valide : {}", d.texte);
        assert_eq!(d.json["identifiant"]["statut"], "declare", "{}", d.texte);
        let declare = uuid(&d.json["identifiant"]["id"]);
        let cf = s
            .req(
                "POST",
                &format!("{chemin}/{declare}/confirmer"),
                Some("Administratif"),
                Some(&json!({"source":"siecle"})),
            )
            .await;
        assert!(
            cf.statut.is_success(),
            "confirmation : {} {}",
            cf.statut,
            cf.texte
        );
        let rv = s
            .req(
                "POST",
                &format!("{chemin}/{conteste}/revoquer"),
                Some("Administratif"),
                Some(&json!({"motif":"doublon avec un autre élève"})),
            )
            .await;
        assert!(
            rv.statut.is_success(),
            "révocation : {} {}",
            rv.statut,
            rv.texte
        );
        let l = s
            .req(
                "GET",
                "/eleves/identifiants/contestes",
                Some("Administratif"),
                None,
            )
            .await;
        assert!(
            !l.texte.contains(&conteste.to_string()),
            "INE révoqué encore contesté : {}",
            l.texte
        );
        let autre = s
            .req(
                "POST",
                &chemin,
                Some("Administratif"),
                Some(&ajout("9876543210B".into())),
            )
            .await;
        assert_eq!(
            autre.statut,
            StatusCode::CONFLICT,
            "2e INE avec un INE confirmé : {}",
            autre.texte
        );
        let e = s
            .req(
                "GET",
                &format!("/eleves/{hermione}"),
                Some("Direction"),
                None,
            )
            .await;
        assert_eq!(e.json["ine"], bon, "INE de la fiche : {}", e.texte);

        for qui in [
            "Lily Potter",
            "Narcissa",
            "Prof 1",
            "Prof 3",
            "CPE",
            "Compta",
        ] {
            let r = s.req("GET", &chemin, Some(qui), None).await;
            assert_refus(&r, &format!("{qui} lit les identifiants d'Hermione"));
        }
    });
}

#[test]
fn rp2_t06_ecritures_des_socles_2_a_4_refusees_aux_non_habilites() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let harry = s.eleve("Harry");
        let liens = responsables_de(&s, harry).await;
        let lily = trouve(&liens, "Lily");
        let lien = uuid(&lily["lien_id"]);
        let lily_p = uuid(&lily["personne"]["id"]);
        let draco_liens = responsables_de(&s, s.eleve("Draco")).await;
        let autre_p = uuid(&draco_liens[0]["personne"]["id"]);
        assert_ne!(lily_p, autre_p);
        let pirate = format!("pirate-{}@exemple.lab", Uuid::new_v4());
        let ecritures: Vec<(&str, String, Value)> = vec![
            (
                "POST",
                format!("/eleves/{harry}/identite/corrections"),
                json!({"champs":{"prenoms":["Harold"]},"motif_code":"faute_de_frappe","motif":"x"}),
            ),
            (
                "POST",
                format!("/eleves/{harry}/identite/usages"),
                json!({"prenom_usage":"Hal","motif_code":"demande_eleve","motif":"x"}),
            ),
            (
                "POST",
                format!("/eleves/{harry}/identifiants"),
                json!({"systeme":"INE","valeur":"000000000XX"}),
            ),
            (
                "POST",
                format!("/eleves/{harry}/scolarisations/cloturer"),
                json!({"date_sortie":"2026-11-07","motif":"demenagement","destination":{}}),
            ),
            (
                "POST",
                format!("/eleves/{harry}/deces"),
                json!({"date":"2026-11-07"}),
            ),
            (
                "POST",
                format!("/dossier/eleves/{harry}/responsables/ajout"),
                json!({"nouvelle_personne":{"nom":"Pirate","prenom":"X","civilite":null,
                        "email":pirate,"telephone":null,"telephone_2":null,"adresse":null},
                    "lien":{"personne_id":null,"foyer_id":null,"lien":"autre",
                        "autorite_parentale":false,"garde":null,"date_debut":null,
                        "date_fin":null,"peut_voir_notes":true,"peut_voir_vs":true,
                        "peut_justifier_absence":true,"peut_autoriser_sorties":true}}),
            ),
            (
                "POST",
                format!("/dossier/liens/{lien}/suspendre"),
                json!({"motif":"motif de test","reference":null}),
            ),
            (
                "PUT",
                format!("/dossier/personnes/{lily_p}"),
                json!({"nom":"Potter","prenom":"Lily","civilite":null,
                    "email":"pirate@exemple.lab","telephone":null,"telephone_2":null,"adresse":null}),
            ),
            (
                "POST",
                "/dossier/fusions".to_string(),
                json!({"source_id":lily_p,"cible_id":autre_p,"motif":"test"}),
            ),
        ];
        for qui in ["Narcissa", "Eleve - Harry", "Prof 1", "Prof 5", "CPE"] {
            for (m, c, corps) in &ecritures {
                let r = s.req(m, c, Some(qui), Some(corps)).await;
                assert_refus(&r, &format!("{qui} : {m} {c}"));
            }
        }
        // Rien n'a changé.
        let e = s
            .req("GET", &format!("/eleves/{harry}"), Some("Direction"), None)
            .await;
        assert_eq!(e.json["prenom"], "Harry", "{}", e.texte);
        assert_eq!(e.json["status"], "actif", "{}", e.texte);
        let liens = responsables_de(&s, harry).await;
        let lily = trouve(&liens, "Lily");
        assert!(lily["suspendu_le"].is_null(), "lien suspendu : {lily}");
        assert_eq!(lily["personne"]["email"], s.email("Lily Potter"));
    });
}
