//! Réécriture des **clés d'objets S3** (et des colonnes qui les référencent).
//!
//! La structure du chemin est conservée (préfixes connus, identifiants techniques, extension du
//! fichier) ; tout ce qui pourrait être un nom (dossier ou fichier) est remplacé par un pseudonyme
//! déterministe. La même fonction sert à la liste d'objets **et** aux colonnes `*_s3_key`, `*_key`,
//! `file_url`… : un objet reste donc relié à la ligne qui le référence.

use super::cle::{Cle, Flux};
use super::pseudo::{est_identifiant_opaque, est_uuid};
use crate::s3::SEGMENTS_STRUCTURELS;

fn hache8(cle: &Cle, etiquette: &str, v: &str) -> String {
    let mut f = Flux::nouveau(cle, etiquette, v);
    (0..8)
        .map(|_| char::from_digit(f.tirer(16) as u32, 16).unwrap_or('0'))
        .collect()
}

fn extension(nom: &str) -> Option<String> {
    nom.rsplit_once('.')
        .map(|(_, e)| e)
        .filter(|e| !e.is_empty() && e.len() <= 8 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(|e| e.to_ascii_lowercase())
}

/// Réécrit une clé d'objet. Une valeur vide reste vide ; une URL `http(s)://` est traitée comme un chemin.
pub fn reecrire_cle(cle: &Cle, k: &str) -> String {
    if k.is_empty() {
        return String::new();
    }
    let (prefixe, chemin) = match k.find("://") {
        Some(i) if k[..i].chars().all(|c| c.is_ascii_alphabetic()) => (&k[..i + 3], &k[i + 3..]),
        _ => ("", k),
    };
    let segments: Vec<&str> = chemin.split('/').collect();
    let dernier = segments.len().saturating_sub(1);
    let mut sortie: Vec<String> = Vec::with_capacity(segments.len());
    for (i, s) in segments.iter().enumerate() {
        if s.is_empty() {
            sortie.push(String::new());
            continue;
        }
        let bas = s.to_ascii_lowercase();
        if i == dernier {
            // Fichier : gardé s'il s'appelle comme un identifiant technique (UUID, jeton opaque).
            let (racine, ext) = match s.rsplit_once('.') {
                Some((r, e)) if extension(s).is_some() => (r, Some(e.to_ascii_lowercase())),
                _ => (*s, None),
            };
            if est_uuid(racine) || est_identifiant_opaque(racine) {
                sortie.push(s.to_string());
            } else {
                let h = hache8(cle, "cle_s3/fichier", s);
                sortie.push(match ext {
                    Some(e) => format!("fichier-{h}.{e}"),
                    None => format!("fichier-{h}"),
                });
            }
        } else if SEGMENTS_STRUCTURELS.contains(&bas.as_str())
            || est_uuid(s)
            || est_identifiant_opaque(s)
        {
            sortie.push(s.to_string());
        } else if k.starts_with("http") && i == 0 {
            // Hôte d'une URL : remplacé par un domaine de test.
            sortie.push("stockage.example.test".to_string());
        } else {
            sortie.push(format!("d-{}", hache8(cle, "cle_s3/dossier", s)));
        }
    }
    format!("{prefixe}{}", sortie.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cle() -> Cle {
        Cle::depuis_texte(&"cd".repeat(32)).unwrap()
    }

    #[test]
    fn structure_gardee_noms_remplaces() {
        let c = cle();
        let r = reecrire_cle(
            &c,
            "documents/users/123e4567-e89b-42d3-a456-426614174000/CV-Dupont-Jean.pdf",
        );
        assert!(
            r.starts_with("documents/users/123e4567-e89b-42d3-a456-426614174000/fichier-"),
            "{r}"
        );
        assert!(r.ends_with(".pdf"));
        assert!(!r.contains("Dupont"));
    }

    #[test]
    fn dossier_nominatif_remplace_et_identifiant_garde() {
        let c = cle();
        let r = reecrire_cle(&c, "eleves/dupont-jean/photo.jpg");
        assert!(r.starts_with("eleves/d-") && !r.contains("dupont"), "{r}");
        let u = "123e4567-e89b-42d3-a456-426614174000.jpg";
        assert_eq!(
            reecrire_cle(&c, &format!("eleves/photos/{u}")),
            format!("eleves/photos/{u}")
        );
    }

    #[test]
    fn deterministe_et_coherent_entre_liste_et_colonne() {
        let c = cle();
        let a = reecrire_cle(&c, "requests/abc/files/justificatif-dupont.pdf");
        assert_eq!(
            a,
            reecrire_cle(&c, "requests/abc/files/justificatif-dupont.pdf")
        );
        assert_ne!(
            a,
            reecrire_cle(&c, "requests/abc/files/justificatif-durand.pdf")
        );
    }

    #[test]
    fn url_et_vide() {
        let c = cle();
        assert_eq!(reecrire_cle(&c, ""), "");
        let u = reecrire_cle(
            &c,
            "https://bucket-dupont.s3.fr-par.scw.cloud/documents/x/Contrat.pdf",
        );
        assert!(
            u.starts_with("https://stockage.example.test/documents/"),
            "{u}"
        );
        assert!(!u.contains("dupont") && !u.contains("Contrat"));
    }
}
