//! Aides communes aux tests d'intégration d'`ent-reprise`.
#![allow(dead_code)]

use ent_reprise::dump::{self, Evt};
use ent_reprise::jeu_fictif;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn tmp(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ent-reprise-it-{}-{nom}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// Chemin du binaire : variables d'exécution de nextest (archives relogées) d'abord, puis celle de cargo.
pub fn binaire() -> Command {
    let chemin = std::env::vars()
        .find(|(k, _)| {
            (k.starts_with("NEXTEST_BIN_EXE_") || k.starts_with("CARGO_BIN_EXE_"))
                && k.contains("reprise")
        })
        .map(|(_, v)| PathBuf::from(v))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_ent-reprise")));
    Command::new(chemin)
}

pub fn lancer(args: &[&str]) -> Output {
    binaire()
        .args(args)
        .output()
        .expect("lancement de ent-reprise")
}

pub fn sortie_texte(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pg16")
}

pub type Tables = BTreeMap<String, (Vec<String>, Vec<Vec<Option<String>>>)>;

pub fn lire_tables(p: &Path) -> Tables {
    let src = dump::ouvrir(p).unwrap();
    let mut t: Tables = BTreeMap::new();
    let mut cur = String::new();
    dump::lire(&src, &mut |e| {
        match e {
            Evt::CopyDebut {
                table, colonnes, ..
            } => {
                cur = table.clone();
                t.insert(table, (colonnes, vec![]));
            }
            Evt::Ligne(c) => t.get_mut(&cur).unwrap().1.push(c),
            _ => {}
        }
        Ok(())
    })
    .unwrap();
    t
}

pub fn cle(dossier: &Path, nom: &str) -> PathBuf {
    let c = dossier.join(nom);
    let o = lancer(&["generer-cle", "--fichier", c.to_str().unwrap()]);
    assert!(o.status.success(), "{}", sortie_texte(&o));
    c
}

pub fn jeu(nom: &str, lignes: usize) -> PathBuf {
    let d = tmp(nom);
    jeu_fictif::generer(&d, lignes).unwrap();
    d
}

pub fn anonymiser(dump: &Path, s3: Option<&Path>, sortie: &Path, cle: &Path) -> Output {
    let mut a = vec![
        "anonymiser",
        "--dump",
        dump.to_str().unwrap(),
        "--sortie",
        sortie.to_str().unwrap(),
        "--cle",
        cle.to_str().unwrap(),
    ];
    if let Some(s) = s3 {
        a.extend(["--objets-s3", s.to_str().unwrap()]);
    }
    lancer(&a)
}

pub fn tout_lire(d: &Path) -> String {
    let mut s = String::new();
    for e in std::fs::read_dir(d).unwrap() {
        let p = e.unwrap().path();
        if p.is_file() {
            s.push_str(&String::from_utf8_lossy(&std::fs::read(&p).unwrap()));
            s.push('\n');
        }
    }
    s
}
