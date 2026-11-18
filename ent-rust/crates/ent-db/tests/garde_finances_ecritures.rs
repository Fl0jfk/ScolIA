//! **Garde de code R-F1** (socle 24, lot F1 ; `docs/socles/24_FINANCES.md` §6.2 point 1, §18 R-F1) :
//! *aucune écriture financière hors du service*. Un seul service de facturation écrit dans
//! `ent_fac_*`, `ent_fin_*` et `ent_foyer_iban_histo` ; les autres domaines (inscriptions, restauration,
//! voyages, bourses, IA…) **demandent** par ses commandes, ils n'écrivent pas.
//!
//! Le service = les fichiers de [`ent_db::facturation_service::FICHIERS_DU_SERVICE`]. Le test lit le
//! **source** de toutes les crates du dépôt (sans base) et échoue si :
//!
//! * un autre fichier de production contient un `INSERT INTO`, `UPDATE` ou `DELETE FROM` sur une table
//!   protégée, ou sur une table dynamique (`{table}`, `$…`) qu'il serait impossible de vérifier ;
//! * un autre fichier appelle les fonctions SQL d'écriture du lot F2 (`ent_fin_compte_du_foyer`,
//!   `ent_fin_reprendre_comptes`, `ent_fin_entite_par_defaut`) ;
//! * un fichier listé n'existe plus (la liste ne pourrit pas) ;
//! * le détecteur lui-même cesse de voir une écriture (test du détecteur).
//!
//! **Aucune exception** : le mandat SEPA signé à l'inscription, seule exception de l'ère F0, est devenu
//! une commande du service (`facturation_mandats`). Une nouvelle exception exige une décision écrite.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use ent_db::facturation_service::FICHIERS_DU_SERVICE;

const PREFIXES_PROTEGES: &[&str] = &["ent_fac_", "ent_fin_"];
const TABLES_PROTEGEES: &[&str] = &["ent_foyer_iban_histo"];
const FONCTIONS_SQL_D_ECRITURE: &[&str] = &[
    "ent_fin_compte_du_foyer",
    "ent_fin_reprendre_comptes",
    "ent_fin_entite_par_defaut",
];

fn racine_des_crates() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Tous les `.rs` de production (`crates/*/src/**`), hors modules de tests (`tests_*.rs`) et hors
/// l'interface JavaScript historique (`ui.rs`, du texte, pas du SQL).
fn fichiers_de_production() -> Vec<PathBuf> {
    fn parcourir(d: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                parcourir(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                let nom = p.file_name().unwrap().to_string_lossy().to_string();
                if nom.starts_with("tests_") || nom == "ui.rs" {
                    continue;
                }
                out.push(p);
            }
        }
    }
    let mut v = Vec::new();
    for c in fs::read_dir(racine_des_crates()).unwrap().flatten() {
        let src = c.path().join("src");
        if src.is_dir() {
            parcourir(&src, &mut v);
        }
    }
    v.sort();
    v
}

/// Retire les éléments `#[cfg(test)]` (modules, fonctions, `use`) **et seulement eux** : le code de production
/// qui suit un bloc de tests (E4) reste analysé. L'élément visé va de l'attribut jusqu'à son `;` ou à
/// l'accolade fermante qui équilibre son `{`.
fn sans_blocs_de_tests(src: &str) -> String {
    let mut out = String::new();
    let mut reste = src;
    while let Some(i) = reste.find("#[cfg(test)]") {
        out.push_str(&reste[..i]);
        let apres = &reste[i + "#[cfg(test)]".len()..];
        // Premier `{` ou `;` après l'attribut (les attributs et le `mod nom` n'en contiennent pas).
        let Some(j) = apres.find(['{', ';']) else {
            reste = "";
            break;
        };
        if apres.as_bytes()[j] == b';' {
            reste = &apres[j + 1..];
            continue;
        }
        let mut profondeur = 0_i32;
        let mut fin = apres.len();
        for (k, c) in apres[j..].char_indices() {
            match c {
                '{' => profondeur += 1,
                '}' => {
                    profondeur -= 1;
                    if profondeur == 0 {
                        fin = j + k + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        reste = &apres[fin..];
    }
    out.push_str(reste);
    out
}

/// Source sans commentaires de ligne ni blocs de tests.
fn code_de_production(src: &str) -> String {
    sans_blocs_de_tests(src)
        .lines()
        .map(|l| match l.find("//") {
            Some(i) if l[..i].trim().is_empty() => "",
            _ => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn est_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Écritures SQL repérées : (verbe, table). Table `{…}` / `$…` = dynamique.
fn ecritures(code: &str) -> Vec<(&'static str, String)> {
    let bas = code.to_lowercase();
    let mut out = Vec::new();
    for (verbe, motif) in [
        ("INSERT", "insert into"),
        ("DELETE", "delete from"),
        ("UPDATE", "update"),
    ] {
        let mut depuis = 0;
        while let Some(i) = bas[depuis..].find(motif) {
            let debut = depuis + i;
            let fin = debut + motif.len();
            depuis = fin;
            let avant = bas[..debut].chars().next_back();
            let apres = bas[fin..].chars().next();
            if avant.is_some_and(est_ident) || apres.is_some_and(est_ident) {
                continue;
            }
            let precedent: String = bas[..debut]
                .trim_end()
                .rsplit(|c: char| !est_ident(c))
                .next()
                .unwrap_or("")
                .to_string();
            // `FOR UPDATE` (verrou de lecture) et `DO UPDATE SET` (ON CONFLICT) : pas une cible.
            if verbe == "UPDATE" && (precedent == "for" || precedent == "do") {
                continue;
            }
            let reste = bas[fin..].trim_start();
            let reste = reste.strip_prefix("only ").unwrap_or(reste).trim_start();
            let table: String = reste
                .chars()
                .take_while(|c| est_ident(*c) || matches!(c, '{' | '}' | '$' | '"' | '.'))
                .collect();
            let table = table.trim_matches('"').to_string();
            if table.is_empty() {
                continue;
            }
            out.push((verbe, table));
        }
    }
    out
}

/// Outillage **générique** qui écrit dans une table dont le nom vient d'un registre (rechiffrement,
/// purge d'établissement, fusion d'élèves, reprise de zone…). Nominatif et justifié : un nouveau fichier
/// à table dynamique fait échouer la garde ; un fichier qui n'écrit plus de table dynamique aussi.
const OUTILS_GENERIQUES: &[(&str, &str)] = &[
    ("champs_chiffres.rs", "rechiffrement des colonnes scellées par le registre des champs (rotation de clé) : ne change aucune valeur métier, comprend les mandats et l'historique d'IBAN"),
    ("eleve_fusion.rs", "fusion d'élèves par le registre TABLES_ELEVE : aucune table financière n'y est déplacée (vérifié ci-dessous), les pièces restent sur la fiche facturée"),
    ("personnels_fiche.rs", "mises à jour de dates et de références sur les tables du socle 6"),
    ("registre_donnees.rs", "purge et neutralisation d'un établissement par le registre des tables (PRA, restauration) sous `ent.purge_etablissement`"),
    ("reprise_bytea.rs", "reprise des fichiers hors base (BYTEA vers objet) sur les tables à fichiers du registre"),
    ("reprise_zone.rs", "zone de reprise de la mise en service : tables de transit `ent_reprise_*`, jamais les tables financières"),
];

fn protegee(t: &str) -> bool {
    PREFIXES_PROTEGES.iter().any(|p| t.starts_with(p)) || TABLES_PROTEGEES.contains(&t)
}

fn dans_le_service(nom: &str) -> bool {
    FICHIERS_DU_SERVICE.contains(&nom)
}

#[test]
fn le_detecteur_voit_les_ecritures_financieres() {
    let exemple = r#"
        sqlx::query("INSERT INTO ent_fac_facture (id) VALUES ($1)");
        sqlx::query("UPDATE ent_fin_compte SET etat = 'archive'");
        sqlx::query("delete from ent_fin_designation where id = $1");
        sqlx::query("INSERT INTO ent_foyer_iban_histo (id) VALUES ($1)");
        sqlx::query("INSERT INTO ent_eleve (id) VALUES ($1)");
        sqlx::query("SELECT 1 FROM ent_fac_facture FOR UPDATE");
        sqlx::query("INSERT INTO ent_fin_tache (id) VALUES ($1) ON CONFLICT DO UPDATE SET a = 1");
        sqlx::query(&format!("UPDATE {table} SET a = 1"));
    "#;
    let trouvees: BTreeSet<String> = ecritures(exemple).into_iter().map(|(_, t)| t).collect();
    for t in [
        "ent_fac_facture",
        "ent_fin_compte",
        "ent_fin_designation",
        "ent_foyer_iban_histo",
        "ent_fin_tache",
        "{table}",
    ] {
        assert!(trouvees.contains(t), "{t} non détectée : {trouvees:?}");
    }
    assert_eq!(
        ecritures("SELECT 1 FROM ent_fac_facture FOR UPDATE").len(),
        0,
        "FOR UPDATE ne doit pas compter"
    );
    assert!(protegee("ent_fac_ligne") && protegee("ent_fin_journal") && protegee("ent_foyer_iban_histo"));
    assert!(!protegee("ent_foyer") && !protegee("ent_eleve") && !protegee("ent_personne"));
}

#[test]
fn aucune_ecriture_financiere_hors_du_service() {
    let fichiers = fichiers_de_production();
    for attendu in ["finances.rs", "fac_repo.rs", "inscriptions.rs", "routes.rs"] {
        assert!(
            fichiers.iter().any(|f| f.file_name().unwrap().to_string_lossy() == attendu),
            "fichier introuvable par la garde : {attendu}"
        );
    }
    let mut violations = Vec::new();
    for f in &fichiers {
        let nom = f.file_name().unwrap().to_string_lossy().to_string();
        if dans_le_service(&nom) {
            continue;
        }
        let code = code_de_production(&fs::read_to_string(f).unwrap());
        for (verbe, table) in ecritures(&code) {
            if protegee(&table) {
                violations.push(format!(
                    "{} : {verbe} sur « {table} » — écriture financière hors du service (R-F1) : passez par une commande du service de facturation",
                    f.display()
                ));
            } else if (table.contains('{') || table.contains('$'))
                && !OUTILS_GENERIQUES.iter().any(|(n, _)| *n == nom)
            {
                violations.push(format!(
                    "{} : {verbe} sur une table dynamique « {table} » (invérifiable : elle pourrait viser une table financière) ; un outil générique se déclare dans OUTILS_GENERIQUES avec sa justification",
                    f.display()
                ));
            }
        }
        for fonction in FONCTIONS_SQL_D_ECRITURE {
            if code.contains(fonction) {
                violations.push(format!(
                    "{} : appelle la fonction SQL d'écriture « {fonction} » hors du service (R-F1)",
                    f.display()
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "écritures financières hors du service de facturation (R-F1) :\n{}",
        violations.join("\n")
    );
}

#[test]
fn la_liste_du_service_ne_pourrit_pas() {
    let src = racine_des_crates().join("ent-db/src");
    for f in FICHIERS_DU_SERVICE {
        assert!(
            src.join(f).is_file(),
            "{f} est dans FICHIERS_DU_SERVICE mais n'existe plus : retirez-le de la liste"
        );
    }
    // Les fichiers du service sont dans `ent-db/src` : aucun fichier d'une autre crate ne peut y figurer.
    let mut vus = BTreeSet::new();
    for f in FICHIERS_DU_SERVICE {
        assert!(vus.insert(*f), "doublon : {f}");
        assert!(!f.contains('/'), "{f} : un nom de fichier de ent-db/src, pas un chemin");
    }
    // Le service ne gagne pas de fichier « fourre-tout » : routes et services généraux restent dehors.
    for interdit in ["service.rs", "routes.rs", "lib.rs", "inscriptions.rs", "seed.rs"] {
        assert!(
            !FICHIERS_DU_SERVICE.contains(&interdit),
            "{interdit} ne doit pas faire partie du service de facturation"
        );
    }
}

/// Les outils génériques déclarés écrivent encore une table dynamique (sinon : à retirer de la liste), et
/// la fusion d'élèves ne déplace aucune table financière (R-F1, R-F2 : une pièce ne se réécrit pas).
#[test]
fn les_outils_generiques_sont_tous_utiles_et_la_fusion_ne_touche_pas_aux_finances() {
    let fichiers = fichiers_de_production();
    for (nom, _) in OUTILS_GENERIQUES {
        // Un même nom peut exister dans deux crates (routes et service) : au moins un doit écrire.
        let concernes: Vec<_> = fichiers
            .iter()
            .filter(|f| f.file_name().unwrap().to_string_lossy() == *nom)
            .collect();
        assert!(!concernes.is_empty(), "outil générique introuvable : {nom}");
        assert!(
            concernes.iter().any(|f| {
                let code = code_de_production(&fs::read_to_string(f).unwrap());
                ecritures(&code).iter().any(|(_, t)| t.contains('{') || t.contains('$'))
            }),
            "{nom} n'écrit plus de table dynamique : retirez-le d'OUTILS_GENERIQUES"
        );
    }
    for t in domain_eleve::TABLES_ELEVE {
        if t.strategie == domain_eleve::StrategieFusion::Deplacer {
            assert!(
                !protegee(t.table),
                "{} : la fusion d'élèves déplacerait une table financière (écriture hors service)",
                t.table
            );
        }
    }
}

/// E4 — test de la garde elle-même : un `#[cfg(test)]` au **milieu** d'un fichier ne masque pas le code de
/// production qui le suit ; le bloc de tests, lui, est ignoré.
#[test]
fn la_garde_analyse_le_code_apres_un_bloc_de_tests() {
    let faux = r#"
fn avant() { sqlx::query("SELECT 1"); }

#[cfg(test)]
mod tests {
    fn dans_les_tests() { sqlx::query("INSERT INTO ent_fac_facture (id) VALUES ($1)"); }
}

fn apres() { sqlx::query("UPDATE ent_fin_compte SET etat = 'x'"); }
"#;
    let code = code_de_production(faux);
    let e = ecritures(&code);
    assert!(
        e.iter().any(|(v, t)| *v == "UPDATE" && t == "ent_fin_compte"),
        "l'écriture située après le bloc de tests doit être vue : {e:?}"
    );
    assert!(
        !e.iter().any(|(_, t)| t == "ent_fac_facture"),
        "l'écriture située dans le bloc de tests est ignorée : {e:?}"
    );
    // Un `#[cfg(test)]` sur un `use` (terminé par `;`) ne coupe rien non plus.
    let avec_use = "#[cfg(test)]\nuse x::y;\nfn f() { sqlx::query(\"DELETE FROM ent_fin_tache\"); }";
    assert!(ecritures(&code_de_production(avec_use)).iter().any(|(_, t)| t == "ent_fin_tache"));
}
