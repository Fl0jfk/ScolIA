//! Liste d'objets S3 fournie sous forme de **fichier** (jamais lue sur le service).
//!
//! Format : texte, une ligne par objet, colonnes séparées par une tabulation :
//! `cle<TAB>taille_en_octets[<TAB>sha256_hexadécimal]`. Les lignes vides, celles qui commencent par
//! `#` et une ligne d'en-tête `cle<TAB>taille…` sont ignorées.

use crate::{err, Res};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjetS3 {
    pub cle: String,
    pub taille: u64,
    pub sha256: Option<String>,
}

/// Lit la liste d'objets.
pub fn lire_liste(chemin: &Path) -> Res<Vec<ObjetS3>> {
    let f = std::fs::File::open(chemin).map_err(|e| err!("liste d'objets illisible : {e}"))?;
    lire_liste_depuis(BufReader::new(f))
}

pub fn lire_liste_depuis<R: BufRead>(r: R) -> Res<Vec<ObjetS3>> {
    let mut v = Vec::new();
    for (i, l) in r.lines().enumerate() {
        let l = l?;
        let l = l.trim_end_matches('\r');
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let p: Vec<&str> = l.split('\t').collect();
        if i == 0 && p[0] == "cle" {
            continue;
        }
        if p.len() < 2 || p.len() > 3 || p[0].is_empty() {
            return Err(err!(
                "liste d'objets, ligne {} : format attendu « cle<TAB>taille[<TAB>sha256] »",
                i + 1
            ));
        }
        let taille: u64 = p[1]
            .trim()
            .parse()
            .map_err(|_| err!("liste d'objets, ligne {} : taille invalide", i + 1))?;
        let sha256 = match p.get(2).map(|s| s.trim()).filter(|s| !s.is_empty()) {
            None => None,
            Some(h) if h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()) => {
                Some(h.to_ascii_lowercase())
            }
            Some(_) => {
                return Err(err!(
                    "liste d'objets, ligne {} : empreinte SHA-256 invalide",
                    i + 1
                ))
            }
        };
        v.push(ObjetS3 {
            cle: p[0].to_string(),
            taille,
            sha256,
        });
    }
    Ok(v)
}

/// Écrit une liste d'objets au format ci-dessus.
pub fn ecrire_liste(objets: &[ObjetS3]) -> String {
    let mut s = String::from("cle\ttaille\tsha256\n");
    for o in objets {
        s.push_str(&o.cle);
        s.push('\t');
        s.push_str(&o.taille.to_string());
        s.push('\t');
        if let Some(h) = &o.sha256 {
            s.push_str(h);
        }
        s.push('\n');
    }
    s
}

/// Une clé d'objet est « secrète » (à ne jamais reprendre) : registre et secrets des établissements
/// (`tenants/…`, T0 §2 point 4), segment `secrets`, extensions de clés.
pub fn est_secret(cle: &str) -> bool {
    let bas = cle.to_ascii_lowercase();
    bas.starts_with("tenants/")
        || bas
            .split('/')
            .any(|s| s == "secrets" || s == ".env" || s.starts_with(".env."))
        || [".pem", ".key", ".p12", ".pfx"]
            .iter()
            .any(|e| bas.ends_with(e))
}

/// Segments de chemin considérés comme **structurels** (jamais personnels) : conservés tels quels à
/// l'anonymisation et seuls autorisés à figurer dans le rapport.
pub const SEGMENTS_STRUCTURELS: &[&str] = &[
    "documents",
    "users",
    "eleves",
    "eleve",
    "photos",
    "photo",
    "requests",
    "request",
    "files",
    "rentree",
    "submissions",
    "pending",
    "knowledge",
    "travels",
    "travel",
    "blog",
    "logos",
    "logo",
    "signatures",
    "factures",
    "facture",
    "pdf",
    "pieces",
    "justificatifs",
    "absences",
    "absence",
    "pap",
    "internat",
    "rh",
    "stage",
    "stages",
    "certificates",
    "messaging",
    "messages",
    "attachments",
    "fiches",
    "dialogue",
    "partenariats",
    "invitation",
    "rdv",
    "inscription",
    "portes-ouvertes",
    "portes_ouvertes",
    "icons",
    "images",
    "settings",
    "pilotage",
    "export",
    "exports",
    "import",
    "imports",
    "tmp",
    "public",
    "private",
    "planning",
    "sst",
    "sante",
    "famille",
    "familles",
    "personnel",
    "personnels",
    "enseignants",
    "classes",
    "uploads",
    "upload",
    "avatars",
    "avatar",
    "docs",
    "archives",
    "archive",
    "data",
    "json",
];

/// Première composante de la clé si elle est structurelle (sinon `(autre)`), pour les comptages.
pub fn premier_segment_sur(cle: &str) -> String {
    let s = cle.split('/').next().unwrap_or("").to_ascii_lowercase();
    if SEGMENTS_STRUCTURELS.contains(&s.as_str()) || s == "tenants" {
        s
    } else {
        "(autre)".to_string()
    }
}

/// Comptage par premier segment : (nombre d'objets, octets).
pub fn compter(objets: &[ObjetS3]) -> BTreeMap<String, (u64, u64)> {
    let mut m: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for o in objets {
        let e = m.entry(premier_segment_sur(&o.cle)).or_default();
        e.0 += 1;
        e.1 += o.taille;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_et_ecriture() {
        let txt = "cle\ttaille\tsha256\n# commentaire\ndocuments/a.pdf\t10\t\nphotos/b.jpg\t20\t"
            .to_string()
            + &"ab".repeat(32)
            + "\n";
        let v = lire_liste_depuis(txt.as_bytes()).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].sha256.as_deref(), Some("ab".repeat(32).as_str()));
        assert_eq!(lire_liste_depuis(ecrire_liste(&v).as_bytes()).unwrap(), v);
    }

    #[test]
    fn lignes_invalides_refusees() {
        assert!(lire_liste_depuis("cle-seule\n".as_bytes()).is_err());
        assert!(lire_liste_depuis("a\tpas-un-nombre\n".as_bytes()).is_err());
        assert!(lire_liste_depuis("a\t1\tcourt\n".as_bytes()).is_err());
    }

    #[test]
    fn secrets_detectes() {
        assert!(est_secret("tenants/index.json"));
        assert!(est_secret("Tenants/etab1/secrets/cle.json"));
        assert!(est_secret("documents/secrets/x"));
        assert!(est_secret("a/b/cle.PEM"));
        assert!(!est_secret("documents/users/abc/contrat.pdf"));
    }

    #[test]
    fn comptage_ne_nomme_que_des_segments_structurels() {
        let o = |c: &str| ObjetS3 {
            cle: c.into(),
            taille: 5,
            sha256: None,
        };
        let m = compter(&[
            o("documents/x"),
            o("documents/y"),
            o("dupont-jean/z"),
            o("tenants/i.json"),
        ]);
        assert_eq!(m["documents"], (2, 10));
        assert_eq!(m["(autre)"], (1, 5));
        assert!(!m.keys().any(|k| k.contains("dupont")));
    }
}
