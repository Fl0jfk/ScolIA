//! Socle N1 (demandes), lot N1-0 — NJ-88 : **aucun dépôt public**.
//!
//! Le Next.js avait une page parents sans compte (`/api/requests/parent-portal`, confirmation par GET) qui
//! rattachait des enfants par l'égalité d'adresse e-mail (D-051 : non reprise). Ici, le dépôt d'une demande passe
//! par une session authentifiée ; ce test lit le **source** des routes (comme l'inventaire de séparation) et échoue
//! si une route de ce type apparaît.
//!
//! * aucune route `parent-portal`, `/requests…`, ni « demande publique » ;
//! * les routes `/demandes…` n'existent pas encore (écrans et API : lots N1-1 et N1-4) ; quand elles arriveront,
//!   elles devront être ajoutées à `ROUTES_DEMANDES_AUTORISEES` **avec** leur contrôle de session, et la matrice
//!   de séparation (`separation.rs`) les couvrira.

use std::fs;

/// Routes `/demandes…` connues et authentifiées (lot N1-4) : chaque handler prend `EntSession` ; aucune n'est publique.
const ROUTES_DEMANDES_AUTORISEES: &[&str] = &[
    "/demandes",
    "/demandes/guichet",
    "/demandes/mes",
    "/demandes/file",
    "/demandes/hse",
    "/demandes/hse/mes",
    "/demandes/hse/a-decider",
    "/demandes/hse/enveloppe",
    "/demandes/hse/{id}/confirmer",
    "/demandes/hse/{id}/decider",
    "/demandes/copies/file",
    "/demandes/copies/mes",
    "/demandes/copies/a-valider",
    "/demandes/copies/{id}/valider",
    "/demandes/copies/{id}/impression",
    "/demandes/copies/{id}/prete",
    "/demandes/{id}",
    "/demandes/{id}/prendre",
    "/demandes/{id}/annuler",
];

fn routes_declarees() -> Vec<(String, String)> {
    let dossier = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut v = Vec::new();
    for e in fs::read_dir(dossier).unwrap() {
        let p = e.unwrap().path();
        if p.extension().and_then(|x| x.to_str()) != Some("rs") {
            continue;
        }
        let nom = p.file_name().unwrap().to_str().unwrap().to_string();
        let texte = fs::read_to_string(&p).unwrap();
        let mut reste = texte.as_str();
        while let Some(i) = reste.find(".route(") {
            reste = &reste[i + ".route(".len()..];
            let apres = reste.trim_start();
            if let Some(litteral) = apres.strip_prefix('"') {
                if let Some(fin) = litteral.find('"') {
                    v.push((nom.clone(), litteral[..fin].to_string()));
                }
            }
        }
    }
    v
}

#[test]
fn t_nj88_aucun_depot_public() {
    let routes = routes_declarees();
    assert!(routes.len() > 100, "l'inventaire des routes semble vide ({})", routes.len());
    let mauvaises: Vec<String> = routes
        .iter()
        .filter(|(_, r)| {
            let b = r.to_ascii_lowercase();
            b.contains("parent-portal")
                || b.contains("parent_portal")
                || b.starts_with("/requests")
                || b.contains("/api/requests")
                || (b.contains("public") && b.contains("demande"))
        })
        .map(|(f, r)| format!("{f} : {r}"))
        .collect();
    assert!(mauvaises.is_empty(), "route de dépôt public : {mauvaises:?}");
    let demandes: Vec<&(String, String)> = routes
        .iter()
        .filter(|(_, r)| r == "/demandes" || r.starts_with("/demandes/"))
        .collect();
    for (f, r) in &demandes {
        assert!(
            ROUTES_DEMANDES_AUTORISEES.contains(&r.as_str()),
            "route {r} ({f}) : à déclarer dans ROUTES_DEMANDES_AUTORISEES avec son contrôle de session"
        );
    }
}

/// NJ-92 : aucune route `GET` de `/demandes…` n'accomplit un geste (« prête », validation, décision) ; ces gestes sont
/// des `POST` de session. Le seul `GET` qui écrit est l'ouverture d'une demande de copies prête par son demandeur
/// (« vue », sans effet sur la demande) : il n'est pas dans cette liste de gestes.
#[test]
fn t_nj92_aucun_geste_par_get() {
    let source = include_str!("../src/demandes.rs");
    for geste in ["prete", "valider", "decider", "impression", "confirmer", "prendre", "annuler"] {
        assert!(
            !source.contains(&format!("/{geste}\", get(")),
            "le geste « {geste} » ne doit jamais être un GET"
        );
    }
    // Toutes les routes de demandes passent par une session : chaque handler prend `session: EntSession`.
    let sans_session = source
        .split("\nasync fn ")
        .skip(1)
        .filter(|f| !f.contains("session: EntSession") && !f.starts_with("geste("))
        .map(|f| f.lines().next().unwrap_or("").to_string())
        .collect::<Vec<_>>();
    assert!(sans_session.is_empty(), "handler sans session : {sans_session:?}");
}
