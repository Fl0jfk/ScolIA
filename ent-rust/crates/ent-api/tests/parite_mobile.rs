//! Socle 11, complément du palier MB2 : **banc de parité web / appli** sur la matrice de
//! `docs/socles/11_APPLICATIONS_MOBILES.md` §7.2, contre le routeur réel et deux établissements de labo (harnais de la
//! matrice de séparation).
//!
//! Ce que le banc MB2 (`ent-api/src/tests_contrat_mobile.rs`) prouvait déjà : fixtures dorées, liste blanche, parité de
//! 9 personas sur une **même** session (seul l'en-tête `X-ScolIA-Appli` changeait). Ce banc ajoute :
//!
//! 1. **Deux sessions par personne** : la session web du harnais (connexion sans `client`, sans en-tête) et une session
//!    ouverte par l'appli (`"client":"mobile"`, `X-ScolIA-Appli`). Les deux portent un jeton Bearer : l'API ne lit
//!    pas de cookie (le cookie du web est tenu par `ent-web-server`, qui appelle l'API avec le jeton). Tant que la clé
//!    d'appareil n'existe pas (MB4), la session de l'appli ouverte par mot de passe est de nature web (MEN-11) : le
//!    banc le vérifie, pour que cette hypothèse ne change pas en silence.
//! 2. **Douze personas** (les 12 acteurs du harnais).
//! 3. **Cibles** : pour un élève ou un fil, lié / non lié / autre établissement / inexistant ; pour un appel ou un
//!    cours, celui du labo / autre établissement / inexistant ; pour une classe, la classe liée et une autre.
//!    L'objet de l'autre établissement doit recevoir **exactement** la réponse de l'inexistant (oracle d'existence).
//! 4. **Écritures refusées** : même refus au web et dans l'appli, enveloppe codée, indistinct de l'inexistant (autre
//!    établissement), **aucune trace** sur les objets visés (relus avant et après) ; un **témoin** autorisé joue
//!    ensuite chaque geste pour prouver que le chemin et le corps étaient valides.
//! 5. **Droits du lien** : parent sans droit aux notes (Molly, sur Ginny), sans droit de justifier (Narcissa, sur
//!    Draco), élève sur un camarade, avec contrôles positifs.
//! 6. **Complétude** : chaque route de `contrat/mobile/routes.txt` est jouée ici, ou exclue avec sa raison ; chaque
//!    chemin joué correspond à son gabarit (§7.2, point 7).
//! 7. **Oracle de fuite** : aucune réponse ne contient un identifiant connu de l'autre établissement (ceux du harnais,
//!    plus le cours, les périodes et les créneaux de B).
//!
//! Hors de portée (documenté dans `docs/socles/11_MB2_PARITE_COMPLETE.md`) : manifeste `/mobile/contexte` (MB5), baux
//! et opérations différées (MB7), session signée par la clé d'appareil (MB4), remplaçant, AED, élève majeur, parent
//! séparé ou interdit (absents du labo).
//!
//! Lancement (comme la CI) : `ENT_ENV=test ENT_DATABASE_URL=postgres://…/ent_ci_test cargo test -p ent-api --features
//! labo --test parite_mobile -- --test-threads 1`. Sans base : SKIP.

#![cfg(feature = "labo")]
#![allow(dead_code)]

#[path = "separation/harness.rs"]
mod harness;

use std::collections::{BTreeMap, BTreeSet};

use axum::http::StatusCode;
use harness::*;
use serde_json::{json, Value};
use uuid::Uuid;

const APPLI: &str = "android/1.4.2 (build 142)";

/// Routes de la liste blanche que ce banc ne joue pas, et pourquoi.
const EXCLUES: &[(&str, &str)] = &[
    ("GET /auth/etablissement", "sans session : rattachement par domaine, joué par le banc MB2 (fixture)"),
    ("POST /auth/login", "ouvre la session : joué ici pour chaque persona (session de l'appli)"),
    ("POST /auth/passkey/login/start", "sans session : défi anti-énumération, joué par le banc MB2"),
    ("POST /auth/passkey/login/finish", "cérémonie WebAuthn : authentificateur réel (MB4)"),
    ("POST /auth/passkey/register/start", "propre à la session : joué par le banc MB2"),
    ("POST /auth/passkey/register/finish", "cérémonie WebAuthn : authentificateur réel (MB4)"),
    ("POST /auth/logout", "propre à la session : ne vise aucun objet d'autrui"),
    ("POST /devices/push", "propre à la session : inscription de l'appareil de la personne (recette MB0)"),
    (
        "POST /eva/evaluations",
        "refus exacts déjà prouvés par le banc MB2 (autre professeur, parent, élève, autre établissement)",
    ),
    ("POST /eva/saisie", "idem : banc MB2"),
    ("POST /eva/evaluations/{id}/publication", "idem : banc MB2"),
];

/// Comment comparer la réponse web et celle de l'appli.
#[derive(Clone, Copy, PartialEq)]
enum Comparer {
    /// Même statut et même corps (clé `trace` exceptée).
    Corps,
    /// Liste des sessions : mêmes identifiants, une seule session courante de chaque côté, et pas la même.
    Sessions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Visee {
    Liee,
    NonLiee,
    AutreEtab,
    Inexistant,
}

/// Une lecture : gabarit de `routes.txt` et chemin joué. Marqueurs : `{eleve}`, `{fil}`, `{appel}`, `{cours}`,
/// `{classe}`, `{jour}`, `{periode}`, `{eva_periode}`.
struct Lecture {
    route: &'static str,
    chemin: &'static str,
    comparer: Comparer,
}

const fn lecture(route: &'static str, chemin: &'static str) -> Lecture {
    Lecture {
        route,
        chemin,
        comparer: Comparer::Corps,
    }
}

const LECTURES: &[Lecture] = &[
    lecture("GET /auth/me", "/auth/me"),
    lecture("GET /roles/mes-droits", "/roles/mes-droits"),
    Lecture {
        route: "GET /auth/sessions",
        chemin: "/auth/sessions",
        comparer: Comparer::Sessions,
    },
    lecture("GET /edt/mes-creneaux", "/edt/mes-creneaux?jour={jour}"),
    lecture("GET /eleves/{id}/edt", "/eleves/{eleve}/edt?jour={jour}"),
    lecture("GET /appels/{id}", "/appels/{appel}"),
    lecture("GET /famille/enfants", "/famille/enfants"),
    lecture(
        "GET /famille/enfants/{id}/resume",
        "/famille/enfants/{eleve}/resume",
    ),
    lecture("GET /vs/absences", "/vs/absences?jour={jour}"),
    lecture("GET /vs/absences", "/vs/absences"),
    lecture("GET /note-periodes", "/note-periodes"),
    lecture("GET /eleves/{id}/notes", "/eleves/{eleve}/notes"),
    lecture(
        "GET /eleves/{id}/moyennes",
        "/eleves/{eleve}/moyennes?periode_id={periode}",
    ),
    lecture(
        "GET /devoirs",
        "/devoirs?periode_id={periode}&classe={classe}",
    ),
    lecture(
        "GET /eleves/{id}/devoirs",
        "/eleves/{eleve}/devoirs?periode_id={periode}",
    ),
    lecture(
        "GET /classes/{classe}/effectif",
        "/classes/{classe}/effectif?jour={jour}",
    ),
    lecture("GET /cahier-texte", "/cahier-texte?classe={classe}"),
    lecture("GET /cahier-texte", "/cahier-texte?eleve_id={eleve}"),
    lecture("GET /messages/fils", "/messages/fils"),
    lecture("GET /messages/fils", "/messages/fils?eleve_id={eleve}"),
    lecture("GET /messages/fils/{id}", "/messages/fils/{fil}"),
    lecture("GET /annonces", "/annonces"),
    lecture("GET /eleves/{id}/annonces", "/eleves/{eleve}/annonces"),
    lecture("GET /notifications", "/notifications"),
    lecture("GET /eva/enseignements", "/eva/enseignements"),
    lecture(
        "GET /eva/cours/{id}/periodes",
        "/eva/cours/{cours}/periodes",
    ),
    lecture(
        "GET /eva/grille",
        "/eva/grille?cours_id={cours}&periode_id={eva_periode}",
    ),
];

/// Écritures dont le banc éprouve les refus, puis le témoin autorisé.
const ECRITURES: &[&str] = &[
    "POST /appels",
    "POST /appels/{id}/lignes",
    "POST /appels/{id}/clore",
    "POST /absences/{id}/justifier",
    "POST /messages/fils/{id}/repondre",
    "DELETE /auth/sessions/{id}",
    "DELETE /devices/push/{id}",
];

/// Visées jouées pour un chemin, selon ses marqueurs.
fn visees(chemin: &str) -> &'static [Visee] {
    if chemin.contains("{eleve}") || chemin.contains("{fil}") {
        &[
            Visee::Liee,
            Visee::NonLiee,
            Visee::AutreEtab,
            Visee::Inexistant,
        ]
    } else if chemin.contains("{appel}") || chemin.contains("{cours}") {
        &[Visee::Liee, Visee::AutreEtab, Visee::Inexistant]
    } else if chemin.contains("{classe}") {
        &[Visee::Liee, Visee::NonLiee]
    } else {
        &[Visee::Liee]
    }
}

fn routes_txt() -> Vec<String> {
    let chemin =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contrat/mobile/routes.txt");
    std::fs::read_to_string(&chemin)
        .unwrap_or_else(|e| panic!("{} : {e}", chemin.display()))
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

/// Le chemin (sans la requête) correspond-il au gabarit `MÉTHODE /a/{x}/b` ? Les marqueurs du banc valent un segment.
fn correspond(route: &str, chemin: &str) -> bool {
    let gabarit = route.split_once(' ').map_or(route, |(_, g)| g);
    let chemin = chemin.split('?').next().unwrap_or(chemin);
    let (g, c): (Vec<_>, Vec<_>) = (gabarit.split('/').collect(), chemin.split('/').collect());
    g.len() == c.len()
        && g.iter()
            .zip(&c)
            .all(|(g, c)| (g.starts_with('{') && !c.is_empty()) || g == c)
}

/// §7.2 point 7 : aucune route de la liste blanche sans ligne dans ce banc, aucune ligne du banc hors de la liste,
/// chaque chemin joué fidèle à son gabarit. Ne demande pas de base.
#[test]
fn chaque_route_de_la_liste_blanche_est_jouee_ou_exclue() {
    let liste: BTreeSet<String> = routes_txt().into_iter().collect();
    let mut couvertes: BTreeMap<String, &str> = BTreeMap::new();
    for l in LECTURES {
        assert!(
            correspond(l.route, l.chemin),
            "{} ne correspond pas à {}",
            l.chemin,
            l.route
        );
        couvertes.insert(l.route.into(), "lecture");
    }
    for e in ECRITURES {
        assert!(
            couvertes.insert((*e).into(), "écriture").is_none(),
            "{e} en double"
        );
    }
    for (r, raison) in EXCLUES {
        assert!(!raison.trim().is_empty(), "{r} : exclusion sans raison");
        assert!(
            couvertes.insert((*r).into(), "exclue").is_none(),
            "{r} à la fois jouée et exclue"
        );
    }
    let manquantes: Vec<_> = liste
        .iter()
        .filter(|r| !couvertes.contains_key(*r))
        .collect();
    let perimees: Vec<_> = couvertes.keys().filter(|r| !liste.contains(*r)).collect();
    assert!(
        manquantes.is_empty(),
        "routes de routes.txt sans ligne dans le banc de parité (les jouer, ou les exclure avec leur raison) : {manquantes:?}"
    );
    assert!(
        perimees.is_empty(),
        "lignes du banc hors de routes.txt (route retirée ?) : {perimees:?}"
    );
    assert!(correspond(
        "GET /eleves/{id}/notes",
        "/eleves/{eleve}/notes"
    ));
    assert!(!correspond(
        "GET /eleves/{id}/notes",
        "/eleve/{eleve}/notes"
    ));
}

// ───────────────────────── Banc ─────────────────────────

struct Banc<'a> {
    m: &'a Monde,
    /// Session ouverte par l'appli, par acteur de l'établissement A.
    appli: BTreeMap<Acteur, String>,
    /// Jour de l'appel ouvert par le harnais : un jour avec des créneaux pour le professeur.
    jour: String,
    periode: String,
    eva_periode: String,
    eva_periode_b: String,
    /// Identifiants de B découverts ici, en plus de ceux du harnais (oracle de fuite).
    ids_b: BTreeSet<String>,
}

fn jeton_b(m: &Monde, a: Acteur) -> &str {
    m.b.jetons.get(&a).map(String::as_str).expect("acteur de B")
}

impl<'a> Banc<'a> {
    async fn ouvrir(m: &'a Monde) -> Banc<'a> {
        let o = &m.a.objets;
        let ob = &m.b.objets;
        // Objets du labo dont le banc a besoin : leur absence est une panne du semis, pas une cellule à sauter.
        for (nom, v) in [
            ("cours de A", o.cours),
            ("cours de B", ob.cours),
            ("appel de A", o.appel),
            ("appel de B", ob.appel),
            ("fil de Harry", o.fil),
            ("fil de Draco", o.fil_autre),
            ("fil de B", ob.fil),
            ("absence de Harry", o.absence),
            ("absence de Draco", o.absence_autre),
            ("absence de B", ob.absence),
        ] {
            assert!(v.is_some(), "{nom} absent du labo");
        }
        let mut appli = BTreeMap::new();
        for a in Acteur::TOUS {
            let r = m
                .requete(
                    "POST",
                    "/auth/login",
                    None,
                    Some(&json!({
                        "email": m.a.emails[&a], "password": m.a.mot_de_passe,
                        "etablissement_id": o.etab, "client": "mobile"
                    })),
                    &[("x-scolia-appli", APPLI.into())],
                )
                .await;
            assert_eq!(
                r.statut,
                StatusCode::OK,
                "connexion appli {a:?} : {}",
                r.texte
            );
            let jeton = r.json["token"].as_str().expect("jeton").to_string();
            assert_ne!(
                jeton,
                m.jeton(a),
                "{a:?} : l'appli doit avoir sa propre session"
            );
            // MEN-11 / MB0 : sans clé d'appareil (MB4), une session « mobile » par mot de passe est une session web.
            let moi = m
                .requete(
                    "GET",
                    "/auth/me",
                    Some(&jeton),
                    None,
                    &[("x-scolia-appli", APPLI.into())],
                )
                .await;
            assert_eq!(
                moi.json["client"], "web",
                "{a:?} : nature de la session de l'appli changée (MB4 ?)"
            );
            assert_eq!(
                moi.json["portee"], "complete",
                "{a:?} : session de l'appli restreinte"
            );
            appli.insert(a, jeton);
        }
        let prof = m.jeton(Acteur::Prof);
        let lu = m
            .appel(
                "GET",
                &format!("/appels/{}", o.appel.unwrap()),
                Some(prof),
                None,
            )
            .await;
        let jour = lu.json["jour"]
            .as_str()
            .unwrap_or_else(|| panic!("jour de l'appel : {}", lu.texte))
            .to_string();
        let creneaux = m
            .appel(
                "GET",
                &format!("/edt/mes-creneaux?jour={jour}"),
                Some(prof),
                None,
            )
            .await;
        assert!(
            !creneaux.est_vide(),
            "le professeur n'a pas de créneau le {jour} : banc trivial"
        );
        let periodes = m.appel("GET", "/note-periodes", Some(prof), None).await;
        let periode = premier_id(&periodes.json)
            .unwrap_or_else(|| panic!("périodes de notes : {}", periodes.texte));
        let eva = |jeton: &'a str, cours: Uuid| async move {
            let r = m
                .appel(
                    "GET",
                    &format!("/eva/cours/{cours}/periodes"),
                    Some(jeton),
                    None,
                )
                .await;
            premier_id(&r.json).unwrap_or_else(|| panic!("périodes du cours {cours} : {}", r.texte))
        };
        let eva_periode = eva(prof, o.cours.unwrap()).await;
        let prof_b = jeton_b(m, Acteur::Prof);
        let eva_periode_b = eva(prof_b, ob.cours.unwrap()).await;

        let mut ids_b: BTreeSet<String> = BTreeSet::new();
        ids_b.insert(ob.cours.unwrap().to_string());
        ids_b.insert(eva_periode_b.clone());
        let lb = m
            .appel(
                "GET",
                &format!("/appels/{}", ob.appel.unwrap()),
                Some(prof_b),
                None,
            )
            .await;
        let jour_b = lb.json["jour"].as_str().unwrap_or(&jour).to_string();
        for chemin in [
            "/note-periodes".to_string(),
            format!("/eva/cours/{}/periodes", ob.cours.unwrap()),
            format!("/edt/mes-creneaux?jour={jour_b}"),
            "/eva/enseignements".to_string(),
        ] {
            let r = m.appel("GET", &chemin, Some(prof_b), None).await;
            for v in r.json.as_array().into_iter().flatten() {
                for cle in ["id", "creneau_id", "cours_id"] {
                    if let Some(i) = v[cle].as_str() {
                        ids_b.insert(i.to_string());
                    }
                }
            }
        }
        // Les identifiants partagés par les deux établissements (aucun attendu) ne prouveraient pas une fuite.
        for i in &m.a.objets.tous_ids {
            ids_b.remove(i);
        }
        Banc {
            m,
            appli,
            jour,
            periode,
            eva_periode,
            eva_periode_b,
            ids_b,
        }
    }

    fn web(&self, a: Acteur) -> &str {
        self.m.jeton(a)
    }

    fn appli(&self, a: Acteur) -> &str {
        &self.appli[&a]
    }

    /// Chemin joué pour `(acteur, visée)`, `None` quand la cellule n'a pas de sens par construction (Molly n'a pas de
    /// fil à elle dans le labo).
    fn chemin(&self, gabarit: &str, a: Acteur, v: Visee) -> Option<String> {
        let o = &self.m.a.objets;
        let b = &self.m.b.objets;
        let x = Uuid::new_v4().to_string();
        let s = |u: Option<Uuid>| u.expect("objet vérifié à l'ouverture").to_string();
        // L'élève (et son fil, sa classe) lié à l'acteur, et un élève qui ne l'est pas, du même établissement.
        let (eleve, non_lie, fil, fil_non_lie, classe, classe_autre) = match a {
            Acteur::ParentAutreFoyer => (
                o.eleve_autre,
                o.eleve,
                Some(s(o.fil_autre)),
                s(o.fil),
                "5A",
                "4B",
            ),
            Acteur::ParentSansDrapeaux => (o.eleve_ginny, o.eleve, None, s(o.fil), "4B", "5A"),
            _ => (
                o.eleve,
                o.eleve_autre,
                Some(s(o.fil)),
                s(o.fil_autre),
                "4B",
                "5A",
            ),
        };
        let val = |cle: &str| -> Option<String> {
            Some(match (cle, v) {
                (_, Visee::Inexistant) => x.clone(),
                ("{eleve}", Visee::Liee) => eleve.to_string(),
                ("{eleve}", Visee::NonLiee) => non_lie.to_string(),
                ("{eleve}", Visee::AutreEtab) => b.eleve.to_string(),
                ("{fil}", Visee::Liee) => fil.clone()?,
                ("{fil}", Visee::NonLiee) => fil_non_lie.clone(),
                ("{fil}", Visee::AutreEtab) => s(b.fil),
                ("{appel}", Visee::AutreEtab) => s(b.appel),
                ("{appel}", _) => s(o.appel),
                ("{cours}", Visee::AutreEtab) => s(b.cours),
                ("{cours}", _) => s(o.cours),
                ("{eva_periode}", Visee::AutreEtab) => self.eva_periode_b.clone(),
                ("{eva_periode}", _) => self.eva_periode.clone(),
                ("{classe}", Visee::NonLiee) => classe_autre.to_string(),
                ("{classe}", _) => classe.to_string(),
                ("{jour}", _) => self.jour.clone(),
                ("{periode}", _) => self.periode.clone(),
                _ => unreachable!("marqueur {cle}"),
            })
        };
        let mut s = gabarit.to_string();
        for cle in [
            "{eleve}",
            "{fil}",
            "{appel}",
            "{cours}",
            "{eva_periode}",
            "{classe}",
            "{jour}",
            "{periode}",
        ] {
            if s.contains(cle) {
                // Inexistant : seul l'identifiant de l'objet est inventé, les paramètres restent valides.
                let v = if v == Visee::Inexistant
                    && ["{classe}", "{jour}", "{periode}"].contains(&cle)
                {
                    self.chemin_parametre(cle, a)
                } else {
                    val(cle)?
                };
                s = s.replace(cle, &v);
            }
        }
        Some(s)
    }

    fn chemin_parametre(&self, cle: &str, a: Acteur) -> String {
        match cle {
            "{jour}" => self.jour.clone(),
            "{periode}" => self.periode.clone(),
            _ => (if a == Acteur::ParentAutreFoyer {
                "5A"
            } else {
                "4B"
            })
            .to_string(),
        }
    }

    async fn lire_web(&self, a: Acteur, chemin: &str) -> Rep {
        self.m
            .requete("GET", chemin, Some(self.web(a)), None, &[])
            .await
    }

    async fn lire_appli(&self, a: Acteur, chemin: &str) -> Rep {
        self.m
            .requete(
                "GET",
                chemin,
                Some(self.appli(a)),
                None,
                &[("x-scolia-appli", APPLI.into())],
            )
            .await
    }

    async fn ecrire_appli(
        &self,
        a: Acteur,
        methode: &str,
        chemin: &str,
        corps: Option<&Value>,
    ) -> Rep {
        self.m
            .requete(
                methode,
                chemin,
                Some(self.appli(a)),
                corps,
                &[
                    ("x-scolia-appli", APPLI.into()),
                    ("idempotency-key", Uuid::new_v4().to_string()),
                ],
            )
            .await
    }

    async fn ecrire_web(
        &self,
        a: Acteur,
        methode: &str,
        chemin: &str,
        corps: Option<&Value>,
    ) -> Rep {
        self.m
            .requete(methode, chemin, Some(self.web(a)), corps, &[])
            .await
    }

    /// Identifiants de B présents dans la réponse, hors ceux que la requête a elle-même fournis.
    fn fuites_b(&self, chemin: &str, texte: &str) -> Vec<String> {
        let mut out: Vec<String> = fuites(self.m, texte, &[])
            .into_iter()
            .filter(|f| {
                f.strip_prefix("identifiant de B : ")
                    .is_none_or(|id| !chemin.contains(id))
            })
            .collect();
        for id in &self.ids_b {
            if texte.contains(id.as_str()) && !chemin.contains(id.as_str()) {
                out.push(format!("identifiant de B : {id}"));
            }
        }
        out
    }
}

fn premier_id(v: &Value) -> Option<String> {
    v.as_array()?.first()?["id"].as_str().map(String::from)
}

fn code(r: &Rep) -> String {
    r.json["erreur"]["code"].as_str().unwrap_or("").to_string()
}

/// Corps comparable : la clé `trace` (propre à chaque requête) est retirée partout.
fn sans_trace(v: &Value) -> Value {
    match v {
        Value::Object(o) => Value::Object(
            o.iter()
                .filter(|(k, _)| k.as_str() != "trace")
                .map(|(k, x)| (k.clone(), sans_trace(x)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(sans_trace).collect()),
        x => x.clone(),
    }
}

/// Signature d'une réponse pour l'oracle d'existence : statut et code (en succès, le corps).
fn signature(r: &Rep) -> String {
    if r.statut.is_success() {
        format!("{} {}", r.statut.as_u16(), sans_trace(&r.json))
    } else {
        format!("{} {}", r.statut.as_u16(), code(r))
    }
}

/// Sessions : mêmes identifiants des deux côtés, une seule courante de chaque côté, et pas la même.
fn sessions_conformes(web: &Value, appli: &Value) -> bool {
    let ids = |v: &Value| -> BTreeSet<String> {
        v.as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| s["id"].as_str().map(String::from))
            .collect()
    };
    let courante = |v: &Value| -> Vec<String> {
        v.as_array()
            .into_iter()
            .flatten()
            .filter(|s| s["courante"] == true)
            .filter_map(|s| s["id"].as_str().map(String::from))
            .collect()
    };
    let (cw, ca) = (courante(web), courante(appli));
    ids(web) == ids(appli) && cw.len() == 1 && ca.len() == 1 && cw != ca
}

macro_rules! monde_ou_skip {
    () => {
        match monde() {
            Some(m) => m,
            None => return eprintln!("SKIP : pas de base de test"),
        }
    };
}

// ───────────────────────── Lectures ─────────────────────────

#[test]
fn lectures_meme_reponse_au_web_et_dans_l_appli_pour_douze_personas() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let b = Banc::ouvrir(m).await;
        let mut ecarts = Vec::new();
        let mut statuts: BTreeMap<u16, usize> = BTreeMap::new();
        // Couverture par (chemin, visée) : chaque cellule prévue est jouée au moins une fois.
        let mut couverture: BTreeMap<(&str, Visee), usize> = BTreeMap::new();
        // Au moins une réponse non vide par acteur : un acteur refusé partout ne prouverait rien.
        let mut contenu: BTreeMap<Acteur, usize> = BTreeMap::new();
        for a in Acteur::TOUS {
            for l in LECTURES {
                let mut sig_autre = None;
                let mut sig_inexistant = None;
                for v in visees(l.chemin) {
                    let Some(chemin) = b.chemin(l.chemin, a, *v) else {
                        continue;
                    };
                    *couverture.entry((l.chemin, *v)).or_default() += 1;
                    let web = b.lire_web(a, &chemin).await;
                    let appli = b.lire_appli(a, &chemin).await;
                    *statuts.entry(appli.statut.as_u16()).or_default() += 1;
                    let cell = format!("{a:?} {v:?} GET {chemin}");
                    if web.statut.is_server_error() || appli.statut.is_server_error() {
                        ecarts.push(format!("{cell} : erreur serveur (web {} / appli {}) {}", web.statut, appli.statut, appli.texte));
                        continue;
                    }
                    if web.statut != appli.statut {
                        ecarts.push(format!("{cell} : web {} / appli {}", web.statut, appli.statut));
                        continue;
                    }
                    if appli.statut.is_success() {
                        let egal = match l.comparer {
                            Comparer::Corps => sans_trace(&web.json) == sans_trace(&appli.json),
                            Comparer::Sessions => sessions_conformes(&web.json, &appli.json),
                        };
                        if !egal {
                            ecarts.push(format!("{cell} : corps différent entre web et appli"));
                        }
                        if !appli.est_vide() {
                            *contenu.entry(a).or_default() += 1;
                        }
                    } else if code(&appli).is_empty() {
                        ecarts.push(format!("{cell} : erreur sans enveloppe codée dans l'appli : {}", appli.texte));
                    }
                    for f in b.fuites_b(&chemin, &web.texte).into_iter().chain(b.fuites_b(&chemin, &appli.texte)) {
                        ecarts.push(format!("{cell} : fuite, {f}"));
                    }
                    // Cloisonnement des familles : jamais l'élève, le fil ou la classe d'une autre famille.
                    if a.est_famille() && *v == Visee::NonLiee {
                        let par_objet = l.chemin.contains("{eleve}") || l.chemin.contains("{fil}");
                        if par_objet && !appli.est_refus() {
                            ecarts.push(format!("{cell} : {} au lieu d'un refus (objet d'une autre famille)", appli.statut));
                        }
                        // Constat PM-03 : un élève lit le cahier de textes d'une autre classe (au web comme dans
                        // l'appli). Laissé visible ici, à trancher par le socle 20.
                        let pm03 = a == Acteur::Eleve && l.chemin == "/cahier-texte?classe={classe}";
                        if !par_objet && !pm03 && appli.statut.is_success() && !appli.est_vide() {
                            ecarts.push(format!("{cell} : classe d'une autre famille servie"));
                        }
                    }
                    if *v == Visee::AutreEtab && appli.statut.is_success() && !appli.est_vide() {
                        ecarts.push(format!("{cell} : objet de l'autre établissement servi ({})", appli.statut));
                    }
                    let sig = (signature(&web), signature(&appli));
                    match v {
                        Visee::AutreEtab => sig_autre = Some(sig),
                        Visee::Inexistant => sig_inexistant = Some(sig),
                        _ => {}
                    }
                }
                if visees(l.chemin).contains(&Visee::AutreEtab) {
                    match (&sig_autre, &sig_inexistant) {
                        (Some(x), Some(y)) if x == y => {}
                        (x, y) => ecarts.push(format!(
                            "{a:?} GET {} : l'objet de l'autre établissement se distingue d'un inexistant ({x:?} / {y:?})",
                            l.chemin
                        )),
                    }
                }
            }
        }
        for l in LECTURES {
            for v in visees(l.chemin) {
                let n = couverture.get(&(l.chemin, *v)).copied().unwrap_or(0);
                // Toutes les visées pour les 12 acteurs, sauf le fil « lié » de Molly (elle n'en a pas).
                let attendu = if l.chemin.contains("{fil}") && *v == Visee::Liee { 11 } else { 12 };
                if n != attendu {
                    ecarts.push(format!("{} {v:?} : {n} cellules jouées au lieu de {attendu}", l.chemin));
                }
            }
        }
        for a in Acteur::TOUS {
            if contenu.get(&a).copied().unwrap_or(0) < 2 {
                ecarts.push(format!("{a:?} : moins de deux réponses non vides, le banc ne prouve rien pour lui"));
            }
        }
        assert!(ecarts.is_empty(), "parité ou cloisonnement rompus ({} écarts) :\n  {}", ecarts.len(), ecarts.join("\n  "));
        let refus: usize = statuts.iter().filter(|(s, _)| (400..500).contains(*s)).map(|(_, n)| n).sum();
        assert!(refus > 150, "trop peu de refus dans la matrice : {statuts:?}");
    });
}

/// Droits du lien : un parent sans droit aux notes ne les lit pas, ni au web ni dans l'appli ; un parent qui a ce
/// droit les lit (contrôle : la cellule de refus n'est pas un refus de principe).
#[test]
fn droits_du_lien_appliques_a_l_identique() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let b = Banc::ouvrir(m).await;
        let o = &m.a.objets;
        let p = &b.periode;
        let mut ecarts = Vec::new();
        let cellules: [(Acteur, String, bool); 6] = [
            // Molly n'a pas le droit aux notes de Ginny.
            (
                Acteur::ParentSansDrapeaux,
                format!("/eleves/{}/notes", o.eleve_ginny),
                false,
            ),
            (
                Acteur::ParentSansDrapeaux,
                format!("/eleves/{}/moyennes?periode_id={p}", o.eleve_ginny),
                false,
            ),
            // Narcissa a le droit aux notes de Draco.
            (
                Acteur::ParentAutreFoyer,
                format!("/eleves/{}/notes", o.eleve_autre),
                true,
            ),
            // Lily a tous les droits sur Harry.
            (Acteur::Parent, format!("/eleves/{}/notes", o.eleve), true),
            (
                Acteur::Parent,
                format!("/famille/enfants/{}/resume", o.eleve),
                true,
            ),
            // Harry lit ses propres notes.
            (Acteur::Eleve, format!("/eleves/{}/notes", o.eleve), true),
        ];
        for (a, chemin, permis) in cellules {
            let web = b.lire_web(a, &chemin).await;
            let appli = b.lire_appli(a, &chemin).await;
            if web.statut != appli.statut {
                ecarts.push(format!(
                    "{a:?} GET {chemin} : web {} / appli {}",
                    web.statut, appli.statut
                ));
            }
            if permis != appli.statut.is_success() {
                let lien = if permis {
                    "le permet"
                } else {
                    "ne le permet pas"
                };
                ecarts.push(format!(
                    "{a:?} GET {chemin} : {} alors que le lien {lien}",
                    appli.statut
                ));
            }
            if !permis && !(appli.est_refus() && !code(&appli).is_empty()) {
                ecarts.push(format!(
                    "{a:?} GET {chemin} : refus sans enveloppe codée : {}",
                    appli.texte
                ));
            }
        }
        assert!(
            ecarts.is_empty(),
            "droits du lien :\n  {}",
            ecarts.join("\n  ")
        );
    });
}

// ───────────────────────── Écritures refusées ─────────────────────────

struct Ecriture {
    route: &'static str,
    acteur: Acteur,
    methode: &'static str,
    chemin: String,
    corps: Option<Value>,
    /// Même geste sur un identifiant inexistant : réponse identique exigée (objet d'un autre établissement).
    inexistant: Option<String>,
    pourquoi: &'static str,
}

/// État des objets visés par les écritures, relu par qui a le droit de le lire (direction de A, de B ; professeur).
async fn etat_des_cibles(m: &Monde) -> BTreeMap<String, Value> {
    let (o, ob) = (&m.a.objets, &m.b.objets);
    let (dir_a, dir_b) = (m.jeton(Acteur::Direction), jeton_b(m, Acteur::Direction));
    let (prof_a, prof_b) = (m.jeton(Acteur::Prof), jeton_b(m, Acteur::Prof));
    let mut sortie = BTreeMap::new();
    for (nom, jeton, chemin) in [
        ("appel A", prof_a, format!("/appels/{}", o.appel.unwrap())),
        ("appel B", prof_b, format!("/appels/{}", ob.appel.unwrap())),
        (
            "absences de Harry",
            dir_a,
            format!("/eleves/{}/absences", o.eleve),
        ),
        (
            "absences de Draco",
            dir_a,
            format!("/eleves/{}/absences", o.eleve_autre),
        ),
        (
            "absences de l'élève de B",
            dir_b,
            format!("/eleves/{}/absences", ob.eleve),
        ),
        (
            "fil de Harry",
            dir_a,
            format!("/messages/fils/{}", o.fil.unwrap()),
        ),
        (
            "fil de Draco",
            dir_a,
            format!("/messages/fils/{}", o.fil_autre.unwrap()),
        ),
        (
            "fil de B",
            dir_b,
            format!("/messages/fils/{}", ob.fil.unwrap()),
        ),
    ] {
        let r = m.appel("GET", &chemin, Some(jeton), None).await;
        assert!(r.est_succes(), "{nom} illisible : {} {}", r.statut, r.texte);
        sortie.insert(nom.to_string(), sans_trace(&r.json));
    }
    sortie
}

#[test]
fn ecritures_refusees_a_l_identique_sans_trace_ni_oracle() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let b = Banc::ouvrir(m).await;
        let o = &m.a.objets;
        let ob = &m.b.objets;
        let x = Uuid::new_v4();
        let (appel, appel_b) = (o.appel.unwrap(), ob.appel.unwrap());
        let (abs, abs_autre, abs_b) = (o.absence.unwrap(), o.absence_autre.unwrap(), ob.absence.unwrap());
        let (fil, fil_b) = (o.fil.unwrap(), ob.fil.unwrap());
        let mut cas: Vec<Ecriture> = Vec::new();
        let mut ajouter = |route, acteur, methode, chemin: String, corps: Option<Value>, inexistant, pourquoi| {
            cas.push(Ecriture { route, acteur, methode, chemin, corps, inexistant, pourquoi })
        };

        // Appel : créneau et jour de l'appel ouvert par le professeur du labo.
        let lu = m.appel("GET", &format!("/appels/{appel}"), Some(m.jeton(Acteur::Prof)), None).await;
        let (creneau, jour) = (lu.json["creneau_id"].clone(), lu.json["jour"].clone());
        assert!(!creneau.is_null(), "créneau de l'appel du labo : {}", lu.texte);
        let ligne = json!({"eleve_id": o.eleve, "etat": "absent"});
        for a in Acteur::FAMILLE {
            let ouvrir = json!({"creneau_id": creneau, "jour": jour});
            ajouter("POST /appels", a, "POST", "/appels".into(), Some(ouvrir), None, "une famille n'ouvre pas d'appel");
            ajouter("POST /appels/{id}/lignes", a, "POST", format!("/appels/{appel}/lignes"), Some(ligne.clone()), None, "une famille ne marque pas un élève");
            ajouter("POST /appels/{id}/clore", a, "POST", format!("/appels/{appel}/clore"), None, None, "une famille ne clôt pas un appel");
        }
        let ligne_b = json!({"eleve_id": ob.eleve, "etat": "absent"});
        for a in Acteur::TOUS {
            ajouter("POST /appels/{id}/lignes", a, "POST", format!("/appels/{appel_b}/lignes"), Some(ligne_b.clone()), Some(format!("/appels/{x}/lignes")), "appel d'un autre établissement");
            ajouter("POST /appels/{id}/clore", a, "POST", format!("/appels/{appel_b}/clore"), None, Some(format!("/appels/{x}/clore")), "appel d'un autre établissement");
        }

        // Justification. Constat PM-01 : dans le même établissement, 403 pour l'objet d'une autre famille, 404 pour un
        // inexistant ; la comparaison à l'inexistant n'est exigée que pour l'autre établissement.
        let motif = json!({"motif_justif": "Rendez-vous médical"});
        for a in [Acteur::ParentAutreFoyer, Acteur::ParentSansDrapeaux] {
            ajouter("POST /absences/{id}/justifier", a, "POST", format!("/absences/{abs}/justifier"), Some(motif.clone()), None, "absence de l'enfant d'une autre famille");
        }
        ajouter("POST /absences/{id}/justifier", Acteur::ParentAutreFoyer, "POST", format!("/absences/{abs_autre}/justifier"), Some(motif.clone()), None, "parent sans droit de justifier (Narcissa sur Draco)");
        ajouter("POST /absences/{id}/justifier", Acteur::Eleve, "POST", format!("/absences/{abs_autre}/justifier"), Some(motif.clone()), None, "élève sur l'absence d'un camarade");
        for a in Acteur::TOUS {
            ajouter("POST /absences/{id}/justifier", a, "POST", format!("/absences/{abs_b}/justifier"), Some(motif.clone()), Some(format!("/absences/{x}/justifier")), "absence d'un autre établissement");
        }

        // Réponse à un fil dont on n'est pas (PM-01 dans l'établissement).
        let reponse = json!({"corps": "Intrusion"});
        for a in [Acteur::ParentAutreFoyer, Acteur::ParentSansDrapeaux] {
            ajouter("POST /messages/fils/{id}/repondre", a, "POST", format!("/messages/fils/{fil}/repondre"), Some(reponse.clone()), None, "fil d'une autre famille");
        }
        for a in Acteur::TOUS {
            ajouter("POST /messages/fils/{id}/repondre", a, "POST", format!("/messages/fils/{fil_b}/repondre"), Some(reponse.clone()), Some(format!("/messages/fils/{x}/repondre")), "fil d'un autre établissement");
        }

        // Session et inscription push d'autrui : une session web de Lily ouverte pour l'occasion (la session du harnais
        // sert aux autres tests du même processus), une inscription push de Lily.
        let tierce = m
            .appel(
                "POST",
                "/auth/login",
                None,
                Some(&json!({"email": m.a.emails[&Acteur::Parent], "password": m.a.mot_de_passe, "etablissement_id": o.etab})),
            )
            .await;
        let jeton_tiers = tierce.json["token"].as_str().expect("session tierce de Lily").to_string();
        let sessions = m.appel("GET", "/auth/sessions", Some(&jeton_tiers), None).await;
        let session_lily = sessions
            .json
            .as_array()
            .and_then(|a| a.iter().find(|s| s["courante"] == true))
            .and_then(|s| s["id"].as_str())
            .expect("session tierce de Lily")
            .to_string();
        let push = m
            .requete(
                "POST",
                "/devices/push",
                Some(b.appli(Acteur::Parent)),
                Some(&json!({"platform": "android", "token": format!("jeton-banc-{}", Uuid::new_v4())})),
                &[("x-scolia-appli", APPLI.into()), ("idempotency-key", Uuid::new_v4().to_string())],
            )
            .await;
        assert!(push.est_succes(), "inscription push de Lily : {}", push.texte);
        let push_lily = push.json["id"].as_str().expect("id push").to_string();
        for a in Acteur::TOUS.into_iter().filter(|a| *a != Acteur::Parent) {
            ajouter("DELETE /auth/sessions/{id}", a, "DELETE", format!("/auth/sessions/{session_lily}"), None, Some(format!("/auth/sessions/{x}")), "session d'une autre personne");
            ajouter("DELETE /devices/push/{id}", a, "DELETE", format!("/devices/push/{push_lily}"), None, Some(format!("/devices/push/{x}")), "inscription push d'une autre personne");
        }

        let avant = etat_des_cibles(m).await;
        let mut ecarts = Vec::new();
        let mut par_route: BTreeMap<&str, usize> = BTreeMap::new();
        for e in &cas {
            *par_route.entry(e.route).or_default() += 1;
            let cell = format!("{:?} {} {} ({})", e.acteur, e.methode, e.chemin, e.pourquoi);
            let web = b.ecrire_web(e.acteur, e.methode, &e.chemin, e.corps.as_ref()).await;
            let appli = b.ecrire_appli(e.acteur, e.methode, &e.chemin, e.corps.as_ref()).await;
            if !web.est_refus() || !appli.est_refus() {
                ecarts.push(format!("{cell} : web {} / appli {} au lieu d'un refus", web.statut, appli.statut));
                continue;
            }
            if web.statut != appli.statut {
                ecarts.push(format!("{cell} : web {} / appli {}", web.statut, appli.statut));
            }
            // Un refus d'authentification ne prouverait pas le cloisonnement.
            if !matches!(code(&appli).as_str(), "droit.refuse" | "objet.introuvable") {
                ecarts.push(format!("{cell} : refus « {} » au lieu de droit.refuse ou objet.introuvable : {}", code(&appli), appli.texte));
            }
            for f in b.fuites_b(&e.chemin, &appli.texte) {
                ecarts.push(format!("{cell} : fuite, {f}"));
            }
            if let Some(ix) = &e.inexistant {
                let r = b.ecrire_appli(e.acteur, e.methode, ix, e.corps.as_ref()).await;
                if (r.statut, code(&r)) != (appli.statut, code(&appli)) {
                    ecarts.push(format!(
                        "{cell} : se distingue d'un inexistant ({} « {} » / {} « {} »)",
                        appli.statut,
                        code(&appli),
                        r.statut,
                        code(&r)
                    ));
                }
            }
        }
        // Chaque route a ses cellules : familles, autre établissement, ou les deux.
        for (route, min) in [
            ("POST /appels", 4),
            ("POST /appels/{id}/lignes", 16),
            ("POST /appels/{id}/clore", 16),
            ("POST /absences/{id}/justifier", 16),
            ("POST /messages/fils/{id}/repondre", 14),
            ("DELETE /auth/sessions/{id}", 11),
            ("DELETE /devices/push/{id}", 11),
        ] {
            let n = par_route.get(route).copied().unwrap_or(0);
            if n < min {
                ecarts.push(format!("{route} : {n} cellules de refus au lieu de {min} au moins"));
            }
        }
        assert_eq!(par_route.len(), ECRITURES.len(), "routes d'écriture sans cellule : {par_route:?}");

        // Aucune trace : appels, absences et fils de A et de B relus identiques ; session de Lily vivante.
        let apres = etat_des_cibles(m).await;
        for (nom, v) in &avant {
            if apres.get(nom) != Some(v) {
                ecarts.push(format!("{nom} a changé après des écritures refusées"));
            }
        }
        let moi = m.appel("GET", "/auth/me", Some(&jeton_tiers), None).await;
        if moi.statut != StatusCode::OK {
            ecarts.push(format!("session de Lily révoquée par un tiers : {}", moi.statut));
        }

        // Témoins : le même geste, par qui en a le droit, n'est pas refusé. Chemins et corps étaient donc valides ;
        // un refus plus haut venait des droits, pas d'une route ou d'un corps faux.
        let prof = Acteur::Prof;
        let temoins: [(&str, Acteur, &str, String, Option<Value>); 6] = [
            ("POST /appels/{id}/lignes", prof, "POST", format!("/appels/{appel}/lignes"), Some(ligne.clone())),
            ("POST /appels/{id}/clore", prof, "POST", format!("/appels/{appel}/clore"), None),
            ("POST /absences/{id}/justifier", Acteur::Parent, "POST", format!("/absences/{abs}/justifier"), Some(motif.clone())),
            ("POST /messages/fils/{id}/repondre", Acteur::Parent, "POST", format!("/messages/fils/{fil}/repondre"), Some(json!({"corps": "Merci"}))),
            ("DELETE /devices/push/{id}", Acteur::Parent, "DELETE", format!("/devices/push/{push_lily}"), None),
            ("DELETE /auth/sessions/{id}", Acteur::Parent, "DELETE", format!("/auth/sessions/{session_lily}"), None),
        ];
        for (route, a, methode, chemin, corps) in temoins {
            // La révocation témoin passe par la session de l'appli de Lily et vise sa session tierce.
            let r = b.ecrire_appli(a, methode, &chemin, corps.as_ref()).await;
            if !r.est_succes() {
                ecarts.push(format!("témoin {route} ({a:?}) : {} {}", r.statut, r.texte));
            }
        }
        // `POST /appels` : le créneau a déjà son appel ; le professeur ne doit pas recevoir un refus de droits.
        let r = b
            .ecrire_appli(prof, "POST", "/appels", Some(&json!({"creneau_id": creneau, "jour": jour})))
            .await;
        if r.est_refus() {
            ecarts.push(format!("témoin POST /appels (professeur) : {} {}", r.statut, r.texte));
        }
        let mort = m.appel("GET", "/auth/me", Some(&jeton_tiers), None).await;
        if mort.statut != StatusCode::UNAUTHORIZED {
            ecarts.push(format!("témoin : la session révoquée répond encore {}", mort.statut));
        }
        assert!(ecarts.is_empty(), "écritures ({} cellules) :\n  {}", cas.len(), ecarts.join("\n  "));
    });
}
