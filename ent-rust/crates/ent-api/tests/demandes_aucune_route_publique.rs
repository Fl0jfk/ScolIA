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

/// Routes `/demandes…` connues et authentifiées (vide en N1-0).
const ROUTES_DEMANDES_AUTORISEES: &[&str] = &[];

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
