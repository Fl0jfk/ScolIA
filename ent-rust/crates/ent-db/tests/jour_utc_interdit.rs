//! Garde statique « jour métier » : le code de production d'`ent-db/src` ne calcule jamais
//! « aujourd'hui » en UTC ni avec `CURRENT_DATE` (fuseau de session = UTC), car la date de
//! l'établissement (Europe/Paris par défaut) diffère de la date UTC entre 22 h/23 h et minuit
//! UTC. On passe par `ent_aujourdhui_etab($etab)` (SQL) ou `domain_eleve::horloge::jour_paris()`
//! (Rust). Les fichiers de tests (`tests_*.rs`) sont couverts par la même règle côté Rust.
//! Sans base : ne dépend que des fichiers sources.

use std::fs;

/// (fichier, extrait, justification) — exceptions nominatives.
const EXCEPTIONS: &[(&str, &str, &str)] = &[(
    "registre_donnees.rs",
    "date_execution >= current_date",
    "vérification humaine après restauration, hors parcours métier : un jour d'écart sans effet",
)];

#[test]
fn aucun_jour_utc_ni_current_date_dans_ent_db() {
    let dossier = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut fautes = Vec::new();
    for e in fs::read_dir(dossier).unwrap() {
        let p = e.unwrap().path();
        if p.extension().and_then(|x| x.to_str()) != Some("rs") {
            continue;
        }
        let nom = p.file_name().unwrap().to_str().unwrap().to_string();
        let texte = fs::read_to_string(&p).unwrap();
        let test_only = nom.starts_with("tests_");
        for (i, ligne) in texte.lines().enumerate() {
            let bas = ligne.to_ascii_lowercase();
            let utc = ligne.contains("Utc::now().date_naive()");
            let sql = bas.contains("current_date") && !test_only;
            if !(utc || sql) {
                continue;
            }
            if EXCEPTIONS.iter().any(|(f, x, _)| *f == nom && ligne.contains(x)) {
                continue;
            }
            fautes.push(format!("{nom}:{} {}", i + 1, ligne.trim()));
        }
    }
    assert!(
        fautes.is_empty(),
        "jour calculé hors fuseau de l'établissement (utiliser ent_aujourdhui_etab / horloge::jour_paris) :\n{}",
        fautes.join("\n")
    );
}
