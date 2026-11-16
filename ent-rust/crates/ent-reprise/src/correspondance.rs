//! Table de correspondance : un identifiant du Next.js donne un identifiant ScolIA, **de façon stable et
//! rejouable**.
//!
//! L'identifiant ScolIA d'un objet est dérivé (SHA-256) de l'établissement cible, de la source, de l'objet
//! source et de l'identifiant source : jamais aléatoire, donc identique à chaque exécution, sur n'importe
//! quelle machine, sans rien lire. Une correspondance déjà enregistrée (décision humaine de rapprochement
//! d'un élève avec une fiche du cœur, par exemple) **prime** sur la dérivation.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const SOURCE_NEXT: &str = "next";

/// Dérivation de l'identifiant ScolIA (UUID version 8, variante RFC 4122) : stable et sans état.
pub fn id_derive(etablissement_cible: &str, source: &str, objet: &str, source_id: &str) -> String {
    let mut h = Sha256::new();
    for p in ["scolia-reprise-v1", etablissement_cible, source, objet, source_id] {
        h.update((p.len() as u64).to_le_bytes());
        h.update(p.as_bytes());
    }
    let d = h.finalize();
    let mut b = [0u8; 16];
    b.copy_from_slice(&d[..16]);
    b[6] = (b[6] & 0x0f) | 0x80;
    b[8] = (b[8] & 0x3f) | 0x80;
    let x = hex::encode(b);
    format!("{}-{}-{}-{}-{}", &x[0..8], &x[8..12], &x[12..16], &x[16..20], &x[20..32])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Statut {
    Rapprochee,
    ADecider,
    Ignoree,
    Creee,
}

impl Statut {
    pub fn code(self) -> &'static str {
        match self {
            Statut::Rapprochee => "rapprochee",
            Statut::ADecider => "a_decider",
            Statut::Ignoree => "ignoree",
            Statut::Creee => "creee",
        }
    }

    pub fn depuis(s: &str) -> Option<Statut> {
        Some(match s {
            "rapprochee" => Statut::Rapprochee,
            "a_decider" => Statut::ADecider,
            "ignoree" => Statut::Ignoree,
            "creee" => Statut::Creee,
            _ => return None,
        })
    }
}

/// Correspondance déjà enregistrée dans ScolIA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Existante {
    pub cible_id: String,
    /// Empreinte de la ligne source à la dernière reprise.
    pub empreinte_source: Option<String>,
    /// L'objet cible a été modifié dans ScolIA depuis la reprise : jamais écrasé.
    pub modifie_depuis_reprise: bool,
    pub statut: Statut,
}

/// Correspondances connues, indexées par (objet source, identifiant source).
pub type Existantes = BTreeMap<(String, String), Existante>;

/// Identifiant cible : la correspondance enregistrée si elle existe, sinon la dérivation.
pub fn resoudre(etablissement_cible: &str, objet: &str, source_id: &str, existantes: &Existantes) -> String {
    match existantes.get(&(objet.to_string(), source_id.to_string())) {
        Some(e) => e.cible_id.clone(),
        None => id_derive(etablissement_cible, SOURCE_NEXT, objet, source_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ETAB: &str = "0e7ab115-0000-4000-8000-00000000f1c7";

    #[test]
    fn derivation_stable_uuid_valide_et_sans_collision_triviale() {
        let a = id_derive(ETAB, SOURCE_NEXT, "eleve", "abc");
        assert_eq!(a, id_derive(ETAB, SOURCE_NEXT, "eleve", "abc"));
        assert!(crate::decodage::cellule::est_uuid(&a));
        assert_eq!(&a[14..15], "8");
        assert!(matches!(&a[19..20], "8" | "9" | "a" | "b"));
        assert_ne!(a, id_derive(ETAB, SOURCE_NEXT, "eleve", "abd"));
        assert_ne!(a, id_derive(ETAB, SOURCE_NEXT, "foyer", "abc"));
        assert_ne!(a, id_derive("0e7ab115-0000-4000-8000-00000000f1c8", SOURCE_NEXT, "eleve", "abc"));
        // Les frontières de champs comptent : ("ab","c") ≠ ("a","bc").
        assert_ne!(id_derive(ETAB, "next", "ab", "c"), id_derive(ETAB, "next", "a", "bc"));
    }

    #[test]
    fn la_correspondance_enregistree_prime() {
        let mut m = Existantes::new();
        let derive = resoudre(ETAB, "eleve", "e1", &m);
        assert_eq!(derive, id_derive(ETAB, SOURCE_NEXT, "eleve", "e1"));
        m.insert(
            ("eleve".into(), "e1".into()),
            Existante {
                cible_id: "11111111-1111-4111-8111-111111111111".into(),
                empreinte_source: None,
                modifie_depuis_reprise: false,
                statut: Statut::Rapprochee,
            },
        );
        assert_eq!(resoudre(ETAB, "eleve", "e1", &m), "11111111-1111-4111-8111-111111111111");
        assert_eq!(resoudre(ETAB, "eleve", "e2", &m), id_derive(ETAB, SOURCE_NEXT, "eleve", "e2"));
    }

    #[test]
    fn statuts_aller_retour() {
        for s in [Statut::Rapprochee, Statut::ADecider, Statut::Ignoree, Statut::Creee] {
            assert_eq!(Statut::depuis(s.code()), Some(s));
        }
        assert_eq!(Statut::depuis("autre"), None);
    }
}
