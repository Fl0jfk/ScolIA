//! Empreintes SHA-256 : calcul en flux et vérification d'un fichier `SHA256SUMS`.
//!
//! Format de `SHA256SUMS` : celui de `sha256sum` — `<64 hexa><deux espaces><chemin relatif>` par ligne.

use crate::{err, Res};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

/// SHA-256 d'un fichier, en flux (mémoire constante).
pub fn sha256_fichier(p: &Path) -> Res<String> {
    let mut f =
        std::fs::File::open(p).map_err(|e| err!("lecture impossible pour l'empreinte : {e}"))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

/// SHA-256 d'un dossier : empreinte de la liste triée « chemin relatif + empreinte du fichier ».
pub fn sha256_dossier(d: &Path) -> Res<String> {
    let mut fichiers = Vec::new();
    lister(d, d, &mut fichiers)?;
    fichiers.sort();
    let mut h = Sha256::new();
    for f in fichiers {
        h.update(f.to_string_lossy().replace('\\', "/").as_bytes());
        h.update([0u8]);
        h.update(sha256_fichier(&d.join(&f))?.as_bytes());
        h.update(b"\n");
    }
    Ok(hex::encode(h.finalize()))
}

fn lister(racine: &Path, d: &Path, out: &mut Vec<PathBuf>) -> Res<()> {
    for e in std::fs::read_dir(d)? {
        let e = e?;
        let p = e.path();
        if p.is_dir() {
            lister(racine, &p, out)?;
        } else {
            out.push(
                p.strip_prefix(racine)
                    .map_err(|_| err!("chemin hors du dossier"))?
                    .to_path_buf(),
            );
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum Verdict {
    Conforme,
    Different,
    Manquant,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Verification {
    pub fichier: String,
    pub verdict: Verdict,
}

/// Vérifie les fichiers listés par un `SHA256SUMS`. Les chemins sont relatifs au dossier du fichier.
pub fn verifier_sommes(sommes: &Path) -> Res<Vec<Verification>> {
    let base = sommes.parent().unwrap_or(Path::new("."));
    let texte = std::fs::read_to_string(sommes).map_err(|e| err!("SHA256SUMS illisible : {e}"))?;
    let mut v = Vec::new();
    for (i, l) in texte.lines().enumerate() {
        if l.trim().is_empty() || l.starts_with('#') {
            continue;
        }
        let (h, f) = l
            .split_once("  ")
            .or_else(|| l.split_once(" *"))
            .ok_or_else(|| {
                err!(
                    "SHA256SUMS ligne {} : format « <empreinte>  <fichier> » attendu",
                    i + 1
                )
            })?;
        if h.len() != 64 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(err!("SHA256SUMS ligne {} : empreinte invalide", i + 1));
        }
        if f.contains("..") || Path::new(f).is_absolute() {
            return Err(err!(
                "SHA256SUMS ligne {} : chemin hors du dossier refusé",
                i + 1
            ));
        }
        let p = base.join(f);
        let verdict = if !p.is_file() {
            Verdict::Manquant
        } else if sha256_fichier(&p)?.eq_ignore_ascii_case(h) {
            Verdict::Conforme
        } else {
            Verdict::Different
        };
        v.push(Verification {
            fichier: f.to_string(),
            verdict,
        });
    }
    Ok(v)
}

/// Écrit `SHA256SUMS` pour les fichiers donnés (chemins relatifs à `base`).
pub fn ecrire_sommes(base: &Path, fichiers: &[&str]) -> Res<String> {
    let mut s = String::new();
    for f in fichiers {
        s.push_str(&format!("{}  {}\n", sha256_fichier(&base.join(f))?, f));
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ent-reprise-test-{}-{nom}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn sha256_connu() {
        let d = tmp("sha");
        std::fs::write(d.join("a.txt"), b"abc").unwrap();
        assert_eq!(
            sha256_fichier(&d.join("a.txt")).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn verification_conforme_different_manquant() {
        let d = tmp("sommes");
        std::fs::write(d.join("a.txt"), b"abc").unwrap();
        std::fs::write(d.join("b.txt"), b"def").unwrap();
        let mut s = ecrire_sommes(&d, &["a.txt", "b.txt"]).unwrap();
        s.push_str(&format!("{}  absent.txt\n", "0".repeat(64)));
        std::fs::write(d.join("SHA256SUMS"), &s).unwrap();
        std::fs::write(d.join("b.txt"), b"modifie").unwrap();
        let v = verifier_sommes(&d.join("SHA256SUMS")).unwrap();
        assert_eq!(v[0].verdict, Verdict::Conforme);
        assert_eq!(v[1].verdict, Verdict::Different);
        assert_eq!(v[2].verdict, Verdict::Manquant);
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn chemin_hors_dossier_refuse() {
        let d = tmp("hors");
        std::fs::write(
            d.join("SHA256SUMS"),
            format!("{}  ../secret\n", "a".repeat(64)),
        )
        .unwrap();
        assert!(verifier_sommes(&d.join("SHA256SUMS")).is_err());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn empreinte_de_dossier_stable_et_sensible() {
        let d = tmp("dossier");
        std::fs::write(d.join("a"), b"1").unwrap();
        let h1 = sha256_dossier(&d).unwrap();
        assert_eq!(h1, sha256_dossier(&d).unwrap());
        std::fs::write(d.join("a"), b"2").unwrap();
        assert_ne!(h1, sha256_dossier(&d).unwrap());
        let _ = std::fs::remove_dir_all(d);
    }
}
