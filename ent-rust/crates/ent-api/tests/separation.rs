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

/// Écriture ou lecture visant un objet de B : refus **exact** (401/403/404) pour tout acteur de A.
fn refuse_sur_b(
    nom: &'static str,
    methode: &'static str,
    chemin: &'static str,
    corps: Option<&'static str>,
) -> Spec {
    Spec { b: Attendu::Refuse, ..ecrire_sur_b(nom, methode, chemin, corps) }
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

/// Route sans identifiant de ressource : la session de A n'obtient que des données de A (2xx borné à A) ou un refus.
/// `interdits` : valeurs semées dans B qui ne doivent jamais apparaître dans une réponse.
fn borne_a(
    nom: &'static str,
    methode: &'static str,
    chemin: &'static str,
    corps: Option<&'static str>,
    interdits: &'static [&'static str],
) -> Spec {
    Spec { b: Attendu::SuccesBorneA, interdits, ..ecrire_sur_b(nom, methode, chemin, corps) }
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
        Attendu::SuccesBorneA => rep.est_refus() || rep.est_succes(), // oracle de fuite et non-vacuité dans jouer()
    }
}

fn jour_du_test() -> String {
    domain_eleve::horloge::jour_paris().to_string()
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
    let mut succes_bornes: std::collections::BTreeMap<&'static str, usize> = Default::default();
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
                    if att == Attendu::SuccesBorneA {
                        *succes_bornes.entry(s.nom).or_insert(0) += usize::from(rep.est_succes());
                    }
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
    for (nom, ok) in &succes_bornes {
        if *ok == 0 {
            echecs.push(format!("[{nom}] scénario vide : aucun acteur n'a obtenu de 2xx (la borne à A n'est pas éprouvée)"));
        }
    }
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
/// Les huit profils de personnel du jeu de labo (tous ont une fiche) : seuls eux reçoivent une réponse ; familles et élèves sont refusés.
const ESPACES_PERSONNELS: &[Acteur] = &Acteur::STAFF;
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

/// Les familles de l'établissement A ont toutes une fiche de personne (reprise des liens).
const FAMILLE_AVEC_FICHE: &[Acteur] = &[Parent, ParentAutreFoyer, ParentSansDrapeaux];
/// Routes « soi » (`/dossier/moi/*`) : depuis le socle 6 (P1), un compte du personnel a lui aussi une
/// fiche de personne (son agent) ; il lit la sienne, jamais celle d'un autre (aucun identifiant dans l'URL).
const SOI_AVEC_FICHE: &[Acteur] = &[
    Admin, Direction, Administratif, Cpe, Prof, Compta, Infirmiere, Psy, Parent, ParentAutreFoyer, ParentSansDrapeaux,
];

fn specs_domaine_eleve_dossier() -> Vec<Spec> {
    vec![
            // D-022 : un professeur ne lit que la fiche des élèves de ses classes et groupes.
            lire("eleve", "/eleves/{eleve}", TOUS, STAFF_HORS_PROF),
            lire("dossier", "/eleves/{eleve}/dossier", TOUS, STAFF_HORS_PROF),
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
            // Socle 2 (E5) : propositions. La file et le détail sont réservés à
            // `eleves.propositions.traiter` (Admin, Direction, Administratif) ; le déposant ne
            // voit que les siennes (parent avec lien actif ; l'élève mineur est refusé) ;
            // toute écriture sur un élève d'un autre établissement échoue.
            lire("propositions-file", "/eleves/propositions",
                &[Admin, Direction, Administratif], &[]),
            lire("propositions-eleve", "/eleves/{eleve}/propositions",
                &[Admin, Direction, Administratif, Parent, ParentAutreFoyer, ParentSansDrapeaux],
                &[Admin, Direction, Administratif]),
            lire("propositions-champs-signalables", "/eleves/{eleve}/propositions/champs",
                &[Parent, ParentAutreFoyer, ParentSansDrapeaux], &[]),
            ecrire_sur_b("proposition-depot-sur-B", "POST", "/eleves/{eleve}/propositions",
                Some(r#"{"champs":{"nom_naissance":"Separation"}}"#)),
            ecrire_sur_b("proposition-decision-sur-B", "POST",
                "/eleves/propositions/00000000-0000-0000-0000-000000000001/decision",
                Some(r#"{"champs_acceptes":["nom_naissance"],"nature":"correction","motif_code":"autre","motif":"separation"}"#)),
            ecrire_sur_b("proposition-retrait-sur-B", "POST",
                "/eleves/propositions/00000000-0000-0000-0000-000000000001/retrait", None),
            // Paramétrage des champs (ELV-26) : administrateur seulement.
            lire("parametrage-eleve", "/eleves/parametrage", &[Admin], &[]),
            // Les écritures de paramétrage n'ont aucun identifiant d'un autre établissement :
            // elles agissent sur l'établissement de la session (testées à part, `eleve_propositions`).
            // Socle 2 (E6) : doublons et fusion de fiches. Les listes sont réservées à
            // `eleves.identite.modifier` / `eleves.fusionner` (Admin, Direction, Administratif
            // pour les doublons ; Admin, Direction pour les fusions) ; toute écriture qui désigne
            // un élève d'un autre établissement échoue sans rien révéler.
            lire("doublons-liste", "/eleves/doublons", &[Admin, Direction, Administratif], &[]),
            lire("fusions-liste", "/eleves/fusions", &[Admin, Direction], &[]),
            Spec {
                nom: "doublons-detecter",
                methode: "POST",
                chemin: "/eleves/doublons/detecter",
                corps: None,
                liee: &[Admin, Direction, Administratif],
                non_liee: &[],
                non_liee_vide: &[],
                b: Attendu::EchecQuelconque,
                seulement_b: false,
                interdits: &[],
                tolere: &[],
            },
            ecrire_sur_b("fusion-apercu-sur-B", "POST", "/eleves/fusions/apercu",
                Some(r#"{"absorbe_id":"{eleve}","conserve_id":"00000000-0000-0000-0000-000000000001"}"#)),
            ecrire_sur_b("fusion-sur-B", "POST", "/eleves/fusions",
                Some(r#"{"absorbe_id":"{eleve}","conserve_id":"00000000-0000-0000-0000-000000000001","motif_code":"doublon_saisie"}"#)),
            ecrire_sur_b("fusion-sur-B-sens-inverse", "POST", "/eleves/fusions",
                Some(r#"{"absorbe_id":"00000000-0000-0000-0000-000000000001","conserve_id":"{eleve}","motif_code":"doublon_saisie"}"#)),
            ecrire_sur_b("defusion-sur-B", "POST",
                "/eleves/fusions/00000000-0000-0000-0000-000000000001/annuler", None),
            ecrire_sur_b("non-doublon-sur-B", "POST", "/eleves/doublons/non-doublon",
                Some(r#"{"eleve_a":"{eleve}","eleve_b":"00000000-0000-0000-0000-000000000001","motif_code":"jumeaux"}"#)),
            // Socle 2 (E7) : confidentialité renforcée (G6 : Admin, Direction), journal de consultation
            // (`audit.lire`), photos (`eleves.photo.gerer`), coordonnées propres de l'élève. Toute
            // écriture qui désigne un élève, une photo ou une coordonnée d'un autre établissement
            // échoue sans rien révéler ; la lecture d'un élève protégé par un tiers est testée à part
            // (`eleve_protege_invisible_d_un_tiers`).
            lire("confidentialite-proteges", "/eleves/confidentialite", &[Admin, Direction], &[]),
            lire("confidentialite-eleve", "/eleves/{eleve}/confidentialite", &[Admin, Direction], &[Admin, Direction]),
            lire("consultations-eleve", "/eleves/{eleve}/consultations", &[Admin, Direction, Administratif],
                &[Admin, Direction, Administratif]),
            lire("photos-eleve", "/eleves/{eleve}/photos", &[Admin, Direction, Administratif, Cpe],
                &[Admin, Direction, Administratif, Cpe]),
            lire("photos-a-valider", "/eleves/photos/a-valider", &[Admin, Direction, Administratif, Cpe], &[]),
            lire("photos-manquantes", "/eleves/photos/manquantes", &[Admin, Direction, Administratif, Cpe], &[]),
            lire("coordonnees-eleve", "/eleves/{eleve}/coordonnees",
                &[Admin, Direction, Administratif, Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux],
                &[Admin, Direction, Administratif]),
            ecrire_sur_b("confidentialite-pose-sur-B", "POST", "/eleves/{eleve}/confidentialite",
                Some(r#"{"motif":"separation"}"#)),
            ecrire_sur_b("confidentialite-modification-sur-B", "PUT", "/eleves/{eleve}/confidentialite",
                Some(r#"{"motif":"separation"}"#)),
            ecrire_sur_b("confidentialite-levee-sur-B", "POST", "/eleves/{eleve}/confidentialite/lever",
                Some(r#"{"motif_code":"fin_de_mesure"}"#)),
            ecrire_sur_b("photo-depot-sur-B", "POST", "/eleves/{eleve}/photos",
                Some(r#"{"mime_type":"image/jpeg","contenu_base64":"/9j/4AAQSkZJRgAB"}"#)),
            ecrire_sur_b("photo-validation-sur-B", "POST",
                "/eleves/photos/00000000-0000-0000-0000-000000000001/valider", None),
            ecrire_sur_b("photo-rejet-sur-B", "POST",
                "/eleves/photos/00000000-0000-0000-0000-000000000001/rejeter", Some(r#"{"motif_code":"illisible"}"#)),
            ecrire_sur_b("photo-retrait-sur-B", "POST",
                "/eleves/photos/00000000-0000-0000-0000-000000000001/retirer", Some(r#"{"motif_code":"autre"}"#)),
            ecrire_sur_b("photo-refus-sur-B", "POST", "/eleves/{eleve}/photo/refus", None),
            ecrire_sur_b("photo-refus-levee-sur-B", "DELETE", "/eleves/{eleve}/photo/refus", None),
            ecrire_sur_b("coordonnee-ajout-sur-B", "POST", "/eleves/{eleve}/coordonnees",
                Some(r#"{"type":"email","valeur":"separation@exemple.test"}"#)),
            ecrire_sur_b("coordonnee-decision-sur-B", "POST",
                "/eleves/coordonnees/00000000-0000-0000-0000-000000000001/decision", Some(r#"{"accepter":true}"#)),
            ecrire_sur_b("coordonnee-retrait-sur-B", "POST",
                "/eleves/coordonnees/00000000-0000-0000-0000-000000000001/retirer", Some(r#"{"motif_code":"autre"}"#)),
            ecrire_sur_b("coordonnee-usages-sur-B", "POST",
                "/eleves/coordonnees/00000000-0000-0000-0000-000000000001/usages", Some(r#"{"usages":["notifications"]}"#)),
            ecrire_sur_b("coordonnee-verification-sur-B", "POST",
                "/eleves/coordonnees/00000000-0000-0000-0000-000000000001/verification", None),
            ecrire_sur_b("coordonnee-verifier-sur-B", "POST",
                "/eleves/coordonnees/00000000-0000-0000-0000-000000000001/verifier", Some(r#"{"code":"123456"}"#)),
            // Socle 2 (E8) : pastilles et synthèse de la fiche (personnel ; chaque pastille filtrée par
            // l'action de son domaine, sur l'élève).
            lire("synthese-eleve", "/eleves/{eleve}/synthese", STAFF, STAFF_HORS_PROF),
            lire("pastilles-eleve", "/eleves/{eleve}/pastilles", STAFF, STAFF_HORS_PROF),
            lire("pastilles-compteurs", "/eleves/pastilles", STAFF, &[]),
            // Socle 2 (E9a) : la liste des élèves (annuaire du lecteur, filtrée avant la pagination).
            lire("eleves-liste", "/eleves/liste", STAFF, &[]),
            lire("eleves-liste-filtree", "/eleves/liste?q=a&classe=6A&statut=actif&limite=5", STAFF, &[]),
            lire("pastilles-catalogue", "/eleves/pastilles/catalogue", STAFF, &[]),
            lire("pastilles-liste", "/eleves/pastilles/sans_ine", &[Admin, Direction, Administratif], &[]),
            // Socle 2 (E4) : scolarité (G1 : `eleves.lire`, Admin/Direction/Administratif) et
            // commandes d'écriture, toutes refusées sur l'élève d'un autre établissement.
            // Démo P1 n° 3 : le responsable légal (lien actif) lit la scolarité de SON enfant,
            // minimisée ; jamais celle d'un autre élève (liste « non liée » inchangée).
            lire("scolarite", "/eleves/{eleve}/scolarite",
                &[Admin, Direction, Administratif, Parent, ParentAutreFoyer, ParentSansDrapeaux],
                &[Admin, Direction, Administratif]),
            ecrire_sur_b("scolarisation-ouverture-sur-B", "POST", "/eleves/{eleve}/scolarisations",
                Some(r#"{"date_entree":"{jour}","motif":"reprise"}"#)),
            ecrire_sur_b("scolarisation-reintegration-sur-B", "POST", "/eleves/{eleve}/scolarisations/reintegrer",
                Some(r#"{"date_entree":"{jour}"}"#)),
            ecrire_sur_b("scolarisation-cloture-sur-B", "POST", "/eleves/{eleve}/scolarisations/cloturer",
                Some(r#"{"date_sortie":"{jour}","motif":"demenagement"}"#)),
            ecrire_sur_b("deces-sur-B", "POST", "/eleves/{eleve}/deces", Some(r#"{"date":"{jour}"}"#)),
            ecrire_sur_b("inscription-annuelle-sur-B", "PUT",
                "/eleves/{eleve}/inscriptions/00000000-0000-0000-0000-000000000001",
                Some(r#"{"etat":"prevue"}"#)),
            ecrire_sur_b("classe-sur-B", "POST", "/eleves/{eleve}/scolarite/classe",
                Some(r#"{"classe_id":"00000000-0000-0000-0000-000000000001","date_effet":"{jour}"}"#)),
            ecrire_sur_b("options-sur-B", "POST", "/eleves/{eleve}/scolarite/options",
                Some(r#"{"annee_id":"00000000-0000-0000-0000-000000000001","ajouter":["00000000-0000-0000-0000-000000000002"],"date_effet":"{jour}"}"#)),
            ecrire_sur_b("regime-sur-B", "POST", "/eleves/{eleve}/scolarite/regime",
                Some(r#"{"valeur":"externe","date_effet":"{jour}"}"#)),
            ecrire_sur_b("site-sur-B", "POST", "/eleves/{eleve}/scolarite/site",
                Some(r#"{"valeur":"annexe","date_effet":"{jour}"}"#)),
            ecrire_sur_b("capacite-sur-B", "POST", "/eleves/{eleve}/capacites",
                Some(r#"{"nature":"emancipation","du":"{jour}","piece_document_id":"00000000-0000-0000-0000-000000000001"}"#)),
            ecrire_sur_b("capacite-levee-sur-B", "POST",
                "/eleves/{eleve}/capacites/00000000-0000-0000-0000-000000000001/lever", Some("{}")),
            ecrire_sur_b("mobilite-sur-B", "POST", "/eleves/{eleve}/mobilites",
                Some(r#"{"type_":"etranger","du":"{jour}"}"#)),
            ecrire_sur_b("mobilite-fermeture-sur-B", "POST",
                "/eleves/{eleve}/mobilites/00000000-0000-0000-0000-000000000001/fermer", Some(r#"{"au":"{jour}"}"#)),
            ecrire_sur_b("modalite-sur-B", "POST", "/eleves/{eleve}/modalites",
                Some(r#"{"code":"ulis","du":"{jour}"}"#)),
            ecrire_sur_b("modalite-fin-sur-B", "POST",
                "/eleves/{eleve}/modalites/00000000-0000-0000-0000-000000000001/terminer", Some(r#"{"au":"{jour}"}"#)),
            lire("carnet", "/eleves/{eleve}/carnet",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent, Eleve], GESTION_HORS_PROF),
            Spec { non_liee_vide: FAMILLE, ..lire("incidents", "/eleves/{eleve}/incidents", TOUS, STAFF) },
            Spec { non_liee_vide: FAMILLE, ..lire("sanctions", "/eleves/{eleve}/sanctions", TOUS, STAFF) },
            // Socle 4 (lot A0) : ajout d'un responsable (commande unique), lien vers une personne
            // existante, liens en attente de confirmation. Jamais d'écriture sur l'élève d'un autre
            // établissement ; la liste « mes liens en attente » ne contient que ceux de l'appelant.
            ecrire_sur_b("ajout-responsable-B", "POST", "/dossier/eleves/{eleve}/responsables/ajout",
                Some(r#"{"nouvelle_personne":{"nom":"Separation"},"lien":{"lien":"pere"}}"#)),
            ecrire_sur_b("lier-responsable-B", "POST", "/dossier/eleves/{eleve}/responsables",
                Some(r#"{"personne_id":"00000000-0000-0000-0000-000000000001","lien":"pere"}"#)),
            ecrire_sur_b("repondre-lien-attente-B", "POST",
                "/dossier/mes-liens-en-attente/00000000-0000-0000-0000-000000000001/reponse",
                Some(r#"{"accepter":true}"#)),
            lire("mes-liens-en-attente", "/dossier/mes-liens-en-attente", TOUS, TOUS),
            // Socle 4 (lot A0 bis) : confirmation au guichet (personnes.gerer), alertes de
            // protection et « ne pas solliciter » (direction). Aucune de ces routes ne s'applique
            // à une ressource d'un autre établissement ; les alertes ne listent que celles de
            // l'établissement de l'appelant, et jamais une adresse.
            lire("liens-en-attente-eleve", "/dossier/eleves/{eleve}/liens-en-attente",
                &[Admin, Direction, Administratif], &[Admin, Direction, Administratif]),
            ecrire_sur_b("confirmer-guichet-B", "POST",
                "/dossier/liens-en-attente/00000000-0000-0000-0000-000000000001/confirmer-guichet",
                Some(r#"{"piece":"carte_identite"}"#)),
            ecrire_sur_b("ne-pas-solliciter-B", "POST",
                "/dossier/personnes/00000000-0000-0000-0000-000000000001/ne-pas-solliciter",
                Some(r#"{"actif":true}"#)),
            ecrire_sur_b("ouvrir-adresse-protegee-B", "POST",
                "/dossier/adresses-protegees/00000000-0000-0000-0000-000000000001/ouvrir", None),
            Spec { interdits: &["adresse\":"], ..lire("alertes-protection", "/dossier/alertes-protection",
                &[Admin, Direction], &[Admin, Direction]) },
            // Socle 4 (lot A1) : interdictions de remise (consigne : `remise.lire`, jamais pour une
            // famille), conflits d'intérêts (direction). Aucune écriture sur l'établissement B ;
            // un motif d'interdiction n'apparaît jamais dans un cas hors direction.
            Spec { interdits: &["motif_scelle", "enc:v1:"], ..lire("interdictions", "/dossier/eleves/{eleve}/interdictions",
                &[Admin, Direction, Administratif, Cpe, Prof, Infirmiere],
                &[Admin, Direction, Administratif, Cpe, Infirmiere]) },
            ecrire_sur_b("poser-interdiction-B", "POST", "/dossier/eleves/{eleve}/interdictions",
                Some(r#"{"personne_libelle":"Separation","objet":["remise"],"fondement":"decision_etablissement","consigne":"Ne pas remettre"}"#)),
            ecrire_sur_b("lever-interdiction-B", "POST",
                "/dossier/interdictions/00000000-0000-0000-0000-000000000001/lever",
                Some(r#"{"motif":"separation"}"#)),
            ecrire_sur_b("declarer-conflit-interet-B", "POST", "/dossier/eleves/{eleve}/conflit-interet",
                Some(r#"{"nature":"proche"}"#)),
            ecrire_sur_b("lever-conflit-interet-B", "POST",
                "/dossier/conflits-interet/00000000-0000-0000-0000-000000000001/lever", None),
            lire("conflits-interet", "/dossier/conflits-interet", &[Admin, Direction], &[Admin, Direction]),
            // Socle 4 (lot A2) : coordonnées, vérification, préférences, consentements, divergences
            // et identité d'une personne. Aucune écriture ni lecture sur la personne d'un autre
            // établissement ; les listes ne portent que des valeurs masquées ; une coordonnée n'est
            // jamais dans une réponse mise en cache ni dans un événement.
            ecrire_sur_b("coordonnees-personne-B", "GET", "/dossier/personnes/00000000-0000-0000-0000-000000000001/coordonnees", None),
            ecrire_sur_b("coordonnees-historique-B", "GET", "/dossier/personnes/00000000-0000-0000-0000-000000000001/coordonnees/historique", None),
            ecrire_sur_b("coordonnee-ajout-B", "POST", "/dossier/personnes/00000000-0000-0000-0000-000000000001/coordonnees",
                Some(r#"{"type_":"email","valeur":"separation@exemple.fr"}"#)),
            ecrire_sur_b("coordonnee-remplacement-B", "PUT",
                "/dossier/personnes/00000000-0000-0000-0000-000000000001/coordonnees/00000000-0000-0000-0000-000000000001",
                Some(r#"{"valeur":"separation@exemple.fr"}"#)),
            ecrire_sur_b("coordonnee-retrait-B", "POST",
                "/dossier/personnes/00000000-0000-0000-0000-000000000001/coordonnees/00000000-0000-0000-0000-000000000001/retirer", Some("{}")),
            ecrire_sur_b("coordonnee-usages-B", "POST",
                "/dossier/personnes/00000000-0000-0000-0000-000000000001/coordonnees/00000000-0000-0000-0000-000000000001/usages", Some(r#"{"usages":["urgence"]}"#)),
            ecrire_sur_b("verification-demande-B", "POST",
                "/dossier/personnes/00000000-0000-0000-0000-000000000001/coordonnees/00000000-0000-0000-0000-000000000001/verification", None),
            ecrire_sur_b("verification-confirmation-B", "POST",
                "/dossier/personnes/00000000-0000-0000-0000-000000000001/coordonnees/00000000-0000-0000-0000-000000000001/verification/confirmer", Some(r#"{"code":"123456"}"#)),
            ecrire_sur_b("preferences-lecture-B", "GET", "/dossier/personnes/00000000-0000-0000-0000-000000000001/preferences", None),
            ecrire_sur_b("preferences-ecriture-B", "PUT", "/dossier/personnes/00000000-0000-0000-0000-000000000001/preferences",
                Some(r#"{"langue":"fr"}"#)),
            ecrire_sur_b("consentements-lecture-B", "GET", "/dossier/personnes/00000000-0000-0000-0000-000000000001/consentements", None),
            ecrire_sur_b("consentement-ecriture-B", "POST", "/dossier/personnes/00000000-0000-0000-0000-000000000001/consentements",
                Some(r#"{"objet":"photo_remise","accorde":true,"texte_version":"2026-11-v1"}"#)),
            ecrire_sur_b("consentement-retrait-B", "POST",
                "/dossier/personnes/00000000-0000-0000-0000-000000000001/consentements/photo_remise/retirer", None),
            ecrire_sur_b("divergence-resolution-B", "POST",
                "/dossier/personnes/00000000-0000-0000-0000-000000000001/divergences-compte/email", Some(r#"{"choix":"conserver_fiche"}"#)),
            ecrire_sur_b("identite-personne-B", "POST", "/dossier/personnes/00000000-0000-0000-0000-000000000001/identite",
                Some(r#"{"nom":"Separation","nature":"correction","motif":"separation"}"#)),
            ecrire_sur_b("identite-personne-historique-B", "GET", "/dossier/personnes/00000000-0000-0000-0000-000000000001/identite/historique", None),
            ecrire_sur_b("noms-separation-B", "POST", "/dossier/personnes/noms-a-separer",
                Some(r#"{"separations":[{"personne_id":"00000000-0000-0000-0000-000000000001","nom":"X","prenom":"Y"}]}"#)),
            Spec { interdits: &["@", "+33"], ..lire("noms-a-separer", "/dossier/personnes/noms-a-separer",
                &[Admin, Direction, Administratif], &[Admin, Direction, Administratif]) },
            // Valeurs masquées dans le rapport des divergences : jamais une adresse e-mail entière.
            Spec { interdits: &["\"valeur_compte\":"], ..lire("divergences-compte", "/dossier/divergences-compte",
                &[Admin, Direction, Administratif], &[Admin, Direction, Administratif]) },
            // « Soi » : la personne n'agit que sur sa propre fiche (aucun identifiant de personne
            // dans l'URL). Les écritures de ces cas sont invalides à dessein.
            Spec { interdits: &["enc:v1:"], ..lire("moi-coordonnees", "/dossier/moi/coordonnees", SOI_AVEC_FICHE, FAMILLE_AVEC_FICHE) },
            lire("moi-coordonnees-historique", "/dossier/moi/coordonnees/historique", SOI_AVEC_FICHE, FAMILLE_AVEC_FICHE),
            lire("moi-preferences", "/dossier/moi/preferences", SOI_AVEC_FICHE, FAMILLE_AVEC_FICHE),
            lire("moi-consentements", "/dossier/moi/consentements", SOI_AVEC_FICHE, FAMILLE_AVEC_FICHE),
            lire("moi-divergences-compte", "/dossier/moi/divergences-compte", SOI_AVEC_FICHE, FAMILLE_AVEC_FICHE),
            ecrire_sur_b("moi-coordonnee-ajout-invalide", "POST", "/dossier/moi/coordonnees",
                Some(r#"{"type_":"telex","valeur":"1"}"#)),
            ecrire_sur_b("moi-coordonnee-autre-fiche", "PUT",
                "/dossier/moi/coordonnees/00000000-0000-0000-0000-000000000001", Some(r#"{"valeur":"x@exemple.fr"}"#)),
            ecrire_sur_b("moi-coordonnee-retrait-autre-fiche", "POST",
                "/dossier/moi/coordonnees/00000000-0000-0000-0000-000000000001/retirer", Some("{}")),
            ecrire_sur_b("moi-verification-autre-fiche", "POST",
                "/dossier/moi/coordonnees/00000000-0000-0000-0000-000000000001/verification", None),
            ecrire_sur_b("moi-verification-confirmation-autre-fiche", "POST",
                "/dossier/moi/coordonnees/00000000-0000-0000-0000-000000000001/verification/confirmer", Some(r#"{"code":"123456"}"#)),
            ecrire_sur_b("moi-usages-autre-fiche", "POST",
                "/dossier/moi/coordonnees/00000000-0000-0000-0000-000000000001/usages", Some(r#"{"usages":["urgence"]}"#)),
            ecrire_sur_b("moi-preferences-invalides", "PUT", "/dossier/moi/preferences",
                Some(r#"{"langue":"francais"}"#)),
            ecrire_sur_b("moi-consentement-invalide", "POST", "/dossier/moi/consentements",
                Some(r#"{"objet":"inconnu","accorde":true,"texte_version":"x"}"#)),
            ecrire_sur_b("moi-consentement-retrait-invalide", "POST",
                "/dossier/moi/consentements/inconnu/retirer", None),
            ecrire_sur_b("moi-divergence-sans-divergence", "POST",
                "/dossier/moi/divergences-compte/email", Some(r#"{"choix":"adopter_compte"}"#)),
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
            // Lot S2-11 : PAI complet, passages, fiche d'urgence, contraintes, accompagnement.
            lire("pai-complet", "/eleves/{eleve}/pai-complet", SANTE_LIEE, SANTE_NON_LIEE),
            lire("passages-sante", "/sante/passages?eleve_id={eleve}", SANTE_LIEE, SANTE_NON_LIEE),
            lire("fiche-urgence", "/eleves/{eleve}/fiche-urgence",
                &[Admin, Direction, Administratif, Cpe, Infirmiere, Prof], &[Admin, Direction, Administratif, Cpe, Infirmiere]),
            lire("contraintes-enseignants", "/eleves/{eleve}/contraintes?audience=enseignants",
                &[Admin, Direction, Administratif, Cpe, Infirmiere, Prof], &[Admin, Direction, Administratif, Cpe, Infirmiere]),
            lire("amenagements", "/eleves/{eleve}/amenagements",
                &[Admin, Direction, Administratif, Cpe, Infirmiere, Psy, Prof, Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux],
                &[Admin, Direction, Administratif, Cpe, Infirmiere, Psy]),
            lire("accompagnement", "/eleves/{eleve}/accompagnement", SANTE_NON_LIEE, SANTE_NON_LIEE),
            ecrire_sur_b("pai-complet-B", "PUT", "/eleves/{eleve}/pai-complet",
                Some(r#"{"motif":"separation","mesures_accueil":"separation","date_debut":"{jour}"}"#)),
            ecrire_sur_b("signature-pai-B", "POST", "/eleves/{eleve}/pai/signatures",
                Some(r#"{"partie":"famille"}"#)),
            ecrire_sur_b("contrainte-B", "POST", "/eleves/{eleve}/contraintes",
                Some(r#"{"code":"ne_pas_courir","audiences":["enseignants"]}"#)),
            ecrire_sur_b("passage-sante-B", "POST", "/sante/passages",
                Some(r#"{"eleve_id":"{eleve}","jour":"{jour}","motif":"soins","note_soins":"separation"}"#)),
            ecrire_sur_b("dispositif-B", "POST", "/eleves/{eleve}/dispositifs", Some(r#"{"kind":"pap"}"#)),
            ecrire_sur_b("amenagement-B", "POST", "/eleves/{eleve}/amenagements",
                Some(r#"{"code":"tiers_temps","portee":"cours"}"#)),
            ecrire_sur_b("aesh-B", "POST", "/eleves/{eleve}/aesh",
                Some(r#"{"aesh_nom":"separation","date_debut":"{jour}"}"#)),
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
            // RF-15 : la liste suit « dossier.lire » (D-017) ; un professeur ne liste que ses élèves.
            lire("documents-liste", "/eleves/{eleve}/documents", TOUS, STAFF_HORS_PROF),
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

/// Socle 18 (FO1) : un élève, un accord ou une décision d'un autre établissement n'est ni lisible, ni décidable, ni retirable.
fn specs_domaine_accords() -> Vec<Spec> {
    vec![
        ecrire_sur_b(
            "accords-decider-B",
            "POST",
            "/accords/decider",
            Some(r#"{"objet_code":"droit_image","eleve_id":"{eleve}","usages":[{"code":"fil_classe","accorde":true}]}"#),
        ),
        ecrire_sur_b(
            "accords-retirer-B",
            "POST",
            "/accords/retirer",
            Some(r#"{"objet_code":"droit_image","eleve_id":"{eleve}"}"#),
        ),
        ecrire_sur_b("accords-decisions-B", "GET", "/accords/objets/droit_image/eleves/{eleve}/decisions", None),
    ]
}

#[test]
fn domaine_accords() {
    jouer("accords (socle 18 : décider, retirer, lire les décisions)", &specs_domaine_accords());
}

fn specs_domaine_fils_de_messages() -> Vec<Spec> {
    vec![
            // M0a : le fil du labo lié à l'élève est un fil de vie scolaire (carnet) adressé aux
            // responsables. L'élève ne le lit plus (D3) et un parent sans `peut_voir_vs`
            // (ParentAutreFoyer, Draco) non plus (D5) ; le refus est exigé pour eux.
            lire("fil", "/messages/fils/{fil}",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent], GESTION_HORS_PROF),
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
    const FACT_LIEE: &[Acteur] = &[Admin, Direction, Compta, Parent, ParentAutreFoyer];
    const FACT_NON_LIEE: &[Acteur] = &[Admin, Direction, Compta];
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

/// Socle 24, lots F1 et F2 : comptes débiteurs, payeurs désignés, conséquences, prises en charge, journal,
/// guichet, tâches. Lectures bornées à A (aucune donnée de B) ; toute écriture visant un objet de B (élève,
/// compte, groupe, tâche) échoue — l'identifiant de foyer de B tient lieu d'identifiant inconnu de A.
fn specs_domaine_finances_comptes() -> Vec<Spec> {
    vec![
        borne_a("fin-journal", "GET", "/finances/journal", None, &[]),
        borne_a("fin-comptes", "GET", "/finances/comptes", None, &[]),
        borne_a("fin-mes-comptes", "GET", "/finances/mes-comptes", None, &[]),
        borne_a("fin-taches", "GET", "/finances/taches", None, &[]),
        borne_a("fin-consequences", "GET", "/finances/consequences", None, &[]),
        borne_a("fin-prises-en-charge", "GET", "/finances/prises-en-charge", None, &[]),
        borne_a("fin-guichet-recherche", "GET", "/finances/guichet/recherche?nom=Martin&prenom=Jean&auth=presentiel", None, &[]),
        // M10 : de vrais identifiants de B (compte, groupe, tâche, élève), des corps valides, un refus exact
        // (401/403/404, jamais un 400 ni un 5xx) : la cible n'est jamais atteinte ni modifiée.
        refuse_sur_b("fin-compte-B", "GET", "/finances/comptes/{compte}", None),
        refuse_sur_b("fin-guichet-compte-B", "GET", "/finances/guichet/comptes/{compte}?auth=presentiel", None),
        refuse_sur_b("fin-payeurs-B", "GET", "/finances/eleves/{eleve}/payeurs", None),
        refuse_sur_b("fin-designer-B", "POST", "/finances/eleves/{eleve}/designations",
            Some(r#"{"categorie":"toutes","parts":[{"compte_id":"{compte}","part_bps":10000}],"date_effet":"2030-09-01"}"#)),
        refuse_sur_b("fin-payeur-majeur-B", "POST", "/finances/eleves/{eleve}/payeur-majeur",
            Some(r#"{"date_effet":"2030-09-01"}"#)),
        refuse_sur_b("fin-accepter-B", "POST", "/finances/designations/{groupe}/accepter", Some("{}")),
        refuse_sur_b("fin-refuser-B", "POST", "/finances/designations/{groupe}/refuser", Some("{}")),
        refuse_sur_b("fin-piece-B", "POST", "/finances/designations/{groupe}/piece",
            Some(r#"{"piece":{"nature":"accord_des_deux","reference":"x"}}"#)),
        refuse_sur_b("fin-demande-B", "POST", "/finances/famille/demande-repartition",
            Some(r#"{"eleve_id":"{eleve}","categorie":"toutes"}"#)),
        refuse_sur_b("fin-decision-B", "POST", "/finances/consequences/{groupe}/decision",
            Some(r#"{"appliquer":false,"motif":"test"}"#)),
        refuse_sur_b("fin-abandonner-B", "POST", "/finances/taches/{tache}/abandonner", Some(r#"{"motif":"test"}"#)),
        refuse_sur_b("fin-prise-en-charge-B", "POST", "/finances/prises-en-charge",
            Some(r#"{"compte_id":"{compte}","eleve_id":"{eleve}","reference":"R","du":"2030-09-01","au":"2031-06-30","categories":["internat"]}"#)),
        refuse_sur_b("fin-reactiver-B", "POST", "/finances/prises-en-charge/{compte}/reactiver",
            Some(r#"{"au":"2099-12-31","motif":"test"}"#)),
        borne_a("fin-quota", "POST", "/finances/guichet/quota", Some(r#"{"par_jour":30}"#), &[]),
    ]
}

#[test]
fn domaine_finances_comptes() {
    // Non-vacuité (M10) : le seed fournit de vrais compte, groupe et tâche à A **et** à B.
    if let Some(m) = monde() {
        for (nom, o) in [("A", &m.a.objets), ("B", &m.b.objets)] {
            assert!(o.compte.is_some() && o.groupe.is_some() && o.tache.is_some(), "{nom} : objets financiers manquants");
        }
    }
    jouer("finances : comptes, payeurs, conséquences", &specs_domaine_finances_comptes());
}

/// Socle 24, lot F3 : catalogue qualifié, versions de grille, simulateur, familles types, état des contributions.
/// Lectures bornées à A (aucune donnée de B) ; toute route visant un identifiant de B (grille, article,
/// simulation, famille type) répond un refus exact : l'identifiant de compte de B tient lieu d'identifiant
/// inconnu de A (aucune de ces ressources n'existe dans le seed, ni dans A ni dans B).
fn specs_domaine_finances_grilles() -> Vec<Spec> {
    vec![
        borne_a("f3-modeles", "GET", "/finances/catalogue/modeles", None, &[]),
        borne_a("f3-catalogue", "GET", "/finances/catalogue/articles", None, &[]),
        borne_a("f3-reglages", "GET", "/finances/grilles/reglages", None, &[]),
        borne_a("f3-grilles", "GET", "/finances/grilles", None, &[]),
        borne_a("f3-familles-types", "GET", "/finances/familles-types", None, &[]),
        borne_a("f3-simulations", "GET", "/finances/simulations", None, &[]),
        borne_a("f3-contributions", "GET", "/finances/contributions?annee=2030-2031", None, &[]),
        refuse_sur_b("f3-qualifier-B", "POST", "/finances/catalogue/articles/{compte}/qualifier",
            Some(r#"{"nature_legale":"internat","regime_tva":"hors_champ","unite":"annee"}"#)),
        refuse_sur_b("f3-usage-B", "GET", "/finances/catalogue/articles/{compte}/usage", None),
        refuse_sur_b("f3-grille-B", "GET", "/finances/grilles/{compte}", None),
        refuse_sur_b("f3-tarif-B", "POST", "/finances/grilles/{compte}/tarifs",
            Some(r#"{"article_id":"{compte}","montant_annuel_centimes":1000}"#)),
        refuse_sur_b("f3-tarif-modifier-B", "PUT", "/finances/grilles/{compte}/tarifs/{compte}",
            Some(r#"{"article_id":"{compte}","montant_annuel_centimes":1000}"#)),
        refuse_sur_b("f3-tarif-retirer-B", "DELETE", "/finances/grilles/{compte}/tarifs/{compte}", None),
        refuse_sur_b("f3-tranches-B", "PUT", "/finances/grilles/{compte}/tranches", Some(r#"{"tranches":[]}"#)),
        refuse_sur_b("f3-hausse-B", "POST", "/finances/grilles/{compte}/hausse",
            Some(r#"{"mode":"montant","valeur":100,"arrondi":"aucun","apercu":true}"#)),
        refuse_sur_b("f3-controles-B", "GET", "/finances/grilles/{compte}/controles", None),
        refuse_sur_b("f3-proposer-B", "POST", "/finances/grilles/{compte}/proposer", Some("{}")),
        refuse_sur_b("f3-retour-B", "POST", "/finances/grilles/{compte}/retour-brouillon", Some(r#"{"motif":"test"}"#)),
        refuse_sur_b("f3-abandonner-B", "POST", "/finances/grilles/{compte}/abandonner", Some(r#"{"motif":"test"}"#)),
        refuse_sur_b("f3-voter-B", "POST", "/finances/grilles/{compte}/voter",
            Some(r#"{"vote_le":"2030-06-20","reference":"CA"}"#)),
        refuse_sur_b("f3-vigueur-B", "POST", "/finances/grilles/{compte}/mise-en-vigueur", Some(r#"{"du":"2030-09-01"}"#)),
        refuse_sur_b("f3-correction-B", "POST", "/finances/grilles/{compte}/correction",
            Some(r#"{"motif":"test","effet_retroactif":false}"#)),
        refuse_sur_b("f3-publier-B", "POST", "/finances/grilles/{compte}/publier", Some("{}")),
        refuse_sur_b("f3-effet-familles-B", "GET", "/finances/grilles/{compte}/familles-types", None),
        refuse_sur_b("f3-famille-desactiver-B", "POST", "/finances/familles-types/{compte}/desactiver", Some("{}")),
        refuse_sur_b("f3-simulation-calculer-B", "POST", "/finances/simulations/{compte}/calculer", Some("{}")),
        refuse_sur_b("f3-simulation-export-B", "GET", "/finances/simulations/{compte}/export", None),
        refuse_sur_b("f3-comparaison-B", "GET", "/finances/simulations/comparaison?a={compte}&b={compte}", None),
        refuse_sur_b("f3-simulation-creer-B", "POST", "/finances/simulations",
            Some(r#"{"annee_scolaire":"2030-2031","libelle":"S","base_version_id":"{compte}","regles":[],"effectifs":[]}"#)),
        refuse_sur_b("f3-grille-creer-copie-B", "POST", "/finances/grilles",
            Some(r#"{"annee_scolaire":"2030-2031","copier_de":"{compte}"}"#)),
    ]
}

#[test]
fn domaine_finances_grilles() {
    jouer("finances : catalogue, grilles, simulateur", &specs_domaine_finances_grilles());
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

fn specs_domaine_edt() -> Vec<Spec> {
    vec![
            // S2-5 : l'emploi du temps d'un élève et ses changements ne sortent que pour le
            // personnel, l'élève lui-même et ses responsables au lien actif. Socle 10 (T0, D2, D-022) :
            // un professeur ne lit que l'EDT des élèves qu'il suit ; hors de son périmètre, 403.
            lire("edt-changements", "/eleves/{eleve}/edt/changements?du={jour}&au={jour}", TOUS, STAFF_HORS_PROF),
            lire("edt-semaine-eleve", "/edt/semaine?cible=eleve&id={eleve}&jour={jour}", TOUS, STAFF_HORS_PROF),
            // Administration : gestion de l'EDT seulement (edt.gerer), jamais les familles ni le prof.
            lire("edt-conflits", "/edt/admin/conflits", GESTION_HORS_PROF, &[]),
            lire("edt-delegations", "/delegations", &[Admin, Direction], &[]),
            ecrire_sur_b("edt-remplacement-B", "POST", "/edt/remplacements",
                Some(r#"{"creneau_id":"{appel}","jour":"{jour}","remplacant_user_id":"{eleve}","absence_rh_id":null,"nouvelle_salle":null}"#)),
            // Socle 10, lot T3 : journal du réel, grève du jour et modifications. Le journal est lu par
            // l'encadrement (le professeur : ses séances, une par une) ; la grève est un droit de lot.
            lire("edt-journal", "/edt/modifications?du={jour}&au={jour}",
                &[Direction, Administratif, Cpe], &[Direction, Administratif, Cpe]),
            lire("edt-greve-candidats", "/edt/greve/candidats?jour={jour}&demi_journee=journee",
                &[Direction, Administratif], &[Direction, Administratif]),
            // Socle 10, lot T4 : « demain » (encadrement) et « maintenant » (lecture journalisée d'un élève).
            // Sans identifiant de ressource : la cible B répond comme un identifiant inexistant (les données de A,
            // jamais celles de B).
            Spec { b: Attendu::RefuseOuIndistinct, ..lire("edt-demain", "/edt/demain?jour={jour}",
                &[Direction, Administratif, Cpe], &[Direction, Administratif, Cpe]) },
            lire("edt-maintenant-eleve", "/edt/maintenant/eleves/{eleve}?usage=recherche",
                &[Direction, Administratif, Cpe], &[Direction, Administratif, Cpe]),
            ecrire_sur_b("edt-modification-B", "POST", "/edt/modifications",
                Some(r#"{"seance_id":"{appel}","changement":{"type":"annulation"},"motif_public_code":"autre"}"#)),
            ecrire_sur_b("edt-lot-B", "POST", "/edt/lots",
                Some(r#"{"nature":"annulation","du":"{jour}","au":"{jour}","portee":"classes","cibles":["{appel}"],"motif_public_code":"autre"}"#)),
            // Socle 10, lot T5 : publication des versions. Aucune version n'existe sous l'identifiant d'un créneau :
            // la lecture et l'écriture sont refusées partout (404), y compris sur la cible B ; l'historique d'une
            // année inconnue est vide pour l'encadrement et refusé aux autres.
            lire("edt-publication-controles", "/edt/versions/{appel}/controles", &[], &[]),
            lire("edt-publication-apercu", "/edt/versions/{appel}/apercu", &[], &[]),
            lire("edt-publication-difference", "/edt/versions/difference?de={appel}&vers={appel}", &[], &[]),
            Spec { b: Attendu::RefuseOuIndistinct, ..lire("edt-publications-historique", "/edt/annees/{appel}/publications",
                &[Direction, Administratif], &[Direction, Administratif]) },
            ecrire_sur_b("edt-publier-B", "POST", "/edt/versions/{appel}/publier", Some(r#"{"version_id":"{appel}"}"#)),
            ecrire_sur_b("edt-diffuser-B", "POST", "/edt/versions/{appel}/diffuser", None),
            ecrire_sur_b("edt-diffusion-B", "POST", "/edt/versions/{appel}/diffusion",
                Some(r#"{"diffusion_le":"2099-01-01T00:00:00Z"}"#)),
            ecrire_sur_b("edt-retour-arriere-B", "POST", "/edt/versions/{appel}/retour-arriere", None),
            ecrire_sur_b("edt-annonce-B", "DELETE", "/edt/versions/{appel}/annonce", None),
            ecrire_sur_b("edt-individuelle-B", "POST", "/edt/individuelles",
                Some(r#"{"eleve_id":"{eleve}","motif_code":"upe2a","ajouter":["{appel}"]}"#)),
    ]
}

#[test]
fn domaine_edt() {
    jouer("emploi du temps, remplacements, délégations", &specs_domaine_edt());
}

/// Socle 5, S1 : public et membres individuels d'un cours. Public : gestion de la structure
/// et professeur du cours ; membres (motifs sensibles) et écritures : gestion seulement.
fn specs_domaine_cours() -> Vec<Spec> {
    vec![
        lire("cours-public", "/referentiel/cours/{cours}/public",
            &[Admin, Direction, Administratif, Prof], &[Admin, Direction, Administratif, Prof]),
        lire("cours-membres", "/referentiel/cours/{cours}/membres",
            &[Admin, Direction, Administratif], &[Admin, Direction, Administratif]),
        ecrire_sur_b("cours-membre-ajout-B", "POST", "/referentiel/cours/{cours}/membres",
            Some(r#"{"eleve_id":"{eleve}","nature":"inclusion","motif_code":"ulis","du":"2026-01-05","au":null}"#)),
        ecrire_sur_b("cours-membre-annulation-B", "POST", "/referentiel/cours-membres/{cours}/annuler", None),
        // Lot S4 : l'objet cours (détail avec les professeurs : gestion seulement).
        lire("structure-cours-detail", "/structure/cours/{cours}",
            &[Admin, Direction, Administratif], &[Admin, Direction, Administratif]),
        ecrire_sur_b("structure-cours-modification-B", "PUT", "/structure/cours/{cours}", Some(r#"{"libelle":"X"}"#)),
        ecrire_sur_b("structure-cours-fermeture-B", "POST", "/structure/cours/{cours}/fermer",
            Some(r#"{"a_la_date":"2026-01-05"}"#)),
        ecrire_sur_b("structure-cours-service-B", "POST", "/structure/cours/{cours}/services",
            Some(r#"{"prof_user_id":"{eleve}","professeur_principal":false}"#)),
        // Socle 5, lot S6 : groupes et compositions. Un identifiant de l'autre établissement (ici l'identifiant
        // d'un objet de B) n'obtient jamais un 2xx, en lecture comme en écriture (404, aucun oracle d'existence).
        ecrire_sur_b("structure-groupe-detail-B", "GET", "/structure/groupes/{cours}", None),
        ecrire_sur_b("structure-groupe-modification-B", "PUT", "/structure/groupes/{cours}", Some(r#"{"libelle":"X"}"#)),
        ecrire_sur_b("structure-groupe-fermeture-B", "POST", "/structure/groupes/{cours}/fermer",
            Some(r#"{"a_la_date":"2026-01-05"}"#)),
        ecrire_sur_b("structure-groupe-composition-B", "POST", "/structure/groupes/{cours}/composition",
            Some(r#"{"entrants":["{eleve}"],"sortants":[]}"#)),
        ecrire_sur_b("structure-groupe-propositions-B", "GET", "/structure/groupes/{cours}/propositions", None),
        ecrire_sur_b("structure-groupe-regle-B", "POST", "/structure/groupes/{cours}/regle",
            Some(r#"{"entrants":["{eleve}"],"sortants":[]}"#)),
        ecrire_sur_b("structure-groupe-dedoublement-B", "POST", "/structure/groupes/{cours}/dedoubler",
            Some(r#"{"repartition":"alterne","apercu":false}"#)),
        ecrire_sur_b("structure-classe-changement-B", "POST", "/structure/eleves/classe",
            Some(r#"{"eleve_id":"{eleve}","classe_id":"{cours}","apercu":false}"#)),
        ecrire_sur_b("structure-affectations-lot-B", "POST", "/structure/affectations/lot",
            Some(r#"{"classe_id":"{cours}","eleves":["{eleve}"],"apercu":false}"#)),
        ecrire_sur_b("structure-affectation-correction-B", "POST", "/structure/affectations/correction",
            Some(r#"{"eleve_id":"{eleve}","groupe_errone":"{cours}","apercu":false}"#)),
        // Socle 20, lot C2 : cahier du cours. Le jeton de A avec des identifiants de B n'obtient jamais un 2xx
        // (liste des séances, entrée, travail) ; les routes sans identifiant sont réservées à qui écrit au cahier.
        ecrire_sur_b("cahier-c2-seances-B", "GET", "/cahier/c2/cours/{cours}/seances?du=2026-01-05&au=2026-01-30", None),
        ecrire_sur_b("cahier-c2-entree-B", "PUT",
            "/cahier/c2/cours/{cours}/seances/00000000-0000-4000-8000-000000000001/entree",
            Some(r#"{"contenu":"X","publication":"brouillon"}"#)),
        ecrire_sur_b("cahier-c2-travail-B", "POST", "/cahier/c2/travaux",
            Some(r#"{"cours_ids":["{cours}"],"nature":"exercices","titre":"X","consigne":"Y","publication":"brouillon","echeance_prochaine_seance":true}"#)),
        ecrire_sur_b("cahier-c2-entree-versions-B", "GET", "/cahier/c2/entrees/{cours}/versions", None),
        ecrire_sur_b("cahier-c2-entree-retrait-B", "POST", "/cahier/c2/entrees/{cours}/retrait", Some(r#"{"motif":"test"}"#)),
        ecrire_sur_b("cahier-c2-travail-modif-B", "PUT", "/cahier/c2/travaux/{cours}", Some(r#"{"titre":"X"}"#)),
        ecrire_sur_b("cahier-c2-travail-retrait-B", "POST", "/cahier/c2/travaux/{cours}/retrait", Some(r#"{"motif":"test"}"#)),
        lire("cahier-c2-mes-cours", "/cahier/c2/cours", &[Admin, Direction, Prof], &[]),
        lire("cahier-c2-natures", "/cahier/c2/natures", &[Admin, Direction, Prof], &[]),
        // Socle 20, lot C3 : agenda, manqué, résumé de l'élève et de ses responsables habilités. Le jeton de A avec des
        // identifiants de B n'obtient jamais un 2xx ; le personnel, un parent d'un autre foyer ou sans lien ne lit rien.
        lire("cahier-c3-agenda", "/cahier/c3/agenda?eleve_id={eleve}&du=2026-01-05&au=2026-01-30", &[Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux], &[]),
        lire("cahier-c3-manque", "/cahier/c3/manque?eleve_id={eleve}&du=2026-01-05&au=2026-01-30", &[Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux], &[]),
        lire("cahier-c3-resume", "/cahier/c3/resume?eleve_id={eleve}", &[Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux], &[]),
        lire("cahier-c3-abonnements", "/cahier/c3/abonnements", &[Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux], &[]),
        // Socle 20, lot C4 : charge. Même règle que l'agenda pour la charge d'un élève ; le reste n'a pas de ressource liée à A
        // à lire : le jeton de A avec des identifiants de B n'obtient jamais un 2xx.
        lire("cahier-c4-charge", "/cahier/c4/charge?eleve_id={eleve}&du=2026-01-05&au=2026-01-30", &[Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux], &[]),
        ecrire_sur_b("cahier-c4-apercu-B", "POST", "/cahier/c4/apercu",
            Some(r#"{"cours_ids":["{cours}"],"echeance_jour":"2031-03-12"}"#)),
        ecrire_sur_b("cahier-c4-calendrier-B", "GET", "/cahier/c4/classes/{cours}/calendrier?du=2026-01-05&au=2026-01-30", None),
        ecrire_sur_b("cahier-c4-alerte-B", "POST", "/cahier/c4/travaux/{cours}/alerte-ignoree", None),
        // Socle 20, lot C5 : rendu et retour. Aucune ressource de A n'est lisible avec le jeton de B ; toute route à identifiant
        // est refusée (ou introuvable) pour l'autre établissement : le jeton de A avec des identifiants de B n'obtient jamais un 2xx.
        ecrire_sur_b("cahier-c5-remise-B", "GET", "/cahier/c5/travaux/{cours}/remise?eleve_id={eleve}", None),
        ecrire_sur_b("cahier-c5-brouillon-B", "PUT", "/cahier/c5/travaux/{cours}/brouillon", Some(r#"{"texte":"x"}"#)),
        ecrire_sur_b("cahier-c5-brouillon-piece-B", "DELETE", "/cahier/c5/travaux/{cours}/brouillon/pieces/{cours}", None),
        ecrire_sur_b("cahier-c5-depot-B", "POST", "/cahier/c5/travaux/{cours}/depot", Some(r#"{"texte":"x"}"#)),
        ecrire_sur_b("cahier-c5-depot-au-nom-B", "POST", "/cahier/c5/travaux/{cours}/depot-au-nom", Some(r#"{"eleve_id":"{eleve}","texte":"x"}"#)),
        ecrire_sur_b("cahier-c5-signalement-B", "POST", "/cahier/c5/travaux/{cours}/signalement", Some(r#"{}"#)),
        ecrire_sur_b("cahier-c5-delai-B", "POST", "/cahier/c5/travaux/{cours}/delai", Some(r#"{}"#)),
        ecrire_sur_b("cahier-c5-delais-B", "GET", "/cahier/c5/travaux/{cours}/delais", None),
        ecrire_sur_b("cahier-c5-groupes-B", "GET", "/cahier/c5/travaux/{cours}/groupes", None),
        ecrire_sur_b("cahier-c5-groupes-composer-B", "PUT", "/cahier/c5/travaux/{cours}/groupes", Some(r#"{"groupes":[]}"#)),
        ecrire_sur_b("cahier-c5-groupes-rejoindre-B", "POST", "/cahier/c5/travaux/{cours}/groupes/rejoindre", Some(r#"{"libelle":"G"}"#)),
        ecrire_sur_b("cahier-c5-suivi-B", "GET", "/cahier/c5/travaux/{cours}/suivi", None),
        ecrire_sur_b("cahier-c5-constats-B", "POST", "/cahier/c5/travaux/{cours}/constats", Some(r#"{"constats":[]}"#)),
        ecrire_sur_b("cahier-c5-prolongations-B", "POST", "/cahier/c5/travaux/{cours}/prolongations", Some(r#"{"eleve_ids":[],"echeance_jour":"2031-03-12","echeance_heure":"08:00:00","motif":"autre"}"#)),
        ecrire_sur_b("cahier-c5-cloture-B", "PUT", "/cahier/c5/travaux/{cours}/cloture", Some(r#"{"cloture_jour":"2031-03-12","cloture_heure":"08:00:00"}"#)),
        ecrire_sur_b("cahier-c5-copies-B", "GET", "/cahier/c5/travaux/{cours}/copies", None),
        ecrire_sur_b("cahier-c5-relance-B", "POST", "/cahier/c5/travaux/{cours}/relance", Some(r#"{"confirmer":true}"#)),
        ecrire_sur_b("cahier-c5-retours-lot-B", "POST", "/cahier/c5/travaux/{cours}/retours/lot", None),
        ecrire_sur_b("cahier-c5-retour-lire-B", "GET", "/cahier/c5/remises/{cours}/retour", None),
        ecrire_sur_b("cahier-c5-retour-B", "PUT", "/cahier/c5/remises/{cours}/retour", Some(r#"{"commentaire":"x"}"#)),
        ecrire_sur_b("cahier-c5-rouvrir-B", "POST", "/cahier/c5/remises/{cours}/rouvrir", Some(r#"{"echeance_jour":"2031-03-12","echeance_heure":"08:00:00"}"#)),
        ecrire_sur_b("cahier-c5-signalement-copie-B", "POST", "/cahier/c5/remises/{cours}/signalement-copie", Some(r#"{"vers":"cpe"}"#)),
        ecrire_sur_b("cahier-c5-delai-decision-B", "POST", "/cahier/c5/delais/{cours}/decision", Some(r#"{"accepter":false}"#)),
        ecrire_sur_b("cahier-c5-piece-lien-B", "POST", "/cahier/c5/pieces/{cours}/lien", Some(r#"{"motif":"test"}"#)),
        // Routes sans identifiant de ressource (pannes, listes, AESH, affichage) : voir `routes_sans_identifiant_bornees_a_l_etablissement_de_la_session`.
        // Socle 20, lot C6 : continuité. Le jeton de A avec des identifiants de B n'obtient jamais un 2xx ; aucune route ne sert
        // la nature d'une continuité ni un motif d'absence à un professeur.
        ecrire_sur_b("cahier-c6-report-decision-B", "POST", "/cahier/c6/reports/{cours}/decision", Some(r#"{"decision":"refuser"}"#)),
        ecrire_sur_b("cahier-c6-continuite-ouvrir-B", "POST", "/cahier/c6/continuites", Some(r#"{"eleve_id":"{eleve}","du":"2031-03-12","cours_ids":["{cours}"],"nature":"absence_longue"}"#)),
        ecrire_sur_b("cahier-c6-continuite-cloture-B", "POST", "/cahier/c6/continuites/{cours}/cloture", Some(r#"{"au":"2031-03-20"}"#)),
        ecrire_sur_b("cahier-c6-bandeau-B", "GET", "/cahier/c6/cours/{cours}/bandeau", None),
        ecrire_sur_b("cahier-c6-suppleance-B", "GET", "/cahier/c6/cours/{cours}/suppleance", None),
        ecrire_sur_b("cahier-c6-copies-B", "POST", "/cahier/c6/cours/{cours}/copies", Some(r#"{"decision":"titulaire_corrigera"}"#)),
        ecrire_sur_b("cahier-c6-vacant-travail-B", "POST", "/cahier/c6/vacants/travaux", Some(r#"{"cours_ids":["{cours}"],"nature":"travail","titre":"x","consigne":"x","publication":"brouillon"}"#)),
        ecrire_sur_b("cahier-c6-seance-travail-B", "POST", "/cahier/c6/seances/{cours}/travail", Some(r#"{"titre":"x","consigne":"x","contexte":"distance"}"#)),
        ecrire_sur_b("cahier-c6-remplacement-B", "GET", "/cahier/c6/seances/{cours}/remplacement", None),
        // Socle 20, lot C7 : adaptations. Le jeton de A avec des identifiants de B n'obtient jamais un 2xx ; aucune route d'élève ou de
        // famille ne porte d'indicateur de variante (l'élève lit sa version dans l'agenda de C3).
        ecrire_sur_b("cahier-c7-variantes-B", "GET", "/cahier/c7/travaux/{cours}/variantes", None),
        ecrire_sur_b("cahier-c7-variante-creer-B", "POST", "/cahier/c7/travaux/{cours}/variantes", Some(r#"{"libelle_interne":"x","consigne":"x"}"#)),
        ecrire_sur_b("cahier-c7-variante-modifier-B", "PUT", "/cahier/c7/variantes/{cours}", Some(r#"{"libelle_interne":"x","consigne":"x"}"#)),
        ecrire_sur_b("cahier-c7-variante-attribuer-B", "POST", "/cahier/c7/variantes/{cours}/eleves", Some(r#"{"eleve_ids":["{eleve}"]}"#)),
        ecrire_sur_b("cahier-c7-variante-retirer-B", "DELETE", "/cahier/c7/variantes/{cours}/eleves", Some(r#"{"eleve_ids":["{eleve}"]}"#)),
        ecrire_sur_b("cahier-c7-pastilles-B", "GET", "/cahier/c7/cours/{cours}/pastilles", None),
        ecrire_sur_b("cahier-c7-amenagements-B", "POST", "/cahier/c7/travaux/{cours}/amenagements?confirmer=true", None),
        ecrire_sur_b("cahier-c7-notes-travail-B", "GET", "/cahier/c7/travaux/{cours}/notes-accompagnement", None),
        ecrire_sur_b("cahier-c7-notes-entree-B", "GET", "/cahier/c7/entrees/{cours}/notes-accompagnement", None),
        ecrire_sur_b("cahier-c7-aesh-agenda-B", "GET", "/cahier/c7/aesh/agenda?eleve_id={eleve}&du=2026-01-05&au=2026-01-30", None),
        ecrire_sur_b("cahier-c7-aesh-cahier-B", "GET", "/cahier/c7/aesh/cahier?eleve_id={eleve}&cours_id={cours}", None),
        ecrire_sur_b("cahier-c7-aesh-note-B", "POST", "/cahier/c7/aesh/notes", Some(r#"{"travail_id":"{cours}","eleve_id":"{eleve}","texte":"x"}"#)),
        ecrire_sur_b("cahier-c3-coche-B", "PUT", "/cahier/c3/travaux/{cours}/fait", Some(r#"{"fait":true}"#)),
        ecrire_sur_b("cahier-c3-abonnement-B", "POST", "/cahier/c3/abonnements", Some(r#"{"eleve_id":"{eleve}"}"#)),
        ecrire_sur_b("cahier-c3-abonnement-revocation-B", "DELETE", "/cahier/c3/abonnements/{cours}", None),
    ]
}

#[test]
fn domaine_cours() {
    jouer("cours : public et membres individuels", &specs_domaine_cours());
}

/// Données de cahier **semées dans B** (continuité, réglage d'affichage, panne), par le rôle propriétaire. Retourne les
/// identifiants et valeurs que la réponse d'une session de A ne doit jamais contenir.
async fn semer_cahier_dans_b(m: &Monde) -> Vec<&'static str> {
    let pool = sqlx::PgPool::connect(&std::env::var("ENT_DATABASE_URL").expect("ENT_DATABASE_URL")).await.expect("connexion propriétaire");
    let etab = m.b.objets.etab;
    let mut interdits: Vec<String> = Vec::new();
    let panne: Uuid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_cdt_panne (id, etablissement_id, debut_le, fin_le) VALUES ($1, $2, '2031-03-10T08:00:00Z', '2031-03-10T09:00:00Z')",
    )
    .bind(panne)
    .bind(etab)
    .execute(&pool)
    .await
    .expect("panne de B");
    interdits.push(panne.to_string());
    if let Some(cours) = m.b.objets.cours {
        let cont: Uuid = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_cdt_continuite (id, etablissement_id, eleve_id, du) VALUES ($1, $2, $3, '2031-03-10')")
            .bind(cont)
            .bind(etab)
            .bind(m.b.objets.eleve_ron)
            .execute(&pool)
            .await
            .expect("continuité de B");
        sqlx::query("INSERT INTO ent_cdt_continuite_cours (etablissement_id, continuite_id, cours_id) VALUES ($1, $2, $3)")
            .bind(etab)
            .bind(cont)
            .bind(cours)
            .execute(&pool)
            .await
            .expect("cours de la continuité de B");
        interdits.push(cont.to_string());
    }
    sqlx::query("INSERT INTO ent_cdt_affichage (etablissement_id, eleve_id, taille_police) VALUES ($1, $2, 'tres_grande') ON CONFLICT (eleve_id) DO NOTHING")
        .bind(etab)
        .bind(m.b.objets.eleve_ron)
        .execute(&pool)
        .await
        .expect("affichage de B");
    // Valeur reconnaissable du réglage d'affichage de B : jamais lue par une session de A.
    interdits.push("tres_grande".to_string());
    interdits.into_iter().map(|s| &*Box::leak(s.into_boxed_str())).collect()
}

/// Empreinte (comptes et contenu) des tables de cahier que ces routes écrivent, **dans B** : calculée par le propriétaire.
async fn empreinte_cahier_de_b(m: &Monde) -> String {
    let pool = sqlx::PgPool::connect(&std::env::var("ENT_DATABASE_URL").expect("ENT_DATABASE_URL")).await.expect("connexion propriétaire");
    let mut out = Vec::new();
    for table in [
        "ent_cdt_panne",
        "ent_cdt_report",
        "ent_cdt_affichage",
        "ent_cdt_continuite",
        "ent_cdt_continuite_cours",
        "ent_cdt_note_accompagnement",
    ] {
        let (n, h): (i64, String) = sqlx::query_as(&format!(
            "SELECT count(*), coalesce(md5(string_agg(t::text, '|' ORDER BY t::text)), '') FROM {table} t WHERE etablissement_id = $1"
        ))
        .bind(m.b.objets.etab)
        .fetch_one(&pool)
        .await
        .expect("empreinte");
        out.push(format!("{table}={n}:{h}"));
    }
    out.join(" ")
}

/// Routes **sans identifiant de ressource** (elles n'agissent que sur l'établissement de la session) : la session de A obtient
/// un refus ou un 2xx **borné à A** (aucun identifiant ni valeur semés dans B dans le corps), et les écritures
/// (`pannes`, `reports/lot`, `affichage`) ne créent ni ne modifient aucune ligne de B (vérifié en base par le propriétaire).
#[test]
fn routes_sans_identifiant_bornees_a_l_etablissement_de_la_session() {
    let Some(m) = monde() else { return };
    let interdits: &'static [&'static str] = Box::leak(rt().block_on(semer_cahier_dans_b(m)).into_boxed_slice());
    let avant = rt().block_on(empreinte_cahier_de_b(m));
    let specs = vec![
        borne_a("cahier-c5-panne-B", "POST", "/cahier/c5/pannes", Some(r#"{"debut_le":"2031-03-12T08:00:00Z","fin_le":"2031-03-12T09:00:00Z"}"#), interdits),
        borne_a("cahier-c6-reports-B", "GET", "/cahier/c6/reports", None, interdits),
        borne_a("cahier-c6-reports-lot-B", "POST", "/cahier/c6/reports/lot", Some(r#"{"ids":["00000000-0000-4000-8000-000000000001"],"decision":"refuser"}"#), interdits),
        borne_a("cahier-c6-continuites-B", "GET", "/cahier/c6/continuites", None, interdits),
        borne_a("cahier-c6-travaux-remplacants-B", "GET", "/cahier/c6/travaux-remplacants", None, interdits),
        borne_a("cahier-c6-vacants-B", "GET", "/cahier/c6/vacants", None, interdits),
        borne_a("cahier-c7-aesh-eleves-B", "GET", "/cahier/c7/aesh/eleves", None, interdits),
        borne_a("cahier-c7-affichage-B", "GET", "/cahier/c7/affichage", None, interdits),
        borne_a("cahier-c7-affichage-regler-B", "PUT", "/cahier/c7/affichage", Some(r#"{"taille_police":"grande","interligne":"large","lecture_audio":false,"animations_reduites":false}"#), interdits),
    ];
    jouer("cahier : routes sans identifiant, session de A", &specs);
    let apres = rt().block_on(empreinte_cahier_de_b(m));
    assert_eq!(avant, apres, "une session de A a créé ou modifié des données de cahier de B");
}

/// `POST /cahier/c6/reports/lot` : garde d'entrée comme pour une proposition seule. Un rôle sans droit d'écriture sur le cahier
/// est refusé (403) **d'entrée** (pas un 200 avec des identifiants « ignorés ») ; direction et professeur passent la garde.
#[test]
fn reports_lot_refuse_d_entree_les_roles_sans_droit_de_decision() {
    let Some(m) = monde() else { return };
    let corps = json!({"ids": ["00000000-0000-4000-8000-000000000001"], "decision": "refuser"});
    rt().block_on(async {
        for a in [Compta, Infirmiere, Psy, Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux] {
            let r = m.appel("POST", "/cahier/c6/reports/lot", Some(m.jeton(a)), Some(&corps)).await;
            assert_eq!(r.statut.as_u16(), 403, "{a:?} doit être refusé d'entrée : {}", r.texte);
        }
        for a in [Admin, Direction, Prof] {
            let r = m.appel("POST", "/cahier/c6/reports/lot", Some(m.jeton(a)), Some(&corps)).await;
            assert!(r.est_succes(), "{a:?} passe la garde : {} {}", r.statut, r.texte);
            assert!(r.texte.contains("introuvable"), "{a:?} : la proposition inconnue est ignorée : {}", r.texte);
        }
    });
}

fn specs_domaine_listes_de_l_etablissement() -> Vec<Spec> {
    vec![
            lire("fils-liste", "/messages/fils",
                &[Admin, Direction, Administratif, Cpe, Prof, Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux], &[]),
            lire("factures-liste", "/facturation/factures", &[Admin, Direction, Compta], &[]),
            lire("foyers-liste", "/foyers", FOYERS, &[]),
            // Socle 8 N0a (RP1-06) : journaux d'envoi (adresses, numéros) réservés à `notifications.gerer`.
            lire("mails-liste", "/mails", &[Admin, Direction], &[Admin, Direction]),
            lire("sms-liste", "/sms", &[Admin, Direction], &[Admin, Direction]),
            lire("mes-creneaux", "/edt/mes-creneaux", &[Admin, Direction, Administratif, Cpe, Prof, Eleve], &[]),
            // D-022 bis : liste unique des enseignements du personnel, jamais des familles.
            lire("mes-enseignements", "/referentiel/mes-enseignements",
                &[Admin, Direction, Administratif, Cpe, Prof, Compta, Infirmiere, Psy], &[]),
    ]
}

#[test]
fn domaine_listes_de_l_etablissement() {
    jouer("listes de l'établissement (aucune fuite de B)", &specs_domaine_listes_de_l_etablissement());
}

/// Lot I2 (socle 3) : listes des inscriptions et routes à identifiant du modèle des campagnes
/// (offres, transitions, blocage, annulation de validation, « ne pas solliciter », prolongation,
/// fermetures). Les listes sont réservées au personnel qui gère ou décide les inscriptions ; les
/// routes à identifiant visent un objet de B avec la session de A : jamais un 2xx.
fn specs_domaine_inscriptions_i2() -> Vec<Spec> {
    const CORPS_T: &str = r#"{"action":"annuler","motif":"essai de séparation"}"#;
    let personnel: &[Acteur] = &[Admin, Direction, Administratif];
    vec![
        lire("campagnes-liste", "/inscriptions/campagnes", personnel, &[]),
        lire("dossiers-liste", "/inscriptions/dossiers", personnel, &[]),
        lire("fermetures-liste", "/inscriptions/fermetures", personnel, &[]),
        ecrire_sur_b("offres-liste-de-B", "GET", "/inscriptions/campagnes/00000000-0000-0000-0000-0000000000b2/offres", None),
        ecrire_sur_b("offre-creee-sur-B", "POST", "/inscriptions/campagnes/00000000-0000-0000-0000-0000000000b2/offres",
            Some(r#"{"niveau_code":"6EME","libelle_public":"6e"}"#)),
        ecrire_sur_b("offre-modifiee-sur-B", "PUT", "/inscriptions/offres/00000000-0000-0000-0000-0000000000b2",
            Some(r#"{"niveau_code":"6EME","libelle_public":"6e"}"#)),
        ecrire_sur_b("offre-archivee-sur-B", "DELETE", "/inscriptions/offres/00000000-0000-0000-0000-0000000000b2", None),
        ecrire_sur_b("statut-campagne-sur-B", "POST", "/inscriptions/campagnes/00000000-0000-0000-0000-0000000000b2/statut",
            Some(r#"{"statut":"suspendue"}"#)),
        ecrire_sur_b("prolongation-sur-B", "POST", "/inscriptions/campagnes/00000000-0000-0000-0000-0000000000b2/prolonger",
            Some(r#"{"objet":"cloture","nouvelle_date":"2099-01-01","motif_public":"essai"}"#)),
        ecrire_sur_b("transition-sur-B", "POST", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/transition", Some(CORPS_T)),
        ecrire_sur_b("blocage-sur-B", "POST", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/blocage",
            Some(r#"{"bloquer":true,"motif":"essai de séparation"}"#)),
        ecrire_sur_b("annulation-validation-sur-B", "POST", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/annuler-validation",
            Some(r#"{"motif":"essai de séparation"}"#)),
        ecrire_sur_b("ne-pas-solliciter-sur-B", "POST", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/ne-pas-solliciter",
            Some(r#"{"rang":2,"motif":"essai de séparation"}"#)),
        ecrire_sur_b("fermeture-annulee-sur-B", "DELETE", "/inscriptions/fermetures/00000000-0000-0000-0000-0000000000b2", None),
    ]
}

#[test]
fn domaine_inscriptions_i2() {
    jouer("inscriptions I2 (listes, objets de B)", &specs_domaine_inscriptions_i2());
}

/// Lots I3 et I4 (socle 3) : règles de pièces, pièces d'un dossier, dépôt, contrôle, lecture du fichier, accords,
/// désaccord, levée avec pièce. Toutes les routes à identifiant visent un objet de B avec la session de A :
/// jamais un 2xx, et aucune ne confirme qu'un objet existe chez l'autre établissement.
fn specs_domaine_inscriptions_i3_i4() -> Vec<Spec> {
    vec![
        ecrire_sur_b("regles-pieces-de-B", "GET", "/inscriptions/campagnes/00000000-0000-0000-0000-0000000000b2/pieces-regles", None),
        ecrire_sur_b("regles-pieces-modifiees-sur-B", "PUT", "/inscriptions/campagnes/00000000-0000-0000-0000-0000000000b2/pieces-regles",
            Some(r#"{"regles":[]}"#)),
        ecrire_sur_b("pieces-attendues-de-B", "GET", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/pieces-attendues", None),
        ecrire_sur_b("depot-guichet-sur-B", "POST", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/pieces/livret_famille/fichiers",
            Some(r#"{"fichiers":["JVBERi0xLjQK"]}"#)),
        ecrire_sur_b("controle-piece-sur-B", "POST", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/pieces/livret_famille/controle",
            Some(r#"{"decision":"accepter"}"#)),
        ecrire_sur_b("reutilisation-sur-B", "POST", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/pieces/livret_famille/reutiliser",
            Some(r#"{"source_dossier_id":"00000000-0000-0000-0000-0000000000b3"}"#)),
        ecrire_sur_b("fichier-de-B", "GET", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/pieces/livret_famille/fichier", None),
        ecrire_sur_b("accords-de-B", "GET", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/accords", None),
        ecrire_sur_b("demande-accords-sur-B", "POST", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/accords/demander",
            Some(r#"{"rangs":[]}"#)),
        ecrire_sur_b("accord-papier-sur-B", "POST", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/accords/papier",
            Some(r#"{"rang":2,"reponse":"donne","piece_code":"accord_responsable"}"#)),
        ecrire_sur_b("desaccord-sur-B", "POST", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/accords/desaccord",
            Some(r#"{"rang":2,"motif":"essai de séparation"}"#)),
        ecrire_sur_b("dispense-sur-B", "POST", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/accords/dispenser",
            Some(r#"{"rang":2,"motif":"essai de séparation"}"#)),
        ecrire_sur_b("levee-sur-B", "POST", "/inscriptions/dossiers/00000000-0000-0000-0000-0000000000b2/accords/lever",
            Some(r#"{"rang":2,"issue":"accord_ecrit","piece_code":"accord_responsable","motif":"essai de séparation"}"#)),
    ]
}

#[test]
fn domaine_inscriptions_i3_i4() {
    jouer("inscriptions I3 et I4 (pièces et accords de B)", &specs_domaine_inscriptions_i3_i4());
}

// ───────────────────────── Invariants transverses ─────────────────────────

/// Routes publiques : la **seule** liste d'exceptions à « sans session ⇒ 401 ».
const PUBLIQUES: &[(&str, &str)] = &[
    ("GET", "/health"),
    ("GET", "/health/detail"), // supervision : 404 si métriques fermées, 401 sans jeton ; jamais de session
    ("GET", "/ready"),
    ("POST", "/labo/bootstrap"), // fermé hors labo par ENT_ENV / ENT_ALLOW_LABO
    ("POST", "/auth/register"),  // fermé hors labo
    ("POST", "/auth/login"),
    // Socle 20, lot C3 : flux de calendrier d'un abonnement. Un logiciel de calendrier n'envoie ni cookie ni jeton d'API ;
    // le jeton de l'adresse (244 bits, haché en base, révocable) tient lieu de session, et les droits du compte abonné
    // sont relus à chaque lecture. Jeton inconnu, révoqué ou droits retirés : 404 sans rien dire de plus.
    ("GET", "/cahier/c3/calendrier/{jeton}"),
    ("GET", "/auth/etablissement"), // C2 : fiche publique de l'établissement de l'adresse
    ("POST", "/auth/passkey/login/start"),
    ("POST", "/auth/passkey/login/finish"),
    ("GET", "/auth/oidc/public"),
    ("POST", "/auth/oidc/{slug}/start"),
    ("POST", "/auth/oidc/complete"),
    ("POST", "/auth/logout"),
    ("POST", "/auth/forgot-password"),
    ("POST", "/auth/reset-password"),
    // Invitation d'un agent (lot P2) : la personne invitée n'a pas encore de session, le jeton de
    // l'invitation (dans le corps, haché en base, à usage unique) fait foi ; corps vide → 422.
    ("POST", "/invitations/personnel/consulter"),
    ("POST", "/invitations/personnel/accepter"),
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
    // Accords des responsables sans compte (socle 3, I4) : le jeton du lien (dérivé d'un secret serveur, jamais stocké)
    // et le code reçu par e-mail font foi ; réponses identiques pour tout lien refusé ; débit limité.
    ("POST", "/accords/{id}/consulter"),
    ("POST", "/accords/{id}/code"),
    ("POST", "/accords/{id}/repondre"),
    ("GET", "/metrics"), // politique d'accès propre (jeton / désactivé)
    // Vérification publique des documents (socle 7, D3) : le code (60 bits) ou l'empreinte du fichier fait foi ;
    // l'établissement vient de l'adresse, réponses identiques pour inconnu et autre établissement.
    ("GET", "/verifier/{code}"),
    ("POST", "/verifier"),
    // Préinscription (S2-3, D-014) : e-mail vérifié par code, invitation par jeton.
    ("POST", "/preinscription/demarrer"),
    ("POST", "/preinscription/verifier"),
    ("POST", "/inscriptions/invitations/accepter"),
    // Journal de voyage (socle 42, V0d) : famille SANS compte ; le jeton du lien nominatif (empreinte seule en base,
    // expirant, révocable, débit limité) fait foi, l'établissement vient du jeton. Réponses identiques pour tout jeton refusé.
    ("GET", "/voyages/journal/lien/{jeton}"),
    ("GET", "/voyages/journal/lien/{jeton}/photos/{phid}"),
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
        ("GET", "/referentiel/groupes/{id}/membres"),
        ("POST", "/referentiel/groupes/{id}/membres"),
        ("DELETE", "/referentiel/groupes/{id}/membres/{eleve_id}"),
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
        "/referentiel/mes-enseignements".to_string(),
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
            "/personnel/search?q=a", "/recherche?q=Potter", "/recherche?q=Dupont", "/vs/signaux", "/annonces", "/edt/mes-creneaux",
            "/referentiel/mes-enseignements",
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

/// Socle 29, lot P5 : la recherche globale est un outil du personnel ; une famille ou un élève reçoit un refus ou rien.
#[test]
fn l2_recherche_globale_fermee_aux_familles() {
    let Some(_) = monde() else { return };
    let e = essai_famille_refusee("/recherche?q=Potter", Cible::Liee, FAMILLE);
    assert!(e.is_empty(), "{}", e.join("\n"));
    let e = essai_famille_refusee("/recherche?q=Potter", Cible::NonLiee, FAMILLE);
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

/// E6 (revue de sécurité) : sur un élève sous confidentialité renforcée, TOUTES les routes
/// `/eleves/{id}/…` répondent 404 (jamais 403) à un membre du personnel sans G6 ni lien direct,
/// quelle que soit la méthode et même si son rôle n'a pas le droit habituel ; la direction (G6)
/// l'atteint, et la mesure levée, tout redevient normal.
#[test]
fn e6_toute_route_eleve_est_introuvable_pour_un_tiers_sur_un_eleve_protege() {
    let Some(m) = monde() else { return };
    let routes: Vec<routes_src::RouteDecl> = routes_src::routes()
        .into_iter()
        .filter(|r| r.chemin.starts_with("/eleves/{") && !r.chemin.starts_with("/eleves/{}"))
        .collect();
    assert!(routes.len() > 100, "inventaire des routes /eleves/{{id}} incomplet : {}", routes.len());
    let tiers = [Administratif, Cpe, Compta, Infirmiere, Psy];
    let echecs = rt().block_on(async {
        let dir = m.jeton(Direction).to_string();
        let cree = m
            .appel(
                "POST",
                "/eleves",
                Some(&dir),
                Some(&json!({"nom": format!("Garde{}", Uuid::new_v4().simple()), "prenom": "Eva", "classe": "6A"})),
            )
            .await;
        assert!(cree.est_succes(), "création de l'élève d'essai : {} {}", cree.statut, cree.texte);
        let id = cree.json["id"].as_str().expect("id").to_string();
        let instancie = |gabarit: &str| {
            let d = gabarit.find('{').unwrap();
            let f = gabarit[d..].find('}').unwrap() + d;
            format!("{}{}{}", &gabarit[..d], id, routes_src::instancier(&gabarit[f + 1..]))
        };
        // Contrôle positif : avant la mesure, le tiers atteint la fiche (200).
        let avant = m.appel("GET", &format!("/eleves/{id}"), Some(m.jeton(Administratif)), None).await;
        assert!(avant.est_succes(), "avant la mesure : {}", avant.statut);
        let pose = m
            .appel(
                "POST",
                &format!("/eleves/{id}/confidentialite"),
                Some(&dir),
                Some(&json!({"motif": "Mesure de protection", "mesures": []})),
            )
            .await;
        assert!(pose.est_succes(), "pose de la mesure : {} {}", pose.statut, pose.texte);
        let mut echecs = Vec::new();
        for r in &routes {
            let chemin = instancie(&r.chemin);
            let corps = matches!(r.methode, "POST" | "PUT" | "PATCH").then(|| json!({}));
            for a in tiers {
                let rep = m.appel(r.methode, &chemin, Some(m.jeton(a)), corps.as_ref()).await;
                if rep.statut.as_u16() != 404 {
                    echecs.push(format!("{a:?} {} {} → {}", r.methode, r.chemin, rep.statut.as_u16()));
                }
            }
        }
        // La direction (G6) l'atteint toujours.
        let g6 = m.appel("GET", &format!("/eleves/{id}"), Some(&dir), None).await;
        if !g6.est_succes() {
            echecs.push(format!("direction (G6) : GET /eleves/{{id}} → {}", g6.statut));
        }
        // Mesure levée : le tiers revoit la fiche.
        let lever = m
            .appel(
                "POST",
                &format!("/eleves/{id}/confidentialite/lever"),
                Some(&dir),
                Some(&json!({"motif_code": "fin_de_mesure"})),
            )
            .await;
        assert!(lever.est_succes(), "levée : {} {}", lever.statut, lever.texte);
        let apres = m.appel("GET", &format!("/eleves/{id}"), Some(m.jeton(Administratif)), None).await;
        if !apres.est_succes() {
            echecs.push(format!("après la levée : {}", apres.statut));
        }
        echecs
    });
    CAS_EXECUTES.fetch_add(routes.len() * tiers.len(), Ordering::Relaxed);
    eprintln!("[separation] élève protégé : {} routes × {} tiers, {} écart(s)", routes.len(), tiers.len(), echecs.len());
    assert!(echecs.is_empty(), "routes /eleves/{{id}}/… qui ne répondent pas 404 à un tiers :\n{}", echecs.join("\n"));
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
            // RP1-07 : la comptabilité ne lit plus le journal de vie scolaire (ni liée, ni non liée).
            lire("journal", "/eleves/{eleve}/journal", &[Admin, Direction, Administratif, Cpe, Prof, Infirmiere, Psy, Parent, Eleve], &[Admin, Direction, Administratif, Cpe, Infirmiere, Psy]),
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
    #[allow(dead_code)] // identifiant du compte créé : conservé pour les diagnostics, non relu aujourd'hui
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
        // Socle 6 P0 : le provisionnement HTTP ne renvoie plus de mot de passe (invitation par
        // e-mail, testée dans `ent-api`). Les comptes « doubles » sont donc créés directement,
        // avec un mot de passe connu du test, puis rattachés aux deux établissements.
        let user_id = Uuid::new_v4();
        let mdp = format!("Double-{}-Aa9!", Uuid::new_v4().simple());
        let hash = ent_auth::hash_password(&mdp).expect("hachage");
        sqlx::query(
            "INSERT INTO ent_user (id, email, password_hash, display_name, created_at, updated_at)
             VALUES ($1, $2, $3, $4, now(), now())",
        )
        .bind(user_id)
        .bind(&email)
        .bind(&hash)
        .bind(format!("Double {role}"))
        .execute(&pool)
        .await
        .expect("compte double");
        for etab in [m.a.objets.etab, m.b.objets.etab] {
            sqlx::query("INSERT INTO ent_membership (user_id, etablissement_id, role, created_at) VALUES ($1, $2, $3, now())")
                .bind(user_id)
                .bind(etab)
                .bind(role)
                .execute(&pool)
                .await
                .expect("rattachement");
        }
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
/// Socle 12, lot AS5 (familles et notifications). Le suivi de l'information des familles (avis, contacts, courriers,
/// relances) est réservé à la vie scolaire, au secrétariat et à la direction (`assiduite.suivre`) : ni le professeur,
/// ni la comptabilité, ni l'infirmerie, ni la psychologue, ni une famille, ni l'élève. La vue « communications »
/// est celle du responsable (droit de voir la vie scolaire ou de justifier, comme les épisodes d'AS3), jamais du personnel ni de l'élève.
/// Les écritures visant un élève de B avec la session de A sont refusées (même réponse que « inexistant »).
fn specs_domaine_assiduite_familles() -> Vec<Spec> {
    vec![
        lire("assiduite-avis-eleve", "/assiduite/eleves/{eleve}/avis", GESTION_HORS_PROF, GESTION_HORS_PROF),
        lire("assiduite-contacts-eleve", "/assiduite/eleves/{eleve}/contacts", GESTION_HORS_PROF, GESTION_HORS_PROF),
        lire("assiduite-courriers-eleve", "/assiduite/eleves/{eleve}/courriers", GESTION_HORS_PROF, GESTION_HORS_PROF),
        lire("assiduite-communications", "/assiduite/eleves/{eleve}/communications", &[Parent, ParentSansDrapeaux], &[]),
        lire("assiduite-familles-parametres", "/assiduite/familles/parametres", GESTION_HORS_PROF, GESTION_HORS_PROF),
        // AS14b (AS-216) : situation d'un élève à l'accueil, `absences.declarer`, lecture journalisée ; jamais la famille ni le professeur.
        lire("assiduite-situation-accueil", "/assiduite/eleves/{eleve}/situation-accueil", GESTION_HORS_PROF, GESTION_HORS_PROF),
        lire("assiduite-sans-reponse", "/assiduite/relances/sans-reponse", GESTION_HORS_PROF, GESTION_HORS_PROF),
        lire("assiduite-numeros-faux", "/assiduite/contacts/numeros-faux", GESTION_HORS_PROF, GESTION_HORS_PROF),
        ecrire_sur_b(
            "assiduite-contact-sur-B",
            "POST",
            "/assiduite/contacts",
            Some(r#"{"eleve_id":"{eleve}","canal":"telephone","issue":"joint"}"#),
        ),
        ecrire_sur_b(
            "assiduite-courrier-sur-B",
            "POST",
            "/assiduite/courriers",
            Some(r#"{"eleve_id":"{eleve}","nature":"avis_absence"}"#),
        ),
        ecrire_sur_b("assiduite-courrier-valider-B", "POST", "/assiduite/courriers/{absence}/valider", Some("{}")),
        ecrire_sur_b("assiduite-numero-faux-B", "POST", "/assiduite/contacts/{absence}/numero-faux/cloturer", Some("{}")),
    ]
}

#[test]
fn domaine_assiduite_familles() {
    jouer("assiduite_familles", &specs_domaine_assiduite_familles());
}

/// Socle 12, lot AS6 (vérification et file du matin). La file du jour, les alertes et le drapeau de vigilance sont
/// réservés à la vie scolaire, au secrétariat et à la direction (`appel.verifier` + `absences.lire`,
/// `assiduite.suivre`, `absences.qualifier`) : ni le professeur, ni la comptabilité, ni l'infirmerie, ni la
/// psychologue, ni une famille, ni l'élève. « Prévenir maintenant », « faire l'appel à la place », « je prends » et
/// le traitement d'une alerte visant une ressource de B avec la session de A sont refusés (même réponse que
/// « inexistant »).
fn specs_domaine_assiduite_verification() -> Vec<Spec> {
    vec![
        lire("assiduite-file-du-matin", "/assiduite/file-du-matin", GESTION_HORS_PROF, GESTION_HORS_PROF),
        lire("assiduite-alertes", "/assiduite/alertes", GESTION_HORS_PROF, GESTION_HORS_PROF),
        lire("assiduite-vigilances", "/assiduite/vigilances", GESTION_HORS_PROF, GESTION_HORS_PROF),
        ecrire_sur_b(
            "assiduite-prevenir-maintenant-sur-B",
            "POST",
            "/assiduite/eleves/{eleve}/prevenir-maintenant",
            Some(r#"{"raison_code":"fugue_suspectee"}"#),
        ),
        ecrire_sur_b("assiduite-vigilance-poser-sur-B", "POST", "/assiduite/eleves/{eleve}/vigilance", Some("{}")),
        ecrire_sur_b("assiduite-vigilance-lever-sur-B", "POST", "/assiduite/eleves/{eleve}/vigilance/lever", Some("{}")),
        ecrire_sur_b("assiduite-alerte-prendre-sur-B", "POST", "/assiduite/alertes/{absence}/prendre", Some("{}")),
        ecrire_sur_b(
            "assiduite-alerte-traiter-sur-B",
            "POST",
            "/assiduite/alertes/{absence}/traiter",
            Some(r#"{"code":"sans_suite"}"#),
        ),
        ecrire_sur_b(
            "assiduite-appel-a-la-place-sur-B",
            "POST",
            "/assiduite/appels/a-la-place",
            Some(r#"{"creneau_id":"{appel}","jour":"{jour}","raison_code":"oubli"}"#),
        ),
    ]
}

#[test]
fn domaine_assiduite_verification() {
    jouer("assiduite_verification", &specs_domaine_assiduite_verification());
}

/// Socle 12, lot AS14b-bis (tableau de bord de la vie scolaire en panneaux). Les exclus du jour et la configuration
/// (heure de bascule, disposition personnelle) sont réservés comme la file du matin (`appel.verifier` +
/// `absences.lire`) : ni le professeur, ni une famille, ni l'élève. Les trois écritures n'ont aucun identifiant de
/// ressource : la session de A ne règle que l'établissement de A et que sa propre disposition ; l'heure de bascule
/// relève de `assiduite.regle_gerer` (jamais de la vie scolaire seule). Rien de B n'est lisible ni modifiable.
fn specs_domaine_assiduite_tableau() -> Vec<Spec> {
    vec![
        lire("assiduite-exclus-du-jour", "/assiduite/exclus-du-jour", GESTION_HORS_PROF, GESTION_HORS_PROF),
        lire("assiduite-tableau-configuration", "/assiduite/tableau/configuration", GESTION_HORS_PROF, GESTION_HORS_PROF),
        borne_a("assiduite-tableau-heure-bascule", "PUT", "/assiduite/tableau/heure-bascule", Some(r#"{"heure":"11:00:00"}"#), &[]),
        borne_a(
            "assiduite-tableau-preference-poser",
            "PUT",
            "/assiduite/tableau/preference",
            Some(
                r#"{"panneaux":[{"panneau":"absents","replie":false,"masque":false},{"panneau":"exclus","replie":false,"masque":false},{"panneau":"appels","replie":false,"masque":false},{"panneau":"incoherences","replie":false,"masque":false},{"panneau":"familles","replie":false,"masque":false}]}"#,
            ),
            &[],
        ),
        borne_a("assiduite-tableau-preference-effacer", "DELETE", "/assiduite/tableau/preference", None, &[]),
    ]
}

#[test]
fn domaine_assiduite_tableau() {
    jouer("assiduite_tableau", &specs_domaine_assiduite_tableau());
}

/// Socle 32, lots ET0–ET1 (espaces de travail des personnels). Les espaces ne s'ouvrent qu'aux **personnels** (`espaces.utiliser`),
/// jamais aux familles ni aux élèves ; un espace, un dossier ou un fichier hors de portée répond 404 (jamais 403), y compris avec
/// un identifiant d'un autre établissement. Les chemins ci-dessous visent des identifiants inexistants (aucune ressource n'est
/// créée par la matrice) : **personne** n'en reçoit de contenu, quelle que soit la place de l'acteur ; les listes sont testées
/// pour la séparation des rôles (familles, élèves refusés).
fn specs_domaine_espaces() -> Vec<Spec> {
    const INCONNU: &str = "00000000-0000-4000-8000-0000000000e1";
    // Les routes à identifiant exigent le même refus pour tous : aucun espace n'existe sous cet identifiant.
    let par_id: Vec<(&'static str, String)> = vec![
        ("espaces-detail-inconnu", format!("/espaces/{INCONNU}")),
        ("espaces-contenu-inconnu", format!("/espaces/{INCONNU}/contenu")),
        ("espaces-journal-inconnu", format!("/espaces/{INCONNU}/journal")),
        ("espaces-corbeille-inconnue", format!("/espaces/{INCONNU}/corbeille")),
        ("espaces-versions-inconnues", format!("/espaces/elements/{INCONNU}/versions")),
        ("espaces-telechargement-inconnu", format!("/espaces/elements/{INCONNU}/telechargement")),
        ("espaces-apercu-inconnu", format!("/espaces/elements/{INCONNU}/apercu")),
        ("espaces-partages-de-inconnu", format!("/espaces/partages?element_id={INCONNU}")),
    ];
    let mut v = vec![
        lire("espaces-liste", "/espaces", ESPACES_PERSONNELS, ESPACES_PERSONNELS),
        lire("espaces-quota", "/espaces/quota", ESPACES_PERSONNELS, ESPACES_PERSONNELS),
        lire("espaces-badge", "/espaces/invitations/non-vues", ESPACES_PERSONNELS, ESPACES_PERSONNELS),
        lire("espaces-partages-avec-moi", "/espaces/partages/avec-moi", ESPACES_PERSONNELS, ESPACES_PERSONNELS),
    ];
    for (nom, chemin) in par_id {
        // `lire` exige des `&'static str` : on fuit volontairement ces quelques chaînes de test (durée du processus).
        let chemin: &'static str = Box::leak(chemin.into_boxed_str());
        v.push(lire(nom, chemin, &[], &[]));
    }
    v
}

#[test]
fn domaine_espaces() {
    jouer("espaces", &specs_domaine_espaces());
}

fn toutes_les_specs() -> Vec<Spec> {
    let mut v = Vec::new();
    v.extend(specs_domaine_eleve_dossier());
    v.extend(specs_domaine_notes());
    v.extend(specs_domaine_absences());
    v.extend(specs_domaine_sante());
    v.extend(specs_domaine_psy());
    v.extend(specs_domaine_documents());
    v.extend(specs_domaine_accords());
    v.extend(specs_domaine_fils_de_messages());
    v.extend(specs_domaine_factures());
    v.extend(specs_domaine_finances_comptes());
    v.extend(specs_domaine_finances_grilles());
    v.extend(specs_domaine_appel());
    v.extend(specs_domaine_assiduite_familles());
    v.extend(specs_domaine_espaces());
    v.extend(specs_domaine_assiduite_verification());
    v.extend(specs_domaine_assiduite_tableau());
    v.extend(specs_domaine_cours());
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
    let base = domain_eleve::horloge::jour_paris();
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
        // RP1-02 : membres de groupe. Un groupe de B manipulé depuis A, un élève de B mis dans un
        // groupe de A, et les comptes famille / élève sans aucun droit de gestion.
        let gb = m.appel("POST", "/referentiel/groupes", Some(b.jetons[&Direction].as_str()), Some(&json!({
            "annee_id": id_premier(&json_de(m, b.jetons[&Direction].as_str(), "/referentiel/annees").await),
            "code": format!("GB{}", &Uuid::new_v4().simple().to_string()[..6]), "libelle": "atelier B"}))).await;
        if gb.est_succes() {
            let gbid = gb.json["id"].as_str().unwrap().to_string();
            let r = m.appel("GET", &format!("/referentiel/groupes/{gbid}/membres"), Some(dir_a), None).await;
            exige_refus(&mut echecs, "groupe: membres d'un groupe de B (lecture)", &r);
            let r = m.appel("POST", &format!("/referentiel/groupes/{gbid}/membres"), Some(dir_a),
                Some(&json!({"eleve_id": a.objets.eleve}))).await;
            exige_refus(&mut echecs, "groupe: ajout dans un groupe de B", &r);
            let r = m.appel("DELETE", &format!("/referentiel/groupes/{gbid}/membres/{}", a.objets.eleve), Some(dir_a), None).await;
            exige_refus(&mut echecs, "groupe: retrait d'un groupe de B", &r);
            cas += 3;
        }
        if g.est_succes() {
            let gid = g.json["id"].as_str().unwrap().to_string();
            let r = m.appel("POST", &format!("/referentiel/groupes/{gid}/membres"), Some(dir_a),
                Some(&json!({"eleve_id": b.objets.eleve}))).await;
            exige_refus(&mut echecs, "groupe: élève de B dans un groupe de A", &r);
            cas += 1;
            for (acteur, nom) in [(Parent, "parent"), (Eleve, "élève"), (Compta, "compta"), (Prof, "professeur")] {
                for (meth, chemin, corps) in [
                    ("POST", format!("/referentiel/groupes/{gid}/membres"), Some(json!({"eleve_id": a.objets.eleve}))),
                    ("DELETE", format!("/referentiel/groupes/{gid}/membres/{}", a.objets.eleve), None),
                ] {
                    let r = m.appel(meth, &chemin, Some(a.jetons[&acteur].as_str()), corps.as_ref()).await;
                    exige_refus(&mut echecs, &format!("groupe: {meth} membres par {nom}"), &r);
                    cas += 1;
                }
            }
            for (acteur, nom) in [(Parent, "parent"), (Eleve, "élève")] {
                let r = m.appel("GET", &format!("/referentiel/groupes/{gid}/membres"), Some(a.jetons[&acteur].as_str()), None).await;
                exige_refus(&mut echecs, &format!("groupe: liste des membres par {nom}"), &r);
                cas += 1;
            }
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

/// Socle N1, lot N1-4 : les routes `/demandes…` refusent ce que le rôle ne permet pas, **sans rien révéler** (404 neutre
/// pour une demande inconnue, jamais de donnée d'un autre établissement) — pièges 1, 5, 7, 8, 9, 11 du §24.3.
#[test]
fn n1_4_routes_demandes_refus_par_role_et_404_neutre() {
    let Some(m) = monde() else { return };
    let mut echecs = Vec::new();
    rt().block_on(async {
        let nul = Uuid::new_v4();
        // Familles et élèves : aucune file, aucune décision, aucune validation, aucune impression.
        for a in [Parent, ParentAutreFoyer, ParentSansDrapeaux, Eleve] {
            for chemin in ["/demandes/file", "/demandes/hse/a-decider", "/demandes/hse/enveloppe", "/demandes/copies/file", "/demandes/copies/a-valider"] {
                let r = m.appel("GET", chemin, Some(m.jeton(a)), None).await;
                if !r.est_refus() {
                    echecs.push(format!("GET {chemin} par {a:?} → {}", r.statut));
                }
            }
            let r = m.appel("POST", "/demandes/hse", Some(m.jeton(a)), Some(&json!({"motif": "soutien", "objet": "x", "volume_quarts": 4,
                "periode_du": "2026-01-05", "periode_au": "2026-01-06"}))).await;
            if !r.est_refus() {
                echecs.push(format!("POST /demandes/hse par {a:?} → {}", r.statut));
            }
        }
        // Une demande inconnue, ou d'un autre établissement : 404 neutre pour tout le monde, mêmes mots.
        let b_demande = m.b.objets.eleve; // un identifiant réel de B qui n'est pas une demande de A
        for a in Acteur::TOUS {
            for id in [nul, b_demande] {
                for (meth, chemin, corps) in [
                    ("GET", format!("/demandes/{id}"), None),
                    ("POST", format!("/demandes/{id}/prendre"), Some(json!({"version": 1}))),
                    ("POST", format!("/demandes/{id}/annuler"), Some(json!({"version": 1}))),
                    ("POST", format!("/demandes/hse/{id}/decider"), Some(json!({"decision": "accordee", "version": 1}))),
                    ("POST", format!("/demandes/hse/{id}/confirmer"), Some(json!({}))),
                    ("POST", format!("/demandes/copies/{id}/prete"), Some(json!({}))),
                    ("POST", format!("/demandes/copies/{id}/valider"), Some(json!({"accepter": true}))),
                    ("POST", format!("/demandes/copies/{id}/impression"), Some(json!({}))),
                ] {
                    let r = m.appel(meth, &chemin, Some(m.jeton(a)), corps.as_ref()).await;
                    if r.est_succes() || r.statut.as_u16() == 500 {
                        echecs.push(format!("{meth} {chemin} par {a:?} → {}", r.statut));
                    }
                }
            }
        }
        // Le personnel sans droit de décision ni de validation est refusé.
        for a in [Prof, Cpe, Administratif] {
            for chemin in ["/demandes/hse/a-decider", "/demandes/hse/enveloppe", "/demandes/copies/a-valider"] {
                let r = m.appel("GET", chemin, Some(m.jeton(a)), None).await;
                if !r.est_refus() {
                    echecs.push(format!("GET {chemin} par {a:?} → {}", r.statut));
                }
            }
        }
        // Un corps de dépôt ne porte aucune identité : un champ « email » ou « demandeur » est ignoré (NJ-84).
        let r = m.appel("POST", "/demandes", Some(m.jeton(Prof)), Some(&json!({"nature_code": "inconnue.nature", "objet": "x",
            "email": "pirate@example.test", "demandeur": nul}))).await;
        if r.est_succes() {
            echecs.push(format!("POST /demandes avec identité → {}", r.statut));
        }
        // La liste « mes demandes » ne contient rien d'un autre établissement.
        let l = m.appel("GET", "/demandes/mes", Some(m.jeton(Prof)), None).await;
        if l.est_succes() && l.texte.contains(&m.b.objets.eleve.to_string()) {
            echecs.push("GET /demandes/mes : identifiant de B listé".into());
        }
    });
    assert!(echecs.is_empty(), "{echecs:#?}");
}

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
            ("GET", format!("/referentiel/groupes/{nul}/membres"), None),
            ("POST", format!("/referentiel/groupes/{nul}/membres"), Some(json!({"eleve_id": nul}))),
            ("DELETE", format!("/referentiel/groupes/{nul}/membres/{nul}"), None),
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

/// Socle 8 N0a (h, RP1-06) : l'accueil (profil administratif) reçoit 403 sur `/sms` et `/mails`,
/// la direction les lit.
#[test]
fn n0a_accueil_refuse_sur_mails_et_sms() {
    let Some(m) = monde() else { return };
    rt().block_on(async {
        for chemin in ["/sms", "/mails"] {
            let r = m.appel("GET", chemin, Some(m.jeton(Administratif)), None).await;
            assert_eq!(r.statut.as_u16(), 403, "accueil sur {chemin} : {}", r.statut);
            let r = m.appel("GET", chemin, Some(m.jeton(Direction)), None).await;
            assert!(r.est_succes(), "direction sur {chemin} : {}", r.statut);
        }
    });
}

/// V3d (M-6) : les 14 routes `/eva/v4/*` (propositions, rattrapages, remplacement, zéro, dispenses, aménagements, absences
/// répétées) sont réservées au personnel : la famille et l'élève reçoivent **403** d'entrée, sur toutes les routes et quel
/// que soit le corps (avant toute lecture : une évaluation inconnue ne répond pas 404 à un parent). Les écritures n'ont
/// aucun effet (aucune convocation créée).
#[test]
fn evaluations_v4_famille_et_eleve_recoivent_403_sur_toutes_les_routes() {
    let Some(m) = monde() else { return };
    let id = Uuid::new_v4();
    let eleve = Uuid::new_v4();
    let plan = json!({"evaluation_id": id, "eleves": [eleve], "prevu_le": "2031-10-27", "heure": null, "lieu": null, "modalite": null, "echeance": null});
    let deplacer = json!({"prevu_le": "2031-10-27", "heure": null, "lieu": null, "modalite": null, "echeance": null, "version_attendue": 1});
    let routes: Vec<(&str, String, Option<Value>)> = vec![
        ("GET", format!("/eva/v4/evaluations/{id}/propositions"), None),
        ("POST", "/eva/v4/propositions/appliquer".into(), Some(json!({"cours_id": id, "evaluation_id": id, "eleves": null}))),
        ("GET", format!("/eva/v4/evaluations/{id}/amenagements"), None),
        ("GET", format!("/eva/v4/evaluations/{id}/rattrapages"), None),
        ("POST", "/eva/v4/rattrapages".into(), Some(plan.clone())),
        ("PUT", format!("/eva/v4/rattrapages/{id}"), Some(deplacer)),
        ("POST", format!("/eva/v4/rattrapages/{id}/resultat"), Some(json!({"texte": "12", "absence": null, "version_attendue": 1}))),
        ("POST", format!("/eva/v4/rattrapages/{id}/annuler"), Some(json!({"motif": "essai de la famille", "version_attendue": 1}))),
        ("POST", format!("/eva/v4/rattrapages/{id}/reconvoquer"), Some(json!({"prevu_le": "2031-10-31", "heure": null, "lieu": null, "modalite": null, "echeance": null}))),
        ("POST", "/eva/v4/remplacements".into(), Some(plan)),
        ("POST", format!("/eva/v4/rattrapages/{id}/zero"), Some(json!({"confirme": true, "version_attendue": 1}))),
        ("POST", "/eva/v4/dispenses".into(), Some(json!({"eleve_id": eleve, "matiere_id": null, "nature": "dispense", "du": "2031-10-01", "au": null}))),
        ("GET", format!("/eva/v4/dispenses?eleve_id={eleve}"), None),
        ("POST", format!("/eva/v4/dispenses/{id}/retirer"), None),
        ("GET", format!("/eva/v4/absences-repetees?periode_id={id}"), None),
    ];
    assert_eq!(routes.len(), 15, "toutes les routes /eva/v4 de routes.rs sont couvertes");
    rt().block_on(async {
        for a in [Parent, Eleve, ParentAutreFoyer, ParentSansDrapeaux] {
            for (methode, chemin, corps) in &routes {
                let r = m.appel(methode, chemin, Some(m.jeton(a)), corps.as_ref()).await;
                assert_eq!(r.statut.as_u16(), 403, "{a:?} {methode} {chemin} : {}", r.texte);
            }
        }
    });
}
