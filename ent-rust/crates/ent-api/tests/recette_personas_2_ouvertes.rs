//! Failles et manques ouverts de la recette 2 (`docs/recette/RECETTE_PERSONAS_2.md`) : chaque test
//! décrit un problème non corrigé (décision produit ou chantier trop large) et échoue tant qu'il
//! existe. Lancer avec `-- --ignored` : ils doivent échouer
//! (`cargo test -p ent-api --features labo --test recette_personas_2_ouvertes -- --ignored`).
//!
//! Le routeur réel (`ent_api::app`) est appelé en process, sur une base PostgreSQL locale de
//! test ; un établissement de labo frais par scénario (`POST /labo/bootstrap`).

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine;
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

struct Scene {
    r: axum::Router,
    etab: Uuid,
    mdp: String,
    emails: BTreeMap<String, String>,
    users: BTreeMap<String, Uuid>,
    eleves: BTreeMap<String, Uuid>,
    jetons: Mutex<BTreeMap<String, String>>,
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
        Some(s)
    }

    async fn appel(
        &self,
        m: &str,
        chemin: &str,
        jeton: Option<&str>,
        corps: Option<&Value>,
    ) -> Rep {
        self.appel_en_tetes(m, chemin, jeton, &[], corps).await
    }

    async fn appel_en_tetes(
        &self,
        m: &str,
        chemin: &str,
        jeton: Option<&str>,
        entetes: &[(&str, &str)],
        corps: Option<&Value>,
    ) -> Rep {
        let mut b = Request::builder().method(m).uri(chemin);
        if let Some(j) = jeton {
            b = b.header("authorization", format!("Bearer {j}"));
        }
        for (k, v) in entetes {
            b = b.header(*k, *v);
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

    /// Donne à `email` le mot de passe de la Direction (base de test, comptes créés sans mot de passe).
    async fn mot_de_passe_de_labo(&self, email: &str) {
        let pool = self.pool().await;
        let n = sqlx::query(
            "UPDATE ent_user SET password_hash = (SELECT password_hash FROM ent_user WHERE email = $1) WHERE email = $2",
        )
        .bind(self.email("Direction"))
        .bind(email)
        .execute(&pool)
        .await
        .expect("mot de passe de labo")
        .rows_affected();
        pool.close().await;
        assert_eq!(n, 1, "compte {email} introuvable");
    }

    /// Nombre de personnes (non fusionnées) de l'établissement portant cet e-mail.
    async fn personnes_avec_email(&self, email: &str) -> i64 {
        let pool = self.pool().await;
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_personne \
             WHERE etablissement_id = $1 AND lower(email) = lower($2) AND fusionnee_dans IS NULL",
        )
        .bind(self.etab)
        .bind(email)
        .fetch_one(&pool)
        .await
        .expect("comptage des personnes");
        pool.close().await;
        n
    }

    /// Dépose un petit PDF dans le tiroir administratif de l'élève ; renvoie l'id du document.
    async fn deposer_pdf(&self, eleve: Uuid, titre: &str) -> Uuid {
        let pdf = b"%PDF-1.4\n1 0 obj<</Type/Catalog>>endobj\ntrailer<</Root 1 0 R>>\n%%EOF\n";
        let d = self
            .req(
                "POST",
                &format!("/eleves/{eleve}/documents"),
                Some("Administratif"),
                Some(&json!({
                    "tiroir":"administratif","titre":titre,"code":null,
                    "mime_type":"application/pdf","confidentialite":"restreint",
                    "contenu_base64": base64::engine::general_purpose::STANDARD.encode(pdf)
                })),
            )
            .await;
        assert!(
            d.statut.is_success(),
            "dépôt de la pièce : {} {}",
            d.statut,
            d.texte
        );
        uuid(&d.json["id"])
    }

    /// Ajoute Marc Martin comme responsable de l'élève (chemin « dossier ») ; renvoie (email, personne).
    async fn ajouter_marc(&self, eleve: Uuid) -> (String, Uuid) {
        let email = format!("marc-{}@exemple.lab", Uuid::new_v4());
        let a = self
            .req(
                "POST",
                &format!("/dossier/eleves/{eleve}/responsables/ajout"),
                Some("Administratif"),
                Some(&json!({
                    "nouvelle_personne":{"nom":"Martin","prenom":"Marc","civilite":"m","email":email,
                        "telephone":null,"telephone_2":null,"adresse":null},
                    "lien":{"personne_id":null,"foyer_id":null,"lien":"pere","autorite_parentale":true,
                        "garde":null,"date_debut":null,"date_fin":null,"peut_voir_notes":true,
                        "peut_voir_vs":true,"peut_justifier_absence":true,"peut_autoriser_sorties":false}
                })),
            )
            .await;
        assert!(
            a.statut.is_success(),
            "ajout de Marc : {} {}",
            a.statut,
            a.texte
        );
        (email, uuid(&a.json["personne"]["id"]))
    }

    /// Marque la personne « ne pas solliciter » (Direction).
    async fn ne_pas_solliciter(&self, personne: Uuid) {
        let n = self
            .req(
                "POST",
                &format!("/dossier/personnes/{personne}/ne-pas-solliciter"),
                Some("Direction"),
                Some(&json!({"actif":true,"reference":"ordonnance"})),
            )
            .await;
        assert!(
            n.statut.is_success(),
            "ne pas solliciter : {} {}",
            n.statut,
            n.texte
        );
    }

    /// Ancien chemin : crée un foyer puis y lie `email` à `eleve` (compte parent + personne).
    async fn ancien_chemin_foyer(&self, eleve: Uuid, email: &str) -> Rep {
        let f = self
            .req(
                "POST",
                "/foyers",
                Some("Administratif"),
                Some(&json!({"libelle":"Foyer X"})),
            )
            .await;
        assert!(f.statut.is_success(), "foyer : {} {}", f.statut, f.texte);
        let foyer = uuid(&f.json["id"]);
        self.req(
            "POST",
            &format!("/foyers/{foyer}/liens"),
            Some("Administratif"),
            Some(&json!({
                "eleve_id":eleve,"email":email,"display_name":"Marc Martin","lien":"pere",
                "peut_voir_notes":true,"peut_voir_vs":true,"peut_justifier_absence":true,
                "peut_autoriser_sorties":true,"est_payeur":false
            })),
        )
        .await
    }

    /// Année courante (id) d'après `GET /referentiel/annees-statut`.
    async fn annee_courante(&self) -> Uuid {
        let a = self
            .req("GET", "/referentiel/annees-statut", Some("Direction"), None)
            .await;
        assert!(a.statut.is_success(), "années : {}", a.texte);
        let c = a
            .json
            .as_array()
            .and_then(|v| v.iter().find(|x| x["courante"] == true))
            .unwrap_or_else(|| panic!("année courante absente : {}", a.texte));
        uuid(&c["id"])
    }

    /// Id de l'année « libelle », la créant au besoin (préparation de l'année suivante).
    async fn annee_par_libelle(&self, libelle: &str) -> Uuid {
        for essai in 0..2 {
            let a = self
                .req("GET", "/referentiel/annees-statut", Some("Direction"), None)
                .await;
            if let Some(x) = a
                .json
                .as_array()
                .and_then(|v| v.iter().find(|x| x["libelle"] == libelle))
            {
                return uuid(&x["id"]);
            }
            if essai == 0 {
                let courante = self.annee_courante().await;
                let p = self
                    .req(
                        "POST",
                        &format!("/referentiel/annees/{courante}/preparer-suivante"),
                        Some("Direction"),
                        Some(&json!({})),
                    )
                    .await;
                assert!(
                    p.statut.is_success(),
                    "préparation de l'année suivante : {} {}",
                    p.statut,
                    p.texte
                );
            }
        }
        panic!("année {libelle} introuvable après préparation");
    }

    /// Parcours d'admission 2027-2028 jusqu'à la décision « admis » ; renvoie (dossier, responsable_id).
    async fn admission_admise(&self) -> (Uuid, Uuid) {
        let lily = self.jeton("Parent - Lily Potter").await;
        let camp = self
            .req(
                "POST",
                "/inscriptions/campagnes",
                Some("Administratif"),
                Some(&json!({
                    "nature":"admission","libelle":"Admissions 2027-2028","annee_cible":"2027-2028",
                    "date_ouverture":"2026-11-01","date_cloture":"2027-06-30","message_famille":null,
                    "niveaux":[{"code":"4","capacite":30}],"pieces":[]
                })),
            )
            .await;
        assert!(
            camp.statut.is_success(),
            "campagne : {} {}",
            camp.statut,
            camp.texte
        );
        let camp_id = uuid(&camp.json["id"]);
        let o = self
            .req(
                "POST",
                &format!("/inscriptions/campagnes/{camp_id}/ouvrir"),
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
        let sess = self
            .appel("POST", "/preinscription/session", Some(&lily), None)
            .await;
        assert!(
            sess.statut.is_success(),
            "session de préinscription : {} {}",
            sess.statut,
            sess.texte
        );
        let jeton = sess.json["jeton"].as_str().expect("jeton").to_string();
        let mut corps = json!({
            "campagne_id":camp_id,"utiliser_mes_infos":false,"enregistrer_profil":false,
            "eleve":{"nom":"Martin","prenom":"Rose","date_naissance":"2013-04-02","sexe":"F","ine":null,
                "niveau_demande":"4","regime_demande":"demi_pension","etablissement_origine":null},
            "responsable":{"nom":"Potter","prenom":"Lily","civilite":"mme",
                "email":self.email("Parent - Lily Potter"),"telephone":null,"adresse":null,
                "lien":"mere","meme_foyer":true,"autorite_parentale":true,"reserve":null},
            "second_parent":null
        });
        let mut d = self
            .appel_en_tetes(
                "POST",
                "/preinscription/dossiers",
                None,
                &[("x-preinscription", &jeton)],
                Some(&corps),
            )
            .await;
        if !d.statut.is_success() {
            // `reserve: null` refusé : objet réserve neutre.
            corps["responsable"]["reserve"] = json!({});
            d = self
                .appel_en_tetes(
                    "POST",
                    "/preinscription/dossiers",
                    None,
                    &[("x-preinscription", &jeton)],
                    Some(&corps),
                )
                .await;
        }
        assert_eq!(d.statut, StatusCode::CREATED, "dépôt : {}", d.texte);
        let dossier = uuid(&d.json["id"]);
        let i = self
            .req(
                "POST",
                &format!("/inscriptions/dossiers/{dossier}/statut"),
                Some("Administratif"),
                Some(&json!({"statut":"en_instruction","motif":null})),
            )
            .await;
        assert!(
            i.statut.is_success(),
            "instruction : {} {}",
            i.statut,
            i.texte
        );
        let dec = self
            .req(
                "POST",
                &format!("/inscriptions/dossiers/{dossier}/decision"),
                Some("Direction"),
                Some(&json!({"decision":"admis","motif":null})),
            )
            .await;
        assert!(
            dec.statut.is_success(),
            "décision : {} {}",
            dec.statut,
            dec.texte
        );
        // Correspondances par la Direction (l'identifiant du responsable du dossier y figure).
        let c = self
            .req(
                "GET",
                &format!("/inscriptions/dossiers/{dossier}/correspondances"),
                Some("Direction"),
                None,
            )
            .await;
        assert!(
            c.statut.is_success(),
            "correspondances (Direction) : {} {}",
            c.statut,
            c.texte
        );
        let resp = uuid(&c.json["responsables"][0]["responsable_id"]);
        (dossier, resp)
    }

    /// Corps de validation d'un dossier admis (nouvel élève, nouvelle personne).
    fn corps_validation(responsable_id: Uuid) -> Value {
        json!({
            "classe_id":null,"regime":null,"niveau":"4","parcours":null,
            "responsables":[{"responsable_id":responsable_id,"personne":{"choix":"nouvelle"},"foyer":null}],
            "eleve":{"choix":"nouveau"}
        })
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

// ───────────────────────── Failles et manques ouverts ─────────────────────────

// RP2-02 : le secrétariat (G5 sans G6) lit « Chang », l'ancien nom de naissance de Cho adoptée.
#[test]
#[ignore = "FAILLE OUVERTE RP2-02 : après une adoption, le secrétariat (G5 sans G6) lit l'ancien nom de naissance dans l'historique d'identité (docs/recette/RECETTE_PERSONAS_2.md)"]
fn rp2_02_adoption_ancien_nom_invisible_hors_g6() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let cho = s.eleve("Cho");
        let piece = s.deposer_pdf(cho, "Jugement").await;
        let c = s
            .req(
                "POST",
                &format!("/eleves/{cho}/identite/changements"),
                Some("Direction"),
                Some(&json!({
                    "champs":{"nom_naissance":"Diggory"},"effet_du":"2026-10-15",
                    "motif_code":"adoption","motif":"jugement","piece_document_id":piece
                })),
            )
            .await;
        assert!(
            c.statut.is_success(),
            "changement d'identité : {} {}",
            c.statut,
            c.texte
        );
        let noms = |r: &Rep| -> Vec<String> {
            r.json
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|v| v["nom_naissance"].as_str().unwrap_or("").to_string())
                        .collect()
                })
                .unwrap_or_default()
        };
        let d = s
            .req(
                "GET",
                &format!("/eleves/{cho}/identite/historique"),
                Some("Direction"),
                None,
            )
            .await;
        let nd = noms(&d);
        assert!(
            nd.iter().any(|n| n == "Chang") && nd.iter().any(|n| n == "Diggory"),
            "condition préalable : la Direction doit voir Chang et Diggory : {}",
            d.texte
        );
        let a = s
            .req(
                "GET",
                &format!("/eleves/{cho}/identite/historique"),
                Some("Administratif"),
                None,
            )
            .await;
        let na = noms(&a);
        assert!(
            !na.iter().any(|n| n == "Chang"),
            "le secrétariat lit l'ancien nom « Chang » de Cho après l'adoption ({} versions : {:?}) : {}",
            na.len(),
            na,
            a.texte
        );
    });
}

// RP2-03 : l'ancien chemin /foyers/{f}/liens crée une 2e personne pour un e-mail déjà connu.
#[test]
#[ignore = "FAILLE OUVERTE RP2-03 : l'ancien chemin /foyers/{f}/liens crée un doublon de personne (et un compte) pour un e-mail déjà connu, sans reporter « ne pas solliciter » (docs/recette/RECETTE_PERSONAS_2.md)"]
fn rp2_03_ancien_chemin_foyers_ne_cree_pas_de_doublon_de_personne() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let (email, marc) = s.ajouter_marc(s.eleve("Harry")).await;
        s.ne_pas_solliciter(marc).await;
        assert_eq!(
            s.personnes_avec_email(&email).await,
            1,
            "condition préalable : une seule personne avant l'ancien chemin"
        );
        let l = s.ancien_chemin_foyer(s.eleve("Hermione"), &email).await;
        let n = s.personnes_avec_email(&email).await;
        assert!(
            !l.statut.is_success() || n == 1,
            "l'ancien chemin /foyers/{{f}}/liens a répondu {} et il existe {n} personnes avec l'e-mail {email} \
             (doublon, compte créé, « ne pas solliciter » non reporté) : {}",
            l.statut,
            l.texte
        );
    });
}

// RP2-04 : fusion vers une personne « ne pas solliciter » : 201, le compte voit l'élève.
#[test]
#[ignore = "FAILLE OUVERTE RP2-04 : fusionner une personne (avec compte) vers une personne « ne pas solliciter » est accepté sans confirmation et révèle l'établissement à ce parent (docs/recette/RECETTE_PERSONAS_2.md)"]
fn rp2_04_fusion_vers_une_personne_a_ne_pas_solliciter_exige_une_confirmation() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let (email, marc) = s.ajouter_marc(s.eleve("Harry")).await;
        s.ne_pas_solliciter(marc).await;
        let l = s.ancien_chemin_foyer(s.eleve("Hermione"), &email).await;
        assert_eq!(l.statut, StatusCode::CREATED, "ancien chemin : {}", l.texte);
        // La personne créée par l'ancien chemin (avec compte) : celle qui n'est pas `marc`.
        let pool = s.pool().await;
        let source: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND lower(email) = lower($2) \
             AND id <> $3 AND fusionnee_dans IS NULL AND user_id IS NOT NULL",
        )
        .bind(s.etab)
        .bind(&email)
        .bind(marc)
        .fetch_optional(&pool)
        .await
        .expect("recherche de la personne doublon");
        pool.close().await;
        let Some(source) = source else {
            // Plus de doublon avec compte : le problème amont est corrigé, rien à fusionner.
            panic!("pas de personne doublon avec compte à fusionner (voir RP2-03)");
        };
        s.mot_de_passe_de_labo(&email).await;
        let f = s
            .req(
                "POST",
                "/dossier/fusions",
                Some("Direction"),
                Some(&json!({"source_id":source,"cible_id":marc,"motif":"doublon"})),
            )
            .await;
        let jeton_marc = s.connecter(&email).await;
        let enfants = s
            .appel("GET", "/famille/enfants", Some(&jeton_marc), None)
            .await;
        assert!(
            f.statut.is_client_error(),
            "la fusion vers une personne « ne pas solliciter » est acceptée sans confirmation ({}) ; \
             le compte de Marc voit alors dans GET /famille/enfants ({}) : {}",
            f.statut,
            enfants.statut,
            enfants.texte
        );
    });
}

// RP2-05 : liste de payeurs vide acceptée, l'élève n'a plus aucun payeur.
#[test]
#[ignore = "FAILLE OUVERTE RP2-05 : la comptabilité peut remplacer les payeurs d'un élève par une liste vide (docs/recette/RECETTE_PERSONAS_2.md)"]
fn rp2_05_liste_de_payeurs_vide_refusee() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let harry = s.eleve("Harry");
        let r = s
            .req(
                "PUT",
                &format!("/dossier/eleves/{harry}/payeurs"),
                Some("Compta"),
                Some(&json!({"payeurs":[]})),
            )
            .await;
        assert!(
            r.statut.is_client_error(),
            "PUT payeurs [] par Compta accepté ({}) : Harry n'a plus aucun payeur : {}",
            r.statut,
            r.texte
        );
    });
}

// RP2-10 : la validation d'une admission écrit l'ancien modèle, la scolarité E4 reste vide.
#[test]
#[ignore = "MANQUE RP2-10 : l'admission validée n'apparaît pas dans GET /eleves/{id}/scolarite (la validation écrit ent_eleve_scolarite_annee, que le modèle E4 ne lit pas) (docs/recette/RECETTE_PERSONAS_2.md)"]
fn rp2_10_inscription_validee_visible_dans_la_scolarite() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let cible = s.annee_par_libelle("2027-2028").await;
        let (dossier, resp) = s.admission_admise().await;
        let v = s
            .req(
                "POST",
                &format!("/inscriptions/dossiers/{dossier}/valider"),
                Some("Direction"),
                Some(&Scene::corps_validation(resp)),
            )
            .await;
        assert!(
            v.statut.is_success(),
            "validation (Direction) : {} {}",
            v.statut,
            v.texte
        );
        let nouvel = uuid(&v.json["eleve_id"]);
        let sc = s
            .req(
                "GET",
                &format!("/eleves/{nouvel}/scolarite"),
                Some("Administratif"),
                None,
            )
            .await;
        assert!(
            sc.statut.is_success(),
            "scolarité : {} {}",
            sc.statut,
            sc.texte
        );
        let insc = sc.json["inscriptions"]
            .as_array()
            .map(|a| a.iter().any(|i| i["annee_id"] == json!(cible)))
            .unwrap_or(false);
        let scol = sc.json["scolarisations"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        assert!(
            insc || scol,
            "après validation, la scolarité du nouvel élève ne montre ni inscription annuelle pour {cible} \
             ni scolarisation à venir : {}",
            sc.texte
        );
    });
}

// RP2-11 : INS-05 donne la validation à « ADF ou DIR » ; le secrétariat reçoit 403.
#[test]
#[ignore = "MANQUE RP2-11 : le secrétariat (Administratif) ne peut pas valider une inscription admise ni lire les correspondances, contrairement à INS-05 (docs/socles/03_INSCRIPTIONS.md)"]
fn rp2_11_secretariat_valide_une_inscription() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        s.annee_par_libelle("2027-2028").await;
        let (dossier, resp) = s.admission_admise().await;
        let c = s
            .req(
                "GET",
                &format!("/inscriptions/dossiers/{dossier}/correspondances"),
                Some("Administratif"),
                None,
            )
            .await;
        let v = s
            .req(
                "POST",
                &format!("/inscriptions/dossiers/{dossier}/valider"),
                Some("Administratif"),
                Some(&Scene::corps_validation(resp)),
            )
            .await;
        assert!(
            v.statut.is_success(),
            "valider par Administratif sur un dossier admis : {} {} (correspondances : {})",
            v.statut,
            v.texte,
            c.statut
        );
    });
}

// RP2-12 : la sortie de l'établissement ne termine pas l'inscription annuelle en cours.
#[test]
#[ignore = "MANQUE RP2-12 : après la clôture de la scolarisation (sortie), l'inscription annuelle de l'année courante reste « en_cours » (docs/recette/RECETTE_PERSONAS_2.md)"]
fn rp2_12_sortie_termine_l_inscription_annuelle() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let seamus = s.eleve("Seamus");
        let etats = |r: &Rep| -> Vec<String> {
            r.json["inscriptions"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|i| i["etat"].as_str().unwrap_or("").to_string())
                        .collect()
                })
                .unwrap_or_default()
        };
        let avant = s
            .req(
                "GET",
                &format!("/eleves/{seamus}/scolarite"),
                Some("Administratif"),
                None,
            )
            .await;
        assert!(
            etats(&avant).iter().any(|e| e == "en_cours"),
            "condition préalable : Seamus doit avoir une inscription en_cours : {}",
            avant.texte
        );
        let c = s
            .req(
                "POST",
                &format!("/eleves/{seamus}/scolarisations/cloturer"),
                Some("Administratif"),
                Some(&json!({
                    "date_sortie":"2026-11-06","motif":"demenagement","motif_libre":null,
                    "destination":{"libelle":"Collège de Brest"},"exeat_document_id":null
                })),
            )
            .await;
        assert!(
            c.statut.is_success(),
            "clôture de la scolarisation : {} {}",
            c.statut,
            c.texte
        );
        let apres = s
            .req(
                "GET",
                &format!("/eleves/{seamus}/scolarite"),
                Some("Administratif"),
                None,
            )
            .await;
        assert!(
            !etats(&apres).iter().any(|e| e == "en_cours"),
            "Seamus est sorti le 2026-11-06 mais son inscription annuelle est encore en_cours : {}",
            apres.texte
        );
    });
}

// RP2-13 : régression de RP1-09 (corrigée sur `ent-recette-personas-1`, non fusionnée).
#[test]
#[ignore = "FAILLE OUVERTE RP2-13 : Harry (14 ans) déclare lui-même son absence (RP1-09 corrigée sur la branche ent-recette-personas-1, non fusionnée) (docs/recette/RECETTE_PERSONAS_2.md)"]
fn rp2_13_eleve_mineur_ne_declare_pas_son_absence_regression() {
    lance(async {
        let s = scene!(Scene::nouvelle());
        let harry = s.eleve("Harry");
        let r = s
            .req(
                "POST",
                &format!("/eleves/{harry}/absences"),
                Some("Eleve - Harry Potter"),
                Some(&json!({"debut":"2026-11-16","fin":"2026-11-16","motif":"appel"})),
            )
            .await;
        assert!(
            !r.statut.is_success(),
            "Harry mineur déclare sa propre absence : {} {}",
            r.statut,
            r.texte
        );
    });
}
