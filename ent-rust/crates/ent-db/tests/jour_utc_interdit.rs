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

/// Formes interdites dans les TESTS (Rust et SQL embarqué) : elles calculent « aujourd'hui » en UTC (le serveur de
/// CI est en UTC) alors que le jour métier est celui de l'établissement (Paris par défaut). Entre 23 h et minuit
/// UTC (minuit–1 h à Paris) un test qui date en UTC et un code qui date à Paris divergent, et la CI lancée de nuit
/// échoue. À la place : `domain_eleve::horloge::jour_paris()` (Rust ; `ent_db::jour_paris()` côté web), `ent_aujourdhui()`
/// / `ent_aujourdhui_etab(etab)` (SQL), ou un jour ouvré explicite (lundi à vendredi) quand le test a besoin d'un jour
/// de classe. Les lignes de commentaire sont ignorées.
const FORMES_TESTS: &[&str] = &[
    "utc::now().date_naive()",
    "utc::now().weekday()",
    "utc::now().naive_utc().date()",
    "local::now()",
    "utc::today",
    "local::today",
    "current_date",
    "current_timestamp::date",
    "now()::date",
];

/// (fichier, extrait, justification) — exceptions nominatives pour les tests.
const EXCEPTIONS_TESTS: &[(&str, &str, &str)] = &[
    (
        "tests_eleve_jour_etablissement.rs",
        "SELECT current_date",
        "méta-test : il mesure le jour UTC du serveur pour prouver que les deux fuseaux extrêmes couvrent toute heure",
    ),
    (
        "tests_eleve_jour_etablissement.rs",
        "(current_date|",
        "garde SQL : l'expression régulière qui détecte current_date dans le code des fonctions ent_eleve_*",
    ),
    (
        "tests_cahier.rs",
        "Utc::now().naive_utc().date()",
        "le test exige justement que le jour du fuseau de test diffère du jour UTC",
    ),
    (
        "jour_utc_interdit.rs",
        "",
        "ce fichier définit les formes interdites",
    ),
];

fn dossiers_de_tests() -> Vec<(std::path::PathBuf, bool)> {
    let racine = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // (dossier, tous les .rs ? sinon seulement les tests_*.rs)
    vec![
        (racine.join("src"), false),
        (racine.join("tests"), true),
        (racine.join("../ent-api/src"), false),
        (racine.join("../ent-api/tests"), true),
        (racine.join("../../web/crates/ent-web-server/tests"), true),
    ]
}

#[test]
fn aucun_jour_utc_dans_les_tests() {
    let mut fautes = Vec::new();
    let mut fichiers_lus = 0;
    for (dossier, tous) in dossiers_de_tests() {
        let entrees = fs::read_dir(&dossier).unwrap_or_else(|e| panic!("{}: {e}", dossier.display()));
        for e in entrees {
            let p = e.unwrap().path();
            if p.extension().and_then(|x| x.to_str()) != Some("rs") {
                continue;
            }
            let nom = p.file_name().unwrap().to_str().unwrap().to_string();
            if !tous && !nom.starts_with("tests_") {
                continue;
            }
            fichiers_lus += 1;
            let texte = fs::read_to_string(&p).unwrap();
            for (i, ligne) in texte.lines().enumerate() {
                let t = ligne.trim_start();
                if t.starts_with("//") {
                    continue;
                }
                let bas = ligne.to_ascii_lowercase();
                if !FORMES_TESTS.iter().any(|f| bas.contains(f)) {
                    continue;
                }
                if EXCEPTIONS_TESTS.iter().any(|(f, x, _)| *f == nom && ligne.contains(x)) {
                    continue;
                }
                fautes.push(format!("{}:{} {}", p.display(), i + 1, ligne.trim()));
            }
        }
    }
    assert!(fichiers_lus > 50, "garde aveugle : {fichiers_lus} fichiers de tests lus seulement");
    assert!(
        fautes.is_empty(),
        "test qui date « aujourd'hui » en UTC (utiliser horloge::jour_paris / ent_aujourdhui, ou un jour ouvré explicite) :\n{}",
        fautes.join("\n")
    );
}
