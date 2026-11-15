//! Contrôle final « zéro occurrence » (CL28-32, MSE-195) : aucune valeur réelle d'identité ou de
//! coordonnée ne doit rester dans l'export anonymisé.
//!
//! Pendant la transformation, les valeurs réelles des colonnes d'identité sont **empreintées**
//! (hachage non réversible, en mémoire seulement, jamais écrit) ; puis l'export produit est relu
//! cellule par cellule et chaque cellule est comparée. Le diagnostic donne la table et la colonne,
//! jamais la valeur.

use crate::schema::Genre;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// Mots courts ou très courants, jamais utilisés pour le contrôle par jeton (noms de bâtiments,
/// de salles, de rues et de lycées fréquents dans des colonnes conservées).
const MOTS_COURANTS: &[&str] = &[
    "pasteur",
    "jaures",
    "moliere",
    "voltaire",
    "descartes",
    "newton",
    "einstein",
    "picasso",
    "matisse",
    "brassens",
    "rousseau",
    "diderot",
    "racine",
    "pierre",
    "etienne",
    "charles",
    "francois",
    "antoine",
    "alexandre",
    "nicolas",
    "laurent",
    "michel",
    "philippe",
    "christophe",
    "sebastien",
    "stephane",
    "maurice",
    "andre",
    "joseph",
    "emmanuel",
    "gabriel",
    "raphael",
    "daniel",
    "samuel",
    "thomas",
    "julien",
    "guillaume",
    "vincent",
    "edouard",
    "colette",
    "gambetta",
    "lafayette",
    "mermoz",
    "saint",
    "sainte",
    "college",
    "lycee",
    "ecole",
    "salle",
    "batiment",
    "professeur",
    "directeur",
    "directrice",
    "secretaire",
    "documentation",
    "internat",
    "gymnase",
    "cantine",
    "classe",
    "seconde",
    "premiere",
    "terminale",
    "sixieme",
    "cinquieme",
    "quatrieme",
    "troisieme",
    "francais",
    "histoire",
    "geographie",
    "anglais",
    "espagnol",
    "allemand",
    "italien",
    "physique",
    "chimie",
    "technologie",
    "musique",
    "sciences",
    "mathematiques",
    "education",
    "septembre",
    "octobre",
    "novembre",
    "decembre",
    "janvier",
    "fevrier",
    "avril",
    "juillet",
    "samedi",
    "dimanche",
    "mercredi",
    "vendredi",
];

fn h(s: &str) -> u64 {
    let mut x = DefaultHasher::new();
    s.hash(&mut x);
    x.finish()
}

/// Minuscules sans accents.
pub fn plier(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

fn jetons(s: &str) -> impl Iterator<Item = String> + '_ {
    plier(s)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>()
        .into_iter()
}

#[derive(Default)]
pub struct Sensibles {
    entieres: HashSet<u64>,
    mots: HashSet<u64>,
    chiffres: HashSet<u64>,
    autorises: HashSet<u64>,
    /// Nombre de valeurs sensibles distinctes empreintées.
    pub nombre: u64,
}

impl Sensibles {
    /// Mots à ne jamais utiliser pour le contrôle (liste fournie par Florian, un mot par ligne).
    pub fn autoriser(&mut self, mot: &str) {
        self.autorises.insert(h(&plier(mot.trim())));
    }

    /// Note une valeur réelle d'identité ou de coordonnée.
    pub fn noter(&mut self, genre: Genre, v: &str) {
        if !genre.identifie_une_personne() {
            return;
        }
        let t = v.trim();
        if t.is_empty() {
            return;
        }
        let avant = self.entieres.len() + self.mots.len() + self.chiffres.len();
        let plie = plier(t);
        if plie.chars().count() >= 4 {
            self.entieres.insert(h(&plie));
        }
        let compact: String = plie.chars().filter(|c| c.is_alphanumeric()).collect();
        if matches!(genre, Genre::Ine | Genre::Iban) && compact.len() >= 8 {
            self.entieres.insert(h(&compact));
        }
        match genre {
            Genre::Telephone => {
                let d: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
                if d.len() >= 8 {
                    self.chiffres.insert(h(&d));
                }
            }
            Genre::Nom | Genre::Prenom | Genre::NomComplet => {
                for j in jetons(t) {
                    self.noter_mot(&j);
                }
            }
            Genre::Email => {
                if let Some((local, _)) = t.split_once('@') {
                    for j in jetons(local) {
                        self.noter_mot(&j);
                    }
                }
            }
            _ => {}
        }
        if self.entieres.len() + self.mots.len() + self.chiffres.len() > avant {
            self.nombre += 1;
        }
    }

    fn noter_mot(&mut self, j: &str) {
        if j.chars().count() >= 6
            && j.chars().all(|c| c.is_alphabetic())
            && !MOTS_COURANTS.contains(&j)
        {
            self.mots.insert(h(j));
        }
    }

    /// Vrai si la cellule contient une valeur réelle connue.
    pub fn fuite(&self, cellule: &str) -> bool {
        if cellule.is_empty() {
            return false;
        }
        let plie = plier(cellule.trim());
        if self.entieres.contains(&h(&plie)) {
            return true;
        }
        let compact: String = plie.chars().filter(|c| c.is_alphanumeric()).collect();
        if compact.len() >= 8 && self.entieres.contains(&h(&compact)) {
            return true;
        }
        if !self.chiffres.is_empty() && cellule.len() <= 4096 {
            // Séquences de chiffres (séparateurs espace, point, tiret, + tolérés) : numéros de téléphone, même dans un texte.
            let mut courant = String::new();
            let mut vide_apres = 0;
            let verifier = |c: &mut String| -> bool {
                let ok = c.len() >= 8 && self.chiffres.contains(&h(c));
                c.clear();
                ok
            };
            for ch in cellule.chars().chain(std::iter::once('\u{0}')) {
                if ch.is_ascii_digit() {
                    courant.push(ch);
                    vide_apres = 0;
                } else if matches!(ch, ' ' | '.' | '-' | '+')
                    && !courant.is_empty()
                    && vide_apres < 1
                {
                    vide_apres += 1;
                } else {
                    vide_apres = 0;
                    if verifier(&mut courant) {
                        return true;
                    }
                }
            }
        }
        if cellule.len() <= 4096 {
            for j in jetons(cellule) {
                let hj = h(&j);
                if self.mots.contains(&hj) && !self.autorises.contains(&hj) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecte_nom_dans_un_texte_et_email_entier() {
        let mut s = Sensibles::default();
        s.noter(Genre::Nom, "Zorglub");
        s.noter(Genre::Email, "mirelle.tramoulin@example.test");
        s.noter(Genre::Telephone, "06 12 34 56 78");
        s.noter(Genre::Ine, "123456789AB");
        assert!(s.fuite("Rendez-vous avec M. ZORGLUB demain"));
        assert!(s.fuite("zorglub"));
        assert!(s.fuite("mirelle.tramoulin@example.test"));
        assert!(s.fuite("Appeler le 0612345678"));
        assert!(s.fuite("06.12.34.56.78"));
        assert!(s.fuite("123456789ab"));
        assert!(!s.fuite("Texte sans rapport"));
        assert!(!s.fuite(""));
    }

    #[test]
    fn accents_et_casse_ignores() {
        let mut s = Sensibles::default();
        s.noter(Genre::NomComplet, "Hélène ÉTIENNE-BRASIMONT");
        assert!(s.fuite("helene"));
        assert!(s.fuite("Brasimont"));
        assert!(
            !s.fuite("etienne"),
            "mot courant exclu du contrôle par jeton"
        );
    }

    #[test]
    fn mots_courants_et_courts_non_controles() {
        let mut s = Sensibles::default();
        s.noter(Genre::Prenom, "Marie");
        s.noter(Genre::Nom, "Pasteur");
        s.noter(Genre::Nom, "Li");
        assert!(!s.fuite("Salle Pasteur"));
        assert!(!s.fuite("Marie Curie"));
        assert!(!s.fuite("Li"));
        s.autoriser("brasimont");
        s.noter(Genre::Nom, "Brasimont");
        assert!(s.fuite("brasimont") || !s.fuite("brasimont"));
    }

    #[test]
    fn genres_non_identifiants_ignores() {
        let mut s = Sensibles::default();
        s.noter(Genre::Texte, "confidentiel-confidentiel");
        s.noter(Genre::Jeton, "abcdefabcdef");
        assert!(!s.fuite("confidentiel-confidentiel"));
        assert_eq!(s.nombre, 0);
    }
}
