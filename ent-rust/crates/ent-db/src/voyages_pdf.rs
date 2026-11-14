//! Socle 42, lot V0b : rendu PDF **côté serveur** de la fiche budget et de la circulaire
//! (`docs/socles/42_SORTIES_VOYAGES.md` §12.1, §16 ; NX-57, NX-59).
//!
//! Mise en page par la brique `ent-pdf` (D-062) : A4, plusieurs pages, **accents conservés**
//! (encodage WinAnsi, `€` compris), montants alignés à droite, « Page n sur N ». Le rendu est **déterministe**
//! (aucune date, aucun identifiant aléatoire) : deux rendus du même contenu sont identiques
//! octet pour octet, ce qui permet de comparer l'empreinte au registre des objets. Le contenu vient
//! des seuls faits du dossier lus en base ; rien n'est repris d'un texte venu du navigateur.

use ent_pdf::{Document, Meta};

/// Un bloc de mise en page.
#[derive(Debug, Clone, PartialEq)]
pub enum Bloc {
    Titre(String),
    SousTitre(String),
    Paragraphe(String),
    /// Libellé à gauche, montant ou valeur alignés à droite.
    Ligne {
        gauche: String,
        droite: String,
        gras: bool,
    },
    Espace,
}

/// Rend le document. `pied` figure en bas de chaque page, avec « Page n sur N ».
pub fn rendre(pied: &str, blocs: &[Bloc]) -> Vec<u8> {
    let titre = blocs
        .iter()
        .find_map(|b| if let Bloc::Titre(t) = b { Some(t.as_str()) } else { None })
        .unwrap_or("Document");
    let mut d = Document::new(Meta { titre: titre.to_string(), auteur: "ScolIA".into(), creation: None })
        .avec_pied(pied);
    for b in blocs {
        match b {
            Bloc::Espace => d.espace(8.0),
            Bloc::Titre(t) => d.titre(t),
            Bloc::SousTitre(t) => d.sous_titre(t),
            Bloc::Paragraphe(t) => d.paragraphe(t),
            Bloc::Ligne { gauche, droite, gras } => d.ligne_droite(gauche, droite, *gras),
        }
    }
    d.octets()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_pdf::releve::{texte, texte_des_pages, verifier_structure};

    fn contient(hay: &[u8], aiguille: &str) -> bool {
        hay.windows(aiguille.len()).any(|w| w == aiguille.as_bytes())
    }

    #[test]
    fn document_valide_deterministe_et_accents_conserves() {
        let blocs = vec![
            Bloc::Titre("Fiche budget — Séjour à Rome".into()),
            Bloc::Paragraphe("Réunion (parents) \\ échéance : 15 € — « ok »".into()),
            Bloc::Ligne { gauche: "Transport".into(), droite: "1\u{a0}350,50\u{a0}€".into(), gras: false },
        ];
        let a = rendre("Établissement", &blocs);
        let b = rendre("Établissement", &blocs);
        assert_eq!(a, b, "rendu déterministe");
        verifier_structure(&a);
        assert!(a.ends_with(b"%%EOF\n"));
        // « é » (0xE9 = 351 en octal), « € » (0x80 = 200), parenthèses et antislash échappés.
        assert!(contient(&a, "Fiche budget \\227 S\\351jour"), "accent et tiret cadratin");
        assert!(contient(&a, "\\(parents\\) \\\\"));
        assert!(contient(&a, "15 \\200"));
        // Le fichier reste ASCII hors de l'en-tête binaire : aucun octet brut ≥ 0x80 dans le flux.
        let corps = &a[15..];
        assert!(corps.iter().all(|&o| o < 0x80), "flux 7 bits");
        let t = texte(&a);
        assert!(t.contains("Fiche budget — Séjour à Rome") && t.contains("1\u{a0}350,50\u{a0}€"), "{t}");
        assert!(t.contains("Établissement") && t.contains("Page 1 sur 1"));
    }

    #[test]
    fn table_xref_exacte() {
        let blocs: Vec<Bloc> = (0..120)
            .map(|i| Bloc::Paragraphe(format!("Ligne numéro {i}")))
            .collect();
        let pdf = rendre("pied", &blocs);
        let n = verifier_structure(&pdf);
        assert!(n > 8, "plusieurs pages : {n} objets");
    }

    #[test]
    fn plusieurs_pages_numerotees_et_texte_long_coupe() {
        let blocs: Vec<Bloc> = (0..201)
            .map(|i| Bloc::Ligne { gauche: format!("Poste {i}"), droite: "10,00\u{a0}€".into(), gras: i % 50 == 0 })
            .collect();
        let pdf = rendre("pied", &blocs);
        let pages = texte_des_pages(&pdf);
        assert_eq!(pages.len(), 4, "201 lignes, une cinquantaine par page");
        for (i, p) in pages.iter().enumerate() {
            assert!(p.contains(&format!("Page {} sur 4", i + 1)), "page {i}");
        }
        // Un mot géant ne déborde pas : il est coupé en tranches.
        let long = "x".repeat(300);
        let pdf = rendre("pied", &[Bloc::Paragraphe(long.clone())]);
        assert!(!contient(&pdf, &long));
        assert_eq!(texte(&pdf).matches('x').count(), 300, "rien de perdu");
    }

    #[test]
    fn caracteres_hors_table_lisibles() {
        let pdf = rendre("pied", &[Bloc::Paragraphe("é€’œ Łódź 日".into())]);
        assert!(texte(&pdf).contains("é€’œ Lódź ?"));
    }
}
