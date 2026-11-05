//! Matrice de tests de **séparation** (rôles × ressources × établissements)
//! contre le vrai routeur `ent-api` et une base PostgreSQL locale de test.
//!
//! Lancement (Git Bash) :
//!   cd ent-rust/web && . scripts/env.sh && cd .. &&
//!   CARGO_TARGET_DIR="$SCOLIA_TARGET_ENT" cargo test -p ent-api --test separation -- --test-threads=1
//! Les cas de failles encore ouvertes sont `#[ignore]` (un test par faille) :
//!   … -- --test-threads=1 --ignored     (ils doivent échouer tant que la faille existe)
//!
//! Sans `ENT_DATABASE_URL`, les tests sont ignorés (message « SKIP »).
//! Voir `separation/harness.rs` (démarrage, acteurs, oracle de fuite) et
//! `separation/routes_src.rs` (inventaire des routes lu dans `routes.rs`).

#[path = "separation/harness.rs"]
mod harness;
#[path = "separation/routes_src.rs"]
mod routes_src;

use base64::Engine;
use harness::*;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

use Acteur::*;

static CAS_EXECUTES: AtomicUsize = AtomicUsize::new(0);

// ───────────────────────── Spécification déclarative ─────────────────────────

/// Une route et, pour chaque acteur, ce qui est attendu selon la cible.
/// `{eleve}`, `{foyer}`, `{doc}`, `{doc_sante}`, `{fil}`, `{facture}`,
/// `{absence}`, `{matiere}`, `{appel}`, `{jour}` sont résolus par cible.
/// Un acteur absent des listes est attendu **refusé** (401/403/404).
/// Sur la cible B (jeton A, identifiants B) tout acteur doit être refusé ou
/// obtenir une réponse vide, et jamais un 2xx avec des données de B.
struct Spec {
    nom: &'static str,
    methode: &'static str,
    chemin: &'static str,
    corps: Option<&'static str>,
    /// Autorisés (2xx) sur une ressource liée à l'acteur.
    liee: &'static [Acteur],
    /// Autorisés (2xx) sur une ressource de A non liée à l'acteur.
    non_liee: &'static [Acteur],
    /// 2xx mais réponse vide attendue sur une ressource de A non liée.
    non_liee_vide: &'static [Acteur],
    /// Attendu sur la cible B.
    b: Attendu,
    /// N'exécuter que la cible B (écritures : on ne modifie jamais A).
    seulement_b: bool,
    /// Sous-chaînes interdites dans toute réponse 2xx de ce cas.
    interdits: &'static [&'static str],
    /// Interdits globaux tolérés (faille connue, couverte par un test ignoré).
    tolere: &'static [&'static str],
}

/// Champs qu'aucune réponse ne doit jamais contenir (secrets, clés d'objets).
const INTERDITS_GLOBAUX: &[&str] = &["storage_key", "password_hash", "argon2"];

/// Attendu supplémentaire : tout échec (4xx/5xx) convient (écritures sur B).
fn echec_quelconque(rep: &Rep) -> bool {
    !rep.statut.is_success()
}

fn lire(
    nom: &'static str,
    chemin: &'static str,
    liee: &'static [Acteur],
    non_liee: &'static [Acteur],
) -> Spec {
    Spec {
        nom,
        methode: "GET",
        chemin,
        corps: None,
        liee,
        non_liee,
        non_liee_vide: &[],
        b: Attendu::RefuseOuVide,
        seulement_b: false,
        interdits: &[],
        tolere: &[],
    }
}

fn ecrire_sur_b(
    nom: &'static str,
    methode: &'static str,
    chemin: &'static str,
    corps: Option<&'static str>,
) -> Spec {
    Spec {
        nom,
        methode,
        chemin,
        corps,
        liee: &[],
        non_liee: &[],
        non_liee_vide: &[],
        b: Attendu::EchecQuelconque,
        seulement_b: true,
        interdits: &[],
        tolere: &[],
    }
}

fn attendu(s: &Spec, a: Acteur, c: Cible) -> Attendu {
    match c {
        Cible::B => s.b,
        Cible::Liee => {
            if s.liee.contains(&a) {
                Attendu::Autorise
            } else {
                Attendu::Refuse
            }
        }
        Cible::NonLiee => {
            if s.non_liee.contains(&a) {
                Attendu::Autorise
            } else if s.non_liee_vide.contains(&a) {
                Attendu::ListeVide
            } else {
                Attendu::Refuse
            }
        }
    }
}

fn verifie(att: Attendu, rep: &Rep) -> bool {
    match att {
        Attendu::Autorise => rep.est_succes(),
        Attendu::Refuse => rep.est_refus(),
        Attendu::ListeVide => rep.est_succes() && rep.est_vide(),
        Attendu::RefuseOuVide => rep.est_refus() || (rep.est_succes() && rep.est_vide()),
        Attendu::EchecQuelconque => echec_quelconque(rep),
        Attendu::RefuseOuIndistinct => rep.est_refus() || rep.est_succes(), // contrôle complet dans jouer()
    }
}

fn jour_du_test() -> String {
    chrono::Utc::now().date_naive().to_string()
}

/// Joue les spécifications : tous acteurs × cibles, oracle de fuite sur chaque 2xx.
fn jouer(domaine: &str, specs: &[Spec]) {
    let Some(m) = monde() else { return };
    let mut echecs: Vec<String> = Vec::new();
    let mut n = 0usize;
    rt().block_on(async {
        for s in specs {
            let sans_cible = !s.chemin.contains('{');
            let cibles: &[Cible] = if s.seulement_b {
                &[Cible::B]
            } else if sans_cible {
                &[Cible::Liee]
            } else {
                &[Cible::Liee, Cible::NonLiee, Cible::B]
            };
            for a in Acteur::TOUS {
                for &c in cibles {
                    let o = m.objets_cible(a, c);
                    let jour = jour_du_test();
                    let Some(chemin) = o.resoudre(&s.chemin.replace("{jour}", &jour)) else {
                        continue; // objet absent de cette cible (ex. pas de document pour Draco)
                    };
                    let corps: Option<Value> = s.corps.map(|t| {
                        let Some(t) = o.resoudre(&t.replace("{jour}", &jour)) else {
                            return Value::Null;
                        };
                        serde_json::from_str(&t).expect("corps JSON du cas")
                    });
                    let rep = m.appel(s.methode, &chemin, Some(m.jeton(a)), corps.as_ref()).await;
                    n += 1;
                    let att = attendu(s, a, c);
                    let mut pb = Vec::new();
                    if !verifie(att, &rep) {
                        pb.push(format!("statut {} (attendu {att:?})", rep.statut.as_u16()));
                    }
                    // Texte sans l'identifiant demandé : un écho de l'entrée n'est pas une fuite.
                    let eid = o.eleve.to_string();
                    let mut texte = rep.texte.clone();
                    if att == Attendu::RefuseOuIndistinct && rep.est_succes() {
                        let neuf = Uuid::new_v4().to_string();
                        let ctl = m.appel(s.methode, &chemin.replace(&eid, &neuf), Some(m.jeton(a)), corps.as_ref()).await;
                        if ctl.statut != rep.statut || ctl.texte.replace(&neuf, "") != rep.texte.replace(&eid, "") {
                            pb.push("réponse distincte de celle d'un identifiant inexistant".to_string());
                        }
                        texte = rep.texte.replace(&eid, "");
                    }
                    if rep.est_succes() {
                        let mut interdits: Vec<&str> = INTERDITS_GLOBAUX
                            .iter()
                            .copied()
                            .filter(|i| !s.tolere.contains(i))
                            .collect();
                        interdits.extend_from_slice(s.interdits);
                        for f in fuites(m, &texte, &interdits) {
                            pb.push(format!("FUITE {f}"));
                        }
                    }
                    if !pb.is_empty() {
                        echecs.push(format!(
                            "[{}] {} {} acteur={a:?} cible={c:?} : {}",
                            s.nom,
                            s.methode,
                            chemin.split('?').next().unwrap_or(""),
                            pb.join(" ; ")
                        ));
                    }
                }
            }
        }
    });
    CAS_EXECUTES.fetch_add(n, Ordering::Relaxed);
    eprintln!("[separation] {domaine} : {n} cas, {} écart(s)", echecs.len());
    assert!(
        echecs.is_empty(),
        "{} écart(s) de séparation dans « {domaine} » :\n{}",
        echecs.len(),
        echecs.join("\n")
    );
}

// ───────────────────────── Groupes d'acteurs ─────────────────────────

/// Personnel de gestion pédagogique et vie scolaire.
const GESTION: &[Acteur] = &[Admin, Direction, Administratif, Cpe, Prof];
/// Socle 1 (B1, B2) : hors de ses élèves, un professeur est refusé (notes, vie
/// scolaire, carnet, journal, fils). Les autres personnels gardent l'établissement.
const GESTION_HORS_PROF: &[Acteur] = &[Admin, Direction, Administratif, Cpe];
const STAFF_HORS_PROF: &[Acteur] = &[Admin, Direction, Administratif, Cpe, Compta, Infirmiere, Psy];
const STAFF: &[Acteur] = &Acteur::STAFF;
const TOUS: &[Acteur] = &Acteur::TOUS;
const FAMILLE: &[Acteur] = &Acteur::FAMILLE;
const SANTE_LIEE: &[Acteur] = &[Admin, Direction, Infirmiere, Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux];
const SANTE_NON_LIEE: &[Acteur] = &[Admin, Direction, Infirmiere];
const PSY_LIEE: &[Acteur] = &[Admin, Direction, Psy, Parent, ParentAutreFoyer, ParentSansDrapeaux];
const PSY_NON_LIEE: &[Acteur] = &[Admin, Direction, Psy];
/// Foyers (coordonnées, IBAN) : gestion administrative, comptabilité, CPE.
const FOYERS: &[Acteur] = &[Admin, Direction, Administratif, Compta, Cpe];

// ───────────────────────── Domaines couverts (cloisonnement correct) ─────────────────────────

#[test]
fn domaine_eleve_dossier() {
    jouer(
        "élève / dossier",
        &[
            lire("eleve", "/eleves/{eleve}", TOUS, STAFF),
            lire("dossier", "/eleves/{eleve}/dossier", TOUS, STAFF),
            lire("carnet", "/eleves/{eleve}/carnet",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent, Eleve], GESTION_HORS_PROF),
            Spec { non_liee_vide: FAMILLE, ..lire("incidents", "/eleves/{eleve}/incidents", TOUS, STAFF) },
            Spec { non_liee_vide: FAMILLE, ..lire("sanctions", "/eleves/{eleve}/sanctions", TOUS, STAFF) },
        ],
    );
}

#[test]
fn domaine_notes() {
    jouer(
        "notes",
        &[
            // Socle 1 (B1) : le professeur ne lit que les notes de ses élèves.
            lire("notes", "/eleves/{eleve}/notes",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent, Eleve, ParentAutreFoyer], GESTION_HORS_PROF),
            ecrire_sur_b("note-sur-B", "POST", "/eleves/{eleve}/notes",
                Some(r#"{"matiere_id":"{matiere}","valeur":12,"date_note":"{jour}"}"#)),
        ],
    );
}

#[test]
fn domaine_absences() {
    jouer(
        "absences",
        &[
            // Parent sans drapeau vie scolaire (Narcissa → Draco, Molly → Ginny) : refusé.
            lire("absences", "/eleves/{eleve}/absences",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent, Eleve], GESTION_HORS_PROF),
            ecrire_sur_b("absence-sur-B", "POST", "/eleves/{eleve}/absences",
                Some(r#"{"debut":"{jour}","fin":"{jour}","motif":"separation"}"#)),
            ecrire_sur_b("justifier-B", "POST", "/absences/{absence}/justifier", Some("{}")),
            ecrire_sur_b("accepter-justificatif-B", "POST", "/absences/{absence}/accepter-justificatif", Some("{}")),
        ],
    );
}

#[test]
fn domaine_sante() {
    jouer(
        "santé (fiche, PAI, infirmerie)",
        &[
            lire("fiche-sante", "/eleves/{eleve}/fiche-sante", SANTE_LIEE, SANTE_NON_LIEE),
            lire("pai", "/eleves/{eleve}/pai", SANTE_LIEE, SANTE_NON_LIEE),
            lire("infirmerie", "/infirmerie/passages?eleve_id={eleve}", SANTE_LIEE, SANTE_NON_LIEE),
            ecrire_sur_b("fiche-sante-B", "PUT", "/eleves/{eleve}/fiche-sante", Some(r#"{"allergies":"separation"}"#)),
        ],
    );
}

#[test]
fn domaine_psy() {
    jouer(
        "psy",
        &[
            lire("dossier-psy", "/eleves/{eleve}/dossier-psy", PSY_LIEE, PSY_NON_LIEE),
            lire("entretiens-psy", "/psy/entretiens?eleve_id={eleve}", PSY_LIEE, PSY_NON_LIEE),
        ],
    );
}

#[test]
fn domaine_documents() {
    jouer(
        "documents (par identifiant)",
        &[
            lire("documents-liste", "/eleves/{eleve}/documents", TOUS, STAFF),
            lire("document-fichier", "/documents/{doc}/fichier",
                &[Admin, Direction, Administratif, Parent, Eleve], &[]),
            // Document restreint (verrouillé) : seuls les rôles de gestion documentaire.
            lire("document-verrouille", "/documents/{doc_sante}/fichier",
                &[Admin, Direction, Administratif], &[]),
            ecrire_sur_b("grant-B", "POST", "/documents/{doc}/grants", Some(r#"{"duration_days":3,"note":"separation"}"#)),
            ecrire_sur_b("grant-verrouille-B", "POST", "/documents/{doc_sante}/grants", Some(r#"{"duration_days":3,"note":"separation"}"#)),
            ecrire_sur_b("depot-B", "POST", "/eleves/{eleve}/documents",
                Some(r#"{"titre":"separation","tiroir":"famille","confidentialite":"standard"}"#)),
        ],
    );
}

#[test]
fn domaine_fils_de_messages() {
    jouer(
        "fils de messages",
        &[
            lire("fil", "/messages/fils/{fil}",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent, Eleve, ParentAutreFoyer], GESTION_HORS_PROF),
            ecrire_sur_b("repondre-B", "POST", "/messages/fils/{fil}/repondre", Some(r#"{"corps":"separation"}"#)),
            ecrire_sur_b("archiver-B", "POST", "/messages/fils/{fil}/archiver", Some("{}")),
            ecrire_sur_b("ouvrir-fil-B", "POST", "/messages/fils",
                Some(r#"{"eleve_id":"{eleve}","sujet":"separation","message":"separation"}"#)),
        ],
    );
}

#[test]
fn domaine_factures() {
    const FACT_LIEE: &[Acteur] = &[Admin, Direction, Administratif, Compta, Parent, ParentAutreFoyer];
    const FACT_NON_LIEE: &[Acteur] = &[Admin, Direction, Administratif, Compta];
    jouer(
        "factures / foyers",
        &[
            lire("facture", "/facturation/factures/{facture}", FACT_LIEE, FACT_NON_LIEE),
            lire("facture-pdf", "/facturation/factures/{facture}/pdf", FACT_LIEE, FACT_NON_LIEE),
            lire("foyer", "/foyers/{foyer}", FOYERS, FOYERS),
            ecrire_sur_b("emettre-B", "POST", "/facturation/factures/{facture}/emettre", Some("{}")),
            ecrire_sur_b("encaisser-B", "POST", "/facturation/factures/{facture}/encaisser",
                Some(r#"{"montant_centimes":100,"moyen":"cheque"}"#)),
        ],
    );
}

#[test]
fn domaine_appel() {
    jouer(
        "appel",
        &[
            // Appel ouvert par le professeur : visible de lui et de la gestion, pas des familles.
            lire("appel", "/appels/{appel}", &[Admin, Direction, Administratif, Cpe, Prof], &[]),
            ecrire_sur_b("appel-ligne-B", "POST", "/appels/{appel}/lignes",
                Some(r#"{"eleve_id":"{eleve}","etat":"present"}"#)),
            ecrire_sur_b("appel-clore-B", "POST", "/appels/{appel}/clore", Some("{}")),
        ],
    );
}

#[test]
fn domaine_listes_de_l_etablissement() {
    jouer(
        "listes de l'établissement (aucune fuite de B)",
        &[
            lire("fils-liste", "/messages/fils",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux], &[]),
            lire("factures-liste", "/facturation/factures", &[Admin, Direction, Administratif, Compta], &[]),
            lire("foyers-liste", "/foyers", FOYERS, &[]),
            lire("mes-creneaux", "/edt/mes-creneaux", &[Admin, Direction, Administratif, Cpe, Prof, Eleve], &[]),
        ],
    );
}

// ───────────────────────── Invariants transverses ─────────────────────────

/// Routes publiques : la **seule** liste d'exceptions à « sans session ⇒ 401 ».
const PUBLIQUES: &[(&str, &str)] = &[
    ("GET", "/health"),
    ("GET", "/ready"),
    ("POST", "/labo/bootstrap"), // fermé hors labo par ENT_ENV / ENT_ALLOW_LABO
    ("POST", "/auth/register"),  // fermé hors labo
    ("POST", "/auth/login"),
    ("POST", "/auth/passkey/login/start"),
    ("POST", "/auth/passkey/login/finish"),
    ("GET", "/auth/oidc/public"),
    ("POST", "/auth/oidc/{slug}/start"),
    ("POST", "/auth/oidc/complete"),
    ("POST", "/auth/logout"),
    ("POST", "/auth/forgot-password"),
    ("POST", "/auth/reset-password"),
    ("GET", "/metrics"), // politique d'accès propre (jeton / désactivé)
];
/// Pages HTML de l'application (coquilles sans donnée) servies par `ui_page`.
const HANDLERS_PUBLICS: &[&str] = &["ui_page"];

fn routes_protegees() -> Vec<routes_src::RouteDecl> {
    let toutes = routes_src::routes();
    for (m, p) in PUBLIQUES {
        assert!(
            toutes.iter().any(|r| r.methode == *m && r.chemin == *p),
            "route publique déclarée mais absente de routes.rs : {m} {p}"
        );
    }
    toutes
        .into_iter()
        .filter(|r| {
            !PUBLIQUES.iter().any(|(m, p)| r.methode == *m && r.chemin == *p)
                && !HANDLERS_PUBLICS.contains(&r.handler.as_str())
        })
        .collect()
}

#[test]
fn invariant_sans_session_401_sur_toute_route_non_publique() {
    let Some(m) = monde() else { return };
    let routes = routes_protegees();
    assert!(routes.len() > 250, "inventaire des routes incomplet : {}", routes.len());
    let mut echecs = Vec::new();
    rt().block_on(async {
        for r in &routes {
            let chemin = routes_src::instancier(&r.chemin);
            let corps = matches!(r.methode, "POST" | "PUT" | "PATCH").then(|| json!({}));
            // Sans en-tête, puis avec un jeton inventé.
            for jeton in [None, Some("jeton-invente-sans-valeur")] {
                let rep = m.appel(r.methode, &chemin, jeton, corps.as_ref()).await;
                if rep.statut.as_u16() != 401 {
                    echecs.push(format!(
                        "{} {} ({}) jeton={} → {}",
                        r.methode,
                        r.chemin,
                        r.handler,
                        jeton.is_some(),
                        rep.statut.as_u16()
                    ));
                }
            }
        }
    });
    CAS_EXECUTES.fetch_add(routes.len() * 2, Ordering::Relaxed);
    eprintln!("[separation] sans session : {} routes × 2, {} écart(s)", routes.len(), echecs.len());
    assert!(echecs.is_empty(), "routes qui ne répondent pas 401 sans session :\n{}", echecs.join("\n"));
}

#[test]
fn invariant_etablissement_id_client_est_ignore() {
    let Some(m) = monde() else { return };
    let b = m.b.objets.etab.to_string();
    let harry = m.a.objets.eleve;
    let routes = [
        "/messages/fils".to_string(),
        "/facturation/factures".to_string(),
        "/foyers".to_string(),
        "/matieres".to_string(),
        "/audit/journal".to_string(),
        "/personnel/search?q=a".to_string(),
        "/vs/signaux".to_string(),
        "/annonces".to_string(),
        "/eleves/search?q=Potter".to_string(),
        "/edt/mes-creneaux".to_string(),
        format!("/eleves/{harry}/documents"),
        format!("/eleves/{harry}/notes"),
        format!("/eleves/{harry}/dossier"),
    ];
    let jeton = m.jeton(Direction).to_string();
    let mut echecs = Vec::new();
    rt().block_on(async {
        for r in &routes {
            let base = m.appel("GET", r, Some(&jeton), None).await;
            let sep = if r.contains('?') { '&' } else { '?' };
            let variantes: Vec<(String, Vec<(&str, String)>)> = vec![
                (format!("{r}{sep}etablissement_id={b}"), vec![]),
                (format!("{r}{sep}tenant_id={b}&etab_id={b}"), vec![]),
                (r.clone(), vec![("x-etablissement-id", b.clone())]),
                (r.clone(), vec![("x-tenant-id", b.clone()), ("etablissement-id", b.clone())]),
            ];
            for (chemin, entetes) in variantes {
                let rep = m.requete("GET", &chemin, Some(&jeton), None, &entetes).await;
                if rep.statut != base.statut {
                    echecs.push(format!("{chemin} {entetes:?} : statut {} ≠ {}", rep.statut, base.statut));
                }
                for f in fuites(m, &rep.texte, &[]) {
                    echecs.push(format!("{chemin} {entetes:?} : FUITE {f}"));
                }
            }
        }
        // Corps : un `etablissement_id` de B dans un dépôt ne doit pas déplacer la ressource.
        let titre = format!("sep-{}", Uuid::new_v4());
        let rep = m
            .appel(
                "POST",
                &format!("/eleves/{harry}/documents"),
                Some(&jeton),
                Some(&json!({"titre": titre, "tiroir": "autre", "confidentialite": "standard",
                             "etablissement_id": b, "etab_id": b})),
            )
            .await;
        if rep.est_succes() {
            let id = rep.json["id"].as_str().unwrap_or("").to_string();
            let liste_b = m
                .appel("GET", &format!("/eleves/{}/documents", m.b.objets.eleve), Some(m.b.jetons[&Direction].as_str()), None)
                .await;
            if liste_b.texte.contains(&id) || liste_b.texte.contains(&titre) {
                echecs.push("dépôt avec etablissement_id=B : document apparu dans B".into());
            }
            let liste_a = m
                .appel("GET", &format!("/eleves/{harry}/documents"), Some(&jeton), None)
                .await;
            if !liste_a.texte.contains(&id) {
                echecs.push("dépôt avec etablissement_id=B : document absent de A".into());
            }
        }
        // Un compte de A ne peut pas ouvrir de session sur B.
        let l = m
            .appel(
                "POST",
                "/auth/login",
                None,
                Some(&json!({"email": m.a.emails[&Direction], "password": m.a.mot_de_passe,
                             "etablissement_id": m.b.objets.etab})),
            )
            .await;
        if l.est_succes() {
            echecs.push("connexion d'un compte de A sur l'établissement B acceptée".into());
        }
    });
    eprintln!("[separation] etablissement_id ignoré : {} écart(s)", echecs.len());
    assert!(echecs.is_empty(), "{}", echecs.join("\n"));
}

#[test]
fn invariant_jeton_de_a_ne_voit_rien_de_b_dans_les_listes() {
    // Doublon volontaire de l'oracle : toutes les listes de A, tous acteurs, aucun id de B.
    let Some(m) = monde() else { return };
    let mut echecs = Vec::new();
    rt().block_on(async {
        for chemin in [
            "/messages/fils", "/facturation/factures", "/foyers", "/matieres", "/audit/journal",
            "/personnel/search?q=a", "/vs/signaux", "/annonces", "/edt/mes-creneaux",
            "/infirmerie/passages", "/psy/entretiens", "/documents/grants/pending",
        ] {
            for a in Acteur::TOUS {
                let rep = m.appel("GET", chemin, Some(m.jeton(a)), None).await;
                if rep.est_succes() {
                    for f in fuites(m, &rep.texte, INTERDITS_GLOBAUX) {
                        echecs.push(format!("{chemin} acteur={a:?} : {f}"));
                    }
                }
            }
        }
    });
    assert!(echecs.is_empty(), "{}", echecs.join("\n"));
}

// ───────────────────────── Failles ouvertes (à activer lot par lot) ─────────────────────────

fn essai_famille_refusee(chemin_gabarit: &str, cible: Cible, acteurs: &[Acteur]) -> Vec<String> {
    let Some(m) = monde() else { return vec![] };
    let mut echecs = Vec::new();
    rt().block_on(async {
        for &a in acteurs {
            let o = m.objets_cible(a, cible);
            let Some(chemin) = o.resoudre(&chemin_gabarit.replace("{jour}", &jour_du_test())) else { continue };
            let rep = m.appel("GET", &chemin, Some(m.jeton(a)), None).await;
            if !(rep.est_refus() || (rep.est_succes() && rep.est_vide())) {
                echecs.push(format!("{chemin} acteur={a:?} cible={cible:?} → {} (attendu refus ou vide)", rep.statut));
            }
        }
    });
    echecs
}

#[test]
fn l2_annuaire_eleves_search_ferme_aux_familles() {
    let Some(_) = monde() else { return };
    let e = essai_famille_refusee("/eleves/search?q=Potter", Cible::Liee, FAMILLE);
    assert!(e.is_empty(), "{}", e.join("\n"));
}

#[test]
fn l2_eleves_labels_ferme_aux_familles_hors_foyer() {
    let Some(_) = monde() else { return };
    // Un parent / élève ne doit obtenir que les libellés de ses propres enfants.
    let e = essai_famille_refusee("/eleves/labels?ids={eleve}", Cible::NonLiee, FAMILLE);
    assert!(e.is_empty(), "{}", e.join("\n"));
}

#[test]
fn l2_eleve_absent_controle_le_role_et_le_lien() {
    let Some(_) = monde() else { return };
    // La cible B (identifiant d'un autre établissement) est couverte par le spec « absent » (indistinct d'un id inexistant).
    let e = essai_famille_refusee("/eleves/{eleve}/absent?jour={jour}", Cible::NonLiee, FAMILLE);
    assert!(e.is_empty(), "{}", e.join("\n"));
}

#[test]
fn l2_compta_ne_voit_pas_notes_et_sanctions_via_le_dossier() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let o = m.objets_cible(Compta, Cible::Liee);
        let rep = m.appel("GET", &format!("/eleves/{}/dossier", o.eleve), Some(m.jeton(Compta)), None).await;
        let mut vus = Vec::new();
        if rep.est_succes() {
            for cle in ["notes", "sanctions", "carnet", "incidents", "continuum", "dispositifs"] {
                let v = &rep.json[cle];
                let plein = match v {
                    Value::Array(a) => !a.is_empty(),
                    Value::Object(o) => !o.is_empty(),
                    Value::Null => false,
                    _ => true,
                };
                if plein {
                    vus.push(cle);
                }
            }
        }
        assert!(vus.is_empty(), "la comptabilité voit via le dossier : {vus:?}");
    });
}

#[test]
fn l1_storage_key_fourni_par_le_client_ne_donne_pas_acces_a_un_autre_etablissement() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let secret = format!("SECRET-B-{}", Uuid::new_v4());
        let b64 = base64::engine::general_purpose::STANDARD.encode(secret.as_bytes());
        let dir_b = m.b.jetons[&Direction].as_str();
        let rb = m
            .appel(
                "POST",
                &format!("/eleves/{}/documents", m.b.objets.eleve),
                Some(dir_b),
                Some(&json!({"titre": "secret de B", "tiroir": "administratif",
                             "confidentialite": "restreint", "mime_type": "text/plain",
                             "contenu_base64": b64})),
            )
            .await;
        assert!(rb.est_succes(), "préparation : dépôt dans B refusé ({})", rb.statut);
        let id_b = rb.json["id"].as_str().expect("id du document de B");
        // Clé d'objet de B (format `{etab}/documents/{id}.txt`) fournie par un client de A.
        let cle = format!("{}/documents/{id_b}.txt", m.b.objets.etab);
        let dir_a = m.jeton(Direction);
        let ra = m
            .appel(
                "POST",
                &format!("/eleves/{}/documents", m.a.objets.eleve),
                Some(dir_a),
                Some(&json!({"titre": "pirate", "tiroir": "administratif",
                             "confidentialite": "standard", "mime_type": "text/plain",
                             "storage_key": cle})),
            )
            .await;
        if ra.est_succes() {
            let id_a = ra.json["id"].as_str().expect("id du document de A");
            let f = m.appel("GET", &format!("/documents/{id_a}/fichier"), Some(dir_a), None).await;
            assert!(
                !f.texte.contains(&secret),
                "le contenu d'un document de B est lisible depuis A via storage_key"
            );
        }
    });
}

/// Crée (par l'API) un parent neuf rattaché à Luna ; renvoie (email, user_id).
async fn rattacher_parent_neuf(m: &Monde, mot_de_passe: Option<&str>) -> (String, Option<Uuid>, Rep) {
    let email = format!("sep-{}@example.test", Uuid::new_v4().simple());
    let mut corps = json!({"eleve_id": m.a.objets.eleve_neutre, "email": email,
                           "display_name": "Parent de test", "lien": "autre"});
    if let Some(p) = mot_de_passe {
        corps["password"] = json!(p);
    }
    let rep = m
        .appel(
            "POST",
            &format!("/foyers/{}/liens", m.a.objets.foyer_neutre),
            Some(m.jeton(Direction)),
            Some(&corps),
        )
        .await;
    let uid = rep.json["user_id"].as_str().and_then(|s| Uuid::parse_str(s).ok());
    (email, uid, rep)
}

#[test]
fn l3_administratif_ne_change_pas_l_email_de_l_admin() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let admin = m.a.user_ids[&Admin];
        let piege = format!("pirate-{}@example.test", Uuid::new_v4().simple());
        let rep = m
            .appel("PUT", &format!("/users/{admin}/email"), Some(m.jeton(Administratif)), Some(&json!({"email": piege})))
            .await;
        // Le refus est le contrat : aucune remise en état n'est nécessaire (et une
        // modification réussie aurait révoqué les sessions du monde partagé).
        assert!(rep.est_refus(), "l'administratif a modifié l'e-mail de l'admin → {}", rep.statut);
    });
}

/// Règle L3 : la direction peut déclencher l'envoi d'un lien à un parent, mais ne
/// reçoit jamais de secret et ne peut pas se connecter à sa place. Le mot de
/// passe envoyé au rattachement est ignoré.
#[test]
fn l3_direction_declenche_un_lien_pour_un_parent_et_ne_recoit_aucun_secret() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let (email, uid, prep) = rattacher_parent_neuf(m, Some("MotDePasseImpose-1")).await;
        assert!(prep.est_succes(), "préparation : rattachement refusé ({})", prep.statut);
        let uid = uid.expect("user_id du parent créé");
        let rep = m
            .appel("POST", &format!("/etablissement/personnel/{uid}/reset-mot-de-passe"), Some(m.jeton(Direction)), Some(&json!({})))
            .await;
        assert!(
            [202, 503].contains(&rep.statut.as_u16()),
            "envoi de lien (202) ou « envoi impossible » attendu, reçu {}",
            rep.statut
        );
        assert!(!rep.texte.contains("mot_de_passe_temporaire"), "secret renvoyé : {}", rep.texte);
        for mdp in ["MotDePasseImpose-1", m.a.mot_de_passe.as_str()] {
            let l = m
                .appel("POST", "/auth/login", None,
                       Some(&json!({"email": email, "password": mdp, "etablissement_id": m.a.objets.etab})))
                .await;
            assert!(!l.est_succes(), "connexion possible avec un secret connu de l'appelant");
        }
    });
}

/// Règle L3 : le personnel n'est jamais réinitialisé hors hiérarchie (administratif refusé).
#[test]
fn l3_administratif_ne_reinitialise_pas_un_parent() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let (_e, uid, prep) = rattacher_parent_neuf(m, None).await;
        assert!(prep.est_succes(), "préparation : rattachement refusé ({})", prep.statut);
        let uid = uid.expect("user_id du parent créé");
        let rep = m
            .appel("POST", &format!("/etablissement/personnel/{uid}/reset-mot-de-passe"), Some(m.jeton(Administratif)), Some(&json!({})))
            .await;
        assert!(rep.est_refus(), "l'administratif a déclenché une réinitialisation → {}", rep.statut);
    });
}

/// Corrigé par L4 : le rattachement n'utilise plus jamais le mot de passe du labo.
#[test]
fn l3_rattachement_parent_sans_mot_de_passe_ne_donne_pas_le_mot_de_passe_par_defaut() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let (email, _uid, prep) = rattacher_parent_neuf(m, None).await;
        if !prep.est_succes() {
            return; // refusé sans mot de passe explicite : comportement sûr
        }
        let l = m
            .appel(
                "POST",
                "/auth/login",
                None,
                Some(&json!({"email": email, "password": m.a.mot_de_passe,
                             "etablissement_id": m.a.objets.etab})),
            )
            .await;
        assert!(!l.est_succes(), "le compte parent créé accepte le mot de passe par défaut du labo");
    });
}

#[test]
fn l1_dossier_n_expose_pas_les_cles_d_objets() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let mut vus = Vec::new();
        for a in Acteur::TOUS {
            let o = m.objets_cible(a, Cible::Liee);
            let rep = m.appel("GET", &format!("/eleves/{}/dossier", o.eleve), Some(m.jeton(a)), None).await;
            if rep.est_succes() && cle_non_nulle(&rep.json, "storage_key") {
                vus.push(format!("{a:?}"));
            }
        }
        assert!(vus.is_empty(), "storage_key exposé dans /dossier pour : {vus:?}");
    });
}

/// Dépose un document avec contenu texte via l'API ; renvoie (id, réponse).
async fn deposer_texte(
    m: &Monde,
    jeton: &str,
    eleve: Uuid,
    tiroir: &str,
    confid: &str,
    contenu: &str,
) -> (Option<String>, Rep) {
    let b64 = base64::engine::general_purpose::STANDARD.encode(contenu.as_bytes());
    let rep = m
        .appel(
            "POST",
            &format!("/eleves/{eleve}/documents"),
            Some(jeton),
            Some(&json!({"titre": "l1", "tiroir": tiroir, "confidentialite": confid,
                         "mime_type": "text/plain", "contenu_base64": b64})),
        )
        .await;
    let id = rep.json["id"].as_str().map(String::from);
    (id, rep)
}

#[test]
fn l1_parent_ne_lit_jamais_un_objet_choisi_par_cle() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let secret = format!("SECRET-L1-{}", Uuid::new_v4());
        let dir = m.jeton(Direction);
        // Document restreint de Harry, contenu connu, créé par la direction.
        let (id_r, r) = deposer_texte(m, dir, m.a.objets.eleve, "administratif", "restreint", &secret).await;
        assert!(r.est_succes(), "préparation : dépôt restreint ({})", r.statut);
        let id_r = id_r.expect("id restreint");
        let cle_restreint = format!("{}/documents/{id_r}.txt", m.a.objets.etab);
        // Objet de B.
        let secret_b = format!("SECRET-B-L1-{}", Uuid::new_v4());
        let (id_b, rb) = deposer_texte(m, m.b.jetons[&Direction].as_str(), m.b.objets.eleve, "administratif", "restreint", &secret_b).await;
        assert!(rb.est_succes(), "préparation : dépôt dans B ({})", rb.statut);
        let cle_b = format!("{}/documents/{}.txt", m.b.objets.etab, id_b.expect("id B"));
        let cles = [
            (cle_restreint, secret.clone()),
            (cle_b, secret_b.clone()),
            ("C:/Windows/win.ini".to_string(), "[fonts]".to_string()),
            ("../../../../Windows/win.ini".to_string(), "[fonts]".to_string()),
        ];
        // Le parent de Harry (dépôt famille) et la direction tentent chaque clé.
        for (acteur, tiroir, confid) in [(Parent, "famille", "standard"), (Direction, "administratif", "standard")] {
            for (cle, marqueur) in &cles {
                let rep = m
                    .appel(
                        "POST",
                        &format!("/eleves/{}/documents", m.a.objets.eleve),
                        Some(m.jeton(acteur)),
                        Some(&json!({"titre": "pirate", "tiroir": tiroir, "confidentialite": confid,
                                     "mime_type": "text/plain", "storage_key": cle})),
                    )
                    .await;
                assert!(!rep.texte.contains(marqueur.as_str()), "{acteur:?} : fuite dans la réponse du dépôt");
                if rep.est_succes() {
                    let id = rep.json["id"].as_str().expect("id").to_string();
                    for lecteur in [acteur, Direction, Admin] {
                        let f = m.appel("GET", &format!("/documents/{id}/fichier"), Some(m.jeton(lecteur)), None).await;
                        assert!(
                            !f.texte.contains(marqueur.as_str()),
                            "{acteur:?} avec la clé {cle} : octets lus par {lecteur:?}"
                        );
                    }
                }
            }
        }
        // Le document restreint reste verrouillé pour le parent.
        let f = m.appel("GET", &format!("/documents/{id_r}/fichier"), Some(m.jeton(Parent)), None).await;
        assert!(f.est_refus(), "document restreint lisible par le parent ({})", f.statut);
    });
}

#[test]
fn l1_documents_legitimes_se_telechargent_avec_leur_contenu_exact() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let contenu = format!("CONTENU-LEGITIME-{}", Uuid::new_v4());
        // Dépôt par la direction (famille, standard) puis dépôt par le parent lui-même.
        let (id_d, r) = deposer_texte(m, m.jeton(Direction), m.a.objets.eleve, "famille", "standard", &contenu).await;
        assert!(r.est_succes(), "dépôt direction ({})", r.statut);
        let (id_p, r) = deposer_texte(m, m.jeton(Parent), m.a.objets.eleve, "famille", "standard", &contenu).await;
        assert!(r.est_succes(), "dépôt parent ({})", r.statut);
        for id in [id_d.unwrap(), id_p.unwrap()] {
            for a in [Admin, Direction, Administratif, Parent, Eleve] {
                let f = m.appel("GET", &format!("/documents/{id}/fichier"), Some(m.jeton(a)), None).await;
                assert_eq!(f.statut.as_u16(), 200, "{a:?} ne télécharge pas son document légitime");
                assert_eq!(f.texte, contenu, "{a:?} : contenu altéré");
            }
            for a in [ParentAutreFoyer, ParentSansDrapeaux, Compta, Prof, Infirmiere, Psy] {
                let f = m.appel("GET", &format!("/documents/{id}/fichier"), Some(m.jeton(a)), None).await;
                assert!(f.est_refus(), "{a:?} lit un document hors périmètre ({})", f.statut);
            }
            let f = m.appel("GET", &format!("/documents/{id}/fichier"), Some(m.b.jetons[&Direction].as_str()), None).await;
            assert!(f.est_refus(), "la direction de B lit un document de A");
        }
    });
}

#[test]
fn zz_bilan() {
    eprintln!("[separation] cas exécutés dans ce processus : {}", CAS_EXECUTES.load(Ordering::Relaxed));
}

// ───────────────────────── Annuaire (lot L2 corrigé) ─────────────────────────

#[test]
fn domaine_annuaire_et_vie_scolaire_famille() {
    jouer(
        "annuaire / absent / journal / points VS",
        &[
            // Recherche et libellés : personnel uniquement (parents et élèves : 403).
            lire("search", "/eleves/search?q=Potter", STAFF, &[]),
            lire("labels", "/eleves/labels?ids={eleve}", STAFF, STAFF),
            Spec { b: Attendu::RefuseOuIndistinct, ..lire("absent", "/eleves/{eleve}/absent?jour={jour}",
                &[Admin, Direction, Administratif, Cpe, Prof, Compta, Infirmiere, Psy, Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux],
                STAFF) },
            // Un parent doit avoir le drapeau « vie scolaire » (Narcissa, Molly : non).
            lire("journal", "/eleves/{eleve}/journal", &[Admin, Direction, Administratif, Cpe, Prof, Compta, Infirmiere, Psy, Parent, Eleve], STAFF_HORS_PROF),
            Spec { b: Attendu::RefuseOuIndistinct, ..lire("points-vs", "/eleves/{eleve}/points-vs",
                &[Admin, Direction, Administratif, Cpe, Parent],
                &[Admin, Direction, Administratif, Cpe]) },
        ],
    );
}


#[test]
fn l2_points_vs_exige_le_drapeau_vie_scolaire() {
    let Some(_) = monde() else { return };
    // Narcissa (Draco) et Molly (Ginny) n'ont pas peut_voir_vs : refus attendu.
    let e = essai_famille_refusee("/eleves/{eleve}/points-vs", Cible::Liee, &[ParentAutreFoyer, ParentSansDrapeaux]);
    assert!(e.is_empty(), "{}", e.join("\n"));
}

// ───────────────────────── Identité élève par niveau ─────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
enum Niveau {
    Complet,
    Sante,
    Reduit,
}

fn niveau_de(a: Acteur) -> Niveau {
    match a {
        Admin | Direction | Administratif | Parent | Eleve => Niveau::Complet,
        Infirmiere => Niveau::Sante,
        _ => Niveau::Reduit,
    }
}

/// Écarts d'un objet élève par rapport au niveau d'identité de l'acteur.
fn ecarts_identite(e: &Value, n: Niveau, ou: &str, a: Acteur) -> Vec<String> {
    let mut out = Vec::new();
    if n == Niveau::Complet {
        return out;
    }
    let mut interdits = vec!["ine", "numero_badge", "regime", "mef", "motif_sortie", "date_entree", "date_sortie"];
    if n == Niveau::Reduit {
        interdits.extend(["sexe", "date_naissance"]);
    }
    for c in interdits {
        if !e[c].is_null() {
            out.push(format!("{ou} acteur={a:?} ({n:?}) : champ « {c} » exposé"));
        }
    }
    if e["redoublant"] == json!(true) {
        out.push(format!("{ou} acteur={a:?} ({n:?}) : « redoublant » exposé"));
    }
    out
}

#[test]
fn identite_eleve_par_niveau_sur_toutes_les_routes() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let harry = m.a.objets.eleve;
        let jour = jour_du_test();
        let mut ecarts = Vec::new();
        let mut vus = 0;
        for a in [Admin, Direction, Administratif, Cpe, Prof, Compta, Infirmiere, Psy, Parent, Eleve] {
            let n = niveau_de(a);
            let sources = [
                (format!("/eleves/{harry}"), "obj"),
                ("/eleves/search?q=Potter".to_string(), "arr"),
                (format!("/classes/4B/effectif?jour={jour}"), "eleves"),
                (format!("/eleves/{harry}/dossier"), "eleve"),
            ];
            for (chemin, forme) in sources {
                let rep = m.appel("GET", &chemin, Some(m.jeton(a)), None).await;
                if !rep.est_succes() {
                    continue; // l'accès lui-même est jugé par les autres tests
                }
                let objets: Vec<Value> = match forme {
                    "obj" => vec![rep.json.clone()],
                    "arr" => rep.json.as_array().cloned().unwrap_or_default(),
                    cle => match &rep.json[cle] {
                        Value::Array(v) => v.clone(),
                        v @ Value::Object(_) => vec![v.clone()],
                        _ => vec![],
                    },
                };
                for e in &objets {
                    vus += 1;
                    ecarts.extend(ecarts_identite(e, n, chemin.split('?').next().unwrap(), a));
                }
                // Positif : les rôles « complet » gardent la date de naissance ou le sexe.
                if n == Niveau::Complet && forme == "obj" {
                    assert!(
                        !rep.json["date_naissance"].is_null() || !rep.json["sexe"].is_null(),
                        "{a:?} : identité complète attendue sur {chemin}"
                    );
                }
                // Infirmerie : sexe et date de naissance conservés.
                if n == Niveau::Sante && forme == "obj" {
                    assert!(
                        !rep.json["date_naissance"].is_null() || !rep.json["sexe"].is_null(),
                        "Infirmerie : sexe / date de naissance attendus sur {chemin}"
                    );
                }
            }
        }
        eprintln!("[separation] identité par niveau : {vus} objets élève contrôlés");
        assert!(vus >= 20, "trop peu d'objets contrôlés ({vus})");
        assert!(ecarts.is_empty(), "{}", ecarts.join("\n"));
    });
}

#[test]
fn recherche_par_badge_reservee_au_niveau_complet() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let harry = m.a.objets.eleve;
        let fiche = m.appel("GET", &format!("/eleves/{harry}"), Some(m.jeton(Direction)), None).await;
        let Some(badge) = fiche.json["numero_badge"].as_str().map(String::from) else {
            panic!("le labo ne fournit pas de numéro de badge à Harry : cas non vérifiable");
        };
        let chemin = format!("/eleves/search?q={badge}");
        let adm = m.appel("GET", &chemin, Some(m.jeton(Administratif)), None).await;
        assert!(
            adm.texte.contains(&harry.to_string()),
            "l'administratif ne retrouve pas l'élève par son badge"
        );
        for a in [Prof, Compta, Cpe, Psy, Infirmiere] {
            let rep = m.appel("GET", &chemin, Some(m.jeton(a)), None).await;
            assert!(
                rep.est_refus() || (rep.est_succes() && rep.est_vide()),
                "{a:?} retrouve un élève par son badge ({})",
                rep.statut
            );
            assert!(!rep.texte.contains(&badge), "{a:?} : numéro de badge dans la réponse");
        }
    });
}
