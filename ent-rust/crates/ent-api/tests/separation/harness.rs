//! Harnais de la matrice de séparation : démarrage du vrai routeur `ent-api`
//! sur une base PostgreSQL **locale de test**, deux établissements de labo
//! (A et B), comptes connectés, client HTTP en process et oracle de fuite.
//!
//! Le routeur est appelé en process (`tower::ServiceExt::oneshot`) : mêmes
//! couches, mêmes extracteurs, même base que le serveur réel, sans socket ni
//! dépendance de test supplémentaire.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

/// Runtime partagé par tous les tests : le pool sqlx est lié au runtime qui
/// l'a créé, il doit donc vivre aussi longtemps que le processus de test.
pub fn rt() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("runtime de test")
    })
}

/// Monde partagé (construit une fois). `None` = SKIP (pas de base de test).
pub fn monde() -> Option<&'static Monde> {
    static MONDE: OnceLock<Option<Monde>> = OnceLock::new();
    MONDE.get_or_init(|| rt().block_on(Monde::construire())).as_ref()
}

// ───────────────────────── Acteurs ─────────────────────────

/// Les acteurs de l'établissement A. `ParentAutreFoyer` et `ParentSansDrapeaux`
/// sont définis relativement à la cible « Harry » (voir `Objets`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Acteur {
    Admin,
    Direction,
    Administratif,
    Cpe,
    Prof,
    Compta,
    Infirmiere,
    Psy,
    /// Parent de Harry (Lily Potter), tous drapeaux.
    Parent,
    /// Élève Harry Potter (compte élève).
    Eleve,
    /// Parent d'un autre foyer (Narcissa Malefoy, mère de Draco).
    ParentAutreFoyer,
    /// Parent sans drapeaux notes / vie scolaire sur Ginny (Molly Weasley).
    ParentSansDrapeaux,
}

impl Acteur {
    pub const TOUS: [Acteur; 12] = [
        Acteur::Admin,
        Acteur::Direction,
        Acteur::Administratif,
        Acteur::Cpe,
        Acteur::Prof,
        Acteur::Compta,
        Acteur::Infirmiere,
        Acteur::Psy,
        Acteur::Parent,
        Acteur::Eleve,
        Acteur::ParentAutreFoyer,
        Acteur::ParentSansDrapeaux,
    ];
    pub const STAFF: [Acteur; 8] = [
        Acteur::Admin,
        Acteur::Direction,
        Acteur::Administratif,
        Acteur::Cpe,
        Acteur::Prof,
        Acteur::Compta,
        Acteur::Infirmiere,
        Acteur::Psy,
    ];
    pub const FAMILLE: [Acteur; 4] = [
        Acteur::Parent,
        Acteur::Eleve,
        Acteur::ParentAutreFoyer,
        Acteur::ParentSansDrapeaux,
    ];
    pub fn est_famille(self) -> bool {
        Self::FAMILLE.contains(&self)
    }
}

// ───────────────────────── Cibles ─────────────────────────

/// Position de la ressource visée par rapport à l'acteur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cible {
    /// Dans l'établissement A et liée à l'acteur (Harry pour Parent / Eleve).
    Liee,
    /// Dans l'établissement A mais non liée à l'acteur (Draco).
    NonLiee,
    /// Dans l'établissement B (jeton A, identifiants B).
    B,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attendu {
    /// 2xx.
    Autorise,
    /// 401 / 403 / 404.
    Refuse,
    /// 2xx mais liste vide (ou objet nul).
    ListeVide,
    /// Refus, ou 2xx vide (cible d'un autre établissement).
    RefuseOuVide,
    /// Tout échec non 2xx (écritures refusées).
    EchecQuelconque,
    /// Refus, ou réponse identique à celle d'un identifiant inexistant (aucune information).
    RefuseOuIndistinct,
}

/// Identifiants des objets d'un établissement, découverts via les API normales.
#[derive(Clone, Debug, Default)]
pub struct Objets {
    pub etab: Uuid,
    pub eleve: Uuid,       // Harry Potter
    pub eleve_autre: Uuid, // Draco Malefoy
    pub eleve_ginny: Uuid, // Ginny Weasley (parent sans drapeaux)
    pub eleve_ron: Uuid,   // Ron Weasley
    pub foyer: Uuid,       // foyer de Harry
    pub foyer_autre: Uuid, // foyer de Draco
    pub eleve_neutre: Uuid, // Luna Lovegood (cible des tests de création de compte)
    pub foyer_neutre: Uuid,
    pub doc: Option<Uuid>,       // document standard de Harry
    pub doc_sante: Option<Uuid>, // document restreint (santé verrouillé) de Harry
    pub doc_autre: Option<Uuid>, // document de Draco
    pub fil: Option<Uuid>,       // fil lié à Harry
    pub fil_autre: Option<Uuid>, // fil lié à Draco
    pub facture: Option<Uuid>,   // facture du foyer de Harry
    pub facture_autre: Option<Uuid>,
    pub absence: Option<Uuid>,
    pub absence_autre: Option<Uuid>,
    pub matiere: Option<Uuid>,
    /// Cours (service d'enseignement) du professeur de l'établissement (socle 5, S1).
    pub cours: Option<Uuid>,
    /// Appel ouvert par le professeur de l'établissement (via `POST /appels`).
    pub appel: Option<Uuid>,
    pub users: BTreeMap<String, Uuid>,
    /// Tous les identifiants connus de l'établissement (pour l'oracle de fuite).
    pub tous_ids: BTreeSet<String>,
}

pub struct Etab {
    pub objets: Objets,
    pub jetons: BTreeMap<Acteur, String>,
    pub mot_de_passe: String,
    pub emails: BTreeMap<Acteur, String>,
    pub user_ids: BTreeMap<Acteur, Uuid>,
}

pub struct Monde {
    pub routeur: axum::Router,
    pub a: Etab,
    pub b: Etab,
}

// ───────────────────────── Réponses ─────────────────────────

pub struct Rep {
    pub statut: StatusCode,
    pub texte: String,
    pub json: Value,
}

impl Rep {
    pub fn est_refus(&self) -> bool {
        matches!(
            self.statut,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
        )
    }
    pub fn est_succes(&self) -> bool {
        self.statut.is_success()
    }
    /// Liste vide, objet nul ou objet vide.
    pub fn est_vide(&self) -> bool {
        match &self.json {
            Value::Null => self.texte.trim().is_empty() || self.texte.trim() == "null",
            Value::Array(a) => a.is_empty(),
            Value::Object(o) => o.is_empty(),
            _ => false,
        }
    }
}

impl Monde {
    pub async fn requete(
        &self,
        methode: &str,
        chemin: &str,
        jeton: Option<&str>,
        corps: Option<&Value>,
        en_tetes: &[(&str, String)],
    ) -> Rep {
        let mut b = Request::builder().method(methode).uri(chemin);
        if let Some(j) = jeton {
            b = b.header("authorization", format!("Bearer {j}"));
        }
        for (k, v) in en_tetes {
            b = b.header(*k, v.as_str());
        }
        let body = match corps {
            Some(c) => {
                b = b.header("content-type", "application/json");
                Body::from(c.to_string())
            }
            None => Body::empty(),
        };
        let res = self
            .routeur
            .clone()
            .oneshot(b.body(body).expect("requête"))
            .await
            .expect("routeur infaillible");
        let statut = res.status();
        let octets = res.into_body().collect().await.expect("corps").to_bytes();
        let texte = String::from_utf8_lossy(&octets).into_owned();
        let json = serde_json::from_slice(&octets).unwrap_or(Value::Null);
        Rep {
            statut,
            texte,
            json,
        }
    }

    pub async fn appel(
        &self,
        methode: &str,
        chemin: &str,
        jeton: Option<&str>,
        corps: Option<&Value>,
    ) -> Rep {
        self.requete(methode, chemin, jeton, corps, &[]).await
    }

    /// Jeton d'un acteur de A.
    pub fn jeton(&self, a: Acteur) -> &str {
        self.a
            .jetons
            .get(&a)
            .map(String::as_str)
            .unwrap_or_else(|| panic!("acteur {a:?} absent du labo"))
    }

    /// Élève visé par `(acteur, cible)`.
    pub fn cible_eleve(&self, a: Acteur, c: Cible) -> Uuid {
        match c {
            Cible::B => self.b.objets.eleve,
            Cible::Liee => match a {
                Acteur::ParentSansDrapeaux => self.a.objets.eleve_ginny,
                Acteur::ParentAutreFoyer => self.a.objets.eleve_autre,
                _ => self.a.objets.eleve,
            },
            Cible::NonLiee => match a {
                Acteur::ParentAutreFoyer | Acteur::ParentSansDrapeaux => self.a.objets.eleve,
                _ => self.a.objets.eleve_autre,
            },
        }
    }

    /// Objets de la cible : B, ou A (Harry / Draco selon l'acteur).
    pub fn objets_cible(&self, a: Acteur, c: Cible) -> CibleObjets {
        let o = |e: &Objets, autre: bool| CibleObjets {
            eleve: if autre { e.eleve_autre } else { e.eleve },
            foyer: if autre { e.foyer_autre } else { e.foyer },
            doc: if autre { e.doc_autre } else { e.doc },
            doc_sante: if autre { None } else { e.doc_sante },
            fil: if autre { e.fil_autre } else { e.fil },
            facture: if autre { e.facture_autre } else { e.facture },
            absence: if autre { e.absence_autre } else { e.absence },
            matiere: e.matiere,
            cours: e.cours,
            appel: if autre { None } else { e.appel },
        };
        match c {
            Cible::B => o(&self.b.objets, false),
            Cible::Liee => match a {
                Acteur::ParentAutreFoyer => o(&self.a.objets, true),
                Acteur::ParentSansDrapeaux => CibleObjets {
                    eleve: self.a.objets.eleve_ginny,
                    ..Default::default()
                },
                _ => o(&self.a.objets, false),
            },
            Cible::NonLiee => match a {
                Acteur::ParentAutreFoyer | Acteur::ParentSansDrapeaux => o(&self.a.objets, false),
                _ => o(&self.a.objets, true),
            },
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CibleObjets {
    pub eleve: Uuid,
    pub foyer: Uuid,
    pub doc: Option<Uuid>,
    pub doc_sante: Option<Uuid>,
    pub fil: Option<Uuid>,
    pub facture: Option<Uuid>,
    pub absence: Option<Uuid>,
    pub matiere: Option<Uuid>,
    pub cours: Option<Uuid>,
    pub appel: Option<Uuid>,
}

impl CibleObjets {
    /// Remplace les marqueurs `{eleve}`, `{foyer}`, `{doc}`, `{doc_sante}`,
    /// `{fil}`, `{facture}`, `{absence}`. `None` si un objet manque.
    pub fn resoudre(&self, gabarit: &str) -> Option<String> {
        let mut s = gabarit.to_string();
        let nul = Uuid::nil().to_string();
        for (cle, val) in [
            ("{eleve}", Some(self.eleve)),
            ("{foyer}", Some(self.foyer)),
            ("{doc}", self.doc),
            ("{doc_sante}", self.doc_sante),
            ("{fil}", self.fil),
            ("{facture}", self.facture),
            ("{absence}", self.absence),
            ("{matiere}", self.matiere),
            ("{cours}", self.cours),
            ("{appel}", self.appel),
        ] {
            if s.contains(cle) {
                let v = val?;
                s = s.replace(cle, &v.to_string());
            }
        }
        let _ = nul;
        Some(s)
    }
}

// ───────────────────────── Construction ─────────────────────────

fn garde_base() -> Option<String> {
    let Ok(url) = std::env::var("ENT_DATABASE_URL") else {
        eprintln!("SKIP : ENT_DATABASE_URL absent (sourcer ent-rust/web/scripts/env.sh)");
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

impl Monde {
    async fn construire() -> Option<Monde> {
        garde_base()?;
        let (state, _bind) = ent_api::boot_from_env()
            .await
            .unwrap_or_else(|e| panic!("démarrage ent-api : {e}"));
        let routeur = ent_api::app(state);
        let mut monde = Monde {
            routeur,
            a: Etab::vide(),
            b: Etab::vide(),
        };
        let a = monde.amorcer().await?;
        let b = monde.amorcer().await?;
        monde.a = a;
        monde.b = b;
        Some(monde)
    }

    /// Un `POST /labo/bootstrap`, connexion des comptes, découverte des objets.
    async fn amorcer(&self) -> Option<Etab> {
        let r = self.appel("POST", "/labo/bootstrap", None, None).await;
        if r.statut == StatusCode::FORBIDDEN || r.statut == StatusCode::NOT_FOUND {
            eprintln!("SKIP : labo fermé ou absent (feature `labo` et ENT_ENV=lab requis)");
            return None;
        }
        assert_eq!(r.statut, StatusCode::OK, "bootstrap labo : {}", r.statut);
        let d = r.json;
        let etab = uuid(&d["etablissement_id"]);
        let mdp = d["password"]
            .as_str()
            .expect("mot de passe de labo")
            .to_string();

        let mut emails = BTreeMap::new();
        let mut users: BTreeMap<String, Uuid> = Default::default();
        for p in d["personnel"].as_array().unwrap() {
            let label = p["label"].as_str().unwrap_or("");
            let email = p["email"].as_str().unwrap().to_string();
            users.insert(label.to_string(), uuid(&p["user_id"]));
            let a = match label {
                "Admin" => Some(Acteur::Admin),
                "Direction" => Some(Acteur::Direction),
                "Administratif" => Some(Acteur::Administratif),
                "Compta" => Some(Acteur::Compta),
                "CPE" => Some(Acteur::Cpe),
                "Prof 1" => Some(Acteur::Prof),
                "Infirmerie 1" => Some(Acteur::Infirmiere),
                "Psy 1" => Some(Acteur::Psy),
                _ => None,
            };
            if let Some(a) = a {
                emails.insert(a, email);
            }
        }
        for p in d["personas"].as_array().unwrap() {
            let label = p["label"].as_str().unwrap_or("");
            let email = p["email"].as_str().unwrap().to_string();
            let a = if label.contains("Lily Potter") {
                Some(Acteur::Parent)
            } else if label.contains("Narcissa") {
                Some(Acteur::ParentAutreFoyer)
            } else if label.contains("Molly") {
                Some(Acteur::ParentSansDrapeaux)
            } else if label.starts_with("Eleve") {
                Some(Acteur::Eleve)
            } else {
                None
            };
            if let Some(a) = a {
                emails.insert(a, email);
            }
        }
        for a in Acteur::TOUS {
            assert!(emails.contains_key(&a), "acteur {a:?} absent du labo");
        }

        let mut jetons = BTreeMap::new();
        let mut user_ids = BTreeMap::new();
        for (a, email) in &emails {
            let l = self
                .appel(
                    "POST",
                    "/auth/login",
                    None,
                    Some(&json!({"email": email, "password": mdp, "etablissement_id": etab})),
                )
                .await;
            assert_eq!(l.statut, StatusCode::OK, "connexion {a:?} : {}", l.statut);
            jetons.insert(*a, l.json["token"].as_str().expect("jeton").to_string());
            user_ids.insert(*a, uuid(&l.json["user_id"]));
        }

        // Le module santé est fermé par défaut (règle produit : pas de donnée de santé sans
        // hébergement HDS). La matrice, qui éprouve les routes de santé, l'active explicitement
        // dans chaque établissement ; le refus par défaut a ses propres tests
        // (`tests_sante_module`, `sante_s2`).
        let act = self
            .appel(
                "PUT",
                "/sante/module",
                jetons.get(&Acteur::Direction).map(String::as_str),
                Some(&json!({"actif": true})),
            )
            .await;
        assert_eq!(act.statut, StatusCode::OK, "activation du module santé");

        let mut o = Objets {
            etab,
            users,
            ..Default::default()
        };
        let eleves = d["eleves"].as_array().unwrap();
        let trouve = |nom: &str| -> (Uuid, Uuid) {
            let e = eleves
                .iter()
                .find(|e| e["prenom"] == nom)
                .unwrap_or_else(|| panic!("élève {nom} absent du labo"));
            (uuid(&e["id"]), uuid(&e["foyer_id"]))
        };
        (o.eleve, o.foyer) = trouve("Harry");
        (o.eleve_autre, o.foyer_autre) = trouve("Draco");
        o.eleve_ginny = trouve("Ginny").0;
        (o.eleve_neutre, o.foyer_neutre) = trouve("Luna");
        o.eleve_ron = trouve("Ron").0;
        for e in eleves {
            o.tous_ids.insert(e["id"].as_str().unwrap().into());
            o.tous_ids.insert(e["foyer_id"].as_str().unwrap().into());
        }
        o.tous_ids.insert(etab.to_string());
        for u in o.users.values() {
            o.tous_ids.insert(u.to_string());
        }

        let dir = jetons[&Acteur::Direction].clone();
        self.decouvrir(&mut o, &dir).await;
        let prof = jetons[&Acteur::Prof].clone();
        let ens = self.appel("GET", "/referentiel/mes-enseignements", Some(&prof), None).await;
        o.cours = ens
            .json
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v["cours_id"].as_str())
            .and_then(|s| Uuid::parse_str(s).ok());
        self.ouvrir_appel(&mut o, &prof).await;
        Some(Etab {
            objets: o,
            jetons,
            mot_de_passe: mdp,
            emails,
            user_ids,
        })
    }

    /// Objets créés par le seed (documents, fils, factures, absences) lus avec
    /// le jeton de la direction, par les API normales.
    async fn decouvrir(&self, o: &mut Objets, dir: &str) {
        let liste = |r: &Rep| r.json.as_array().cloned().unwrap_or_default();
        let id_de = |v: &Value| v["id"].as_str().and_then(|s| Uuid::parse_str(s).ok());

        let docs = liste(
            &self
                .appel("GET", &format!("/eleves/{}/documents", o.eleve), Some(dir), None)
                .await,
        );
        o.doc = docs
            .iter()
            .find(|x| x["confidentialite"] == "standard")
            .and_then(id_de);
        o.doc_sante = docs
            .iter()
            .find(|x| x["confidentialite"] == "restreint")
            .and_then(id_de);
        let docs2 = liste(
            &self
                .appel(
                    "GET",
                    &format!("/eleves/{}/documents", o.eleve_autre),
                    Some(dir),
                    None,
                )
                .await,
        );
        o.doc_autre = docs2.first().and_then(id_de);

        let fils = liste(&self.appel("GET", "/messages/fils", Some(dir), None).await);
        o.fil = fils
            .iter()
            .find(|f| f["eleve_id"] == json!(o.eleve))
            .and_then(id_de);
        o.fil_autre = fils
            .iter()
            .find(|f| f["eleve_id"] == json!(o.eleve_autre))
            .and_then(id_de);

        let facs = liste(&self.appel("GET", "/facturation/factures", Some(dir), None).await);
        o.facture = facs
            .iter()
            .find(|f| f["foyer_id"] == json!(o.foyer))
            .and_then(id_de);
        o.facture_autre = facs
            .iter()
            .find(|f| f["foyer_id"] == json!(o.foyer_autre))
            .and_then(id_de);

        let abs = liste(
            &self
                .appel("GET", &format!("/eleves/{}/absences", o.eleve), Some(dir), None)
                .await,
        );
        o.absence = abs.first().and_then(id_de);
        let abs2 = liste(
            &self
                .appel(
                    "GET",
                    &format!("/eleves/{}/absences", o.eleve_autre),
                    Some(dir),
                    None,
                )
                .await,
        );
        o.absence_autre = abs2.first().and_then(id_de);

        let mat = liste(&self.appel("GET", "/matieres", Some(dir), None).await);
        o.matiere = mat.first().and_then(id_de);

        for v in docs
            .iter()
            .chain(&docs2)
            .chain(&fils)
            .chain(&facs)
            .chain(&abs)
            .chain(&abs2)
            .chain(&mat)
        {
            if let Some(i) = v["id"].as_str() {
                o.tous_ids.insert(i.to_string());
            }
        }
    }
}

impl Etab {
    fn vide() -> Etab {
        Etab {
            objets: Objets::default(),
            jetons: Default::default(),
            mot_de_passe: String::new(),
            emails: Default::default(),
            user_ids: Default::default(),
        }
    }
}

fn uuid(v: &Value) -> Uuid {
    Uuid::parse_str(v.as_str().expect("uuid attendu")).expect("uuid valide")
}

// ───────────────────────── Oracle de fuite ─────────────────────────

/// Retourne les identifiants de B présents dans `texte` (doit être vide) et
/// les sous-chaînes interdites propres au cas.
pub fn fuites(monde: &Monde, texte: &str, interdits: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for id in &monde.b.objets.tous_ids {
        if texte.contains(id.as_str()) {
            out.push(format!("identifiant de B : {id}"));
        }
    }
    for i in interdits {
        let present = if *i == "storage_key" {
            serde_json::from_str::<Value>(texte).map_or_else(|_| texte.contains(i), |v| cle_non_nulle(&v, i))
        } else {
            texte.contains(i)
        };
        if present {
            out.push(format!("champ interdit : {i}"));
        }
    }
    out
}

impl Monde {
    /// Ouvre un appel pour le professeur de l'établissement (API normale),
    /// sur un jour futur neuf : un appel clos ne peut plus être modifié.
    async fn ouvrir_appel(&self, o: &mut Objets, prof: &str) {
        let micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;
        let base = domain_eleve::horloge::jour_paris()
            + chrono::Duration::weeks(1 + micros.rem_euclid(2000));
        for decalage in 0..7 {
            let jour = base + chrono::Duration::days(decalage);
            let r = self
                .appel("GET", &format!("/edt/mes-creneaux?jour={jour}"), Some(prof), None)
                .await;
            let Some(c) = r.json.as_array().and_then(|a| a.first()).cloned() else {
                continue;
            };
            let cid = c["creneau_id"].clone();
            let cid = if cid.is_null() { c["id"].clone() } else { cid };
            let r = self
                .appel(
                    "POST",
                    "/appels",
                    Some(prof),
                    Some(&json!({"creneau_id": cid, "jour": jour.to_string()})),
                )
                .await;
            if r.statut.is_success() {
                o.appel = r.json["id"].as_str().and_then(|s| Uuid::parse_str(s).ok());
                if let Some(i) = o.appel {
                    o.tous_ids.insert(i.to_string());
                }
                return;
            }
        }
    }
}

/// Vrai si une clé JSON `nom` porte une valeur non nulle quelque part dans `v`
/// (une clé présente mais `null` n'expose rien).
pub fn cle_non_nulle(v: &Value, nom: &str) -> bool {
    match v {
        Value::Object(o) => o
            .iter()
            .any(|(k, x)| (k == nom && !x.is_null()) || cle_non_nulle(x, nom)),
        Value::Array(a) => a.iter().any(|x| cle_non_nulle(x, nom)),
        _ => false,
    }
}
