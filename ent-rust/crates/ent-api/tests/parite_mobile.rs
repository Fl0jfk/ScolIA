//! Socle 11, complément du palier MB2 : **banc de parité web / appli** sur la matrice complète de
//! `docs/socles/11_APPLICATIONS_MOBILES.md` §7.2, contre le routeur réel et deux établissements de labo (harnais de la
//! matrice de séparation).
//!
//! Ce que le banc MB2 (`ent-api/src/tests_contrat_mobile.rs`) prouvait déjà : fixtures dorées, liste blanche, parité de
//! 9 personas sur une **même** session (seul l'en-tête `X-ScolIA-Appli` changeait). Ce banc ajoute :
//!
//! 1. **Deux sessions réelles par personne** : la session web du harnais (connexion sans `client`, sans en-tête) et une
//!    session ouverte par l'appli (`"client":"mobile"`, `X-ScolIA-Appli`). Chaque lecture est jouée avec les deux.
//! 2. **Douze personas** (les 12 acteurs du harnais : direction, administratif, comptable, CPE, professeur,
//!    infirmière, psychologue, administrateur, parent avec tous les droits, parent de l'autre foyer, parent sans droit
//!    aux notes ni à la vie scolaire, élève).
//! 3. **Quatre cibles** pour chaque route à identifiant : l'élève (ou l'objet) lié, un non lié, un objet de l'autre
//!    établissement, un identifiant inexistant. L'objet de l'autre établissement doit recevoir **exactement** la réponse
//!    de l'inexistant (statut et code) : aucune information sur son existence (oracle d'existence).
//! 4. **Refus d'écriture** des routes de la liste blanche (appel, justification, réponse à un fil, révocation de
//!    session, désinscription push) par qui n'en a pas le droit : même statut au web et dans l'appli, enveloppe codée,
//!    indistinct de l'inexistant, aucune trace laissée.
//! 5. **Droits du lien** : parent sans droit aux notes (Molly, sur Ginny), parent sans droit de justifier (Narcissa, sur
//!    Draco), élève sur un camarade.
//! 6. **Complétude** : chaque route de `contrat/mobile/routes.txt` est jouée ici, ou exclue avec sa raison ; une route
//!    ajoutée à la liste blanche sans ligne dans ce banc le rend rouge (§7.2, point 7).
//! 7. **Oracle de fuite** sur chaque réponse : aucun identifiant de l'autre établissement.
//!
//! Hors de portée (lots suivants, documenté dans `docs/socles/11_MB2_PARITE_COMPLETE.md`) : manifeste `/mobile/contexte`
//! (MB5), baux hors ligne et opérations différées (MB7), session mobile signée par la clé d'appareil (MB4 : tant
//! qu'elle n'existe pas, une connexion « mobile » par mot de passe reçoit une session web, MB0 / MEN-11), remplaçant en
//! mission et élève majeur (absents du labo).
//!
//! Lancement : `ENT_ENV=lab ENT_DATABASE_URL=postgres://…/ent_ci_test cargo test -p ent-api --features labo --test
//! parite_mobile -- --test-threads 1`. Sans base : SKIP.

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
    /// Même statut et même nombre d'éléments : la liste des sessions marque la session courante, qui diffère.
    Statut,
}

/// Cibles d'une lecture à identifiant.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Visee {
    Liee,
    NonLiee,
    AutreEtab,
    Inexistant,
}

const VISEES: [Visee; 4] = [
    Visee::Liee,
    Visee::NonLiee,
    Visee::AutreEtab,
    Visee::Inexistant,
];

/// Une lecture : gabarit de `routes.txt` et chemin joué. Marqueurs : `{eleve}`, `{fil}`, `{appel}`, `{cours}`,
/// `{classe}`, `{jour}`, `{periode}`, `{eva_periode}`.
struct Lecture {
    route: &'static str,
    chemin: &'static str,
    comparer: Comparer,
}

const LECTURES: &[Lecture] = &[
    Lecture {
        route: "GET /auth/me",
        chemin: "/auth/me",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /roles/mes-droits",
        chemin: "/roles/mes-droits",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /auth/sessions",
        chemin: "/auth/sessions",
        comparer: Comparer::Statut,
    },
    Lecture {
        route: "GET /edt/mes-creneaux",
        chemin: "/edt/mes-creneaux?jour={jour}",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /eleves/{id}/edt",
        chemin: "/eleves/{eleve}/edt?jour={jour}",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /appels/{id}",
        chemin: "/appels/{appel}",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /famille/enfants",
        chemin: "/famille/enfants",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /famille/enfants/{id}/resume",
        chemin: "/famille/enfants/{eleve}/resume",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /vs/absences",
        chemin: "/vs/absences?jour={jour}",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /note-periodes",
        chemin: "/note-periodes",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /eleves/{id}/notes",
        chemin: "/eleves/{eleve}/notes",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /eleves/{id}/moyennes",
        chemin: "/eleves/{eleve}/moyennes?periode_id={periode}",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /devoirs",
        chemin: "/devoirs?periode_id={periode}&classe={classe}",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /eleves/{id}/devoirs",
        chemin: "/eleves/{eleve}/devoirs?periode_id={periode}",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /classes/{classe}/effectif",
        chemin: "/classes/{classe}/effectif?jour={jour}",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /cahier-texte",
        chemin: "/cahier-texte?classe={classe}",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /cahier-texte",
        chemin: "/cahier-texte?eleve_id={eleve}",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /messages/fils",
        chemin: "/messages/fils",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /messages/fils/{id}",
        chemin: "/messages/fils/{fil}",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /annonces",
        chemin: "/annonces",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /eleves/{id}/annonces",
        chemin: "/eleves/{eleve}/annonces",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /notifications",
        chemin: "/notifications",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /eva/enseignements",
        chemin: "/eva/enseignements",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /eva/cours/{id}/periodes",
        chemin: "/eva/cours/{cours}/periodes",
        comparer: Comparer::Corps,
    },
    Lecture {
        route: "GET /eva/grille",
        chemin: "/eva/grille?cours_id={cours}&periode_id={eva_periode}",
        comparer: Comparer::Corps,
    },
];

/// Écritures dont le banc éprouve les refus (jamais un succès : aucune n'est jouée par qui en a le droit).
const ECRITURES: &[&str] = &[
    "POST /appels",
    "POST /appels/{id}/lignes",
    "POST /appels/{id}/clore",
    "POST /absences/{id}/justifier",
    "POST /messages/fils/{id}/repondre",
    "DELETE /auth/sessions/{id}",
    "DELETE /devices/push/{id}",
];

/// Lectures par élève qu'un membre d'une famille ne doit jamais obtenir sur un élève qui n'est pas le sien.
const PAR_ELEVE: &[&str] = &[
    "GET /eleves/{id}/edt",
    "GET /famille/enfants/{id}/resume",
    "GET /eleves/{id}/notes",
    "GET /eleves/{id}/moyennes",
    "GET /eleves/{id}/devoirs",
    "GET /eleves/{id}/annonces",
];

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

/// §7.2 point 7 : aucune route de la liste blanche sans ligne dans ce banc, aucune ligne du banc hors de la liste.
/// Ne demande pas de base.
#[test]
fn chaque_route_de_la_liste_blanche_est_jouee_ou_exclue() {
    let liste: BTreeSet<String> = routes_txt().into_iter().collect();
    let mut couvertes: BTreeMap<String, &str> = BTreeMap::new();
    for l in LECTURES {
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
    for r in PAR_ELEVE {
        assert!(
            liste.contains(*r),
            "{r} : cellule de cloisonnement sur une route hors de la liste blanche"
        );
    }
}

// ───────────────────────── Sessions de l'appli ─────────────────────────

struct Banc<'a> {
    m: &'a Monde,
    /// Session ouverte par l'appli, par acteur de l'établissement A.
    appli: BTreeMap<Acteur, String>,
    jour: String,
    periode: String,
    eva_periode: Option<String>,
    classe_liee: String,
    classe_autre: String,
}

impl<'a> Banc<'a> {
    async fn ouvrir(m: &'a Monde) -> Banc<'a> {
        let mut appli = BTreeMap::new();
        for a in Acteur::TOUS {
            let r = m
                .requete(
                    "POST",
                    "/auth/login",
                    None,
                    Some(&json!({
                        "email": m.a.emails[&a], "password": m.a.mot_de_passe,
                        "etablissement_id": m.a.objets.etab, "client": "mobile"
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
            appli.insert(a, jeton);
        }
        let jour = domain_eleve::horloge::jour_paris().to_string();
        let prof = m.jeton(Acteur::Prof);
        let periodes = m.appel("GET", "/note-periodes", Some(prof), None).await;
        let periode = periodes.json[0]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("périodes de notes : {}", periodes.texte))
            .to_string();
        let eva_periode = match m.a.objets.cours {
            Some(c) => {
                let r = m
                    .appel("GET", &format!("/eva/cours/{c}/periodes"), Some(prof), None)
                    .await;
                premier_id(&r.json)
            }
            None => None,
        };
        // Classes du semis du labo (`ent_db::seed`, ELEVES_SEED) : Harry en 4B, Draco en 5A.
        let classe_liee = "4B".to_string();
        let classe_autre = "5A".to_string();
        Banc {
            m,
            appli,
            jour,
            periode,
            eva_periode,
            classe_liee,
            classe_autre,
        }
    }

    fn web(&self, a: Acteur) -> &str {
        self.m.jeton(a)
    }

    fn appli(&self, a: Acteur) -> &str {
        &self.appli[&a]
    }

    /// Objets visés par `(acteur, visée)` ; `None` pour un marqueur sans objet (cellule ignorée).
    fn chemin(&self, gabarit: &str, a: Acteur, v: Visee) -> Option<String> {
        let o = &self.m.a.objets;
        let b = &self.m.b.objets;
        let inexistant = Uuid::new_v4();
        let famille = a.est_famille();
        // L'élève lié à l'acteur et un élève qui ne l'est pas (même établissement).
        let (liee, non_liee) = match a {
            Acteur::ParentAutreFoyer => (o.eleve_autre, o.eleve),
            Acteur::ParentSansDrapeaux => (o.eleve_ginny, o.eleve),
            _ => (o.eleve, o.eleve_autre),
        };
        let eleve = match v {
            Visee::Liee => liee,
            Visee::NonLiee => non_liee,
            Visee::AutreEtab => b.eleve,
            Visee::Inexistant => inexistant,
        };
        let fil = match v {
            Visee::Liee if a == Acteur::ParentAutreFoyer => o.fil_autre,
            Visee::Liee => o.fil,
            Visee::NonLiee if a == Acteur::ParentAutreFoyer => o.fil,
            Visee::NonLiee => o.fil_autre,
            Visee::AutreEtab => b.fil,
            Visee::Inexistant => Some(inexistant),
        };
        let appel = match v {
            Visee::Liee | Visee::NonLiee => o.appel,
            Visee::AutreEtab => b.appel,
            Visee::Inexistant => Some(inexistant),
        };
        let cours = match v {
            Visee::Liee | Visee::NonLiee => o.cours,
            Visee::AutreEtab => b.cours,
            Visee::Inexistant => Some(inexistant),
        };
        let classe = match (v, famille) {
            (Visee::NonLiee, _) => self.classe_autre.clone(),
            _ => self.classe_liee.clone(),
        };
        let mut s = gabarit.to_string();
        for (cle, val) in [
            ("{eleve}", Some(eleve.to_string())),
            ("{fil}", fil.map(|u| u.to_string())),
            ("{appel}", appel.map(|u| u.to_string())),
            ("{cours}", cours.map(|u| u.to_string())),
            ("{classe}", Some(classe)),
            ("{jour}", Some(self.jour.clone())),
            ("{periode}", Some(self.periode.clone())),
            ("{eva_periode}", self.eva_periode.clone()),
        ] {
            if s.contains(cle) {
                s = s.replace(cle, &val?);
            }
        }
        Some(s)
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

/// Identifiants de B présents dans la réponse, hors ceux que la requête a elle-même fournis.
fn fuites_b(m: &Monde, chemin: &str, texte: &str) -> Vec<String> {
    fuites(m, texte, &[])
        .into_iter()
        .filter(|f| {
            f.strip_prefix("identifiant de B : ")
                .is_none_or(|id| !chemin.contains(id))
        })
        .collect()
}

/// Signature d'une réponse pour l'oracle d'existence : statut et code (en succès, le corps).
fn signature(r: &Rep) -> String {
    if r.statut.is_success() {
        format!("{} {}", r.statut.as_u16(), sans_trace(&r.json))
    } else {
        format!("{} {}", r.statut.as_u16(), code(r))
    }
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
fn lectures_meme_reponse_au_web_et_dans_l_appli_pour_douze_personas_et_quatre_cibles() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let b = Banc::ouvrir(m).await;
        let mut ecarts = Vec::new();
        let mut statuts: BTreeMap<u16, usize> = BTreeMap::new();
        let mut cellules = 0usize;
        for a in Acteur::TOUS {
            for l in LECTURES {
                let a_identifiant = ["{eleve}", "{fil}", "{appel}", "{cours}"].iter().any(|k| l.chemin.contains(k));
                let visees: &[Visee] = if a_identifiant { &VISEES } else { &[Visee::Liee] };
                // Oracle d'existence : l'objet de l'autre établissement répond comme un identifiant inexistant.
                let mut sig_autre: Option<(String, String)> = None;
                let mut sig_inexistant: Option<(String, String)> = None;
                for v in visees {
                    let Some(chemin) = b.chemin(l.chemin, a, *v) else { continue };
                    cellules += 1;
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
                            Comparer::Statut => {
                                web.json.as_array().map(Vec::len) == appli.json.as_array().map(Vec::len)
                            }
                        };
                        if !egal {
                            ecarts.push(format!("{cell} : corps différent entre web et appli"));
                        }
                    } else if code(&appli).is_empty() {
                        ecarts.push(format!("{cell} : erreur sans enveloppe codée dans l'appli : {}", appli.texte));
                    }
                    for f in fuites_b(m, &chemin, &web.texte).into_iter().chain(fuites_b(m, &chemin, &appli.texte)) {
                        ecarts.push(format!("{cell} : fuite, {f}"));
                    }
                    // Cloisonnement des familles : jamais l'élève d'un autre.
                    if a.est_famille() && *v == Visee::NonLiee && PAR_ELEVE.contains(&l.route) && !appli.est_refus() {
                        ecarts.push(format!("{cell} : {} au lieu d'un refus (élève d'une autre famille)", appli.statut));
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
                if let (Some(x), Some(y)) = (&sig_autre, &sig_inexistant) {
                    if x != y {
                        ecarts.push(format!(
                            "{a:?} GET {} : l'objet de l'autre établissement se distingue d'un inexistant ({x:?} / {y:?})",
                            l.route
                        ));
                    }
                }
            }
        }
        assert!(ecarts.is_empty(), "parité ou cloisonnement rompus ({} écarts) :\n  {}", ecarts.len(), ecarts.join("\n  "));
        // Le banc doit voir des succès et des refus : sinon il ne prouve rien.
        let succes = statuts.get(&200).copied().unwrap_or(0);
        let refus: usize = statuts.iter().filter(|(s, _)| (400..500).contains(*s)).map(|(_, n)| n).sum();
        assert!(cellules > 600, "trop peu de cellules : {cellules}");
        assert!(succes > 150 && refus > 150, "matrice déséquilibrée : {statuts:?}");
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
                ecarts.push(format!(
                    "{a:?} GET {chemin} : {} alors que le lien {}",
                    appli.statut,
                    if permis {
                        "le permet"
                    } else {
                        "ne le permet pas"
                    }
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
    /// Chemin du même geste sur un identifiant inexistant : la réponse doit être la même (oracle d'existence), quand
    /// la cible est un objet que l'acteur ne doit pas pouvoir distinguer.
    inexistant: Option<String>,
    pourquoi: &'static str,
}

#[test]
fn ecritures_refusees_a_l_identique_sans_trace_ni_oracle() {
    let m = monde_ou_skip!();
    rt().block_on(async {
        let b = Banc::ouvrir(m).await;
        let o = &m.a.objets;
        let ob = &m.b.objets;
        let x = Uuid::new_v4();
        let mut cas: Vec<Ecriture> = Vec::new();

        // Appel : créneau de l'appel ouvert par le professeur du labo.
        let appel = o.appel.expect("appel ouvert par le harnais");
        let lu = m.appel("GET", &format!("/appels/{appel}"), Some(m.jeton(Acteur::Prof)), None).await;
        assert!(lu.est_succes(), "appel du labo : {}", lu.texte);
        let lignes_avant = sans_trace(&lu.json);
        let creneau = lu.json["creneau_id"].clone();
        let jour = lu.json["jour"].clone();
        let ligne = json!({"eleve_id": o.eleve, "etat": "absent"});
        for a in Acteur::FAMILLE {
            cas.push(Ecriture {
                route: "POST /appels",
                acteur: a,
                methode: "POST",
                chemin: "/appels".into(),
                corps: Some(json!({"creneau_id": creneau, "jour": jour})),
                inexistant: None,
                pourquoi: "une famille n'ouvre pas d'appel",
            });
            cas.push(Ecriture {
                route: "POST /appels/{id}/lignes",
                acteur: a,
                methode: "POST",
                chemin: format!("/appels/{appel}/lignes"),
                corps: Some(ligne.clone()),
                inexistant: None,
                pourquoi: "une famille ne marque pas un élève",
            });
            cas.push(Ecriture {
                route: "POST /appels/{id}/clore",
                acteur: a,
                methode: "POST",
                chemin: format!("/appels/{appel}/clore"),
                corps: None,
                inexistant: None,
                pourquoi: "une famille ne clôt pas un appel",
            });
        }
        // Tous les acteurs de A, sur l'appel de B : refus indistinct d'un appel inexistant.
        if let Some(appel_b) = ob.appel {
            for a in Acteur::TOUS {
                cas.push(Ecriture {
                    route: "POST /appels/{id}/lignes",
                    acteur: a,
                    methode: "POST",
                    chemin: format!("/appels/{appel_b}/lignes"),
                    corps: Some(json!({"eleve_id": ob.eleve, "etat": "absent"})),
                    inexistant: Some(format!("/appels/{x}/lignes")),
                    pourquoi: "appel d'un autre établissement",
                });
                cas.push(Ecriture {
                    route: "POST /appels/{id}/clore",
                    acteur: a,
                    methode: "POST",
                    chemin: format!("/appels/{appel_b}/clore"),
                    corps: None,
                    inexistant: Some(format!("/appels/{x}/clore")),
                    pourquoi: "appel d'un autre établissement",
                });
            }
        }

        // Justification : sans droit sur l'élève, ou sans droit de justifier.
        let motif = json!({"motif_justif": "Rendez-vous médical"});
        if let Some(abs) = o.absence {
            for a in [Acteur::ParentAutreFoyer, Acteur::ParentSansDrapeaux] {
                cas.push(Ecriture {
                    route: "POST /absences/{id}/justifier",
                    acteur: a,
                    methode: "POST",
                    chemin: format!("/absences/{abs}/justifier"),
                    corps: Some(motif.clone()),
                    // Constat PM-01 : 403 dans le même établissement, 404 pour un inexistant.
                    inexistant: None,
                    pourquoi: "absence de l'enfant d'une autre famille",
                });
            }
        }
        if let Some(abs) = o.absence_autre {
            cas.push(Ecriture {
                route: "POST /absences/{id}/justifier",
                acteur: Acteur::ParentAutreFoyer,
                methode: "POST",
                chemin: format!("/absences/{abs}/justifier"),
                corps: Some(motif.clone()),
                inexistant: None,
                pourquoi: "parent sans droit de justifier (Narcissa sur Draco)",
            });
            cas.push(Ecriture {
                route: "POST /absences/{id}/justifier",
                acteur: Acteur::Eleve,
                methode: "POST",
                chemin: format!("/absences/{abs}/justifier"),
                corps: Some(motif.clone()),
                // Constat PM-01.
                inexistant: None,
                pourquoi: "élève sur l'absence d'un camarade",
            });
        }
        if let Some(abs_b) = ob.absence {
            for a in Acteur::TOUS {
                cas.push(Ecriture {
                    route: "POST /absences/{id}/justifier",
                    acteur: a,
                    methode: "POST",
                    chemin: format!("/absences/{abs_b}/justifier"),
                    corps: Some(motif.clone()),
                    inexistant: Some(format!("/absences/{x}/justifier")),
                    pourquoi: "absence d'un autre établissement",
                });
            }
        }

        // Réponse à un fil dont on n'est pas.
        let reponse = json!({"corps": "Intrusion"});
        if let Some(fil) = o.fil {
            for a in [Acteur::ParentAutreFoyer, Acteur::ParentSansDrapeaux] {
                cas.push(Ecriture {
                    route: "POST /messages/fils/{id}/repondre",
                    acteur: a,
                    methode: "POST",
                    chemin: format!("/messages/fils/{fil}/repondre"),
                    corps: Some(reponse.clone()),
                    // Constat PM-01 (`11_MB2_PARITE_COMPLETE.md`) : dans le même établissement, le fil d'une autre
                    // famille répond 403 et un fil inexistant 404. Identifiants non devinables ; l'autre
                    // établissement, lui, est indistinct (cellules plus bas).
                    inexistant: None,
                    pourquoi: "fil d'une autre famille",
                });
            }
        }
        if let Some(fil_b) = ob.fil {
            for a in Acteur::TOUS {
                cas.push(Ecriture {
                    route: "POST /messages/fils/{id}/repondre",
                    acteur: a,
                    methode: "POST",
                    chemin: format!("/messages/fils/{fil_b}/repondre"),
                    corps: Some(reponse.clone()),
                    inexistant: Some(format!("/messages/fils/{x}/repondre")),
                    pourquoi: "fil d'un autre établissement",
                });
            }
        }

        // Session et inscription push d'autrui : la session web de Lily, une inscription push de Lily.
        let sessions = m.appel("GET", "/auth/sessions", Some(b.web(Acteur::Parent)), None).await;
        let session_lily = sessions
            .json
            .as_array()
            .and_then(|a| a.iter().find(|s| s["courante"] == true))
            .and_then(|s| s["id"].as_str())
            .expect("session courante de Lily")
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
            cas.push(Ecriture {
                route: "DELETE /auth/sessions/{id}",
                acteur: a,
                methode: "DELETE",
                chemin: format!("/auth/sessions/{session_lily}"),
                corps: None,
                inexistant: Some(format!("/auth/sessions/{x}")),
                pourquoi: "session d'une autre personne",
            });
            cas.push(Ecriture {
                route: "DELETE /devices/push/{id}",
                acteur: a,
                methode: "DELETE",
                chemin: format!("/devices/push/{push_lily}"),
                corps: None,
                inexistant: Some(format!("/devices/push/{x}")),
                pourquoi: "inscription push d'une autre personne",
            });
        }

        let mut ecarts = Vec::new();
        let mut routes_vues = BTreeSet::new();
        for e in &cas {
            routes_vues.insert(e.route);
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
            if code(&appli).is_empty() {
                ecarts.push(format!("{cell} : refus sans enveloppe codée : {}", appli.texte));
            }
            for f in fuites_b(m, &e.chemin, &appli.texte) {
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
        for r in ECRITURES {
            if !routes_vues.contains(r) {
                ecarts.push(format!("{r} : aucune cellule de refus jouée (objet manquant dans le labo ?)"));
            }
        }

        // Aucune trace : l'appel, la session et l'inscription de Lily sont intacts.
        let relu = m.appel("GET", &format!("/appels/{appel}"), Some(m.jeton(Acteur::Prof)), None).await;
        if sans_trace(&relu.json) != lignes_avant {
            ecarts.push("l'appel du labo a changé après des écritures refusées".into());
        }
        let moi = m.appel("GET", "/auth/me", Some(b.web(Acteur::Parent)), None).await;
        if moi.statut != StatusCode::OK {
            ecarts.push(format!("session web de Lily révoquée par un tiers : {}", moi.statut));
        }
        let suppr = b.ecrire_appli(Acteur::Parent, "DELETE", &format!("/devices/push/{push_lily}"), None).await;
        if !suppr.est_succes() {
            ecarts.push(format!("inscription push de Lily supprimée par un tiers : {} {}", suppr.statut, suppr.texte));
        }
        assert!(ecarts.is_empty(), "écritures ({} cellules) :\n  {}", cas.len(), ecarts.join("\n  "));
        assert!(cas.len() > 60, "trop peu de cellules d'écriture : {}", cas.len());
    });
}
