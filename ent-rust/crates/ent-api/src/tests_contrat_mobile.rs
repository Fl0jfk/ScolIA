//! Socle 11, palier MB2 : **fixtures dorées** du contrat mobile et **banc de parité** web / appli.
//!
//! - Chaque route qu'appellent les applis (`contrat/mobile/routes.txt`, liste blanche) est jouée en HTTP sur un
//!   établissement de labo jetable, avec l'en-tête `X-ScolIA-Appli`. La réponse, normalisée (identifiants, instants,
//!   jetons, courriels), est comparée **par forme** à `contrat/mobile/<cas>.json` : mêmes clés, mêmes types. Les
//!   cœurs Kotlin et Swift décodent ces mêmes fichiers dans leurs tests : changer un DTO sans régénérer les fixtures
//!   rend ce test rouge, et régénérer sans mettre à jour les cœurs rend leurs tests rouges.
//! - Régénérer : `ENT_CONTRAT_ECRIRE=1 cargo nextest run -p ent-api --features labo contrat_mobile` (base locale).
//! - Banc de parité : pour chaque lecture et chaque persona du labo, la même requête avec et sans `X-ScolIA-Appli`
//!   doit donner le même statut et, en succès, le même corps. L'appli n'a jamais plus (ni moins) que le web.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{DateTime, NaiveDate};
use ent_db::{LaboCast, LaboPersona, LABO_PASSWORD};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use tower::ServiceExt;
use uuid::Uuid;

const APPLI: (&str, &str) = ("x-scolia-appli", "android/1.4.2 (build 142)");

/// Routes de la liste blanche que le banc ne joue pas, avec la raison (revue à chaque palier).
const NON_JOUEES: &[(&str, &str)] = &[
    (
        "POST /auth/passkey/login/finish",
        "cérémonie WebAuthn : demande un authentificateur réel (MB4)",
    ),
    (
        "POST /auth/passkey/register/finish",
        "cérémonie WebAuthn : demande un authentificateur réel (MB4)",
    ),
];

fn dossier() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contrat/mobile")
}

fn ecrire() -> bool {
    std::env::var("ENT_CONTRAT_ECRIRE").is_ok_and(|v| v == "1")
}

struct Reponse {
    statut: StatusCode,
    texte: String,
}

impl Reponse {
    fn json(&self) -> Value {
        if self.texte.trim().is_empty() {
            return Value::Null;
        }
        serde_json::from_str(&self.texte)
            .unwrap_or_else(|_| panic!("corps non JSON : {}", self.texte))
    }
}

async fn envoyer(
    app: &axum::Router,
    methode: &str,
    uri: &str,
    jeton: Option<&str>,
    en_tetes: &[(&str, &str)],
    corps: Option<&Value>,
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
            Body::from(serde_json::to_vec(c).unwrap())
        }
        None => Body::empty(),
    };
    let res = app.clone().oneshot(b.body(corps).unwrap()).await.unwrap();
    let (parts, body) = res.into_parts();
    let octets = axum::body::to_bytes(body, 8 << 20).await.unwrap();
    Reponse {
        statut: parts.status,
        texte: String::from_utf8_lossy(&octets).to_string(),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Normalisation et forme
// ---------------------------------------------------------------------------------------------------------------

/// Rend une réponse stable d'un semis à l'autre : identifiants numérotés dans l'ordre d'apparition (UUID valides),
/// instants et dates fixes, jetons et courriels masqués. Les formats restent ceux du serveur, pour que les cœurs
/// décodent les fixtures comme une vraie réponse.
#[derive(Default)]
struct Normaliseur {
    uuids: HashMap<String, String>,
}

impl Normaliseur {
    fn valeur(&mut self, cle: Option<&str>, v: &Value) -> Value {
        match v {
            Value::Object(o) => {
                let mut m = Map::new();
                for (k, x) in o {
                    m.insert(k.clone(), self.valeur(Some(k), x));
                }
                Value::Object(m)
            }
            Value::Array(a) => Value::Array(a.iter().map(|x| self.valeur(cle, x)).collect()),
            Value::String(s) => Value::String(self.texte(cle, s)),
            autre => autre.clone(),
        }
    }

    fn texte(&mut self, cle: Option<&str>, s: &str) -> String {
        // Tout champ qui porte un secret (jeton, mot de passe, code) est masqué, quel que soit son nom exact.
        if cle.is_some_and(|c| {
            let c = c.to_ascii_lowercase();
            ["token", "secret", "password", "jeton", "mot_de_passe"]
                .iter()
                .any(|m| c.contains(m))
        }) {
            return "secret-masque".into();
        }
        if Uuid::parse_str(s).is_ok() && s.len() == 36 {
            let n = self.uuids.len() + 1;
            return self
                .uuids
                .entry(s.to_ascii_lowercase())
                .or_insert_with(|| format!("00000000-0000-4000-8000-{n:012}"))
                .clone();
        }
        if DateTime::parse_from_rfc3339(s).is_ok() {
            return "2027-03-08T09:00:00Z".into();
        }
        if s.len() == 10 && NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok() {
            return "2027-03-08".into();
        }
        if s.contains('@') && !s.contains(' ') {
            return "persona@ent.test".into();
        }
        s.to_string()
    }
}

fn normaliser(v: &Value) -> Value {
    Normaliseur::default().valeur(None, v)
}

/// Forme d'une valeur JSON. `Null` est compatible avec tout (champ facultatif absent du semis), un tableau vide
/// aussi ; les éléments d'un tableau sont fusionnés (union des clés), les objets doivent avoir les mêmes clés.
#[derive(Debug, Clone, PartialEq)]
enum Forme {
    Nul,
    Booleen,
    Nombre,
    Texte,
    Tableau(Option<Box<Forme>>),
    Objet(BTreeMap<String, Forme>),
}

fn forme(v: &Value) -> Forme {
    match v {
        Value::Null => Forme::Nul,
        Value::Bool(_) => Forme::Booleen,
        Value::Number(_) => Forme::Nombre,
        Value::String(_) => Forme::Texte,
        Value::Array(a) => Forme::Tableau(a.iter().map(forme).reduce(fusion).map(Box::new)),
        Value::Object(o) => Forme::Objet(o.iter().map(|(k, x)| (k.clone(), forme(x))).collect()),
    }
}

fn fusion(a: Forme, b: Forme) -> Forme {
    match (a, b) {
        (Forme::Nul, x) | (x, Forme::Nul) => x,
        (Forme::Tableau(None), x) | (x, Forme::Tableau(None)) => x,
        (Forme::Tableau(Some(x)), Forme::Tableau(Some(y))) => {
            Forme::Tableau(Some(Box::new(fusion(*x, *y))))
        }
        (Forme::Objet(mut x), Forme::Objet(y)) => {
            for (k, fy) in y {
                let f = match x.remove(&k) {
                    Some(fx) => fusion(fx, fy),
                    None => fy,
                };
                x.insert(k, f);
            }
            Forme::Objet(x)
        }
        (x, _) => x,
    }
}

/// Écarts de forme entre la fixture (`a`) et la réponse (`b`), avec leur chemin JSON.
fn ecarts(chemin: &str, a: &Forme, b: &Forme, sortie: &mut Vec<String>) {
    match (a, b) {
        (Forme::Nul, _) | (_, Forme::Nul) => {}
        (Forme::Tableau(None), Forme::Tableau(None)) => {}
        // Liste vide d'un seul côté : la forme de ses éléments n'est plus vérifiée ; on le signale (le semis est
        // déterministe : si la liste se remplit ou se vide, la fixture doit suivre).
        (Forme::Tableau(None), Forme::Tableau(Some(_))) => sortie.push(format!(
            "{chemin} : liste vide dans la fixture, remplie dans la réponse"
        )),
        (Forme::Tableau(Some(_)), Forme::Tableau(None)) => sortie.push(format!(
            "{chemin} : liste remplie dans la fixture, vide dans la réponse"
        )),
        (Forme::Tableau(Some(x)), Forme::Tableau(Some(y))) => {
            ecarts(&format!("{chemin}[]"), x, y, sortie)
        }
        (Forme::Objet(x), Forme::Objet(y)) => {
            let cles: BTreeSet<&String> = x.keys().chain(y.keys()).collect();
            for k in cles {
                let c = format!("{chemin}.{k}");
                match (x.get(k), y.get(k)) {
                    (Some(fx), Some(fy)) => ecarts(&c, fx, fy, sortie),
                    (Some(_), None) => sortie.push(format!(
                        "{c} : présent dans la fixture, absent de la réponse"
                    )),
                    (None, Some(_)) => sortie.push(format!(
                        "{c} : nouveau dans la réponse, absent de la fixture"
                    )),
                    (None, None) => {}
                }
            }
        }
        (x, y) if x == y => {}
        (x, y) => sortie.push(format!(
            "{chemin} : {x:?} dans la fixture, {y:?} dans la réponse"
        )),
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Banc
// ---------------------------------------------------------------------------------------------------------------

struct Cas {
    nom: String,
    route: String,
    /// Chemin réellement joué (avec la chaîne de requête) : rejoué par le banc de parité.
    uri: String,
    statut: StatusCode,
    corps: Value,
}

struct Banc {
    app: axum::Router,
    ent: Arc<PgEnt>,
    cas: Vec<Cas>,
}

impl Banc {
    /// Joue une requête de l'appli (en-tête `X-ScolIA-Appli`, `Idempotency-Key` sur les écritures hors `/auth`),
    /// exige un succès et l'enregistre comme cas `nom` de la route `route` (gabarit de `routes.txt`).
    async fn jouer(
        &mut self,
        nom: &str,
        route: &str,
        uri: &str,
        jeton: Option<&str>,
        corps: Option<Value>,
    ) -> Value {
        let methode = route.split_once(' ').expect("route « MÉTHODE /chemin »").0;
        let cle = Uuid::new_v4().to_string();
        let mut en_tetes = vec![APPLI];
        if methode != "GET" && !uri.starts_with("/auth") {
            en_tetes.push(("idempotency-key", cle.as_str()));
        }
        let r = envoyer(&self.app, methode, uri, jeton, &en_tetes, corps.as_ref()).await;
        assert!(
            r.statut.is_success(),
            "{nom} ({methode} {uri}) : {} {}",
            r.statut,
            r.texte
        );
        let corps = r.json();
        self.cas.push(Cas {
            nom: nom.into(),
            route: route.into(),
            uri: uri.into(),
            statut: r.statut,
            corps: corps.clone(),
        });
        corps
    }
}

fn id(v: &Value) -> String {
    v["id"]
        .as_str()
        .unwrap_or_else(|| panic!("pas d'id dans {v}"))
        .to_string()
}

fn premier<'a>(v: &'a Value, quoi: &str) -> &'a Value {
    v.as_array()
        .and_then(|a| a.first())
        .unwrap_or_else(|| panic!("{quoi} : liste vide dans le semis du labo"))
}

async fn connecter(app: &axum::Router, email: &str, etab: Uuid) -> Value {
    let r = envoyer(
        app,
        "POST",
        "/auth/login",
        None,
        &[APPLI],
        Some(&json!({"email": email, "password": LABO_PASSWORD, "etablissement_id": etab, "client": "mobile"})),
    )
    .await;
    assert_eq!(r.statut, StatusCode::OK, "login {email} : {}", r.texte);
    r.json()
}

fn lire_routes() -> Vec<String> {
    let chemin = dossier().join("routes.txt");
    let texte =
        std::fs::read_to_string(&chemin).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
    texte
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

/// Joue tout le parcours des applis sur un établissement de labo neuf. Retourne le banc et le cast.
async fn parcours() -> Option<(Banc, LaboCast, String)> {
    let url = std::env::var("ENT_DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())?;
    let ent = PgEnt::connect(&url).await.expect("connect");
    ent.migrate().await.expect("migrate");
    let state = AppState::nouveau(ent);
    let cast = state.ent.seed_labo_cast().await.expect("cast");
    let state_ent = state.ent.clone();
    let jour = domain_eleve::horloge::jour_paris().to_string();
    let mut b = Banc {
        ent: state.ent.clone(),
        app: app(state),
        cas: Vec::new(),
    };
    let etab = cast.etablissement_id;
    let prof = cast.prof.token.clone();
    let cpe = cast.cpe.token.clone();
    let parent = cast.parent_lea.token.clone();
    let lea = cast.eleve_lea_id;
    let classe_lea = cast
        .eleves
        .iter()
        .find(|e| e.id == lea)
        .expect("Harry")
        .classe
        .clone();

    // --- Rattachement et connexion ------------------------------------------------------------------------
    // Établissement et domaine déclarés par la plateforme (B-02), comme en production.
    let domaine = format!("mb2-{}.ent.test", Uuid::new_v4().simple());
    let ent = state_ent.clone();
    ent.declarer_etablissement_plateforme(
        etab,
        &format!("mb2-{}", &Uuid::new_v4().simple().to_string()[..12]),
        "Collège du banc MB2",
    )
    .await
    .expect("déclaration");
    ent.ajouter_domaine_plateforme(etab, &domaine)
        .await
        .expect("domaine");
    let r = envoyer(
        &b.app,
        "GET",
        "/auth/etablissement",
        None,
        &[APPLI, ("host", &domaine)],
        None,
    )
    .await;
    assert_eq!(r.statut, StatusCode::OK, "etablissement : {}", r.texte);
    b.cas.push(Cas {
        nom: "auth_etablissement".into(),
        route: "GET /auth/etablissement".into(),
        uri: "/auth/etablissement".into(),
        statut: r.statut,
        corps: r.json(),
    });

    // Connexion par passkey, première étape : défi (factice pour un compte sans passkey, anti-énumération), sans
    // authentificateur. Les étapes suivantes demandent une cérémonie réelle (MB4).
    b.jouer(
        "auth_passkey_login_start",
        "POST /auth/passkey/login/start",
        "/auth/passkey/login/start",
        None,
        Some(json!({"email": cast.parent_lea.email, "etablissement_id": etab, "client": "mobile"})),
    )
    .await;
    let corps_login = json!({"email": cast.parent_lea.email, "password": LABO_PASSWORD, "etablissement_id": etab, "client": "mobile"});
    let session = b
        .jouer(
            "auth_login",
            "POST /auth/login",
            "/auth/login",
            None,
            Some(corps_login),
        )
        .await;
    assert!(
        session["token"].as_str().is_some_and(|t| !t.is_empty()),
        "jeton : {session}"
    );
    b.jouer(
        "auth_passkey_register_start",
        "POST /auth/passkey/register/start",
        "/auth/passkey/register/start",
        Some(&parent),
        Some(json!({})),
    )
    .await;
    b.jouer("auth_me", "GET /auth/me", "/auth/me", Some(&parent), None)
        .await;
    b.jouer(
        "roles_mes_droits",
        "GET /roles/mes-droits",
        "/roles/mes-droits",
        Some(&prof),
        None,
    )
    .await;
    let sessions = b
        .jouer(
            "auth_sessions",
            "GET /auth/sessions",
            "/auth/sessions",
            Some(&parent),
            None,
        )
        .await;
    assert!(
        sessions.as_array().is_some_and(|a| a.len() >= 2),
        "sessions : {sessions}"
    );

    // --- Notifications poussées ---------------------------------------------------------------------------
    let push = b
        .jouer(
            "devices_push",
            "POST /devices/push",
            "/devices/push",
            Some(&parent),
            Some(json!({"platform": "android", "token": format!("fcm-labo-{}", Uuid::new_v4()), "app_id": "fr.scolia.mobile"})),
        )
        .await;
    b.jouer(
        "devices_push_supprimer",
        "DELETE /devices/push/{id}",
        &format!("/devices/push/{}", id(&push)),
        Some(&parent),
        None,
    )
    .await;

    // --- Emploi du temps et appel (prof) ------------------------------------------------------------------
    b.jouer(
        "edt_mes_creneaux",
        "GET /edt/mes-creneaux",
        &format!("/edt/mes-creneaux?jour={jour}"),
        Some(&prof),
        None,
    )
    .await;
    b.jouer(
        "eleves_edt",
        "GET /eleves/{id}/edt",
        &format!("/eleves/{lea}/edt?jour={jour}"),
        Some(&parent),
        None,
    )
    .await;
    let appel = b
        .jouer(
            "appels_ouvrir",
            "POST /appels",
            "/appels",
            Some(&prof),
            Some(json!({"creneau_id": cast.creneau_rogue_4b, "jour": jour})),
        )
        .await;
    let appel_id = id(&appel);
    b.jouer(
        "appel",
        "GET /appels/{id}",
        &format!("/appels/{appel_id}"),
        Some(&prof),
        None,
    )
    .await;
    b.jouer(
        "appels_lignes",
        "POST /appels/{id}/lignes",
        &format!("/appels/{appel_id}/lignes"),
        Some(&prof),
        Some(json!({"eleve_id": lea, "etat": "absent"})),
    )
    .await;
    b.jouer(
        "appels_clore",
        "POST /appels/{id}/clore",
        &format!("/appels/{appel_id}/clore"),
        Some(&prof),
        Some(json!({})),
    )
    .await;

    // --- Famille et absences ------------------------------------------------------------------------------
    b.jouer(
        "famille_enfants",
        "GET /famille/enfants",
        "/famille/enfants",
        Some(&parent),
        None,
    )
    .await;
    let resume = b
        .jouer(
            "famille_resume",
            "GET /famille/enfants/{id}/resume",
            &format!("/famille/enfants/{lea}/resume"),
            Some(&parent),
            None,
        )
        .await;
    let absence = premier(&resume["absences_a_justifier"], "absences à justifier").clone();
    b.jouer(
        "absences_justifier",
        "POST /absences/{id}/justifier",
        &format!("/absences/{}/justifier", id(&absence)),
        Some(&parent),
        Some(json!({"motif_justif": "Rendez-vous médical (banc MB2)"})),
    )
    .await;
    b.jouer(
        "vs_absences",
        "GET /vs/absences",
        &format!("/vs/absences?jour={jour}"),
        Some(&cpe),
        None,
    )
    .await;

    // --- Notes, devoirs, cahier de textes ------------------------------------------------------------------
    let periodes = b
        .jouer(
            "note_periodes",
            "GET /note-periodes",
            "/note-periodes",
            Some(&prof),
            None,
        )
        .await;
    let periode = id(premier(&periodes, "périodes"));
    let notes = b
        .jouer(
            "eleves_notes",
            "GET /eleves/{id}/notes",
            &format!("/eleves/{lea}/notes"),
            Some(&parent),
            None,
        )
        .await;
    assert!(!premier(&notes, "notes de Harry")["matiere_id"].is_null());
    // Moyennes sur la période des notes du semis (la première période peut n'en avoir aucune).
    let periode_notes = premier(&notes, "notes de Harry")["periode_id"]
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| periode.clone());
    b.jouer(
        "eleves_moyennes",
        "GET /eleves/{id}/moyennes",
        &format!("/eleves/{lea}/moyennes?periode_id={periode_notes}"),
        Some(&parent),
        None,
    )
    .await;
    b.jouer(
        "devoirs",
        "GET /devoirs",
        &format!("/devoirs?periode_id={periode}&classe={classe_lea}"),
        Some(&prof),
        None,
    )
    .await;
    b.jouer(
        "eleves_devoirs",
        "GET /eleves/{id}/devoirs",
        &format!("/eleves/{lea}/devoirs?periode_id={periode}"),
        Some(&parent),
        None,
    )
    .await;
    b.jouer(
        "classes_effectif",
        "GET /classes/{classe}/effectif",
        &format!("/classes/{classe_lea}/effectif?jour={jour}"),
        Some(&prof),
        None,
    )
    .await;
    b.jouer(
        "cahier_texte",
        "GET /cahier-texte",
        &format!("/cahier-texte?classe={classe_lea}"),
        Some(&prof),
        None,
    )
    .await;
    b.jouer(
        "cahier_texte_eleve",
        "GET /cahier-texte",
        &format!("/cahier-texte?eleve_id={lea}"),
        Some(&parent),
        None,
    )
    .await;

    // --- Saisie des notes sur la grille d'évaluation `/eva/*` (palier « notes eva » du socle 11) ------------
    // Le professeur choisit un cours, une période, crée une évaluation, saisit une case puis publie.
    let enseignements = b
        .jouer(
            "eva_enseignements",
            "GET /eva/enseignements",
            "/eva/enseignements",
            Some(&prof),
            None,
        )
        .await;
    let cours = enseignements
        .as_array()
        .and_then(|a| a.iter().find(|e| e["droit"] == "saisie"))
        .unwrap_or_else(|| panic!("aucun cours en saisie pour le professeur : {enseignements}"));
    let cours_id = cours["cours_id"].as_str().expect("cours_id").to_string();
    let periodes_eva = b
        .jouer(
            "eva_cours_periodes",
            "GET /eva/cours/{id}/periodes",
            &format!("/eva/cours/{cours_id}/periodes"),
            Some(&prof),
            None,
        )
        .await;
    // La période du jour si elle existe (c'est celle que l'appli choisit par défaut), sinon la première.
    let p_eva = periodes_eva
        .as_array()
        .and_then(|a| {
            a.iter()
                .find(|p| {
                    p["date_debut"].as_str().is_some_and(|d| d <= jour.as_str())
                        && p["date_fin"].as_str().is_some_and(|f| f >= jour.as_str())
                })
                .or_else(|| a.first())
        })
        .unwrap_or_else(|| panic!("aucune période pour le cours : {periodes_eva}"))
        .clone();
    let periode_eva = id(&p_eva);
    let date_eval = if p_eva["date_debut"]
        .as_str()
        .is_some_and(|d| d <= jour.as_str())
        && p_eva["date_fin"]
            .as_str()
            .is_some_and(|f| f >= jour.as_str())
    {
        jour.clone()
    } else {
        p_eva["date_debut"]
            .as_str()
            .expect("date_debut")
            .to_string()
    };
    let evaluation = b
        .jouer(
            "eva_evaluations",
            "POST /eva/evaluations",
            "/eva/evaluations",
            Some(&prof),
            Some(json!({
                "cours_id": cours_id,
                "titre": "Interrogation (banc mobile)",
                "date_eval": date_eval,
                "bareme_m": 20000,
                "coef_m": 1000,
            })),
        )
        .await;
    let evaluation_id = id(&evaluation);
    let grille = b
        .jouer(
            "eva_grille",
            "GET /eva/grille",
            &format!("/eva/grille?cours_id={cours_id}&periode_id={periode_eva}"),
            Some(&prof),
            None,
        )
        .await;
    let rang = grille["evaluations"]
        .as_array()
        .and_then(|a| a.iter().position(|e| e["id"] == evaluation_id.as_str()))
        .unwrap_or_else(|| panic!("évaluation créée absente de la grille : {grille}"));
    let eleve_saisi = grille["eleves"]
        .as_array()
        .and_then(|a| {
            a.iter().find(|l| {
                let c = &l["cellules"][rang];
                c["concerne"] == true && c["saisie_ouverte"] == true
            })
        })
        .unwrap_or_else(|| panic!("aucune case ouverte dans la grille : {grille}"))["eleve_id"]
        .as_str()
        .expect("eleve_id")
        .to_string();
    let retour = b
        .jouer(
            "eva_saisie",
            "POST /eva/saisie",
            "/eva/saisie",
            Some(&prof),
            Some(json!({
                "cours_id": cours_id,
                "cellules": [
                    {"evaluation_id": evaluation_id, "eleve_id": eleve_saisi, "texte": "15,5", "version_attendue": null},
                ],
            })),
        )
        .await;
    assert_eq!(retour["cellules"][0]["issue"], "ecrit", "saisie : {retour}");
    assert_eq!(
        retour["cellules"][0]["valeur_m"], 15500,
        "saisie : {retour}"
    );
    // Seconde saisie avec une version périmée : le serveur rend le conflit, jamais un écrasement (l'appli le montre).
    let conflit = b
        .jouer(
            "eva_saisie_conflit",
            "POST /eva/saisie",
            "/eva/saisie",
            Some(&prof),
            Some(json!({
                "cours_id": cours_id,
                "cellules": [
                    {"evaluation_id": evaluation_id, "eleve_id": eleve_saisi, "texte": "abs", "version_attendue": null},
                ],
            })),
        )
        .await;
    assert_eq!(
        conflit["cellules"][0]["issue"], "conflit",
        "conflit : {conflit}"
    );
    b.jouer(
        "eva_evaluations_publication",
        "POST /eva/evaluations/{id}/publication",
        &format!("/eva/evaluations/{evaluation_id}/publication"),
        Some(&prof),
        Some(json!({"action": "maintenant"})),
    )
    .await;
    // Un lot de plusieurs cases : une réécriture identique sur la version lue (`inchange`) et une note hors barème
    // (`refuse`, avec son message) ; puis la grille relue, cases remplies et statistiques calculées (contre-revue M5, M6).
    let autre_eleve = grille["eleves"]
        .as_array()
        .and_then(|a| {
            a.iter().find(|l| {
                let c = &l["cellules"][rang];
                l["eleve_id"] != eleve_saisi.as_str()
                    && c["concerne"] == true
                    && c["saisie_ouverte"] == true
            })
        })
        .unwrap_or_else(|| panic!("une seule case ouverte dans la grille : {grille}"))["eleve_id"]
        .as_str()
        .expect("eleve_id")
        .to_string();
    let version_lue = retour["cellules"][0]["version"].clone();
    let lot = b
        .jouer(
            "eva_saisie_lot",
            "POST /eva/saisie",
            "/eva/saisie",
            Some(&prof),
            Some(json!({
                "cours_id": cours_id,
                "cellules": [
                    {"evaluation_id": evaluation_id, "eleve_id": eleve_saisi, "texte": "15,5", "version_attendue": version_lue},
                    {"evaluation_id": evaluation_id, "eleve_id": autre_eleve, "texte": "21", "version_attendue": null},
                ],
            })),
        )
        .await;
    assert_eq!(lot["cellules"][0]["issue"], "inchange", "lot : {lot}");
    assert_eq!(lot["cellules"][1]["issue"], "refuse", "lot : {lot}");
    assert!(lot["cellules"][1]["message"].is_string(), "lot : {lot}");
    let relue = b
        .jouer(
            "eva_grille_apres_saisie",
            "GET /eva/grille",
            &format!("/eva/grille?cours_id={cours_id}&periode_id={periode_eva}"),
            Some(&prof),
            None,
        )
        .await;
    assert!(
        !relue["evaluations"][rang]["stats"].is_null(),
        "statistiques absentes : {relue}"
    );
    assert!(
        relue["eleves"]
            .as_array()
            .is_some_and(|a| a.iter().any(|l| l["cellules"][rang]["valeur_m"] == 15500)),
        "case saisie absente de la grille relue : {relue}"
    );

    // --- Signaler un incident (registre des violations, socle 27, complément G5) ------------------------------
    // Tout personnel déclare en trente secondes et suit l'avancement de ses déclarations, rien d'autre (R-RG32).
    let declaration = b
        .jouer(
            "rgpd_violations_declarer",
            "POST /rgpd/violations/declarations",
            "/rgpd/violations/declarations",
            Some(&prof),
            Some(json!({
                "intitule": "Clé USB avec des copies perdue (banc mobile)",
                "description": "Perdue dans le bus.",
                "incident_securite": false,
            })),
        )
        .await;
    assert_eq!(declaration["avancement"], "recue", "déclaration : {declaration}");
    for champ in ["risque", "echeance_cnil_le", "chronologie"] {
        assert!(declaration.get(champ).is_none(), "{champ} exposé au déclarant : {declaration}");
    }
    b.jouer(
        "rgpd_violations_mes_declarations",
        "GET /rgpd/violations/declarations",
        "/rgpd/violations/declarations",
        Some(&prof),
        None,
    )
    .await;

    // Annonces : la direction en publie une pour tous et une pour les familles de la classe de Harry (sinon les
    // fixtures n'auraient que des listes vides, que tout modèle décode).
    for corps in [
        json!({"titre": "Sortie au musée (banc MB2)", "corps": "Départ 8 h.", "audience": "tous"}),
        json!({"titre": "Réunion parents (banc MB2)", "corps": "Salle B12.", "audience": "classe", "cible_classe": classe_lea}),
    ] {
        let r = envoyer(
            &b.app,
            "POST",
            "/annonces",
            Some(&cast.direction.token),
            &[],
            Some(&corps),
        )
        .await;
        assert_eq!(r.statut, StatusCode::CREATED, "annonce : {}", r.texte);
        let a = r.json();
        if let Some(i) = a["id"].as_str().or(a["annonce"]["id"].as_str()) {
            let _ = envoyer(
                &b.app,
                "POST",
                &format!("/annonces/{i}/confirmer"),
                Some(&cast.direction.token),
                &[],
                Some(&json!({})),
            )
            .await;
        }
    }

    // --- Messagerie, annonces, notifications --------------------------------------------------------------
    let fils = b
        .jouer(
            "messages_fils",
            "GET /messages/fils",
            "/messages/fils",
            Some(&parent),
            None,
        )
        .await;
    let fil = id(premier(&fils, "fils du parent de Harry"));
    b.jouer(
        "messages_fil",
        "GET /messages/fils/{id}",
        &format!("/messages/fils/{fil}"),
        Some(&parent),
        None,
    )
    .await;
    b.jouer(
        "messages_repondre",
        "POST /messages/fils/{id}/repondre",
        &format!("/messages/fils/{fil}/repondre"),
        Some(&parent),
        Some(json!({"corps": "Merci, bien reçu (banc MB2)."})),
    )
    .await;
    b.jouer("annonces", "GET /annonces", "/annonces", Some(&prof), None)
        .await;
    b.jouer(
        "eleves_annonces",
        "GET /eleves/{id}/annonces",
        &format!("/eleves/{lea}/annonces"),
        Some(&parent),
        None,
    )
    .await;
    b.jouer(
        "notifications",
        "GET /notifications",
        "/notifications",
        Some(&parent),
        None,
    )
    .await;

    // --- Fin de session : révocation d'un autre appareil, puis déconnexion ---------------------------------
    let autre = connecter(&b.app, &cast.parent_lea.email, etab).await;
    let autre_jeton = autre["token"].as_str().expect("token").to_string();
    let sessions = envoyer(
        &b.app,
        "GET",
        "/auth/sessions",
        Some(&autre_jeton),
        &[APPLI],
        None,
    )
    .await
    .json();
    let courante = sessions
        .as_array()
        .and_then(|a| {
            a.iter()
                .find(|s| s["current"] == json!(true) || s["courante"] == json!(true))
        })
        .map(id);
    let a_revoquer = sessions
        .as_array()
        .and_then(|a| a.iter().map(id).find(|i| Some(i) != courante.as_ref()))
        .expect("une autre session");
    b.jouer(
        "auth_sessions_revoquer",
        "DELETE /auth/sessions/{id}",
        &format!("/auth/sessions/{a_revoquer}"),
        Some(&autre_jeton),
        None,
    )
    .await;
    let autre = connecter(&b.app, &cast.parent_lea.email, etab).await;
    let jeton_fin = autre["token"].as_str().expect("token").to_string();
    b.jouer(
        "auth_logout",
        "POST /auth/logout",
        "/auth/logout",
        Some(&jeton_fin),
        None,
    )
    .await;
    Some((b, cast, jour))
}

#[tokio::test]
async fn contrat_mobile_fixtures_dorees() {
    let Some((b, _cast, _jour)) = parcours().await else {
        eprintln!("skip MB2 : ENT_DATABASE_URL");
        return;
    };
    let dir = dossier();
    let mut erreurs = Vec::new();
    let mut noms = BTreeSet::new();
    for c in &b.cas {
        assert!(noms.insert(c.nom.clone()), "cas en double : {}", c.nom);
        let attendu =
            json!({"route": c.route, "statut": c.statut.as_u16(), "corps": normaliser(&c.corps)});
        let fichier = dir.join(format!("{}.json", c.nom));
        if ecrire() {
            std::fs::create_dir_all(&dir).unwrap();
            let mut texte = serde_json::to_string_pretty(&attendu).unwrap();
            texte.push('\n');
            std::fs::write(&fichier, texte).unwrap();
            continue;
        }
        let Ok(texte) = std::fs::read_to_string(&fichier) else {
            erreurs.push(format!(
                "{} : fixture absente (ENT_CONTRAT_ECRIRE=1 pour la créer)",
                c.nom
            ));
            continue;
        };
        let fixture: Value =
            serde_json::from_str(&texte).unwrap_or_else(|e| panic!("{} : {e}", fichier.display()));
        if fixture["route"] != json!(c.route) || fixture["statut"] != json!(c.statut.as_u16()) {
            erreurs.push(format!(
                "{} : route ou statut changé ({} {} dans la fixture, {} {} servi)",
                c.nom, fixture["route"], fixture["statut"], c.route, c.statut
            ));
        }
        let mut e = Vec::new();
        ecarts(
            "$",
            &forme(&fixture["corps"]),
            &forme(&attendu["corps"]),
            &mut e,
        );
        erreurs.extend(e.into_iter().map(|x| format!("{} : {x}", c.nom)));
    }
    // Aucune fixture orpheline : les cœurs décodent tout le dossier.
    if !ecrire() {
        for entree in std::fs::read_dir(&dir).expect("contrat/mobile") {
            let p = entree.unwrap().path();
            if p.extension().is_some_and(|x| x == "json") {
                let nom = p.file_stem().unwrap().to_string_lossy().to_string();
                if !noms.contains(&nom) {
                    erreurs.push(format!(
                        "{nom}.json : fixture orpheline (aucun cas ne la produit)"
                    ));
                }
            }
        }
    }
    // Liste blanche : chaque route jouée y figure, chaque route listée est jouée ou expliquée.
    let routes = lire_routes();
    let jouees: BTreeSet<&str> = b.cas.iter().map(|c| c.route.as_str()).collect();
    for r in &jouees {
        if !routes.iter().any(|x| x == r) {
            erreurs.push(format!("{r} : jouée mais absente de routes.txt"));
        }
    }
    for r in &routes {
        let expliquee = NON_JOUEES.iter().any(|(x, _)| x == r);
        if !jouees.contains(r.as_str()) && !expliquee {
            erreurs.push(format!(
                "{r} : listée dans routes.txt mais ni jouée ni expliquée"
            ));
        }
        if jouees.contains(r.as_str()) && expliquee {
            erreurs.push(format!("{r} : jouée et pourtant listée comme non jouée"));
        }
    }
    assert!(
        erreurs.is_empty(),
        "contrat mobile rompu ; si le changement est voulu, régénérer avec ENT_CONTRAT_ECRIRE=1 puis mettre à jour \
         les cœurs Kotlin et Swift :\n  {}",
        erreurs.join("\n  ")
    );
}

/// Jeton d'un professeur d'un **autre** établissement de labo (même base).
async fn state_autre_etab(b: &Banc) -> String {
    let autre = b.ent.seed_labo_cast().await.expect("second cast");
    autre.prof.token
}

#[tokio::test]
async fn contrat_mobile_banc_de_parite() {
    let Some((b, cast, jour)) = parcours().await else {
        eprintln!("skip MB2 : ENT_DATABASE_URL");
        return;
    };
    let lea = cast.eleve_lea_id;
    let tom = cast.eleve_tom_id;
    let harry = cast.eleves.iter().find(|e| e.id == lea).expect("Harry");
    let draco = cast.eleves.iter().find(|e| e.id == tom).expect("Draco");
    let mut personas: Vec<(&str, &LaboPersona)> = vec![
        ("prof", &cast.prof),
        ("cpe", &cast.cpe),
        ("direction", &cast.direction),
        ("administratif", &cast.administratif),
        ("parent_harry", &cast.parent_lea),
        ("parent_draco", &cast.parent_tom),
    ];
    if let Some(p) = cast.profs.get(1) {
        personas.push(("autre_prof", p));
    }
    if let Some(c) = harry.compte.as_ref() {
        personas.push(("eleve_harry", c));
    }
    if let Some(c) = draco.compte.as_ref() {
        personas.push(("eleve_draco", c));
    }
    let periode = b
        .cas
        .iter()
        .find(|c| c.nom == "note_periodes")
        .map(|c| id(premier(&c.corps, "périodes")))
        .expect("périodes");
    let mut lectures: Vec<String> = vec![
        "/auth/me".into(),
        "/roles/mes-droits".into(),
        format!("/edt/mes-creneaux?jour={jour}"),
        "/famille/enfants".into(),
        format!("/vs/absences?jour={jour}"),
        "/note-periodes".into(),
        "/messages/fils".into(),
        "/annonces".into(),
        "/notifications".into(),
        "/eva/enseignements".into(),
        "/rgpd/violations/declarations".into(),
    ];
    for (eleve, classe) in [(lea, &harry.classe), (tom, &draco.classe)] {
        lectures.extend([
            format!("/eleves/{eleve}/edt?jour={jour}"),
            format!("/famille/enfants/{eleve}/resume"),
            format!("/eleves/{eleve}/notes"),
            format!("/eleves/{eleve}/moyennes?periode_id={periode}"),
            format!("/eleves/{eleve}/devoirs?periode_id={periode}"),
            format!("/cahier-texte?eleve_id={eleve}"),
            format!("/devoirs?periode_id={periode}&classe={classe}"),
            format!("/classes/{classe}/effectif?jour={jour}"),
            format!("/cahier-texte?classe={classe}"),
            format!("/eleves/{eleve}/annonces"),
        ]);
    }
    // Les lectures par identifiant jouées par le parcours (appel, fil) entrent aussi dans la matrice.
    for c in &b.cas {
        if c.route == "GET /appels/{id}" {
            lectures.push(format!("/appels/{}", id(&c.corps)));
        }
        if c.nom == "eva_cours_periodes" {
            lectures.push(c.uri.clone());
        }
        if c.nom == "eva_grille" {
            lectures.push(c.uri.clone());
        }
        if c.nom == "messages_fils" {
            lectures.push(format!("/messages/fils/{}", id(premier(&c.corps, "fils"))));
        }
    }

    let mut ecarts_parite = Vec::new();
    let mut matrice = BTreeMap::new();
    for (nom, p) in &personas {
        for uri in &lectures {
            let web = envoyer(&b.app, "GET", uri, Some(&p.token), &[], None).await;
            let appli = envoyer(&b.app, "GET", uri, Some(&p.token), &[APPLI], None).await;
            assert!(
                !web.statut.is_server_error() && !appli.statut.is_server_error(),
                "{nom} GET {uri} : erreur serveur ({} / {}) {}",
                web.statut,
                appli.statut,
                appli.texte
            );
            *matrice.entry(appli.statut.as_u16()).or_insert(0usize) += 1;
            if web.statut != appli.statut {
                ecarts_parite.push(format!(
                    "{nom} GET {uri} : web {} / appli {}",
                    web.statut, appli.statut
                ));
                continue;
            }
            if appli.statut.is_success() {
                if normaliser(&web.json()) != normaliser(&appli.json()) {
                    ecarts_parite.push(format!(
                        "{nom} GET {uri} : corps différent entre web et appli"
                    ));
                }
            } else {
                // L'appli reçoit l'enveloppe codée, jamais du texte nu.
                let e = appli.json();
                if e["erreur"]["code"].as_str().unwrap_or("").is_empty() {
                    ecarts_parite.push(format!(
                        "{nom} GET {uri} : erreur sans enveloppe codée : {}",
                        appli.texte
                    ));
                }
            }
        }
    }
    assert!(
        ecarts_parite.is_empty(),
        "parité web / appli rompue :\n  {}",
        ecarts_parite.join("\n  ")
    );
    // Cloisonnement attendu, cellule par cellule (contre-revue du lot 1, M1) : la parité seule serait vraie même
    // si web et appli fuyaient ensemble.
    let autre_etab = state_autre_etab(&b).await;
    let eleve_harry = harry.compte.as_ref().map(|c| c.token.clone());
    let mut refus_attendus: Vec<(&str, String, String)> = vec![
        (
            "parent_harry",
            cast.parent_lea.token.clone(),
            format!("/eleves/{tom}/notes"),
        ),
        (
            "parent_harry",
            cast.parent_lea.token.clone(),
            format!("/famille/enfants/{tom}/resume"),
        ),
        (
            "parent_harry",
            cast.parent_lea.token.clone(),
            format!("/eleves/{tom}/edt?jour={jour}"),
        ),
        (
            "parent_draco",
            cast.parent_tom.token.clone(),
            format!("/eleves/{lea}/notes"),
        ),
        (
            "parent_harry",
            cast.parent_lea.token.clone(),
            format!("/vs/absences?jour={jour}"),
        ),
        (
            "autre_etablissement",
            autre_etab.clone(),
            format!("/eleves/{lea}/notes"),
        ),
        (
            "autre_etablissement",
            autre_etab.clone(),
            format!("/famille/enfants/{lea}/resume"),
        ),
    ];
    // La grille d'évaluation : refus exacts plus bas (lectures et écritures `/eva/*`).
    let grille_uri = b
        .cas
        .iter()
        .find(|c| c.nom == "eva_grille")
        .map(|c| c.uri.clone())
        .expect("grille jouée");
    if let Some(t) = eleve_harry {
        refus_attendus.push(("eleve_harry", t.clone(), format!("/eleves/{tom}/notes")));
        refus_attendus.push(("eleve_harry", t, format!("/vs/absences?jour={jour}")));
    }
    let mut fuites = Vec::new();
    // Grille et écritures `/eva/*` (contre-revue E1, M1) : statut exact (403 ou 404, jamais 400, 401 ni 429) et
    // enveloppe codée, pour chaque persona qui n'enseigne pas ce cours, y compris un collègue du même établissement.
    let cas_de = |nom: &str| {
        b.cas
            .iter()
            .find(|c| c.nom == nom)
            .unwrap_or_else(|| panic!("{nom} non joué"))
            .corps
            .clone()
    };
    let evaluation = cas_de("eva_evaluations");
    let ev_id = id(&evaluation);
    let cours_id = cas_de("eva_grille")["enseignement"]["cours_id"]
        .as_str()
        .expect("cours")
        .to_string();
    let eleve_eva = cas_de("eva_saisie")["cellules"][0]["eleve_id"]
        .as_str()
        .expect("élève")
        .to_string();
    let autre_prof = cast
        .profs
        .get(1)
        .expect("second professeur du labo")
        .token
        .clone();
    let eleve_tok = harry
        .compte
        .as_ref()
        .expect("compte élève de Harry")
        .token
        .clone();
    let mut refus_eva: Vec<(&str, String)> = vec![
        ("autre_prof", autre_prof),
        ("parent_harry", cast.parent_lea.token.clone()),
        ("eleve_harry", eleve_tok),
        ("autre_etablissement", state_autre_etab(&b).await),
    ];
    let saisie = json!({
        "cours_id": cours_id,
        "cellules": [{"evaluation_id": ev_id, "eleve_id": eleve_eva, "texte": "2", "version_attendue": null}],
    });
    let creation = json!({
        "cours_id": cours_id, "titre": "Intrusion", "date_eval": evaluation["date_eval"],
        "bareme_m": 20000, "coef_m": 1000,
    });
    for (nom, jeton) in refus_eva.drain(..) {
        let ecritures: [(&str, String, Option<&Value>); 5] = [
            ("GET", grille_uri.clone(), None),
            ("GET", format!("/eva/cours/{cours_id}/periodes"), None),
            ("POST", "/eva/saisie".into(), Some(&saisie)),
            ("POST", "/eva/evaluations".into(), Some(&creation)),
            (
                "POST",
                format!("/eva/evaluations/{ev_id}/publication"),
                Some(&json!({"action": "retirer", "motif": "intrusion"})),
            ),
        ];
        for (methode, uri, corps) in ecritures {
            let cle = Uuid::new_v4().to_string();
            let r = envoyer(
                &b.app,
                methode,
                &uri,
                Some(&jeton),
                &[APPLI, ("idempotency-key", cle.as_str())],
                corps,
            )
            .await;
            let code = r.json()["erreur"]["code"]
                .as_str()
                .unwrap_or("")
                .to_string();
            if !matches!(r.statut.as_u16(), 403 | 404) || code.is_empty() {
                fuites.push(format!(
                    "{nom} {methode} {uri} : {} (code « {code} ») au lieu de 403 ou 404 codé",
                    r.statut
                ));
            }
        }
    }
    // La grille n'a pas bougé : aucune écriture refusée n'a laissé de trace.
    let relue = envoyer(
        &b.app,
        "GET",
        &grille_uri,
        Some(&cast.prof.token),
        &[APPLI],
        None,
    )
    .await;
    let relue = relue.json();
    assert!(
        relue["evaluations"]
            .as_array()
            .is_some_and(|a| a.iter().all(|e| e["titre"] != "Intrusion")),
        "évaluation créée par un tiers : {relue}"
    );
    for (nom, jeton, uri) in &refus_attendus {
        for avec_appli in [false, true] {
            let en_tetes: &[(&str, &str)] = if avec_appli { &[APPLI] } else { &[] };
            let r = envoyer(&b.app, "GET", uri, Some(jeton), en_tetes, None).await;
            // 403 ou 404 seulement : un 400, 401 ou 429 ne prouverait pas le cloisonnement (contre-revue M1).
            if !matches!(r.statut.as_u16(), 403 | 404) {
                fuites.push(format!(
                    "{nom} GET {uri} (appli : {avec_appli}) : {} au lieu de 403 ou 404",
                    r.statut
                ));
            }
        }
    }
    assert!(
        fuites.is_empty(),
        "cloisonnement rompu :\n  {}",
        fuites.join("\n  ")
    );

    // Le banc doit voir des refus (cloisonnement) et des succès : sinon il ne prouve rien.
    assert!(
        matrice.get(&200).is_some_and(|n| *n > 20),
        "trop peu de succès : {matrice:?}"
    );
    assert!(
        matrice
            .iter()
            .any(|(s, n)| (400..500).contains(s) && *n > 0),
        "aucun refus dans la matrice : {matrice:?}"
    );
}
