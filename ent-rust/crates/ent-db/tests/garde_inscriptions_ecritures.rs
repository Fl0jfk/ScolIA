//! Garde statique des frontières (socle 3, lot I1 ; `docs/socles/03_INSCRIPTIONS.md` §5.2 point 5) :
//! le module d'inscriptions **n'écrit jamais** (`INSERT`, `UPDATE`, `DELETE`) dans une table qui
//! n'est pas à lui. Il passe par les commandes des socles 2 et 4 (`eleve_ports`,
//! `dossier_personnes::integrer_declaration_tx`, `rattacher_compte_tx`…).
//!
//! Le « module d'inscriptions » = les fichiers de `ent-db/src` et `ent-api/src` dont le nom contient
//! `inscription` ou `import` (hors tests), plus `inscriptions_reactions.rs`. Sans base : le test ne
//! lit que les sources.
//!
//! * Tables du socle 3 : préfixes `ent_inscription_`, `ent_preinscription_`, `ent_import_`,
//!   `ent_mandat_sepa_demande`.
//! * `EXCEPTIONS_AUTRES_SOCLES` : écritures **connues** vers d'autres domaines (comptes du socle 1,
//!   notifications du socle 8, mandats et IBAN du socle 25), **nominatives et justifiées**, à
//!   retirer par les lots qui portent ces commandes (I7 pour les finances). Une exception qui ne
//!   sert plus fait échouer le test (la liste ne pourrit pas) ; une nouvelle écriture hors socle 3
//!   aussi, tant qu'elle n'est pas portée par une commande.
//! * Une table dynamique (`INSERT INTO {…}`) est refusée : la garde ne peut pas la vérifier.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const SOCLE3: &[&str] = &[
    "ent_inscription_",
    "ent_preinscription_",
    "ent_import_",
    "ent_mandat_sepa_demande",
];

/// (table, justification). Écritures vers d'autres domaines, en attente de leur commande.
const EXCEPTIONS_AUTRES_SOCLES: &[(&str, &str)] = &[
    ("ent_user", "socle 1 : création du compte à l'acceptation d'une invitation (commande de compte à porter, lot I6)"),
    ("ent_membership", "socle 1 : appartenance du compte à l'établissement, confirmée par la personne (idem)"),
    ("ent_profil_personnel", "socle 1 : profil du compte créé à l'acceptation d'invitation (idem)"),
    ("ent_identite_journal", "socle 1 : journal d'identité du compte (idem)"),
    ("ent_notification", "socle 8 : notification d'accueil in-app, remplacée par la file d'envoi du socle 8"),
    ("ent_fac_sepa_mandat", "socle 25 (finances) : mandat SEPA signé à la validation (commande facturation, lot I7)"),
    ("ent_foyer_iban_histo", "socle 25 (finances) : historique d'IBAN du foyer, créé avec le mandat (lot I7) — ce n'est pas la table `ent_foyer` du socle 4"),
];

fn racine() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn fichiers_du_module() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for dossier in ["ent-db/src", "ent-api/src"] {
        let d = racine().join(dossier);
        for e in fs::read_dir(&d).unwrap_or_else(|e| panic!("{}: {e}", d.display())) {
            let p = e.unwrap().path();
            let nom = p.file_name().unwrap().to_string_lossy().to_string();
            if !nom.ends_with(".rs") || nom.starts_with("tests_") {
                continue;
            }
            if nom.contains("inscription") || nom.contains("import") {
                v.push(p);
            }
        }
    }
    v.sort();
    v
}

/// Source sans commentaires de ligne ni modules de tests (`#[cfg(test)]` jusqu'à la fin).
fn code_de_production(src: &str) -> String {
    let coupe = src.find("#[cfg(test)]").unwrap_or(src.len());
    src[..coupe]
        .lines()
        .map(|l| match l.find("//") {
            // Un `//` dans un littéral (URL) ne doit pas masquer du SQL : on ne coupe que les
            // lignes de commentaire pur.
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
            // Mot entier.
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
            // Texte courant (un message d'erreur « … update … ») : pas une requête.
            if table.is_empty() {
                continue;
            }
            out.push((verbe, table));
        }
    }
    out
}

fn dans_socle3(t: &str) -> bool {
    SOCLE3.iter().any(|p| t.starts_with(p))
}

#[test]
fn le_detecteur_voit_les_ecritures_interdites() {
    let exemple = r#"
        sqlx::query("INSERT INTO ent_personne (id) VALUES ($1)");
        sqlx::query("UPDATE ent_eleve SET nom = $1");
        sqlx::query("update ent_eleve_lien set x = 1");
        sqlx::query("DELETE FROM ent_foyer WHERE id = $1");
        sqlx::query("INSERT INTO ent_affectation (id) VALUES ($1)");
        sqlx::query("SELECT 1 FROM ent_personne FOR UPDATE");
        sqlx::query("INSERT INTO ent_inscription_dossier (id) VALUES ($1) ON CONFLICT DO UPDATE SET a = 1");
        sqlx::query(&format!("UPDATE {table} SET a = 1"));
    "#;
    let trouvees: BTreeSet<String> = ecritures(exemple).into_iter().map(|(_, t)| t).collect();
    for t in [
        "ent_personne",
        "ent_eleve",
        "ent_eleve_lien",
        "ent_foyer",
        "ent_affectation",
        "ent_inscription_dossier",
        "{table}",
    ] {
        assert!(trouvees.contains(t), "{t} non détectée : {trouvees:?}");
    }
    // `FOR UPDATE` n'est pas une écriture.
    assert_eq!(
        ecritures("SELECT 1 FROM ent_personne FOR UPDATE").len(),
        0,
        "FOR UPDATE ne doit pas compter"
    );
    // Les tables des socles 2 et 4 ne sont jamais au socle 3.
    for t in [
        "ent_eleve",
        "ent_personne",
        "ent_foyer",
        "ent_eleve_lien",
        "ent_affectation",
        "ent_eleve_scolarite_annee",
    ] {
        assert!(!dans_socle3(t), "{t}");
    }
}

#[test]
fn le_module_d_inscriptions_n_ecrit_que_dans_ses_tables() {
    let fichiers = fichiers_du_module();
    for attendu in ["inscriptions.rs", "inscriptions_reactions.rs", "imports.rs"] {
        assert!(
            fichiers
                .iter()
                .any(|f| f.file_name().unwrap().to_string_lossy() == attendu),
            "fichier du module introuvable par la garde : {attendu}"
        );
    }
    let mut violations = Vec::new();
    let mut exceptions_utilisees: BTreeSet<&str> = BTreeSet::new();
    for f in &fichiers {
        let src = fs::read_to_string(f).unwrap();
        let nom = f.file_name().unwrap().to_string_lossy().to_string();
        for (verbe, table) in ecritures(&code_de_production(&src)) {
            if table.contains('{') || table.contains('$') {
                violations.push(format!("{nom} : {verbe} sur une table dynamique « {table} » (invérifiable)"));
            } else if dans_socle3(&table) {
                // chez soi
            } else if let Some((t, _)) = EXCEPTIONS_AUTRES_SOCLES.iter().find(|(t, _)| *t == table) {
                exceptions_utilisees.insert(t);
            } else {
                violations.push(format!(
                    "{nom} : {verbe} sur « {table} » — table d'un autre socle : passez par sa commande"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "le module d'inscriptions écrit hors du socle 3 :\n{}",
        violations.join("\n")
    );
    let inutiles: Vec<&str> = EXCEPTIONS_AUTRES_SOCLES
        .iter()
        .map(|(t, _)| *t)
        .filter(|t| !exceptions_utilisees.contains(t))
        .collect();
    assert!(
        inutiles.is_empty(),
        "exceptions devenues inutiles, à retirer de la liste : {inutiles:?}"
    );
}

/// Les commandes des socles 2 et 4 sont bien celles qu'appelle le module (et non une copie).
#[test]
fn les_commandes_attendues_sont_appelees() {
    let ins = fs::read_to_string(racine().join("ent-db/src/inscriptions.rs")).unwrap();
    for appel in [
        "eleve_ports::creer_eleve_tx",
        "eleve_ports::enregistrer_inscription_annuelle_tx",
        "dossier_personnes::integrer_declaration_tx",
        "dossier_personnes::rattacher_compte_tx",
        "dossier_personnes::confirmer_liens_attente_tx",
    ] {
        assert!(ins.contains(appel), "inscriptions.rs n'appelle plus {appel}");
    }
    let imp = fs::read_to_string(racine().join("ent-db/src/imports.rs")).unwrap();
    for appel in ["eleve_ports::creer_eleve_tx", "dossier_personnes::integrer_declaration_tx"] {
        assert!(imp.contains(appel), "imports.rs n'appelle plus {appel}");
    }
}
