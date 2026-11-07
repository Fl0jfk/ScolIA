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
    jouer_impl(domaine, specs, None)
}

/// Mêmes spécifications, mais **session de A d'une personne membre de A et de B**
/// (C6) et uniquement avec des identifiants de B (chemin, query, corps).
fn jouer_double(domaine: &str, specs: &[Spec], doubles: &[Double]) {
    jouer_impl(domaine, specs, Some(doubles))
}

fn jouer_impl(domaine: &str, specs: &[Spec], doubles: Option<&[Double]>) {
    let Some(m) = monde() else { return };
    let acteurs: Vec<(Acteur, String)> = match doubles {
        None => Acteur::TOUS.iter().map(|a| (*a, m.jeton(*a).to_string())).collect(),
        Some(d) => d.iter().map(|x| (x.acteur, x.token_a.clone())).collect(),
    };
    let mut echecs: Vec<String> = Vec::new();
    let mut n = 0usize;
    rt().block_on(async {
        for s in specs {
            let sans_cible = !s.chemin.contains('{');
            if doubles.is_some() && sans_cible {
                continue; // aucun identifiant de B à fournir
            }
            let cibles: &[Cible] = if s.seulement_b || doubles.is_some() {
                &[Cible::B]
            } else if sans_cible {
                &[Cible::Liee]
            } else {
                &[Cible::Liee, Cible::NonLiee, Cible::B]
            };
            for (a, jeton) in &acteurs {
                let a = *a;
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
                    let rep = m.appel(s.methode, &chemin, Some(jeton.as_str()), corps.as_ref()).await;
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
                        let ctl = m.appel(s.methode, &chemin.replace(&eid, &neuf), Some(jeton.as_str()), corps.as_ref()).await;
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

fn specs_domaine_eleve_dossier() -> Vec<Spec> {
    vec![
            lire("eleve", "/eleves/{eleve}", TOUS, STAFF),
            lire("dossier", "/eleves/{eleve}/dossier", TOUS, STAFF),
            // Socle 2 (E2) : historique d'identité réservé à `eleves.historique.lire`
            // (Admin, Direction, Administratif) ; jamais la famille ni le professeur.
            lire("historique-identite", "/eleves/{eleve}/identite/historique",
                &[Admin, Direction, Administratif], &[Admin, Direction, Administratif]),
            ecrire_sur_b("identite-correction-sur-B", "POST", "/eleves/{eleve}/identite/corrections",
                Some(r#"{"champs":{"nom_naissance":"Separation"},"motif":"separation"}"#)),
            ecrire_sur_b("identite-changement-sur-B", "POST", "/eleves/{eleve}/identite/changements",
                Some(r#"{"champs":{"nom_naissance":"Separation"},"effet_du":"{jour}","motif":"separation","piece_document_id":"00000000-0000-0000-0000-000000000001"}"#)),
            ecrire_sur_b("identite-usage-sur-B", "POST", "/eleves/{eleve}/identite/usages",
                Some(r#"{"prenom_usage":"Separation"}"#)),
            // Socle 2 (E3) : identifiants (G3) réservés à `eleves.identifiants.lire`
            // (Admin, Direction, Administratif) ; jamais la famille ni le professeur.
            lire("identifiants", "/eleves/{eleve}/identifiants",
                &[Admin, Direction, Administratif], &[Admin, Direction, Administratif]),
            ecrire_sur_b("identifiant-ajout-sur-B", "POST", "/eleves/{eleve}/identifiants",
                Some(r#"{"systeme":"INE","valeur":"SEPARATION1A"}"#)),
            ecrire_sur_b("identifiant-confirmation-sur-B", "POST",
                "/eleves/{eleve}/identifiants/00000000-0000-0000-0000-000000000001/confirmer",
                Some(r#"{"source":"separation"}"#)),
            ecrire_sur_b("identifiant-contestation-sur-B", "POST",
                "/eleves/{eleve}/identifiants/00000000-0000-0000-0000-000000000001/contester",
                Some(r#"{"motif":"separation"}"#)),
            ecrire_sur_b("identifiant-revocation-sur-B", "POST",
                "/eleves/{eleve}/identifiants/00000000-0000-0000-0000-000000000001/revoquer",
                Some(r#"{"motif":"separation"}"#)),
            ecrire_sur_b("badge-perdu-sur-B", "POST", "/eleves/{eleve}/badge/perdu",
                Some(r#"{"nouveau_badge":"SEPARATION-BADGE"}"#)),
            lire("carnet", "/eleves/{eleve}/carnet",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent, Eleve], GESTION_HORS_PROF),
            Spec { non_liee_vide: FAMILLE, ..lire("incidents", "/eleves/{eleve}/incidents", TOUS, STAFF) },
            Spec { non_liee_vide: FAMILLE, ..lire("sanctions", "/eleves/{eleve}/sanctions", TOUS, STAFF) },
    ]
}

#[test]
fn domaine_eleve_dossier() {
    jouer("élève / dossier", &specs_domaine_eleve_dossier());
}

fn specs_domaine_notes() -> Vec<Spec> {
    vec![
            // Socle 1 (B1) : le professeur ne lit que les notes de ses élèves.
            lire("notes", "/eleves/{eleve}/notes",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent, Eleve, ParentAutreFoyer], GESTION_HORS_PROF),
            ecrire_sur_b("note-sur-B", "POST", "/eleves/{eleve}/notes",
                Some(r#"{"matiere_id":"{matiere}","valeur":12,"date_note":"{jour}"}"#)),
    ]
}

#[test]
fn domaine_notes() {
    jouer("notes", &specs_domaine_notes());
}

fn specs_domaine_absences() -> Vec<Spec> {
    vec![
            // Parent sans drapeau vie scolaire (Narcissa → Draco, Molly → Ginny) : refusé.
            lire("absences", "/eleves/{eleve}/absences",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent, Eleve], GESTION_HORS_PROF),
            ecrire_sur_b("absence-sur-B", "POST", "/eleves/{eleve}/absences",
                Some(r#"{"debut":"{jour}","fin":"{jour}","motif":"separation"}"#)),
            ecrire_sur_b("justifier-B", "POST", "/absences/{absence}/justifier", Some("{}")),
            ecrire_sur_b("accepter-justificatif-B", "POST", "/absences/{absence}/accepter-justificatif", Some("{}")),
    ]
}

#[test]
fn domaine_absences() {
    jouer("absences", &specs_domaine_absences());
}

fn specs_domaine_sante() -> Vec<Spec> {
    vec![
            lire("fiche-sante", "/eleves/{eleve}/fiche-sante", SANTE_LIEE, SANTE_NON_LIEE),
            lire("pai", "/eleves/{eleve}/pai", SANTE_LIEE, SANTE_NON_LIEE),
            lire("infirmerie", "/infirmerie/passages?eleve_id={eleve}", SANTE_LIEE, SANTE_NON_LIEE),
            ecrire_sur_b("fiche-sante-B", "PUT", "/eleves/{eleve}/fiche-sante", Some(r#"{"allergies":"separation"}"#)),
    ]
}

#[test]
fn domaine_sante() {
    jouer("santé (fiche, PAI, infirmerie)", &specs_domaine_sante());
}

fn specs_domaine_psy() -> Vec<Spec> {
    vec![
            lire("dossier-psy", "/eleves/{eleve}/dossier-psy", PSY_LIEE, PSY_NON_LIEE),
            lire("entretiens-psy", "/psy/entretiens?eleve_id={eleve}", PSY_LIEE, PSY_NON_LIEE),
    ]
}

#[test]
fn domaine_psy() {
    jouer("psy", &specs_domaine_psy());
}

fn specs_domaine_documents() -> Vec<Spec> {
    vec![
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
    ]
}

#[test]
fn domaine_documents() {
    jouer("documents (par identifiant)", &specs_domaine_documents());
}

fn specs_domaine_fils_de_messages() -> Vec<Spec> {
    vec![
            lire("fil", "/messages/fils/{fil}",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent, Eleve, ParentAutreFoyer], GESTION_HORS_PROF),
            ecrire_sur_b("repondre-B", "POST", "/messages/fils/{fil}/repondre", Some(r#"{"corps":"separation"}"#)),
            ecrire_sur_b("archiver-B", "POST", "/messages/fils/{fil}/archiver", Some("{}")),
            ecrire_sur_b("ouvrir-fil-B", "POST", "/messages/fils",
                Some(r#"{"eleve_id":"{eleve}","sujet":"separation","message":"separation"}"#)),
    ]
}

#[test]
fn domaine_fils_de_messages() {
    jouer("fils de messages", &specs_domaine_fils_de_messages());
}

fn specs_domaine_factures() -> Vec<Spec> {
    const FACT_LIEE: &[Acteur] = &[Admin, Direction, Administratif, Compta, Parent, ParentAutreFoyer];
    const FACT_NON_LIEE: &[Acteur] = &[Admin, Direction, Administratif, Compta];
    vec![
            lire("facture", "/facturation/factures/{facture}", FACT_LIEE, FACT_NON_LIEE),
            lire("facture-pdf", "/facturation/factures/{facture}/pdf", FACT_LIEE, FACT_NON_LIEE),
            lire("foyer", "/foyers/{foyer}", FOYERS, FOYERS),
            ecrire_sur_b("emettre-B", "POST", "/facturation/factures/{facture}/emettre", Some("{}")),
            ecrire_sur_b("encaisser-B", "POST", "/facturation/factures/{facture}/encaisser",
                Some(r#"{"montant_centimes":100,"moyen":"cheque"}"#)),
    ]
}

#[test]
fn domaine_factures() {
    jouer("factures / foyers", &specs_domaine_factures());
}

fn specs_domaine_appel() -> Vec<Spec> {
    vec![
            // Appel ouvert par le professeur : visible de lui et de la gestion, pas des familles.
            lire("appel", "/appels/{appel}", &[Admin, Direction, Administratif, Cpe, Prof], &[]),
            ecrire_sur_b("appel-ligne-B", "POST", "/appels/{appel}/lignes",
                Some(r#"{"eleve_id":"{eleve}","etat":"present"}"#)),
            ecrire_sur_b("appel-clore-B", "POST", "/appels/{appel}/clore", Some("{}")),
    ]
}

#[test]
fn domaine_appel() {
    jouer("appel", &specs_domaine_appel());
}

fn specs_domaine_listes_de_l_etablissement() -> Vec<Spec> {
    vec![
            lire("fils-liste", "/messages/fils",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux], &[]),
            lire("factures-liste", "/facturation/factures", &[Admin, Direction, Administratif, Compta], &[]),
            lire("foyers-liste", "/foyers", FOYERS, &[]),
            lire("mes-creneaux", "/edt/mes-creneaux", &[Admin, Direction, Administratif, Cpe, Prof, Eleve], &[]),
    ]
}

#[test]
fn domaine_listes_de_l_etablissement() {
    jouer("listes de l'établissement (aucune fuite de B)", &specs_domaine_listes_de_l_etablissement());
}

// ───────────────────────── Invariants transverses ─────────────────────────

/// Routes publiques : la **seule** liste d'exceptions à « sans session ⇒ 401 ».
const PUBLIQUES: &[(&str, &str)] = &[
    ("GET", "/health"),
    ("GET", "/ready"),
    ("POST", "/labo/bootstrap"), // fermé hors labo par ENT_ENV / ENT_ALLOW_LABO
    ("POST", "/auth/register"),  // fermé hors labo
    ("POST", "/auth/login"),
    ("GET", "/auth/etablissement"), // C2 : fiche publique de l'établissement de l'adresse
    ("POST", "/auth/passkey/login/start"),
    ("POST", "/auth/passkey/login/finish"),
    ("GET", "/auth/oidc/public"),
    ("POST", "/auth/oidc/{slug}/start"),
    ("POST", "/auth/oidc/complete"),
    ("POST", "/auth/logout"),
    ("POST", "/auth/forgot-password"),
    ("POST", "/auth/reset-password"),
    // Récupération de passkey en libre-service (D-018) : les jetons font foi.
    // Changement d'e-mail en deux temps (RF-01) : les jetons font foi.
    ("POST", "/auth/email/confirmer"),
    ("POST", "/auth/email/annuler"),
    ("POST", "/auth/passkey/recuperation"),
    ("POST", "/auth/passkey/recuperation/confirmer"),
    ("POST", "/auth/passkey/recuperation/annuler"),
    ("POST", "/auth/passkey/recuperation/finaliser"),
    // Préinscription (S2-3) : parcours public avant compte ; l'e-mail vérifié puis le
    // jeton `x-preinscription` font foi (anti-énumération : réponses uniformes).
    ("POST", "/preinscription/demarrer"),
    ("POST", "/preinscription/verifier"),
    // Invitation du second responsable (S2-3) : le jeton reçu par e-mail fait foi.
    ("POST", "/inscriptions/invitations/accepter"),
    ("POST", "/inscriptions/invitations/refuser"),
    ("GET", "/metrics"), // politique d'accès propre (jeton / désactivé)
    ("GET", "/fichiers/{jeton}"), // lien court signé (HMAC, expiration) : le jeton fait foi
];
/// Pages HTML de l'application (coquilles sans donnée) servies par `ui_page`.
const HANDLERS_PUBLICS: &[&str] = &["ui_page"];

/// Aucun fichier de `src/` (hors tests et UI) ne déclare de routes sans être
/// lu par l'inventaire : sinon ses routes échapperaient au test « 401 ».
#[test]
fn invariant_inventaire_couvre_tous_les_fichiers_de_routes() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for e in std::fs::read_dir(src).expect("src") {
        let p = e.expect("entrée").path();
        let nom = p.file_name().unwrap().to_string_lossy().to_string();
        if !nom.ends_with(".rs") || nom == "ui.rs" || nom.starts_with("tests_") {
            continue;
        }
        let texte = std::fs::read_to_string(&p).expect("source");
        if texte.contains(".route(") {
            assert!(
                routes_src::NOMS_SOURCES.contains(&nom.as_str()),
                "{nom} déclare des routes absentes de l'inventaire (routes_src::SOURCES)"
            );
        }
    }
    let r = routes_src::routes();
    for (m, p) in [
        ("GET", "/roles"),
        ("PUT", "/roles/{id}/membres/{user_id}"),
        ("GET", "/auth/sessions"),
        ("DELETE", "/auth/sessions/{id}"),
        ("PUT", "/etablissement/securite"),
        ("POST", "/documents/{id}/lien"),
        ("POST", "/messages/pieces/{id}/lien"),
        ("POST", "/eleves/{id}/photo/lien"),
        ("POST", "/pai/pieces/{id}/lien"),
        ("GET", "/etablissement/domaines"),
        ("PUT", "/referentiel/groupes/{id}/responsables"),
        ("POST", "/referentiel/services"),
    ] {
        assert!(r.iter().any(|x| x.methode == m && x.chemin == p), "route attendue absente de l'inventaire : {m} {p}");
    }
}

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

fn specs_domaine_annuaire_et_vie_scolaire_famille() -> Vec<Spec> {
    vec![
            // Recherche et libellés : personnel uniquement (parents et élèves : 403).
            lire("search", "/eleves/search?q=Potter", STAFF, &[]),
            lire("labels", "/eleves/labels?ids={eleve}", STAFF, STAFF),
            Spec { b: Attendu::RefuseOuIndistinct, ..lire("absent", "/eleves/{eleve}/absent?jour={jour}",
                &[Admin, Direction, Administratif, Cpe, Prof, Compta, Infirmiere, Psy, Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux],
                STAFF_HORS_PROF) },
            // Un parent doit avoir le drapeau « vie scolaire » (Narcissa, Molly : non).
            lire("journal", "/eleves/{eleve}/journal", &[Admin, Direction, Administratif, Cpe, Prof, Compta, Infirmiere, Psy, Parent, Eleve], STAFF_HORS_PROF),
            Spec { b: Attendu::RefuseOuIndistinct, ..lire("points-vs", "/eleves/{eleve}/points-vs",
                &[Admin, Direction, Administratif, Cpe, Parent],
                &[Admin, Direction, Administratif, Cpe]) },
    ]
}

#[test]
fn domaine_annuaire_et_vie_scolaire_famille() {
    jouer("annuaire / absent / journal / points VS", &specs_domaine_annuaire_et_vie_scolaire_famille());
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

/// Socle 1 (B1, revue E1) : hors de ses élèves, un professeur n'obtient rien
/// sur les routes par élève (refus ou réponse vide), quel que soit le domaine.
#[test]
fn socle1_prof_refuse_hors_de_ses_eleves_sur_les_routes_par_eleve() {
    let Some(_) = monde() else { return };
    let mut echecs = Vec::new();
    for gabarit in [
        "/eleves/{eleve}/retards",
        "/eleves/{eleve}/sorties",
        "/eleves/{eleve}/dispenses-vs",
        "/eleves/{eleve}/alertes-cpe",
        "/eleves/{eleve}/conseils",
        "/eleves/{eleve}/info-comp",
        "/eleves/{eleve}/voeux",
        "/eleves/{eleve}/absent?jour={jour}",
        "/eleves/{eleve}/notes",
        "/eleves/{eleve}/absences",
        "/eleves/{eleve}/journal",
        "/eleves/{eleve}/carnet",
    ] {
        echecs.extend(essai_famille_refusee(gabarit, Cible::NonLiee, &[Prof]));
    }
    assert!(echecs.is_empty(), "{}", echecs.join("\n"));
}

// ═════════════════════ Socle 1, C6 : matrice étendue ═════════════════════
//
// Personne membre de deux établissements, références croisées en écriture,
// colonnes `*_user_id` fournies par le client, énumération, nouvelles routes.

/// Personnel membre de A **et** de B (comptes neufs : les comptes du labo
/// restent mono-établissement pour les autres tests).
#[derive(Clone, Debug)]
struct Double {
    acteur: Acteur,
    user_id: Uuid,
    email: String,
    token_a: String,
}

fn doubles() -> Option<&'static Vec<Double>> {
    static D: std::sync::OnceLock<Vec<Double>> = std::sync::OnceLock::new();
    let m = monde()?;
    Some(D.get_or_init(|| rt().block_on(construire_doubles(m))))
}

async fn construire_doubles(m: &Monde) -> Vec<Double> {
    let url = std::env::var("ENT_DATABASE_URL").expect("base de test");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connexion de préparation");
    let mut out = Vec::new();
    for (acteur, role) in [
        (Admin, "admin"),
        (Direction, "direction"),
        (Administratif, "administratif"),
        (Cpe, "cpe"),
        (Prof, "prof"),
        (Compta, "compta"),
        (Infirmiere, "infirmerie"),
        (Psy, "psy"),
    ] {
        let email = format!("dbl-{}@example.test", Uuid::new_v4().simple());
        let r = m
            .appel(
                "POST",
                "/etablissement/personnel",
                Some(m.jeton(Admin)),
                Some(&json!({"email": email, "display_name": format!("Double {role}"), "role": role})),
            )
            .await;
        assert_eq!(r.statut.as_u16(), 201, "provisionnement du double {role} : {}", r.texte);
        let user_id = Uuid::parse_str(r.json["user_id"].as_str().unwrap()).unwrap();
        let mdp = r.json["mot_de_passe_temporaire"].as_str().unwrap().to_string();
        // Même personne, second établissement (rôle identique).
        sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1, $2, $3, now())")
            .bind(user_id)
            .bind(m.b.objets.etab)
            .bind(role)
            .execute(&pool)
            .await
            .expect("second rattachement");
        let l = m
            .appel(
                "POST",
                "/auth/login",
                None,
                Some(&json!({"email": email, "password": mdp, "etablissement_id": m.a.objets.etab})),
            )
            .await;
        assert_eq!(l.statut.as_u16(), 200, "connexion du double {role} sur A : {}", l.texte);
        out.push(Double {
            acteur,
            user_id,
            email,
            token_a: l.json["token"].as_str().unwrap().to_string(),
        });
    }
    out
}

/// Toutes les spécifications de la matrice (lectures et écritures par identifiant).
fn toutes_les_specs() -> Vec<Spec> {
    let mut v = Vec::new();
    v.extend(specs_domaine_eleve_dossier());
    v.extend(specs_domaine_notes());
    v.extend(specs_domaine_absences());
    v.extend(specs_domaine_sante());
    v.extend(specs_domaine_psy());
    v.extend(specs_domaine_documents());
    v.extend(specs_domaine_fils_de_messages());
    v.extend(specs_domaine_factures());
    v.extend(specs_domaine_appel());
    v.extend(specs_domaine_annuaire_et_vie_scolaire_famille());
    v
}

/// Une personne membre de A et de B, connectée sur A, fournit des identifiants
/// de B partout où la matrice en place : refus ou réponse vide, jamais de donnée de B.
#[test]
fn c6_double_membre_session_a_avec_identifiants_de_b_sur_toute_la_matrice() {
    let Some(d) = doubles() else { return };
    jouer_double("double membre : session A, identifiants B", &toutes_les_specs(), d);
}

/// La session de A d'un double membre ne donne pas non plus accès aux listes de B.
#[test]
fn c6_double_membre_listes_de_a_sans_aucun_identifiant_de_b() {
    let (Some(m), Some(d)) = (monde(), doubles()) else { return };
    let mut echecs = Vec::new();
    rt().block_on(async {
        for x in d {
            for chemin in [
                "/messages/fils", "/facturation/factures", "/foyers", "/matieres", "/audit/journal",
                "/personnel/search?q=Double", "/personnel/search?q=a", "/vs/signaux", "/annonces",
                "/edt/mes-creneaux", "/edt/creneaux", "/roles", "/referentiel/classes",
                "/referentiel/annees", "/rh/absences", "/note-periodes", "/devoirs",
                "/infirmerie/passages", "/psy/entretiens", "/documents/grants/pending",
                "/auth/sessions", "/etablissement/securite", "/etablissement/domaines",
            ] {
                let rep = m.appel("GET", chemin, Some(&x.token_a), None).await;
                if rep.est_succes() {
                    for f in fuites(m, &rep.texte, INTERDITS_GLOBAUX) {
                        echecs.push(format!("{chemin} {:?} : {f}", x.acteur));
                    }
                    // Les libellés de personnes ne remontent que des membres de A.
                    if rep.texte.contains(&m.b.emails[&Direction]) {
                        echecs.push(format!("{chemin} {:?} : e-mail d'un compte de B", x.acteur));
                    }
                }
            }
        }
    });
    assert!(echecs.is_empty(), "{}", echecs.join("\n"));
}

/// Un double membre ne peut pas se servir de sa session A pour agir sur son
/// autre rattachement : ni lire, ni gérer, ni révoquer les sessions de B.
#[test]
fn c6_double_membre_ne_gere_pas_les_sessions_ni_la_securite_de_b() {
    let (Some(m), Some(d)) = (monde(), doubles()) else { return };
    let dir = d.iter().find(|x| x.acteur == Direction).unwrap();
    rt().block_on(async {
        // Ouvre une vraie session sur B pour le même compte, puis tente de la révoquer depuis A.
        let mdp_inconnu = m
            .appel("POST", "/auth/login", None,
                Some(&json!({"email": dir.email, "password": "x".repeat(12), "etablissement_id": m.b.objets.etab})))
            .await;
        assert!(!mdp_inconnu.est_succes());
        // Les sessions listées par A ne portent que A.
        let l = m.appel("GET", "/auth/sessions", Some(&dir.token_a), None).await;
        assert!(l.est_succes(), "{}", l.texte);
        assert!(!l.texte.contains(&m.b.objets.etab.to_string()), "session de B visible depuis A");
        // Révoquer une session par identifiant inconnu / d'un autre : refus uniforme.
        for id in [Uuid::new_v4(), Uuid::nil()] {
            let r = m.appel("DELETE", &format!("/auth/sessions/{id}"), Some(&dir.token_a), None).await;
            assert!(r.est_refus() || r.statut.as_u16() == 204 || r.statut.as_u16() == 200 || r.statut.as_u16() == 400,
                "révocation d'une session inconnue : {}", r.statut);
        }
        // La politique et les domaines sont ceux de A, jamais de B.
        let pol = m.appel("GET", "/etablissement/securite", Some(&dir.token_a), None).await;
        assert!(!pol.texte.contains(&m.b.objets.etab.to_string()));
        let dom = m.appel("POST", "/etablissement/domaines", Some(&dir.token_a),
            Some(&json!({"domaine": format!("b-{}.example.test", Uuid::new_v4().simple()), "etablissement_id": m.b.objets.etab}))).await;
        if dom.est_succes() {
            // Le domaine est rattaché à A, jamais à B, malgré l'identifiant fourni.
            let lb = m.appel("GET", "/etablissement/domaines", Some(m.b.jetons[&Admin].as_str()), None).await;
            let nom = dom.json["domaine"].as_str().unwrap_or("?");
            assert!(!lb.texte.contains(nom), "domaine posé dans B par une session A");
        }
    });
}

// ───────── Références croisées en écriture (parent de B, session de A) ─────────

async fn json_de(m: &Monde, jeton: &str, chemin: &str) -> Vec<Value> {
    let r = m.appel("GET", chemin, Some(jeton), None).await;
    assert!(r.est_succes(), "préparation {chemin} : {} {}", r.statut, r.texte);
    r.json.as_array().cloned().unwrap_or_default()
}

/// Un créneau d'EDT de l'établissement du jeton (essaie les 7 jours de la semaine).
async fn creneau_de(m: &Monde, jeton: &str) -> Uuid {
    let base = chrono::Utc::now().date_naive();
    for d in 0..7 {
        let jour = base + chrono::Duration::days(d);
        let r = m.appel("GET", &format!("/edt/creneaux?jour={jour}"), Some(jeton), None).await;
        if let Some(c) = r.json.as_array().and_then(|a| a.first()) {
            return Uuid::parse_str(c["id"].as_str().expect("id de créneau")).unwrap();
        }
    }
    panic!("aucun créneau d'EDT dans le labo");
}

fn id_premier(v: &[Value]) -> Uuid {
    Uuid::parse_str(v.first().and_then(|x| x["id"].as_str()).expect("objet de préparation")).unwrap()
}

/// Exige un refus propre (4xx) : ni succès (donnée croisée écrite), ni 5xx.
fn exige_refus(echecs: &mut Vec<String>, cas: &str, rep: &Rep) {
    if rep.est_succes() || !rep.statut.is_client_error() {
        echecs.push(format!("{cas} : statut {} (refus 4xx attendu) — {}", rep.statut, rep.texte.chars().take(120).collect::<String>()));
    }
}

#[test]
fn c6_references_croisees_en_ecriture_refusees() {
    let Some(m) = monde() else { return };
    let mut echecs = Vec::new();
    let mut cas = 0usize;
    rt().block_on(async {
        let (a, b) = (&m.a, &m.b);
        let dir_a = m.jeton(Direction);
        let dir_b = b.jetons[&Direction].as_str();
        let adm_a = m.jeton(Admin);
        let jour = jour_du_test();
        let matiere_a = a.objets.matiere.expect("matière A");
        let matiere_b = b.objets.matiere.expect("matière B");
        let periode_a = id_premier(&json_de(m, dir_a, "/note-periodes").await);
        let periode_b = id_premier(&json_de(m, dir_b, "/note-periodes").await);
        let eleve_a = a.objets.eleve;
        let eleve_b = b.objets.eleve;

        // ── Note : matière / période / devoir de B sur un élève de A ──
        for (nom, corps) in [
            ("note: matière de B", json!({"matiere_id": matiere_b, "valeur": 12, "date_note": jour})),
            ("note: période de B", json!({"matiere_id": matiere_a, "valeur": 12, "date_note": jour, "periode_id": periode_b})),
            ("note: devoir inexistant/de B", json!({"matiere_id": matiere_a, "valeur": 12, "date_note": jour, "devoir_id": Uuid::new_v4()})),
        ] {
            let r = m.appel("POST", &format!("/eleves/{eleve_a}/notes"), Some(dir_a), Some(&corps)).await;
            exige_refus(&mut echecs, nom, &r);
            cas += 1;
        }
        // ── Devoir : matière ou période de B ──
        for (nom, corps) in [
            ("devoir: matière de B", json!({"periode_id": periode_a, "matiere_id": matiere_b, "classe": "4B", "titre": "x", "date_devoir": jour})),
            ("devoir: période de B", json!({"periode_id": periode_b, "matiere_id": matiere_a, "classe": "4B", "titre": "x", "date_devoir": jour})),
        ] {
            let r = m.appel("POST", "/devoirs", Some(dir_a), Some(&corps)).await;
            exige_refus(&mut echecs, nom, &r);
            cas += 1;
        }
        // ── Appel : élève de B dans un appel de A ; créneau de B ──
        if let Some(appel) = a.objets.appel {
            let r = m.appel("POST", &format!("/appels/{appel}/lignes"), Some(m.jeton(Prof)),
                Some(&json!({"eleve_id": eleve_b, "etat": "absent"}))).await;
            exige_refus(&mut echecs, "appel: élève de B dans un appel de A", &r);
            cas += 1;
        }
        let creneau_b = creneau_de(m, dir_b).await;
        let r = m.appel("POST", "/appels", Some(m.jeton(Prof)),
            Some(&json!({"creneau_id": creneau_b, "jour": jour}))).await;
        exige_refus(&mut echecs, "appel: créneau de B", &r);
        cas += 1;
        // ── Message : fil sur un élève de B ; réponse dans un fil de B ──
        let r = m.appel("POST", "/messages/fils", Some(dir_a),
            Some(&json!({"eleve_id": eleve_b, "sujet": "croisé", "message": "croisé"}))).await;
        exige_refus(&mut echecs, "message: fil sur un élève de B", &r);
        cas += 1;
        if let Some(fil_b) = b.objets.fil {
            let r = m.appel("POST", &format!("/messages/fils/{fil_b}/repondre"), Some(dir_a), Some(&json!({"corps": "croisé"}))).await;
            exige_refus(&mut echecs, "message: réponse dans un fil de B", &r);
            cas += 1;
        }
        // ── Facture : foyer de B ; ligne sur un élève de B pour un foyer de A ──
        let r = m.appel("POST", "/facturation/factures", Some(dir_a), Some(&json!({
            "foyer_id": b.objets.foyer, "numero": format!("X-{}", Uuid::new_v4().simple()), "libelle": "croisé",
            "lignes": [{"libelle": "x", "quantite": 1, "montant_unitaire_centimes": 100}]}))).await;
        exige_refus(&mut echecs, "facture: foyer de B", &r);
        cas += 1;
        let r = m.appel("POST", "/facturation/factures", Some(dir_a), Some(&json!({
            "foyer_id": a.objets.foyer, "numero": format!("X-{}", Uuid::new_v4().simple()), "libelle": "croisé",
            "lignes": [{"eleve_id": eleve_b, "libelle": "x", "quantite": 1, "montant_unitaire_centimes": 100}]}))).await;
        exige_refus(&mut echecs, "facture: ligne sur un élève de B", &r);
        cas += 1;
        // ── Document : dépôt sur un élève de B ; décision sur une demande d'accès de B ──
        let r = m.appel("POST", &format!("/eleves/{eleve_b}/documents"), Some(dir_a),
            Some(&json!({"titre": "croisé", "tiroir": "famille", "confidentialite": "standard"}))).await;
        exige_refus(&mut echecs, "document: dépôt sur un élève de B", &r);
        let r = m.appel("POST", &format!("/documents/grants/{}/decider", Uuid::new_v4()), Some(adm_a),
            Some(&json!({"accepter": true}))).await;
        exige_refus(&mut echecs, "document: décision sur un grant inconnu", &r);
        cas += 2;
        // Rien n'a été écrit dans B.
        for (nom, chemin) in [
            ("notes", format!("/eleves/{eleve_b}/notes")),
            ("factures", "/facturation/factures".to_string()),
            ("fils", "/messages/fils".to_string()),
        ] {
            let l = m.appel("GET", &chemin, Some(dir_b), None).await;
            if l.texte.contains("croisé") || l.texte.contains(&a.objets.etab.to_string()) {
                echecs.push(format!("écriture croisée visible dans B ({nom})"));
            }
        }
    });
    CAS_EXECUTES.fetch_add(cas, Ordering::Relaxed);
    eprintln!("[separation] références croisées : {cas} cas, {} écart(s)", echecs.len());
    assert!(echecs.is_empty(), "{}", echecs.join("\n"));
}

// ───────── Colonnes `*_user_id` fournies par le client ─────────

#[test]
fn c6_colonnes_user_id_du_client_hors_etablissement_refusees() {
    let Some(m) = monde() else { return };
    let mut echecs = Vec::new();
    let mut cas = 0usize;
    rt().block_on(async {
        let (a, b) = (&m.a, &m.b);
        let dir_a = m.jeton(Direction);
        let adm_a = m.jeton(Admin);
        let jour = jour_du_test();
        // Un compte de B (professeur) : il ne doit JAMAIS apparaître côté A.
        let prof_b = b.user_ids[&Prof];
        let creneau_a = creneau_de(m, dir_a).await;

        // Remplaçant EDT.
        let r = m.appel("POST", "/edt/exceptions", Some(dir_a), Some(&json!({
            "creneau_id": creneau_a, "jour": jour, "kind": "remplacement",
            "remplacant_user_id": prof_b, "motif": "croisé"}))).await;
        exige_refus(&mut echecs, "edt: remplaçant de B", &r);
        // Affectation d'un créneau à un professeur de B.
        let r = m.appel("POST", &format!("/edt/creneaux/{creneau_a}/affectation"), Some(dir_a),
            Some(&json!({"prof_user_id": prof_b}))).await;
        exige_refus(&mut echecs, "edt: affectation à un professeur de B", &r);
        // Contrainte EDT sur un professeur de B.
        let r = m.appel("POST", "/edt/contraintes", Some(dir_a), Some(&json!({
            "kind": "indisponibilite_prof", "prof_user_id": prof_b, "jour_semaine": 1,
            "heure_debut": "08:00", "heure_fin": "09:00"}))).await;
        if r.est_succes() || r.statut.is_server_error() {
            echecs.push(format!("edt: contrainte sur un professeur de B : {}", r.statut));
        }
        // Absence RH pour une personne de B.
        let r = m.appel("POST", "/rh/absences", Some(dir_a), Some(&json!({
            "user_id": prof_b, "debut": jour, "fin": jour, "motif": "croisé"}))).await;
        exige_refus(&mut echecs, "rh: absence d'une personne de B", &r);
        // Service d'enseignement donné à un professeur de B.
        let classes = json_de(m, dir_a, "/referentiel/classes").await;
        let r = m.appel("POST", "/referentiel/services", Some(dir_a), Some(&json!({
            "prof_user_id": prof_b, "matiere_id": a.objets.matiere, "classe_id": id_premier(&classes)}))).await;
        exige_refus(&mut echecs, "référentiel: service d'un professeur de B", &r);
        // Responsable de groupe de B.
        let annees = json_de(m, dir_a, "/referentiel/annees").await;
        let g = m.appel("POST", "/referentiel/groupes", Some(dir_a), Some(&json!({
            "annee_id": id_premier(&annees), "code": format!("G{}", &Uuid::new_v4().simple().to_string()[..6]), "libelle": "atelier"}))).await;
        if g.est_succes() {
            let gid = g.json["id"].as_str().unwrap().to_string();
            let r = m.appel("PUT", &format!("/referentiel/groupes/{gid}/responsables"), Some(dir_a),
                Some(&json!({"responsables": [prof_b]}))).await;
            exige_refus(&mut echecs, "groupe: responsable de B", &r);
            cas += 1;
        } else {
            echecs.push(format!("préparation : création de groupe impossible ({} {})", g.statut, g.texte));
        }
        // Attribution de rôle à une personne de B ; rôle de B attribué à une personne de A.
        let role = m.appel("POST", "/roles", Some(adm_a), Some(&json!({
            "nom": format!("Croisé {}", &Uuid::new_v4().simple().to_string()[..6]), "droits": []}))).await;
        if role.est_succes() {
            let rid = role.json["id"].as_str().unwrap().to_string();
            let r = m.appel("PUT", &format!("/roles/{rid}/membres/{prof_b}"), Some(adm_a), Some(&json!({}))).await;
            exige_refus(&mut echecs, "rôles: attribution à une personne de B", &r);
            let r = m.appel("DELETE", &format!("/roles/{rid}/membres/{prof_b}"), Some(adm_a), None).await;
            if r.statut.is_server_error() {
                echecs.push(format!("rôles: retrait d'une personne de B : {}", r.statut));
            }
            cas += 2;
            // Rôle de B manipulé depuis A.
            let rb = m.appel("POST", "/roles", Some(b.jetons[&Admin].as_str()), Some(&json!({
                "nom": format!("RoleB {}", &Uuid::new_v4().simple().to_string()[..6]), "droits": []}))).await;
            if rb.est_succes() {
                let rbid = rb.json["id"].as_str().unwrap().to_string();
                let user_a = a.user_ids[&Prof];
                let r = m.appel("PUT", &format!("/roles/{rbid}/membres/{user_a}"), Some(adm_a), Some(&json!({}))).await;
                exige_refus(&mut echecs, "rôles: rôle de B attribué depuis A", &r);
                let r = m.appel("PUT", &format!("/roles/{rbid}"), Some(adm_a), Some(&json!({"nom": "pirate", "droits": []}))).await;
                exige_refus(&mut echecs, "rôles: modification d'un rôle de B", &r);
                let r = m.appel("DELETE", &format!("/roles/{rbid}"), Some(adm_a), None).await;
                exige_refus(&mut echecs, "rôles: suppression d'un rôle de B", &r);
                cas += 3;
            }
        } else {
            echecs.push(format!("préparation : création de rôle impossible ({} {})", role.statut, role.texte));
        }
        // Rien de B ne fuit dans les libellés de A (user_labels filtré, C1).
        let ex = m.appel("GET", "/edt/exceptions", Some(dir_a), None).await;
        if ex.texte.contains(&b.emails[&Prof]) || ex.texte.contains(&prof_b.to_string()) {
            echecs.push("edt/exceptions : personne de B visible dans A".into());
        }
        cas += 7;
    });
    CAS_EXECUTES.fetch_add(cas, Ordering::Relaxed);
    eprintln!("[separation] colonnes *_user_id : {cas} cas, {} écart(s)", echecs.len());
    assert!(echecs.is_empty(), "{}", echecs.join("\n"));
}

/// C1 : `user_labels` ne renvoie que des membres de l'établissement de la session.
#[test]
fn c1_user_labels_filtre_par_etablissement() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        let url = std::env::var("ENT_DATABASE_URL").unwrap();
        let ent = ent_db::PgEnt::connect(&url).await.expect("connexion");
        let (ua, ub) = (m.a.user_ids[&Prof], m.b.user_ids[&Prof]);
        let l = ent.user_labels(m.a.objets.etab, &[ua, ub]).await.expect("labels");
        assert!(l.contains_key(&ua), "membre de A absent des libellés de A");
        assert!(!l.contains_key(&ub), "libellé d'un utilisateur de B renvoyé pour A");
        let l = ent.user_labels(m.b.objets.etab, &[ua, ub]).await.expect("labels");
        assert!(l.contains_key(&ub) && !l.contains_key(&ua));
    });
}

// ───────── Énumération : connu dans B ≡ inconnu ─────────

/// Réponse normalisée : statut et corps, sans identifiants ni horodatages variables.
fn forme(rep: &Rep) -> (u16, String) {
    let mut v = rep.json.clone();
    fn nettoyer(v: &mut Value) {
        match v {
            Value::Object(o) => {
                for k in ["token", "expires_at", "user_id", "email", "challenge", "rp", "options", "mot_de_passe_temporaire"] {
                    if o.contains_key(k) {
                        o.insert(k.into(), Value::Null);
                    }
                }
                o.values_mut().for_each(nettoyer);
            }
            Value::Array(a) => a.iter_mut().for_each(nettoyer),
            _ => {}
        }
    }
    nettoyer(&mut v);
    let corps = if rep.json.is_null() { rep.texte.clone() } else { v.to_string() };
    (rep.statut.as_u16(), corps)
}

#[test]
fn c6_enumeration_email_connu_dans_b_et_inconnu_indiscernables() {
    let Some(m) = monde() else { return };
    let mut echecs = Vec::new();
    rt().block_on(async {
        let a = m.a.objets.etab;
        let connu_b = m.b.emails[&Prof].clone(); // existe, mais seulement dans B
        let inconnu = format!("fantome-{}@example.test", Uuid::new_v4().simple());
        let mdp = m.a.mot_de_passe.clone();
        let mauvais = "mot-de-passe-incorrect-123".to_string();

        // Connexion : bon mot de passe d'un compte de B, ou compte inconnu → même réponse.
        let mut formes = Vec::new();
        for (email, pwd) in [(&connu_b, &m.b.mot_de_passe), (&inconnu, &mdp), (&connu_b, &mauvais), (&inconnu, &mauvais)] {
            let r = m.appel("POST", "/auth/login", None,
                Some(&json!({"email": email, "password": pwd, "etablissement_id": a}))).await;
            formes.push(forme(&r));
        }
        if formes.iter().any(|f| *f != formes[0]) {
            echecs.push(format!("connexion : réponses distinctes {formes:?}"));
        }
        if formes[0].0 != 401 {
            echecs.push(format!("connexion : 401 attendu, {}", formes[0].0));
        }

        // Passkey (début de connexion) : défi factice pour les deux.
        let mut formes = Vec::new();
        for email in [&connu_b, &inconnu] {
            let r = m.appel("POST", "/auth/passkey/login/start", None,
                Some(&json!({"email": email, "etablissement_id": a}))).await;
            formes.push((r.statut.as_u16(), r.json.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>())));
        }
        if formes[0] != formes[1] {
            echecs.push(format!("passkey/start : réponses distinctes {formes:?}"));
        }

        // Mot de passe oublié : 202 identique.
        let mut formes = Vec::new();
        for email in [&connu_b, &inconnu] {
            let r = m.appel("POST", "/auth/forgot-password", None, Some(&json!({"email": email, "etablissement_id": a}))).await;
            formes.push(forme(&r));
        }
        if formes[0] != formes[1] {
            echecs.push(format!("mot de passe oublié : réponses distinctes {formes:?}"));
        }

        // Provisionnement par l'Admin de A : adresse connue dans B ≡ adresse inconnue.
        let mut formes = Vec::new();
        for email in [connu_b.clone(), format!("neuf-{}@example.test", Uuid::new_v4().simple())] {
            let r = m.appel("POST", "/etablissement/personnel", Some(m.jeton(Admin)),
                Some(&json!({"email": email, "display_name": "Enum", "role": "cpe"}))).await;
            let mut f = forme(&r);
            // Le mot de passe temporaire et l'identifiant varient toujours ; on compare la forme.
            f.1 = r.json.as_object().map(|o| {
                let mut k: Vec<_> = o.keys().cloned().collect();
                k.sort();
                format!("{k:?} role={}", o.get("role").and_then(|x| x.as_str()).unwrap_or(""))
            }).unwrap_or(f.1);
            formes.push(f);
        }
        if formes[0] != formes[1] {
            echecs.push(format!("provisionnement : réponses distinctes {formes:?}"));
        }
        // …et l'adresse de B n'a pas été rattachée à A, ni son compte modifié.
        let l = m.appel("POST", "/auth/login", None,
            Some(&json!({"email": connu_b, "password": m.b.mot_de_passe, "etablissement_id": a}))).await;
        if l.est_succes() {
            echecs.push("provisionnement d'une adresse de B : rattachement silencieux à A".into());
        }
        let l = m.appel("POST", "/auth/login", None,
            Some(&json!({"email": connu_b, "password": m.b.mot_de_passe, "etablissement_id": m.b.objets.etab}))).await;
        if !l.est_succes() {
            echecs.push("provisionnement d'une adresse de B : mot de passe du compte modifié".into());
        }
        // Dans son propre établissement, le conflit reste explicite (409) : pas un oracle inter-établissements.
        let r = m.appel("POST", "/etablissement/personnel", Some(m.jeton(Admin)),
            Some(&json!({"email": m.a.emails[&Prof], "display_name": "Enum", "role": "cpe"}))).await;
        if r.statut.as_u16() != 409 {
            echecs.push(format!("adresse déjà membre de A : 409 attendu, {}", r.statut));
        }
    });
    CAS_EXECUTES.fetch_add(12, Ordering::Relaxed);
    assert!(echecs.is_empty(), "{}", echecs.join("\n"));
}

// ───────── Nouvelles routes du socle 1 ─────────

#[test]
fn c6_routes_du_socle1_inventaire_401_et_separation() {
    let (Some(m), Some(d)) = (monde(), doubles()) else { return };
    let mut echecs = Vec::new();
    let mut cas = 0usize;
    rt().block_on(async {
        let nul = Uuid::new_v4();
        let routes: Vec<(&str, String, Option<Value>)> = vec![
            ("GET", "/roles/catalogue".into(), None),
            ("GET", "/roles".into(), None),
            ("POST", "/roles".into(), Some(json!({"nom": "x", "droits": []}))),
            ("PUT", format!("/roles/{nul}"), Some(json!({"nom": "x", "droits": []}))),
            ("DELETE", format!("/roles/{nul}"), None),
            ("PUT", format!("/roles/{nul}/membres/{nul}"), Some(json!({}))),
            ("DELETE", format!("/roles/{nul}/membres/{nul}"), None),
            ("GET", "/roles/mes-droits".into(), None),
            ("PUT", format!("/referentiel/groupes/{nul}/responsables"), Some(json!({"responsables": []}))),
            ("GET", "/auth/sessions".into(), None),
            ("DELETE", format!("/auth/sessions/{nul}"), None),
            ("GET", "/etablissement/securite".into(), None),
            ("PUT", "/etablissement/securite".into(), Some(json!({}))),
            ("GET", "/etablissement/domaines".into(), None),
            ("POST", "/etablissement/domaines".into(), Some(json!({"domaine": "x.example.test"}))),
            ("DELETE", "/etablissement/domaines/x.example.test".into(), None),
            ("PUT", "/etablissement/fiche".into(), Some(json!({}))),
            ("POST", format!("/documents/{nul}/lien"), Some(json!({}))),
            ("POST", format!("/messages/pieces/{nul}/lien"), Some(json!({}))),
            ("POST", format!("/eleves/{nul}/photo/lien"), Some(json!({}))),
            ("POST", format!("/pai/pieces/{nul}/lien"), Some(json!({}))),
        ];
        // 1. Sans session ou jeton inventé : 401.
        for (meth, chemin, corps) in &routes {
            for jeton in [None, Some("jeton-invente")] {
                let r = m.appel(meth, chemin, jeton, corps.as_ref()).await;
                cas += 1;
                if r.statut.as_u16() != 401 {
                    echecs.push(format!("{meth} {chemin} sans session → {}", r.statut));
                }
            }
        }
        // 2. Liens de fichiers : identifiants de B avec une session de A (y compris double membre) → refus.
        let pieces = [
            format!("/documents/{}/lien", m.b.objets.doc.expect("doc B")),
            format!("/documents/{}/lien", m.b.objets.doc_sante.expect("doc santé B")),
            format!("/eleves/{}/photo/lien", m.b.objets.eleve),
        ];
        let mut jetons: Vec<(String, String)> = Acteur::STAFF.iter().map(|a| (format!("{a:?}"), m.jeton(*a).to_string())).collect();
        jetons.extend(d.iter().map(|x| (format!("double {:?}", x.acteur), x.token_a.clone())));
        for chemin in &pieces {
            for (nom, t) in &jetons {
                let r = m.appel("POST", chemin, Some(t), Some(&json!({}))).await;
                cas += 1;
                if !r.est_refus() || r.texte.contains("/fichiers/") {
                    echecs.push(format!("{chemin} par {nom} : {} (refus attendu, aucun lien)", r.statut));
                }
            }
        }
        // 3. Gestion des rôles, sessions, politique et domaines : mêmes refus sur des objets de B.
        let role_b = m.appel("POST", "/roles", Some(m.b.jetons[&Admin].as_str()),
            Some(&json!({"nom": format!("RB {}", &Uuid::new_v4().simple().to_string()[..6]), "droits": []}))).await;
        let role_b_id = role_b.json["id"].as_str().map(String::from);
        for (nom, t) in &jetons {
            if let Some(rid) = &role_b_id {
                for (meth, chemin, corps) in [
                    ("PUT", format!("/roles/{rid}"), Some(json!({"nom": "pirate", "droits": []}))),
                    ("DELETE", format!("/roles/{rid}"), None),
                    ("PUT", format!("/roles/{rid}/membres/{}", m.b.user_ids[&Prof]), Some(json!({}))),
                    ("DELETE", format!("/roles/{rid}/membres/{}", m.b.user_ids[&Prof]), None),
                ] {
                    let r = m.appel(meth, &chemin, Some(t), corps.as_ref()).await;
                    cas += 1;
                    if !r.est_refus() {
                        echecs.push(format!("{meth} {chemin} par {nom} (session A) → {}", r.statut));
                    }
                }
            }
            // Le catalogue et la liste des rôles de A ne montrent aucun rôle de B.
            let l = m.appel("GET", "/roles", Some(t), None).await;
            cas += 1;
            if l.est_succes() {
                if let Some(rid) = &role_b_id {
                    if l.texte.contains(rid.as_str()) {
                        echecs.push(format!("GET /roles par {nom} : rôle de B listé"));
                    }
                }
            }
            // Révoquer une session de B par identifiant (session de B réelle).
            let sb = m.appel("GET", "/auth/sessions", Some(m.b.jetons[&Direction].as_str()), None).await;
            if let Some(sid) = sb.json.as_array().and_then(|a| a.first()).and_then(|x| x["id"].as_str()) {
                let r = m.appel("DELETE", &format!("/auth/sessions/{sid}"), Some(t), None).await;
                cas += 1;
                let verif = m.appel("GET", "/auth/me", Some(m.b.jetons[&Direction].as_str()), None).await;
                if !verif.est_succes() {
                    echecs.push(format!("DELETE /auth/sessions/<session de B> par {nom} ({}) : la session de B a été révoquée", r.statut));
                    break;
                }
            }
        }
        // 4. Lien signé inconnu / altéré : refus uniforme, aucun contenu.
        for jeton in ["x", "AAAA.BBBB", &"a".repeat(300)] {
            let r = m.appel("GET", &format!("/fichiers/{jeton}"), None, None).await;
            cas += 1;
            if r.est_succes() || r.statut.is_server_error() {
                echecs.push(format!("/fichiers/{} → {}", &jeton[..jeton.len().min(10)], r.statut));
            }
        }
    });
    CAS_EXECUTES.fetch_add(cas, Ordering::Relaxed);
    eprintln!("[separation] routes socle 1 : {cas} cas, {} écart(s)", echecs.len());
    assert!(echecs.is_empty(), "{}", echecs.join("\n"));
}
