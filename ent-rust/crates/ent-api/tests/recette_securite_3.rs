//! Recette de sécurité n° 3 — cloisonnement multi-établissements (multitenant).
//!
//! On joue des **attaquants réels** contre le **logiciel réel** : le routeur
//! `ent_api::app` est appelé en process (`tower::ServiceExt::oneshot`, mêmes
//! couches, mêmes extracteurs, même base) sur un PostgreSQL **local de test**,
//! avec deux établissements de labo A et B créés par `POST /labo/bootstrap`.
//!
//! Priorité absolue : un acteur de A (quel que soit son rôle) ne doit jamais
//! lire ni modifier quoi que ce soit de B. On réutilise le harnais de la
//! matrice de séparation (`separation/harness.rs` : A/B, acteurs, oracle de
//! fuite) et l'inventaire de routes extrait du source (`separation/routes_src.rs`).
//!
//! Contenu :
//!  - `matrice_cloisonnement_toutes_routes` : **générique**, parcourt TOUTES les
//!    routes GET à paramètre et y injecte les identifiants de B, avec un acteur
//!    de A ; toute nouvelle route est donc couverte automatiquement (RS3, but
//!    « matrice multi-établissements »).
//!  - des tests ciblés par faille (`rs3_*`) : vert si ça tient ; pour une faille
//!    ouverte, un test `#[ignore = "FAILLE OUVERTE RS3-xx …"]` qui échoue tant
//!    qu'elle existe (`cargo test … -- --ignored`).
//!
//! Lancement :
//!   service postgresql start
//!   ENT_DATABASE_URL='postgres://ent_ci:ci@127.0.0.1:5432/ent_ci_test' ENT_ENV=test \
//!   RUST_MIN_STACK=16777216 cargo test --locked -p ent-api --features ent-api/labo \
//!   --test recette_securite_3 -- --test-threads=1
//! Sans `ENT_DATABASE_URL` : tests ignorés proprement (« SKIP »).

#![allow(dead_code)]

#[path = "separation/harness.rs"]
mod harness;
#[path = "separation/routes_src.rs"]
mod routes_src;

use harness::*;
use serde_json::json;
use uuid::Uuid;

use Acteur::*;

/// Champs qu'aucune réponse ne doit jamais contenir (secrets, clés d'objets).
const INTERDITS_GLOBAUX: &[&str] = &["storage_key", "password_hash", "argon2"];

/// Routes publiques (pas de session) — mêmes que l'inventaire de `separation`.
/// On ne les éprouve pas pour le cloisonnement (elles n'ouvrent pas de données
/// d'un établissement par identifiant).
const PUBLIQUES: &[(&str, &str)] = &[
    ("GET", "/health"),
    ("GET", "/ready"),
    ("GET", "/metrics"),
    ("POST", "/labo/bootstrap"),
    ("POST", "/auth/register"),
    ("POST", "/auth/login"),
    ("GET", "/auth/etablissement"),
    ("POST", "/auth/passkey/login/start"),
    ("POST", "/auth/passkey/login/finish"),
    ("GET", "/auth/oidc/public"),
    ("POST", "/auth/oidc/{slug}/start"),
    ("POST", "/auth/oidc/complete"),
    ("POST", "/auth/logout"),
    ("POST", "/auth/forgot-password"),
    ("POST", "/auth/reset-password"),
    ("POST", "/auth/email/confirmer"),
    ("POST", "/auth/email/annuler"),
    ("POST", "/auth/passkey/recuperation"),
    ("POST", "/auth/passkey/recuperation/confirmer"),
    ("POST", "/auth/passkey/recuperation/annuler"),
    ("POST", "/auth/passkey/recuperation/finaliser"),
    ("POST", "/preinscription/demarrer"),
    ("POST", "/preinscription/verifier"),
    ("POST", "/inscriptions/invitations/accepter"),
    ("POST", "/inscriptions/invitations/refuser"),
    ("GET", "/fichiers/{jeton}"),
];
const HANDLERS_PUBLICS: &[&str] = &["ui_page"];

fn est_publique(r: &routes_src::RouteDecl) -> bool {
    PUBLIQUES.iter().any(|(m, p)| r.methode == *m && r.chemin == *p)
        || HANDLERS_PUBLICS.contains(&r.handler.as_str())
}

/// Tous les identifiants « réels » de B connus du harnais, pour les injecter à
/// la place d'un `{id}` dans une route GET. On ajoute aussi l'identifiant d'un
/// utilisateur (membre) de B : beaucoup de routes prennent un `user_id`.
fn identifiants_de_b(m: &Monde) -> Vec<(&'static str, Uuid)> {
    let o = &m.b.objets;
    let mut v: Vec<(&'static str, Uuid)> = vec![
        ("etablissement", o.etab),
        ("eleve", o.eleve),
        ("eleve_autre", o.eleve_autre),
        ("foyer", o.foyer),
        ("foyer_autre", o.foyer_autre),
    ];
    if let Some(x) = o.doc {
        v.push(("document", x));
    }
    if let Some(x) = o.fil {
        v.push(("fil", x));
    }
    if let Some(x) = o.facture {
        v.push(("facture", x));
    }
    if let Some(x) = o.absence {
        v.push(("absence", x));
    }
    if let Some(x) = o.matiere {
        v.push(("matiere", x));
    }
    if let Some(x) = o.appel {
        v.push(("appel", x));
    }
    if let Some(u) = m.b.user_ids.get(&Direction) {
        v.push(("user_direction", *u));
    }
    if let Some(u) = m.b.user_ids.get(&Eleve) {
        v.push(("user_eleve", *u));
    }
    v
}

/// Instancie une route en remplaçant TOUS les paramètres `{..}` par le même
/// identifiant de B (format UUID homogène) ; `{*rest}` par une valeur neutre.
fn instancier_avec(chemin: &str, id: Uuid) -> String {
    let mut out = String::new();
    let mut reste = chemin;
    while let Some(d) = reste.find('{') {
        out.push_str(&reste[..d]);
        let f = reste[d..].find('}').map(|x| d + x).unwrap_or(reste.len() - 1);
        let nom = &reste[d + 1..f];
        if nom.starts_with('*') {
            out.push('x');
        } else {
            out.push_str(&id.to_string());
        }
        reste = &reste[f + 1..];
    }
    out.push_str(reste);
    out
}

// ───────────────────────── Matrice générique de cloisonnement ─────────────────────────

/// GÉNÉRIQUE. Pour chaque route GET à paramètre, un acteur **privilégié de A**
/// (Direction : le plus de droits dans A) demande la ressource en injectant un
/// identifiant **de B**. Aucune réponse ne doit révéler une donnée de B : on
/// exige un refus (401/403/404), ou une réponse vide, ou — si la route répond
/// 2xx — l'absence de tout autre identifiant de B et de tout champ interdit
/// (après retrait de l'identifiant injecté, qui n'est qu'un écho de l'entrée).
///
/// Cette matrice couvre AUTOMATIQUEMENT toute route ajoutée au routeur : si une
/// nouvelle route GET ouvre une ressource de B à un acteur de A, ce test échoue.
#[test]
fn matrice_cloisonnement_toutes_routes_get() {
    let Some(m) = monde() else { return };
    let ids_b = identifiants_de_b(m);
    // Acteurs de A dont on éprouve le jeton : le plus privilégié, plus un
    // professeur (portée réduite) et une famille (hors personnel).
    let acteurs_a = [Direction, Admin, Prof, Parent, Eleve];
    let routes: Vec<routes_src::RouteDecl> = routes_src::routes()
        .into_iter()
        .filter(|r| r.methode == "GET" && r.chemin.contains('{') && !est_publique(r))
        .collect();
    assert!(
        routes.len() > 30,
        "inventaire des routes GET à paramètre trop court : {}",
        routes.len()
    );

    let mut echecs: Vec<String> = Vec::new();
    let mut n = 0usize;
    rt().block_on(async {
        for r in &routes {
            for &acteur in &acteurs_a {
                let jeton = m.jeton(acteur).to_string();
                for (etiquette, id) in &ids_b {
                    let chemin = instancier_avec(&r.chemin, *id);
                    let rep = m.appel(r.methode, &chemin, Some(&jeton), None).await;
                    n += 1;
                    if !rep.est_succes() {
                        continue; // refus : correct
                    }
                    // 2xx : retirer l'écho de l'identifiant injecté, puis chercher
                    // tout AUTRE identifiant de B ou un champ interdit.
                    let texte = rep.texte.replace(&id.to_string(), "");
                    let mut fuites_trouvees: Vec<String> = fuites(m, &texte, INTERDITS_GLOBAUX);
                    // Une réponse 2xx non vide sur une cible de B qui n'est pas
                    // qu'un écho est suspecte même sans identifiant repérable.
                    if fuites_trouvees.is_empty() && !rep.est_vide() {
                        // Contrôle d'indiscernabilité : la même route avec un UUID
                        // inexistant doit donner la même chose (sinon la présence
                        // de l'objet de B a fuité par le statut / le corps).
                        let neuf = Uuid::new_v4();
                        let ctl = m
                            .appel(r.methode, &instancier_avec(&r.chemin, neuf), Some(&jeton), None)
                            .await;
                        let a = texte;
                        let b = ctl.texte.replace(&neuf.to_string(), "");
                        if ctl.statut != rep.statut || a != b {
                            fuites_trouvees.push(format!(
                                "réponse 2xx distincte d'un id inexistant (statut {} vs {})",
                                rep.statut.as_u16(),
                                ctl.statut.as_u16()
                            ));
                        }
                    }
                    for f in fuites_trouvees {
                        echecs.push(format!(
                            "{} {} (handler {}) acteur={acteur:?} id_B={etiquette} : {f}",
                            r.methode, r.chemin, r.handler
                        ));
                    }
                }
            }
        }
    });
    eprintln!(
        "[RS3] matrice cloisonnement GET : {} routes × {} acteurs × {} ids = {n} appels, {} écart(s)",
        routes.len(),
        acteurs_a.len(),
        ids_b.len(),
        echecs.len()
    );
    assert!(
        echecs.is_empty(),
        "cloisonnement rompu sur {} cas :\n{}",
        echecs.len(),
        echecs.join("\n")
    );
}

/// GÉNÉRIQUE (écritures). Pour chaque route POST/PUT/PATCH/DELETE à paramètre,
/// un acteur de A injecte un identifiant de B dans l'URL avec un corps vide
/// (`{}`). Aucune écriture ne doit **aboutir** (2xx) sur une cible de B : soit
/// un refus d'autorisation (401/403/404), soit une erreur de validation du
/// corps (400/409/422), jamais un 2xx qui aurait modifié B. Un corps vide ne
/// prouve pas qu'une écriture complète échouerait, mais tout 2xx ici est déjà
/// une anomalie (la cible appartient à B).
#[test]
fn matrice_cloisonnement_toutes_routes_ecritures_corps_vide() {
    let Some(m) = monde() else { return };
    let ids_b = identifiants_de_b(m);
    let acteurs_a = [Direction, Admin];
    let routes: Vec<routes_src::RouteDecl> = routes_src::routes()
        .into_iter()
        .filter(|r| {
            matches!(r.methode, "POST" | "PUT" | "PATCH" | "DELETE")
                && r.chemin.contains('{')
                && !est_publique(r)
        })
        .collect();

    let mut echecs: Vec<String> = Vec::new();
    let mut n = 0usize;
    rt().block_on(async {
        for r in &routes {
            for &acteur in &acteurs_a {
                let jeton = m.jeton(acteur).to_string();
                for (etiquette, id) in &ids_b {
                    let chemin = instancier_avec(&r.chemin, *id);
                    let corps = json!({});
                    let rep = m.appel(r.methode, &chemin, Some(&jeton), Some(&corps)).await;
                    n += 1;
                    if rep.est_succes() {
                        echecs.push(format!(
                            "{} {} (handler {}) acteur={acteur:?} id_B={etiquette} : 2xx ({}) sur une cible de B",
                            r.methode,
                            r.chemin,
                            r.handler,
                            rep.statut.as_u16()
                        ));
                    }
                    // Fuite dans le corps d'erreur (identifiant de B renvoyé) : interdite.
                    let texte = rep.texte.replace(&id.to_string(), "");
                    for f in fuites(m, &texte, INTERDITS_GLOBAUX) {
                        echecs.push(format!(
                            "{} {} (handler {}) acteur={acteur:?} id_B={etiquette} : {f}",
                            r.methode, r.chemin, r.handler
                        ));
                    }
                }
            }
        }
    });
    eprintln!(
        "[RS3] matrice cloisonnement écritures (corps vide) : {} routes, {n} appels, {} écart(s)",
        routes.len(),
        echecs.len()
    );
    assert!(
        echecs.is_empty(),
        "écriture inter-établissements possible sur {} cas :\n{}",
        echecs.len(),
        echecs.join("\n")
    );
}

// ───────────────────────── Invariant : inventaire à jour ─────────────────────────

/// Garde-fou : l'inventaire des routes de `routes_src` couvre toujours tous les
/// fichiers de `src/` qui déclarent des routes (sinon une route échapperait aux
/// deux matrices ci-dessus). Doublon volontaire de `separation.rs`, pour que ce
/// fichier reste autonome.
#[test]
fn rs3_inventaire_couvre_tous_les_fichiers_de_routes() {
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
    assert!(r.len() > 250, "inventaire des routes incomplet : {}", r.len());
}
